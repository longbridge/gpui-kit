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
    App, AppContext, Context, DefiniteLength, ElementId, Entity, FocusHandle, InteractiveElement,
    IntoElement, ParentElement as _, Render, RenderOnce, Styled, WeakFocusHandle, Window, div,
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
    pub(crate) native_menu_overlay: Entity<FallbackMenuOverlay>,
    touch_selection_overlay: Entity<WindowTouchSelectionOverlay>,
    sheet_size: Option<DefiniteLength>,
    pending_focus_restore: Option<WeakFocusHandle>,
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
        if let Some(pending_handle) = self.pending_focus_restore.take() {
            previous_focused_handle = Some(pending_handle);
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
        self.focused_input = None;
        self.active_dialogs
            .pop()
            .and_then(|d| d.previous_focused_handle)
            .and_then(|h| h.upgrade())
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
            let dialogs_count = self.active_dialogs.len();

            // Save for new dialogs opened during animation to maintain focus chain
            self.pending_focus_restore = Some(handle.downgrade());

            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(*ANIMATION_DURATION).await;
                let _ = this.update_in(cx, |this, window, cx| {
                    let current_dialogs_count = this.active_dialogs.len();
                    // Only restore focus if no new dialogs were opened during animation
                    if current_dialogs_count == dialogs_count {
                        window.focus(&handle, cx);
                    }
                    this.pending_focus_restore = None;
                });
            })
            .detach();
        }
        gpui_base::TextSelection::clear(window, cx);
        cx.notify();
    }

    pub fn close_all_dialogs(&mut self, window: &mut Window, cx: &mut Context<'_, WindowState>) {
        self.focused_input = None;
        let previous_focused_handle = self
            .active_dialogs
            .first()
            .and_then(|d| d.previous_focused_handle.clone());
        self.active_dialogs.clear();
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
