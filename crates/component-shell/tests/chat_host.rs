use std::{
    fs,
    ops::Deref as _,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use gpui::{Entity, TestAppContext, VisualTestContext};

static NEXT_APP: AtomicU64 = AtomicU64::new(0);

struct TempApp(PathBuf);

impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "gpui-component-shell-chat-host-{}-{}",
            std::process::id(),
            NEXT_APP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create temporary application directory");
        fs::write(path.join("main.js"), source).expect("write temporary application entry");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
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
fn chat_components_materialize_through_the_public_host(cx: &mut TestAppContext) {
    cx.update(gpui_component_shell::init);
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let app = TempApp::new(
        r#"
import { div, View } from "gpui-kit";
import {
  Attachment, Bubble, Marker, Message, MessageScroller,
  MessageScrollerState, ShimmerText,
} from "gpui-component";

export default class ChatHost extends View {
  init() { this.scroller = MessageScrollerState(2); }
  render() {
    const rows = ["first", "second"];
    return div()
      .child(new Attachment("attachment").status("complete").child("report.pdf"))
      .child(new Bubble().alignment("end").variant("filled").child("bubble"))
      .child(new Marker("marker").variant("separator").loading(true).child("marker"))
      .child(new Message().alignment("start").child("message"))
      .child(new ShimmerText("thinking").id("shimmer").duration_ms(900))
      .child(new MessageScroller("messages", this.scroller,
        (index) => div().child(rows[index]))
        .h(120).scrollbar(true).jump_button_label("Latest"));
  }
}
"#,
    );
    let loaded = runtime
        .load_application(app.path(), "main.js")
        .expect("load application");
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let mounted_for_window = mounted.clone();
    let runtime_for_window = runtime.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_window
            .mount_application(&loaded, window, cx)
            .expect("mount application");
        *mounted_for_window.borrow_mut() = Some(view.clone());
        ScriptRoot(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let tree = context.update(|_, cx| {
        let view = mounted.borrow().clone().expect("mounted view");
        assert_eq!(view.read(cx).build_error(), None);
        view.read(cx).snapshot().expect("snapshot").debug_tree()
    });
    for expected in [
        "Attachment",
        "Bubble",
        "Marker",
        "Message",
        "ShimmerText",
        "MessageScroller",
    ] {
        assert!(tree.contains(expected), "missing `{expected}`:\n{tree}");
    }
}
