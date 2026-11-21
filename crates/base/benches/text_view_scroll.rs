//! Scrolling a long Markdown `TextView`: one frame per iteration, the way a
//! chat scrolls. Every frame re-renders and re-lays out the visible
//! paragraphs, so this is where the per-paragraph text costs (shaping-cache
//! lookups, line wrappers, measurement) show up. Resizing one long paragraph
//! is where wrapping costs show up: every frame lays it out again.
//!
//! Run with `cargo bench -p gpui-base --bench text_view_scroll`.

#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;
#[cfg(feature = "gpui-fast")]
extern crate gpui_fast_platform as gpui_platform;

use std::fmt::Write as _;

use gpui::{
    AppContext as _, BenchAppContext, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Pixels, Render, ScrollHandle, StatefulInteractiveElement as _, Styled as _,
    Window, div, point, prelude::FluentBuilder as _, px, relative,
};
use gpui_base::text::{TextView, TextViewState};

/// A long assistant answer: headings, emphasis, links, lists, a code block,
/// and plenty of prose paragraphs.
fn markdown() -> String {
    markdown_with(false)
}

/// The same answer with inline code in every paragraph and list item, the
/// way a coding assistant's answers name identifiers and paths: those
/// paragraphs are laid out by `InlineFlow` instead of one `Inline`.
fn markdown_with_inline_code() -> String {
    markdown_with(true)
}

fn markdown_with(inline_code: bool) -> String {
    let mut out = String::new();
    let (c, s) = if inline_code { ("`", "`") } else { ("", "") };
    for section in 0..24 {
        let _ = writeln!(out, "## Section {section}: what the data shows\n");
        for paragraph in 0..4 {
            let _ = writeln!(
                out,
                "Paragraph {paragraph} of section {section}. The **dip window** is {c}08-18{s} → {c}08-26{s} \
                 (trough $208.48 on 08-24), followed by a post-earnings jump. Let me confirm the \
                 earnings date and date the specific news events in that window, then weave in \
                 [the filing](https://example.com/filing/{section}) and the *guidance* update so \
                 the numbers line up with what the market priced in before the open.\n"
            );
        }
        let _ = writeln!(
            out,
            "- FOMC 09-16: +25bp to {c}3.75–4.00%{s}, 12-0. First hike since July 2023."
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

/// A streamed reply that never starts a new paragraph: prose with inline code
/// and bold, five hundred chunks long, so one `InlineFlow` wraps it into a
/// few hundred lines.
fn long_paragraph() -> String {
    "Some prose with `inline code` and **bold text** here. ".repeat(500)
}

struct Chat {
    /// The parsed answer, held the way a chat holds a streamed message.
    answer: Entity<TextViewState>,
    scroll: ScrollHandle,
    /// Shows the answer in a bubble that shrinks to its text, the way a chat
    /// shows a reply, so taffy measures each paragraph at four widths a frame.
    bubble: bool,
}

impl Render for Chat {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let answer = TextView::new(&self.answer);
        div().size_full().child(
            div()
                .id("viewport")
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .child(div().w_full().px(px(20.)).map(|this| {
                    if self.bubble {
                        this.flex().child(
                            div()
                                .flex()
                                .flex_col()
                                .items_start()
                                .max_w(relative(0.8))
                                .child(answer),
                        )
                    } else {
                        this.child(answer)
                    }
                })),
        )
    }
}

#[gpui::bench]
fn text_view_scroll(cx: &mut BenchAppContext) {
    scroll_answer(markdown(), false, cx);
}

#[gpui::bench]
fn text_view_scroll_inline_code(cx: &mut BenchAppContext) {
    scroll_answer(markdown_with_inline_code(), false, cx);
}

#[gpui::bench]
fn text_view_scroll_inline_code_bubble(cx: &mut BenchAppContext) {
    scroll_answer(markdown_with_inline_code(), true, cx);
}

fn scroll_answer(markdown: String, bubble: bool, cx: &mut BenchAppContext) {
    cx.update(gpui_base::init);
    let mut window = cx.add_empty_window();
    let chat = window.update(|window, cx| {
        window.replace_root(cx, |_, cx| Chat {
            answer: cx.new(|cx| TextViewState::markdown(&markdown, cx)),
            scroll: ScrollHandle::new(),
            bubble,
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

/// The long paragraph at a new width every frame, as when the window is
/// resized: the flow's layout cache misses and the paragraph is wrapped and
/// laid out again, as it is when a streamed chunk is appended.
struct Resize {
    answer: Entity<TextViewState>,
    width: Pixels,
}

impl Render for Resize {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(self.width).child(TextView::new(&self.answer))
    }
}

#[gpui::bench]
fn text_view_resize_long_paragraph(cx: &mut BenchAppContext) {
    cx.update(gpui_base::init);
    let mut window = cx.add_empty_window();
    let markdown = long_paragraph();
    let resize = window.update(|window, cx| {
        window.replace_root(cx, |_, cx| Resize {
            answer: cx.new(|cx| TextViewState::markdown(&markdown, cx)),
            width: px(480.),
        })
    });
    cx.run_until_idle();

    // More widths than a flow keeps a layout for, so every frame misses.
    let mut frame = 0;
    cx.bench_renderer(resize, move |resize, _, cx| {
        frame += 1;
        resize.width = px(480. + (frame % 64) as f32);
        cx.notify();
    });
}

/// The long paragraph redrawn at the same width, as when the cursor blinks or
/// another part of the window changes: the flow finds its layout cached, so
/// what remains is what the paragraph rebuilds to find it.
#[gpui::bench]
fn text_view_redraw_long_paragraph(cx: &mut BenchAppContext) {
    cx.update(gpui_base::init);
    let mut window = cx.add_empty_window();
    let markdown = long_paragraph();
    let resize = window.update(|window, cx| {
        window.replace_root(cx, |_, cx| Resize {
            answer: cx.new(|cx| TextViewState::markdown(&markdown, cx)),
            width: px(480.),
        })
    });
    cx.run_until_idle();

    cx.bench_renderer(resize, move |_, _, cx| cx.notify());
}

gpui::bench_group!(
    benches,
    text_view_scroll,
    text_view_scroll_inline_code,
    text_view_scroll_inline_code_bubble,
    text_view_resize_long_paragraph,
    text_view_redraw_long_paragraph
);
gpui::bench_main!(benches);
