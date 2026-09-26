use crate::common;
use gpui_kit::{
    App, AppContext, ClipboardItem, Context, Entity, IntoElement, Render, SharedString,
    Subscription, TestAppContext, Window, WindowHandle,
    base::Root,
    component::input::{Input, InputEvent, InputState},
    div, point,
    prelude::*,
    px, size,
    test::TestWindowExt,
};
use std::ops::Range;

struct EditingForm {
    first: Entity<InputState>,
    second: Entity<InputState>,
    submissions: Vec<(SharedString, bool, bool)>,
    _subscription: Subscription,
}

impl Render for EditingForm {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .p_4()
            .gap_4()
            .child(
                Input::new(&self.first)
                    .id("editing-first")
                    .w_80()
                    .prefix(div().child("Name"))
                    .suffix(div().child("Required")),
            )
            .child(Input::new(&self.second).id("editing-second").w_80())
    }
}

fn editing_form(cx: &mut TestAppContext) -> (WindowHandle<Root>, Entity<EditingForm>) {
    cx.update(gpui_kit::init);
    common::open_window(cx, Some(size(px(640.), px(360.))), |window, cx| {
        cx.new(|cx| {
            let first = cx.new(|cx| InputState::new(window, cx));
            let second = cx.new(|cx| InputState::new(window, cx));
            let subscription = cx.subscribe(&first, |this: &mut EditingForm, input, event, cx| {
                if let InputEvent::PressEnter { secondary, shift } = event {
                    this.submissions
                        .push((input.read(cx).value(), *secondary, *shift));
                }
            });
            EditingForm {
                first,
                second,
                submissions: Vec::new(),
                _subscription: subscription,
            }
        })
    })
}

fn assert_edit(
    input: &Entity<InputState>,
    value: &str,
    selection: Range<usize>,
    cursor: usize,
    cx: &App,
) {
    let input = input.read(cx);
    assert_eq!(input.value().as_ref(), value);
    assert_eq!(input.selected_range(), selection);
    assert_eq!(input.cursor(), cursor);
}

#[gpui_kit::test]
fn shift_arrows_reverse_selection_and_typing_replaces_it(cx: &mut TestAppContext) {
    let (handle, form) = editing_form(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        let input = form.read(cx).first.clone();
        window.click("editing-first", cx);
        window.input("abcd", cx);
        window.press("shift-left", cx);
        window.press("shift-left", cx);
        assert_edit(&input, "abcd", 2..4, 2, cx);
        window.press("shift-right", cx);
        assert_edit(&input, "abcd", 3..4, 3, cx);
        window.input("X", cx);
        assert_edit(&input, "abcX", 4..4, 4, cx);
        assert_eq!(window.find("editing-first").value(), Some("abcX"));

        window.press("home", cx);
        window.press("shift-right", cx);
        window.press("shift-right", cx);
        assert_edit(&input, "abcX", 0..2, 2, cx);
        window.input("Y", cx);
        assert_edit(&input, "YcX", 1..1, 1, cx);
    })
    .unwrap();
}

#[gpui_kit::test]
fn home_end_and_shift_select_line_boundaries(cx: &mut TestAppContext) {
    let (handle, form) = editing_form(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        let input = form.read(cx).first.clone();
        window.click("editing-first", cx);
        window.input("alpha beta", cx);
        window.press("home", cx);
        assert_edit(&input, "alpha beta", 0..0, 0, cx);
        window.press("right", cx);
        window.press("shift-end", cx);
        assert_edit(&input, "alpha beta", 1..10, 10, cx);
        window.input("!", cx);
        assert_edit(&input, "a!", 2..2, 2, cx);
        window.press("shift-home", cx);
        assert_edit(&input, "a!", 0..2, 0, cx);
        window.press("end", cx);
        assert_edit(&input, "a!", 2..2, 2, cx);
        window.input("?", cx);
        assert_eq!(window.find("editing-first").value(), Some("a!?"));
    })
    .unwrap();
}

