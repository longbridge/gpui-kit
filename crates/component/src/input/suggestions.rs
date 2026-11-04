//! The suggestion list behind [`Input::suggestions`](super::Input::suggestions),
//! the counterpart of an HTML `<input list>` with its `<datalist>`.

use gpui::{
    App, Context, Entity, InteractiveElement, IntoElement as _, MouseButton, ParentElement,
    ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window,
    deferred, div, relative, rems,
};

use super::{Enter, Escape, InputEvent, InputState, MoveDown, MoveUp};
use crate::{ActiveTheme as _, ThemeStyled as _, list::ListItem, v_flex};

/// The suggestions worth offering for `text`: those containing it, ignoring
/// case, prefix matches first and each group in the order given. The text
/// itself is left out — offering what is already typed helps no one — and an
/// empty text offers everything.
fn matches(text: &str, suggestions: &[SharedString]) -> Vec<SharedString> {
    let needle = text.trim().to_lowercase();
    let (mut prefixed, contained): (Vec<_>, Vec<_>) = suggestions
        .iter()
        .filter(|suggestion| {
            let haystack = suggestion.to_lowercase();
            haystack != needle && haystack.contains(&needle)
        })
        .cloned()
        .partition(|suggestion| suggestion.to_lowercase().starts_with(&needle));
    prefixed.extend(contained);
    prefixed
}

/// What the suggestion list remembers between frames, keyed to its input.
struct Menu {
    open: bool,
    highlighted: Option<usize>,
    /// Set while a suggestion replaces the text, so the change that replace
    /// emits closes the list instead of reopening it on the accepted value.
    accepting: bool,
    scroll: ScrollHandle,
    _input_events: Subscription,
}

impl Menu {
    fn new(input: &Entity<InputState>, cx: &mut Context<Self>) -> Self {
        Self {
            open: false,
            highlighted: None,
            accepting: false,
            scroll: ScrollHandle::new(),
            _input_events: cx.subscribe(input, |menu, _, event: &InputEvent, cx| match event {
                InputEvent::Change => {
                    menu.open = !std::mem::take(&mut menu.accepting);
                    menu.highlighted = None;
                    cx.notify();
                }
                InputEvent::Blur => {
                    menu.close();
                    cx.notify();
                }
                _ => {}
            }),
        }
    }

    fn close(&mut self) {
        self.open = false;
        self.highlighted = None;
    }

    /// Move the highlight `delta` rows through `count` suggestions; above the
    /// first row is the typed text, with nothing highlighted.
    fn step(&mut self, delta: isize, count: usize) {
        self.open = true;
        self.highlighted = match (self.highlighted, delta.signum()) {
            (None, 1) => Some(0),
            (None, _) => None,
            (Some(0), -1) => None,
            (Some(ix), _) => Some(ix.saturating_add_signed(delta).min(count - 1)),
        };
        if let Some(ix) = self.highlighted {
            self.scroll.scroll_to_item(ix);
        }
    }
}

/// One input's suggestion list for one frame: the matches for the text it
/// holds, and the keys and popup it adds to the input.
pub(super) struct SuggestionList {
    menu: Entity<Menu>,
    state: Entity<InputState>,
    items: Vec<SharedString>,
    highlighted: Option<usize>,
    /// Whether the popup shows this frame.
    visible: bool,
    scroll: ScrollHandle,
}

impl SuggestionList {
    /// The list for `state` offering `suggestions`. A disabled or read-only
    /// input offers none.
    pub(super) fn new(
        state: &Entity<InputState>,
        suggestions: &[SharedString],
        editable: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let menu = window.use_keyed_state(("input-suggestions", state.entity_id()), cx, |_, cx| {
            Menu::new(state, cx)
        });
        let items = matches(&state.read(cx).value(), suggestions);
        let (open, highlighted, scroll) = {
            let menu = menu.read(cx);
            (menu.open, menu.highlighted, menu.scroll.clone())
        };
        Self {
            visible: open && editable && !items.is_empty(),
            highlighted: highlighted.filter(|ix| *ix < items.len()),
            menu,
            state: state.clone(),
            items: if editable { items } else { Vec::new() },
            scroll,
        }
    }

