use std::{cell::RefCell, rc::Rc};

use gpui::{AppContext, ParentElement as _, TestAppContext, div};

use super::{
    Diff, DiffDocument, DiffFile, DiffLineAnnotation, DiffLinePosition, DiffSide, DiffState,
};

#[gpui::test]
fn test_diff_builder(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let document = DiffDocument::new(
        DiffFile::new("before.rs", "let value = 1;\n"),
        DiffFile::new("after.rs", "let value = 2;\n"),
    );
    let state = cx.new(|cx| DiffState::new(document.clone(), cx));
    let defaults = Diff::new(&state);
    assert_eq!(defaults.state.entity_id(), state.entity_id());
    assert!(defaults.header && defaults.line_numbers);
    assert!(defaults.inline_highlight && defaults.syntax_highlight);
    assert!(defaults.annotations.is_empty());
    assert_eq!(defaults.layout_revision, 0);
    assert!(defaults.header_renderer.is_none());
    assert!(defaults.header_prefix_renderer.is_none());
    assert!(defaults.header_filename_suffix_renderer.is_none());
    assert!(defaults.header_metadata_renderer.is_none());
    assert!(defaults.annotation_renderer.is_none());

    let calls = Rc::new(RefCell::new(Vec::new()));
    let header_renderer = |slot| {
        let calls = calls.clone();
        move |document: &DiffDocument, _: &mut gpui::Window, _: &mut gpui::App| {
            assert_eq!(document.original().unwrap().name().as_str(), "before.rs");
            assert_eq!(document.modified().unwrap().name().as_str(), "after.rs");
            assert_eq!(
                document.original().unwrap().text().as_str(),
                "let value = 1;\n"
            );
            assert_eq!(
                document.modified().unwrap().text().as_str(),
                "let value = 2;\n"
            );
            calls.borrow_mut().push(slot);
            div().child(slot)
        }
    };
    let position = DiffLinePosition::new(DiffSide::Modified, 1);
    let annotation = DiffLineAnnotation::new("review-comment", position);
    let annotation_calls = Rc::new(RefCell::new(Vec::new()));
    let mut diff = Diff::new(&state)
        .header(false)
        .line_numbers(false)
        .inline_highlight(false)
        .syntax_highlight(false)
        .with_annotations([annotation.clone()])
        .with_layout_revision(7)
        .render_header(header_renderer("header"))
        .render_header_prefix(header_renderer("prefix"))
        .render_header_filename_suffix(header_renderer("suffix"))
        .render_header_metadata(header_renderer("metadata"))
        .render_annotation({
            let annotation_calls = annotation_calls.clone();
            move |annotation, _, _| {
                assert_eq!(annotation.id(), &"review-comment".into());
                annotation_calls.borrow_mut().push(annotation.position());
                div().child("Review comment")
            }
        });

    assert_eq!(diff.state.entity_id(), state.entity_id());
    assert!(!diff.header && !diff.line_numbers);
    assert!(!diff.inline_highlight && !diff.syntax_highlight);
    assert_eq!(diff.layout_revision, 7);
    assert_eq!(diff.annotations.len(), 1);
    assert_eq!(diff.annotations[0].id(), annotation.id());
    assert_eq!(diff.annotations[0].position(), position);

    let window = cx.add_empty_window();
    window.update(|window, cx| {
        // A complete header replacement suppresses the default-header slots.
        let _ = diff.file_header(&document, window, cx);
        assert_eq!(calls.borrow().as_slice(), &["header"]);
        diff.header_renderer = None;
        let _ = diff.file_header(&document, window, cx);
        assert_eq!(
            calls.borrow().as_slice(),
            &["header", "prefix", "suffix", "metadata"]
        );
        let render = diff.annotation_renderer.as_ref().unwrap();
        let _ = render(&diff.annotations[0], window, cx);
    });
    assert_eq!(annotation_calls.borrow().as_slice(), &[position]);
}
