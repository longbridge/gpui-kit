//! What a script view costs, split into the three costs that are not the same.
//!
//! The design doc (§20.3) asked one question — how long does it take script code
//! to describe a realistic interface — and that number still decides whether the
//! approach is viable. But it is no longer the frame cost, because a description
//! is built once and replayed by every frame that follows it. So the measurement
//! is split three ways:
//!
//! ```text
//! A  script ──▶ snapshot      cost of one application invalidation
//! B  snapshot ──▶ elements    cost that GPUI actually pays per frame
//! C  snapshot ──▶ N frames    proof that A is not paid per frame at all
//! ```
//!
//! A and B are timings. C is an architectural assertion with a timing attached:
//! if the script render count moves while a clean view repaints, the runtime has
//! regressed to the coupling this design exists to remove.
//!
//! Run with output:
//!
//! ```text
//! cargo test -p gpui-shell --release --test benchmark -- --nocapture
//! ```

use std::{ops::Deref, time::Instant};

use crate::{RenderSnapshot, ScriptView, ShellRuntime, materialize::materialize};
use gpui::{AppContext as _, Entity, IntoElement as _, TestAppContext, VisualTestContext};

/// Rows and columns chosen to land near the doc's "typical panel" figure:
/// 40 rows x 5 cells plus wrappers is ~250 nodes, each carrying 8-12 ops.
const ROWS: usize = 40;
const COLUMNS: usize = 5;
const ITERATIONS: usize = 50;
/// How many batches of [`ITERATIONS`] a timing takes before believing the
/// fastest one.
const ROUNDS: usize = 7;

/// Panel sizes the scaling test walks, from the typical panel above to one no
/// single view should hold. The last two exist to show where the description
/// stops fitting a frame — and that the frame still never enters the VM.
const SIZES: [(usize, usize); 4] = [(40, 5), (100, 10), (200, 10), (400, 10)];

const TEMPLATE: &str = r#"
import { View, div } from "gpui-kit";
import { v_flex, h_flex, Button } from "gpui-base";

export default class Grid extends View {
  init() {
    this.rows = __ROWS__;
    this.columns = __COLUMNS__;
  }

  cell(row, column) {
    return div()
      .flex()
      .items_center()
      .justify_center()
      .w(90)
      .h(24)
      .px(6)
      .rounded(4)
      .bg(`#f8f8f8`)
      .text_color(`#111111`)
      .text_sm()
      .child(`${row}:${column}`);
  }

  row(row) {
    const cells = [];
    for (let column = 0; column < this.columns; column += 1) {
      cells.push(this.cell(row, column));
    }
    return h_flex().gap(6).py(2).children(cells);
  }

  render(cx) {
    const rows = [];
    for (let row = 0; row < this.rows; row += 1) {
      rows.push(this.row(row));
    }
    return v_flex()
      .size_full()
      .p(12)
      .gap(4)
      .bg(`#ffffff`)
      .children(rows)
      .child(Button.new("refresh").px(10).py(4).rounded(6).bg(`#2563eb`).child("Refresh"));
  }
}
"#;

const _: () = assert!(ROWS > 0 && COLUMNS > 0);

fn source(rows: usize, columns: usize) -> String {
    TEMPLATE
        .replace("__ROWS__", &rows.to_string())
        .replace("__COLUMNS__", &columns.to_string())
}

#[gpui::test]
fn describing_a_panel_stays_inside_the_frame_budget(cx: &mut TestAppContext) {
    let (runtime, mut context, object) = grid(cx, ROWS, COLUMNS);

    // Warm up: the style table and the module both initialize lazily.
    let nodes = context.update(|window, cx| {
        runtime
            .build_snapshot(&object, None, crate::policy::default(), window, cx)
            .expect("render")
            .len()
    });

    // Best of several batches rather than one average. Every source of noise
    // available to a benchmark on a shared machine adds time; none removes it,
    // so the fastest batch is the closest reading of what the work costs — and
    // averaging one batch made changes worth a few per cent unreadable.
    let mut per_build = std::time::Duration::MAX;
    context.update(|window, cx| {
        for _ in 0..ROUNDS {
            let started = Instant::now();
            for _ in 0..ITERATIONS {
                runtime
                    .build_snapshot(&object, None, crate::policy::default(), window, cx)
                    .expect("render");
            }
            per_build = per_build.min(started.elapsed() / ITERATIONS as u32);
        }
    });

    let ops = nodes * 10; // roughly ten recorded calls per node
    println!(
        "\n[A] script → snapshot: {nodes} nodes ({ROWS}x{COLUMNS}) — {:.3} ms per build, \
         ~{} ns per recorded op ({ITERATIONS} iterations)",
        per_build.as_secs_f64() * 1000.0,
        per_build.as_nanos() as usize / ops.max(1),
    );

    // A smoke bound, not the real gate: the doc's 1.5 ms budget is for a
    // release build, and this assertion has to hold in debug too.
    assert!(
        per_build.as_millis() < 200,
        "describing {nodes} nodes took {per_build:?}, which is far outside any budget"
    );
}

