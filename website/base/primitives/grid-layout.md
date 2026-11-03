---
title: Grid Layout
description: A grid whose items the user drags and resizes, with the layout model and its arithmetic.
order: 12.5
---

# Grid Layout

A grid whose items the user drags and resizes into place, the way dashboards and widget boards are arranged. Items push one another aside, float toward the top as space opens up, and flow around locked items.

Like every `gpui-base` primitive, Grid Layout supplies behavior and the geometry required to implement it — where each item sits, the drag, the resize, the keyboard model, and the layout arithmetic — without imposing a visual language. The drop placeholder, the resize grip, the dragged and selected item styles, and the motion items settle with all come from the application. GPUI Component's [GridLayout](../../component/grid-layout.md) supplies them from the theme.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- grid-layout
```

## Import

```rust
use gpui_kit::base::{GridArea, GridItem, GridLayout, GridLayoutEvent, GridLayoutState, grid_drag_handle};
```

## Anatomy and API

- `GridLayoutState` owns the layout and the rules it settles under: `cols(n)`, `compaction(Option<Axis>)`, `prevent_collision(bool)`, `draggable(bool)`, `resizable(bool)`, and the starting `default_areas(...)`.
- `GridArea` is one item's place, in zero-based cells: `col`, `row`, `col_span`, `row_span`, the resize limits `min_span` / `max_span`, and `locked`. It is the record an application persists.
- `GridLayout` is the element. It places each `GridItem` in the area with the same id. Set the row height with `row_height(rems(..))` and the gaps with GPUI's own `gap`, `gap_x` and `gap_y`, as on a CSS grid.
- `GridItem` is one item; it implements `Styled` and `ParentElement`.
- `grid_drag_handle(&state, id)` returns the element an item built with `drag_handle_only(true)` is moved by.

```rust
let state = cx.new(|cx| {
    GridLayoutState::new(cx).cols(12).default_areas([
        GridArea::new("revenue").col_span(8).row_span(4).min_span(4, 2),
        GridArea::new("orders").col(8).col_span(4).row_span(2),
        GridArea::new("notes").col(8).row(2).col_span(4).row_span(2).locked(true),
    ])
});

GridLayout::new("dashboard", &state)
    .row_height(rems(2.5))
    .gap_3()
    .item(GridItem::new("revenue").child(revenue_card))
    .item(GridItem::new("orders").child(orders_card))
    .item(GridItem::new("notes").child(notes_card))
```

`col` is a cell index counted from zero, not a one-based CSS grid line as in GPUI's `col_start`, so a layout reads the same in code and in its saved form.

### Presentation

Grid Layout paints nothing by default. Give it a look with:

- `dragging_item(|style| ..)` and `selected_item(|style| ..)` — styles layered over the item under the pointer and over the keyboard-selected item;
- `placeholder(|window, cx| ..)` — what fills the cells the dragged item will land in;
- `resize_handle(|window, cx| ..)` — what a resizable item shows in the trailing-bottom corner it is resized by;
- `transition(Transition)` — how items ease into their cells. Without one, items move at once.

## State and events

Subscribe to `GridLayoutEvent::Change` to persist what the user arranges. It carries the whole layout and is emitted once per committed change — when a drag or resize is released, after a keyboard command, and after `move_item` or `resize_item` — not on every pointer movement. `set_areas` replaces the layout without emitting it, because the application already holds the layout it set. `GridLayoutEvent` is `#[non_exhaustive]`, so match it with a wildcard arm.

`GridArea` implements `Serialize` and `Deserialize`. Every field after the place has a default, so a layout saved with only `id`, `col`, `row`, `col_span` and `row_span` still loads.

Set the state's rules before `default_areas`, because the areas are settled under the rules set so far.

## Keyboard

Clicking an item selects it and focuses the grid. With the grid focused, the arrow keys move the selected item one cell, Shift with the arrow keys resizes it, Tab and Shift-Tab move the selection, and Escape cancels a drag. Past the last or before the first item, Tab is not handled, so focus moves on out of the grid. The keys are bound to the `MoveLeft`, `MoveRight`, `MoveUp`, `MoveDown`, `ShrinkWidth`, `GrowWidth`, `ShrinkHeight`, `GrowHeight`, `SelectNextItem` and `SelectPreviousItem` actions in the `GridLayout` key context.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/grid_layout.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

The grid is a labelled group, "Grid layout" by default; name what it contains with `GridLayout::accessibility_label`. Each item is a group that reports whether it is selected; name it with `GridItem::accessibility_label`, usually its visible title.

## Notes

Give each item a stable id that does not change when items are reordered. The grid is as wide as its container and as tall as its rows, so put it in a scrollable region when the rows can outgrow the window.
