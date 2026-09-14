use std::{
    fs,
    ops::Deref as _,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use gpui::{AppContext as _, Modifiers, TestAppContext, VisualTestContext, point, px};

static NEXT_APP: AtomicU64 = AtomicU64::new(0);

struct TempApp(PathBuf);

impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "gpui-component-shell-input-group-host-{}-{}",
            std::process::id(),
            NEXT_APP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create temporary application directory");
        fs::write(path.join("main.js"), source).expect("write application entry");
        Self(path)
    }
}

impl Drop for TempApp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove temporary application directory");
    }
}

#[gpui::test]
fn input_group_retains_text_callbacks_and_routes_addon_actions(cx: &mut TestAppContext) {
    cx.update(gpui_component_shell::init);
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let app = TempApp::new(
        r##"
import { div, View } from "gpui-kit";
import { InputGroup, InputGroupInput, InputGroupTextarea, InputGroupAddon,
  InputGroupButton, InputGroupText, InputState, TextareaState, Button } from "gpui-component";
export default class InputGroupHost extends View {
  init() {
    this.input = InputState("Search");
    this.textarea = TextareaState();
    this.value = ""; this.message = ""; this.changes = 0;
    this.clicks = 0; this.disabled = false; this.styleBuilds = 0;
  }
  render() {
    return div().relative().w(500).h(400)
      .child(new InputGroup("search-group").absolute().left(0).top(0).w(400).disabled(this.disabled)
        .focused_style(style => style.border_color("#2563eb"))
        .disabled_style(style => style.opacity(0.7))
        .invalid_style(style => style.border_color("#dc2626"))
        // Replaced controls must never be materialized or subscribe to events.
        .input(new InputGroupInput(this.input).child("discarded"))
        .input(new InputGroupInput(this.input).value(this.value).aria_label("Search")
          .editor_style(style => { this.styleBuilds += 1; return style.px(16); })
          .editor_style(style => style.py(0))
          .on_change((value, cx) => { this.value = value; this.changes += 1; cx.notify(); }))
        .addon(new InputGroupAddon("leading").w(64)
          .child(new InputGroupText().child("Find")))
        .addon(new InputGroupAddon("actions").align("inline-end")
          .child(new InputGroupButton("replace").w(80).label("Replace")
            .label_style(style => style.font_semibold())
            .on_click((_event, cx) => { this.clicks += 1; this.value = "server"; cx.notify(); }))
          .child(new Button("between-actions").w(24).label("/").disabled(true))
          .child(new InputGroupButton("last-action").label("Last").icon("icons/check.svg")
            .icon_style(style => style.size_4()))))
      .child(new InputGroup("message-group").absolute().left(0).top(80).w(400)
        .input(new InputGroupTextarea(this.textarea).value(this.message).placeholder("Message")
          .auto_grow(1, 4).aria_label("Message")
          .editor_style(style => style.p(16).text_base())
          .on_change((value, cx) => { this.message = value; cx.notify(); }))
        .addon(new InputGroupAddon("header").align("block-start").child("Message header"))
        .addon(new InputGroupAddon("footer").align("block-end").child("Message footer")))
      .child(new Button("disable").absolute().left(0).top(300).w(100).h(30).label("Disable")
        .on_click((_event, cx) => { this.disabled = !this.disabled; cx.notify(); }))
      .child(div().absolute().left(0).top(340)
        .child(`value:${this.value};changes:${this.changes};clicks:${this.clicks};message:${this.message};styles:${this.styleBuilds}`));
  }
}
"##,
    );
    let loaded = runtime.load_application(&app.0, "main.js").expect("load");
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = mounted.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime
            .mount_application(&loaded, window, cx)
            .expect("mount");
        *capture.borrow_mut() = Some(view.clone());
        gpui_component::Root::new(view, window, cx)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = mounted.borrow().clone().unwrap();
    let draw = |context: &mut VisualTestContext| {
        context.run_until_parked();
        context.update(|window, cx| window.draw(cx).clear(cx));
        context.update(|_, cx| {
            assert_eq!(view.read(cx).build_error(), None);
            view.read(cx).snapshot().unwrap().debug_tree()
        })
    };
    let initial = draw(&mut context);
    assert!(initial.contains("changes:0"), "{initial}");
    assert!(initial.contains("styles:1"), "{initial}");
    assert!(
        draw(&mut context).contains("styles:1"),
        "native repaint must not rerun style declarations"
    );
    let bounds = |context: &mut VisualTestContext, id: &str| {
        context.update(|window, _| {
            gpui_base::test_support::find(window, &[], &gpui::ElementId::from(id.to_owned()))
                .unwrap_or_else(|| panic!("missing addon part {id}"))
                .bounds()
        })
    };
    let replace = bounds(&mut context, "replace");
    let between = bounds(&mut context, "between-actions");
    let last = bounds(&mut context, "last-action");
    assert!(replace.right() <= between.left() && between.right() <= last.left());
    context.simulate_click(point(px(24.), px(16.)), Modifiers::default());
    context.simulate_input("abc");
    let changed = draw(&mut context);
    assert!(changed.contains("value:abc;changes:3"), "{changed}");

    context.simulate_click(replace.center(), Modifiers::default());
    let replaced = draw(&mut context);
    assert!(
        replaced.contains("value:server;changes:3;clicks:1"),
        "{replaced}"
    );
    // Programmatic value synchronization must not echo a change or loop.
    assert!(draw(&mut context).contains("changes:3"));

    context.simulate_click(point(px(25.), px(310.)), Modifiers::default());
    draw(&mut context);
    context.simulate_click(replace.center(), Modifiers::default());
    context.simulate_click(point(px(24.), px(16.)), Modifiers::default());
    context.simulate_input("x");
    let disabled = draw(&mut context);
    assert!(
        disabled.contains("value:server;changes:3;clicks:1"),
        "{disabled}"
    );

    context.simulate_click(point(px(30.), px(95.)), Modifiers::default());
    context.simulate_input("hello");
    let message = draw(&mut context);
    assert!(message.contains("message:hello"), "{message}");
}

