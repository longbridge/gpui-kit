---
title: GridLayout
description: A grid whose items the user drags and resizes into place.
---

# GridLayout

GridLayout arranges items on a grid of columns and rows and lets the user move and resize them with the pointer or the keyboard. Items push one another out of the way, float toward the top as space opens up, and flow around locked items. Use it for dashboards, widget boards, and other surfaces where the user owns the arrangement.

The behavior and layout model are the [Grid Layout](../base/primitives/grid-layout.md) primitive from `gpui-base`; this module re-exports it and adds `grid_layout`, which dresses it in the theme.

## Import

```rust
use gpui_kit::component::grid_layout::{
    GridArea, GridItem, GridLayoutEvent, GridLayoutState, grid_drag_handle, grid_layout,
};
```

## Usage

Create one `GridLayoutState` with the grid's rules and its areas, then pair each `GridItem` with the area of the same id.

```rust
let state = cx.new(|cx| {
    GridLayoutState::new(cx).default_areas([
        GridArea::new("revenue").col_span(8).row_span(4),
        GridArea::new("orders").col(8).col_span(4).row_span(2),
        GridArea::new("visitors").col(8).row(2).col_span(4).row_span(2),
    ])
});

grid_layout("dashboard", &state, cx)
    .item(GridItem::new("revenue").child(revenue_card))
    .item(GridItem::new("orders").child(orders_card))
    .item(GridItem::new("visitors").child(visitors_card))
```

A `GridArea` places an item in zero-based grid cells: `col` is the leading column, `row` the top row, and `col_span` and `row_span` the number of columns and rows it covers. An item without an area is not drawn.

The grid is as wide as its container and as tall as its rows. Place it inside a scrollable region when the rows can outgrow the window.

## Grid rules

Set the rules before the areas, because `default_areas` settles the areas under the rules set so far.

```rust
use gpui_kit::Axis;

let state = cx.new(|cx| {
    GridLayoutState::new(cx)
        .cols(6)
        .compaction(Some(Axis::Vertical))
        .default_areas(areas)
});
```

| Rule | Default | Description |
| --- | --- | --- |
| `cols` | `12` | The number of equal columns the width is divided into |
| `compaction` | `Some(Axis::Vertical)` | The direction items float toward, or `None` to leave them where they are put |
| `prevent_collision` | `false` | Refuse a move or resize that would overlap another item instead of pushing it |
| `draggable` | `true` | Whether items can be moved |
| `resizable` | `true` | Whether items can be resized |

Each rule has a `set_*` counterpart for changing it later, such as `set_compaction`, or `set_draggable` to switch a dashboard between viewing and editing.

The row height and the gaps belong to the element, as on a CSS grid:

```rust
grid_layout("dashboard", &state, cx)
    .row_height(rems(3.))
    .gap_4()
```

`grid_layout` uses the theme's medium spacing; `.gap(..)`, `.gap_x(..)` or `.gap_y(..)` override it. Rows default to `2.5rem`, so they follow the application's zoom.

## Item limits

Limit how far an item can be resized, or lock it in place. A locked item is neither moved, resized, nor pushed; other items flow around it.

```rust
GridArea::new("revenue").col_span(6).row_span(4).min_span(3, 2).max_span(8, 6);

GridArea::new("notes").col(8).col_span(4).row_span(3).locked(true);
```

## Saving the layout

`GridArea` implements `Serialize` and `Deserialize`. Subscribe to `GridLayoutEvent::Change` to persist the arrangement whenever the user changes it, and restore it with `default_areas` or `set_areas`.

```rust
cx.subscribe(&state, |this, _, event: &GridLayoutEvent, cx| {
    if let GridLayoutEvent::Change(areas) = event {
        this.save_dashboard_layout(areas, cx);
    }
})
.detach();
```

`Change` is emitted when a drag or resize ends, when a keyboard command changes the layout, and after `move_item` or `resize_item`. A drag reports one change on release, not one per pointer movement. `set_areas` does not emit `Change`, because the application already holds the layout it set. `GridLayoutEvent` is `#[non_exhaustive]`, so match it with `if let` or a wildcard arm.

## Following the selection

`GridLayoutEvent::Select` reports the item the user selects — by pressing it, or with Tab or Shift-Tab — once per change. A settings panel can follow it without watching every redraw:

```rust
cx.subscribe(&state, |this, _, event: &GridLayoutEvent, cx| {
    if let GridLayoutEvent::Select(id) = event {
        this.inspected_tile = id.clone();
        cx.notify();
    }
})
.detach();
```

`set_selected_item` changes the selection without emitting it.

## Narrow windows

`reflow(cols)` shows the items over fewer columns without changing the stored layout — a one-column stack below a breakpoint, say:

```rust
grid_layout("dashboard", &state, cx)
    .when(window.viewport_size().width < px(768.), |grid| grid.reflow(1))
```

While reflowed the grid is read-only, so nothing is saved at the narrower width; selection still works.

## Drag handle

By default the whole item moves it. When an item's content has its own pointer interaction, such as a table or a chart, move the item only by a handle, usually its header:

```rust
GridItem::new("orders")
    .drag_handle_only(true)
    .child(
        v_flex()
            .size_full()
            .child(grid_drag_handle(&state, "orders").p_2().child("Orders"))
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

`grid_layout` adds a dashed placeholder where the dragged item will land, a resize grip in each resizable item's corner, a raised dragged item, and the theme's move motion, which follows the application's reduced-motion preference. While the grid is editable, the selected item carries a ring in the primary color, so an editor shows which item its settings apply to; while the keyboard drives the grid the focus ring takes its place, never both. A read-only grid marks no selection. Each can be replaced on the returned builder with `placeholder`, `resize_handle`, `dragging_item`, `selected_item`, `focused_item` or `transition`.

A `GridItem` paints nothing of its own: its content supplies the surface, usually a card or a GroupBox. Give the item the same corner radius as its content so that the raised shadow and the rings follow the same corners.

```rust
GridItem::new("orders")
    .rounded(cx.theme().radius_lg)
    .child(orders_card)
```

## Keyboard

Clicking an item selects it and focuses the grid. With the grid focused:

| Key | Action |
| --- | --- |
| `Tab` / `Shift-Tab` | Select the next or previous item in reading order; past either end, focus leaves the grid |
| Arrow keys | Move the selected item one cell |
| `Shift` + arrow keys | Resize the selected item by one cell |
| `Escape` | Cancel the drag in progress |

When a command cannot apply, such as moving past the leading edge, the key passes to the surrounding view.

## Accessibility

The grid is a labelled group, "Grid layout" by default; use `accessibility_label` on the grid to name what it contains. Each item is a group that reports whether it is selected. Name items with `GridItem::accessibility_label`, usually their visible title.
