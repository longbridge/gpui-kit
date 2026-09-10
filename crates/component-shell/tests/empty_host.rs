use std::{
    fs,
    ops::Deref as _,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use gpui::{AppContext as _, Entity, Modifiers, TestAppContext, VisualTestContext, point, px};

static NEXT_APP: AtomicU64 = AtomicU64::new(0);

struct TempApp(PathBuf);

impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "gpui-component-shell-empty-host-{}-{}",
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

struct ScriptRoot(Entity<gpui_shell::ScriptView>);

impl gpui::Render for ScriptRoot {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        self.0.clone()
    }
}

#[gpui::test]
fn empty_slots_replace_previous_parts_and_preserve_child_actions(cx: &mut TestAppContext) {
    cx.update(gpui_component_shell::init);
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let app = TempApp::new(
        r#"
import { div, View } from "gpui-kit";
import {
  Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription, EmptyContent, Button, Icon,
} from "gpui-component";

export default class EmptyHost extends View {
  init() { this.hits = 0; }
  render() {
    return div()
      .child(new Empty().relative().w(400).h(260).p(0)
        .child(div().child(`Hits: ${this.hits}`))
        // Invalid, overwritten values must not be materialized.
        .header(new EmptyHeader().child("discarded header"))
        .content(new EmptyTitle())
        .content(new EmptyContent().absolute().left(0).top(100).w(180).h(40).p(0)
          .child(new Button("create-project").w(180).h(40).label("Create project")
            .on_click((_event, cx) => { this.hits += 1; cx.notify(); })))
        .header(new EmptyHeader().items_start().gap(4)
          .media(new EmptyContent())
          .title(new EmptyContent())
          .description(new EmptyContent())
          .description(new EmptyDescription().text_size(12).child("Create your first project."))
          .title(new EmptyTitle().font_semibold().child("No projects"))
          .media(new EmptyMedia().variant("icon").variant("default").p(2)
            .child(new Icon("folder")))))
      // Parts also render directly, outside the typed slots.
      .child(new EmptyHeader().title(new EmptyTitle().child("Standalone header")))
      .child(new EmptyMedia().child("Standalone media"))
      .child(new EmptyTitle().child("Standalone title"))
      .child(new EmptyDescription().child("Standalone description"))
      .child(new EmptyContent().child("Standalone content"));
  }
}
"#,
    );
    let loaded = runtime
        .load_application(&app.0, "main.js")
        .expect("load application");
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let mounted_for_window = mounted.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime
            .mount_application(&loaded, window, cx)
            .expect("mount application");
        *mounted_for_window.borrow_mut() = Some(view.clone());
        ScriptRoot(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = mounted.borrow().clone().expect("mounted view");
    let draw = |context: &mut VisualTestContext| {
        context.update(|window, cx| window.draw(cx).clear(cx));
        context.update(|_, cx| {
            assert_eq!(view.read(cx).build_error(), None);
            view.read(cx).snapshot().expect("snapshot").debug_tree()
        })
    };
    let initial = draw(&mut context);
    assert!(initial.contains("Hits: 0"), "{initial}");
    for part in [
        "Empty",
        "EmptyHeader",
        "EmptyMedia",
        "EmptyTitle",
        "EmptyDescription",
        "EmptyContent",
    ] {
        assert!(initial.contains(part), "missing {part}: {initial}");
    }
    context.simulate_click(point(px(20.), px(120.)), Modifiers::default());
    context.run_until_parked();
    let changed = draw(&mut context);
    assert!(changed.contains("Hits: 1"), "{changed}");
}

#[gpui::test]
fn empty_rejects_wrong_slot_types_and_ordinary_header_children(cx: &mut TestAppContext) {
    cx.update(gpui_component_shell::init);
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let window = cx.add_window(|window, cx| {
        let empty = cx.new(|_| gpui::Empty);
        gpui_component::Root::new(empty, window, cx)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    for (expression, diagnostic) in [
        (
            "new Empty().header(new EmptyTitle())",
            "EmptyHeader materialized an incompatible element",
        ),
        (
            "new Empty().content(new EmptyTitle())",
            "EmptyContent materialized an incompatible element",
        ),
        (
            "new EmptyHeader().media(new EmptyTitle())",
            "EmptyMedia materialized an incompatible element",
        ),
        (
            "new EmptyHeader().title(new EmptyContent())",
            "EmptyTitle materialized an incompatible element",
        ),
        (
            "new EmptyHeader().description(new EmptyTitle())",
            "EmptyDescription materialized an incompatible element",
        ),
        (
            "new Empty().header(div())",
            "EmptyHeader materialized an incompatible element",
        ),
        (
            "new EmptyHeader().child(div())",
            "EmptyHeader does not accept children",
        ),
        ("new EmptyMedia().variant('avatar')", "variant"),
    ] {
        let app = TempApp::new(&format!(
            "import {{ View, div }} from 'gpui-kit';
             import {{ Empty, EmptyHeader, EmptyTitle, EmptyContent, EmptyMedia }} from 'gpui-component';
             export default class Invalid extends View {{ render() {{ return {expression}; }} }}"
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
