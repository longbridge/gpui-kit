---
title: VirtualList
description: Render a hundred thousand differently sized rows by drawing only the ones on screen.
order: 5
example: virtual-list
exampleKind: base
---

# Virtual List

Render a list of any length by drawing only the items currently on screen. Unlike `gpui_kit::uniform_list`, **each item may have a different size** — which is what makes it usable for tables with variable row heights, chat transcripts, and outline trees.

Virtual List is infrastructure rather than a component: it has no appearance of its own, contributes no chrome, and imposes nothing on the items you return. You give it the sizes up front and a closure that renders a range.

## Why sizes up front

Virtualization needs to know the total extent of the list and which items intersect the viewport **without rendering anything**. Two designs solve this:

| Approach                 | How                                                        | Cost                                                                          |
| ------------------------ | ---------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `gpui_kit::uniform_list` | Every item is the same size, so offsets are multiplication | No per-item data, but no variable sizes                                       |
| Measure as you scroll    | Render, measure, correct                                   | Scrollbar jumps; scroll position drifts                                       |
| **`VirtualList`**        | You supply every item's size                               | Exact offsets and a stable scrollbar, at the cost of knowing sizes in advance |

The third is why `item_sizes` is a required argument rather than a callback. If your items are genuinely unmeasurable until drawn, compute a good estimate, or use a fixed row height and let content clip.

## Get started

```rust
use std::rc::Rc;
use gpui_kit::base::{v_virtual_list, VirtualListScrollHandle};
use gpui_kit::{px, size};

let sizes = Rc::new(vec![size(px(280.), px(32.)); 100_000]);

v_virtual_list(
    cx.entity(),
    "customers",
    sizes,
    |_this, range, _window, _cx| {
        range
            .map(|ix| div().h_8().px_2().child(format!("Customer {ix}")))
            .collect()
    },
)
.track_scroll(&self.scroll_handle)
.size_full()
```

The closure is handed a `Range<usize>` — only the visible slice, plus a small overdraw — and returns one element per index in that range. It receives `&mut V` for the entity you passed, so it can read your data without cloning it into the closure.

Use `h_virtual_list` for a horizontal list; everything else is identical.

## The size contract

Three rules, and breaking any of them shows up as misplaced items rather than a panic.

**Only one dimension is read.** A vertical list uses each `Size`'s `height` and ignores its `width`; a horizontal list uses `width` and ignores `height`. The value you pass for the unused axis is free.

**The cross axis is measured, not declared.** A vertical list gets its width by laying out one item and measuring it — by default item 0. If your first item is not representative (an unusually short label, say), point it at one that is:

```rust
v_virtual_list(entity, "rows", sizes, render)
    .with_item_to_measure_index(3)
```

**`item_sizes.len()` is the item count.** The list renders exactly that many items; there is no separate count argument. If the vector disagrees with your data, the extra indices are still requested from your closure.

`Rc<Vec<Size<Pixels>>>` is shared rather than owned so that rebuilding the element every frame does not reallocate the size table. Keep the `Rc` in your entity and clone the handle, rather than constructing the vector inside `render`.

## Scrolling

`VirtualListScrollHandle` owns the scroll position and survives re-renders, so it belongs in your entity, not in `render`.

```rust
struct CustomerList {
    scroll: VirtualListScrollHandle,
}

// Jump to an item
self.scroll.scroll_to_item(4_200, ScrollStrategy::Top);

// Follow a growing list
self.scroll.scroll_to_bottom();
```

`scroll_to_item` takes a `ScrollStrategy` — `Top`, `Center`, or `Bottom` — and works on indices that have never been rendered, because the offset comes from the size table rather than from measurement. `base_handle()` exposes the underlying GPUI `ScrollHandle` when you need it.

Attach the handle with `.track_scroll(&handle)`.

## With a scrollbar

`VirtualListScrollHandle` implements `ScrollbarHandle`, so the base `Scrollbar` reads it directly. The list draws no scrollbar of its own.

```rust
div()
    .relative()
    .child(
        v_virtual_list(cx.entity(), "rows", sizes, render)
            .track_scroll(&self.scroll)
            .size_full(),
    )
    .child(Scrollbar::vertical(&self.scroll))
```

The container needs `relative()` because the scrollbar positions itself against it.

## Sizing behavior

`with_sizing_behavior` controls whether the list computes a size of its own:

- `ListSizingBehavior::Auto` (default) — the list does not calculate a fixed size, and takes the space its parent gives it.
- `ListSizingBehavior::Infer` — the list calculates its size from its items.

The default plus a bounded parent is what almost every layout wants. A virtual list inside an unbounded parent has nothing to virtualize against, so it would try to lay out every item.

## The render closure

The closure runs on **every frame that the visible range changes**, so treat it as a hot path:

- Do no I/O, sorting, or filtering inside it. Keep the prepared data in your entity and index into it.
- Return elements, not entities. Creating a GPUI entity per row defeats virtualization — the entity outlives the frame, so a hundred thousand rows would create a hundred thousand entities.
- Element ids, if you set them, should derive from the item index or a stable key, not from position within the returned vector.

State lives outside the closure. Update it in your own callbacks and call `cx.notify()`; do not mutate it during render.

## What it costs

Work per frame is proportional to the number of **visible** items, not total items — the example on this page holds 100,000 rows and draws about a dozen. The size table is the one part that scales with the total: it is one `Size<Pixels>` per item, held once behind an `Rc`.

The tradeoff is that the size table must exist before the first frame. For a million rows of uniform height that is a megabyte of sizes for information a single number could carry — that is the case `gpui_kit::uniform_list` exists for, and it is the better choice there.

## Complete Rust example

```bash
cargo run -p gpui-base-examples -- virtual-list
```

<<< ../../crates/base/examples/showcase/components/virtual_list.rs{rust}

## Checklist

- Hold `item_sizes` and the scroll handle in your entity; rebuild neither during render.
- Keep `item_sizes.len()` equal to your data length.
- Point `with_item_to_measure_index` at a representative item if item 0 is not one.
- Give the list a bounded parent, and `relative()` on that parent if you add a scrollbar.
- Preserve logical order, item counts, and stable identity so assistive technology sees a coherent list across virtualization.
