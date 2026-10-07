use gpui_kit::component::{
    ActiveTheme as _, IconName, WindowExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::{DropdownMenu as _, PopupMenuItem},
    popover::Popover,
    v_flex,
};
use gpui_kit::*;
use gpui_webview::WebView;

pub struct Example {
    focus_handle: FocusHandle,
    webview: Entity<WebView>,
    address_input: Entity<InputState>,
}

impl Example {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let webview = cx.new(|cx| {
            let builder = wry::WebViewBuilder::new();
            #[cfg(any(debug_assertions, feature = "inspector"))]
            let builder = builder.with_devtools(true);

            WebView::build(builder, window, cx).expect("Failed to create WebView")
        });

        let address_input =
            cx.new(|cx| InputState::new(window, cx).default_value("https://gpui-kit.com"));

        let url = address_input.read(cx).value().clone();
        webview.update(cx, |view, _| {
            view.load_url(&url);
        });

        cx.new(|cx| {
            let this = Self {
                focus_handle: cx.focus_handle(),
                webview,
                address_input: address_input.clone(),
            };

            cx.subscribe(
                &address_input,
                |this: &mut Self, input, event: &InputEvent, cx| match event {
                    InputEvent::PressEnter { .. } => {
                        let url = input.read(cx).value().clone();
                        this.webview.update(cx, |view, _| {
                            view.load_url(&url);
                        });
                    }
                    _ => {}
                },
            )
            .detach();

            this
        })
    }

    pub fn hide(&self, _: &mut Window, cx: &mut App) {
        self.webview.update(cx, |webview, _| webview.hide())
    }

    fn go_back(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.webview.update(cx, |webview, _| {
            webview.back().unwrap();
        });
    }

    fn go_forward(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.webview.update(cx, |webview, _| {
            webview.forward().unwrap();
        });
    }
}

impl Focusable for Example {
    fn focus_handle(&self, _cx: &gpui_kit::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let webview = self.webview.clone();

        v_flex()
            .p_2()
            .gap_3()
            .size_full()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new("back")
                            .ghost()
                            .icon(IconName::ChevronLeft)
                            .on_click(cx.listener(Self::go_back)),
                    )
                    .child(
                        Button::new("forward")
                            .ghost()
                            .icon(IconName::ChevronRight)
                            .on_click(cx.listener(Self::go_forward)),
                    )
                    .child(Input::new(&self.address_input))
                    // Overlays that open above the WebView, to verify they are not covered.
                    .child(
                        Popover::new("popover")
                            .trigger(Button::new("popover-trigger").outline().label("Popover"))
                            .w(px(320.))
                            .h(px(240.))
                            .child("This popover should render above the WebView."),
                    )
                    .child(
                        Button::new("menu-trigger")
                            .outline()
                            .label("Menu")
                            .dropdown_menu({
                                let webview = self.webview.clone();
                                move |menu, _, _| {
                                    let reload = webview.clone();
                                    menu.item(PopupMenuItem::new("Reload").on_click(
                                        move |_, _, cx| {
                                            let _ = reload.read(cx).raw().reload();
                                        },
                                    ))
                                    .separator()
                                    .item(
                                        PopupMenuItem::new("About").on_click(|_, window, cx| {
                                            window.open_dialog(cx, |dialog, _, _| {
                                                dialog.title("About").child(
                                                    "A WebView embedded in a GPUI Kit window.",
                                                )
                                            });
                                        }),
                                    )
                                }
                            }),
                    )
                    .child(
                        Button::new("dialog-trigger")
                            .outline()
                            .label("Dialog")
                            .on_click(|_, window, cx| {
                                window.open_dialog(cx, |dialog, _, _| {
                                    dialog
                                        .title("Dialog")
                                        .child("This dialog should render above the WebView.")
                                });
                            }),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .border_1()
                    .h(gpui_kit::px(400.))
                    .border_color(cx.theme().border)
                    .child(webview.clone()),
            )
    }
}

fn main() {
    // Required this for Windows to render the WebView.
    #[cfg(target_os = "windows")]
    unsafe {
        std::env::set_var("GPUI_DISABLE_DIRECT_COMPOSITION", "true");
    }

    // WebKitGTK embeds only into X11 windows, so use XWayland in a Wayland session.
    #[cfg(target_os = "linux")]
    let app = gpui_kit::platform::linux(WindowingModes::X11);
    #[cfg(not(target_os = "linux"))]
    let app = gpui_kit::application();

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_kit::init(cx);

        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            Example::new(window, cx)
        })
        .expect("Failed to open window");
    });
}
