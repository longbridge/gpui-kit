---
title: ElementId
description: Give GPUI elements stable identity and understand how keyed state survives frames.
order: -2.4
---

# ElementId

An `ElementId` is a **local key** for an element in GPUI's rendered tree. GPUI combines that key with the IDs of its keyed ancestors to form a `GlobalElementId`. This path lets GPUI associate interaction and element state with the same logical element when a View renders again. It also preserves node identity in the [accessibility tree](./accessibility) when the element has a role.

An ID is not a handle to an [Entity] and is not a way to look up an element like an HTML DOM ID. Use an `Entity<T>` for shared application state. Use an `ElementId` for identity in the element tree, including keyed component state, focus or scroll behavior supplied by a component, and state retained by a custom [Element].

## Assign an ID

Calling `.id(...)` on an interactive element such as `div()` returns a `Stateful<Div>`:

```rust
let save = div()
    .id("save-button")
    .on_click(|_, window, cx| {
        // Handle the click.
    })
    .child("Save");
```

The `Stateful<E>` wrapper exposes GPUI's stateful interaction methods and carries the element's ID. A custom `Element` can return `Some(id)` from its `id()` method without using this wrapper. A plain, unkeyed `div()` can still be a layout parent; it contributes no segment to the ID path.

Strings, integers, and name plus integer tuples are common `ElementId` inputs:

```rust
use gpui_kit::component::button::Button;

div().id("search")
div().id(("message", message.id))
Button::new(("delete-project", project.id)).label("Delete")
```

Choose a key from the object's identity, not from the text displayed to the user. A translated label, current selection, or freshly generated random value can change while the object stays the same.

## GlobalElementId and the keyed ancestor path

GPUI builds a `GlobalElementId` from the IDs on the path to an element. Only keyed ancestors add path segments:

```text
div().id("workspace")
├── div().id("inbox")
│   └── div().id(("row", 42))  → ["workspace", "inbox", ("row", 42)]
└── div().id("archive")
    └── div().id(("row", 42))  → ["workspace", "archive", ("row", 42)]
```

The two rows may share a local ID because their keyed ancestor paths differ. This diagram shows only IDs written in the example: an entity-backed View also adds its `EntityId` as a path segment, and a `RenderOnce` component adds a type-name namespace. The path is scoped to the Window's rendered tree; `GlobalElementId` is GPUI's internal path, not a process-wide string you need to construct at call sites. For a custom drawing API that needs a path for its own key, `window.with_global_id(key, |global_id, window| { … })` creates one within that callback.

The practical uniqueness rule is: **within the same nearest keyed ancestor, each keyed descendant branch needs a distinct ID**. An unkeyed container does not open a new namespace:

```rust
div().id("workspace")
    .child(div().child(div().id("item")))
    .child(div().child(div().id("item"))) // Same keyed path: collision.
```

Give the branches their own stable IDs, or make the item IDs distinct. Duplicate paths can make retained state and interaction attach to the wrong logical element.

## Stable keys in changing lists

For a list that can insert, remove, filter, or reorder rows, derive each row ID from a stable domain value:

```rust
div().id("messages").children(messages.iter().map(|message| {
    div()
        .id(("message", message.id))
        .child(message.preview.clone())
}))
```

`("message", message.id)` separates the row's purpose from other controls that may use the same numeric ID. Reordering changes the drawing position, but each message keeps its keyed path. By contrast, `.id(index)` attaches state to a *position*: after an insertion, the old first row's focus, scroll, animation, or other keyed state may be reused for a different message. An index is appropriate only when the position itself is the identity and cannot shift.

If a repeated row contains several controls, key the row and give its children distinct local keys such as `"edit"` and `"delete"`. Moving the row then carries its whole keyed subtree with it.

## How IDs retain state

GPUI reconstructs elements during rendering. The Rust value returned by `div()` or a `RenderOnce` component is temporary; assigning it an ID does not turn it into a persistent `Entity<T>`. The ID gives GPUI a way to reconnect element-local state across consecutive frames.

`window.use_keyed_state(key, cx, init)` forms a path from the current keyed ancestors plus `key`. It returns an `Entity<S>` kept for as long as that keyed state is used in consecutive rendered frames. `Window` owns this keyed state; `cx` supplies application access (see [Context](./context) for the different GPUI contexts). `init` runs when that state has no previous entry. GPUI also observes this state entity and notifies the current View when it changes:

```rust
let focus_handle = window
    .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
    .read(cx)
    .clone();
```

GPUI Kit's `Button` uses this pattern for its focus handle. Its public ID gives the recreated button instance the same state key on later frames. The focus handle still owns focus behavior; the element ID identifies where that handle's state belongs. The window retains keyed state only while its path is accessed in successive frames; a [cached View](./view-cache) replays those accesses when it reuses its subtree. A separate strong `Entity<S>` handle can keep that entity alive after its window state entry disappears, but recreating the path will run `init` again.

For a custom `Element`, GPUI passes `Option<&GlobalElementId>` into `request_layout`, `prepaint`, and `paint` when its `id()` returns a value. During drawing, `window.with_element_state(global_id, ...)` can read state from the preceding frame and return the value to store for the next:

```rust
let state = window.with_element_state(
    id.expect("this element always has an ID"),
    |previous: Option<AnimationState>, _window| {
        let state = previous.unwrap_or_default();
        (state.clone(), state)
    },
);
```

This is the pattern behind GPUI Kit's `ScrollBounce` element, which retains motion state during `prepaint`. `with_element_state` is a drawing-phase API for element authors; ordinary Views should prefer Entity state or a component's documented API. GPUI keys stored element state by global path **and state type** and drops it when the element no longer participates in the rendered frames.

:::info
`window.use_state(cx, init)` uses the call site's code location as its local key. It works when the *full path* is unique: a call inside each row is safe if every row has a stable keyed ancestor. If repeated calls share the same keyed ancestor path, use `use_keyed_state` with a stable item key or introduce a keyed namespace around each item.
:::

## Identity changes are state changes

- Changing an element's ID or a keyed ancestor's ID gives it a new path and resets its associated element state.
- Removing an element ends its consecutive-frame state lifetime. Recreating it later initializes that state again.
- An ID does not preserve a View's `Entity<T>` by itself; a strong Entity owner controls that lifetime.
- Keyed state belongs to the Window's rendering context. Do not rely on a `GlobalElementId` to transfer state between windows.

See [Element](./element) for the layout, prepaint, and paint lifecycle, and [Entity](./entity) for state that must outlive an element's presence in the tree.

[Element]: /docs/element
[Entity]: /docs/entity
