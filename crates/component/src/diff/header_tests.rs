use std::{cell::RefCell, rc::Rc};

use gpui::{AppContext, ParentElement as _, TestAppContext, div};

use super::{
    Diff, DiffFile, DiffLineAnnotation, DiffLinePosition, DiffSide, DiffState, document::fixture,
};

#[gpui::test]
fn test_diff_builder(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let document = fixture::document(
        Some(("before.rs", "let value = 1;\n")),
        Some(("after.rs", "let value = 2;\n")),
    );
    let state = cx.new(|cx| DiffState::new([document.clone()], cx));
    let defaults = Diff::new(&state);
    assert_eq!(defaults.state.entity_id(), state.entity_id());
    assert!(defaults.header_visible && defaults.line_number);
    assert!(defaults.syntax_highlight);
    assert!(defaults.annotations.is_empty());
    assert!(defaults.header_renderer.is_none());
    assert!(defaults.annotation_renderer.is_none());

    let header_calls = Rc::new(RefCell::new(Vec::new()));
    let position = DiffLinePosition::new("after.rs", DiffSide::Modified, 1);
    let annotation = DiffLineAnnotation::new("review-comment", position.clone());
    let annotation_calls = Rc::new(RefCell::new(Vec::new()));
    let diff = Diff::new(&state)
        .header_visible(false)
        .line_number(false)
        .syntax_highlight(false)
        .annotations([annotation.clone()])
        .header({
            let header_calls = header_calls.clone();
            move |document: &DiffFile, _, _| {
                header_calls.borrow_mut().push((
                    document.original_path().cloned(),
                    document.modified_path().cloned(),
                ));
                div().child("header")
            }
        })
        .annotation_content({
            let annotation_calls = annotation_calls.clone();
            move |annotation, _, _| {
                assert_eq!(annotation.id(), &"review-comment".into());
                annotation_calls
                    .borrow_mut()
                    .push(annotation.position().clone());
                div().child("Review comment")
            }
        });

    assert_eq!(diff.state.entity_id(), state.entity_id());
    assert!(!diff.header_visible && !diff.line_number);
    assert!(!diff.syntax_highlight);
    assert_eq!(diff.annotations.len(), 1);
    assert_eq!(diff.annotations[0].id(), annotation.id());
    assert_eq!(diff.annotations[0].position(), &position);
    assert_eq!(diff.annotation_index[&position], [0]);

    let window = cx.add_empty_window();
    window.update(|window, cx| {
        let render = diff.header_renderer.as_ref().unwrap();
        let _ = render(&document, window, cx);
        let render = diff.annotation_renderer.as_ref().unwrap();
        let _ = render(&diff.annotations[0], window, cx);
    });
    assert_eq!(
        header_calls.borrow().as_slice(),
        &[(Some("before.rs".into()), Some("after.rs".into()))]
    );
    assert_eq!(annotation_calls.borrow().as_slice(), &[position]);
}