#[gpui::test]
fn materializing_a_snapshot_stays_inside_the_frame_budget(cx: &mut TestAppContext) {
    let (runtime, mut context, object) = grid(cx, ROWS, COLUMNS);

    let snapshot = context.update(|window, cx| {
        runtime
            .build_snapshot(&object, None, crate::policy::default(), window, cx)
            .expect("render")
    });
    let nodes = snapshot.len();

    // This is the number that belongs to the frame budget: no VM runs here, and
    // it is what a repaint of an unchanged view actually costs.
    let per_materialize = time_materializations(&mut context, &runtime, &snapshot, ITERATIONS);

    println!(
        "\n[B] snapshot → elements: {nodes} nodes — {:.3} ms per materialization \
         ({ITERATIONS} iterations)",
        per_materialize.as_secs_f64() * 1000.0,
    );

    assert!(
        per_materialize.as_millis() < 200,
        "materializing {nodes} nodes took {per_materialize:?}, which is far outside any budget"
    );
}

#[gpui::test]
fn repainting_a_clean_view_never_enters_the_vm(cx: &mut TestAppContext) {
    let (runtime, mut context, object) = grid(cx, ROWS, COLUMNS);
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime.clone(), object)));

    draw(&mut context, &view);
    let after_first_frame = runtime.metrics().read().script_renders();

    let started = Instant::now();
    for _ in 0..ITERATIONS {
        draw(&mut context, &view);
    }
    let per_frame = started.elapsed() / ITERATIONS as u32;

    println!(
        "\n[C] cached frames: {ITERATIONS} repaints — {:.3} ms per frame, \
         {} script renders\n",
        per_frame.as_secs_f64() * 1000.0,
        runtime.metrics().read().script_renders() - after_first_frame,
    );

    assert_eq!(
        runtime.metrics().read().script_renders(),
        after_first_frame,
        "{ITERATIONS} repaints of a clean view entered the VM; script cost is back on the \
         frame budget"
    );
}

/// What all three costs do as a panel grows.
///
/// A single size cannot answer the question the design actually asks, because
/// two of these costs scale and one assertion does not. A, B and C all grow
/// roughly linearly with the node count — but the script render count stays at
/// zero at every size, which is the property the snapshot exists to buy. It is
/// ignored by default because the largest size costs seconds in a debug build:
///
/// ```text
/// cargo test -p gpui-shell --release --test benchmark -- --ignored --nocapture
/// ```
///
/// Two runs out of thirteen have seen the largest size report one script render
/// rather than zero, and it has not reproduced since — not under CPU load, not
/// with per-frame instrumentation, which is itself a hint that it is a timing
/// window. If it returns, the thing to instrument is `invalidate`: the deferred
/// drain in `scheduler::drain_after_render` is the only path that reaches a view
/// between frames, and the default-size test above has never failed.
#[gpui::test]
#[ignore = "walks panel sizes up to 8403 nodes; run explicitly in release"]
fn every_size_pays_the_script_only_when_it_changes(cx: &mut TestAppContext) {
    println!(
        "\n{:>6} | {:>12} | {:>12} | {:>12} | {}",
        "nodes", "A build", "B materialize", "C frame", "script renders"
    );

    for (rows, columns) in SIZES {
        let (runtime, mut context, object) = grid(cx, rows, columns);

        let nodes = context.update(|window, cx| {
            runtime
                .build_snapshot(&object, None, crate::policy::default(), window, cx)
                .expect("render")
                .len()
        });

        let started = Instant::now();
        let snapshot = context.update(|window, cx| {
            let mut last = None;
            for _ in 0..ITERATIONS {
                last = Some(
                    runtime
                        .build_snapshot(&object, None, crate::policy::default(), window, cx)
                        .expect("render"),
                );
            }
            last.expect("snapshot")
        });
        let per_build = started.elapsed() / ITERATIONS as u32;

        let per_materialize = time_materializations(&mut context, &runtime, &snapshot, ITERATIONS);

        let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime.clone(), object)));
        draw(&mut context, &view);
        let after_first_frame = runtime.metrics().read().script_renders();

        let started = Instant::now();
        for _ in 0..ITERATIONS {
            draw(&mut context, &view);
        }
        let per_frame = started.elapsed() / ITERATIONS as u32;
        let renders = runtime.metrics().read().script_renders() - after_first_frame;

        println!(
            "{nodes:>6} | {:>9.2} ms | {:>9.2} ms | {:>9.2} ms | {renders}",
            per_build.as_secs_f64() * 1000.0,
            per_materialize.as_secs_f64() * 1000.0,
            per_frame.as_secs_f64() * 1000.0,
        );

        assert_eq!(
            renders, 0,
            "{ITERATIONS} repaints of a clean {nodes}-node view entered the VM"
        );
    }
    println!();
}

