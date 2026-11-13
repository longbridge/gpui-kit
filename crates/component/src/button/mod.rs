mod button;
mod button_group;
mod button_icon;
mod dropdown_button;
mod toggle;

pub use button::*;
pub use button_group::*;
pub(crate) use button_icon::*;
pub use dropdown_button::*;
pub use toggle::*;

/// Reads the start/end corners and edges a group assigns to its children as
/// physical ones for the global layout direction: in RTL the group's first
/// child sits on the right, so its "left" corners become the right ones.
pub(crate) fn logical_corners_edges(
    corners: gpui::Corners<bool>,
    edges: gpui::Edges<bool>,
    cx: &gpui::App,
) -> (gpui::Corners<bool>, gpui::Edges<bool>) {
    if !gpui_base::LayoutDirection::global(cx).is_rtl() {
        return (corners, edges);
    }
    (
        gpui::Corners {
            top_left: corners.top_right,
            top_right: corners.top_left,
            bottom_left: corners.bottom_right,
            bottom_right: corners.bottom_left,
        },
        gpui::Edges {
            left: edges.right,
            right: edges.left,
            top: edges.top,
            bottom: edges.bottom,
        },
    )
}
