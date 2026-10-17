use crate::{
    ActiveTheme, ElementExt, Placement,
    dialog::{ANIMATION_DURATION, Dialog},
    input::AnyInputState,
    native_menu::FallbackMenuOverlay,
    notification::{Notification, NotificationList},
    sheet::Sheet,
    tooltip::render_tooltip,
    touch_selection::WindowTouchSelectionOverlay,
    window_border,
};
use gpui::{
    App, AppContext, Bounds, Context, DefiniteLength, DispatchPhase, ElementId, Entity,
    FocusHandle, GlobalElementId, InteractiveElement, IntoElement, MouseMoveEvent,
    ParentElement as _, Pixels, Render, RenderOnce, Styled, WeakFocusHandle, Window, canvas, div,
    prelude::FluentBuilder as _,
};
use gpui_base::{TextSelection, TextSelectionScopeId};
use std::{any::TypeId, rc::Rc};

pub(crate) fn init(cx: &mut App) {
    gpui_base::Root::register_plugin::<WindowState>(cx, WindowState::new);
}

/// Component-owned window state and presentation; Base owns the actual root.
pub(crate) struct WindowState {
    pub(crate) active_sheet: Option<ActiveSheet>,
    pub(crate) active_dialogs: Vec<ActiveDialog>,
    pub(super) focused_input: Option<AnyInputState>,
    pub notification: Entity<NotificationList>,
    pub(crate) tooltip_overlay: Entity<gpui_base::TooltipOverlay>,
    tooltip_hovered_trigger: Option<GlobalElementId>,
    /// Bounds of the tooltip trigger last pressed, whose tooltip stays
    /// dismissed until the pointer moves outside them.
    tooltip_pressed_trigger: Option<Bounds<Pixels>>,
    pub(crate) native_menu_overlay: Entity<FallbackMenuOverlay>,
    touch_selection_overlay: Entity<WindowTouchSelectionOverlay>,
    sheet_size: Option<DefiniteLength>,
    pending_focus_restore: Option<Rc<PendingFocusRestore>>,
}

struct PendingFocusRestore {
    handle: WeakFocusHandle,
}

#[derive(Clone)]
pub(crate) struct ActiveSheet {
    focus_handle: FocusHandle,
    /// The previous focused handle before opening the Sheet.
    previous_focused_handle: Option<WeakFocusHandle>,
    placement: Placement,
    selection_scope: TextSelectionScopeId,
    builder: Rc<dyn Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static>,
}

#[derive(Clone)]
pub(crate) struct ActiveDialog {
    focus_handle: FocusHandle,
    /// The previous focused handle before opening the Dialog.
    previous_focused_handle: Option<WeakFocusHandle>,
    selection_scope: TextSelectionScopeId,
    builder: Rc<dyn Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static>,
}

impl ActiveDialog {
    pub(crate) fn new(
        focus_handle: FocusHandle,
        previous_focused_handle: Option<WeakFocusHandle>,
        selection_scope: TextSelectionScopeId,
        builder: impl Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    ) -> Self {
        Self {
            focus_handle,
            previous_focused_handle,
            selection_scope,
            builder: Rc::new(builder),
        }
    }
}

