//! Whether a rebuild produces the structure it replaced, and what that would
//! be worth.
//!
//! §20.7 of `docs/gpui-shell.md` proposes a template cache: a description split
//! into a reusable structure and the dynamic slots inside it, so that a
//! value-only change writes slots instead of running the builder again. The
//! whole idea rests on an assumption nothing had measured — that a dirty render
//! usually produces the shape the previous one produced — and on a bound nobody
//! had counted: how much of a description is values a template could fill,
//! against handlers it could not.
//!
//! These tests answer both, in that order:
//!
//! ```text
//! does the shape repeat?   ── StructureFingerprint, counted by RuntimeMetrics
//! how much would it save?  ── the slot census below
//! ```
//!
//! The census is a `println!` rather than an assertion. It is a reading of a
//! synthetic panel, not a property of the runtime, and pinning it would make an
//! unrelated change to the panel look like a regression.

use std::ops::Deref;

use crate::{
    ScriptView, ShellRuntime,
    spec::{Component, SpecOp},
};
use gpui::{AppContext as _, Entity, IntoElement as _, TestAppContext, VisualTestContext};

const ENTRY: &str = "structure.js";

/// A view whose every render changes a value and nothing else.
const VALUES_ONLY: &str = r#"
import { View } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";

export default class Quote extends View {
  init() {
    this.tick = 0;
  }

  render() {
    this.tick += 1;
    return v_flex()
      .gap(4)
      .child(h_flex().gap(6).child("AAPL").child(`${230 + this.tick}.42`))
      .child(h_flex().gap(6).child("MSFT").child(`${410 + this.tick}.08`));
  }
}
"#;

/// The same, with a handler on every row: the `CallbackId` behind each one is
/// minted fresh on every render and retired with the snapshot generation, so a
/// fingerprint that kept it would report a change every single time.
const VALUES_ONLY_WITH_HANDLERS: &str = r#"
import { View } from "gpui-kit";
import { v_flex, h_flex, Button } from "gpui-base";

export default class Quote extends View {
  init() {
    this.tick = 0;
  }

  render() {
    this.tick += 1;
    return v_flex()
      .gap(4)
      .child(Button.new("aapl").on_click(() => this.tick).child(`${230 + this.tick}.42`))
      .child(Button.new("msft").on_click(() => this.tick).child(`${410 + this.tick}.08`));
  }
}
"#;

/// A view that alternates between two shapes.
const ALTERNATING_BRANCH: &str = r#"
import { View } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";

export default class Branch extends View {
  init() {
    this.tick = 0;
  }

  render() {
    this.tick += 1;
    if (this.tick % 2 === 0) {
      return v_flex().gap(4).child("loading");
    }
    return v_flex().gap(4).child(h_flex().child("content"));
  }
}
"#;

/// A view that grows by one row per render.
const GROWING_LIST: &str = r#"
import { View } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Growing extends View {
  init() {
    this.rows = 0;
  }

  render() {
    this.rows += 1;
    const children = [];
    for (let row = 0; row < this.rows; row += 1) {
      children.push(`row ${row}`);
    }
    return v_flex().gap(4).children(children);
  }
}
"#;

/// The census panel: a watchlist of the shape §20.7 names as the best case —
/// repeated rows, a handler each, and only the numbers moving.
const WATCHLIST: &str = r#"
import { View, div } from "gpui-kit";
import { v_flex, h_flex, Button } from "gpui-base";

export default class Watchlist extends View {
  init() {
    this.tick = 0;
    this.rows = 40;
  }

  row(index) {
    const price = (100 + index + this.tick / 100).toFixed(2);
    return h_flex()
      .gap(6)
      .py(2)
      .px(6)
      .rounded(4)
      .bg(`#f8f8f8`)
      .child(div().w(80).text_sm().text_color(`#111111`).child(`SYM${index}`))
      .child(div().w(80).text_sm().text_color(`#111111`).child(price))
      .child(div().w(60).text_sm().text_color(`#6b7280`).child("+1.42%"))
      .child(Button.new(`trade-${index}`).px(8).py(2).on_click(() => index).child("Trade"));
  }

  render() {
    this.tick += 1;
    const rows = [];
    for (let index = 0; index < this.rows; index += 1) {
      rows.push(this.row(index));
    }
    return v_flex().size_full().p(12).gap(4).bg(`#ffffff`).children(rows);
  }
}
"#;

