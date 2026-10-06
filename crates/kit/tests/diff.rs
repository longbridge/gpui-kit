#![cfg(all(feature = "test-support", feature = "component"))]

mod common;

use gpui_kit::component::{
    button::Button,
    diff::{
        Diff, DiffDocument, DiffLineAnnotation, DiffLinePosition, DiffMode, DiffSide, DiffState,
    },
    input::{Input, InputState},
};
use gpui_kit::test::{TestSupportExt as _, TestWindowExt as _};
use gpui_kit::{
    App, AppContext, Context, Entity, Focusable as _, InputEvent as _, Modifiers, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Role, TestAppContext, Window,
    WindowHandle, div, point, prelude::*, px, size,
};

struct Review {
    state: Entity<DiffState>,
    annotations: Vec<DiffLineAnnotation>,
    editor: Option<Entity<InputState>>,
}

impl Render for Review {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let editor = self.editor.clone();
        div()
            .id("review")
            .test_support()
            .size_full()
            .flex()
            .flex_col()
            .child(Button::new("before-diff").label("Before diff"))
            .child(
                Diff::new(&self.state)
                    .header_visible(false)
                    .flex_1()
                    .min_h_0()
                    .annotations(self.annotations.clone())
                    .annotation(move |_, _, _| {
                        div()
                            .id("comment-content")
                            .test_support()
                            .child(match &editor {
                                Some(editor) => Input::new(editor)
                                    .id("annotation-input")
                                    .w_full()
                                    .into_any_element(),
                                None => div().child("Review the original line").into_any_element(),
                            })
                    }),
            )
    }
}

fn patch_document(hunks: &str) -> DiffDocument {
    DiffDocument::parse(&format!("--- a/review.txt\n+++ b/review.txt\n{hunks}"))
        .expect("Test patch is valid")
        .into_iter()
        .next()
        .expect("Test patch contains a file")
}

fn review(
    cx: &mut TestAppContext,
    patch: &str,
    context: Option<usize>,
    mode: DiffMode,
    annotations: Vec<DiffLineAnnotation>,
) -> (WindowHandle<gpui_kit::base::Root>, Entity<DiffState>) {
    review_documents(cx, vec![patch_document(patch)], context, mode, annotations)
}

fn review_documents(
    cx: &mut TestAppContext,
    documents: Vec<DiffDocument>,
    context: Option<usize>,
    mode: DiffMode,
    annotations: Vec<DiffLineAnnotation>,
) -> (WindowHandle<gpui_kit::base::Root>, Entity<DiffState>) {
    cx.update(gpui_kit::init);
    let (handle, view) = common::open_window(cx, Some(size(px(800.), px(480.))), |_, cx| {
        cx.new(|cx| Review {
            state: cx.new(|cx| {
                DiffState::new(documents, cx)
                    .with_context_lines(context)
                    .with_mode(mode)
            }),
            annotations,
            editor: None,
        })
    });
    let state = cx.update(|cx| view.read(cx).state.clone());
    (handle, state)
}

