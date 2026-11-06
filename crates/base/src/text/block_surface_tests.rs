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

const DOCUMENT: &str = "before\n\n<!-- surface:summary -->\n\n# inside one\n\ninside **two**\n\n- three\n\n<!-- /surface -->\n\nafter";
const WITHOUT_FENCES: &str = "before\n\n# inside one\n\ninside **two**\n\n- three\n\nafter";

#[test]
fn a_document_without_fences_parses_the_same_with_or_without_a_registration() {
    assert_eq!(
        parsed(WITHOUT_FENCES, extensions()),
        parsed(WITHOUT_FENCES, MarkdownExtensions::default())
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
    let (root, cx) = window(DOCUMENT, counted(renders.clone()), false, cx);
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

#[test]
fn mdx_fails_the_whole_document_as_any_comment_does() {
    let mut cx = NodeContext {
        markdown_extensions: Arc::new(extensions().mdx()),
        ..NodeContext::default()
    };
    assert!(markdown::parse(DOCUMENT, &mut cx).is_err());
}
