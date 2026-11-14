//! A ChatGPT desktop client built from GPUI Kit's AI elements.
//!
//! Sign in with a ChatGPT account (the OAuth flow the Codex CLI uses), or set
//! `OPENAI_API_KEY`, then chat through the streaming Responses API.

mod auth;
mod chat;
mod responses;
mod store;

use gpui_kit::{assets::AllAssets, component::TitleBar, *};

fn main() {
    gpui_kit::application()
        .with_assets(AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);

            let options = WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(1100.), px(820.)), cx)),
                window_min_size: Some(size(px(640.), px(480.))),
                ..TitleBar::window_options()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| chat::ChatView::new(window, cx))
            })
            .expect("Failed to open window");
            cx.activate(true);
        });
}