    /// Add the list's keys and popup to the input's root element.
    ///
    /// The keys are captured, so they reach the list before the input's own
    /// handlers, and only while they have something to do: otherwise Up and
    /// Down move the caret, Escape and Enter reach the input as usual.
    pub(super) fn attach<E>(self, element: E, cx: &App) -> E
    where
        E: InteractiveElement + ParentElement,
    {
        let Self {
            menu,
            state,
            items,
            highlighted,
            visible,
            scroll,
        } = self;
        let count = items.len();

        let mut element = element
            .capture_action({
                let menu = menu.clone();
                move |_: &MoveDown, _, cx| {
                    if count > 0 {
                        menu.update(cx, |menu, cx| {
                            menu.step(1, count);
                            cx.notify();
                        });
                        cx.stop_propagation();
                    }
                }
            })
            .capture_action({
                let menu = menu.clone();
                move |_: &MoveUp, _, cx| {
                    if visible {
                        menu.update(cx, |menu, cx| {
                            menu.step(-1, count);
                            cx.notify();
                        });
                        cx.stop_propagation();
                    }
                }
            })
            .capture_action({
                let menu = menu.clone();
                move |_: &Escape, _, cx| {
                    if visible {
                        menu.update(cx, |menu, cx| {
                            menu.close();
                            cx.notify();
                        });
                        cx.stop_propagation();
                    }
                }
            })
            .capture_action({
                let (menu, state, items) = (menu.clone(), state.clone(), items.clone());
                move |_: &Enter, window, cx| {
                    let Some(value) = highlighted
                        .filter(|_| visible)
                        .and_then(|ix| items.get(ix).cloned())
                    else {
                        return;
                    };
                    cx.stop_propagation();
                    accept(&menu, &state, value, window, cx);
                }
            });
        if visible {
            element.extend([popup(&menu, &state, items, highlighted, &scroll, cx)]);
        }
        element
    }
}

/// The list under the field, as wide as it, styled as a popover.
fn popup(
    menu: &Entity<Menu>,
    state: &Entity<InputState>,
    items: Vec<SharedString>,
    highlighted: Option<usize>,
    scroll: &ScrollHandle,
    cx: &App,
) -> gpui::AnyElement {
    deferred(
        div()
            .absolute()
            .left_0()
            .right_0()
            .top(relative(1.))
            .pt_1()
            .child(
                v_flex()
                    .id("input-suggestions")
                    .occlude()
                    .max_h(rems(20.))
                    .overflow_y_scroll()
                    .track_scroll(scroll)
                    .p_1()
                    .gap_y_0p5()
                    .popover_style(cx)
                    // Pressing a suggestion must not take focus from the input
                    // before the click accepts it.
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    })
                    .children(items.into_iter().enumerate().map(|(ix, value)| {
                        let (menu, state) = (menu.clone(), state.clone());
                        ListItem::new(("input-suggestion", ix))
                            .selected(highlighted == Some(ix))
                            .rounded(cx.theme().radius)
                            .on_click({
                                let value = value.clone();
                                move |_, window, cx| {
                                    accept(&menu, &state, value.clone(), window, cx)
                                }
                            })
                            .child(value)
                    })),
            ),
    )
    .into_any_element()
}

/// Replace the input's text with `value` in one undoable edit.
fn accept(
    menu: &Entity<Menu>,
    state: &Entity<InputState>,
    value: SharedString,
    window: &mut Window,
    cx: &mut App,
) {
    menu.update(cx, |menu, cx| {
        menu.accepting = true;
        menu.close();
        cx.notify();
    });
    state.update(cx, |input, cx| {
        let end = input.value().len();
        input.set_selected_range(0..end, cx);
        input.replace(value, window, cx);
        input.focus(window, cx);
    });
}

#[cfg(test)]
mod tests {
    use std::ops::Deref as _;

    use gpui::{
        AppContext as _, IntoElement, Modifiers, Render, TestAppContext, VisualTestContext, point,
        px,
    };

    use super::*;
    use crate::input::Input;

    fn suggestions() -> Vec<SharedString> {
        [
            "cpu.load",
            "cpu.temperature",
            "memory.used",
            "disk.cpu_wait",
        ]
        .map(Into::into)
        .to_vec()
    }

    #[test]
    fn matching_puts_prefixes_first_and_leaves_out_the_typed_text() {
        let all = suggestions();
        assert_eq!(
            matches("CPU", &all),
            ["cpu.load", "cpu.temperature", "disk.cpu_wait"]
        );
        assert_eq!(matches("", &all).len(), 4);
        assert!(matches("cpu.load", &all).is_empty());
        assert!(matches("gpu", &all).is_empty());
    }

