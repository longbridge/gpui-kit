---
title: ElementId
description: To introduce the ElementId concept in GPUI.
order: -2.4
---

# ElementId

The [ElementId] is a unique identifier for a GPUI element. It is used to reference elements in the GPUI component tree.

Before you start using GPUI and GPUI Component, you need to understand the [ElementId].

For example:

```rs
div().id("my-element").child("Hello, World!")
```

In this case, the `div` element has an `id` of `"my-element"`. The add `id` is used for GPUI for binding events, for example `on_click` or `on_mouse_move`, the `element` with `id` in GPUI we call [Stateful\<E\>].

We also use `id` (actually, it uses [GlobalElementId] internally in GPUI) to manage the `state` in some elements, by using `window.use_keyed_state`, so it is important to keep the `id` unique.

## Unique

The `id` should be unique within the layout scope (In a same [Stateful\<E\>] parent).

For example we have a list with multiple items:

```rs
div().id("app").child(
    div().id("list1").child(vec![
        div().id(1).child("Item 1"),
        div().id(2).child("Item 2"),
        div().id(3).child("Item 3"),
    ])
).child(
    div().id("list2").child(vec![
        div().id(1).child("Item 1"),
    ])
)
```

In this case, we can named the child items with a very simple id, because they are have a parent `list1` element with an `id`.

GPUI internal will generate [GlobalElementId] with the parent elements's `id`, in this example, the `Item 1` will have global_id:

```rs
["app", "list1", 1]
```

And the `Item 1` in `list2` will have global_id:

```rs
["app", "list2", 1]
```

So we can named the child items with a very simple id.

[ElementId]: https://docs.rs/gpui/latest/gpui/enum.ElementId.html
[GlobalElementId]: https://docs.rs/gpui/latest/gpui/struct.GlobalElementId.html
[Stateful]: https://docs.rs/gpui/latest/gpui/struct.Stateful.html
[Stateful\<E\>]: https://docs.rs/gpui/latest/gpui/struct.Stateful.html
