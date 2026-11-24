use gpui::{
    App, Bounds, Edges, Element, ElementId, GlobalElementId, InspectorElementId, LayoutId, Pixels,
    Position, Role, SharedString, Style, Window, accesskit, px, size,
};

/// Text that assistive technology announces whenever it changes, without
/// moving focus or the reader's place.
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
/// The region draws nothing. It is laid out as a 1px absolutely positioned box
/// in its parent's top-left corner, because a node without area may be left out
/// of the platform tree. Keep its ID stable across frames: a new ID is a new
/// node, which platforms do not announce as a change.
///
/// ```
/// # mod gpui_kit { pub extern crate gpui_component as component; }
/// use gpui::{div, prelude::*};
/// use gpui_kit::component::live_region::LiveRegion;
///
/// let saved = 3;
/// div()
///     .relative()
///     .child(format!("Saved {saved} files"))
///     .child(LiveRegion::new("save-status", format!("Saved {saved} files")));
/// ```
pub struct LiveRegion {
    id: ElementId,
    text: SharedString,
    live: accesskit::Live,
}

impl LiveRegion {
    /// Creates a polite live region announcing `text`.
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            live: accesskit::Live::Polite,
        }
    }

    /// Announces changes immediately, interrupting current speech. Reserve it
    /// for urgent status such as an error that blocks the current task.
    pub fn assertive(mut self) -> Self {
        self.live = accesskit::Live::Assertive;
        self
    }
}

impl Element for LiveRegion {
    type RequestLayoutState = ();
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
    ) -> (LayoutId, ()) {
        let style = Style {
            position: Position::Absolute,
            inset: Edges {
                top: px(0.).into(),
                left: px(0.).into(),
                ..Default::default()
            },
            size: size(px(1.).into(), px(1.).into()),
            ..Default::default()
        };
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn a11y_role(&self) -> Option<Role> {
        Some(Role::Status)
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        node.set_label(self.text.to_string());
        node.set_value(self.text.to_string());
        node.set_live(self.live);
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
    fn assertive_interrupts_current_speech() {
        let node = node(&LiveRegion::new("error", "Disk full").assertive());

        assert_eq!(node.live(), Some(accesskit::Live::Assertive));
    }
}
