#![cfg(all(feature = "test-support", feature = "component"))]

mod common;

use gpui_kit::component::{
    button::Button,
    diff::{
        Diff, DiffAnnotation, DiffEvent, DiffFile, DiffLinePosition, DiffLineRange, DiffMode,
        DiffSide, DiffState,
    },
    input::{Input, InputState},
};
use gpui_kit::test::{TestSupportExt as _, TestWindowExt as _};
use gpui_kit::{
    App, AppContext, Context, Entity, Focusable as _, InputEvent as _, Modifiers, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Role, ScrollDelta, TestAppContext,
    Window, WindowHandle, div, point, prelude::*, px, size,
};

struct Review {
    state: Entity<DiffState>,
    annotations: Vec<DiffAnnotation>,
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
                    .render_annotation(move |_, _, _| {
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

fn patch_document(hunks: &str) -> DiffFile {
    DiffFile::parse(&format!("--- a/review.txt\n+++ b/review.txt\n{hunks}"))
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
    annotations: Vec<DiffAnnotation>,
) -> (WindowHandle<gpui_kit::base::Root>, Entity<DiffState>) {
    review_documents(cx, vec![patch_document(patch)], context, mode, annotations)
}

fn review_documents(
    cx: &mut TestAppContext,
    documents: Vec<DiffFile>,
    context: Option<usize>,
    mode: DiffMode,
    annotations: Vec<DiffAnnotation>,
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
        vec![DiffAnnotation::line(
            "original-comment",
            DiffLinePosition::new("review.txt", DiffSide::Original, 1),
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
            annotations: vec![DiffAnnotation::line(
                "editable-comment",
                DiffLinePosition::new("review.txt", DiffSide::Modified, 1),
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
    let documents = DiffFile::parse(
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
        assert_eq!(range.path().as_str(), "second.txt");
        assert_eq!(range.side(), DiffSide::Modified);
        window.within("review").press("secondary-c", cx);
        assert_clipboard(cx, "dos\n");
    })
    .unwrap();
}

struct Interactions {
    state: Entity<DiffState>,
    added: std::rc::Rc<std::cell::RefCell<Vec<DiffLineRange>>>,
    clicked: std::rc::Rc<std::cell::RefCell<Vec<DiffLinePosition>>>,
}

impl Render for Interactions {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let added = self.added.clone();
        let clicked = self.clicked.clone();
        div().id("review").test_support().size_full().child(
            Diff::new(&self.state)
                .size_full()
                .annotations([DiffAnnotation::file("file-note", "review.txt")])
                .render_annotation(|annotation, _, _| {
                    div()
                        .id("note-content")
                        .test_support()
                        .child(annotation.id().to_string())
                })
                .on_add_annotation(move |range, _, _| added.borrow_mut().push(range.clone()))
                .on_line_click(move |position, _, _, _| {
                    clicked.borrow_mut().push(position.clone())
                }),
        )
    }
}

fn interactions(
    cx: &mut TestAppContext,
    patch: &str,
) -> (WindowHandle<gpui_kit::base::Root>, Entity<Interactions>) {
    cx.update(gpui_kit::init);
    let file = patch_document(patch);
    common::open_window(cx, Some(size(px(800.), px(480.))), |_, cx| {
        cx.new(|cx| Interactions {
            state: cx.new(|cx| DiffState::new([file], cx).with_context_lines(None)),
            added: Default::default(),
            clicked: Default::default(),
        })
    })
}

#[gpui_kit::test]
fn add_annotation_offers_the_line_or_the_selection_containing_it(cx: &mut TestAppContext) {
    let (handle, view) = interactions(cx, "@@ -1,3 +1,3 @@\n one\n two\n three\n");
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        // The button appears while its row is hovered.
        window.within(("new", 1usize)).hover("source", cx);
        window
            .within(("new", 1usize))
            .click(("add-annotation", 1usize), cx);
        // Selecting lines 1–3, then adding from inside the selection, offers it.
        window
            .within(("new", 0usize))
            .within("new-gutter")
            .click(("line", 0usize), cx);
        let last = window
            .within(("new", 2usize))
            .within("new-gutter")
            .find(("line", 2usize))
            .bounds()
            .center();
        shift_click(window, last, cx);
        window.within(("new", 1usize)).hover("source", cx);
        window
            .within(("new", 1usize))
            .click(("add-annotation", 1usize), cx);
        let added = view.read(cx).added.borrow().clone();
        assert_eq!(
            added,
            [
                DiffLineRange::new("review.txt", DiffSide::Modified, 2, 2),
                DiffLineRange::new("review.txt", DiffSide::Modified, 1, 3),
            ]
        );
        // The button does not start a line selection of its own.
        assert_eq!(
            view.read(cx).state.read(cx).selected_lines(),
            Some(DiffLineRange::new("review.txt", DiffSide::Modified, 1, 3))
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn line_number_drag_selects_and_code_click_reports_the_line(cx: &mut TestAppContext) {
    let (handle, view) = interactions(cx, "@@ -1,3 +1,3 @@\n one\n two\n three\n");
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let from = window
            .within(("new", 0usize))
            .within("new-gutter")
            .find(("line", 0usize))
            .bounds()
            .center();
        let to = window
            .within(("new", 2usize))
            .within("new-gutter")
            .find(("line", 2usize))
            .bounds()
            .center();
        window.drag(from, to, cx);
        assert_eq!(
            view.read(cx).state.read(cx).selected_lines(),
            Some(DiffLineRange::new("review.txt", DiffSide::Modified, 1, 3))
        );
        window.within(("new", 1usize)).click("source", cx);
        assert_eq!(
            view.read(cx).clicked.borrow().as_slice(),
            &[DiffLinePosition::new("review.txt", DiffSide::Modified, 2)]
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn header_collapses_its_file_and_shows_file_annotations(cx: &mut TestAppContext) {
    let (handle, view) = interactions(cx, "@@ -1,2 +1,2 @@\n one\n-two\n+TWO\n");
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("note-content").visible());
        assert!(window.within(("new", 0usize)).try_find("source").is_some());
        window
            .within(("diff-file", 0usize))
            .click("collapse-file", cx);
        assert!(view.read(cx).state.read(cx).is_file_collapsed("review.txt"));
        assert!(window.try_find("source").is_none());
        assert!(window.find("note-content").visible());
        window
            .within(("diff-file", 0usize))
            .click("collapse-file", cx);
        assert!(window.within(("new", 0usize)).try_find("source").is_some());
    })
    .unwrap();
}

struct Viewer {
    state: Entity<DiffState>,
    soft_wrap: bool,
    line_number: bool,
}

impl Render for Viewer {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().id("review").test_support().size_full().child(
            Diff::new(&self.state)
                .soft_wrap(self.soft_wrap)
                .line_number(self.line_number)
                .size_full(),
        )
    }
}

fn viewer(
    cx: &mut TestAppContext,
    file: DiffFile,
    mode: DiffMode,
    soft_wrap: bool,
) -> (WindowHandle<gpui_kit::base::Root>, Entity<DiffState>) {
    cx.update(gpui_kit::init);
    let (handle, view) = common::open_window(cx, Some(size(px(480.), px(480.))), |_, cx| {
        cx.new(|cx| Viewer {
            state: cx.new(|cx| DiffState::new([file], cx).with_mode(mode)),
            soft_wrap,
            line_number: true,
        })
    });
    let state = cx.update(|cx| view.read(cx).state.clone());
    (handle, state)
}

#[gpui_kit::test]
fn conflict_headings_resolve_from_their_buttons(cx: &mut TestAppContext) {
    let file = DiffFile::parse_conflicts(
        "merge.txt",
        "before\n<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> feature\nafter\n",
    )
    .unwrap();
    let (handle, state) = viewer(cx, file, DiffMode::Split, false);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        // Conflicted files keep one column, with only the working-file gutter.
        assert!(window.try_find(("old", 0usize)).is_none());
        window.click(("accept-incoming", 0usize), cx);
        assert_eq!(
            state.read(cx).resolved_text("merge.txt").as_deref(),
            Some("before\ntheirs\nafter\n")
        );
        window.click(("undo-resolution", 0usize), cx);
        assert_eq!(state.read(cx).resolved_text("merge.txt"), None);
        assert!(window.try_find(("accept-both", 0usize)).is_some());
    })
    .unwrap();
}

#[gpui_kit::test]
fn soft_wrap_grows_rows_instead_of_scrolling(cx: &mut TestAppContext) {
    let long = "word ".repeat(60);
    let text = format!("short\n{long}\n");
    let (handle, _) = viewer(
        cx,
        DiffFile::unchanged("notes.txt", &text),
        DiffMode::Unified,
        true,
    );
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.render_frame(cx);
        let short = window.within(("new", 0usize)).find("source").bounds();
        let wrapped = window.within(("new", 1usize)).find("source").bounds();
        assert!(wrapped.size.height > short.size.height * 2.);
        assert!(wrapped.right() <= window.find("review").bounds().right());
    })
    .unwrap();
}

#[gpui_kit::test]
fn clearing_gutter_selection_notifies_without_interrupting_shift_extension(
    cx: &mut TestAppContext,
) {
    let (handle, state) = review(
        cx,
        "@@ -1,3 +1,3 @@\n one\n-old\n+new\n three\n",
        None,
        DiffMode::Split,
        vec![],
    );
    let changes = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let _subscription = cx.update(|cx| {
        let changes = changes.clone();
        cx.subscribe(&state, move |_, event: &DiffEvent, _| {
            if let DiffEvent::SelectionChanged(range) = event {
                changes.borrow_mut().push(range.clone());
            }
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.within(("old", 0usize)).click(("line", 0usize), cx);
    })
    .unwrap();
    assert_eq!(changes.borrow().len(), 1);
    cx.update_window(handle.into(), |_, window, cx| {
        let target = window
            .within(("old", 2usize))
            .find(("line", 2usize))
            .bounds()
            .center();
        shift_click(window, target, cx);
    })
    .unwrap();
    assert_eq!(changes.borrow().len(), 2);
    assert!(changes.borrow().iter().all(Option::is_some));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("before-diff", cx);
    })
    .unwrap();
    cx.update(|cx| assert!(state.read(cx).selected_lines().is_none()));
    assert_eq!(changes.borrow().len(), 3);
    assert!(changes.borrow().last().unwrap().is_none());
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("before-diff", cx);
    })
    .unwrap();
    assert_eq!(changes.borrow().len(), 3);
}