    struct Harness {
        state: Entity<InputState>,
        disabled: bool,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(300.)).child(
                Input::new(&self.state)
                    .suggestions(suggestions())
                    .disabled(self.disabled),
            )
        }
    }

    fn mount_with(
        cx: &mut TestAppContext,
        disabled: bool,
    ) -> (Entity<InputState>, VisualTestContext) {
        cx.update(crate::init);
        let state = std::rc::Rc::new(std::cell::RefCell::new(None));
        let captured = state.clone();
        let window = cx.add_window(move |window, cx| {
            let input = cx.new(|cx| InputState::new(window, cx));
            *captured.borrow_mut() = Some(input.clone());
            let harness = cx.new(|_| Harness {
                state: input,
                disabled,
            });
            crate::Root::new(harness, window, cx)
        });
        let state = state.borrow().clone().unwrap();
        let mut cx = VisualTestContext::from_window(*window.deref(), cx);
        cx.update(|window, cx| state.update(cx, |input, cx| input.focus(window, cx)));
        draw(&mut cx);
        (state, cx)
    }

    fn mount(cx: &mut TestAppContext) -> (Entity<InputState>, VisualTestContext) {
        mount_with(cx, false)
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn value(state: &Entity<InputState>, cx: &mut VisualTestContext) -> String {
        state.read_with(cx, |input, _| input.value().to_string())
    }

    #[gpui::test]
    fn typing_keeps_free_text_and_enter_submits_it(cx: &mut TestAppContext) {
        let (state, mut cx) = mount(cx);
        let entered = std::rc::Rc::new(std::cell::Cell::new(0));
        cx.update(|_, cx| {
            let entered = entered.clone();
            cx.subscribe(&state, move |_, event: &InputEvent, _| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    entered.set(entered.get() + 1);
                }
            })
            .detach();
        });

        cx.simulate_input("gpu.fan");
        draw(&mut cx);
        cx.simulate_keystrokes("enter");
        draw(&mut cx);

        // Text no suggestion matches is kept, and Enter reaches the input.
        assert_eq!(value(&state, &mut cx), "gpu.fan");
        assert_eq!(entered.get(), 1);
    }

    #[gpui::test]
    fn the_keyboard_accepts_a_suggestion(cx: &mut TestAppContext) {
        let (state, mut cx) = mount(cx);

        cx.simulate_input("cpu");
        draw(&mut cx);
        cx.simulate_keystrokes("down down enter");
        draw(&mut cx);
        assert_eq!(value(&state, &mut cx), "cpu.temperature");

        // Accepting closed the list, so the next Enter is the input's again.
        cx.simulate_keystrokes("enter");
        draw(&mut cx);
        assert_eq!(value(&state, &mut cx), "cpu.temperature");

        // Up from the first row returns to the typed text, so Enter submits
        // it instead of accepting a suggestion.
        cx.simulate_input(".");
        draw(&mut cx);
        cx.simulate_keystrokes("backspace down up enter");
        draw(&mut cx);
        assert_eq!(value(&state, &mut cx), "cpu.temperature");

        // Escape closes the list without changing the text.
        cx.simulate_input("x");
        draw(&mut cx);
        cx.simulate_keystrokes("escape down");
        draw(&mut cx);
        assert_eq!(value(&state, &mut cx), "cpu.temperaturex");
    }

    #[gpui::test]
    fn clicking_a_suggestion_accepts_it(cx: &mut TestAppContext) {
        let (state, mut cx) = mount(cx);

        cx.simulate_input("mem");
        draw(&mut cx);
        // The list sits under the field; its first row holds `memory.used`.
        let field_bottom = cx.update(|window, _| window.rem_size() * 2.25);
        cx.simulate_click(point(px(40.), field_bottom + px(16.)), Modifiers::default());
        draw(&mut cx);

        assert_eq!(value(&state, &mut cx), "memory.used");
    }

    #[gpui::test]
    fn a_disabled_input_offers_nothing(cx: &mut TestAppContext) {
        let (state, mut cx) = mount_with(cx, true);
        state.update_in(&mut cx, |input, window, cx| {
            input.set_value("cpu", window, cx)
        });
        draw(&mut cx);
        cx.simulate_keystrokes("down enter");
        draw(&mut cx);
        assert_eq!(value(&state, &mut cx), "cpu");
    }
}
