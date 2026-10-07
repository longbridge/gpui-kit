use crate::root::WindowState;
use crate::{
    Placement,
    dialog::{AlertDialog, Dialog},
    input::AnyInputState,
    notification::Notification,
    sheet::Sheet,
};
use gpui::{App, ElementId, Entity, Window};
use std::rc::Rc;

/// Extension trait for [`Window`] to add dialog, sheet .. functionality.
pub trait WindowExt: Sized {
    /// Opens a Sheet at right placement.
    ///
    /// The builder follows the same per-frame contract as
    /// [`WindowExt::open_dialog`]: it re-runs on every frame the sheet stays
    /// open, so keep it cheap and create entities before opening, not inside
    /// the builder.
    fn open_sheet<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;

    /// Opens a Sheet at the given placement.
    ///
    /// The builder follows the same per-frame contract as
    /// [`WindowExt::open_dialog`]: it re-runs on every frame the sheet stays
    /// open, so keep it cheap and create entities before opening, not inside
    /// the builder.
    fn open_sheet_at<F>(&mut self, placement: Placement, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;

    /// Return true, if there is an active Sheet.
    fn has_active_sheet(&mut self, cx: &mut App) -> bool;

    /// Closes the active Sheet.
    fn close_sheet(&mut self, cx: &mut App);

    /// Opens a Dialog.
    ///
    /// The builder is a render closure, not a one-shot constructor: the dialog
    /// layer re-runs it on every frame the dialog stays open, which is what
    /// lets an open dialog reflect fresh state. That contract comes with two
    /// rules:
    ///
    /// - Keep the builder cheap and idempotent, it runs on every frame.
    /// - Do not create entities inside it. Anything from `cx.new(...)` — an
    ///   [`InputState`](crate::input::InputState), a child view —
    ///   is dropped and rebuilt every frame, which resets input while the
    ///   user types. Create entities before opening the dialog and clone them
    ///   into the builder.
    ///
    /// To move focus into the dialog's content, focus from inside the builder:
    /// `open_dialog` focuses the dialog shell when it is called, and the
    /// builder runs after that, on the dialog's first render, so a handle
    /// focused earlier — for example in the constructor of an entity created
    /// before the call — is overwritten by the shell.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use gpui_kit::component::input::{Input, InputState};
    ///
    /// // Created before opening, cloned into the builder: the input keeps
    /// // its value across the builder's per-frame re-runs.
    /// let input = cx.new(|cx| InputState::new(window, cx).placeholder("URL"));
    /// window.open_dialog(cx, move |dialog, _, _| {
    ///     dialog
    ///         .title("Import")
    ///         .child(Input::new(&input))
    /// });
    /// ```
    fn open_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static;

    /// Opens an AlertDialog.
    ///
    /// This is a convenience method for opening an alert dialog with opinionated defaults.
    /// The footer buttons are center-aligned and include an icon based on the variant.
    ///
    /// The builder follows the same per-frame contract as
    /// [`WindowExt::open_dialog`]: it re-runs on every frame the dialog stays
    /// open, so keep it cheap and create entities before opening, not inside
    /// the builder.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use gpui_kit::component::{AlertDialog, alert::AlertVariant};
    ///
    /// window.open_alert_dialog(cx, |alert, _, _| {
    ///     alert.warning()
    ///         .title("Unsaved Changes")
    ///         .description("You have unsaved changes. Are you sure you want to leave?")
    ///         .show_cancel(true)
    /// });
    /// ```
    fn open_alert_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(AlertDialog, &mut Window, &mut App) -> AlertDialog + 'static;

    /// Return true, if there is an active Dialog.
    fn has_active_dialog(&mut self, cx: &mut App) -> bool;

    /// Closes the last active Dialog.
    fn close_dialog(&mut self, cx: &mut App);

    /// Closes all active Dialogs.
    fn close_all_dialogs(&mut self, cx: &mut App);

    /// Pushes a notification to the notification list.
    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App);

    /// Removes all notifications whose id matches `T`, including ones registered with
    /// either `Notification::id` or `Notification::id1` (any key).
    fn remove_notification<T: Sized + 'static>(&mut self, cx: &mut App);

    /// Removes a single notification matching the given type `T` and `key` (paired with `Notification::id1`).
    fn remove_notification1<T: Sized + 'static>(&mut self, key: impl Into<ElementId>, cx: &mut App);

