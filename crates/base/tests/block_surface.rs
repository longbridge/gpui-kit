//! A block surface draws blocks inside a container and changes nothing else about
//! them: a selection, a find and a reveal go across and inside it as in a document
//! without one. See `MarkdownExtensions::block_surface`.
//!
//! Places in the text are found the way a find shows them: a range highlight in its
//! own color, read back from the quads of the frame. So a position is where the text
//! was really painted, on any platform's fonts.
#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;

use core::prelude::v1::test;
use gpui::{prelude::*, *};
use gpui_base::{
    TextSelection, TextSelectionLayer,
    text::{
        MarkdownExtensions, MarkdownNode, RangeHighlight, SelectionFormat, TextView, TextViewState,
        markdown_ast::Node,
    },
};
use std::ops::Range;

const HEADER: f32 = 30.;
const PADDING: f32 = 20.;
const WIDTH: f32 = 500.;
const PANEL: Hsla = Hsla {
    h: 0.6,
    s: 0.5,
    l: 0.9,
    a: 1.,
};
const MARK: Hsla = Hsla {
    h: 0.15,
    s: 1.,
    l: 0.5,
    a: 1.,
};

fn extensions() -> MarkdownExtensions {
    MarkdownExtensions::default().block_surface("summary", |_, _| {
        div()
            .bg(PANEL)
            .rounded(px(20.))
            .p(px(PADDING))
            .child(div().h(px(HEADER)).child("Summary"))
    })
}

fn fenced(body: &str) -> String {
    format!("<!-- surface:summary -->\n\n{body}\n\n<!-- /surface -->")
}

fn words(count: usize) -> String {
    (0..count)
        .map(|ix| format!("w{ix}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn paragraphs(count: usize) -> String {
    (0..count)
        .map(|ix| format!("paragraph {ix}"))
        .collect::<Vec<_>>()
        .join("\n\n")
}

struct Root {
    view: Entity<TextViewState>,
    width: f32,
    height: f32,
    scrollable: bool,
    format: SelectionFormat,
    extensions: MarkdownExtensions,
}

impl Render for Root {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(self.width))
            .h(px(self.height))
            .text_size(px(16.))
            .line_height(px(24.))
            .child(TextSelectionLayer)
            .child(
                TextView::new(&self.view)
                    .scrollable(self.scrollable)
                    .selection_format(self.format)
                    .markdown_extensions(self.extensions.clone()),
            )
    }
}

struct Doc<'a> {
    root: Entity<Root>,
    view: Entity<TextViewState>,
    cx: &'a mut VisualTestContext,
}

fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
}

fn open<'a>(
    source: &str,
    (height, scrollable, format): (f32, bool, SelectionFormat),
    extensions: MarkdownExtensions,
    cx: &'a mut TestAppContext,
) -> Doc<'a> {
    cx.update(gpui_base::init);
    let (root, cx) = cx.add_window_view(|_, cx| Root {
        view: cx.new(|cx| TextViewState::markdown(source, cx).selectable(true)),
        width: WIDTH,
        height,
        scrollable,
        format,
        extensions,
    });
    let view = root.read_with(cx, |root, _| root.view.clone());
    settle(cx);
    Doc { root, view, cx }
}

fn tall(scrollable: bool, format: SelectionFormat) -> (f32, bool, SelectionFormat) {
    (600., scrollable, format)
}