fn time_materializations(
    context: &mut VisualTestContext,
    runtime: &std::rc::Rc<ShellRuntime>,
    snapshot: &RenderSnapshot,
    iterations: usize,
) -> std::time::Duration {
    // Elements are arena-allocated and only live inside a draw, so the timing
    // runs inside one — measuring materialization alone, with layout and paint
    // outside the clock.
    let mut elapsed = std::time::Duration::MAX;
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(800.), gpui::px(600.)),
        |window, cx| {
            // Best of several batches, for the reason given in [A].
            for _ in 0..ROUNDS {
                let started = Instant::now();
                for _ in 0..iterations {
                    let element = materialize(runtime, snapshot, window, cx);
                    std::hint::black_box(&element);
                }
                elapsed = elapsed.min(started.elapsed());
            }
            gpui::div().into_any_element()
        },
    );
    elapsed / iterations as u32
}

fn draw(context: &mut VisualTestContext, view: &Entity<ScriptView>) {
    let view = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(800.), gpui::px(600.)),
        move |_, _| view.into_any_element(),
    );
}

/// The grid application, instantiated in a window.
fn grid(
    cx: &mut TestAppContext,
    rows: usize,
    columns: usize,
) -> (
    std::rc::Rc<ShellRuntime>,
    VisualTestContext,
    crate::engine::ViewObject,
) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let view_type = runtime
        .load_source("grid", &source(rows, columns))
        .expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    (runtime, context, object)
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

// --- What one recorded call costs, and where the cost sits ------------------
//
// [A] prices a whole description. It does not say what to change to make one
// cheaper, because a description is a loop in script around a call that crosses
// into Rust, and those are two costs with two different remedies. [D] separates
// them by walking one call through progressively more of the generic path, then
// prints what the same call costs on the path the prelude actually binds.
//
// Each row is the whole path up to that point, so the difference between two
// adjacent rows is the cost of exactly the piece that row adds. The last row is
// not part of the walk: it is the shipped call, and the distance between it and
// `recorded` is what the dedicated entry points removed.

/// 20,000 recorded calls per round: a little over four times what the [A] panel
/// describes, and long enough that a round is milliseconds rather than
/// microseconds.
const BENCH_ELEMENTS: usize = 2_000;
const BENCH_PER_ELEMENT: usize = 10;

/// One prototype per stage, built over the same `function (...args)` shape the
/// prelude uses for behaviours — so what is timed is the real builder method
/// and not a hand-written approximation of it.
const BENCH_SETUP: &str = r#"
globalThis.__bench = (() => {
  const nothing = () => {};
  const method = (body) => {
    const methods = {};
    methods.m = body;
    return methods;
  };
  const generic = (target, name) =>
    method(function (...args) {
      target(this.__id, name, args);
      return this;
    });

  const nullaryIndex = __nullaryStyleIndexes[__nullaryStyles.indexOf("items_center")];
  const paramIndex = __paramStyles.indexOf("bg");

  const stages = {
    nullary: {
      js: generic(nothing, "items_center"),
      crossing: generic(__benchId, "items_center"),
      name: generic(__benchName, "items_center"),
      arguments: generic(__benchArgs, "items_center"),
      recorded: generic(__apply, "items_center"),
      shipped: method(function () {
        __applyNullaryStyle(this.__id, nullaryIndex);
        return this;
      }),
    },
    parametric: {
      js: generic(nothing, "bg"),
      crossing: generic(__benchId, "bg"),
      name: generic(__benchName, "bg"),
      arguments: generic(__benchArgs, "bg"),
      recorded: generic(__apply, "bg"),
      shipped: method(function (value) {
        __applyParamStyle(this.__id, paramIndex, value);
        return this;
      }),
    },
  };

  // The floor: the loop and the element, with no method call in it at all.
  const floor = (elements, per) => {
    for (let e = 0; e < elements; e += 1) {
      const object = Object.create(stages.nullary.js);
      object.__id = __div();
      for (let i = 0; i < per; i += 1) {
      }
    }
  };
  const bare = (methods, elements, per) => {
    for (let e = 0; e < elements; e += 1) {
      const object = Object.create(methods);
      object.__id = __div();
      for (let i = 0; i < per; i += 1) object.m();
    }
  };
  const valued = (methods, elements, per, value) => {
    for (let e = 0; e < elements; e += 1) {
      const object = Object.create(methods);
      object.__id = __div();
      for (let i = 0; i < per; i += 1) object.m(value);
    }
  };

  return { stages, floor, bare, valued };
})();
"#;

