#![cfg(feature = "components")]
#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;

use std::{
    fs,
    ops::Deref,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use gpui::{AppContext as _, Modifiers, TestAppContext, VisualTestContext, point, px};

static NEXT_APP: AtomicU64 = AtomicU64::new(0);

struct TempApp(PathBuf);

impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "gpui-shell-otp-host-{}-{}",
            std::process::id(),
            NEXT_APP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create temporary application directory");
        fs::write(path.join("main.js"), source).expect("write temporary application entry");
        Self(path)
    }
}

impl Drop for TempApp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove temporary application directory");
    }
}

fn draw(context: &mut VisualTestContext) {
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));
}

/// A keystroke must repaint the code without the pointer moving.
///
/// The window is rooted the way the `gpui-shell` binary roots it:
/// `gpui_component::Root` around `ShellRoot`, whose content is a cached view.
/// `OtpState` is a plain entity that the cells read while they are built, so
/// its `notify` dirties no view on its own; unless the binding mounts the state
/// as a view, the cached frame is replayed and the typed digit only appears on
/// the next pointer move.
#[gpui::test]
fn typing_into_a_retained_otp_input_repaints_the_cached_content(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::components::init(cx));
    // Two groups of two 32px cells: 4px inside a group, 20px between them.
    let source = r#"
import { View } from "gpui-kit";
import { OtpInput, OtpState } from "gpui-component";
export default class Code extends View {
  init() { this.otp = OtpState(4); }
  render() { return new OtpInput(this.otp).w(124); }
}
"#;
    let app = TempApp::new(source);
    let runtime = gpui_shell::components::new_isolated_runtime().expect("runtime");
    let loaded = runtime
        .load_application(&app.0, "main.js")
        .expect("load application");
    let window = cx.add_window({
        let runtime = runtime.clone();
        move |window, cx| {
            let view = runtime
                .mount_application(&loaded, window, cx)
                .expect("mount application");
            let shell = cx.new(|cx| gpui_shell::ShellRoot::new(view.into(), window, cx));
            gpui_component::Root::new(shell, window, cx)
        }
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    draw(&mut context);

    context.simulate_click(point(px(16.), px(16.)), Modifiers::default());
    draw(&mut context);
    assert!(
        context.update(|window, cx| window.focused(cx).is_some()),
        "clicking the first cell must give the code the keyboard"
    );

    // The first key switches the window from pointer to keyboard modality,
    // and that switch alone refreshes every view. The second key is the one
    // that has only the state's notification to go on.
    context.simulate_keystrokes("1");
    draw(&mut context);

    let materialized = runtime.read_metrics().materializations();
    let script_renders = runtime.read_metrics().script_renders();
    context.simulate_keystrokes("2");
    draw(&mut context);

    assert!(
        runtime.read_metrics().materializations() > materialized,
        "the typed digit must rebuild the cells on the next frame, not wait for the pointer"
    );
    assert_eq!(
        runtime.read_metrics().script_renders(),
        script_renders,
        "and drawing it must not re-enter the script"
    );
}
