//! What of block surfaces only private state can tell. Their behavior is tested from
//! outside in `tests/block_surface.rs`.
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use gpui::{
    AppContext as _, Context, Entity, IntoElement, ParentElement as _, Pixels, Render, Styled as _,
    TestAppContext, VisualTestContext, Window, div, hsla, px,
};

use super::{
    MarkdownExtensions, SelectionFormat, TextView, TextViewState,
    format::markdown,
    node::{BlockNode, NodeContext},
};

fn extensions() -> MarkdownExtensions {
    MarkdownExtensions::default()
        .parser_revision(1)
        .block_surface("summary", |_, _| {
            div()
                .bg(hsla(0.6, 0.5, 0.9, 1.))
                .rounded(px(20.))
                .p(px(20.))
                .child(div().h(px(30.)).child("Summary"))
        })
}

fn parsed(source: &str, extensions: MarkdownExtensions) -> Vec<BlockNode> {
    let mut cx = NodeContext {
        markdown_extensions: Arc::new(extensions),
        ..NodeContext::default()
    };
    let document = markdown::parse(source, &mut cx).unwrap();
    document.blocks.as_ref().clone()
}

/// The surface `source` parses to, as its name and its children's Markdown,
/// or `None` when it parses to something else.
fn surface(source: &str) -> Option<(String, String)> {
    match &parsed(source, extensions())[..] {
        [BlockNode::Surface { name, children, .. }] => Some((
            name.to_string(),
            children
                .iter()
                .map(BlockNode::to_markdown)
                .collect::<Vec<_>>()
                .join("\n\n"),
        )),
        _ => None,
    }
}

#[test]
fn a_registered_marker_alone_on_the_first_line_makes_a_surface() {
    for (source, children) in [
        ("> [!summary]\n> one\n> two", "one two"),
        ("> [!SUMMARY]\n>\n> one", "one"),
        ("> [!Summary]  \n> one", "one"),
        ("> [!summary] \n> *one*", "*one*"),
        ("> [!summary]\nlazy", "lazy"),
        ("> [!summary]\n> - one\n> - two", "- one\n- two"),
        ("> [!summary]", ""),
    ] {
        assert_eq!(
            surface(source),
            Some(("summary".to_string(), children.to_string())),
            "{source}"
        );
    }
    // The marker is not its first paragraph's text, which starts after it.
    let source = "> [!summary]\n>  one";
    let [BlockNode::Surface { children, span, .. }] = &parsed(source, extensions())[..] else {
        panic!("a surface");
    };
    let BlockNode::Paragraph(paragraph) = &children[0] else {
        panic!("a paragraph");
    };
    assert_eq!(paragraph.span.unwrap().start, source.find("one").unwrap());
    assert_eq!(span.unwrap().start, 0);
}

#[test]
fn anything_else_parses_as_it_does_without_a_registration() {
    for source in [
        "> [!other]\n> one",
        "> [!summary] Title\n> one",
        "> [!summary]*one*",
        "> [!summary\n> one",
        "> \\[!summary]\n> one",
        "> &#91;!summary]\n> one",
        "> one\n> [!summary]",
        "> > [!summary]\n> > one",
        "- > [!summary]\n  > one",
        "[!summary]\n\none",
        "[!summary]: http://x\n\n> [!summary]\n> one",
    ] {
        assert_eq!(
            parsed(source, extensions()),
            parsed(source, MarkdownExtensions::default()),
            "{source}"
        );
    }
}

#[test]
fn a_surface_writes_markdown_as_its_marker_and_quoted_blocks() {
    let source = "> [!SUMMARY]\n> one\n>\n> - two";
    let blocks = parsed(source, extensions());
    assert_eq!(blocks[0].to_markdown(), "> [!summary]\n> one\n>\n> - two");
    assert_eq!(
        parsed(&blocks[0].to_markdown(), extensions()),
        parsed("> [!summary]\n> one\n>\n> - two", extensions())
    );
}

#[test]
fn registered_surfaces_are_part_of_the_parser_configuration() {
    let plain = MarkdownExtensions::default().parser_revision(1);
    assert!(!plain.has_same_parser_configuration(&extensions()));
    // Rebuilt during rendering, as a view's extensions commonly are.
    assert!(extensions().has_same_parser_configuration(&extensions()));
}

struct Root {
    view: Entity<TextViewState>,
    extensions: MarkdownExtensions,
    width: Pixels,
    scrollable: bool,
}

impl Render for Root {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(self.width)
            .h(px(600.))
            .text_size(px(16.))
            .child(crate::TextSelectionLayer)
            .child(
                TextView::new(&self.view)
                    .scrollable(self.scrollable)
                    .selection_format(SelectionFormat::Source)
                    .markdown_extensions(self.extensions.clone()),
            )
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn window<'a>(
    source: &str,
    extensions: MarkdownExtensions,
    scrollable: bool,
    cx: &'a mut TestAppContext,
) -> (Entity<Root>, &'a mut VisualTestContext) {
    cx.update(crate::init);
    let (root, cx) = cx.add_window_view(|_, cx| Root {
        view: cx.new(|cx| TextViewState::markdown(source, cx)),
        extensions,
        width: px(500.),
        scrollable,
    });
    let cx: &mut VisualTestContext = cx;
    settle(cx);
    (root, cx)
}

fn view(root: &Entity<Root>, cx: &mut VisualTestContext) -> Entity<TextViewState> {
    root.read_with(cx, |root, _| root.view.clone())
}

#[gpui::test]
fn a_new_container_needs_no_new_parse(cx: &mut TestAppContext) {
    let renders = Arc::new(AtomicUsize::new(0));
    let counted = |renders: Arc<AtomicUsize>| {
        MarkdownExtensions::default()
            .parser_revision(1)
            .block_surface("summary", move |_, _| {
                renders.fetch_add(1, Ordering::Relaxed);
                div()
            })
    };
    let (root, cx) = window("> [!summary]\n> one", counted(renders.clone()), false, cx);
    let view = view(&root, cx);
    let document = view.read_with(cx, |state, _| state.parsed_content.document.blocks.clone());
    let before = renders.load(Ordering::Relaxed);
    assert!(before > 0);
    root.update(cx, |root, cx| {
        // As a theme change does: a rebuilt registration with the same name.
        root.extensions = counted(renders.clone());
        cx.notify();
    });
    settle(cx);
    assert!(renders.load(Ordering::Relaxed) > before);
    view.read_with(cx, |state, _| {
        assert!(Arc::ptr_eq(
            &document,
            &state.parsed_content.document.blocks
        ));
    });
}
