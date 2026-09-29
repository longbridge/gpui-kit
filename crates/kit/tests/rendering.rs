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
        App, AppContext, AssetSource, Bounds, Context, Entity, Focusable as _, HeadlessAppContext,
        Pixels, Render, Result, Rgba, SharedString, Window,
        assets::Assets,
        component::{
            ActiveTheme, IconName, IndexPath, Theme, ThemeMode,
            button::{Button, ButtonVariants as _},
            checkbox::Checkbox,
            input::{Input, InputState},
            list::{List, ListDelegate, ListItem, ListState},
            menu::{PopupMenu, PopupMenuItem},
            table::{Column, DataTable, TableDelegate, TableState},
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

    fn buttons_window(cx: &mut HeadlessAppContext, mode: ThemeMode) -> gpui_kit::AnyWindowHandle {
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
        scale: f32,
        raw: Vec<u8>,
    }
    impl Capture {
        fn take(cx: &mut HeadlessAppContext, handle: gpui_kit::AnyWindowHandle) -> Self {
            let scale = cx
                .update_window(handle, |_, window, _| window.scale_factor())
                .unwrap();
            let image = cx.capture_screenshot(handle).unwrap();
            if let Ok(dir) = std::env::var("RENDERING_DUMP_DIR") {
                static SHOT: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(0);
                let n = SHOT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                image.save(format!("{dir}/shot-{n:02}.png")).unwrap();
            }
            let (width, height) = image.dimensions();
            Self {
                width,
                height,
                scale,
                raw: image.into_raw(),
            }
        }
        fn scale(&self) -> f32 {
            self.scale
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
            (
                Rgba::from(cx.theme().ring),
                cx.theme().radius + px(LINE_GAP),
            )
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

    fn open_window<V: Render + 'static>(
        cx: &mut HeadlessAppContext,
        window_size: (f32, f32),
        build: impl FnOnce(&mut Window, &mut Context<V>) -> V + 'static,
    ) -> gpui_kit::AnyWindowHandle {
        let (handle, _) = cx
            .update(|cx| {
                gpui_kit::open_window(
                    gpui_kit::WindowOptions {
                        window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds {
                            origin: Default::default(),
                            size: size(px(window_size.0), px(window_size.1)),
                        })),
                        focus: false,
                        show: false,
                        ..Default::default()
                    },
                    cx,
                    |window, cx| cx.new(|cx| build(window, cx)),
                )
            })
            .unwrap();
        cx.update_window(handle, |_, window, cx| window.render_frame(cx))
            .unwrap();
        handle
    }

    /// Whether the fill just inside the left edge of `row` differs from `idle`,
    /// that is, whether the row is highlighted. The sample sits in the row's
    /// padding, clear of its label.
    fn is_highlighted(capture: &Capture, idle: &Capture, row: Bounds<Pixels>) -> bool {
        let x = capture.device(row.left() + px(3.));
        let y = capture.device(row.center().y);
        capture.get_pixel(x, y) != idle.get_pixel(x, y)
    }

    fn highlighted_rows(capture: &Capture, idle: &Capture, rows: &[Bounds<Pixels>]) -> Vec<usize> {
        (0..rows.len())
            .filter(|&ix| is_highlighted(capture, idle, rows[ix]))
            .collect()
    }

    const MENU_ITEMS: [&str; 4] = ["Alpha", "Beta", "Gamma", "Delta"];

    struct Menu {
        menu: Entity<PopupMenu>,
    }
    impl Render for Menu {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .bg(cx.theme().background)
                .p_4()
                .child(div().w(px(200.)).child(self.menu.clone()))
        }
    }

    /// A menu has one highlight, and it is the keyboard cursor: the pointer
    /// moves it on hover, and keys move it from wherever it is. A key press
    /// under a still pointer must neither leave the hovered item lit next to
    /// the cursor nor put the highlight out.
    fn menu_highlight_is_the_keyboard_cursor() {
        let mut cx = context(Arc::new(Assets));
        cx.update(|cx| Theme::change(ThemeMode::Light, None, cx));
        let handle = open_window(&mut cx, (320., 240.), |window, cx| {
            let menu = PopupMenu::build(window, cx, |menu, _, _| {
                MENU_ITEMS.iter().fold(menu, |menu, label| {
                    menu.item(PopupMenuItem::new(*label).on_click(|_, _, _| {}))
                })
            });
            menu.read(cx).focus_handle(cx).focus(window, cx);
            Menu { menu }
        });
        let idle = Capture::take(&mut cx, handle);
        let rows = cx
            .update_window(handle, |_, window, _| {
                (0..MENU_ITEMS.len())
                    .map(|ix| window.within("popup-menu").find(ix).bounds())
                    .collect::<Vec<_>>()
            })
            .unwrap();
        let step = |cx: &mut HeadlessAppContext, act: &dyn Fn(&mut Window, &mut App)| {
            cx.update_window(handle, |_, window, cx| {
                act(window, cx);
                window.render_frame(cx);
            })
            .unwrap();
            let capture = Capture::take(cx, handle);
            highlighted_rows(&capture, &idle, &rows)
        };

        let hovered = step(&mut cx, &|window, cx| {
            window.within("popup-menu").hover(1usize, cx)
        });
        assert_eq!(hovered, [1], "hovering `Beta` must light it, and only it");
        let down = step(&mut cx, &|window, cx| window.press("down", cx));
        assert_eq!(
            down,
            [2],
            "`down` must move the highlight to `Gamma` without leaving the hovered `Beta` lit"
        );
        let hovered = step(&mut cx, &|window, cx| {
            window.within("popup-menu").hover(1usize, cx)
        });
        assert_eq!(
            hovered,
            [1],
            "moving the pointer must bring the highlight back to `Beta`"
        );
        let unbound = step(&mut cx, &|window, cx| window.press("x", cx));
        assert_eq!(
            unbound,
            [1],
            "a key press under a still pointer must keep `Beta` highlighted"
        );
        let down = step(&mut cx, &|window, cx| window.press("down", cx));
        assert_eq!(
            down,
            [2],
            "`down` must move on from `Beta`, not restart at the top"
        );
    }

    struct Rows;
    impl ListDelegate for Rows {
        type Item = ListItem;

        fn items_count(&self, _: usize, _: &App) -> usize {
            MENU_ITEMS.len()
        }

        fn render_item(
            &mut self,
            ix: IndexPath,
            _: &mut Window,
            _: &mut Context<ListState<Self>>,
        ) -> Option<Self::Item> {
            Some(ListItem::new(("row", ix.row)).child(MENU_ITEMS[ix.row]))
        }

        fn set_selected_index(
            &mut self,
            _: Option<IndexPath>,
            _: &mut Window,
            _: &mut Context<ListState<Self>>,
        ) {
        }
    }

    struct Listed {
        list: Entity<ListState<Rows>>,
    }
    impl Render for Listed {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .bg(cx.theme().background)
                .p_4()
                .child(div().w(px(200.)).h(px(160.)).child(List::new(&self.list)))
        }
    }

    /// The selected row is a list's keyboard cursor. Arrow keys must light
    /// it, and the hover highlight must yield to it, so the two never read as
    /// two cursors; hovering alone must not select.
    fn list_selection_is_the_keyboard_cursor() {
        let mut cx = context(Arc::new(Assets));
        cx.update(|cx| Theme::change(ThemeMode::Light, None, cx));
        let handle = open_window(&mut cx, (320., 240.), |window, cx| {
            let list = cx.new(|cx| ListState::new(Rows, window, cx));
            list.read(cx).focus_handle(cx).focus(window, cx);
            Listed { list }
        });
        let idle = Capture::take(&mut cx, handle);
        let rows = cx
            .update_window(handle, |_, window, _| {
                (0..MENU_ITEMS.len())
                    .map(|ix| window.find(("row", ix)).bounds())
                    .collect::<Vec<_>>()
            })
            .unwrap();
        let step = |cx: &mut HeadlessAppContext, act: &dyn Fn(&mut Window, &mut App)| {
            cx.update_window(handle, |_, window, cx| {
                act(window, cx);
                window.render_frame(cx);
            })
            .unwrap();
            Capture::take(cx, handle)
        };

        let hovered = step(&mut cx, &|window, cx| window.hover(("row", 1usize), cx));
        assert_eq!(highlighted_rows(&hovered, &idle, &rows), [1]);
        let down = step(&mut cx, &|window, cx| window.press("down", cx));
        assert_eq!(
            highlighted_rows(&down, &idle, &rows),
            [0],
            "`down` must light the first row, and the hovered row must yield to it"
        );
        let x = hovered.device(rows[1].left() + px(3.));
        let y = hovered.device(rows[1].center().y);
        let hover_fill = hovered.get_pixel(x, y);
        let x = down.device(rows[0].left() + px(3.));
        let y = down.device(rows[0].center().y);
        assert_ne!(
            down.get_pixel(x, y),
            hover_fill,
            "the keyboard cursor must not look like a hovered row"
        );
    }

    struct Records;
    impl TableDelegate for Records {
        fn columns_count(&self, _: &App) -> usize {
            2
        }
        fn rows_count(&self, _: &App) -> usize {
            20
        }
        fn column(&self, ix: usize, _: &App) -> Column {
            Column::new(
                format!("column-{ix}"),
                if ix == 0 { "Name" } else { "Status" },
            )
            .width(px(100.))
        }
        fn render_td(
            &mut self,
            row: usize,
            col: usize,
            _: &mut Window,
            _: &mut Context<TableState<Self>>,
        ) -> impl IntoElement {
            div().child(format!("{row}:{col}"))
        }
    }

    struct Table {
        table: Entity<TableState<Records>>,
    }
    impl Render for Table {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .bg(cx.theme().background)
                .p_4()
                .flex()
                .flex_col()
                .gap_4()
                .child(Button::new("before").label("Before"))
                .child(div().h(px(140.)).child(DataTable::new(&self.table)))
        }
    }

    fn table_window(cx: &mut HeadlessAppContext, focus_ring: bool) -> gpui_kit::AnyWindowHandle {
        cx.update(|cx| {
            Theme::change(ThemeMode::Light, None, cx);
            Theme::update(cx, |theme| theme.focus_ring = focus_ring);
        });
        open_window(cx, (320., 240.), |window, cx| Table {
            table: cx.new(|cx| TableState::new(Records, window, cx)),
        })
    }

    fn table_bounds(
        cx: &mut HeadlessAppContext,
        handle: gpui_kit::AnyWindowHandle,
    ) -> Bounds<Pixels> {
        cx.update_window(handle, |_, window, _| window.find("table").bounds())
            .unwrap()
    }

    /// A table is a Tab stop, so Tab into it must show focus on its edge;
    /// clicking a row focuses it too, and must not.
    fn table_shows_keyboard_focus_only(focus_ring: bool) {
        let mut cx = context(Arc::new(Assets));
        let handle = table_window(&mut cx, focus_ring);
        let bounds = table_bounds(&mut cx, handle);
        let ring = cx.update(|cx| Rgba::from(cx.theme().ring));
        cx.update_window(handle, |_, window, cx| {
            window.focus_next(cx);
            window.render_frame(cx);
            assert_eq!(window.find("before").focused(), Some(true));
        })
        .unwrap();
        let before = Capture::take(&mut cx, handle);
        let half_device_px = px(0.5 / before.scale());
        cx.update_window(handle, |_, window, cx| {
            window.press("tab", cx);
            window.render_frame(cx);
            assert_eq!(window.find("table").focused(), Some(true));
        })
        .unwrap();
        let focused = Capture::take(&mut cx, handle);
        assert!(
            focused
                .top_row(bounds, half_device_px)
                .into_iter()
                .any(|pixel| is_ring(pixel, ring)),
            "a keyboard-focused table must tint its border (focus_ring = {focus_ring})"
        );
        if focus_ring {
            assert!(
                focused.top_row(bounds, -px(1.5)) != before.top_row(bounds, -px(1.5)),
                "a keyboard-focused table must draw the outer ring"
            );
        }

        let mut cx = context(Arc::new(Assets));
        let handle = table_window(&mut cx, focus_ring);
        let bounds = table_bounds(&mut cx, handle);
        let offset = gpui_kit::point(px(40.), bounds.size.height - px(20.));
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.hover("table", cx);
        })
        .unwrap();
        let hovered = Capture::take(&mut cx, handle);
        cx.update_window(handle, |_, window, cx| {
            window.click_at("table", offset, cx);
            window.render_frame(cx);
            assert_eq!(
                window.find("table").focused(),
                Some(true),
                "the click must focus the table"
            );
        })
        .unwrap();
        let clicked = Capture::take(&mut cx, handle);
        for offset in [half_device_px, -px(1.5)] {
            assert!(
                clicked.top_row(bounds, offset) == hovered.top_row(bounds, offset),
                "clicking a row must draw no focus on the table's edge (focus_ring = {focus_ring})"
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
        println!("running menu_highlight_is_the_keyboard_cursor");
        menu_highlight_is_the_keyboard_cursor();
        println!("passed menu_highlight_is_the_keyboard_cursor");
        println!("running list_selection_is_the_keyboard_cursor");
        list_selection_is_the_keyboard_cursor();
        println!("passed list_selection_is_the_keyboard_cursor");
        for focus_ring in [false, true] {
            println!("running table_shows_keyboard_focus_only (focus_ring = {focus_ring})");
            table_shows_keyboard_focus_only(focus_ring);
            println!("passed table_shows_keyboard_focus_only (focus_ring = {focus_ring})");
        }
        println!("rendering: 10 passed (Metal)");
    }
}
