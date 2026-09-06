use std::rc::Rc;

use gpui::{
    AccessibleAction, AnyElement, App, ElementId, FocusHandle, InteractiveElement as _,
    IntoElement, KeyBinding, ParentElement, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _,
};

use crate::StyledExt as _;
use crate::actions::{Cancel, Confirm, SelectDown, SelectUp};

const CONTEXT: &str = "Select";

#[doc(hidden)]
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new(
            "secondary-enter",
            Confirm { secondary: true },
            Some(CONTEXT),
        ),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
    ]);
}

type OpenChangeHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;
type ActionHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// An unstyled controlled select root.
///
/// Applications own the trigger and popup presentation, the option collection,
/// and the selected value. This root owns combobox accessibility semantics,
/// keyboard opening and dismissal, and focus transfer between the trigger and
/// popup content.
///
/// GPUI marks the active option on the option element itself rather than on the
/// container, so the application marks its highlighted option with
/// `aria_active_descendant()`; this root cannot do it on the caller's behalf.
#[derive(IntoElement)]
pub struct Select {
    id: ElementId,
    open: bool,
    disabled: bool,
    focus_handle: Option<FocusHandle>,
    content_focus_handle: Option<FocusHandle>,
    accessibility_label: Option<SharedString>,
    accessibility_value: Option<SharedString>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
    on_open_change: Option<OpenChangeHandler>,
    key_context: &'static str,
    on_dismiss: Option<ActionHandler>,
    on_confirm: Option<ActionHandler>,
}

impl Select {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            open: false,
            disabled: false,
            focus_handle: None,
            content_focus_handle: None,
            accessibility_label: None,
            accessibility_value: None,
            style: StyleRefinement::default(),
            children: Vec::new(),
            on_open_change: None,
            key_context: CONTEXT,
            on_dismiss: None,
            on_confirm: None,
        }
    }

    /// Sets the application-controlled open state.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Prevents keyboard and accessible activation and removes the trigger from tab traversal.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Supplies the focus handle for the select trigger.
    pub fn focus_handle(mut self, focus_handle: &FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle.clone());
        self
    }

    /// Supplies the focus handle that receives keyboard navigation while open.
    pub fn content_focus_handle(mut self, focus_handle: &FocusHandle) -> Self {
        self.content_focus_handle = Some(focus_handle.clone());
        self
    }

    /// Sets the accessible name exposed by the controlled root.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Sets the committed value exposed by the controlled root.
    ///
    /// Supply a readable selection title, not the current search query or cursor.
    pub fn accessibility_value(mut self, value: impl Into<SharedString>) -> Self {
        self.accessibility_value = Some(value.into());
        self
    }

    /// Handles requests to update the controlled open state.
    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(handler));
        self
    }

    #[doc(hidden)]
    pub fn key_context(mut self, key_context: &'static str) -> Self {
        self.key_context = key_context;
        self
    }

    /// Handles a dismissal, however it was requested: the Cancel action, or
    /// the accessible activation that closes an open control.
    ///
    /// This runs before the controlled open state is asked to close, so a
    /// caller that commits its pending value on dismissal can still read that
    /// value here.
    pub fn on_dismiss(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }

    /// Handles the Confirm action while the select is open.
    ///
    /// Confirming a closed select opens it instead, so this never runs for
    /// that case.
    pub fn on_confirm(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_confirm = Some(Rc::new(handler));
        self
    }
}

impl Styled for Select {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Select {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Select {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let open = self.open;
        let disabled = self.disabled;
        let focus_handle = self.focus_handle;
        let content_focus_handle = self.content_focus_handle;
        let on_open_change = self.on_open_change;
        let on_dismiss = self.on_dismiss;
        let on_confirm = self.on_confirm;

        // Every way of closing runs the same steps. A caller that tracks
        // dismissal has to see one however the popup was closed, and the
        // accessible activation closes exactly what Escape closes.
        let close: ActionHandler = Rc::new({
            let on_open_change = on_open_change.clone();
            let on_dismiss = on_dismiss.clone();
            let focus_handle = focus_handle.clone();
            move |window: &mut Window, cx: &mut App| {
                if let Some(handler) = on_dismiss.as_ref() {
                    handler(window, cx);
                }
                if let Some(handler) = on_open_change.as_ref() {
                    handler(false, window, cx);
                }
                if let Some(handle) = focus_handle.as_ref() {
                    handle.focus(window, cx);
                }
            }
        });