impl WindowState {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            active_sheet: None,
            active_dialogs: Vec::new(),
            focused_input: None,
            notification: cx.new(|cx| NotificationList::new(window, cx)),
            tooltip_overlay: cx
                .new(|_| gpui_base::TooltipOverlay::new().render_with(render_tooltip)),
            tooltip_hovered_trigger: None,
            tooltip_pressed_trigger: None,
            native_menu_overlay: cx.new(|_| FallbackMenuOverlay::new()),
            touch_selection_overlay: cx.new(|cx| WindowTouchSelectionOverlay::new(window, cx)),
            sheet_size: None,
            pending_focus_restore: None,
        }
    }

    fn entity(window: &Window, cx: &App) -> Option<Entity<Self>> {
        window.root::<gpui_base::Root>()??.read(cx).plugin::<Self>()
    }

    fn allocate_text_selection_scope(&mut self) -> TextSelectionScopeId {
        TextSelectionScopeId::new()
    }

    pub(crate) fn active_text_selection_scope(&self) -> TextSelectionScopeId {
        self.active_dialogs
            .last()
            .map(|dialog| dialog.selection_scope)
            .or_else(|| {
                self.active_sheet
                    .as_ref()
                    .map(|sheet| sheet.selection_scope)
            })
            .unwrap_or_default()
    }

    pub fn update<F, R>(window: &mut Window, cx: &mut App, f: F) -> R
    where
        F: FnOnce(&mut Self, &mut Window, &mut Context<Self>) -> R,
    {
        let root = Self::entity(window, cx).expect(ROOT_MISSING);

        root.update(cx, |root, cx| f(root, window, cx))
    }

    pub(crate) fn try_read<'a>(window: &Window, cx: &'a App) -> Option<&'a Self> {
        Self::entity(window, cx).map(|root| root.read(cx))
    }

    pub(crate) fn try_update<F, R>(window: &mut Window, cx: &mut App, f: F) -> Option<R>
    where
        F: FnOnce(&mut Self, &mut Window, &mut Context<Self>) -> R,
    {
        let root = Self::entity(window, cx)?;
        Some(root.update(cx, |root, cx| f(root, window, cx)))
    }

    pub fn read<'a>(window: &'a Window, cx: &'a App) -> &'a Self {
        Self::entity(window, cx).expect(ROOT_MISSING).read(cx)
    }

    fn notification_layer(
        root: &Entity<WindowState>,
        cx: &App,
    ) -> Option<impl IntoElement + use<>> {
        let active_sheet_placement = root.read(cx).active_sheet.clone().map(|d| d.placement);

        let sheet_size = root.read(cx).sheet_size;
        let (mt, mr, mb, ml) = match active_sheet_placement {
            Some(Placement::Top) => (sheet_size, None, None, None),
            Some(Placement::Right) => (None, sheet_size, None, None),
            Some(Placement::Bottom) => (None, None, sheet_size, None),
            Some(Placement::Left) => (None, None, None, sheet_size),
            _ => (None, None, None, None),
        };

        Some(
            div()
                .absolute()
                .inset_0()
                .when_some(mt, |this, offset| this.mt(offset))
                .when_some(mr, |this, offset| this.mr(offset))
                .when_some(mb, |this, offset| this.mb(offset))
                .when_some(ml, |this, offset| this.ml(offset))
                .child(root.read(cx).notification.clone()),
        )
    }

    fn sheet_layer(
        root: Entity<WindowState>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<impl IntoElement + use<>> {
        if let Some(active_sheet) = root.read(cx).active_sheet.clone() {
            let mut sheet = Sheet::new(window, cx);
            sheet = (active_sheet.builder)(sheet, window, cx);
            sheet.focus_handle = active_sheet.focus_handle.clone();
            sheet.placement = active_sheet.placement;
            sheet.selection_scope = active_sheet.selection_scope;

            let size = sheet.size;

            return Some(
                div()
                    .relative()
                    .child(sheet)
                    .on_prepaint(move |_, _, cx| root.update(cx, |r, _| r.sheet_size = Some(size))),
            );
        }

        None
    }

    fn dialog_layer(
        root: &Entity<WindowState>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<impl IntoElement + use<>> {
        let active_dialogs = root.read(cx).active_dialogs.clone();

        if active_dialogs.is_empty() {
            return None;
        }

        let mut show_overlay_ix = None;

        let mut dialogs = active_dialogs
            .iter()
            .enumerate()
            .map(|(i, active_dialog)| {
                let mut dialog = Dialog::new(cx);

                dialog = (active_dialog.builder)(dialog, window, cx);

                // Give the dialog the focus handle, because `dialog` is a temporary value, is not possible to
                // keep the focus handle in the dialog.
                //
                // So we keep the focus handle in the `active_dialog`, this is owned by the `WindowState`.
                dialog.focus_handle = active_dialog.focus_handle.clone();
                dialog.selection_scope = active_dialog.selection_scope;

                dialog.layer_ix = i;
                // Find the dialog which one needs to show overlay.
                if dialog.has_overlay() {
                    show_overlay_ix = Some(i);
                }

                dialog
            })
            .collect::<Vec<_>>();

        if let Some(ix) = show_overlay_ix {
            if let Some(dialog) = dialogs.get_mut(ix) {
                dialog.props.overlay_visible = true;
            }
        }

        // Named so a test can assert the layer actually reached the screen. A
        // dialog that opens into a root which never renders this layer looks
        // exactly like one that does not open.
        Some(
            div()
                .debug_selector(|| "dialog-layer".to_string())
                .children(dialogs),
        )
    }

    pub fn open_dialog<F>(
        &mut self,
        build: F,
        window: &mut Window,
        cx: &mut Context<'_, WindowState>,
    ) where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    {
        let mut previous_focused_handle = window.focused(cx).map(|h| h.downgrade());

        // Use pending focus restore if available to maintain correct focus chain
        // when a new dialog is opened immediately after closing another dialog.
        if let Some(pending) = self.pending_focus_restore.take() {
            previous_focused_handle = Some(pending.handle.clone());
        }

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);

        let selection_scope = self.allocate_text_selection_scope();
        self.active_dialogs.push(ActiveDialog::new(
            focus_handle,
            previous_focused_handle,
            selection_scope,
            build,
        ));
        // Opening a modal confines selection to it; drop any background
        // selection so it cannot linger (or be copied) under the modal.
        gpui_base::TextSelection::clear(window, cx);
        cx.notify();
    }

    fn close_dialog_internal(&mut self) -> Option<FocusHandle> {
        let dialog = self.active_dialogs.pop()?;
        self.focused_input = None;
        self.pending_focus_restore = None;
        dialog
            .previous_focused_handle
            .and_then(|handle| handle.upgrade())
    }

    pub fn close_dialog(&mut self, window: &mut Window, cx: &mut Context<'_, WindowState>) {
        if let Some(handle) = self.close_dialog_internal() {
            window.focus(&handle, cx);
        }
        gpui_base::TextSelection::clear(window, cx);
        cx.notify();
    }

    pub(crate) fn defer_close_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, WindowState>,
    ) {
        if let Some(handle) = self.close_dialog_internal() {
            // Save for new dialogs opened during animation to maintain focus chain
            let pending = Rc::new(PendingFocusRestore {
                handle: handle.downgrade(),
            });
            self.pending_focus_restore = Some(pending.clone());

            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(*ANIMATION_DURATION).await;
                let _ = this.update_in(cx, |this, window, cx| {
                    // A newer dialog can have the same stack depth, and a
                    // newer close can return to the same opener. Only this
                    // exact pending request may restore or clear its target.
                    if !this
                        .pending_focus_restore
                        .as_ref()
                        .is_some_and(|current| Rc::ptr_eq(current, &pending))
                    {
                        return;
                    }
                    this.pending_focus_restore = None;
                    if let Some(handle) = pending.handle.upgrade() {
                        window.focus(&handle, cx);
                    }
                });
            })
            .detach();
        }
        gpui_base::TextSelection::clear(window, cx);
        cx.notify();
    }

    pub fn close_all_dialogs(&mut self, window: &mut Window, cx: &mut Context<'_, WindowState>) {
        self.focused_input = None;
        let previous_focused_handle = match self.active_dialogs.first() {
            Some(dialog) => dialog.previous_focused_handle.clone(),
            None => self
                .pending_focus_restore
                .as_ref()
                .map(|pending| pending.handle.clone()),
        };
        self.active_dialogs.clear();
        self.pending_focus_restore = None;
        if let Some(handle) = previous_focused_handle.and_then(|h| h.upgrade()) {
            window.focus(&handle, cx);
        }
        gpui_base::TextSelection::clear(window, cx);
        cx.notify();
    }

    pub fn open_sheet_at<F>(
        &mut self,
        placement: Placement,
        build: F,
        window: &mut Window,
        cx: &mut Context<'_, WindowState>,
    ) where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static,
    {
        let previous_focused_handle = self
            .active_sheet
            .take()
            .and_then(|s| s.previous_focused_handle)
            .or_else(|| window.focused(cx).map(|h| h.downgrade()));

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);
        let selection_scope = self.allocate_text_selection_scope();
        self.active_sheet = Some(ActiveSheet {
            focus_handle,
            previous_focused_handle,
            placement,
            selection_scope,
            builder: Rc::new(build),
        });
        // Opening a modal confines selection to it; drop any background
        // selection so it cannot linger (or be copied) under the modal.
        gpui_base::TextSelection::clear(window, cx);
        cx.notify();
    }

    pub fn close_sheet(&mut self, window: &mut Window, cx: &mut Context<'_, WindowState>) {
        self.focused_input = None;
        if let Some(previous_handle) = self
            .active_sheet
            .as_ref()
            .and_then(|s| s.previous_focused_handle.as_ref())
            .and_then(|h| h.upgrade())
        {
            window.focus(&previous_handle, cx);
        }
        self.active_sheet = None;
        gpui_base::TextSelection::clear(window, cx);
        cx.notify();
    }

    pub fn push_notification(
        &mut self,
        note: impl Into<Notification>,
        window: &mut Window,
        cx: &mut Context<'_, WindowState>,
    ) {
        self.notification
            .update(cx, |view, cx| view.push(note, window, cx));
        cx.notify();
    }

    /// Removes all notifications whose id matches `T`, including ones registered with
    /// either [`Notification::id`] or [`Notification::id1`] (any key).
    pub fn remove_notification<T: Sized + 'static>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, WindowState>,
    ) {
        self.notification.update(cx, |view, cx| {
            view.close_by_type(TypeId::of::<T>(), window, cx);
        });
        cx.notify();
    }

    /// Removes the notification matching the given type and element id (paired with [`Notification::id1`]).
    pub fn remove_notification1<T: Sized + 'static>(
        &mut self,
        key: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut Context<'_, WindowState>,
    ) {
        let key = key.into();
        self.notification.update(cx, |view, cx| {
            view.close((TypeId::of::<T>(), key), window, cx);
        });
        cx.notify();
    }

    pub fn clear_notifications(&mut self, window: &mut Window, cx: &mut Context<'_, WindowState>) {
        self.notification
            .update(cx, |view, cx| view.clear(window, cx));
        cx.notify();
    }

    /// Get the tooltip overlay entity for this window.
    pub(crate) fn tooltip_overlay(
        window: &Window,
        cx: &App,
    ) -> Option<Entity<gpui_base::TooltipOverlay>> {
        let root = Self::entity(window, cx)?;
        Some(root.read(cx).tooltip_overlay.clone())
    }

    /// Only the current trigger may request a hide. Hover listeners run in
    /// reverse paint order, so a new trigger can enter before the old one leaves.
    pub(crate) fn tooltip_overlay_for_trigger(
        trigger: &GlobalElementId,
        hovered: bool,
        window: &Window,
        cx: &mut App,
    ) -> Option<Entity<gpui_base::TooltipOverlay>> {
        Self::entity(window, cx)?.update(cx, |state, _| {
            if hovered {
                state.tooltip_hovered_trigger = Some(trigger.clone());
            } else if state.tooltip_hovered_trigger.as_ref() == Some(trigger) {
                state.tooltip_hovered_trigger = None;
            } else {
                return None;
            }
            Some(state.tooltip_overlay.clone())
        })
    }

    /// Keeps the tooltip of the trigger at `bounds` dismissed until the
    /// pointer moves outside them.
    pub(crate) fn press_tooltip_trigger(bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        Self::try_update(window, cx, |state, _, cx| {
            state.tooltip_pressed_trigger = Some(bounds);
            cx.notify();
        });
    }

    pub(crate) fn is_tooltip_trigger_pressed(
        bounds: Bounds<Pixels>,
        window: &Window,
        cx: &App,
    ) -> bool {
        Self::try_read(window, cx)
            .is_some_and(|state| state.tooltip_pressed_trigger == Some(bounds))
    }

    /// Ends the press on the first pointer move outside the trigger. It is
    /// window-level because a removed or moved trigger never reports a
    /// hover-leave.
    fn tooltip_press_listener(&self, cx: &Context<Self>) -> Option<impl IntoElement + use<>> {
        let bounds = self.tooltip_pressed_trigger?;
        let state = cx.entity().downgrade();
        Some(canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                    if phase != DispatchPhase::Capture || bounds.contains(&event.position) {
                        return;
                    }
                    _ = state.update(cx, |state, cx| {
                        if state.tooltip_pressed_trigger == Some(bounds) {
                            state.tooltip_pressed_trigger = None;
                            cx.notify();
                        }
                    });
                });
            },
        ))
    }

    /// Get the fallback native-menu overlay entity for this window.
    pub(crate) fn native_menu_overlay(
        window: &Window,
        cx: &App,
    ) -> Option<Entity<FallbackMenuOverlay>> {
        let root = Self::entity(window, cx)?;
        Some(root.read(cx).native_menu_overlay.clone())
    }
}

