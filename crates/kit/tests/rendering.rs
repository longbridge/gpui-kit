//! These tests need GPUI's Metal renderer. Other platforms still run the native
//! event/state suite; no fake renderer is substituted for missing GPU support.
fn main() {
    #[cfg(target_os = "macos")]
    macos::run();
    #[cfg(not(target_os = "macos"))]
    println!("rendering: skipped; GPUI does not supply a headless renderer on this platform");
}

#[cfg(target_os = "macos")]
mod macos {
    use gpui_kit::{
        AppContext, AssetSource, Context, Entity, HeadlessAppContext, Render, Result, SharedString,
        Window,
        assets::Assets,
        component::{
            ActiveTheme, Theme,
            checkbox::Checkbox,
            input::{Input, InputState},
        },
        div,
        prelude::*,
        px, size,
        test::TestWindowExt,
    };
    use std::{borrow::Cow, sync::Arc};

    // A deliberate rendering defect: the production Checkbox keeps its state and
    // behavior, but its check-mark asset contains no path.
    struct MissingCheck;
    impl AssetSource for MissingCheck {
        fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
            if path == "icons/check.svg" {
                Ok(Some(Cow::Borrowed(
                    br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"/>"#,
                )))
            } else {
                Assets.load(path)
            }
        }
        fn list(&self, path: &str) -> Result<Vec<SharedString>> {
            Assets.list(path)
        }
    }

    fn context(assets: Arc<dyn AssetSource>) -> HeadlessAppContext {
        let mut cx = HeadlessAppContext::with_platform(
            gpui_kit::platform::current_platform(true).text_system(),
            assets,
            gpui_kit::platform::current_headless_renderer,
        );
        cx.update(gpui_kit::init);
        cx
    }

    struct Checked;
    impl Render for Checked {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .bg(cx.theme().background)
                .text_color(cx.theme().foreground)
                .p_2()
                .child(Checkbox::new("agree").checked(true))
        }
    }

    fn checkbox_pixels(assets: Arc<dyn AssetSource>) -> Vec<u8> {
        let mut cx = context(assets);
        let handle = cx
            .open_window(size(px(80.), px(60.)), |_, cx| cx.new(|_| Checked))
            .unwrap();
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            assert_eq!(window.find("agree").checked(), Some(true));
        })
        .unwrap();
        cx.capture_screenshot(handle.into())
            .expect("Metal rendering must be available")
            .into_raw()
    }

    fn pixels_detect_missing_check_even_when_checked_state_is_correct() {
        let expected = checkbox_pixels(Arc::new(Assets));
        let repeated = checkbox_pixels(Arc::new(Assets));
        assert!(
            expected == repeated,
            "identical controls must render deterministically"
        );
        let missing = checkbox_pixels(Arc::new(MissingCheck));
        assert!(
            expected != missing,
            "checked() alone cannot detect a missing check mark"
        );
    }

    struct Editor {
        input: Entity<InputState>,
    }
    impl Render for Editor {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .bg(cx.theme().background)
                .text_color(cx.theme().foreground)
                .p_2()
                .child(Input::new(&self.input).id("name").w(px(200.)))
        }
    }

    fn pixels_detect_missing_input_text_even_when_value_is_correct() {
        let mut cx = context(Arc::new(Assets));
        let handle = cx
            .open_window(size(px(240.), px(70.)), |window, cx| {
                cx.new(|cx| Editor {
                    input: cx.new(|cx| InputState::new(window, cx)),
                })
            })
            .unwrap();
        cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
            .unwrap();
        let empty = cx.capture_screenshot(handle.into()).unwrap();
        cx.update_window(handle.into(), |_, window, cx| {
            window.click("name", cx);
            window.input("Ada", cx);
            window.blur(cx);
            window.render_frame(cx);
            assert_eq!(window.find("name").value(), Some("Ada"));
        })
        .unwrap();
        let populated = cx.capture_screenshot(handle.into()).unwrap();
        assert!(empty != populated, "typing must change the rendered input");
        cx.update_window(handle.into(), |_, window, cx| {
            // Inject a production styling defect without changing the editor value.
            Theme::global_mut(cx).foreground = cx.theme().transparent;
            Theme::sync_base(cx);
            window.render_frame(cx);
            assert_eq!(window.find("name").value(), Some("Ada"));
        })
        .unwrap();
        let hidden = cx.capture_screenshot(handle.into()).unwrap();
        assert!(
            populated != hidden,
            "value() alone cannot detect invisible text"
        );
    }

    pub fn run() {
        println!("running pixels_detect_missing_check_even_when_checked_state_is_correct");
        pixels_detect_missing_check_even_when_checked_state_is_correct();
        println!("passed pixels_detect_missing_check_even_when_checked_state_is_correct");
        println!("running pixels_detect_missing_input_text_even_when_value_is_correct");
        pixels_detect_missing_input_text_even_when_value_is_correct();
        println!("passed pixels_detect_missing_input_text_even_when_value_is_correct");
        println!("rendering: 2 passed (Metal)");
    }
}