#[gpui::test]
fn input_group_rejects_wrong_part_types_and_invalid_layout_options(cx: &mut TestAppContext) {
    cx.update(gpui_component_shell::init);
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let window = cx.add_window(|window, cx| {
        let empty = cx.new(|_| gpui::Empty);
        gpui_component::Root::new(empty, window, cx)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    for (expression, diagnostic) in [
        (
            "new InputGroup('g').child(div())",
            "InputGroup does not accept ordinary children",
        ),
        (
            "new InputGroup('g').input(new InputGroupText())",
            "InputGroupInput",
        ),
        (
            "new InputGroup('g').addon(new InputGroupText())",
            "InputGroupAddon",
        ),
        ("new InputGroupInput(this.textarea)", "InputState"),
        (
            "new InputGroupTextarea(this.textarea).auto_grow(4, 2)",
            "max_rows",
        ),
        (
            "new InputGroupTextarea(this.textarea).rows(0)",
            "positive integer",
        ),
        ("new InputGroupAddon('a').align('left')", "align"),
        ("new InputGroupButton('b').size('giant')", "size"),
        (
            "new InputGroup('g').focused_style(12)",
            "style declaration function",
        ),
        (
            "new InputGroupButton('b').label_style(style => style.child('oops'))",
            "only style declarations",
        ),
        (
            "new InputGroupButton('b').icon_style(style => style.on_click(() => {}))",
            "only style declarations",
        ),
        (
            "new InputGroupInput(this.input).editor_style(style => div())",
            "must return its style argument",
        ),
        (
            "new InputGroupInput(this.input).content_type('unknown')",
            "content_type",
        ),
    ] {
        let app = TempApp::new(&format!(
            "import {{ View, div }} from 'gpui-kit';
             import {{ InputGroup, InputGroupInput, InputGroupTextarea, InputGroupAddon,
               InputGroupButton, InputGroupText, InputState, TextareaState, Button }} from 'gpui-component';
             export default class Invalid extends View {{
               init() {{ this.input = InputState(); this.textarea = TextareaState(); }}
               render() {{ return {expression}; }}
             }}"
        ));
        let error = context
            .update(|window, cx| runtime.check(&app.0, window, cx))
            .expect_err(expression);
        assert!(
            format!("{error:#}").contains(diagnostic),
            "{expression}: {error:#}"
        );
    }
}