#[gpui::test]
fn one_recorded_call_is_priced_stage_by_stage(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx);
        // What a script render does on its way in. These stages run outside
        // every call scope, matching normal application render setup.
        crate::theme_tokens::sync(cx);
    });
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    runtime.eval_for_benchmark(BENCH_SETUP).expect("setup");

    let calls = BENCH_ELEMENTS * BENCH_PER_ELEMENT;
    let floor = time_stage(
        &runtime,
        &format!("__bench.floor({BENCH_ELEMENTS}, {BENCH_PER_ELEMENT})"),
    );
    let per_call = |elapsed: std::time::Duration| {
        elapsed.saturating_sub(floor).as_nanos() as f64 / calls as f64
    };

    println!(
        "\n[D] one recorded call, {calls} calls per round (best of {ROUNDS})\n\
         {:>11} | {:>12} | {:>12} | {}",
        "stage", "ns per call", "added", "what the step adds"
    );

    let mut shipped = Vec::new();
    for (family, argument, label) in [
        ("nullary", None, "items_center()"),
        ("parametric", Some("\"#f8f8f8\""), "bg(\"#f8f8f8\")"),
    ] {
        println!("{label}:");
        let mut previous = floor;
        for (stage, adds) in [
            ("js", "QuickJS interpreting the builder"),
            ("crossing", "the bare crossing into Rust"),
            ("name", "the method name as a Rust String"),
            (
                "arguments",
                "the argument list, in a JS array, as `Bridged`",
            ),
            ("recorded", "dispatch and the arena write"),
            ("shipped", "— what the prelude binds instead"),
        ] {
            let call = match argument {
                None => format!(
                    "__bench.bare(__bench.stages.{family}.{stage}, {BENCH_ELEMENTS}, \
                     {BENCH_PER_ELEMENT})"
                ),
                Some(value) => format!(
                    "__bench.valued(__bench.stages.{family}.{stage}, {BENCH_ELEMENTS}, \
                     {BENCH_PER_ELEMENT}, {value})"
                ),
            };
            let elapsed = time_stage(&runtime, &call);
            println!(
                "{stage:>11} | {:>9.0} ns | {:>9.0} ns | {adds}",
                per_call(elapsed),
                per_call(elapsed) - per_call(previous),
            );
            if stage == "shipped" {
                shipped.push((label, per_call(previous), per_call(elapsed)));
            }
            previous = elapsed;
        }
    }
    println!();

    // Not a threshold on any of the numbers — those are hardware — but on the
    // shape, which is the claim the table is making: the entry point the
    // prelude binds is the reason a style call is worth what it is worth, and
    // a change that undid it would leave the table saying so while every other
    // test still passed.
    for (label, generic, shipped) in shipped {
        assert!(
            shipped < generic,
            "`{label}` cost {shipped:.0} ns through its own entry point and {generic:.0} ns \
             through the generic one; the dedicated path has stopped paying for itself"
        );
    }
}

fn time_stage(runtime: &std::rc::Rc<ShellRuntime>, call: &str) -> std::time::Duration {
    // The first round pays for the arena's first growth and for QuickJS's
    // inline caches, neither of which a steady-state description pays.
    runtime.eval_for_benchmark(call).expect("stage");
    runtime.reset_arena_for_benchmark();

    let mut best = std::time::Duration::MAX;
    for _ in 0..ROUNDS {
        let started = Instant::now();
        runtime.eval_for_benchmark(call).expect("stage");
        best = best.min(started.elapsed());
        runtime.reset_arena_for_benchmark();
    }
    best
}