/// The census panel with the one thing a template cannot fill removed, and
/// nothing else changed: the difference between rebuilding this and rebuilding
/// [`WATCHLIST`] is what minting forty closures and registering forty callbacks
/// costs.
const WATCHLIST_WITHOUT_HANDLERS: &str = r#"
import { View, div } from "gpui-kit";
import { v_flex, h_flex, Button } from "gpui-base";

export default class Watchlist extends View {
  init() {
    this.tick = 0;
    this.rows = 40;
  }

  row(index) {
    const price = (100 + index + this.tick / 100).toFixed(2);
    return h_flex()
      .gap(6)
      .py(2)
      .px(6)
      .rounded(4)
      .bg(`#f8f8f8`)
      .child(div().w(80).text_sm().text_color(`#111111`).child(`SYM${index}`))
      .child(div().w(80).text_sm().text_color(`#111111`).child(price))
      .child(div().w(60).text_sm().text_color(`#6b7280`).child("+1.42%"))
      .child(Button.new(`trade-${index}`).px(8).py(2).child("Trade"));
  }

  render() {
    this.tick += 1;
    const rows = [];
    for (let index = 0; index < this.rows; index += 1) {
      rows.push(this.row(index));
    }
    return v_flex().size_full().p(12).gap(4).bg(`#ffffff`).children(rows);
  }
}
"#;

#[gpui::test]
fn a_value_only_change_repeats_the_structure(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, VALUES_ONLY);

    render_once(&mut context, &view);
    let first = runtime.metrics().read();
    assert_eq!(
        (first.structure_repeats(), first.structure_changes()),
        (0, 0),
        "a first build has no predecessor and is not a data point either way"
    );
    assert_eq!(
        first.structure_repeat_rate(),
        None,
        "and a rate over no comparisons is absent rather than zero"
    );

    invalidate(&mut context, &view);
    invalidate(&mut context, &view);

    let reading = runtime.metrics().read();
    assert_eq!(
        (reading.structure_repeats(), reading.structure_changes()),
        (2, 0),
        "only the numbers moved, so both rebuilds described the shape they replaced"
    );
    assert_eq!(reading.structure_repeat_rate(), Some(1.0));
}

#[gpui::test]
fn a_fresh_handler_is_not_a_change_of_structure(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, VALUES_ONLY_WITH_HANDLERS);

    render_once(&mut context, &view);
    invalidate(&mut context, &view);

    let reading = runtime.metrics().read();
    assert_eq!(
        (reading.structure_repeats(), reading.structure_changes()),
        (1, 0),
        "every render mints new CallbackIds; counting them as shape would make \
         every description containing a handler look new"
    );
}

#[gpui::test]
fn taking_the_other_branch_changes_the_structure(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, ALTERNATING_BRANCH);

    render_once(&mut context, &view);
    invalidate(&mut context, &view);
    invalidate(&mut context, &view);

    let reading = runtime.metrics().read();
    assert_eq!(
        (reading.structure_repeats(), reading.structure_changes()),
        (0, 2),
        "the two branches describe different trees"
    );
    assert_eq!(reading.structure_repeat_rate(), Some(0.0));
}

#[gpui::test]
fn a_row_appearing_changes_the_structure(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, GROWING_LIST);

    render_once(&mut context, &view);
    invalidate(&mut context, &view);

    let reading = runtime.metrics().read();
    assert_eq!(
        (reading.structure_repeats(), reading.structure_changes()),
        (0, 1),
        "one more child is one more node and one more attachment"
    );
}

