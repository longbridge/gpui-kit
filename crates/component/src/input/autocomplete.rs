//! A text input that offers suggestions while accepting any text — the
//! counterpart of HTML's `<datalist>`.

use gpui::{
    App, Context, Entity, InteractiveElement as _, IntoElement, Length, MouseButton,
    ParentElement as _, RenderOnce, ScrollHandle, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Subscription, Window, deferred, div, prelude::FluentBuilder as _,
    relative, rems,
};

use super::{Enter, Escape, Input, InputEvent, InputState, MoveDown, MoveUp};
use crate::{
    ActiveTheme as _, Disableable, Sizable, Size, StyledExt as _, ThemeStyled as _, list::ListItem,
    v_flex,
};

/// A text input that suggests values from a list while accepting any text.
///
/// Unlike [`Combobox`](crate::combobox::Combobox), whose value is always one
/// of its items, the value here is whatever the input holds: the suggestions
/// only help type it. They open under the field as the user types, filtered
/// to those containing the text — prefix matches first — and never offer the
/// text already typed.
///
/// The application owns the [`InputState`] and reads the value from it, and
/// subscribes to its [`InputEvent`]s as for any input: accepting a suggestion
/// replaces the text, which emits [`InputEvent::Change`] and can be undone.
///
/// ```ignore
/// let counter = cx.new(|cx| InputState::new(window, cx).placeholder("Counter"));
///
/// Autocomplete::new(&counter).suggestions(["cpu.load", "cpu.temperature", "memory.used"])
/// ```
///
/// Down opens the suggestions and moves through them, Up moves back to the
/// typed text, Enter accepts the highlighted suggestion, and Escape closes
/// them. With no suggestion highlighted, Enter reaches the input, which
/// emits [`InputEvent::PressEnter`] for the typed text.
#[derive(IntoElement)]
pub struct Autocomplete {
    state: Entity<InputState>,
    suggestions: Vec<SharedString>,
    size: Size,
    disabled: bool,
    style: StyleRefinement,
    menu_max_h: Length,
    decorate: Option<Box<dyn FnOnce(Input) -> Input>>,
}

impl Autocomplete {
    /// An autocomplete bound to the application's [`InputState`].
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            suggestions: Vec::new(),
            size: Size::default(),
            disabled: false,
            style: StyleRefinement::default(),
            menu_max_h: rems(20.).into(),
            decorate: None,
        }
    }

    /// The values to suggest, in the order to offer them.
    pub fn suggestions(
        mut self,
        suggestions: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.suggestions = suggestions.into_iter().map(Into::into).collect();
        self
    }

    /// The tallest the suggestion list grows before it scrolls. Defaults to
    /// `20rem`.
    pub fn menu_max_h(mut self, max_h: impl Into<Length>) -> Self {
        self.menu_max_h = max_h.into();
        self
    }

    /// Configure the underlying [`Input`] — a prefix icon, a clear button,
    /// an accessibility label.
    pub fn input(mut self, decorate: impl FnOnce(Input) -> Input + 'static) -> Self {
        self.decorate = Some(Box::new(decorate));
        self
    }
}

impl Sizable for Autocomplete {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Autocomplete {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for Autocomplete {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

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

/// What the suggestion list remembers between frames.
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
                    menu.open = false;
                    menu.highlighted = None;
                    cx.notify();
                }
                _ => {}
            }),
        }
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
        menu.open = false;
        menu.highlighted = None;
        cx.notify();
    });
    state.update(cx, |input, cx| {
        let end = input.value().len();
        input.set_selected_range(0..end, cx);
        input.replace(value, window, cx);
        input.focus(window, cx);
    });
}

impl RenderOnce for Autocomplete {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state;
        let menu = window.use_keyed_state(("autocomplete-menu", state.entity_id()), cx, |_, cx| {
            Menu::new(&state, cx)
        });
        let items = matches(&state.read(cx).value(), &self.suggestions);
        let (open, highlighted, scroll) = {
            let menu = menu.read(cx);
            (menu.open, menu.highlighted, menu.scroll.clone())
        };
        let visible = open && !self.disabled && !items.is_empty();
        let highlighted = highlighted.filter(|ix| *ix < items.len());
        let count = items.len();

        let input = Input::new(&state)
            .with_size(self.size)
            .disabled(self.disabled)
            .refine_style(&self.style);
        let input = match self.decorate {
            Some(decorate) => decorate(input),
            None => input,
        };

        div()
            .relative()
            .w_full()
            // The list owns these keys while it shows; otherwise they reach
            // the input. Capturing runs before the input's own handlers.
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
                            menu.open = false;
                            menu.highlighted = None;
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
            })
            .child(input)
            .when(visible, |this| {
                this.child(deferred(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top(relative(1.))
                        .pt_1()
                        .child(
                            v_flex()
                                .id("autocomplete-suggestions")
                                .occlude()
                                .max_h(self.menu_max_h)
                                .overflow_y_scroll()
                                .track_scroll(&scroll)
                                .p_1()
                                .gap_y_0p5()
                                .popover_style(cx)
                                // Pressing a suggestion must not take focus from
                                // the input before the click accepts it.
                                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                    window.prevent_default();
                                    cx.stop_propagation();
                                })
                                .children(items.into_iter().enumerate().map(|(ix, value)| {
                                    let (menu, state) = (menu.clone(), state.clone());
                                    ListItem::new(("autocomplete-suggestion", ix))
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
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Deref as _;

    use gpui::{AppContext as _, Modifiers, Render, TestAppContext, VisualTestContext, point, px};

    use super::*;

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

    #[gpui::test]
    fn test_autocomplete_builder(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let state = cx.new(|cx| InputState::new(window, cx));
            let autocomplete = Autocomplete::new(&state)
                .suggestions(["a", "b"])
                .menu_max_h(px(120.))
                .input(|input| input.cleanable(true))
                .small()
                .disabled(true)
                .w_64();
            assert_eq!(autocomplete.suggestions, ["a", "b"]);
            assert_eq!(autocomplete.menu_max_h, px(120.).into());
            assert!(autocomplete.decorate.is_some());
            assert_eq!(autocomplete.size, Size::Small);
            assert!(autocomplete.disabled);
            assert!(autocomplete.style.size.width.is_some());
        });
    }

    struct Harness {
        state: Entity<InputState>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(300.))
                .child(Autocomplete::new(&self.state).suggestions(suggestions()))
        }
    }

    fn mount(cx: &mut TestAppContext) -> (Entity<InputState>, VisualTestContext) {
        cx.update(crate::init);
        let state = std::rc::Rc::new(std::cell::RefCell::new(None));
        let captured = state.clone();
        let window = cx.add_window(move |window, cx| {
            let input = cx.new(|cx| InputState::new(window, cx));
            *captured.borrow_mut() = Some(input.clone());
            let harness = cx.new(|_| Harness { state: input });
            crate::Root::new(harness, window, cx)
        });
        let state = state.borrow().clone().unwrap();
        let mut cx = VisualTestContext::from_window(*window.deref(), cx);
        cx.update(|window, cx| state.update(cx, |input, cx| input.focus(window, cx)));
        draw(&mut cx);
        (state, cx)
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
}
