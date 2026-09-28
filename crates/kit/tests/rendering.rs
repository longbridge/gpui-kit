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
        AppContext, AssetSource, Bounds, Context, Entity, HeadlessAppContext, Pixels, Render,
        Result, Rgba, SharedString, Window,
        assets::Assets,
        component::{
            ActiveTheme, IconName, Theme, ThemeMode,
            button::{Button, ButtonVariants as _},
            checkbox::Checkbox,
            input::{Input, InputState},
            text::TextView,
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
        let (handle, _) = cx
            .update(|cx| {
                gpui_kit::open_window(
                    gpui_kit::WindowOptions {
                        window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds {
                            origin: Default::default(),
                            size: size(px(80.), px(60.)),
                        })),
                        focus: false,
                        show: false,
                        ..Default::default()
                    },
                    cx,
                    |_, cx| cx.new(|_| Checked),
                )
            })
            .unwrap();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert_eq!(window.find("agree").checked(), Some(true));
        })
        .unwrap();
        cx.capture_screenshot(handle)
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
        let (handle, _) = cx
            .update(|cx| {
                gpui_kit::open_window(
                    gpui_kit::WindowOptions {
                        window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds {
                            origin: Default::default(),
                            size: size(px(240.), px(70.)),
                        })),
                        focus: false,
                        show: false,
                        ..Default::default()
                    },
                    cx,
                    |window, cx| {
                        cx.new(|cx| Editor {
                            input: cx.new(|cx| InputState::new(window, cx)),
                        })
                    },
                )
            })
            .unwrap();
        cx.update_window(handle, |_, window, cx| window.render_frame(cx))
            .unwrap();
        let empty = cx.capture_screenshot(handle).unwrap();
        cx.update_window(handle, |_, window, cx| {
            window.click("name", cx);
            window.input("Ada", cx);
            window.blur(cx);
            window.render_frame(cx);
            assert_eq!(window.find("name").value(), Some("Ada"));
        })
        .unwrap();
        let populated = cx.capture_screenshot(handle).unwrap();
        assert!(empty != populated, "typing must change the rendered input");
        cx.update_window(handle, |_, window, cx| {
            // Inject a production styling defect without changing the editor value.
            let transparent = cx.theme().transparent;
            Theme::update(cx, |theme| theme.foreground = transparent);
            window.render_frame(cx);
            assert_eq!(window.find("name").value(), Some("Ada"));
        })
        .unwrap();
        let hidden = cx.capture_screenshot(handle).unwrap();
        assert!(
            populated != hidden,
            "value() alone cannot detect invisible text"
        );
    }

    // CJK full-width punctuation shapes wider mid-line than on its own, and the
    // inline code routes the paragraph through the inline flow's own wrapping.
    const CJK_WITH_CODE: &str = "行情显示 HUT 近 5 日下跌约 18%，成交放大到均量的 2.4 倍。\
        财报接口这次失败了，改用网页搜索里的季报摘要：前两大客户贡献约 60% 的租约收入，\
        合约期 10 年以上。可以放一张对比卡 `HUT.US` `MARA.US`，单只行情就不重复放了。";

    const WRAP_PAD: f32 = 8.;

    struct Wrapped {
        width: f32,
    }
    impl Render for Wrapped {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .bg(cx.theme().background)
                .text_color(cx.theme().foreground)
                .p(px(WRAP_PAD))
                .child(
                    div()
                        .w(px(self.width))
                        .child(TextView::markdown("wrapped", CJK_WITH_CODE).text_sm()),
                )
        }
    }

    /// Columns right of the wrap width that hold painted pixels. Ink there
    /// is a line laid out wider than the space it was wrapped for, which a
    /// clipping parent would cut off.
    fn ink_past_wrap_width(width: f32) -> usize {
        let mut cx = context(Arc::new(Assets));
        let window_width = width + WRAP_PAD * 2. + 60.;
        let (handle, _) = cx
            .update(|cx| {
                gpui_kit::open_window(
                    gpui_kit::WindowOptions {
                        window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds {
                            origin: Default::default(),
                            size: size(px(window_width), px(260.)),
                        })),
                        focus: false,
                        show: false,
                        ..Default::default()
                    },
                    cx,
                    |_, cx| cx.new(|_| Wrapped { width }),
                )
            })
            .unwrap();
        cx.update_window(handle, |_, window, cx| window.render_frame(cx))
            .unwrap();
        let image = cx.capture_screenshot(handle).unwrap();
        let (w, h) = image.dimensions();
        let scale = w as f32 / window_width;
        let background = *image.get_pixel(w - 1, 0);
        let first = ((WRAP_PAD + width) * scale).ceil() as u32 + 1;
        (first..w)
            .filter(|&x| (0..h).any(|y| *image.get_pixel(x, y) != background))
            .count()
    }

    fn wrapped_cjk_with_inline_code_stays_within_the_wrap_width() {
        for width in [300., 330., 360., 390., 420., 450., 480.] {
            let columns = ink_past_wrap_width(width);
            assert_eq!(
                columns, 0,
                "lines wrapped at {width}px painted {columns} device columns past the wrap width"
            );
        }
    }

    /// Where each button is expected to draw its focus line.
    #[derive(Clone, Copy, PartialEq)]
    enum Line {
        Edge,
        Inside,
        Outside,
    }

    const BUTTONS: [(&str, Line); 4] = [
        ("ghost", Line::Edge),
        ("text", Line::Outside),
        ("link", Line::Outside),
        ("primary", Line::Inside),
    ];

    /// The gap `FOCUS_LINE_GAP` puts between the edge and an inset or outset
    /// line, at the default rem size.
    const LINE_GAP: f32 = 2.;

    struct Buttons;
    impl Render for Buttons {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .bg(cx.theme().background)
                .text_color(cx.theme().foreground)
                .p_4()
                .flex()
                .items_center()
                .gap_4()
                .child(Button::new("ghost").ghost().icon(IconName::Copy))
                .child(Button::new("text").text().label("Ocean"))
                .child(Button::new("link").link().label("Ocean"))
                .child(Button::new("primary").primary().label("Ocean"))
        }
    }

    const BUTTONS_SIZE: (f32, f32) = (320., 64.);

    fn buttons_window(
        cx: &mut HeadlessAppContext,
        mode: ThemeMode,
    ) -> gpui_kit::AnyWindowHandle {
        cx.update(|cx| {
            Theme::change(mode, None, cx);
            Theme::update(cx, |theme| theme.focus_ring = false);
        });
        let (handle, _) = cx
            .update(|cx| {
                gpui_kit::open_window(
                    gpui_kit::WindowOptions {
                        window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds {
                            origin: Default::default(),
                            size: size(px(BUTTONS_SIZE.0), px(BUTTONS_SIZE.1)),
                        })),
                        focus: false,
                        show: false,
                        ..Default::default()
                    },
                    cx,
                    |_, cx| cx.new(|_| Buttons),
                )
            })
            .unwrap();
        cx.update_window(handle, |_, window, cx| window.render_frame(cx))
            .unwrap();
        handle
    }

    /// A window capture as raw RGBA rows.
    struct Capture {
        width: u32,
        height: u32,
        raw: Vec<u8>,
    }
    impl Capture {
        fn take(cx: &mut HeadlessAppContext, handle: gpui_kit::AnyWindowHandle) -> Self {
            let image = cx.capture_screenshot(handle).unwrap();
            if let Ok(dir) = std::env::var("RENDERING_DUMP_DIR") {
                static SHOT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
                let n = SHOT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                image.save(format!("{dir}/shot-{n:02}.png")).unwrap();
            }
            let (width, height) = image.dimensions();
            Self {
                width,
                height,
                raw: image.into_raw(),
            }
        }
        fn scale(&self) -> f32 {
            self.width as f32 / BUTTONS_SIZE.0
        }
        fn device(&self, value: Pixels) -> i32 {
            (f32::from(value) * self.scale()).round() as i32
        }
        fn get_pixel(&self, x: i32, y: i32) -> [i32; 3] {
            let start = ((y as u32 * self.width + x as u32) * 4) as usize;
            [0, 1, 2].map(|i| self.raw[start + i] as i32)
        }
        /// Device pixels that differ from `other` inside (or outside) `bounds`,
        /// skipping the `corner` square at each corner of `bounds`.
        fn changes(
            &self,
            other: &Capture,
            bounds: Bounds<Pixels>,
            inside: bool,
            corner: Pixels,
        ) -> usize {
            let (left, right) = (self.device(bounds.left()), self.device(bounds.right()));
            let (top, bottom) = (self.device(bounds.top()), self.device(bounds.bottom()));
            let corner = self.device(corner);
            (0..self.height as i32)
                .flat_map(|y| (0..self.width as i32).map(move |x| (x, y)))
                .filter(|&(x, y)| {
                    let within = x >= left && x < right && y >= top && y < bottom;
                    let at_corner = (x < left + corner || x >= right - corner)
                        && (y < top + corner || y >= bottom - corner);
                    within == inside
                        && !(within && at_corner)
                        && self.get_pixel(x, y) != other.get_pixel(x, y)
                })
                .count()
        }
        /// The middle half of the device row `offset` below the top of `bounds`.
        fn top_row(&self, bounds: Bounds<Pixels>, offset: Pixels) -> Vec<[i32; 3]> {
            let (left, right) = (self.device(bounds.left()), self.device(bounds.right()));
            let quarter = (right - left) / 4;
            let y = self.device(bounds.top() + offset);
            ((left + quarter)..(right - quarter))
                .map(|x| self.get_pixel(x, y))
                .collect()
        }
    }

    fn distance(a: [i32; 3], b: [i32; 3]) -> i32 {
        (0..3).map(|i| (a[i] - b[i]).abs()).max().unwrap()
    }

    fn is_ring(pixel: [i32; 3], ring: Rgba) -> bool {
        let ring = [ring.r, ring.g, ring.b].map(|channel| (channel * 255.).round() as i32);
        distance(pixel, ring) <= 8
    }

    fn focus_lines_stay_off_content_and_contrast_with_fills(mode: ThemeMode) {
        let mut cx = context(Arc::new(Assets));
        let handle = buttons_window(&mut cx, mode);
        let (ring, corner) = cx.update(|cx| {
            (Rgba::from(cx.theme().ring), cx.theme().radius + px(LINE_GAP))
        });
        let idle = Capture::take(&mut cx, handle);
        let half_device_px = px(0.5 / idle.scale());

        for (id, line) in BUTTONS {
            let bounds = cx
                .update_window(handle, |_, window, cx| {
                    // Tab only reaches Root's binding once something inside
                    // it holds focus, so the first stop is what Tab does.
                    if window.focused(cx).is_none() {
                        window.focus_next(cx);
                    } else {
                        window.press("tab", cx);
                    }
                    window.render_frame(cx);
                    window.find(id).bounds()
                })
                .unwrap();
            let focused = Capture::take(&mut cx, handle);
            // The box the line may paint in: the button, plus the gap and
            // one pixel of antialiasing when it sits outside.
            let reach = if line == Line::Outside {
                bounds.dilate(px(LINE_GAP + 1.))
            } else {
                bounds
            };
            let outside = focused.changes(&idle, reach, false, Pixels::ZERO);
            assert_eq!(
                outside, 0,
                "focusing `{id}` changed {outside} device pixels outside its reach"
            );
            match line {
                Line::Edge => assert!(
                    focused
                        .top_row(bounds, half_device_px)
                        .into_iter()
                        .any(|pixel| is_ring(pixel, ring)),
                    "a focused `{id}` button must draw a ring on its edge"
                ),
                Line::Outside => {
                    // The line's rounded corners cut across the square
                    // corners of the bounds; the label sits between them.
                    let inside = focused.changes(&idle, bounds, true, corner);
                    assert_eq!(
                        inside, 0,
                        "the focus line of `{id}` changed {inside} device pixels over its label"
                    );
                    assert!(
                        focused
                            .top_row(bounds, -px(LINE_GAP) - half_device_px)
                            .into_iter()
                            .any(|pixel| is_ring(pixel, ring)),
                        "a focused `{id}` button must draw a ring just outside its edge"
                    );
                }
                Line::Inside => {
                    let fill = focused.top_row(bounds, px(1.))[0];
                    let line = focused
                        .top_row(bounds, px(LINE_GAP) + half_device_px)
                        .into_iter()
                        .map(|pixel| distance(pixel, fill))
                        .max()
                        .unwrap();
                    assert!(
                        line >= 96,
                        "the focus line of `{id}` differs from its fill by only {line}/255"
                    );
                }
            }
        }
    }

    fn clicking_a_button_draws_no_focus_line() {
        let mut cx = context(Arc::new(Assets));
        let handle = buttons_window(&mut cx, ThemeMode::Light);
        for (id, _) in BUTTONS {
            cx.update_window(handle, |_, window, cx| {
                window.hover(id, cx);
                window.render_frame(cx);
            })
            .unwrap();
            // Hovered, so the click below changes nothing but focus.
            let hovered = Capture::take(&mut cx, handle);
            cx.update_window(handle, |_, window, cx| {
                window.click(id, cx);
                window.render_frame(cx);
                assert!(window.focused(cx).is_none(), "clicking `{id}` took focus");
            })
            .unwrap();
            let clicked = Capture::take(&mut cx, handle);
            assert!(
                clicked.raw == hovered.raw,
                "clicking `{id}` must draw no focus line"
            );
        }
    }

    pub fn run() {
        println!("running pixels_detect_missing_check_even_when_checked_state_is_correct");
        pixels_detect_missing_check_even_when_checked_state_is_correct();
        println!("passed pixels_detect_missing_check_even_when_checked_state_is_correct");
        println!("running pixels_detect_missing_input_text_even_when_value_is_correct");
        pixels_detect_missing_input_text_even_when_value_is_correct();
        println!("passed pixels_detect_missing_input_text_even_when_value_is_correct");
        println!("running wrapped_cjk_with_inline_code_stays_within_the_wrap_width");
        wrapped_cjk_with_inline_code_stays_within_the_wrap_width();
        println!("passed wrapped_cjk_with_inline_code_stays_within_the_wrap_width");
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            println!("running focus_lines_stay_off_content_and_contrast_with_fills ({mode:?})");
            focus_lines_stay_off_content_and_contrast_with_fills(mode);
            println!("passed focus_lines_stay_off_content_and_contrast_with_fills ({mode:?})");
        }
        println!("running clicking_a_button_draws_no_focus_line");
        clicking_a_button_draws_no_focus_line();
        println!("passed clicking_a_button_draws_no_focus_line");
        println!("rendering: 6 passed (Metal)");
    }
}