/// What a template could and could not fill, counted on a 40-row watchlist
/// whose prices move and whose structure does not.
///
/// Prints rather than asserts — see this module's comment. The three numbers
/// are §20.7's step 2: the slot ceiling (`arena.len()` against the positions
/// that actually differ), and the handler share that bounds it.
#[gpui::test]
fn the_slot_census_of_a_repeating_panel(cx: &mut TestAppContext) {
    let (runtime, mut context, object) = script_object(cx, WATCHLIST);

    let (before, after) = context.update(|window, cx| {
        let mut build = || {
            runtime
                .build_snapshot(&object, None, crate::policy::default(), window, cx)
                .expect("render")
        };
        // Three builds, comparing the last two: the first pays for lazily
        // initialized module state and is not representative of a steady tick.
        build();
        let before = census(&build());
        let after = census(&build());
        (before, after)
    });

    assert_eq!(
        before.structure, after.structure,
        "the watchlist's shape must repeat, or the census below is measuring \
         something other than the case a template serves"
    );

    let diff = before.diff(&after);
    let total_ops: usize = after.nodes.iter().map(|node| node.1.len()).sum();

    println!(
        "\n[E] slot census — 40-row watchlist, prices moving, shape repeating\
         \n    nodes                    {}\
         \n    recorded ops             {total_ops}\
         \n    components that differ   {} ({:.1}% of nodes)\
         \n    value ops that differ    {} ({:.1}% of ops)\
         \n    handler ops              {} ({:.1}% of ops) — always differ, never fillable\
         \n    slot ceiling             {} of {} positions ({:.1}%)",
        after.nodes.len(),
        diff.components,
        percent(diff.components, after.nodes.len()),
        diff.value_ops,
        percent(diff.value_ops, total_ops),
        diff.handler_ops,
        percent(diff.handler_ops, total_ops),
        diff.components + diff.value_ops + diff.handler_ops,
        after.nodes.len() + total_ops,
        percent(
            diff.components + diff.value_ops + diff.handler_ops,
            after.nodes.len() + total_ops
        ),
    );
}

fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    part as f64 * 100.0 / whole as f64
}

/// One description, flattened into owned data so two of them can be compared
/// after both borrows have ended.
struct Census {
    structure: crate::spec::StructureFingerprint,
    root: crate::spec::SpecId,
    nodes: Vec<(Option<Component>, Vec<SpecOp>)>,
    children: Vec<Vec<crate::spec::SpecId>>,
}

/// How many positions two descriptions of the same shape disagree on.
struct Diff {
    components: usize,
    value_ops: usize,
    handler_ops: usize,
}

impl Census {
    fn diff(&self, other: &Self) -> Diff {
        let mut diff = Diff {
            components: 0,
            value_ops: 0,
            handler_ops: 0,
        };

        for (mine, theirs) in self.nodes.iter().zip(&other.nodes) {
            if mine.0 != theirs.0 {
                diff.components += 1;
            }
            for (left, right) in mine.1.iter().zip(&theirs.1) {
                // Handlers are counted apart from values rather than among
                // them: a `CallbackId` differs on every render by construction,
                // so folding it in would overstate what a template could fill.
                if matches!(right, SpecOp::Callback(..) | SpecOp::ActionCallback(..)) {
                    diff.handler_ops += 1;
                } else if left != right {
                    diff.value_ops += 1;
                }
            }
        }

        diff
    }
}

fn census(snapshot: &crate::RenderSnapshot) -> Census {
    let arena = snapshot.arena();
    Census {
        structure: snapshot.structure(),
        root: snapshot.root(),
        nodes: (0..arena.len() as u32)
            .map(|id| {
                let node = arena.node(id).expect("node");
                (node.component().cloned(), node.ops().to_vec())
            })
            .collect(),
        children: (0..arena.len() as u32)
            .map(|id| arena.node(id).expect("node").children().to_vec())
            .collect(),
    }
}

struct Empty;

impl gpui::Render for Empty {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
    }
}

pub(super) fn script_view(
    cx: &mut TestAppContext,
    source: &str,
) -> (
    std::rc::Rc<ShellRuntime>,
    VisualTestContext,
    Entity<ScriptView>,
) {
    let (runtime, mut context, object) = script_object(cx, source);
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime.clone(), object)));
    (runtime, context, view)
}

pub(super) fn script_object(
    cx: &mut TestAppContext,
    source: &str,
) -> (
    std::rc::Rc<ShellRuntime>,
    VisualTestContext,
    crate::engine::ViewObject,
) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let view_type = runtime.load_source(ENTRY, source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context.update(|window, cx| {
        runtime
            .instantiate(&view_type, window, cx)
            .expect("instantiate")
    });

    (runtime, context, object)
}

pub(super) fn render_once(context: &mut VisualTestContext, view: &Entity<ScriptView>) {
    let view = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| view.into_any_element(),
    );
}

/// Marks the view dirty the way `cx.notify()` does, then draws — which is the
/// only thing that runs the script again.
pub(super) fn invalidate(context: &mut VisualTestContext, view: &Entity<ScriptView>) {
    context.update(|_, cx| view.update(cx, |view, _| view.invalidate()));
    render_once(context, view);
}

