use gpui_kit::component::{
    ActiveTheme as _,
    text::{TextView, TextViewState},
    tooltip::Tooltip,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

struct LinkTooltipExample {
    text: Entity<TextViewState>,
    show_preview: bool,
}

impl Render for LinkTooltipExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_6()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(TextView::new(&self.text).w_full().selectable(true).when(
                self.show_preview,
                |view| {
                    view.link_tooltip(|url, window, cx| {
                        let destination = url.clone();
                        Tooltip::element(move |_, _| {
                            div().max_w(rems(32.)).child(destination.clone())
                        })
                        .build(window, cx)
                    })
                },
            ))
    }
}

fn main() {
    let show_preview = !std::env::args().any(|arg| arg == "--no-preview");
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            let options = WindowOptions {
                app_id: Some("gpui-link-tooltip-example".into()),
                window_bounds: Some(WindowBounds::centered(size(px(720.), px(360.)), cx)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Link previews".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |_, cx| {
                cx.new(|cx| LinkTooltipExample {
                    show_preview,
                    text: cx.new(|cx| TextViewState::markdown(
                        "# Reports\n\nMove the pointer along a link to inspect its destination.\n\n[Cleanup report](file:///home/demo/reports/cleanup-report.json)\n\n[Review the complete report with a link label that wraps across multiple lines and remains selectable](https://example.com/reports/2026/11/cleanup-report.json)\n\n[GPUI Kit documentation](https://gpui-kit.com/docs/getting-started)",
                        cx,
                    )),
                })
            })
            .expect("open link preview example");
        });
}
