//! What this design system paints on a grid layout.
//!
//! Base owns the grid: where items sit, the drag, the resize, the keyboard and
//! the layout arithmetic. Everything here is appearance — the gap, the motion
//! items settle with, the drop placeholder, the resize grip, and how the
//! dragged and the keyboard-selected items stand out.

use gpui::{
    App, ElementId, Entity, InteractiveElement as _, IntoElement as _, ParentElement as _,
    Styled as _, div, prelude::FluentBuilder as _, px,
};
use gpui_base::{Transition, box_shadow};

pub use gpui_base::grid_layout::*;

use crate::{
    styled::{FOCUS_RING_OPACITY, FOCUS_RING_WIDTH},
    theme::ActiveTheme as _,
};

/// The resize corner, which brightens the grip it holds while hovered.
const GRIP_GROUP: &str = "grid-layout-resize-grip";

/// Create a [`GridLayout`] wearing this design system: the theme's medium
/// spacing between items, its move motion, a dashed drop placeholder, a
/// resize grip in each resizable item's corner and a raised dragged item.
///
/// While the grid is editable — its items can be moved or resized — the
/// selected item carries a ring in the primary color, so an editor shows which
/// item its settings apply to. While the keyboard drives the grid the focus
/// ring takes its place, never both. A read-only grid marks no selection.
///
/// Every part can still be overridden on the returned builder — `.gap_4()` for
/// more room, say.
///
/// ```ignore
/// grid_layout("dashboard", &state, cx)
///     .item(GridItem::new("revenue").rounded(cx.theme().radius_lg).child(revenue_card))
/// ```
pub fn grid_layout(
    id: impl Into<ElementId>,
    state: &Entity<GridLayoutState>,
    cx: &App,
) -> GridLayout {
    let theme = cx.theme();
    let motion = theme.motion_tokens();
    let editable = {
        let state = state.read(cx);
        state.is_draggable() || state.is_resizable()
    };
    let selected = box_shadow(0., 0., 0., px(2.), theme.primary);
    let ring = if theme.focus_ring {
        box_shadow(
            0.,
            0.,
            0.,
            FOCUS_RING_WIDTH,
            theme.ring.alpha(FOCUS_RING_OPACITY),
        )
    } else {
        box_shadow(0., 0., 0., px(1.), theme.ring)
    };

    GridLayout::new(id, state)
        .gap(theme.spacing_tokens().md)
        .transition(Transition::new(motion.duration_normal).easing(motion.easing_move.clone()))
        .dragging_item(|style| style.shadow_lg())
        // Both rings are the item's shadow, so the focus ring replaces the
        // selection ring rather than drawing a second one around it.
        .when(editable, |grid| {
            grid.selected_item(move |style| style.shadow(vec![selected]))
        })
        .focused_item(move |style| style.shadow(vec![ring]))
        .placeholder(|_, cx| {
            let theme = cx.theme();
            div()
                .size_full()
                .rounded(theme.radius_lg)
                .bg(theme.primary.opacity(0.08))
                .border_1()
                .border_dashed()
                .border_color(theme.primary.opacity(0.5))
                .into_any_element()
        })
        .resize_handle(|_, cx| {
            let color = cx.theme().muted_foreground;
            // The corner mark sits a few pixels in from the item's corner, and
            // strengthens while the pointer is on the corner it marks.
            div()
                .group(GRIP_GROUP)
                .size_full()
                .child(
                    div()
                        .absolute()
                        .right(px(3.))
                        .bottom(px(3.))
                        .size(px(6.))
                        .border_r_2()
                        .border_b_2()
                        .border_color(color.opacity(0.35))
                        .group_hover(GRIP_GROUP, |this| this.border_color(color.opacity(0.9))),
                )
                .into_any_element()
        })
}
