//! `template(build)`: a description recorded once and filled per call.
//!
//! The behaviour these pin is the contract of `engine/quickjs/template.rs` —
//! that a template describes exactly what the same builder chain would
//! describe, that filling it changes values and never structure, and that the
//! four things a template cannot hold are refused where they were written
//! rather than baked in.

use gpui::TestAppContext;

use super::structure::script_object;

fn tree(cx: &mut TestAppContext, source: &str) -> Result<String, String> {
    let (runtime, mut context, object) = script_object(cx, source);
    context.update(|window, cx| {
        runtime
            .build_snapshot(&object, None, crate::policy::default(), window, cx)
            .map(|snapshot| snapshot.debug_tree())
            .map_err(|error| error.to_string())
    })
}

const INLINE: &str = r#"
import { View, div } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";

export default class Board extends View {
  render() {
    const rows = [["AAPL", "230.42"], ["MSFT", "410.08"]];
    return v_flex().gap(4).children(rows.map(([symbol, price]) =>
      h_flex().gap(6).px(6)
        .child(div().w(80).child(symbol))
        .child(div().w(80).child(price))));
  }
}
"#;

const TEMPLATED: &str = r#"
import { View, div } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
const template = globalThis.__template;

const Row = template((symbol, price) =>
  h_flex().gap(6).px(6)
    .child(div().w(80).child(symbol))
    .child(div().w(80).child(price)));

export default class Board extends View {
  render() {
    const rows = [["AAPL", "230.42"], ["MSFT", "410.08"]];
    return v_flex().gap(4).children(rows.map(([symbol, price]) => Row(symbol, price)));
  }
}
"#;

#[gpui::test]
fn a_template_describes_what_the_builder_chain_describes(cx: &mut TestAppContext) {
    let inline = tree(cx, INLINE).expect("the inline board renders");
    let templated = tree(cx, TEMPLATED).expect("the templated board renders");
    assert_eq!(
        inline, templated,
        "a filled template must be indistinguishable from the chain it replaces"
    );
}

