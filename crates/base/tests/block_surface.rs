//! A block surface draws a blockquote that opens with a registered `[!name]`
//! marker inside a container, and keeps its blocks part of the document: a
//! selection, a find and a reveal go across and inside it. See
//! `MarkdownExtensions::block_surface`.
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

/// `body` in a `summary` surface: a blockquote under the marker `[!summary]`.
fn alert(body: &str) -> String {
    std::iter::once("[!summary]")
        .chain(body.lines())
        .map(|line| {
            if line.is_empty() {
                ">".to_string()
            } else {
                format!("> {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
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

const DOCUMENT: &str = "before text\n\n> [!summary]\n> inside one\n>\n> inside two\n\nafter text";

/// Anything else stays a blockquote, as `block_surface_tests` checks.
#[gpui::test]
fn a_registered_marker_draws_its_blockquote_in_a_panel(cx: &mut TestAppContext) {
    let mut doc = open(
        DOCUMENT,
        tall(true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    assert_eq!(doc.rows(), 3);
    // The marker line is not text.
    assert_eq!(
        doc.rendered(),
        "before text\ninside one\ninside two\nafter text\n"
    );
    let panel = doc.quads(PANEL);
    assert_eq!(panel.len(), 1);
    let line = doc.marked("inside one")[0];
    assert_eq!(line.left(), panel[0].left() + px(PADDING));
    assert!(panel[0].contains(&line.origin));
    for text in ["before text", "after text"] {
        assert!(!panel[0].contains(&doc.marked(text)[0].origin), "{text}");
    }
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
            ("before text", "inside one", "before text\ninside one"),
            ("inside two", "after text", "inside two\nafter text"),
            (
                "before text",
                "after text",
                "before text\ninside one\ninside two\nafter text",
            ),
        ] {
            assert_eq!(doc.drag_copy(from, to), copied, "scrollable: {scrollable}");
        }
        // The container's title is not text.
        let panel = doc.quads(PANEL)[0];
        let title = panel.origin + point(px(PADDING + 1.), px(PADDING + HEADER / 2.));
        let end = doc.end_of("inside");
        doc.drag(title, end);
        assert_eq!(doc.copied().trim_end(), "inside");
        doc.cx.update(TextSelection::clear);
        assert_eq!(
            doc.select_all(),
            "before text\ninside one\ninside two\nafter text\n"
        );
        drop(doc);
    }
}

/// A source copy of a surface is what a copy of the same blockquote is, under
/// the marker line, and the source verbatim where the selection covers all of it.
#[gpui::test]
fn a_source_copy_quotes_the_selection_under_the_marker(cx: &mut TestAppContext) {
    let source = format!("before\n\n{}\n\nafter", alert("first _em_\n\nmid\n\nlast"));
    for format in [SelectionFormat::Source, SelectionFormat::SourceAcrossStyles] {
        for scrollable in [false, true] {
            let mut doc = open(&source, tall(scrollable, format), extensions(), cx);
            let in_one_block = match format {
                SelectionFormat::Source => "> [!summary]\n> mid",
                _ => "mid",
            };
            for (from, to, copied) in [
                ("mid", "mid", in_one_block),
                ("first", "last", "> [!summary]\n> first *em*\n> mid\n> last"),
                (
                    "before",
                    "last",
                    "before\n\n> [!summary]\n> first *em*\n> mid\n> last",
                ),
                ("mid", "after", "> [!summary]\n> mid\n> last\n\nafter"),
                ("before", "after", &source),
            ] {
                assert_eq!(
                    doc.drag_copy(from, to),
                    copied,
                    "{format:?}, scrollable: {scrollable}"
                );
            }
            drop(doc);
        }
    }
}

#[gpui::test]
fn a_select_all_copies_the_source_verbatim(cx: &mut TestAppContext) {
    for source in [
        DOCUMENT.to_string(),
        format!("{}\n\n{}\n", alert("- one"), alert("- two")),
        "\n\n> [!Summary]  \n> one\n\n<!-- a comment -->\n\n- two\n\n\n".to_string(),
    ] {
        for format in [SelectionFormat::Source, SelectionFormat::SourceAcrossStyles] {
            let mut doc = open(&source, tall(true, format), extensions(), cx);
            assert_eq!(doc.select_all(), source);
            drop(doc);
        }
    }
}

// Find.

#[gpui::test]
fn highlights_and_source_positions_go_inside_and_across_the_edges(cx: &mut TestAppContext) {
    // `inside one` shares its source line run with the marker.
    let source = "before text\n\n> [!summary]\n> inside one\n> inside two\n\nafter text";
    let mut doc = open(source, tall(true, SelectionFormat::Plain), extensions(), cx);
    for needle in ["inside one", "inside two"] {
        let start = source.find(needle).unwrap();
        let mapped = doc.view.read_with(doc.cx, |view, _| {
            view.rendered_text()
                .range_for_source(start..start + needle.len())
        });
        assert_eq!(mapped, Some(doc.range_of(needle)), "{needle}");
    }
    let panel = doc.quads(PANEL)[0];

    // Inside: one quad, over that text, inside the panel's padding.
    let inside = doc.marked("two");
    assert_eq!(inside.len(), 1);
    assert!(panel.contains(&inside[0].origin));
    assert!(inside[0].left() > panel.left() + px(PADDING));

    // Across both edges: a quad per line, none over the title.
    let across = doc.marked("text\ninside one inside two\nafter");
    let in_panel: Vec<_> = across
        .iter()
        .map(|quad| panel.contains(&quad.origin))
        .collect();
    assert_eq!(in_panel, [false, true, false]);
    assert!(across[1].top() > panel.top() + px(PADDING + HEADER - 1.));
}

// Reveal.

#[gpui::test]
fn a_reveal_reaches_a_line_inside_a_surface(cx: &mut TestAppContext) {
    // A long surface is one row, so a line has to be scrolled to within it.
    let source = format!("{}\n\nafter", alert(&words(800)));
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
    drop(doc);

    let source = format!(
        "{}\n\n{}\n\n{}",
        paragraphs(200),
        alert("target line"),
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
fn a_reveal_shows_a_custom_block_inside_a_surface_and_settles(cx: &mut TestAppContext) {
    // Text above the card fills the view, so the line before it can be shown
    // without it. At 50px, the card is taller than the view and fills it.
    let body = format!("{}\n\n[card](target)\n\n{}", paragraphs(20), paragraphs(20));
    let source = format!("{}\n\nafter", alert(&body));
    for height in [200., 50.] {
        let mut doc = open(
            &source,
            (height, true, SelectionFormat::Plain),
            with_cards(extensions()),
            cx,
        );
        let range = doc.range_of("target");
        doc.view
            .update(doc.cx, |view, cx| view.reveal_range(range, cx).unwrap());
        let mut tops = Vec::new();
        for _ in 0..16 {
            doc.cx.update(|window, cx| window.draw(cx).clear(cx));
            tops.push(doc.scroll_top().offset_in_item);
        }
        assert!(
            tops[1..].iter().all(|top| *top == tops[1]),
            "it moved after the first frames: {tops:?}"
        );
        let viewport = doc.viewport();
        let card = doc.cx.debug_bounds("target-card").unwrap();
        let shown = if card.size.height <= viewport.size.height {
            card.top() >= viewport.top() - px(0.5) && card.bottom() <= viewport.bottom() + px(0.5)
        } else {
            card.top() <= viewport.top() + px(0.5) && card.bottom() >= viewport.bottom() - px(0.5)
        };
        assert!(shown, "{card:?} in {viewport:?}");
        drop(doc);
    }
}

// Height.

#[gpui::test]
fn a_surface_measures_as_its_blocks_and_its_container(cx: &mut TestAppContext) {
    let mut plain = open(
        "before text\n\ninside one\n\ninside two\n\nafter text",
        tall(true, SelectionFormat::Plain),
        MarkdownExtensions::default(),
        cx,
    );
    let blocks = [plain.row_height(1), plain.row_height(2)];
    drop(plain);
    let mut doc = open(
        DOCUMENT,
        tall(true, SelectionFormat::Plain),
        extensions(),
        cx,
    );
    // Each paragraph's gap is in its row. In the surface the last one's moves
    // below the container, so it is still one row's worth of each.
    assert_eq!(
        doc.row_height(1),
        blocks[0] + blocks[1] + px(HEADER + 2. * PADDING),
    );
    // The panel is the row, less that gap.
    assert_eq!(doc.quads(PANEL)[0].size.height, doc.row_height(1) - px(12.));
    drop(doc);

    // With nothing to show, it draws nothing.
    for body in ["", "\n[r]: http://x"] {
        let mut doc = open(
            &format!("{}\n\nafter", alert(body)),
            tall(true, SelectionFormat::Plain),
            extensions(),
            cx,
        );
        assert!(doc.quads(PANEL).is_empty(), "{body:?}");
        assert_eq!(doc.row_height(0), px(0.), "{body:?}");
        drop(doc);
    }
}

#[gpui::test]
fn a_surface_wraps_to_the_view_and_remeasures_at_a_new_width(cx: &mut TestAppContext) {
    // A column flex container, as a callout with a title row is, sizes a child
    // to its content unless the surface takes the row's width.
    let flex = MarkdownExtensions::default().block_surface("summary", |_, _| {
        div().flex().flex_col().bg(PANEL).p(px(PADDING))
    });
    let mut doc = open(
        &format!("{}\n\nafter", alert(&format!("- {}", words(200)))),
        tall(true, SelectionFormat::Plain),
        flex,
        cx,
    );
    let panel = doc.quads(PANEL)[0];
    assert_eq!(panel.size.width, px(WIDTH));
    let last = *doc.marked("w199").last().unwrap();
    assert!(last.right() <= panel.right() - px(PADDING) + px(0.5));
    assert!(last.top() > doc.marked("w0")[0].bottom(), "the item wraps");

    let (wide, wide_extent) = (doc.row_height(0), doc.extent());
    doc.root.update(doc.cx, |root, cx| {
        root.width = WIDTH / 2.;
        cx.notify();
    });
    settle(doc.cx);
    assert!(doc.row_height(0) > wide * 1.5);
    assert!(doc.extent() > wide_extent);
    assert_eq!(doc.quads(PANEL)[0].size.width, px(WIDTH / 2.));
}

// Streaming.

#[gpui::test]
fn a_stream_builds_what_a_whole_parse_does(cx: &mut TestAppContext) {
    let source = format!(
        "before\n\n{}\n\nafter",
        alert("# inside one\n\ninside **two**\n\n- three")
    );
    let mut whole = open(
        &source,
        tall(true, SelectionFormat::Source),
        extensions(),
        cx,
    );
    let expected = (whole.rendered(), whole.rows(), whole.row_height(1));
    drop(whole);

    // Every chunk size, the marker split included: 1 is each byte alone.
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
        let streamed = (doc.rendered(), doc.rows(), doc.row_height(1));
        assert_eq!(streamed, expected, "chunk {chunk}");
        drop(doc);
    }
}
