---
title: GridLayout
description: A grid whose items the user drags and resizes into place.
---

# GridLayout

GridLayout arranges items on a grid of columns and rows and lets the user move and resize them with the pointer or the keyboard. Items push one another out of the way, float toward the top as space opens up, and flow around locked items. Use it for dashboards, widget boards, and other surfaces where the user owns the arrangement.

## Import

```rust
use gpui_kit::component::grid_layout::{
    GridDragHandle, GridItem, GridLayout, GridLayoutEvent, GridLayoutItem, GridLayoutState,
};
```

## Usage

Create one `GridLayoutState` with the grid's rules and its items, then pair each `GridLayoutItem` with the layout entry of the same id.

```rust
let state = cx.new(|cx| {
    GridLayoutState::new(cx).with_items([
        GridItem::new("revenue", 0, 0, 8, 4),
        GridItem::new("orders", 8, 0, 4, 2),
        GridItem::new("visitors", 8, 2, 4, 2),
    ])
});

GridLayout::new("dashboard", &state)
    .item(GridLayoutItem::new("revenue").child(revenue_card))
    .item(GridLayoutItem::new("orders").child(orders_card))
    .item(GridLayoutItem::new("visitors").child(visitors_card))
```

A `GridItem` places an item in grid cells: `x` is the leading column, `y` the top row, and `w` and `h` the number of columns and rows it spans. An item without a layout entry is not drawn.

The grid is as wide as its container and as tall as its rows. Place it inside a scrollable region when the rows can outgrow the window.

## Grid rules

Set the rules before the items, because `with_items` settles the items under the rules set so far.

```rust
use gpui_kit::{Axis, px, rems};

let state = cx.new(|cx| {
    GridLayoutState::new(cx)
        .with_columns(6)
        .with_row_height(rems(3.))
        .with_gap(px(8.))
        .with_compaction(Some(Axis::Vertical))
        .with_items(items)
});
```

| Rule | Default | Description |
| --- | --- | --- |
| `with_columns` | `12` | The number of equal columns the width is divided into |
| `with_row_height` | `2.5rem` | The height of one row; follows the application's zoom |
| `with_gap` | theme spacing `md` | The space between neighbouring items |
| `with_compaction` | `Some(Axis::Vertical)` | The direction items float toward, or `None` to leave them where they are put |
| `with_prevent_collision` | `false` | Refuse a move or resize that would overlap another item instead of pushing it |
| `with_draggable` | `true` | Whether items can be moved |
| `with_resizable` | `true` | Whether items can be resized |

Each rule has a `set_*` counterpart for changing it later, such as `set_compaction` or `set_draggable` to switch a dashboard between viewing and editing.

## Item limits

Limit how far an item can be resized, or lock it in place. A locked item is neither moved, resized, nor pushed; other items flow around it.

```rust
GridItem::new("revenue", 0, 0, 6, 4)
    .with_min_size(3, 2)
    .with_max_size(8, 6);

GridItem::new("notes", 8, 0, 4, 3).with_locked(true);
```

## Saving the layout

`GridItem` implements `Serialize` and `Deserialize`. Subscribe to `GridLayoutEvent::Change` to persist the arrangement whenever the user changes it, and restore it with `with_items` or `set_items`.

```rust
cx.subscribe(&state, |this, _, event: &GridLayoutEvent, cx| {
    let GridLayoutEvent::Change(items) = event;
    this.save_dashboard_layout(items, cx);
})
.detach();
```

`Change` is emitted when a drag or resize ends, when a keyboard command changes the layout, and after `move_item` or `resize_item`. A drag reports one change on release, not one per pointer movement. `set_items` does not emit `Change`, because the application already holds the layout it set.

## Drag handle

By default the whole item moves it. When an item's content has its own pointer interaction, such as a table or a chart, move the item only by a handle, usually its header:

```rust
GridLayoutItem::new("orders")
    .drag_handle_only(true)
    .child(
        v_flex()
            .size_full()
            .child(GridDragHandle::new(&state, "orders").p_2().child("Orders"))
            .child(orders_table),
    )
```

## Programmatic changes

```rust
state.update(cx, |state, cx| {
    state.move_item("orders", 0, 4, cx);
    state.resize_item("revenue", 6, 3, cx);
});
```

Both apply the same pushing and compaction as the pointer and return whether the layout changed.

## Appearance

A `GridLayoutItem` paints nothing of its own: its content supplies the surface, usually a card or a GroupBox. Give the item the same corner radius as its content so that the drag placeholder and the focus ring follow the same corners.

```rust
GridLayoutItem::new("orders")
    .rounded(cx.theme().radius_lg)
    .child(orders_card)
```

While an item is dragged it follows the pointer above the others, and a dashed placeholder shows where it will land. The other items ease into their new cells; the motion follows the application's reduced-motion preference.

## Keyboard

Clicking an item selects it and focuses the grid. With the grid focused:

| Key | Action |
| --- | --- |
| `Tab` / `Shift-Tab` | Select the next or previous item in reading order |
| Arrow keys | Move the selected item one cell |
| `Shift` + arrow keys | Resize the selected item by one cell |
| `Escape` | Cancel the drag in progress |

When a command cannot apply, such as moving past the leading edge, the key passes to the surrounding view.

## Accessibility

The grid is a labelled group, "Grid layout" by default; use `GridLayout::accessibility_label` to name what it contains. Each item is a group that reports whether it is selected. Name items with `GridLayoutItem::accessibility_label`, usually their visible title.
