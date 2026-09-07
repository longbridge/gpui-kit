mod terminal;
mod terminal_panel;
mod workspace;

use gpui_kit::assets::Assets;
use gpui_kit::component::{Root, TitleBar};
use gpui_kit::*;

use workspace::Workspace;

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        gpui_kit::init(cx);
        cx.activate(true);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1280.), px(800.)), cx)),
            window_min_size: Some(gpui_kit::Size {
                width: px(960.),
                height: px(600.),
            }),
            window_background: WindowBackgroundAppearance::Blurred,
            kind: WindowKind::Normal,
            // The workspace renders its own TitleBar, which owns dragging and
            // the double-click zoom.
            ..TitleBar::window_options()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| Workspace::new(window, cx));
                cx.new(|cx| {
                    Root::new(view, window, cx)
                        .bordered(false)
                        .bg(hsla(0., 0., 0., 0.))
                })
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