/// What the boundary is worth, in the units the rest of this file uses.
///
/// §20.4 says the unit that decides cost is the view that was invalidated, and
/// the Performance page draws it. This is the number under that drawing: the
/// same forty-row watchlist described whole, against one of its rows described
/// alone — which is what a price tick costs when the row is a retained view of
/// its own rather than part of the panel's description.
#[gpui::test]
fn one_row_against_the_panel_that_holds_it(cx: &mut TestAppContext) {
    const ITERATIONS: usize = 50;
    const ROUNDS: usize = 7;

    let panel = time_source(cx, WATCHLIST, ITERATIONS, ROUNDS);
    let row = time_source(cx, ONE_ROW, ITERATIONS, ROUNDS);

    println!(
        "\n[J] the invalidation boundary — 40-row watchlist\
         \n    the whole panel (361 nodes)   {:.3} ms\
         \n    one row (9 nodes)             {:.3} ms\
         \n    ratio                         {:.0}x",
        panel.as_secs_f64() * 1000.0,
        row.as_secs_f64() * 1000.0,
        panel.as_secs_f64() / row.as_secs_f64().max(f64::MIN_POSITIVE),
    );

    assert!(
        row < panel,
        "describing one row must cost less than describing forty: {row:?} against {panel:?}"
    );
}

/// One row of [`WATCHLIST`], as a view of its own.
const ONE_ROW: &str = r#"
import { View, div } from "gpui-kit";
import { h_flex, Button } from "gpui-base";

export default class Row extends View {
  init() { this.tick = 0; }

  render() {
    this.tick += 1;
    const price = (100 + this.tick / 100).toFixed(2);
    return h_flex()
      .gap(6)
      .py(2)
      .px(6)
      .rounded(4)
      .bg(`#f8f8f8`)
      .child(div().w(80).text_sm().text_color(`#111111`).child("SYM0"))
      .child(div().w(80).text_sm().text_color(`#111111`).child(price))
      .child(div().w(60).text_sm().text_color(`#6b7280`).child("+1.42%"))
      .child(Button.new("trade-0").px(8).py(2).on_click(() => 0).child("Trade"));
  }
}
"#;

fn time_source(
    cx: &mut TestAppContext,
    source: &str,
    iterations: usize,
    rounds: usize,
) -> std::time::Duration {
    let (runtime, mut context, object) = script_object(cx, source);
    context.update(|window, cx| {
        let mut build = || {
            runtime
                .build_snapshot(&object, None, crate::policy::default(), window, cx)
                .expect("render")
        };
        build();

        let mut best = std::time::Duration::MAX;
        for _ in 0..rounds {
            let started = std::time::Instant::now();
            for _ in 0..iterations {
                build();
            }
            best = best.min(started.elapsed() / iterations as u32);
        }
        best
    })
}

