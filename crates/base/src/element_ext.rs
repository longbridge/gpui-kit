use gpui::{
    App, Bounds, ElementId, IntoElement, ParentElement, Pixels, Styled as _, Window, canvas,
};

use crate::TextSelectionScopeId;

/// Extends a GPUI parent element with post-layout prepaint observation.
pub trait ElementExt: ParentElement + Sized {
    /// Marks this element subtree as belonging to a text-selection scope.
    fn text_selection_scope(self, scope: TextSelectionScopeId) -> impl IntoElement
    where
        Self: IntoElement,
    {
        crate::text_selection::text_selection_scope(scope, self)
    }

    /// Marks this element as a control that acts on the window's text
    /// selection, such as a button beside it: pressing anywhere on it
    /// neither clears the selection nor starts a new one, so its click still
    /// finds the selection. `id` must be unique among this element's
    /// children. Works inside a cached view.
    ///
    /// The window remembers each control it has drawn until it closes, so
    /// mark the row of buttons, not each row of a long list.
    fn text_selection_control(self, id: impl Into<ElementId>) -> Self {
        self.child(crate::text_selection::text_selection_control(id.into()))
    }

    /// Invokes `callback` during prepaint with this element's resolved bounds.
    fn on_prepaint<F>(self, callback: F) -> Self
    where
        F: FnOnce(Bounds<Pixels>, &mut Window, &mut App) + 'static,
    {
        self.child(
            canvas(
                move |bounds, window, cx| callback(bounds, window, cx),
                |_, _, _, _| {},
            )
            // Without an inset, an absolute child starts after the padding but
            // is still sized to the padding box, overhanging the far edges.
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }
}

impl<T: ParentElement> ElementExt for T {}
