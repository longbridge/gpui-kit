//! An application-owned completion menu around a single-line `Input`.
//!
//! Typing `$` opens a menu of shell-style variables; typing filters it, the
//! keyboard drives it, and accepting replaces the typed `$prefix` with the
//! completed `$NAME `. `InputState` carries no completion state: candidates,
//! filtering, ranking, the popup, keyboard ownership and the accepted edit all
//! live here, composed from the input's public API.
//!
//! - `InputEvent::Change` rescans the trigger under the caret. Caret movement
//!   emits no event, so `render` repeats the rescan against the live caret,
//!   and every captured key revalidates before acting: a stale menu dismisses
//!   instead of owning the keyboard or swallowing an Enter.
//! - `capture_action` on the input's wrapper owns `MoveUp` / `MoveDown` /
//!   `Escape` / `IndentInline` before the input sees them while the menu is
//!   open.
//! - `InputEvent::PressEnter` lets the application decide between accepting a
//!   candidate and submitting.
//! - `set_selected_range` + `replace` swap the typed `$prefix` in one undo
//!   step.
//!
//! The menu anchors to the input region; applications that need caret
//! anchoring can resolve it with `cursor_layout()` / `range_to_bounds()`.
use gpui_kit::base::input::{Escape, IndentInline, MoveDown, MoveUp};
use gpui_kit::component::{
    ActiveTheme as _, ThemeStyled as _,
    input::{Input, InputEvent, InputState},
    list::ListItem,
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled,
    Subscription, Window, deferred, div, prelude::FluentBuilder as _, relative,
};

/// The candidate table. A real application fetches or computes candidates per
/// context; the input never knows they exist.
const VARIABLES: &[(&str, &str)] = &[
    ("EDITOR", "default editor"),
    ("HOME", "user home directory"),
    ("LANG", "locale identifier"),
    ("PATH", "executable search path"),
    ("PWD", "working directory"),
    ("SHELL", "login shell"),
    ("TERM", "terminal type"),
    ("USER", "current user name"),
];

/// An open menu. `token_start` is the byte offset of the `$` that opened it,
/// the unit `cursor()` and `set_selected_range()` speak.
#[derive(Clone)]
struct Menu {
    token_start: usize,
    /// Filtered candidate names, in display order.
    items: Vec<&'static str>,
    selected: usize,
}

impl Menu {
    fn selected_name(&self) -> Option<&'static str> {
        self.items.get(self.selected).copied()
    }
}

/// Find the `$`-token the caret sits in.
///
/// Returns `(token_start, query)`. The `$` must open the token, at the start
/// of the text or after whitespace, so `a$b` never triggers. `None` means the
/// caret is outside any token and the menu dismisses.
fn active_variable_query(text: &str, cursor: usize) -> Option<(usize, String)> {
    let bytes = text.as_bytes();
    let cursor = cursor.min(bytes.len());
    let mut start = cursor;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }
    if start == 0 || bytes[start - 1] != b'$' {
        return None;
    }
    if start >= 2 && !bytes[start - 2].is_ascii_whitespace() {
        return None;
    }
    Some((start - 1, text[start..cursor].to_string()))
}

/// Prefix matches first, then substring matches, each alphabetical. Empty
/// when nothing matches, which dismisses the menu quietly.
fn ranked_variables(query: &str) -> Vec<&'static str> {
    let query = query.to_ascii_lowercase();
    let (mut prefixed, mut contained): (Vec<&'static str>, Vec<&'static str>) = VARIABLES
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| name.to_ascii_lowercase().contains(&query))
        .partition(|name| name.to_ascii_lowercase().starts_with(&query));
    prefixed.sort_unstable();
    contained.sort_unstable();
    prefixed.extend(contained);
    prefixed
}

fn description(name: &str) -> &'static str {
    VARIABLES
        .iter()
        .find(|(var, _)| *var == name)
        .map_or("", |(_, description)| *description)
}

pub(super) struct CompletionExample {
    input: Entity<InputState>,
    /// `Some` while the menu is open.
    menu: Option<Menu>,
    /// The last accepted completion or submission.
    status: SharedString,
    _subscription: Subscription,
}