/// What level 2 would cost, if the surface existed to reach it.
///
/// §20.7's second problem is that a template cache only pays if the *builder
/// calls are not made* — reusing the arena on the Rust side while JavaScript
/// still runs `div().flex().gap(4)` removes the smaller half of a recorded
/// call and leaves the interpreter cost untouched. That is an argument, not a
/// number, and the number is what decides whether the surface is worth
/// designing.
///
/// So this replays one description into a fresh arena through the arena's own
/// API — `push`, `push_op`, `attach`, the exact calls an instantiation would
/// make — with this render's values written into the positions the census
/// found varying. No JavaScript runs, nothing crosses the bridge, and no
/// `Bridged` is converted. It is the fill path with the surface left out.
///
/// It is a floor rather than a promise. A real instantiation carries costs
/// this does not — resolving which template and which variant, and minting the
/// handlers §20.7's third problem says stay — and the census above says half of
/// what a watchlist row writes is exactly those handlers.
#[gpui::test]
fn the_fill_path_against_the_rebuild_it_would_replace(cx: &mut TestAppContext) {
    const ITERATIONS: usize = 50;
    const ROUNDS: usize = 7;

    let (runtime, mut context, object) = script_object(cx, WATCHLIST);

    let (rebuild, fill, nodes, ops) = context.update(|window, cx| {
        let mut build = || {
            runtime
                .build_snapshot(&object, None, crate::policy::default(), window, cx)
                .expect("render")
        };

        build();
        let template = census(&build());
        let ops: usize = template.nodes.iter().map(|node| node.1.len()).sum();

        // Best of several batches rather than one average, for the reason
        // `benchmark.rs` gives: every source of noise on a shared machine adds
        // time and none removes it.
        let mut rebuild = std::time::Duration::MAX;
        for _ in 0..ROUNDS {
            let started = std::time::Instant::now();
            for _ in 0..ITERATIONS {
                build();
            }
            rebuild = rebuild.min(started.elapsed() / ITERATIONS as u32);
        }

        let mut fill = std::time::Duration::MAX;
        for _ in 0..ROUNDS {
            let started = std::time::Instant::now();
            for _ in 0..ITERATIONS {
                std::hint::black_box(replay(&template));
            }
            fill = fill.min(started.elapsed() / ITERATIONS as u32);
        }

        (rebuild, fill, template.nodes.len(), ops)
    });

    // The same panel with `.on_click(...)` removed and nothing else changed.
    // The gap between the two rebuilds is the cost a template keeps paying:
    // forty closures allocated and forty callbacks registered, which happen
    // whether or not the structure around them was reused.
    let (bare_runtime, mut bare_context, bare) = script_object(cx, WATCHLIST_WITHOUT_HANDLERS);
    let handlerless = bare_context.update(|window, cx| {
        let runtime = &bare_runtime;
        let mut build = || {
            runtime
                .build_snapshot(&bare, None, crate::policy::default(), window, cx)
                .expect("render")
        };
        build();
        let mut best = std::time::Duration::MAX;
        for _ in 0..ROUNDS {
            let started = std::time::Instant::now();
            for _ in 0..ITERATIONS {
                build();
            }
            best = best.min(started.elapsed() / ITERATIONS as u32);
        }
        best
    });

    let handlers = rebuild.saturating_sub(handlerless);
    let realistic = fill + handlers;

    println!(
        "\n[G] fill against rebuild — 40-row watchlist, {nodes} nodes, {ops} recorded ops\
         \n    rebuild (script → snapshot)        {:.3} ms\
         \n    fill    (replay + slots)           {:.3} ms   {:.1}x\
         \n    rebuild without the 40 handlers    {:.3} ms\
         \n    ...so handlers cost               {:.3} ms, and a template still pays them\
         \n    fill + handlers                    {:.3} ms   {:.1}x",
        rebuild.as_secs_f64() * 1000.0,
        fill.as_secs_f64() * 1000.0,
        rebuild.as_secs_f64() / fill.as_secs_f64().max(f64::MIN_POSITIVE),
        handlerless.as_secs_f64() * 1000.0,
        handlers.as_secs_f64() * 1000.0,
        realistic.as_secs_f64() * 1000.0,
        rebuild.as_secs_f64() / realistic.as_secs_f64().max(f64::MIN_POSITIVE),
    );

    // A smoke bound rather than the real reading, which is the printed number:
    // the assertion has to hold in a debug build too, where both sides are
    // slower by different factors.
    assert!(
        fill < rebuild,
        "replaying a description in Rust should not cost more than describing it \
         through the bridge: fill {fill:?} against rebuild {rebuild:?}"
    );
}

/// Rebuilds one description into a fresh arena, through the calls an
/// instantiation would make.
///
/// Nodes first, then their operations, then the tree — because the arena
/// refuses an operation on a node that has already been attached, which is the
/// same rule that stops a script reusing an element.
fn replay(template: &Census) -> crate::spec::SpecArena {
    let mut arena = crate::spec::SpecArena::new();

    for (component, _) in &template.nodes {
        let component = component.clone().expect("a described node");
        arena.push(component);
    }

    for (id, (_, ops)) in template.nodes.iter().enumerate() {
        for op in ops {
            arena.push_op(id as u32, op.clone()).expect("op");
        }
    }

    // The tree, bottom up. The arena refuses an attachment to a node that has
    // already been attached itself — the same rule that stops a script reusing
    // an element — so a parent has to receive all of its children before its
    // own parent receives it. A post-order walk from the root is exactly that
    // order, and it is the order the builder produces naturally.
    attach_below(&mut arena, template, template.root);

    arena
}

fn attach_below(arena: &mut crate::spec::SpecArena, template: &Census, parent: u32) {
    for child in &template.children[parent as usize] {
        attach_below(arena, template, *child);
    }
    for child in &template.children[parent as usize] {
        arena.attach(parent, *child).expect("attach");
    }
}