impl Doc<'_> {
    fn rendered(&mut self) -> String {
        self.view
            .read_with(self.cx, |view, _| view.rendered_text().as_str().to_string())
    }

    fn range_of(&mut self, needle: &str) -> Range<usize> {
        let start = self
            .rendered()
            .find(needle)
            .unwrap_or_else(|| panic!("{needle} is in the rendered text"));
        start..start + needle.len()
    }

    /// The quads painted in `color`, in window pixels, from the top of the window.
    fn quads(&mut self, color: Hsla) -> Vec<Bounds<Pixels>> {
        let background: Background = color.into();
        self.cx.update(|window, _| {
            let scale = window.scale_factor();
            let mut quads: Vec<_> = window
                .painted_quads()
                .into_iter()
                .filter(|quad| quad.background == background)
                .map(|quad| {
                    let bounds = quad.bounds;
                    Bounds::new(
                        point(
                            px(bounds.origin.x.as_f32() / scale),
                            px(bounds.origin.y.as_f32() / scale),
                        ),
                        size(
                            px(bounds.size.width.as_f32() / scale),
                            px(bounds.size.height.as_f32() / scale),
                        ),
                    )
                })
                .collect();
            quads.sort_by_key(|quad| quad.top());
            quads
        })
    }

    /// What highlighting `needle` paints.
    fn marked(&mut self, needle: &str) -> Vec<Bounds<Pixels>> {
        let range = self.range_of(needle);
        self.view.update(self.cx, |view, cx| {
            view.set_range_highlights([RangeHighlight::new(range, MARK)], cx)
                .unwrap();
        });
        settle(self.cx);
        let marked = self.quads(MARK);
        self.view
            .update(self.cx, |view, cx| view.clear_range_highlights(cx));
        settle(self.cx);
        marked
    }

    /// A point just inside the start of the text `needle` paints.
    fn start_of(&mut self, needle: &str) -> Point<Pixels> {
        let bounds = self.marked(needle)[0];
        point(bounds.left() + px(0.1), bounds.center().y)
    }

    /// A point just inside the end of the text `needle` paints.
    fn end_of(&mut self, needle: &str) -> Point<Pixels> {
        let marked = self.marked(needle);
        let bounds = marked.last().unwrap();
        point(bounds.right() - px(0.1), bounds.center().y)
    }

    fn drag(&mut self, from: Point<Pixels>, to: Point<Pixels>) {
        self.cx
            .simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
        settle(self.cx);
        self.cx
            .simulate_mouse_move(to, MouseButton::Left, Modifiers::default());
        settle(self.cx);
        self.cx
            .simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
        settle(self.cx);
    }

    /// What the app's copy reads, without writing to the clipboard.
    fn copied(&mut self) -> String {
        self.cx.update(TextSelection::selected_text)
    }

    /// What a drag from the start of `from` to the end of `to` copies, the same
    /// in both directions.
    fn drag_copy(&mut self, from: &str, to: &str) -> String {
        let (start, end) = (self.start_of(from), self.end_of(to));
        self.drag(start, end);
        let forward = self.copied();
        self.cx.update(TextSelection::clear);
        settle(self.cx);
        self.drag(end, start);
        assert_eq!(forward, self.copied(), "a drag copies the same both ways");
        self.cx.update(TextSelection::clear);
        settle(self.cx);
        forward.trim_end().to_string()
    }

    fn select_all(&mut self) -> String {
        self.view.update(self.cx, |view, cx| view.select_all(cx));
        settle(self.cx);
        self.copied()
    }

    fn reveal(&mut self, needle: &str) {
        let range = self.range_of(needle);
        self.view
            .update(self.cx, |view, cx| view.reveal_range(range, cx).unwrap());
        settle(self.cx);
    }

    fn viewport(&mut self) -> Bounds<Pixels> {
        self.view
            .read_with(self.cx, |view, _| view.list_state().viewport_bounds())
    }

    fn scroll_top(&mut self) -> ListOffset {
        self.view
            .read_with(self.cx, |view, _| view.list_state().logical_scroll_top())
    }

    fn rows(&mut self) -> usize {
        self.view
            .read_with(self.cx, |view, _| view.list_state().item_count())
    }

    fn row_height(&mut self, row: usize) -> Pixels {
        self.view.read_with(self.cx, |view, _| {
            view.list_state().bounds_for_item(row).unwrap().size.height
        })
    }

    fn extent(&mut self) -> Pixels {
        self.view.read_with(self.cx, |view, _| {
            let list = view.list_state();
            list.max_offset_for_scrollbar().y + list.viewport_bounds().size.height
        })
    }

    /// `needle` was revealed: all of the line it starts on is inside the viewport.
    #[track_caller]
    fn assert_shown(&mut self, needle: &str) {
        let viewport = self.viewport();
        let line = self.marked(needle)[0];
        assert!(
            line.top() >= viewport.top() - px(0.5) && line.bottom() <= viewport.bottom() + px(0.5),
            "{needle}: {line:?} in {viewport:?}"
        );
    }
}