fn shift_click(window: &mut Window, position: Point<Pixels>, cx: &mut App) {
    let modifiers = Modifiers {
        shift: true,
        ..Default::default()
    };
    window.dispatch_event(
        MouseMoveEvent {
            position,
            pressed_button: None,
            modifiers,
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
    window.dispatch_event(
        MouseDownEvent {
            position,
            button: MouseButton::Left,
            modifiers,
            click_count: 1,
            first_mouse: false,
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
    window.dispatch_event(
        MouseUpEvent {
            position,
            button: MouseButton::Left,
            modifiers,
            click_count: 1,
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
}

fn assert_clipboard(cx: &mut App, expected: &str) {
    assert_eq!(
        cx.read_from_clipboard()
            .and_then(|item| item.text())
            .as_deref(),
        Some(expected),
    );
}

#[gpui_kit::test]
fn gutter_keyboard_selection_copies_exact_source_and_survives_mode_change(cx: &mut TestAppContext) {
    let (handle, state) = review(
        cx,
        "@@ -1,3 +1,3 @@\n before\r\n-old\r\n+new\r\n \tlast 🦀\r\n",
        None,
        DiffMode::Split,
        vec![],
    );
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.within(("new", 1usize)).click(("line", 1usize), cx);
        assert_eq!(
            state.read(cx).selected_lines().unwrap().side(),
            DiffSide::Modified
        );
        window.within("review").press("shift-down", cx);
        window.within("review").press("secondary-c", cx);
        assert_clipboard(cx, "new\r\n\tlast 🦀\r\n");
        let selection = state.read(cx).selected_lines();
        state.update(cx, |state, cx| state.set_mode(DiffMode::Unified, cx));
        window.render_frame(cx);
        assert_eq!(state.read(cx).selected_lines(), selection);
        window.within("review").press("secondary-c", cx);
        assert_clipboard(cx, "new\r\n\tlast 🦀\r\n");
    })
    .unwrap();
}

#[gpui_kit::test]
fn double_click_on_a_later_source_line_copies_that_word(cx: &mut TestAppContext) {
    let (handle, state) = review(
        cx,
        "@@ -1,3 +1,3 @@\n first\n-previous\n+current\n last\n",
        None,
        DiffMode::Split,
        vec![],
    );
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.within(("new", 1usize)).double_click("source", cx);
        assert!(state.read(cx).selected_lines().is_none());
        window.within("review").press("secondary-c", cx);
        assert_clipboard(cx, "current");
    })
    .unwrap();
}

#[gpui_kit::test]
fn split_text_drag_keeps_the_original_side_when_pointer_crosses_the_divider(
    cx: &mut TestAppContext,
) {
    let (handle, state) = review(
        cx,
        "@@ -1,2 +1,2 @@\n-old-one\r\n-old-two\r\n+new-one\r\n+new-two\r\n",
        None,
        DiffMode::Split,
        vec![],
    );
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let original = window.within(("old", 0usize)).find("source").bounds();
        let modified = window.within(("new", 1usize)).find("source").bounds();
        window.drag(
            point(original.left(), original.center().y),
            point(modified.right() - px(1.), modified.center().y),
            cx,
        );
        assert!(state.read(cx).selected_lines().is_none());
        window.within("review").press("secondary-c", cx);
        assert_clipboard(cx, "old-one\r\nold-two");
    })
    .unwrap();
}

#[gpui_kit::test]
fn keyboard_expansion_exposes_original_annotation_on_an_unchanged_unified_line(
    cx: &mut TestAppContext,
) {
    let (handle, _) = review(
        cx,
        "@@ -1,5 +1,5 @@\n first\n second\n-old\n+new\n fourth\n fifth\n",
        Some(0),
        DiffMode::Unified,
        vec![DiffLineAnnotation::new(
            "original-comment",
            DiffLinePosition::new(0, DiffSide::Original, 1),
        )],
    );
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("comment-content").is_none());
        // Tab only reaches Root's binding once something holds focus, and a
        // button does not take focus on click; start from the first stop.
        window.focus_next(cx);
        window.render_frame(cx);
        assert_eq!(window.find("before-diff").focused(), Some(true));
        // Traverse the real Tab order instead of focusing the fold programmatically.
        for _ in 0..8 {
            if window.find(("expand", 0usize)).focused() == Some(true) {
                break;
            }
            window.press("tab", cx);
        }
        assert_eq!(window.find(("expand", 0usize)).focused(), Some(true));
        window.within("review").press("enter", cx);
        assert!(window.try_find(("expand", 0usize)).is_none());
        assert!(
            window
                .within("original-comment")
                .find("comment-content")
                .visible()
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn shift_gutter_click_extends_from_anchor_and_resets_when_side_changes(cx: &mut TestAppContext) {
    let (handle, state) = review(
        cx,
        "@@ -1,3 +1,3 @@\n one\r\n-old\r\n+new\r\n three\r\n",
        None,
        DiffMode::Split,
        vec![],
    );
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.within(("old", 0usize)).click(("line", 0usize), cx);
        for (ix, expected) in [
            (2usize, "one\r\nold\r\nthree\r\n"),
            (1usize, "one\r\nold\r\n"),
        ] {
            let target = window
                .within(("old", ix))
                .find(("line", ix))
                .bounds()
                .center();
            shift_click(window, target, cx);
            let range = state.read(cx).selected_lines().unwrap();
            assert_eq!(range.side(), DiffSide::Original);
            assert_eq!(range.start(), 1);
            assert_eq!(range.end(), ix + 1);
            window.within("review").press("secondary-c", cx);
            assert_clipboard(cx, expected);
        }
        let target = window
            .within(("new", 1usize))
            .find(("line", 1usize))
            .bounds()
            .center();
        shift_click(window, target, cx);
        let range = state.read(cx).selected_lines().unwrap();
        assert_eq!(range.side(), DiffSide::Modified);
        assert_eq!((range.start(), range.end()), (2, 2));
        window.within("review").press("secondary-c", cx);
        assert_clipboard(cx, "new\r\n");
    })
    .unwrap();
}

