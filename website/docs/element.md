---
title: Element
description: Understand GPUI's element tree and low-level rendering lifecycle.
order: -2.7
---

# Element

An **Element** is a node in the element tree that GPUI builds for one frame. Elements perform layout, prepare hit testing, and paint pixels into a Window. GPUI drops the tree and its frame-local callbacks before the next frame, then builds a new tree from the application's current state.

Most application code should compose the elements provided by GPUI and GPUI Kit:

```rust
div()
    .flex()
    .items_center()
    .gap_2()
    .child(Icon::new(IconName::Search))
    .child("Search")
```

This code creates an element tree. It does not implement GPUI's low-level `Element` trait.

## `Element`, `IntoElement`, and `AnyElement`

These types have different jobs:

| Type | Purpose |
| --- | --- |
| `Element` | Implements the low-level layout and painting lifecycle. |
| `IntoElement` | Converts a value into a concrete `Element`, allowing strings, components, Entities, and other values to be passed to `.child(...)`. |
| `AnyElement` | Erases the concrete element type, which is useful for heterogeneous collections, conditional branches, slots, and stored children. |

Keep the concrete type when every branch has the same type. Erase it only at a boundary that needs different element types:

```rust
fn status_icon(online: bool) -> AnyElement {
    if online {
        Icon::new(IconName::CircleCheck).into_any_element()
    } else {
        div().child("Offline").into_any_element()
    }
}
```

Zed and Longbridge Pro use `AnyElement` this way for optional slots, table cells, and functions whose branches return different UI types. `IntoElement` is the usual API boundary when the caller does not need type erasure.

## The three phases

GPUI drives an `Element` through three phases in order:

```text
request_layout → prepaint → paint
```

### `request_layout`

Register the element's `Style` and child layout nodes with `window.request_layout`. GPUI's Taffy layout engine resolves their sizes and positions after layout has been requested.

Return a `LayoutId` and any `RequestLayoutState` needed by the later phases. Do not assume the final `Bounds` are available here.

### `prepaint`

GPUI now provides the resolved `Bounds`. Use this phase to shape text, calculate geometry, insert hitboxes, prepaint children, and prepare data that `paint` needs.

Return that data as `PrepaintState`. Hit testing belongs here because GPUI must establish the current frame's spatial and dispatch information before painting.

### `paint`

Paint quads, text, paths, or images with the prepared state. Register frame-local input handlers when the custom element requires them. This phase should consume the geometry prepared earlier rather than repeat layout work.

State flows forward through the frame:

```text
RequestLayoutState ───────────────┐
        │                         │
        ▼                         ▼
     prepaint ── PrepaintState ──► paint
```

The associated state is frame-local. Persistent application state belongs in an [Entity](./entity), while small element state that must survive frames can be associated with an [ElementId](./element_id).

## When to implement `Element`

Implement `Element` when existing elements cannot express the work, for example:

- a code editor or text input that shapes text and paints selections and cursors;
- a chart or canvas with custom geometry;
- a custom layout algorithm;
- a performance-sensitive primitive that needs direct control over layout, hitboxes, and painting.

The GPUI text input example uses a custom `Element` because it must shape a line during `prepaint`, register an input handler, and paint its selection and cursor. GPUI's `Svg`, `Img`, lists, and canvas use the same lifecycle. In contrast, most Zed and Longbridge Pro UI is built by composing existing elements and converting differing branches to `AnyElement`.

For a reusable UI component, start with [`RenderOnce`](./render). For stateful UI owned by an Entity, use [`Render`](./render). Drop down to `Element` only when you need to control the rendering pipeline itself.

## Identity and interactivity

These concepts sit on separate boundaries:

- [`ElementId`](./element_id) identifies an element within its keyed scope. GPUI uses its global form to connect state and retained work across frames.
- Calling `.id(...)` on an `InteractiveElement` returns `Stateful<E>`. That wrapper enables APIs whose state must be associated with a stable element identity.
- `InteractiveElement` exposes GPUI's standard interaction machinery, including hitboxes, mouse listeners, Focus tracking, Key Context, and Action handlers. A custom `Element` does not gain those behaviors automatically.

When an existing interactive element such as `div()` already provides the behavior you need, compose it. If a custom primitive needs standard interaction, embed or delegate to GPUI's `Interactivity`, as GPUI's built-in elements do. Implementing raw hitboxes and event registration yourself also makes you responsible for dispatch, clipping, cursor behavior, and accessibility.

:::info
`Element::id()` returning an `ElementId` does more than label pixels: it creates stable identity across frames. Keep IDs unique within their nearest keyed ancestor, and do not add an ID unless the element or an attached behavior needs identity.
:::

[Element]: https://docs.rs/gpui/latest/gpui/trait.Element.html
[IntoElement]: https://docs.rs/gpui/latest/gpui/trait.IntoElement.html
[AnyElement]: https://docs.rs/gpui/latest/gpui/struct.AnyElement.html
