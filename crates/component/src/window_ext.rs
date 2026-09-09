use crate::{
    Placement, Root,
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
    fn open_sheet<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;

    /// Opens a Sheet at the given placement.
    fn open_sheet_at<F>(&mut self, placement: Placement, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;

    /// Return true, if there is an active Sheet.
    fn has_active_sheet(&mut self, cx: &mut App) -> bool;

    /// Closes the active Sheet.
    fn close_sheet(&mut self, cx: &mut App);

    /// Opens a Dialog.
    fn open_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static;

    /// Opens an AlertDialog.
    ///
    /// This is a convenience method for opening an alert dialog with opinionated defaults.
    /// The footer buttons are center-aligned and include an icon based on the variant.
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

    /// Returns the merged selected text across registered selectable regions
    /// in this window, in logical document order and joined with `\n`.
    #[deprecated(note = "use gpui_base::TextSelection::selected_text instead")]
    fn selected_text(&mut self, cx: &mut App) -> String;

    /// Returns true if any registered region has an active text selection in
    /// this window, including renderer-local selections such as select-all.
    #[deprecated(note = "use gpui_base::TextSelection::has_selection instead")]
    fn has_text_selection(&mut self, cx: &mut App) -> bool;

    /// Clears the window text selection and all registered renderer-local selections.
    #[deprecated(note = "use gpui_base::TextSelection::clear instead")]
    fn clear_text_selection(&mut self, cx: &mut App);

    /// Ends the in-progress window-level text selection drag (if any).
    #[deprecated(note = "use gpui_base::TextSelection::end instead")]
    fn end_text_selection(&mut self, cx: &mut App);
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
        Root::update(self, cx, move |root, window, cx| {
            root.open_sheet_at(placement, build, window, cx);
        })
    }

    #[inline]
    fn has_active_sheet(&mut self, cx: &mut App) -> bool {
        Root::read(self, cx).active_sheet.is_some()
    }

    #[inline]
    fn close_sheet(&mut self, cx: &mut App) {
        Root::update(self, cx, |root, window, cx| {
            root.close_sheet(window, cx);
        })
    }

    #[inline]
    fn open_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    {
        Root::update(self, cx, move |root, window, cx| {
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
        Root::read(self, cx).active_dialogs.len() > 0
    }

    #[inline]
    fn close_dialog(&mut self, cx: &mut App) {
        Root::update(self, cx, |root, window, cx| {
            root.close_dialog(window, cx);
        })
    }

    #[inline]
    fn close_all_dialogs(&mut self, cx: &mut App) {
        Root::update(self, cx, |root, window, cx| {
            root.close_all_dialogs(window, cx);
        })
    }

    #[inline]
    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App) {
        let note = note.into();
        Root::update(self, cx, |root, window, cx| {
            root.push_notification(note, window, cx);
        })
    }

    #[inline]
    fn remove_notification<T: Sized + 'static>(&mut self, cx: &mut App) {
        Root::update(self, cx, |root, window, cx| {
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
        Root::update(self, cx, |root, window, cx| {
            root.remove_notification1::<T>(key, window, cx);
        })
    }

    #[inline]
    fn clear_notifications(&mut self, cx: &mut App) {
        Root::update(self, cx, |root, window, cx| {
            root.clear_notifications(window, cx);
        })
    }

    #[inline]
    fn notifications(&mut self, cx: &mut App) -> Rc<Vec<Entity<Notification>>> {
        Rc::new(Root::read(self, cx).notification.read(cx).notifications())
    }

    #[inline]
    fn has_focused_input(&mut self, cx: &mut App) -> bool {
        self.focused_input(cx).is_some()
    }

    fn focused_input(&mut self, cx: &mut App) -> Option<AnyInputState> {
        let state = Root::read(self, cx).focused_input.clone()?;
        if state.focus_handle(cx).is_focused(self) {
            return Some(state);
        }

        // An input removed from the tree while focused never re-renders to
        // unregister itself; drop the stale registration lazily.
        Root::try_update(self, cx, |root, _, cx| {
            if root.focused_input.as_ref() == Some(&state) {
                root.focused_input = None;
                cx.notify();
            }
        });
        None
    }

    #[inline]
    fn selected_text(&mut self, cx: &mut App) -> String {
        gpui_base::TextSelection::selected_text(self, cx)
    }

    #[inline]
    fn has_text_selection(&mut self, cx: &mut App) -> bool {
        gpui_base::TextSelection::has_selection(self, cx)
    }

    #[inline]
    fn clear_text_selection(&mut self, cx: &mut App) {
        gpui_base::TextSelection::clear(self, cx);
    }

    #[inline]
    fn end_text_selection(&mut self, cx: &mut App) {
        gpui_base::TextSelection::end(self, cx);
    }
}