#[gpui::test]
fn a_style_argument_and_a_handler_are_slots_too(cx: &mut TestAppContext) {
    let tree = tree(
        cx,
        r#"
import { View, div } from "gpui-kit";
import { v_flex, Button } from "gpui-base";
const template = globalThis.__template;

const Row = template((color, label, onPick) =>
  Button.new("pick").bg(color).on_click(onPick).child(label));

export default class Board extends View {
  render() {
    return v_flex()
      .child(Row(`#f8f8f8`, "one", () => 1))
      .child(Row(`#2563eb`, "two", () => 2));
  }
}
"#,
    )
    .expect("the board renders");

    assert!(
        tree.contains(r##".bg[Str("#f8f8f8")]"##) && tree.contains(r##".bg[Str("#2563eb")]"##),
        "each call must write its own style argument: {tree}"
    );
    assert!(
        tree.contains(r#"text "one""#) && tree.contains(r#"text "two""#),
        "each call must write its own text: {tree}"
    );
}

/// Two calls must not share a handler, and a second render must not reuse the
/// first render's.
///
/// This is the property that makes a template safe rather than the one that
/// makes it fast: a callback belongs to the snapshot that registered it and is
/// retired with it, so a template holding one would hand a retired id to every
/// call that followed.
#[gpui::test]
fn every_call_mints_its_own_handler(cx: &mut TestAppContext) {
    let (runtime, mut context, object) = script_object(
        cx,
        r#"
import { View } from "gpui-kit";
import { v_flex, Button } from "gpui-base";
const template = globalThis.__template;

const Row = template((label, onPick) => Button.new("pick").on_click(onPick).child(label));

export default class Board extends View {
  render() {
    return v_flex().child(Row("one", () => 1)).child(Row("two", () => 2));
  }
}
"#,
    );

    let (first, second) = context.update(|window, cx| {
        let mut callbacks = || {
            let snapshot = runtime
                .build_snapshot(&object, None, crate::policy::default(), window, cx)
                .expect("render");
            let arena = snapshot.arena();
            (0..arena.len() as u32)
                .filter_map(|id| arena.node(id))
                .flat_map(crate::spec::SpecNode::ops)
                .filter_map(|op| match op {
                    crate::spec::SpecOp::Callback("on_click", callback) => Some(*callback),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        (callbacks(), callbacks())
    });

    assert_eq!(first.len(), 2, "one handler per call: {first:?}");
    assert_ne!(first[0], first[1], "two calls must not share a handler");
    assert!(
        first.iter().all(|id| !second.contains(id)),
        "a second render must mint its own: {first:?} against {second:?}"
    );
}

#[gpui::test]
fn a_template_argument_may_not_be_computed_on(cx: &mut TestAppContext) {
    let error = tree(
        cx,
        r#"
import { View, div } from "gpui-kit";
const template = globalThis.__template;
const Row = template((price) => div().child(`$${price}`));
export default class Board extends View {
  render() { return Row("230.42"); }
}
"#,
    )
    .expect_err("a formatted sentinel must be refused");

    assert!(
        error.contains("passed to a builder call but not computed on"),
        "the refusal must name the rule: {error}"
    );
}

#[gpui::test]
fn a_body_may_not_register_its_own_handler(cx: &mut TestAppContext) {
    let error = tree(
        cx,
        r#"
import { View } from "gpui-kit";
import { Button } from "gpui-base";
const template = globalThis.__template;
const Row = template((label) => Button.new("pick").on_click(() => 1).child(label));
export default class Board extends View {
  render() { return Row("one"); }
}
"#,
    )
    .expect_err("an inline handler must be refused");

    assert!(
        error.contains("on_click") && error.contains("Take the handler as a parameter"),
        "the refusal must say what to do instead: {error}"
    );
}

#[gpui::test]
fn a_parameter_that_fills_nothing_is_refused(cx: &mut TestAppContext) {
    let error = tree(
        cx,
        r#"
import { View, div } from "gpui-kit";
const template = globalThis.__template;
const Row = template((symbol, unused) => div().child(symbol));
export default class Board extends View {
  render() { return Row("AAPL", "ignored"); }
}
"#,
    )
    .expect_err("an unused parameter must be refused");

    assert!(
        error.contains("template argument 1 is never used"),
        "the refusal must name the parameter: {error}"
    );
}

#[gpui::test]
fn a_template_body_may_not_use_another_template(cx: &mut TestAppContext) {
    let error = tree(
        cx,
        r#"
import { View, div } from "gpui-kit";
const template = globalThis.__template;
const Cell = template((value) => div().child(value));
const Row = template((value) => div().child(Cell(value)));
export default class Board extends View {
  render() { return Row("AAPL"); }
}
"#,
    )
    .expect_err("nesting must be refused");

    assert!(
        error.contains("template body cannot"),
        "the refusal must name the rule: {error}"
    );
}

#[gpui::test]
fn a_slot_in_a_position_a_template_cannot_fill_is_refused(cx: &mut TestAppContext) {
    let error = tree(
        cx,
        r#"
import { View } from "gpui-kit";
import { Checkbox } from "gpui-base";
const template = globalThis.__template;
const Row = template((flag) => Checkbox.new("pick").disabled(flag));
export default class Board extends View {
  render() { return Row(true); }
}
"#,
    )
    .expect_err("an unsupported slot position must be refused");

    assert!(
        error.contains("disabled") && error.contains("text children, style arguments and handlers"),
        "the refusal must name what a template does fill: {error}"
    );
}

#[gpui::test]
fn calling_a_template_with_the_wrong_number_of_arguments_is_refused(cx: &mut TestAppContext) {
    let error = tree(
        cx,
        r#"
import { View, div } from "gpui-kit";
const template = globalThis.__template;
const Row = template((symbol, price) => div().child(symbol).child(price));
export default class Board extends View {
  render() { return Row("AAPL"); }
}
"#,
    )
    .expect_err("the wrong arity must be refused");

    assert!(
        error.contains("takes 2 argument(s) and was given 1"),
        "the refusal must say what it expected: {error}"
    );
}

#[gpui::test]
fn a_bad_style_argument_still_reports_at_the_call(cx: &mut TestAppContext) {
    let error = tree(
        cx,
        r#"
import { View, div } from "gpui-kit";
const template = globalThis.__template;
const Row = template((color) => div().bg(color));
export default class Board extends View {
  render() { return Row("not-a-color"); }
}
"#,
    )
    .expect_err("an invalid colour must be refused");

    assert!(
        error.contains("not-a-color") || error.contains("color"),
        "the check the ordinary path runs must still run: {error}"
    );
}

/// What a template costs against the chain it replaces, on the panel §20.7's
/// census measured.
///
/// Both boards describe the same forty rows with the same four cells and the
/// same handler; one runs the builder for every one of them, the other grafts
/// a structure recorded once and writes eighty values into it.
#[gpui::test]
fn a_templated_panel_against_the_chain_it_replaces(cx: &mut TestAppContext) {
    const ITERATIONS: usize = 50;
    const ROUNDS: usize = 7;

    let inline = time_board(cx, INLINE_WATCHLIST, ITERATIONS, ROUNDS);
    let templated = time_board(cx, TEMPLATED_WATCHLIST, ITERATIONS, ROUNDS);

    println!(
        "\n[H] template against builder chain — 40-row watchlist\
         \n    builder chain   {:.3} ms\
         \n    template        {:.3} ms\
         \n    ratio           {:.1}x",
        inline.as_secs_f64() * 1000.0,
        templated.as_secs_f64() * 1000.0,
        inline.as_secs_f64() / templated.as_secs_f64().max(f64::MIN_POSITIVE),
    );

    // A smoke bound, not the reading: the printed number is release-build
    // information and this assertion also has to hold in a debug build.
    assert!(
        templated < inline,
        "grafting a recorded structure should not cost more than describing it: \
         {templated:?} against {inline:?}"
    );
}

fn time_board(
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

const INLINE_WATCHLIST: &str = r#"
import { View, div } from "gpui-kit";
import { v_flex, h_flex, Button } from "gpui-base";

export default class Watchlist extends View {
  init() { this.tick = 0; }

  row(index, price) {
    return h_flex().gap(6).py(2).px(6).rounded(4).bg(`#f8f8f8`)
      .child(div().w(80).text_sm().text_color(`#111111`).child(`SYM${index}`))
      .child(div().w(80).text_sm().text_color(`#111111`).child(price))
      .child(div().w(60).text_sm().text_color(`#6b7280`).child("+1.42%"))
      .child(Button.new("trade").px(8).py(2).on_click(() => index).child("Trade"));
  }

  render() {
    this.tick += 1;
    const rows = [];
    for (let index = 0; index < 40; index += 1) {
      rows.push(this.row(index, (100 + index + this.tick / 100).toFixed(2)));
    }
    return v_flex().size_full().p(12).gap(4).bg(`#ffffff`).children(rows);
  }
}
"#;

const TEMPLATED_WATCHLIST: &str = r#"
import { View, div } from "gpui-kit";
import { v_flex, h_flex, Button } from "gpui-base";
const template = globalThis.__template;

const Row = template((symbol, price, change, onTrade) =>
  h_flex().gap(6).py(2).px(6).rounded(4).bg(`#f8f8f8`)
    .child(div().w(80).text_sm().text_color(`#111111`).child(symbol))
    .child(div().w(80).text_sm().text_color(`#111111`).child(price))
    .child(div().w(60).text_sm().text_color(`#6b7280`).child(change))
    .child(Button.new("trade").px(8).py(2).on_click(onTrade).child("Trade")));

export default class Watchlist extends View {
  init() { this.tick = 0; }

  render() {
    this.tick += 1;
    const rows = [];
    for (let index = 0; index < 40; index += 1) {
      rows.push(Row(
        `SYM${index}`,
        (100 + index + this.tick / 100).toFixed(2),
        "+1.42%",
        () => index,
      ));
    }
    return v_flex().size_full().p(12).gap(4).bg(`#ffffff`).children(rows);
  }
}
"#;

/// A template does not have to be a row. This one is the whole view.
#[gpui::test]
fn a_whole_view_can_be_one_template(cx: &mut TestAppContext) {
    let tree = tree(
        cx,
        r#"
import { View, div } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
const template = globalThis.__template;

const Panel = template((title, price, change) =>
  v_flex().gap(4).p(12).bg(`#ffffff`)
    .child(div().text_sm().child(title))
    .child(h_flex().gap(6)
      .child(div().w(80).child(price))
      .child(div().w(60).child(change))));

export default class Detail extends View {
  init() { this.price = "230.42"; }
  render() { return Panel("Apple Inc.", this.price, "+1.42%"); }
}
"#,
    )
    .expect("a whole-view template renders");

    assert!(
        tree.contains(r#"text "Apple Inc.""#) && tree.contains(r#"text "230.42""#),
        "{tree}"
    );
}

/// How much of a real panel an *automatic* template cache could reach.
///
/// Nothing in the runtime templates anything on its own yet, so this measures
/// the ceiling by hand: the same board built twice, once with its presentation
/// helpers as plain functions and once with the helpers a rule-following
/// wrapper could safely template.
///
/// The helper shapes are taken from the Shell story's own `ui.js`, which was
/// written years before this question and is the closest thing here to code
/// nobody wrote for the benchmark. Its split is the interesting part:
///
/// * `title`, `label`, `muted`, `rule` — one varying value handed straight to
///   a builder call. A wrapper could template these without the author
///   knowing.
/// * `cell(width, options)`, `watchMarker(watched)`, `action(…, {primary})` —
///   an argument decides *structure*, through a ternary or a `when`. A sentinel
///   would be read and never land, which is what a wrapper must detect and
///   fall back on.
/// * `quoteRow` — templatable in shape, since its handler already arrives as a
///   parameter, but it calls the helpers above, so it needs templates to nest.
///
/// So this measures the first group only, which is the part an automatic
/// wrapper could take today with no authoring change and no risk.
#[gpui::test]
fn what_automatic_templating_of_the_safe_helpers_would_buy(cx: &mut TestAppContext) {
    const ITERATIONS: usize = 50;
    const ROUNDS: usize = 7;

    let plain = time_board(cx, BOARD_PLAIN, ITERATIONS, ROUNDS);
    let templated = time_board(cx, BOARD_TEMPLATED_HELPERS, ITERATIONS, ROUNDS);

    println!(
        "\n[I] automatic templating of leaf helpers — 20-row board, 6 cells\
         \n    helpers as plain functions   {:.3} ms\
         \n    helpers templated            {:.3} ms\
         \n    ratio                        {:.2}x",
        plain.as_secs_f64() * 1000.0,
        templated.as_secs_f64() * 1000.0,
        plain.as_secs_f64() / templated.as_secs_f64().max(f64::MIN_POSITIVE),
    );

    assert!(
        templated < plain,
        "templating the leaves should not cost more than describing them: \
         {templated:?} against {plain:?}"
    );
}

/// The story's `ui.js` shape: leaf helpers, a row that composes them, a header.
const BOARD_PLAIN: &str = r#"
import { View, div } from "gpui-kit";
import { v_flex, h_flex, Button } from "gpui-base";

const label = (value) =>
  div().text_size("0.6875rem").line_height(1.4).text_color(`#111111`).child(value);
const muted = (value) =>
  div().text_size("0.6875rem").line_height(1.4).text_color(`#6b7280`).child(value);
const cell = (width, right) => {
  const box = div().w(width).flex_none();
  return right ? box.text_right() : box;
};

const row = (quote, onClick) =>
  Button.new(`quote-${quote.symbol}`)
    .flex().w_full().items_center().gap("0.5rem").px("0.5rem").py("0.125rem")
    .on_click(onClick)
    .child(cell("4.875rem", false).child(label(quote.symbol)))
    .child(div().flex_1().child(muted(quote.name)))
    .child(cell("4.25rem", true).child(label(quote.last)))
    .child(cell("4.125rem", true).child(label(quote.percent)))
    .child(cell("5.125rem", true).child(muted(quote.volume)))
    .child(div().w("0.375rem").h("0.375rem").flex_none());

export default class Board extends View {
  init() { this.tick = 0; }

  render() {
    this.tick += 1;
    const rows = [];
    for (let index = 0; index < 20; index += 1) {
      rows.push(row({
        symbol: `SYM${index}`,
        name: `Instrument ${index}`,
        last: (100 + index + this.tick / 100).toFixed(2),
        percent: "+1.42%",
        volume: "1.2M",
      }, () => index));
    }
    return v_flex().w_full().gap("0.75rem")
      .child(h_flex().w_full().gap("0.5rem")
        .child(cell("4.875rem", false).child(muted("Symbol")))
        .child(div().flex_1())
        .child(cell("4.25rem", true).child(muted("Last")))
        .child(cell("4.125rem", true).child(muted("Change")))
        .child(cell("5.125rem", true).child(muted("Volume"))))
      .children(rows);
  }
}
"#;

/// The same board, with only the helpers a wrapper could safely take.
const BOARD_TEMPLATED_HELPERS: &str = r#"
import { View, div } from "gpui-kit";
import { v_flex, h_flex, Button } from "gpui-base";

const template = globalThis.__template;

const label = template((value) =>
  div().text_size("0.6875rem").line_height(1.4).text_color(`#111111`).child(value));
const muted = template((value) =>
  div().text_size("0.6875rem").line_height(1.4).text_color(`#6b7280`).child(value));
// Not templated: `right` decides structure, so a sentinel would be read and
// never land — the case a wrapper has to detect and fall back on.
const cell = (width, right) => {
  const box = div().w(width).flex_none();
  return right ? box.text_right() : box;
};

const row = (quote, onClick) =>
  Button.new(`quote-${quote.symbol}`)
    .flex().w_full().items_center().gap("0.5rem").px("0.5rem").py("0.125rem")
    .on_click(onClick)
    .child(cell("4.875rem", false).child(label(quote.symbol)))
    .child(div().flex_1().child(muted(quote.name)))
    .child(cell("4.25rem", true).child(label(quote.last)))
    .child(cell("4.125rem", true).child(label(quote.percent)))
    .child(cell("5.125rem", true).child(muted(quote.volume)))
    .child(div().w("0.375rem").h("0.375rem").flex_none());

export default class Board extends View {
  init() { this.tick = 0; }

  render() {
    this.tick += 1;
    const rows = [];
    for (let index = 0; index < 20; index += 1) {
      rows.push(row({
        symbol: `SYM${index}`,
        name: `Instrument ${index}`,
        last: (100 + index + this.tick / 100).toFixed(2),
        percent: "+1.42%",
        volume: "1.2M",
      }, () => index));
    }
    return v_flex().w_full().gap("0.75rem")
      .child(h_flex().w_full().gap("0.5rem")
        .child(cell("4.875rem", false).child(muted("Symbol")))
        .child(div().flex_1())
        .child(cell("4.25rem", true).child(muted("Last")))
        .child(cell("4.125rem", true).child(muted("Change")))
        .child(cell("5.125rem", true).child(muted("Volume"))))
      .children(rows);
  }
}
"#;

/// A state style is a detached node the operation points at by id, so grafting
/// a template has to move that id along with everything else.
///
/// Nothing above catches this: every other test's template is flat, and a
/// wrongly remapped id would not fail — it would point at whatever node landed
/// land at that index, and the second instance would style the first one's
/// interior.
#[gpui::test]
fn a_state_style_survives_being_grafted_twice(cx: &mut TestAppContext) {
    let tree = tree(
        cx,
        r#"
import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";

const template = globalThis.__template;

const Row = template((label) =>
  div().bg(`#f8f8f8`).hover((style) => style.bg(`#f3f4f6`)).child(label));

export default class Board extends View {
  render() { return v_flex().child(Row("one")).child(Row("two")); }
}
"#,
    )
    .expect("the board renders");

    assert_eq!(
        tree.matches("hover").count(),
        2,
        "each instance needs a hover style of its own: {tree}"
    );
    assert_eq!(
        tree.matches(r##"bg[Str("#f3f4f6")]"##).count(),
        2,
        "and each hover style needs its own declarations: {tree}"
    );
    assert!(
        tree.contains(r#"text "one""#) && tree.contains(r#"text "two""#),
        "the slots still fill: {tree}"
    );
}

/// The same for a named slot, which is the other operation carrying a `SpecId`.
#[gpui::test]
fn a_named_slot_survives_being_grafted_twice(cx: &mut TestAppContext) {
    let tree = tree(
        cx,
        r#"
import { View, div } from "gpui-kit";
import { v_flex, Collapsible } from "gpui-base";

const template = globalThis.__template;

const Panel = template((label, body) =>
  Collapsible.new().child(div().child(label)).content(div().child(body)));

export default class Board extends View {
  render() { return v_flex().child(Panel("one", "first")).child(Panel("two", "second")); }
}
"#,
    )
    .expect("the board renders");

    assert_eq!(
        tree.matches("content").count(),
        2,
        "each instance needs its own filled slot: {tree}"
    );
    assert!(
        tree.contains(r#"text "first""#) && tree.contains(r#"text "second""#),
        "and each slot's content must be its own: {tree}"
    );
}

/// A body that throws must leave the description it interrupted exactly where
/// it was.
///
/// Discovery swaps the arena the live render is recording into for an empty
/// one, so a body that throws half-way through has to put the original back.
/// If it did not, the rest of that render would be recorded into an arena that
/// is about to be discarded, and the view would draw nothing or the wrong
/// thing — with no error to say why.
#[gpui::test]
fn a_body_that_throws_leaves_the_render_it_interrupted_intact(cx: &mut TestAppContext) {
    let tree = tree(
        cx,
        r#"
import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";

const template = globalThis.__template;

// Formatting the argument consumes the sentinel, so this body throws.
const Broken = template((value) => div().child(`value: ${value}`));

export default class Board extends View {
  render() {
    const parts = v_flex().child(div().child("before"));
    try {
      parts.child(Broken("x"));
    } catch (error) {
      parts.child(div().child("caught"));
    }
    return parts.child(div().child("after"));
  }
}
"#,
    )
    .expect("the render must survive a template body that threw");

    assert!(
        tree.contains(r#"text "before""#)
            && tree.contains(r#"text "caught""#)
            && tree.contains(r#"text "after""#),
        "everything described around the failed body must still be there: {tree}"
    );
}