    /// Clears all notifications.
    fn clear_notifications(&mut self, cx: &mut App);

    /// Returns number of notifications.
    fn notifications(&mut self, cx: &mut App) -> Rc<Vec<Entity<Notification>>>;

    /// Return the currently focused input state.
    ///
    /// Covers `Input`, `Textarea`, `Editor` and `OtpInput`, use
    /// [`AnyInputState::as_input`] and friends to get the concrete state.
    /// A registration whose focus handle is no longer focused (e.g. the input
    /// was removed from the tree while focused) is treated as `None`.
    fn focused_input(&mut self, cx: &mut App) -> Option<AnyInputState>;
    /// Returns true if there is a focused Input entity.
    fn has_focused_input(&mut self, cx: &mut App) -> bool;
}

impl WindowExt for Window {
    #[inline]
    fn open_sheet<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static,
    {
        self.open_sheet_at(Placement::Right, cx, build)
    }

    #[inline]
    fn open_sheet_at<F>(&mut self, placement: Placement, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static,
    {
        WindowState::update(self, cx, move |root, window, cx| {
            root.open_sheet_at(placement, build, window, cx);
        })
    }

    #[inline]
    fn has_active_sheet(&mut self, cx: &mut App) -> bool {
        WindowState::read(self, cx).active_sheet.is_some()
    }

    #[inline]
    fn close_sheet(&mut self, cx: &mut App) {
        WindowState::update(self, cx, |root, window, cx| {
            root.close_sheet(window, cx);
        })
    }

    #[inline]
    fn open_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    {
        WindowState::update(self, cx, move |root, window, cx| {
            root.open_dialog(build, window, cx);
        })
    }

    #[inline]
    fn open_alert_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(AlertDialog, &mut Window, &mut App) -> AlertDialog + 'static,
    {
        self.open_dialog(cx, move |_, window, cx| {
            build(AlertDialog::new(cx), window, cx).build_surface(window, cx)
        })
    }

    #[inline]
    fn has_active_dialog(&mut self, cx: &mut App) -> bool {
        !WindowState::read(self, cx).active_dialogs.is_empty()
    }

    #[inline]
    fn close_dialog(&mut self, cx: &mut App) {
        WindowState::update(self, cx, |root, window, cx| {
            root.close_dialog(window, cx);
        })
    }

    #[inline]
    fn close_all_dialogs(&mut self, cx: &mut App) {
        WindowState::update(self, cx, |root, window, cx| {
            root.close_all_dialogs(window, cx);
        })
    }

    #[inline]
    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App) {
        let note = note.into();
        WindowState::update(self, cx, |root, window, cx| {
            root.push_notification(note, window, cx);
        })
    }

    #[inline]
    fn remove_notification<T: Sized + 'static>(&mut self, cx: &mut App) {
        WindowState::update(self, cx, |root, window, cx| {
            root.remove_notification::<T>(window, cx);
        })
    }

    #[inline]
    fn remove_notification1<T: Sized + 'static>(
        &mut self,
        key: impl Into<ElementId>,
        cx: &mut App,
    ) {
        let key = key.into();
        WindowState::update(self, cx, |root, window, cx| {
            root.remove_notification1::<T>(key, window, cx);
        })
    }

    #[inline]
    fn clear_notifications(&mut self, cx: &mut App) {
        WindowState::update(self, cx, |root, window, cx| {
            root.clear_notifications(window, cx);
        })
    }

    #[inline]
    fn notifications(&mut self, cx: &mut App) -> Rc<Vec<Entity<Notification>>> {
        Rc::new(
            WindowState::read(self, cx)
                .notification
                .read(cx)
                .notifications(),
        )
    }

    #[inline]
    fn has_focused_input(&mut self, cx: &mut App) -> bool {
        self.focused_input(cx).is_some()
    }

    fn focused_input(&mut self, cx: &mut App) -> Option<AnyInputState> {
        let state = WindowState::read(self, cx).focused_input.clone()?;
        if state.focus_handle(cx).is_focused(self) {
            return Some(state);
        }

        // An input removed from the tree while focused never re-renders to
        // unregister itself; drop the stale registration lazily.
        WindowState::try_update(self, cx, |root, _, cx| {
            if root.focused_input.as_ref() == Some(&state) {
                root.focused_input = None;
                cx.notify();
            }
        });
        None
    }
}
