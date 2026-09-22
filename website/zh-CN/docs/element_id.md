---
title: ElementId
description: 介绍 GPUI 中的 ElementId 概念。
order: -2.4
---

# ElementId

[ElementId] 是 GPUI 元素的唯一标识符，用于在 GPUI 组件树中引用具体元素。

在开始使用 GPUI 和 GPUI Component 之前，最好先理解 [ElementId] 的作用。

例如：

```rs
div().id("my-element").child("Hello, World!")
```

在这个例子里，`div` 元素的 `id` 是 `"my-element"`。为元素添加 `id` 后，GPUI 才能将事件绑定到它上面，例如 `on_click` 或 `on_mouse_move`。在 GPUI 中，带有 `id` 的元素通常称为 [Stateful\<E\>]。

我们也会在某些组件内部使用 `id` 来管理状态。实际上 GPUI 会在内部使用 [GlobalElementId]，并通过 `window.use_keyed_state` 这类机制保存状态，因此 `id` 保持唯一非常重要。

## 唯一性

`id` 需要在当前布局作用域内唯一，也就是在同一个 [Stateful\<E\>] 父节点下不能重复。

例如下面这个包含多个列表项的结构：

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

这里子项可以使用很简单的 `id`，因为它们已经处于带有 `id` 的父元素之下。

GPUI 内部会结合父元素的 `id` 自动生成 [GlobalElementId]。在这个例子中，`list1` 里的 `Item 1` 对应的 `global_id` 是：

```rs
["app", "list1", 1]
```

而 `list2` 里的 `Item 1` 对应的 `global_id` 是：

```rs
["app", "list2", 1]
```

因此，只要父级路径不同，子元素就可以复用较简单的局部 `id`。

[ElementId]: https://docs.rs/gpui/latest/gpui/enum.ElementId.html
[GlobalElementId]: https://docs.rs/gpui/latest/gpui/struct.GlobalElementId.html
[Stateful]: https://docs.rs/gpui/latest/gpui/struct.Stateful.html
[Stateful\<E\>]: https://docs.rs/gpui/latest/gpui/struct.Stateful.html