impl CompletionExample {
    pub(super) fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let input = cx
                .new(|cx| InputState::new(window, cx).placeholder("Type $ to complete a variable"));
            let subscription = cx.subscribe_in(&input, window, Self::on_input_event);
            Self {
                input,
                menu: None,
                status: "Enter submits when no menu is open.".into(),
                _subscription: subscription,
            }
        })
    }

    fn on_input_event(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                if self.rescan(cx) {
                    cx.notify();
                }
            }
            // A current menu takes Enter; otherwise Enter submits.
            InputEvent::PressEnter {
                secondary: false, ..
            } => {
                if self.menu_is_current(cx) {
                    self.accept(window, cx);
                } else {
                    self.menu = None;
                    self.submit(cx);
                }
            }
            InputEvent::Blur => self.menu = None,
            _ => {}
        }
    }

    /// Open, update or dismiss the menu from the buffer and caret as they are
    /// now. Returns whether the menu changed.
    fn rescan(&mut self, cx: &App) -> bool {
        let trigger = {
            let input = self.input.read(cx);
            active_variable_query(&input.value(), input.cursor())
        };
        let Some((token_start, query)) = trigger else {
            return self.menu.take().is_some();
        };
        let items = ranked_variables(&query);
        if items.is_empty() {
            return self.menu.take().is_some();
        }

        // Keep the highlighted row while the filter narrows.
        let selected = self
            .menu
            .as_ref()
            .map_or(0, |menu| menu.selected.min(items.len() - 1));
        let changed = self.menu.as_ref().is_none_or(|menu| {
            menu.token_start != token_start || menu.items != items || menu.selected != selected
        });
        self.menu = Some(Menu {
            token_start,
            items,
            selected,
        });
        changed
    }

    /// Whether the open menu still belongs to the token under the caret.
    fn menu_is_current(&self, cx: &App) -> bool {
        let input = self.input.read(cx);
        let trigger = active_variable_query(&input.value(), input.cursor());
        self.menu
            .as_ref()
            .zip(trigger)
            .is_some_and(|(menu, (token_start, _))| menu.token_start == token_start)
    }

    /// Take a captured key while the menu is current. A stale menu dismisses
    /// and leaves the key to the input.
    fn own_key(&mut self, cx: &mut Context<Self>) -> bool {
        if self.menu.is_none() {
            return false;
        }
        if !self.menu_is_current(cx) {
            self.menu = None;
            cx.notify();
            return false;
        }
        cx.stop_propagation();
        true
    }

    fn select(&mut self, delta: isize, cx: &mut Context<Self>) {
        if !self.own_key(cx) {
            return;
        }
        if let Some(menu) = self.menu.as_mut() {
            menu.selected = menu
                .selected
                .saturating_add_signed(delta)
                .min(menu.items.len() - 1);
        }
        cx.notify();
    }

    /// Replace the typed `$prefix` with `$NAME ` in one undo step.
    fn accept(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.menu.take() else {
            return;
        };
        cx.notify();
        let Some(name) = menu.selected_name() else {
            return;
        };

        // The buffer may have changed since the menu opened; edit only the
        // token that is under the caret now.
        let (text, cursor) = {
            let input = self.input.read(cx);
            (input.value().to_string(), input.cursor())
        };
        match active_variable_query(&text, cursor) {
            Some((token_start, _)) if token_start == menu.token_start => {}
            _ => return,
        }

        self.input.update(cx, |input, cx| {
            input.set_selected_range(menu.token_start..cursor, cx);
            input.replace(format!("${name} "), window, cx);
            input.focus(window, cx);
        });
        self.status = format!("Completed ${name}").into();
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        self.status = format!("Submitted {:?}", self.input.read(cx).value()).into();
        cx.notify();
    }

    fn render_menu(&self, menu: &Menu, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .id("completion-menu")
            .w_72()
            .max_h_64()
            .overflow_y_scroll()
            .p_1()
            .gap_y_0p5()
            .popover_style(cx)
            .children(menu.items.iter().enumerate().map(|(ix, name)| {
                ListItem::new(ix)
                    .selected(ix == menu.selected)
                    .rounded(cx.theme().radius)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if let Some(menu) = this.menu.as_mut() {
                            menu.selected = ix;
                        }
                        this.accept(window, cx);
                    }))
                    .child(
                        div().flex().gap_2().text_sm().child(*name).child(
                            div()
                                .text_color(cx.theme().muted_foreground)
                                .child(description(name)),
                        ),
                    )
            }))
    }
}

