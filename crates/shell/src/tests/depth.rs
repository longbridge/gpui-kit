//! The description depth bound: a chain a script builds one call at a time is
//! refused at the call that would cross it, whichever edge it grows through.
//! And the nesting bound: views mounted inside one another draw nothing where
//! they would recurse or nest past it.

use gpui::{AppContext as _, TestAppContext};

use super::structure::script_object;
use crate::{
    ScriptView,
    spec::MAX_DESCRIPTION_DEPTH,
    view::{MAX_NESTED_VIEW_DEPTH, NestingRefusal},
};

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

/// A source whose default export is `Root`, with every import these tests use.
fn views(body: &str) -> String {
    format!(
        r#"
import {{ View, div }} from "gpui-kit";
import {{ Checkbox, DockArea, dock_area }} from "gpui-base";
{body}
"#
    )
}

/// Mounting two views inside each other used to recurse until the process
/// aborted: from the second frame both snapshots are current, so layout
/// reaches neither the script nor its stack limit.
#[gpui::test]
fn views_mounted_inside_each_other_draw_the_inner_mount_as_nothing(cx: &mut TestAppContext) {
    let (runtime, mut context, root) = mount(
        cx,
        &views(
            r#"
class A extends View { render() { return div().child("a").child(globalThis.b); } }
class B extends View { render() { return div().child("b").child(globalThis.a); } }
export default class Root extends View {
  init(_props, cx) { globalThis.a = cx.new(A); globalThis.b = cx.new(B); }
  render() { return div().child(globalThis.a); }
}
"#,
        ),
    );
    draw(&mut context, &root);
    draw(&mut context, &root);

    context.update(|_, cx| {
        let a = mounted(&root, cx).expect("a");
        let b = mounted(&a, cx).expect("b");
        assert_eq!(mounted(&b, cx).as_ref(), Some(&a));
        assert_eq!(
            a.read(cx).nesting_refusal(),
            Some(NestingRefusal::Reentered)
        );
        assert_eq!(b.read(cx).nesting_refusal(), None);
    });
    assert_eq!(runtime.metrics().read().script_renders(), 3);
}

#[gpui::test]
fn a_view_mounted_inside_itself_draws_the_inner_mount_as_nothing(cx: &mut TestAppContext) {
    let (runtime, mut context, root) = mount(
        cx,
        &views(
            r#"
class A extends View { render() { return div().child("a").child(globalThis.a); } }
export default class Root extends View {
  init(_props, cx) { globalThis.a = cx.new(A); }
  render() { return div().child(globalThis.a); }
}
"#,
        ),
    );
    draw(&mut context, &root);
    draw(&mut context, &root);

    context.update(|_, cx| {
        let a = mounted(&root, cx).expect("a");
        assert_eq!(mounted(&a, cx).as_ref(), Some(&a));
        assert_eq!(
            a.read(cx).nesting_refusal(),
            Some(NestingRefusal::Reentered)
        );
    });
    assert_eq!(runtime.metrics().read().script_renders(), 2);
}

/// A dock panel is mounted by the dock, not by a child view, and its own
/// description can name the dock that holds it.
#[gpui::test]
fn a_panel_that_mounts_its_own_dock_draws_the_inner_mount_as_nothing(cx: &mut TestAppContext) {
    let (runtime, mut context, root) = mount(
        cx,
        &views(
            r#"
class Panel extends View {
  render() { return div().size_full().child(dock_area(globalThis.dock)); }
}
export default class Root extends View {
  init(_props, cx) {
    DockArea.register_panel("panel", Panel);
    globalThis.dock = DockArea.new("cycle");
    globalThis.dock.add_panel(cx.new(Panel), { name: "panel" });
  }
  render() { return div().size_full().child(dock_area(globalThis.dock)); }
}
"#,
        ),
    );
    draw(&mut context, &root);
    draw(&mut context, &root);

    assert_eq!(runtime.metrics().read().script_renders(), 2);
}

/// A chain of views long enough to exhaust the stack is cut where the views
/// above it reach the bound. The view past the cut is never drawn.
#[gpui::test]
fn a_chain_of_views_is_cut_at_the_nesting_bound(cx: &mut TestAppContext) {
    let (_runtime, mut context, root) = mount(
        cx,
        &views(
            r#"
class Link extends View {
  init(props) { this.index = props.index; }
  render() {
    const next = globalThis.chain[this.index + 1];
    return next ? div().child(next) : div().child("end");
  }
}
export default class Root extends View {
  init(_props, cx) {
    globalThis.chain = [];
    for (let i = 0; i < 500; i++) globalThis.chain.push(cx.new(Link, { index: i }));
  }
  render() { return div().child(globalThis.chain[0]); }
}
"#,
        ),
    );
    draw(&mut context, &root);
    draw(&mut context, &root);

    // Every view here is a div and a child view: two levels, and one for itself.
    let drawn = (MAX_NESTED_VIEW_DEPTH / 3) as usize;
    context.update(|_, cx| {
        let mut chain = vec![root.clone()];
        while let Some(next) = mounted(chain.last().expect("root"), cx) {
            chain.push(next);
        }
        for view in &chain[..drawn] {
            assert_eq!(view.read(cx).nesting_refusal(), None);
        }
        assert_eq!(
            chain[drawn].read(cx).nesting_refusal(),
            Some(NestingRefusal::TooDeep)
        );
        assert_eq!(chain.len(), drawn + 2);
        assert!(chain[drawn + 1].read(cx).snapshot().is_none());
    });
}