impl gpui_base::RootPlugin for WindowState {
    fn prepare(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.set_rem_size(cx.theme().font_size);
        TextSelection::activate_scope(self.active_text_selection_scope(), window, cx);
    }

    fn style(&self, surface: &mut gpui::Stateful<gpui::Div>, _window: &mut Window, cx: &mut App) {
        use gpui::Refineable as _;
        surface.style().refine(
            &gpui::StyleRefinement::default()
                .font_family(cx.theme().font_family.clone())
                .bg(cx.theme().tokens.background)
                .text_color(cx.theme().foreground),
        );
    }

    fn decorate(
        &self,
        surface: gpui::AnyElement,
        _root: &gpui_base::Root,
        _window: &mut Window,
        _cx: &mut App,
    ) -> impl IntoElement {
        window_border().child(surface)
    }
}

impl Render for WindowState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .inset_0()
            .child(WindowStateLayers { root: cx.entity() })
            .child(self.touch_selection_overlay.clone())
            .child(self.tooltip_overlay.clone())
            .children(self.tooltip_press_listener(cx))
            .child(self.native_menu_overlay.clone())
    }
}

const ROOT_MISSING: &str =
    "component window state is missing; call gpui_component::init before gpui_kit::open_window";

/// Window-level layers, always mounted once after the application content.
/// Child view caching does not affect their ownership or rendering.
#[derive(IntoElement)]
struct WindowStateLayers {
    root: Entity<WindowState>,
}