/// A source, what to drag from and to, and what that copies.
const AS_EVER: &[(&str, &str, &str, &str)] = &[
    ("- a1\n\n* b1", "a1", "b1", "- a1\n\n- b1"),
    ("1. a1\n\n1) b1", "a1", "b1", "1. a1\n\n1. b1"),
    ("- a1\n- a2\n\n* b1", "a1", "b1", "- a1\n- a2\n\n- b1"),
    (
        "- a1\n\n<!-- c -->\n\n- b1",
        "a1",
        "b1",
        "- a1\n\n<!-- c -->\n\n- b1",
    ),
    (
        "one _em_\n\n---\n\n![](a.png)\n\n| a   | b |\n|-----|---|\n| 1   | 2 |\n\n[r]: http://x\n\ntwo",
        "one",
        "two",
        "one *em*\n\n---\n\n![](a.png)\n\n| a   | b |\n|-----|---|\n| 1   | 2 |\n\n[r]: http://x\n\ntwo",
    ),
    (
        "> quote\n\n```\ncode\n```\n\nlast",
        "quote",
        "last",
        "> quote\n\n```\ncode\n```\n\nlast",
    ),
];

const DOCUMENT: &str = "before text\n\n<!-- surface:summary -->\n\ninside one\n\ninside two\n\n<!-- /surface -->\n\nafter text";

// What gathers, and what does not.

