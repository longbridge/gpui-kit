//! Scrolling a long Markdown `TextView`: one frame per iteration, the way a
//! chat scrolls. Every frame re-renders and re-lays out the visible
//! paragraphs, so this is where the per-paragraph text costs (shaping-cache
//! lookups, line wrappers, measurement) show up.
//!
//! Run with `cargo bench -p gpui-base --bench text_view_scroll`.

use std::fmt::Write as _;

use gpui::{
    AppContext as _, BenchAppContext, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, ScrollHandle, StatefulInteractiveElement as _, Styled as _, Window,
    div, point, px,
};
use gpui_base::text::{TextView, TextViewState};

/// A long assistant answer: headings, emphasis, links, lists, a code block,
/// and plenty of prose paragraphs.
fn markdown() -> String {
    let mut out = String::new();
    for section in 0..24 {
        let _ = writeln!(out, "## Section {section}: what the data shows\n");
        for paragraph in 0..4 {
            let _ = writeln!(
                out,
                "Paragraph {paragraph} of section {section}. The **dip window** is 08-18 → 08-26 \
                 (trough $208.48 on 08-24), followed by a post-earnings jump. Let me confirm the \
                 earnings date and date the specific news events in that window, then weave in \
                 [the filing](https://example.com/filing/{section}) and the *guidance* update so \
                 the numbers line up with what the market priced in before the open.\n"
            );
        }
        let _ = writeln!(
            out,
            "- FOMC 09-16: +25bp to 3.75–4.00%, 12-0. First hike since July 2023."
        );
        let _ = writeln!(
            out,
            "- Dot plot: one more hike implied this year; 2026 projection 4.1%."
        );
        let _ = writeln!(out, "- XLE +45.2% YTD, the standout winner; XLU flat.\n");
        if section % 6 == 0 {
            let _ = writeln!(
                out,
                "```toml\n[dependencies]\ngpui-kit = \"0.6\"\nserde = {{ version = \"1\", features = [\"derive\"] }}\n```\n"
            );
        }
    }
    out
}

struct Chat {
    /// The parsed answer, held the way a chat holds a streamed message.
    answer: Entity<TextViewState>,
    scroll: ScrollHandle,
}

impl Render for Chat {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            div()
                .id("viewport")
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .child(
                    div()
                        .w_full()
                        .px(px(20.))
                        .child(TextView::new(&self.answer)),
                ),
        )
    }
}

#[gpui::bench]
fn text_view_scroll(cx: &mut BenchAppContext) {
    cx.update(gpui_base::init);
    let markdown = markdown();
    let mut window = cx.add_empty_window();
    let chat = window.update(|window, cx| {
        window.replace_root(cx, |_, cx| Chat {
            answer: cx.new(|cx| TextViewState::markdown(&markdown, cx)),
            scroll: ScrollHandle::new(),
        })
    });
    // The Markdown parse is a background task; scroll only once it is in.
    cx.run_until_idle();

    // A steady swipe: 40px a frame, wrapping back to the top at the end, so
    // every frame moves the visible paragraphs and reuses the shaped text of
    // the ones still on screen.
    let mut offset = px(0.);
    cx.bench_renderer(chat, move |chat, _, cx| {
        let max = chat.scroll.max_offset().y;
        offset += px(40.);
        if offset > max {
            offset = px(0.);
        }
        chat.scroll.set_offset(point(px(0.), -offset));
        cx.notify();
    });
}

gpui::bench_group!(benches, text_view_scroll);
gpui::bench_main!(benches);