#[gpui_kit::test]
fn wheel_axes_stay_independent_when_code_overflows_in_both_directions(cx: &mut TestAppContext) {
    let line = "let long_line = ".repeat(40);
    let mut patch = String::from("@@ -1,60 +1,60 @@\n");
    for tag in ['-', '+'] {
        for ix in 0..60 {
            patch.push_str(&format!("{tag}{line}{ix}\n"));
        }
    }
    for mode in [DiffMode::Unified, DiffMode::Split] {
        let (handle, _) = review(cx, &patch, None, mode, vec![]);
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            let before = window.within(("old", 10usize)).find("source").bounds();
            window.scroll(
                "diff-body",
                ScrollDelta::Pixels(point(px(0.), px(-100.))),
                cx,
            );
            let vertical = window.within(("old", 10usize)).find("source").bounds();
            assert!(vertical.top() < before.top());
            assert_eq!(vertical.left(), before.left());
            window.scroll(
                "diff-body",
                ScrollDelta::Pixels(point(px(-100.), px(0.))),
                cx,
            );
            let horizontal = window.within(("old", 10usize)).find("source").bounds();
            assert!(horizontal.left() < vertical.left());
            assert_eq!(horizontal.top(), vertical.top());
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn hidden_line_numbers_keep_wrapped_source_aligned_and_selectable(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let long = "changed word ".repeat(24);
    let patch = format!("@@ -1,3 +1,4 @@\n keep\n-old\n+{long}\n+extra\n tail\n");
    let file = patch_document(&patch);
    let (handle, view) = common::open_window(cx, Some(size(px(480.), px(480.))), |_, cx| {
        cx.new(|cx| Viewer {
            state: cx.new(|cx| DiffState::new([file], cx).with_mode(DiffMode::Split)),
            soft_wrap: true,
            line_number: false,
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.render_frame(cx);
        let keep = window.within(("new", 0usize)).find("source").bounds();
        let changed = window.within(("new", 1usize)).find("source").bounds();
        let extra = window.within(("new", 2usize)).find("source").bounds();
        let viewport = window.find("review").bounds();
        assert!(keep.left() > viewport.left() + viewport.size.width / 2. + window.rem_size());
        assert!(keep.right() < viewport.right());
        assert_eq!(keep.left(), changed.left());
        assert_eq!(changed.left(), extra.left());
        assert!(changed.size.height > keep.size.height * 2.);
        assert!(extra.top() >= changed.bottom());
        window.within(("new", 2usize)).click("source", cx);
        window.press(
            if cfg!(target_os = "macos") {
                "cmd-a"
            } else {
                "ctrl-a"
            },
            cx,
        );
        let state = view.read(cx).state.clone();
        let expected = format!("keep\n{long}\nextra\ntail\n");
        assert_eq!(state.read(cx).selected_text(cx), expected);
        view.update(cx, |view, cx| {
            view.line_number = true;
            cx.notify();
        });
        window.render_frame(cx);
        assert!(
            window
                .within(("new", 0usize))
                .find("source")
                .bounds()
                .left()
                > keep.left()
        );
        assert_eq!(state.read(cx).selected_text(cx), expected);
    })
    .unwrap();
}