#[gpui_kit::test]
fn annotation_input_retains_focus_and_handles_its_own_text_commands(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, view) = common::open_window(cx, Some(size(px(800.), px(480.))), |window, cx| {
        cx.new(|cx| Review {
            state: cx.new(|cx| DiffState::new([patch_document("@@ -1 +1 @@\n-old\n+new\n")], cx)),
            annotations: vec![DiffLineAnnotation::new(
                "editable-comment",
                DiffLinePosition::new(0, DiffSide::Modified, 1),
            )],
            editor: Some(cx.new(|cx| InputState::new(window, cx))),
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window
            .within("editable-comment")
            .click("annotation-input", cx);
        assert_eq!(window.find("annotation-input").focused(), Some(true));
        assert!(
            !view
                .read(cx)
                .state
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
        window
            .within("editable-comment")
            .input("review note 🦀", cx);
        window.within("editable-comment").press("secondary-a", cx);
        window.within("editable-comment").press("secondary-c", cx);
        assert_clipboard(cx, "review note 🦀");
        assert_eq!(
            window.find("annotation-input").value(),
            Some("review note 🦀")
        );
        assert!(view.read(cx).state.read(cx).selected_lines().is_none());
        assert_eq!(
            view.read(cx).editor.as_ref().unwrap().read(cx).value(),
            "review note 🦀"
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn source_accessibility_labels_preserve_raw_tabs_unicode_and_trailing_spaces(
    cx: &mut TestAppContext,
) {
    let (handle, _) = review(
        cx,
        "@@ -1,2 +1,2 @@\n-\t旧 🦀  \r\n+\t新 🦀  \r\n unchanged\r\n",
        None,
        DiffMode::Unified,
        vec![],
    );
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        for (side, ix, expected) in [
            ("old", 0usize, "\t旧 🦀  "),
            ("new", 0usize, "\t新 🦀  "),
            ("new", 1usize, "unchanged"),
        ] {
            let source = window.within((side, ix)).find("source");
            assert_eq!(source.role(), Some(Role::Label));
            assert_eq!(source.label(), Some(expected));
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn files_share_one_list_and_select_lines_in_their_own_file(cx: &mut TestAppContext) {
    let documents = DiffDocument::parse(
        "diff --git a/first.txt b/first.txt\n--- a/first.txt\n+++ b/first.txt\n@@ -1 +1 @@\n-one\n+uno\ndiff --git a/second.txt b/second.txt\n--- a/second.txt\n+++ b/second.txt\n@@ -1 +1 @@\n-two\n+dos\n",
    )
    .unwrap();
    let (handle, state) = review_documents(cx, documents, None, DiffMode::Split, vec![]);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        // Row identities repeat in each file; the file scope keeps them distinct.
        window
            .within(("diff-file", 1usize))
            .within(("new", 0usize))
            .click(("line", 0usize), cx);
        let range = state.read(cx).selected_lines().unwrap();
        assert_eq!((range.file(), range.side()), (1, DiffSide::Modified));
        window.within("review").press("secondary-c", cx);
        assert_clipboard(cx, "dos\n");
    })
    .unwrap();
}