#[gpui::test]
fn the_blocks_between_fences_draw_as_one_row_in_one_panel(cx: &mut TestAppContext) {
    let mut doc = open(
        DOCUMENT,
        tall(true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    assert_eq!(doc.rows(), 3);
    let panel = doc.quads(PANEL);
    assert_eq!(panel.len(), 1);
    for text in ["inside one", "inside two"] {
        let line = doc.marked(text)[0];
        assert_eq!(line.left(), panel[0].left() + px(PADDING), "{text}");
        assert!(panel[0].contains(&line.origin), "{text}");
    }
    for text in ["before text", "after text"] {
        let line = doc.marked(text)[0];
        assert!(!panel[0].contains(&line.origin), "{text}");
    }
}

#[gpui::test]
fn fences_need_no_blank_lines(cx: &mut TestAppContext) {
    let mut doc = open(
        "<!-- surface:summary -->\ninside\n<!-- /surface -->\nafter",
        tall(true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    assert_eq!(doc.rows(), 2);
    assert_eq!(doc.quads(PANEL).len(), 1);
}

#[gpui::test]
fn without_a_registration_a_fence_is_a_comment(cx: &mut TestAppContext) {
    let mut doc = open(
        DOCUMENT,
        tall(true, SelectionFormat::Plain),
        MarkdownExtensions::default().block_surface("other", |_, _| div().bg(PANEL)),
        cx,
    );
    // Before, comment, one, two, comment, after.
    assert_eq!(doc.rows(), 6);
    assert!(doc.quads(PANEL).is_empty());
    assert_eq!(
        doc.rendered(),
        "before text\ninside one\ninside two\nafter text\n"
    );
}

#[gpui::test]
fn a_closing_fence_with_nothing_open_is_a_comment(cx: &mut TestAppContext) {
    let mut doc = open(
        "one\n\n<!-- /surface -->\n\ntwo",
        tall(true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    assert_eq!(doc.rows(), 3);
    assert!(doc.quads(PANEL).is_empty());
}

#[gpui::test]
fn fences_are_only_for_the_top_level(cx: &mut TestAppContext) {
    const OPEN: &str = "<!-- surface:summary -->";
    for source in [
        format!("> {OPEN}\n>\n> quoted\n\nafter"),
        format!("- item\n\n  {OPEN}\n\n  listed\n\nafter"),
        format!("1. item\n\n   {OPEN}\n\n   listed\n\n   <!-- /surface -->\n\nafter"),
        format!("```\n{OPEN}\ncode\n```\n\nafter"),
        format!("    {OPEN}\n\nafter"),
        format!("text `{OPEN}` more\n\nafter"),
    ] {
        let mut doc = open(
            &source,
            tall(true, SelectionFormat::Plain),
            extensions(),
            cx,
        );
        assert_eq!(doc.rows(), 2, "{source}");
        assert!(doc.quads(PANEL).is_empty(), "{source}");
        assert!(doc.rendered().contains("after"), "{source}");
        drop(doc);
    }
}

#[gpui::test]
fn the_end_of_the_text_or_another_fence_ends_a_surface(cx: &mut TestAppContext) {
    let mut doc = open(
        "<!-- surface:summary -->\n\none\n\n<!-- surface:summary -->\n\ntwo",
        tall(true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    assert_eq!(doc.rows(), 2);
    assert_eq!(doc.quads(PANEL).len(), 2);
}

#[gpui::test]
fn a_surface_with_nothing_to_show_draws_nothing(cx: &mut TestAppContext) {
    for body in [
        "",
        "<!-- a comment -->",
        "[r]: http://x",
        "<!-- a comment -->\n\n[r]: http://x",
        "<br>",
    ] {
        let source = format!("{}\n\nafter", fenced(body));
        let mut doc = open(
            &source,
            tall(true, SelectionFormat::Plain),
            extensions(),
            cx,
        );
        assert!(doc.quads(PANEL).is_empty(), "{body}");
        assert_eq!(doc.row_height(0), px(0.), "{body}");
        drop(doc);
    }
}

#[gpui::test]
fn a_definition_after_the_last_block_leaves_no_gap_in_the_panel(cx: &mut TestAppContext) {
    let mut with = open(
        &format!("{}\n\nafter", fenced("[l][r]\n\n[r]: http://x")),
        tall(true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    let with_definition = with.row_height(0);
    drop(with);
    let mut without = open(
        &format!("{}\n\nafter\n\n[r]: http://x", fenced("[l][r]")),
        tall(true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    assert_eq!(with_definition, without.row_height(0));
}

// Copy.

#[gpui::test]
fn a_selection_copies_as_text_across_and_inside(cx: &mut TestAppContext) {
    for scrollable in [false, true] {
        let mut doc = open(
            DOCUMENT,
            tall(scrollable, SelectionFormat::Plain),
            extensions(),
            cx,
        );
        for (from, to, copied) in [
            ("inside one", "inside two", "inside one\ninside two"),
            (
                "before text",
                "inside two",
                "before text\ninside one\ninside two",
            ),
            (
                "inside one",
                "after text",
                "inside one\ninside two\nafter text",
            ),
            (
                "before text",
                "after text",
                "before text\ninside one\ninside two\nafter text",
            ),
        ] {
            assert_eq!(doc.drag_copy(from, to), copied, "scrollable: {scrollable}");
        }
        drop(doc);
    }
}

/// The source of `body` between its first and last paragraphs, dragged in from
/// inside the surface and across each of its edges.
fn assert_source_is_verbatim(body: &str, format: SelectionFormat, cx: &mut TestAppContext) {
    let source = format!("before\n\n{}\n\nafter", fenced(body));
    for scrollable in [false, true] {
        let mut doc = open(&source, tall(scrollable, format), extensions(), cx);
        for (from, to, copied) in [
            ("first", "last", body.to_string()),
            ("before", "last", format!("before\n\n{body}")),
            ("first", "after", format!("{body}\n\nafter")),
            ("before", "after", format!("before\n\n{body}\n\nafter")),
        ] {
            assert_eq!(doc.drag_copy(from, to), copied, "scrollable: {scrollable}");
        }
        drop(doc);
    }
}

#[gpui::test]
fn a_source_copy_keeps_blocks_with_no_text_inside_a_surface(cx: &mut TestAppContext) {
    assert_source_is_verbatim(
        "first\n\n---\n\n![](a.png)\n\nlast",
        SelectionFormat::Source,
        cx,
    );
}

#[gpui::test]
fn a_source_copy_keeps_what_the_author_wrote_inside_a_surface(cx: &mut TestAppContext) {
    assert_source_is_verbatim(
        "first\n\n_italic_ a \\*b\\* \\<!-- c [l][r]\n\n| a   | b |\n|-----|---|\n| 1   | 2 |\n\n[r]: http://x\n\nlast",
        SelectionFormat::Source,
        cx,
    );
}

#[gpui::test]
fn across_styles_a_copy_is_source_across_blocks_and_edges(cx: &mut TestAppContext) {
    assert_source_is_verbatim(
        "first\n\n---\n\n_italic_ \\*b\\*\n\nlast",
        SelectionFormat::SourceAcrossStyles,
        cx,
    );
}

#[gpui::test]
fn across_styles_a_copy_in_one_block_of_a_surface_is_its_text(cx: &mut TestAppContext) {
    let source = format!("before\n\n{}", fenced("one **bold** run\n\nsecond"));
    let mut doc = open(
        &source,
        tall(true, SelectionFormat::SourceAcrossStyles),
        extensions(),
        cx,
    );
    assert_eq!(doc.drag_copy("bold", "bold"), "bold");
    assert_eq!(doc.drag_copy("one", "run"), "one **bold** run");
}

#[gpui::test]
fn a_partial_source_copy_takes_only_what_is_selected(cx: &mut TestAppContext) {
    let source = format!(
        "before\n\n{}",
        fenced("skipped\n\none **two** three\n\nlast")
    );
    let mut doc = open(
        &source,
        tall(false, SelectionFormat::Source),
        extensions(),
        cx,
    );
    assert_eq!(doc.drag_copy("two", "last"), "**two** three\n\nlast");
    assert_eq!(doc.drag_copy("before", "one"), "before\n\nskipped\n\none");
}

/// These strings are what `TextView` copied before block surfaces existed.
#[gpui::test]
fn a_document_without_a_surface_copies_as_ever(cx: &mut TestAppContext) {
    for (source, from, to, copied) in AS_EVER {
        for scrollable in [false, true] {
            let mut doc = open(
                source,
                tall(scrollable, SelectionFormat::Source),
                MarkdownExtensions::default(),
                cx,
            );
            assert_eq!(doc.drag_copy(from, to), *copied, "{source}");
            drop(doc);
        }
    }
}

#[gpui::test]
fn a_select_all_leaves_out_fences_and_their_lines(cx: &mut TestAppContext) {
    for (source, copied) in [
        (fenced("only"), "only".to_string()),
        (
            DOCUMENT.to_string(),
            "before text\n\ninside one\n\ninside two\n\nafter text".to_string(),
        ),
        (
            "<!-- surface:summary -->\nno blank lines\n<!-- /surface -->\nafter".to_string(),
            "no blank lines\n\nafter".to_string(),
        ),
        (
            format!("{}\n\n{}\n", fenced("one"), fenced("two")),
            "one\n\ntwo".to_string(),
        ),
        (
            format!("{}\n\nafter", fenced("[r]: http://x")),
            "[r]: http://x\n\nafter".to_string(),
        ),
    ] {
        let mut doc = open(
            &source,
            tall(true, SelectionFormat::Source),
            extensions(),
            cx,
        );
        assert_eq!(doc.select_all(), copied, "{source}");
        drop(doc);
    }
}

#[gpui::test]
fn a_select_all_without_a_surface_is_verbatim(cx: &mut TestAppContext) {
    let source = "\n\none\n\n<!-- a comment -->\n\n- two\n\n\n";
    let mut doc = open(
        source,
        tall(true, SelectionFormat::Source),
        extensions(),
        cx,
    );
    assert_eq!(doc.select_all(), source);
}

#[gpui::test]
fn a_select_all_as_text_has_no_fences_and_no_title(cx: &mut TestAppContext) {
    let mut doc = open(
        DOCUMENT,
        tall(true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    assert_eq!(
        doc.select_all(),
        "before text\ninside one\ninside two\nafter text\n"
    );
}

#[gpui::test]
fn the_title_is_not_selectable_text(cx: &mut TestAppContext) {
    let mut doc = open(
        DOCUMENT,
        tall(false, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    let panel = doc.quads(PANEL)[0];
    let title = panel.origin + point(px(PADDING + 1.), px(PADDING + HEADER / 2.));
    let in_the_line = doc.end_of("inside");
    doc.drag(title, in_the_line);
    assert_eq!(doc.copied().trim_end(), "inside");
}

// Find.

#[gpui::test]
fn the_rendered_text_is_what_it_is_without_fences(cx: &mut TestAppContext) {
    let mut doc = open(
        DOCUMENT,
        tall(true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    let rendered = doc.rendered();
    drop(doc);
    let mut plain = open(
        "before text\n\ninside one\n\ninside two\n\nafter text",
        tall(true, SelectionFormat::Plain),
        MarkdownExtensions::default(),
        cx,
    );
    assert_eq!(rendered, plain.rendered());
}

#[gpui::test]
fn source_positions_map_through_the_fences(cx: &mut TestAppContext) {
    let mut doc = open(
        DOCUMENT,
        tall(true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    let expected = doc.range_of("inside two");
    let start = DOCUMENT.find("inside two").unwrap();
    let mapped = doc.view.read_with(doc.cx, |view, _| {
        view.rendered_text().range_for_source(start..start + 10)
    });
    assert_eq!(mapped, Some(expected));
}

#[gpui::test]
fn a_highlight_paints_inside_a_surface_and_across_its_edges(cx: &mut TestAppContext) {
    let mut doc = open(
        DOCUMENT,
        tall(true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    let panel = doc.quads(PANEL)[0];

    // Inside: one quad, over that text, inside the panel's padding.
    let inside = doc.marked("two");
    assert_eq!(inside.len(), 1);
    assert!(panel.contains(&inside[0].origin));
    assert!(inside[0].left() > panel.left() + px(PADDING));
    let before_two = doc.marked("inside ")[0];
    assert!(inside[0].left() >= before_two.right() - px(1.));

    // Across both edges: a quad per block, none over a separator or the title.
    let across = doc.marked("text\ninside one\ninside two\nafter");
    assert_eq!(across.len(), 4);
    let in_panel: Vec<_> = across
        .iter()
        .map(|quad| panel.contains(&quad.origin))
        .collect();
    assert_eq!(in_panel, [false, true, true, false]);
    assert!(across[1].top() > panel.top() + px(PADDING + HEADER - 1.));
}

// Reveal.

#[gpui::test]
fn a_reveal_reaches_a_line_inside_a_surface(cx: &mut TestAppContext) {
    // A long surface, which is one row: the line has to be scrolled to within it.
    let source = format!("{}\n\nafter", fenced(&words(800)));
    let mut doc = open(
        &source,
        (100., true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    for word in ["w790", "w400", "w5"] {
        doc.reveal(word);
        assert_eq!(doc.scroll_top().item_ix, 0, "{word}");
        doc.assert_shown(word);
    }
}

#[gpui::test]
fn a_reveal_reaches_a_surface_off_screen(cx: &mut TestAppContext) {
    let source = format!(
        "{}\n\n{}\n\n{}",
        paragraphs(200),
        fenced("target line"),
        paragraphs(50)
    );
    let mut doc = open(
        &source,
        (100., true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    doc.reveal("target line");
    doc.assert_shown("target line");
    doc.reveal("paragraph 3");
    assert!(doc.scroll_top().item_ix <= 3);
    doc.assert_shown("paragraph 3");
}

/// A paragraph `[card](…)` as a block of its own, 80px tall.
fn with_cards(extensions: MarkdownExtensions) -> MarkdownExtensions {
    extensions
        .block_parser(|node, _| {
            let Node::Paragraph(paragraph) = node else {
                return None;
            };
            let [Node::Link(link)] = &paragraph.children[..] else {
                return None;
            };
            Some(MarkdownNode::new("card", ()).text(link.url.clone()))
        })
        .block_renderer("card", |node, _, _| {
            let selector: &'static str = if node.as_text() == "target" {
                "target-card"
            } else {
                "other-card"
            };
            div().h(px(80.)).debug_selector(|| selector.to_string())
        })
}

#[gpui::test]
fn a_reveal_shows_all_of_a_custom_block_inside_a_surface(cx: &mut TestAppContext) {
    // Text above it fills the view, so the line before it can be shown without it.
    let body = format!("{}\n\n[card](target)\n\n{}", paragraphs(20), paragraphs(20));
    let source = format!("{}\n\nafter", fenced(&body));
    let mut doc = open(
        &source,
        (200., true, SelectionFormat::Plain),
        with_cards(extensions()),
        cx,
    );
    doc.reveal("target");
    settle(doc.cx);
    let viewport = doc.viewport();
    let card = doc.cx.debug_bounds("target-card").unwrap();
    assert!(
        card.top() >= viewport.top() - px(0.5) && card.bottom() <= viewport.bottom() + px(0.5),
        "{card:?} in {viewport:?}"
    );
}

#[gpui::test]
fn a_reveal_of_a_custom_block_taller_than_the_view_settles(cx: &mut TestAppContext) {
    let body = format!("{}\n\n[card](target)\n\n{}", paragraphs(20), paragraphs(20));
    let source = format!("{}\n\nafter", fenced(&body));
    let mut doc = open(
        &source,
        (50., true, SelectionFormat::Plain),
        with_cards(extensions()),
        cx,
    );
    let range = doc.range_of("target");
    doc.view
        .update(doc.cx, |view, cx| view.reveal_range(range, cx).unwrap());
    let mut tops = Vec::new();
    for _ in 0..16 {
        doc.cx.update(|window, cx| window.draw(cx).clear(cx));
        let top = doc.scroll_top();
        tops.push(top.offset_in_item);
    }
    assert!(
        tops[1..].iter().all(|top| *top == tops[1]),
        "it moved after the first frames: {tops:?}"
    );
    let viewport = doc.viewport();
    let card = doc.cx.debug_bounds("target-card").unwrap();
    assert!(
        card.top() <= viewport.top() + px(0.5) && card.bottom() >= viewport.bottom() - px(0.5),
        "{card:?} fills {viewport:?}"
    );
}

// Height.

#[gpui::test]
fn a_surface_measures_as_its_blocks_and_its_container(cx: &mut TestAppContext) {
    let mut plain = open(
        DOCUMENT,
        tall(true, SelectionFormat::Plain),
        MarkdownExtensions::default(),
        cx,
    );
    assert_eq!(plain.rows(), 6);
    let blocks = [plain.row_height(2), plain.row_height(3)];
    drop(plain);
    let mut doc = open(
        DOCUMENT,
        tall(true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    assert_eq!(doc.rows(), 3);
    // Each paragraph's gap is in its row. In the surface the last one's moves below
    // the container, so it is still one row's worth of each.
    assert_eq!(
        doc.row_height(1),
        blocks[0] + blocks[1] + px(HEADER + 2. * PADDING),
    );
    // The panel is the row, less that gap.
    assert_eq!(doc.quads(PANEL)[0].size.height, doc.row_height(1) - px(12.));
}

#[gpui::test]
fn a_surface_wraps_inside_its_padding_and_remeasures_at_a_new_width(cx: &mut TestAppContext) {
    let source = format!("{}\n\nafter", fenced(&words(200)));
    // Comment, paragraph, comment, after; the paragraph wraps at the full width.
    let mut plain = open(
        &source,
        tall(true, SelectionFormat::Plain),
        MarkdownExtensions::default(),
        cx,
    );
    let full_width = plain.row_height(1);
    drop(plain);
    let mut doc = open(
        &source,
        tall(true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    let (wide, wide_extent) = (doc.row_height(0), doc.extent());
    // The padding takes width from the text, so it wraps into more lines.
    assert!(
        wide > full_width + px(HEADER + 2. * PADDING),
        "{wide:?} {full_width:?}"
    );
    doc.root.update(doc.cx, |root, cx| {
        root.width = WIDTH / 2.;
        cx.notify();
    });
    settle(doc.cx);
    let narrow = doc.row_height(0);
    assert!(narrow > wide * 1.5, "{wide:?} to {narrow:?}");
    assert!(doc.extent() > wide_extent);
    assert_eq!(doc.quads(PANEL)[0].size.width, px(WIDTH / 2.));
}

// Streaming.

#[gpui::test]
fn a_stream_builds_what_a_whole_parse_does(cx: &mut TestAppContext) {
    let source = format!(
        "before\n\n{}\n\nafter",
        fenced("# inside one\n\ninside **two**\n\n- three")
    );
    let mut whole = open(
        &source,
        tall(true, SelectionFormat::Source),
        extensions(),
        cx,
    );
    let expected = (
        whole.rendered(),
        whole.rows(),
        whole.row_height(1),
        whole.select_all(),
    );
    drop(whole);

    // Every chunk size, fences split included: 1 is each byte alone.
    for chunk in [1, 7, 30] {
        let mut doc = open(
            &source[..chunk],
            tall(true, SelectionFormat::Source),
            extensions(),
            cx,
        );
        let mut at = chunk;
        while at < source.len() {
            let end = (at + chunk).min(source.len());
            let piece = source[at..end].to_string();
            doc.view
                .update(doc.cx, |view, cx| view.push_str(&piece, cx));
            settle(doc.cx);
            at = end;
        }
        assert_eq!(doc.quads(PANEL).len(), 1, "chunk {chunk}");
        let streamed = (
            doc.rendered(),
            doc.rows(),
            doc.row_height(1),
            doc.select_all(),
        );
        assert_eq!(streamed, expected, "chunk {chunk}");
        drop(doc);
    }
}
