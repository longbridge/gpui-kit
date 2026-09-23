---
title: Element
description: 了解 GPUI 的 Element 树与底层渲染生命周期。
order: -2.7
---

# Element

**Element** 是 GPUI 为当前一帧构建的 Element 树节点。Element 负责布局、准备命中测试，并把像素绘制到 Window。下一帧开始前，GPUI 会释放整棵 Element 树及其中注册的帧级 callback，再根据应用的最新状态重新构建。

大多数应用代码只需要组合 GPUI 和 GPUI Kit 提供的 Element：

```rust
div()
    .flex()
    .items_center()
    .gap_2()
    .child(Icon::new(IconName::Search))
    .child("搜索")
```

这段代码构建了一棵 Element 树，并没有实现 GPUI 底层的 `Element` trait。

## `Element`、`IntoElement` 与 `AnyElement`

它们承担不同的职责：

| 类型 | 作用 |
| --- | --- |
| `Element` | 实现底层的布局与绘制生命周期。 |
| `IntoElement` | 把值转换为具体的 `Element`，因此字符串、组件、Entity 等值都可以传给 `.child(...)`。 |
| `AnyElement` | 擦除具体 Element 类型，适合异构集合、不同类型的条件分支、slot 和需要存储的子元素。 |

如果所有分支的类型相同，保留具体类型即可。只有边界需要容纳不同 Element 类型时才做类型擦除：

```rust
fn status_icon(online: bool) -> AnyElement {
    if online {
        Icon::new(IconName::CircleCheck).into_any_element()
    } else {
        div().child("离线").into_any_element()
    }
}
```

Zed 和 Longbridge Pro 会在可选 slot、表格单元格以及返回不同 UI 类型的函数中这样使用 `AnyElement`。如果调用方不需要类型擦除，通常用 `IntoElement` 作为 API 边界。

## 三个阶段

GPUI 会依次执行 `Element` 的三个阶段：

```text
request_layout → prepaint → paint
```

### `request_layout`

通过 `window.request_layout` 注册 Element 的 `Style` 和子布局节点。GPUI 使用 Taffy 布局引擎，在所有布局请求完成后计算尺寸和位置。

这个阶段返回 `LayoutId`，以及后续阶段需要的 `RequestLayoutState`。此时不要假设最终 `Bounds` 已经确定。

### `prepaint`

此时 GPUI 会传入布局完成后的 `Bounds`。在这里进行文字 shaping、几何计算、插入 hitbox、对子元素执行 prepaint，并准备 `paint` 所需的数据。

这些数据通过 `PrepaintState` 返回。命中测试应在这里准备，因为 GPUI 需要在绘制前建立当前帧的空间信息和派发信息。

### `paint`

使用前面准备好的状态绘制 quad、文字、路径或图片；自定义 Element 需要输入处理时，也在这个阶段注册帧级 handler。这里应复用前面算好的几何信息，不要再次进行布局计算。

状态在一帧内单向传递：

```text
RequestLayoutState ───────────────┐
        │                         │
        ▼                         ▼
     prepaint ── PrepaintState ──► paint
```

这些 associated state 只属于当前帧。长期存在的应用状态应放在 [Entity](./entity) 中；少量需要跨帧保存的 Element 状态可以通过 [ElementId](./element_id) 建立关联。

## 什么时候实现 `Element`

只有现有 Element 无法表达需求时才实现 `Element`，例如：

- 需要进行文字 shaping，并绘制选区和光标的代码编辑器或文本输入框；
- 具有自定义几何图形的图表或 canvas；
- 自定义布局算法；
- 需要直接控制布局、hitbox 与绘制的高性能底层原语。

GPUI 的文本输入示例使用自定义 `Element`，因为它需要在 `prepaint` 中 shape 一行文字，注册输入 handler，并绘制选区和光标。GPUI 的 `Svg`、`Img`、列表和 canvas 也使用同一套生命周期。相比之下，Zed 与 Longbridge Pro 的大部分 UI 都通过组合已有 Element 完成，只在不同分支返回不同类型时转换成 `AnyElement`。

编写可复用 UI 组件时，先使用 [`RenderOnce`](./render)。编写由 Entity 持有状态的 UI 时，使用 [`Render`](./render)。只有需要直接控制渲染管线时，才下沉到 `Element`。

## Identity 与交互

下面几个概念处在不同边界：

- [`ElementId`](./element_id) 在所在的 keyed scope 内标识一个 Element。GPUI 使用它的全局形式把跨帧状态和 retained work 连接起来。
- 对 `InteractiveElement` 调用 `.id(...)` 会返回 `Stateful<E>`。这个 wrapper 让需要稳定 Element identity 的 API 可以使用对应状态。
- `InteractiveElement` 提供 GPUI 的标准交互机制，包括 hitbox、鼠标 listener、Focus 跟踪、Key Context 与 Action handler。自定义 `Element` 不会自动获得这些能力。

如果 `div()` 等现有 Interactive Element 已能满足需要，直接组合它。自定义底层原语需要标准交互时，可以像 GPUI 内置 Element 一样，嵌入或委托给 GPUI 的 `Interactivity`。如果自行实现 hitbox 与输入事件注册，也要自行正确处理派发、裁剪、cursor 行为和无障碍信息。

:::info
`Element::id()` 返回 `ElementId` 不只是给像素加一个标签，它会建立跨帧的稳定 identity。ID 在最近的 keyed ancestor 中必须唯一；只有 Element 或附加行为确实需要 identity 时才添加 ID。
:::

[Element]: https://docs.rs/gpui/latest/gpui/trait.Element.html
[IntoElement]: https://docs.rs/gpui/latest/gpui/trait.IntoElement.html
[AnyElement]: https://docs.rs/gpui/latest/gpui/struct.AnyElement.html
