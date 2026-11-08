#![cfg(feature = "components")]
#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;

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
            "gpui-shell-chat-host-{}-{}",
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
    cx.update(gpui_shell::components::init);
    let runtime = gpui_shell::components::new_isolated_runtime().expect("runtime");
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

/// Rows are rebuilt during layout on every frame; a row's own handlers belong
/// to that frame, so they can be registered and they fire.
#[gpui::test]
fn message_scroller_rows_hold_their_own_handlers(cx: &mut TestAppContext) {
    cx.update(gpui_shell::components::init);
    let runtime = gpui_shell::components::new_isolated_runtime().expect("runtime");
    let app = TempApp::new(
        r#"
import { div, View } from "gpui-kit";
import { TextView } from "gpui-base";
import { MessageScroller, MessageScrollerState } from "gpui-component";

export default class Rows extends View {
  init() { this.scroller = MessageScrollerState(1); this.clicked = false; }
  render() {
    return div().size_full()
      .when(this.clicked, (el) => el.child(div().id("row-clicked")))
      .child(new MessageScroller("messages", this.scroller, () =>
        div().id("row").w(200).h(40)
          .on_click((_event, cx) => { this.clicked = true; cx.notify(); })
          .child(TextView.markdown("text", "[link](https://example.com)")
            .on_link_click((_url, cx) => cx.notify())))
        .w(200).h(120));
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
    context.simulate_click(
        gpui::point(gpui::px(150.), gpui::px(30.)),
        gpui::Modifiers::none(),
    );
    context.update(|window, cx| window.draw(cx).clear(cx));
    let tree = context.update(|_, cx| {
        let view = mounted.borrow().clone().expect("mounted view");
        assert_eq!(view.read(cx).build_error(), None);
        view.read(cx).snapshot().expect("snapshot").debug_tree()
    });
    assert!(
        tree.contains("row-clicked"),
        "the row's click was not delivered:\n{tree}"
    );
}

/// A transcript is edited while it is on screen, so its retained state answers
/// script calls in place; out-of-range edits report `false` and malformed
/// arguments throw rather than being rounded or clamped.
#[gpui::test]
fn message_scroller_state_is_edited_from_script(cx: &mut TestAppContext) {
    cx.update(gpui_shell::components::init);
    let runtime = gpui_shell::components::new_isolated_runtime().expect("runtime");
    let app = TempApp::new(
        r#"
import { div, View } from "gpui-kit";
import { MessageScroller, MessageScrollerState } from "gpui-component";

const rejects = (call) => { try { call(); return "ok"; } catch { return "err"; } };

export default class Edits extends View {
  init() { this.scroller = MessageScrollerState.new(2); this.results = "none"; }
  run() {
    const s = this.scroller;
    const out = [];
    out.push(s.append(3), s.item_count());
    out.push(s.prepend(1), s.item_count());
    out.push(s.splice({ start: 0, end: 2 }, 1), s.item_count());
    out.push(s.splice({ start: 4, end: 9 }, 1), s.item_count());
    out.push(s.remeasure_items({ start: 0, end: 2 }), s.remeasure_items({ start: 4, end: 9 }));
    out.push(s.scroll_to_item(1), s.logical_scroll_top().item_ix, s.scroll_to_item(5));
    s.remeasure();
    s.reset(4);
    out.push(s.item_count(), s.is_following_tail());
    out.push(rejects(() => s.append(-1)), rejects(() => s.append(1.5)), rejects(() => s.splice(1, 2, 1)));
    this.results = out.join("-");
  }
  render() {
    return div().size_full()
      .child(div().id(`count-${this.scroller.item_count()}`))
      .child(div().id(`results-${this.results}`))
      .child(div().id("run").w(200).h(40)
        .on_click((_event, cx) => { this.run(); cx.notify(); }))
      .child(MessageScroller.new("messages", this.scroller, (index) =>
        div().h(30).child(`row ${index}`)).w(200).h(120));
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
    let tree = |context: &mut VisualTestContext| {
        context.update(|_, cx| {
            let view = mounted.borrow().clone().expect("mounted view");
            assert_eq!(view.read(cx).build_error(), None);
            view.read(cx).snapshot().expect("snapshot").debug_tree()
        })
    };
    let before = tree(&mut context);
    assert!(before.contains("count-2"), "initial row count:\n{before}");

    context.simulate_click(
        gpui::point(gpui::px(100.), gpui::px(20.)),
        gpui::Modifiers::none(),
    );
    context.update(|window, cx| window.draw(cx).clear(cx));
    let after = tree(&mut context);
    assert!(
        after.contains(
            "results-true-5-true-6-true-5-false-5-true-false-true-1-false-4-true-err-err-err"
        ),
        "unexpected results:\n{after}"
    );
    assert!(after.contains("count-4"), "row count after reset:\n{after}");
}
