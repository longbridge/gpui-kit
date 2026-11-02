//! A grid whose items the user drags and resizes into place.
//!
//! [`GridLayoutState`] owns the layout — one [`GridItem`] per item, in grid
//! cells — and the rules it settles under; [`GridLayout`] draws the items and
//! turns pointer and keyboard input into layout changes.

mod grid_layout;
mod layout;
mod state;

pub use grid_layout::{GridDragHandle, GridLayout, GridLayoutItem};
pub use layout::GridItem;
pub use state::{GridLayoutEvent, GridLayoutState};

use gpui::{App, KeyBinding, actions};

use crate::actions::Cancel;

pub(super) const CONTEXT: &str = "GridLayout";

actions!(
    grid_layout,
    [
        /// Move the selected item one column toward the leading edge.
        MoveLeft,
        /// Move the selected item one column toward the trailing edge.
        MoveRight,
        /// Move the selected item up one row.
        MoveUp,
        /// Move the selected item down one row.
        MoveDown,
        /// Make the selected item one column narrower.
        ShrinkWidth,
        /// Make the selected item one column wider.
        GrowWidth,
        /// Make the selected item one row shorter.
        ShrinkHeight,
        /// Make the selected item one row taller.
        GrowHeight,
        /// Select the next item in reading order.
        SelectNextItem,
        /// Select the previous item in reading order.
        SelectPreviousItem,
    ]
);

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", MoveLeft, Some(CONTEXT)),
        KeyBinding::new("right", MoveRight, Some(CONTEXT)),
        KeyBinding::new("up", MoveUp, Some(CONTEXT)),
        KeyBinding::new("down", MoveDown, Some(CONTEXT)),
        KeyBinding::new("shift-left", ShrinkWidth, Some(CONTEXT)),
        KeyBinding::new("shift-right", GrowWidth, Some(CONTEXT)),
        KeyBinding::new("shift-up", ShrinkHeight, Some(CONTEXT)),
        KeyBinding::new("shift-down", GrowHeight, Some(CONTEXT)),
        KeyBinding::new("tab", SelectNextItem, Some(CONTEXT)),
        KeyBinding::new("shift-tab", SelectPreviousItem, Some(CONTEXT)),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
    ]);
}
