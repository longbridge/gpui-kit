use gpui_kit::component::{ActiveTheme as _, Root, StyledExt as _, h_flex, v_flex};
use gpui_kit::*;

struct RootBorderlessExample;

impl Render for RootBorderlessExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_4()
            .p_8()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                div()
                    .text_2xl()
                    .font_semibold()
                    .child("Root::bordered(false)"),
            )
            .child(
                div()
                    .max_w(px(560.))
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        "This window requests client-side decorations, while Root disables GPUI Component's window border wrapper.",
                    ),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        div()
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().border)
                            .px_3()
                            .py_2()
                            .child("Root.bordered = false"),
                    )
                    .child(
                        div()
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().border)
                            .px_3()
                            .py_2()
                            .child("window_decorations = Client"),
                    ),
            )
    }
}

fn main() {
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);

        let window_options = WindowOptions {
            titlebar: None,
            window_bounds: Some(WindowBounds::centered(size(px(640.), px(320.)), cx)),
            window_decorations: Some(WindowDecorations::Client),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|_| RootBorderlessExample);
                cx.new(|cx| Root::new(view, window, cx).bordered(false))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