/// The nesting bound is only safe if views at it can be drawn. The costliest
/// level measured is a nested `Checkbox`, and here it fills the bound across
/// three views. Each view is built alone first, so the draw measures layout
/// rather than the interpreter's own stack limit, which a script render this
/// deep in an unoptimized build would reach.
#[gpui::test]
fn views_at_the_nesting_bound_are_drawn(cx: &mut TestAppContext) {
    // The root is two levels and its own, the first link forty and its own.
    let second = MAX_NESTED_VIEW_DEPTH - 3 - 41 - 1;
    let (_runtime, mut context, root) = mount(
        cx,
        &views(&format!(
            r#"
class Link extends View {{
  init(props) {{ this.index = props.index; }}
  render() {{
    const next = globalThis.chain[this.index + 1];
    let node = next ? div().child(next) : div().child("end");
    const levels = [40, {second}, 2][this.index];
    for (let level = 3; level <= levels; level++) {{
      node = Checkbox.new(`checkbox-${{level}}`).child(node);
    }}
    return node;
  }}
}}
export default class Root extends View {{
  init(_props, cx) {{
    globalThis.chain = [0, 1, 2].map((index) => cx.new(Link, {{ index }}));
  }}
  render() {{ return div().child(globalThis.chain[0]); }}
}}
"#
        )),
    );

    let mut chain = vec![root.clone()];
    loop {
        let view = chain.last().expect("root").clone();
        context.update(|window, cx| {
            view.update(cx, |view, cx| drop(gpui::Render::render(view, window, cx)))
        });
        match context.update(|_, cx| mounted(&view, cx)) {
            Some(next) => chain.push(next),
            None => break,
        }
    }
    assert_eq!(chain.len(), 4);

    draw(&mut context, &root);

    context.update(|_, cx| {
        for view in &chain[..3] {
            assert_eq!(view.read(cx).nesting_refusal(), None);
        }
        assert_eq!(
            chain[3].read(cx).nesting_refusal(),
            Some(NestingRefusal::TooDeep)
        );
    });
}

/// One view under two parents in the same frame is not a recursion: neither
/// mount encloses the other, so both draw it. Each description still mounts a
/// view once.
#[gpui::test]
fn a_view_under_two_parents_is_drawn_under_both(cx: &mut TestAppContext) {
    let (runtime, mut context, root) = mount(
        cx,
        &views(
            r#"
class Shared extends View { render() { return div().child("shared"); } }
class Holder extends View { render() { return div().child(globalThis.shared); } }
export default class Root extends View {
  init(_props, cx) {
    globalThis.shared = cx.new(Shared);
    this.left = cx.new(Holder);
    this.right = cx.new(Holder);
  }
  render() { return div().child(this.left).child(this.right); }
}
"#,
        ),
    );
    draw(&mut context, &root);
    draw(&mut context, &root);

    context.update(|_, cx| {
        let left = mounted(&root, cx).expect("left");
        let shared = mounted(&left, cx).expect("shared");
        assert_eq!(shared.read(cx).nesting_refusal(), None);
    });
    assert_eq!(runtime.metrics().read().script_renders(), 4);
}

/// Loads `source` and instantiates its default export as a view.
fn mount(
    cx: &mut TestAppContext,
    source: &str,
) -> (
    std::rc::Rc<crate::ShellRuntime>,
    gpui::VisualTestContext,
    gpui::Entity<ScriptView>,
) {
    cx.update(crate::init);
    let runtime = crate::ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source("nested.js", source).expect("load");
    let window = cx.add_window(|_, _| gpui::Empty);
    let mut context = gpui::VisualTestContext::from_window(*std::ops::Deref::deref(&window), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    (runtime, context, view)
}

fn draw(context: &mut gpui::VisualTestContext, view: &gpui::Entity<ScriptView>) {
    let view = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| gpui::IntoElement::into_any_element(view),
    );
}

/// The child view `view`'s description mounts, if any.
fn mounted(view: &gpui::Entity<ScriptView>, cx: &gpui::App) -> Option<gpui::Entity<ScriptView>> {
    let snapshot = view.read(cx).snapshot()?;
    (0..snapshot.len() as u32)
        .filter_map(|id| snapshot.arena().node(id))
        .find_map(|node| match node.component() {
            Some(crate::spec::Component::ChildView(child)) => Some(child.view().clone()),
            _ => None,
        })
}