impl RenderOnce for WindowStateLayers {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let root = self.root;
        div()
            .absolute()
            .inset_0()
            .debug_selector(|| "root-layers".to_string())
            .children(WindowState::sheet_layer(root.clone(), window, cx))
            .children(WindowState::dialog_layer(&root, window, cx))
            .children(WindowState::notification_layer(&root, cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WindowExt as _;
    use gpui::{TestAppContext, VisualTestContext, px};
    use std::time::Duration;

    struct Host(FocusHandle);

    impl Render for Host {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                div()
                    .id("opener")
                    .track_focus(&self.0)
                    .tab_index(0)
                    .size(px(40.)),
            )
        }
    }

    fn window(cx: &mut TestAppContext) -> (&mut VisualTestContext, FocusHandle) {
        cx.update(|cx| {
            crate::init(cx);
            cx.set_reduce_motion(true);
        });
        let opener = cx.update(|cx| cx.focus_handle().tab_stop(true));
        let (_, cx) = cx.add_window_view({
            let opener = opener.clone();
            move |window, cx| {
                let host = cx.new(|_| Host(opener));
                crate::Root::new(host, window, cx)
            }
        });
        cx.update(|window, cx| {
            opener.focus(window, cx);
            window.draw(cx).clear(cx);
        });
        (cx, opener)
    }