#[gpui_kit::test]
fn backspace_and_delete_remove_opposite_sides_and_stop_at_boundaries(cx: &mut TestAppContext) {
    let (handle, form) = editing_form(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        let input = form.read(cx).first.clone();
        window.click("editing-first", cx);
        window.input("abcd", cx);
        window.press("left", cx);
        window.press("left", cx);
        assert_edit(&input, "abcd", 2..2, 2, cx);
        window.press("backspace", cx);
        assert_edit(&input, "acd", 1..1, 1, cx);
        window.press("delete", cx);
        assert_edit(&input, "ad", 1..1, 1, cx);
        window.press("home", cx);
        window.press("backspace", cx);
        assert_edit(&input, "ad", 0..0, 0, cx);
        window.press("end", cx);
        window.press("delete", cx);
        assert_edit(&input, "ad", 2..2, 2, cx);
        window.press("shift-home", cx);
        window.press("delete", cx);
        assert_edit(&input, "", 0..0, 0, cx);
        window.press("backspace", cx);
        window.press("delete", cx);
        assert_eq!(window.find("editing-first").value(), Some(""));
        window.input("abcd", cx);
        window.press("left", cx);
        window.press("shift-left", cx);
        window.press("shift-left", cx);
        assert_edit(&input, "abcd", 1..3, 1, cx);
        window.press("backspace", cx);
        assert_edit(&input, "ad", 1..1, 1, cx);
    })
    .unwrap();
}

#[gpui_kit::test]
fn copy_preserves_selection_and_pastes_only_selected_text_into_another_input(
    cx: &mut TestAppContext,
) {
    let (handle, form) = editing_form(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        let input = form.read(cx).first.clone();
        cx.write_to_clipboard(ClipboardItem::new_string("stale".into()));
        window.click("editing-first", cx);
        window.input("keep copy", cx);
        for _ in 0..4 {
            window.press("shift-left", cx);
        }
        window.press("secondary-c", cx);
        assert_edit(&input, "keep copy", 5..9, 5, cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("copy")
        );
        window.click("editing-second", cx);
        window.press("secondary-v", cx);
        assert_eq!(window.find("editing-second").value(), Some("copy"));
        assert_eq!(input.read(cx).value(), "keep copy");
        assert_eq!(form.read(cx).second.read(cx).value(), "copy");
    })
    .unwrap();
}

#[gpui_kit::test]
fn cut_removes_selection_and_paste_replaces_destination_selection(cx: &mut TestAppContext) {
    let (handle, form) = editing_form(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        let input = form.read(cx).first.clone();
        window.click("editing-first", cx);
        window.input("keep cut", cx);
        for _ in 0..3 {
            window.press("shift-left", cx);
        }
        window.press("secondary-x", cx);
        assert_edit(&input, "keep ", 5..5, 5, cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("cut")
        );
        window.click("editing-second", cx);
        window.input("replace", cx);
        window.press("secondary-a", cx);
        window.press("secondary-v", cx);
        assert_edit(&form.read(cx).second, "cut", 3..3, 3, cx);
        assert_eq!(window.find("editing-first").value(), Some("keep "));
        assert_eq!(window.find("editing-second").value(), Some("cut"));
    })
    .unwrap();
}