impl Render for CompletionExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Caret movement emits no event; rescan against the caret as it stands
        // so a menu whose token the caret left is gone in this frame.
        self.rescan(cx);

        v_flex()
            .w_full()
            .gap_2()
            .child(
                div()
                    .relative()
                    .w_full()
                    // Tab accepts: `IndentInline` is what the input binds tab to.
                    .capture_action(cx.listener(|this, _: &MoveUp, _, cx| this.select(-1, cx)))
                    .capture_action(cx.listener(|this, _: &MoveDown, _, cx| this.select(1, cx)))
                    .capture_action(cx.listener(|this, _: &Escape, _, cx| {
                        if this.own_key(cx) {
                            this.menu = None;
                            cx.notify();
                        }
                    }))
                    .capture_action(cx.listener(|this, _: &IndentInline, window, cx| {
                        if this.own_key(cx) {
                            this.accept(window, cx);
                        }
                    }))
                    .child(Input::new(&self.input))
                    .when_some(self.menu.as_ref(), |this, menu| {
                        this.child(deferred(
                            div()
                                .absolute()
                                .left_0()
                                .top(relative(1.))
                                .pt_1()
                                .child(self.render_menu(menu, cx)),
                        ))
                    }),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(self.status.clone()),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{TestAppContext, VisualTestContext};
    use std::{cell::RefCell, ops::Deref as _, rc::Rc};

    #[test]
    fn trigger_opens_at_line_start_and_after_whitespace() {
        assert_eq!(active_variable_query("$HO", 3), Some((0, "HO".into())));
        assert_eq!(active_variable_query("echo $HO", 8), Some((5, "HO".into())));
        assert_eq!(active_variable_query("echo $", 6), Some((5, String::new())));
        assert_eq!(active_variable_query("a$b", 3), None);
        assert_eq!(active_variable_query("mail$x", 6), None);
        assert_eq!(active_variable_query("echo $HO ", 9), None);
    }

    #[test]
    fn ranking_puts_prefix_matches_first() {
        assert_eq!(ranked_variables("P"), vec!["PATH", "PWD"]);
        assert_eq!(
            ranked_variables("e"),
            vec!["EDITOR", "HOME", "SHELL", "TERM", "USER"]
        );
        assert!(ranked_variables("ZZZ").is_empty());
    }

    fn mount(cx: &mut TestAppContext) -> (Entity<CompletionExample>, VisualTestContext) {
        cx.update(gpui_kit::init);
        let mounted = Rc::new(RefCell::new(None));
        let capture = mounted.clone();
        let window = cx.add_window(move |window, cx| {
            let story = CompletionExample::new(window, cx);
            *capture.borrow_mut() = Some(story.clone());
            gpui_kit::component::Root::new(story, window, cx)
        });
        let story = mounted.borrow().clone().unwrap();
        let mut visual = VisualTestContext::from_window(*window.deref(), cx);
        visual.update(|window, cx| {
            story.update(cx, |story, cx| {
                story.input.update(cx, |input, cx| input.focus(window, cx))
            })
        });
        draw(&mut visual);
        (story, visual)
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn value(story: &Entity<CompletionExample>, cx: &mut VisualTestContext) -> String {
        story.read_with(cx, |story, cx| story.input.read(cx).value().to_string())
    }

    fn menu_items(
        story: &Entity<CompletionExample>,
        cx: &mut VisualTestContext,
    ) -> Option<Vec<&'static str>> {
        story.read_with(cx, |story, _| {
            story.menu.as_ref().map(|menu| menu.items.clone())
        })
    }

    #[gpui_kit::test]
    fn keyboard_selects_and_accepts_a_candidate(cx: &mut TestAppContext) {
        let (story, mut cx) = mount(cx);
        cx.simulate_input("echo $P");
        draw(&mut cx);
        assert_eq!(menu_items(&story, &mut cx), Some(vec!["PATH", "PWD"]));

        cx.simulate_keystrokes("down tab");
        draw(&mut cx);
        assert_eq!(value(&story, &mut cx), "echo $PWD ");
        assert_eq!(menu_items(&story, &mut cx), None);

        cx.simulate_input("$HO");
        draw(&mut cx);
        cx.simulate_keystrokes("enter");
        draw(&mut cx);
        assert_eq!(value(&story, &mut cx), "echo $PWD $HOME ");
    }

    #[gpui_kit::test]
    fn leaving_the_token_without_editing_releases_the_keyboard(cx: &mut TestAppContext) {
        let (story, mut cx) = mount(cx);
        cx.simulate_input("echo $HO");
        draw(&mut cx);
        assert_eq!(menu_items(&story, &mut cx), Some(vec!["HOME"]));

        // Moving the caret changes no text, so no `Change` event fires. No
        // draw separates the keys: Enter must notice the stale menu itself
        // and submit instead of accepting.
        cx.simulate_keystrokes("home enter");
        assert_eq!(value(&story, &mut cx), "echo $HO");
        story.read_with(&cx, |story, _| {
            assert!(story.menu.is_none());
            assert_eq!(story.status, "Submitted \"echo $HO\"");
        });
    }
}