    fn open(cx: &mut VisualTestContext, title: &'static str) -> FocusHandle {
        let control = cx.update(|_, cx| cx.focus_handle().tab_stop(true));
        cx.update({
            let control = control.clone();
            move |window, cx| {
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog.title(title).child(
                        div()
                            .id(title)
                            .track_focus(&control)
                            .tab_index(0)
                            .size(px(40.)),
                    )
                });
            }
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            control.focus(window, cx);
            window.draw(cx).clear(cx);
            assert!(
                WindowState::read(window, cx)
                    .active_dialogs
                    .last()
                    .unwrap()
                    .focus_handle
                    .contains_focused(window, cx)
            );
        });
        control
    }

    fn defer_close(cx: &mut VisualTestContext) {
        cx.update(|window, cx| {
            WindowState::update(window, cx, |state, window, cx| {
                state.defer_close_dialog(window, cx)
            });
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }

    fn advance(cx: &mut VisualTestContext, duration: Duration) {
        cx.executor().advance_clock(duration);
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn assert_focus(cx: &mut VisualTestContext, handle: &FocusHandle) {
        cx.update(|window, cx| assert_eq!(window.focused(cx).as_ref(), Some(handle)));
    }

    #[gpui::test]
    fn delayed_close_cannot_focus_a_replaced_dialog_at_the_same_stack_depth(
        cx: &mut TestAppContext,
    ) {
        let (cx, opener) = window(cx);
        let _outer = open(cx, "Outer");
        let _inner = open(cx, "Inner");
        defer_close(cx);
        cx.update(|window, cx| window.close_dialog(cx));
        let replacement = open(cx, "Replacement");

        // The old close timer expects one remaining dialog. There is still
        // one, but its identity and opener have both changed.
        advance(cx, *ANIMATION_DURATION);
        assert_focus(cx, &replacement);
        cx.update(|window, cx| window.close_dialog(cx));
        assert_focus(cx, &opener);
    }

    #[gpui::test]
    fn older_close_timer_cannot_clear_a_newer_pending_restore(cx: &mut TestAppContext) {
        let (cx, opener) = window(cx);
        let _first = open(cx, "First");
        defer_close(cx);
        advance(cx, Duration::from_millis(100));
        let _second = open(cx, "Second");
        defer_close(cx);

        // Only the first close timer has elapsed. The second still owns the
        // pending opener until its own animation window completes.
        advance(cx, *ANIMATION_DURATION - Duration::from_millis(100));
        cx.update(|window, cx| {
            assert!(
                WindowState::read(window, cx)
                    .pending_focus_restore
                    .is_some()
            );
            assert_ne!(window.focused(cx).as_ref(), Some(&opener));
        });
        advance(cx, Duration::from_millis(100));
        assert_focus(cx, &opener);
    }

    #[gpui::test]
    fn delayed_nested_close_and_replacement_preserve_the_original_opener(cx: &mut TestAppContext) {
        let (cx, opener) = window(cx);
        let outer = open(cx, "Outer");
        let _inner = open(cx, "Inner");
        defer_close(cx);
        advance(cx, *ANIMATION_DURATION);
        assert_focus(cx, &outer);

        // Replacing the outer dialog during its close animation inherits the
        // original background opener, not the unmounted dialog control.
        defer_close(cx);
        let replacement = open(cx, "Replacement");
        advance(cx, *ANIMATION_DURATION);
        assert_focus(cx, &replacement);
        cx.update(|window, cx| window.close_dialog(cx));
        assert_focus(cx, &opener);
    }

    #[gpui::test]
    fn closing_all_dialogs_retires_pending_restoration(cx: &mut TestAppContext) {
        let (cx, opener) = window(cx);
        let _outer = open(cx, "Outer");
        let _inner = open(cx, "Inner");
        defer_close(cx);
        cx.update(|window, cx| window.close_all_dialogs(cx));
        assert_focus(cx, &opener);
        let replacement = open(cx, "Replacement");
        advance(cx, *ANIMATION_DURATION);
        assert_focus(cx, &replacement);

        // Closing all while the last dialog is already leaving completes its
        // return to the opener immediately and leaves no stale timer owner.
        defer_close(cx);
        cx.update(|window, cx| {
            window.close_all_dialogs(cx);
            assert!(
                WindowState::read(window, cx)
                    .pending_focus_restore
                    .is_none()
            );
        });
        assert_focus(cx, &opener);
        let next = open(cx, "Next");
        advance(cx, *ANIMATION_DURATION);
        assert_focus(cx, &next);
    }
}