#[gpui_kit::test]
fn multiline_clipboard_normalization_preserves_surrounding_text(cx: &mut TestAppContext) {
    let (handle, form) = editing_form(cx);
    for payload in ["one\ntwo", "one\r\ntwo", "one\rtwo", "\none\r\ntwo\n"] {
        cx.update_window(handle.into(), |_, window, cx| {
            let input = form.read(cx).first.clone();
            window.click("editing-first", cx);
            window.press("secondary-a", cx);
            window.input("[old]", cx);
            window.press("left", cx);
            for _ in 0..3 {
                window.press("shift-left", cx);
            }
            assert_edit(&input, "[old]", 1..4, 1, cx);
            cx.write_to_clipboard(ClipboardItem::new_string(payload.into()));
            window.press("secondary-v", cx);
            assert_edit(&input, "[onetwo]", 7..7, 7, cx);
            assert_eq!(window.find("editing-first").value(), Some("[onetwo]"));
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some(payload)
            );
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn double_click_selects_a_word_for_replacement(cx: &mut TestAppContext) {
    let (handle, form) = editing_form(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        let input = form.read(cx).second.clone();
        window.click("editing-second", cx);
        // A word wider than the field keeps the center over actual glyphs,
        // including after horizontal scrolling to the typing cursor.
        let word = "word".repeat(24);
        window.input(&word, cx);
        window.double_click("editing-second", cx);
        assert_eq!(input.read(cx).selected_range(), 0..96);
        assert_eq!(input.read(cx).selected_value().as_ref(), word.as_str());
        window.input("replacement", cx);
        assert_edit(&input, "replacement", 11..11, 11, cx);
        assert_eq!(window.find("editing-second").value(), Some("replacement"));
    })
    .unwrap();
}

#[gpui_kit::test]
fn pointer_drag_selects_text_and_click_collapses_selection(cx: &mut TestAppContext) {
    let (handle, form) = editing_form(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        let input = form.read(cx).second.clone();
        window.click("editing-second", cx);
        window.input("drag me", cx);
        let text = input.read(cx).text_bounds().expect("rendered text bounds");
        let control = window.find("editing-second").bounds();
        let start = point(text.left(), text.center().y);
        let end = point(control.center().x, start.y);
        window.drag(start, end, cx);
        assert_eq!(input.read(cx).selected_range(), 0..7);
        assert_eq!(input.read(cx).selected_value(), "drag me");
        window.click("editing-second", cx);
        assert_edit(&input, "drag me", 7..7, 7, cx);
        window.drag(end, start, cx);
        assert_edit(&input, "drag me", 0..7, 0, cx);
        window.input("done", cx);
        assert_edit(&input, "done", 4..4, 4, cx);
        assert_eq!(window.find("editing-second").value(), Some("done"));
    })
    .unwrap();
}

#[gpui_kit::test]
fn enter_reports_modifiers_without_inserting_newlines_or_replacing_selection(
    cx: &mut TestAppContext,
) {
    let (handle, form) = editing_form(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("editing-first", cx);
        window.input("submit", cx);
        window.press("shift-left", cx);
    })
    .unwrap();
    for (key, secondary, shift) in [
        ("enter", false, false),
        ("shift-enter", false, true),
        ("secondary-enter", true, false),
    ] {
        let previous_count = cx.update(|cx| form.read(cx).submissions.len());
        cx.update_window(handle.into(), |_, window, cx| {
            window.press(key, cx);
            assert_edit(&form.read(cx).first, "submit", 5..6, 5, cx);
            assert_eq!(window.find("editing-first").value(), Some("submit"));
        })
        .unwrap();
        cx.run_until_parked();
        common::update_content(handle, &form, cx, |view, _, _| {
            assert_eq!(view.submissions.len(), previous_count + 1);
            assert_eq!(
                view.submissions.last(),
                Some(&(SharedString::from("submit"), secondary, shift))
            );
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn tab_moves_between_inputs_with_passive_prefix_and_suffix(cx: &mut TestAppContext) {
    let (handle, form) = editing_form(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("editing-first", cx);
        window.input("first", cx);
        window.press("tab", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("editing-first").focused(), Some(false));
        assert_eq!(window.find("editing-second").focused(), Some(true));
        window.input("second", cx);
        window.press("shift-tab", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("editing-first").focused(), Some(true));
        assert_eq!(window.find("editing-second").focused(), Some(false));
        window.press("end", cx);
        window.input("!", cx);
        assert_eq!(window.find("editing-first").value(), Some("first!"));
        assert_eq!(window.find("editing-second").value(), Some("second"));
        assert_eq!(form.read(cx).first.read(cx).value(), "first!");
        assert_eq!(form.read(cx).second.read(cx).value(), "second");
    })
    .unwrap();
}
