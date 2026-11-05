//! The description depth bound: a chain a script builds one call at a time is
//! refused at the call that would cross it, whichever edge it grows through.

use gpui::{AppContext as _, TestAppContext};

use super::structure::script_object;
use crate::{ScriptView, spec::MAX_DESCRIPTION_DEPTH};

fn render(cx: &mut TestAppContext, body: &str) -> Result<String, String> {
    let source = format!(
        r#"
import {{ View, div }} from "gpui-kit";
import {{ v_flex, Button, Collapsible }} from "gpui-base";
const template = globalThis.__template;

export default class Deep extends View {{
  render() {{
{body}
  }}
}}
"#
    );
    let (runtime, mut context, object) = script_object(cx, &source);
    context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .map_err(|error| error.to_string())
    })
}

/// Builds a chain until a call throws, and answers how many levels stood and
/// what the throw said.
fn refused_at(cx: &mut TestAppContext, setup: &str, step: &str) -> (usize, String) {
    let tree = render(
        cx,
        &format!(
            r#"
    {setup}
    let levels = 1;
    try {{
      for (let i = 0; i < 100000; i++) {{
        {step}
        levels += 1;
      }}
      return div().child("never refused");
    }} catch (error) {{
      return div().child(`${{levels}}|${{error.message}}`);
    }}
"#
        ),
    )
    .expect("a refused call is a script error the script can catch");
    let line = tree
        .lines()
        .find_map(|line| line.trim().strip_prefix("text \""))
        .expect("the outcome text")
        .trim_end_matches('"');
    let (levels, message) = line.split_once('|').expect("levels|message");
    (levels.parse().expect("a count"), message.to_owned())
}

fn assert_depth_error(message: &str) {
    assert!(
        message.contains(&format!("more than {MAX_DESCRIPTION_DEPTH} levels")),
        "the error must name the bound: {message}"
    );
}

#[gpui::test]
fn a_child_chain_is_refused_at_the_call_that_crosses_the_bound(cx: &mut TestAppContext) {
    let (levels, message) = refused_at(cx, "let node = div();", "node = div().child(node);");

    assert_eq!(levels, usize::from(MAX_DESCRIPTION_DEPTH));
    assert_depth_error(&message);
}

#[gpui::test]
fn a_slot_chain_is_refused_at_the_call_that_crosses_the_bound(cx: &mut TestAppContext) {
    let (levels, message) = refused_at(
        cx,
        "let node = div();",
        "node = Collapsible.new().open(true).content(node);",
    );

    assert_eq!(levels, usize::from(MAX_DESCRIPTION_DEPTH));
    assert_depth_error(&message);
}

/// An element given to a slot still takes children afterwards, so a chain can
/// grow downward from an element that is already held. Each new level raises
/// every holder above it.
#[gpui::test]
fn a_chain_grown_below_a_filled_slot_is_refused_at_the_bound(cx: &mut TestAppContext) {
    let (levels, message) = refused_at(
        cx,
        "const root = Collapsible.new().open(true); let tip = root;",
        "const next = Collapsible.new().open(true); tip.content(next); tip = next;",
    );

    assert_eq!(levels, usize::from(MAX_DESCRIPTION_DEPTH));
    assert_depth_error(&message);
}

/// A refused call leaves nothing behind: the element it was given is still
/// free, and the tree it would have joined is unchanged.
#[gpui::test]
fn a_refused_call_records_nothing(cx: &mut TestAppContext) {
    let tree = render(
        cx,
        &format!(
            r#"
    let chain = div();
    for (let i = 1; i < {MAX_DESCRIPTION_DEPTH}; i++) chain = div().child(chain);
    let refused = 0;
    try {{ Collapsible.new().content(chain); }} catch {{ refused += 1; }}
    try {{ div().child(chain); }} catch {{ refused += 1; }}
    return chain.id(`refused-${{refused}}`);
"#
        ),
    )
    .expect("the chain both calls were refused is still the script's to return");

    assert!(tree.starts_with("div :id[Str(\"refused-2\")]"), "{tree}");
    assert_eq!(tree.lines().count(), usize::from(MAX_DESCRIPTION_DEPTH));
    assert!(!tree.contains("Collapsible"), "{tree}");
}

#[gpui::test]
fn an_element_cannot_be_slotted_inside_its_own_content(cx: &mut TestAppContext) {
    let tree = render(
        cx,
        r#"
    const outer = Collapsible.new().open(true);
    const inner = div();
    outer.content(inner);
    const messages = [];
    try { inner.child(outer); } catch (error) { messages.push(error.message); }
    try { outer.content(outer); } catch (error) { messages.push(error.message); }
    return div().child(messages.join("|"));
"#,
    )
    .expect("render");

    assert_eq!(
        tree.matches("cannot be placed inside itself").count(),
        2,
        "both cycles must be refused: {tree}"
    );
}

/// A grafted template root carries its height, so attaching it counts the
/// levels inside the template.
#[gpui::test]
fn a_grafted_template_counts_its_own_levels(cx: &mut TestAppContext) {
    let (levels, message) = refused_at(
        cx,
        r#"
    const Leaf = template((label) =>
      div().child(div().child(div().child(div().child(label)))));
    let node = Leaf("leaf");"#,
        "node = div().child(node);",
    );

    // The template is five levels: four divs and the text.
    assert_eq!(levels, usize::from(MAX_DESCRIPTION_DEPTH) - 4);
    assert_depth_error(&message);
}

#[gpui::test]
fn a_template_body_is_bounded_too(cx: &mut TestAppContext) {
    let tree = render(
        cx,
        r#"
    try {
      const Deep = template(() => {
        let chain = div();
        for (;;) chain = div().child(chain);
      });
      Deep();
      return div().child("never refused");
    } catch (error) {
      return div().child(error.message);
    }
"#,
    )
    .expect("render");

    assert_depth_error(&tree);
}

/// The bound is only safe if a description at it can be drawn. The test
/// thread's stack is 2 MiB, a quarter of a macOS main thread's, and this is an
/// unoptimized build. `Checkbox` is the costliest level measured; the slot
/// keeps the other kind of edge in the chain.
#[gpui::test]
fn a_description_at_the_bound_is_drawn(cx: &mut TestAppContext) {
    let source = format!(
        r#"
import {{ View, div }} from "gpui-kit";
import {{ Checkbox, Collapsible }} from "gpui-base";

export default class Deep extends View {{
  render() {{
    let node = div().child("leaf");
    for (let level = 3; level <= {MAX_DESCRIPTION_DEPTH}; level++) {{
      node = level % 4 === 0
        ? Collapsible.new().open(true).content(node)
        : Checkbox.new(`checkbox-${{level}}`).child(node);
    }}
    return node;
  }}
}}
"#
    );
    let (runtime, mut context, object) = script_object(cx, &source);
    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("a description at the bound renders")
    });
    assert!(tree.contains("text \"leaf\""), "{tree}");

    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| gpui::IntoElement::into_any_element(view),
    );
}