        div()
            .id(self.id)
            .role(Role::ComboBox)
            .aria_expanded(open)
            .when_some(self.accessibility_label, |this, label| {
                this.aria_label(label)
            })
            .when_some(self.accessibility_value, |this, value| {
                this.aria_value(value)
            })
            .key_context(self.key_context)
            .when_some(
                focus_handle.clone().filter(|_| !disabled),
                |this, handle| this.track_focus(&handle.tab_stop(true)),
            )
            .when(!disabled, |this| {
                let on_open_change = on_open_change.clone();
                let content_focus_handle = content_focus_handle.clone();
                let close = close.clone();

                // Platform adapters may flatten the trigger child.
                // Expose activation on the semantic root itself.
                this.on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                    if open {
                        close(window, cx);
                        return;
                    }

                    if let Some(handler) = on_open_change.as_ref() {
                        handler(true, window, cx);
                    }
                    if let Some(handle) = content_focus_handle.as_ref() {
                        handle.focus(window, cx);
                    }
                })
            })
            .on_action({
                let on_open_change = on_open_change.clone();
                let content_focus_handle = content_focus_handle.clone();
                move |_: &SelectUp, window, cx| {
                    if disabled {
                        cx.propagate();
                        return;
                    }

                    if !open {
                        if let Some(handler) = on_open_change.as_ref() {
                            handler(true, window, cx);
                        }
                    }

                    if let Some(handle) = content_focus_handle.as_ref() {
                        handle.focus(window, cx);
                    }
                    cx.propagate();
                }
            })
            .on_action({
                let on_open_change = on_open_change.clone();
                let content_focus_handle = content_focus_handle.clone();
                move |_: &SelectDown, window, cx| {
                    if disabled {
                        cx.propagate();
                        return;
                    }

                    if !open {
                        if let Some(handler) = on_open_change.as_ref() {
                            handler(true, window, cx);
                        }
                    }

                    if let Some(handle) = content_focus_handle.as_ref() {
                        handle.focus(window, cx);
                    }
                    cx.propagate();
                }
            })
            .on_action({
                let on_open_change = on_open_change.clone();
                move |_: &Confirm, window, cx| {
                    if disabled {
                        cx.propagate();
                        return;
                    }

                    cx.propagate();
                    if open {
                        if let Some(handler) = on_confirm.as_ref() {
                            handler(window, cx);
                        }
                    } else if let Some(handler) = on_open_change.as_ref() {
                        handler(true, window, cx);
                    }

                    if let Some(handle) = content_focus_handle.as_ref() {
                        handle.focus(window, cx);
                    }
                }
            })
            .on_action(move |_: &Cancel, window, cx| {
                if !open {
                    cx.propagate();
                    return;
                }

                cx.stop_propagation();
                close(window, cx);
            })
            .children(self.children)
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        Context, Element as _, Focusable, Render, TestAppContext, VisualTestContext, accesskit, px,
    };
    use std::sync::{Arc, Mutex};

    struct SelectHarness {
        open: bool,
        disabled: bool,
        focus_handle: FocusHandle,
        content_focus_handle: FocusHandle,
        changes: Arc<Mutex<Vec<bool>>>,
        /// Every step of a close, in the order it ran.
        closing: Arc<Mutex<Vec<&'static str>>>,
    }

    impl SelectHarness {
        fn new(disabled: bool, cx: &mut Context<Self>) -> Self {
            Self {
                open: false,
                disabled,
                focus_handle: cx.focus_handle(),
                content_focus_handle: cx.focus_handle(),
                changes: Arc::new(Mutex::new(Vec::new())),
                closing: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl Focusable for SelectHarness {
        fn focus_handle(&self, _: &App) -> FocusHandle {
            self.focus_handle.clone()
        }
    }

    impl Render for SelectHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let state = cx.entity();
            let changes = self.changes.clone();
            let opened = self.closing.clone();
            let dismissed = self.closing.clone();

            Select::new("select")
                .open(self.open)
                .disabled(self.disabled)
                .focus_handle(&self.focus_handle)
                .content_focus_handle(&self.content_focus_handle)
                .on_open_change(move |open, _, cx| {
                    changes.lock().unwrap().push(open);
                    opened
                        .lock()
                        .unwrap()
                        .push(if open { "open" } else { "close" });
                    state.update(cx, |state, cx| {
                        state.open = open;
                        cx.notify();
                    });
                })
                .on_dismiss(move |_, _| dismissed.lock().unwrap().push("dismiss"))
                .child(div().track_focus(&self.content_focus_handle).size(px(20.)))
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        disabled: bool,
    ) -> (&mut VisualTestContext, gpui::Entity<SelectHarness>) {
        cx.update(crate::init);
        let (state, cx) = cx.add_window_view(move |_, cx| SelectHarness::new(disabled, cx));
        cx.update(|window, cx| {
            state.focus_handle(cx).focus(window, cx);
            window.draw(cx).clear(cx);
        });
        (cx, state)
    }

    #[gpui::test]
    fn arrows_open_and_transfer_focus_to_content(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, false);

        cx.simulate_keystrokes("down");
        cx.update(|window, cx| {
            assert!(state.read(cx).open);
            assert!(state.read(cx).content_focus_handle.is_focused(window));
        });
        assert_eq!(
            &*state
                .read_with(cx, |state, _| state.changes.clone())
                .lock()
                .unwrap(),
            &[true]
        );
    }

    #[gpui::test]
    fn confirm_opens_a_closed_select(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, false);

        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            assert!(state.read(cx).open);
            assert!(state.read(cx).content_focus_handle.is_focused(window));
        });
    }

    #[gpui::test]
    fn escape_closes_and_restores_trigger_focus(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, false);

        cx.simulate_keystrokes("down escape");
        cx.update(|window, cx| {
            assert!(!state.read(cx).open);
            assert!(state.read(cx).focus_handle.is_focused(window));
        });
        assert_eq!(
            &*state
                .read_with(cx, |state, _| state.changes.clone())
                .lock()
                .unwrap(),
            &[true, false]
        );
    }

    /// Closing runs `on_dismiss`, and runs it before the open state is asked
    /// to close, so a caller that commits a pending value on dismissal can
    /// still read that value.
    ///
    /// Every close shares one path, which is the point: the accessible
    /// activation used to close by calling `on_open_change` alone, so a
    /// consumer wiring `on_dismiss` — `crates/shell` forwards it to JS as
    /// `onDismiss` — saw Escape but not a screen reader pressing the same
    /// control. GPUI exposes no way to dispatch an accessibility action in a
    /// test (`Window::handle_a11y_action` is `pub(crate)`), so this covers the
    /// shared path through the route a test can reach.
    #[gpui::test]
    fn every_close_dismisses_before_it_closes(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, false);

        cx.simulate_keystrokes("down escape");
        assert_eq!(
            &*state
                .read_with(cx, |state, _| state.closing.clone())
                .lock()
                .unwrap(),
            &["open", "dismiss", "close"]
        );
    }

    #[gpui::test]
    fn disabled_select_is_not_keyboard_interactive(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, true);

        cx.simulate_keystrokes("down enter");
        assert!(!state.read_with(cx, |state, _| state.open));
        assert!(
            state
                .read_with(cx, |state, _| state.changes.clone())
                .lock()
                .unwrap()
                .is_empty()
        );
    }

    #[gpui::test]
    fn projects_application_owned_accessible_state(cx: &mut TestAppContext) {
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            let mut info = |select: Select| {
                let mut node = accesskit::Node::new(Role::ComboBox);
                select
                    .render(window, cx)
                    .into_element()
                    .write_a11y_info(&mut node);
                node
            };
            let enabled = info(
                Select::new("enabled")
                    .open(true)
                    .accessibility_label("Programming language")
                    .accessibility_value("Rust"),
            );
            // Open, so the expanded assertion below says something about
            // `disabled` rather than about the default open state.
            let disabled = info(Select::new("disabled").open(true).disabled(true));

            assert_eq!(enabled.label(), Some("Programming language"));
            assert_eq!(enabled.value(), Some("Rust"));
            assert_eq!(enabled.is_expanded(), Some(true));
            assert_eq!(
                disabled.is_expanded(),
                Some(true),
                "a disabled control still reports the state it is in"
            );
            assert!(enabled.supports_action(accesskit::Action::Click));
            assert!(!disabled.supports_action(accesskit::Action::Click));
        });
    }
}
