use gpui::{
    AnyElement, App, Bounds, Div, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement as _, LayoutId, ParentElement as _, Pixels, Role, SharedString, StyleRefinement,
    Styled, Window, accesskit, div,
};

/// Text that is shown and that assistive technology announces whenever it
/// changes, without moving focus or the reader's place.
///
/// Use it for status the user should hear as it happens: a save finishing, a
/// search's result count, a move played in a game. The owner keeps the text in
/// its own state and passes the current value on every render; the region
/// holds no state of its own.
///
/// GPUI elements cannot mark themselves live, so this element writes
/// AccessKit's `live` property on its own node. Each platform adapter then
/// announces the node when its text changes: macOS reads the value, Windows
/// (UI Automation) and Linux (AT-SPI) read the name, so the text is written to
/// both. Announcements are polite by default, waiting for the current speech
/// to finish; [`LiveRegion::assertive`] interrupts it.
///
/// A region is also announced when it first appears with text, and empty text
/// announces nothing. Platforms announce a change, so setting the same text
/// again is silent. Keep the ID stable across frames: a new ID is a new node,
/// announced again.
///
/// The region draws its text and is the one accessibility node for it, so a
/// reader browsing the view meets the text once. Style it like a `div`. To
/// announce without showing anything, give it no size: `.absolute().size_0()`
/// `.overflow_hidden()`.
///
/// ```
/// # mod gpui_kit { pub extern crate gpui_component as component; }
/// use gpui::prelude::*;
/// use gpui_kit::component::live_region::LiveRegion;
///
/// let saved = 3;
/// LiveRegion::new("save-status", format!("Saved {saved} files")).text_sm();
/// ```
pub struct LiveRegion {
    id: ElementId,
    text: SharedString,
    live: accesskit::Live,
    base: Div,
}

impl LiveRegion {
    /// Creates a polite live region announcing `text`.
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            live: accesskit::Live::Polite,
            base: div(),
        }
    }

    /// Announces changes immediately, interrupting current speech. Reserve it
    /// for urgent status such as an error that blocks the current task.
    pub fn assertive(mut self) -> Self {
        self.live = accesskit::Live::Assertive;
        self
    }
}

impl Styled for LiveRegion {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl Element for LiveRegion {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, AnyElement) {
        // The text is a bare string child, which GPUI keeps out of the
        // accessibility tree; this element's node is the only one carrying it.
        let base = std::mem::replace(&mut self.base, div());
        let mut content = base.child(self.text.clone()).into_any_element();
        (content.request_layout(window, cx), content)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        content: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        content.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        content: &mut AnyElement,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        content.paint(window, cx);
    }

    fn a11y_role(&self) -> Option<Role> {
        Some(Role::Status)
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        node.set_live(self.live);
        if !self.text.is_empty() {
            node.set_label(self.text.to_string());
            node.set_value(self.text.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(region: &LiveRegion) -> accesskit::Node {
        let mut node = accesskit::Node::new(Role::Status);
        region.write_a11y_info(&mut node);
        node
    }

    #[test]
    fn names_its_text_for_every_platform_adapter() {
        let region = LiveRegion::new("status", "Saved 3 files");
        let node = node(&region);

        assert_eq!(region.a11y_role(), Some(Role::Status));
        assert_eq!(node.live(), Some(accesskit::Live::Polite));
        assert_eq!(node.label(), Some("Saved 3 files"));
        assert_eq!(node.value(), Some("Saved 3 files"));
    }

    #[test]
    fn empty_text_announces_nothing() {
        let node = node(&LiveRegion::new("status", ""));

        assert_eq!(node.label(), None);
        assert_eq!(node.value(), None);
    }

    #[test]
    fn assertive_interrupts_current_speech() {
        let node = node(&LiveRegion::new("error", "Disk full").assertive());

        assert_eq!(node.live(), Some(accesskit::Live::Assertive));
    }
}
