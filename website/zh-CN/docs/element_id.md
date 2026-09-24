---
title: ElementId
description: 为 GPUI 元素提供稳定标识，并理解带 key 的状态如何跨帧保留。
order: -2.4
---

# ElementId

`ElementId` 是 GPUI 渲染树中元素的**局部 key**。GPUI 将它与带 key 的祖先元素 ID 组合，形成 `GlobalElementId`。凭借这条路径，View 再次 render 时，GPUI 能把交互和元素状态关联到同一个逻辑元素。当元素有 role 时，这条路径也让它在[无障碍树](./accessibility)中保持节点身份。

ID 不是 [Entity] 的 handle，也不能像 HTML DOM ID 一样用来查找元素。共享的应用状态用 `Entity<T>`；元素树中的身份用 `ElementId`，例如组件内部按 key 保存的状态、组件提供的焦点或滚动行为，以及自定义 [Element] 保留的状态。

## 赋予元素 ID

在 `div()` 等交互元素上调用 `.id(...)`，会得到一个 `Stateful<Div>`：

```rust
let save = div()
    .id("save-button")
    .on_click(|_, window, cx| {
        // 处理点击。
    })
    .child("Save");
```

`Stateful<E>` wrapper 提供 GPUI 的有状态交互方法，并携带元素 ID。自定义 `Element` 可以在 `id()` 方法中返回 `Some(id)`，而不经过这个 wrapper。普通、没有 key 的 `div()` 仍可以作为布局父节点，只是不会在 ID 路径中增加一段。

字符串、整数，以及名称加整数的元组，都是常见的 `ElementId` 输入：

```rust
use gpui_kit::component::button::Button;

div().id("search")
div().id(("message", message.id))
Button::new(("delete-project", project.id)).label("Delete")
```

key 应取自对象的身份，而非显示给用户的文字。翻译后的标签、当前选中项、每次新生成的随机值，都可能在对象仍然相同的时候改变。

## GlobalElementId 与祖先路径

GPUI 从元素路径上的 ID 组成 `GlobalElementId`。只有带 key 的祖先才会增加路径段：

```text
div().id("workspace")
├── div().id("inbox")
│   └── div().id(("row", 42))  → ["workspace", "inbox", ("row", 42)]
└── div().id("archive")
    └── div().id(("row", 42))  → ["workspace", "archive", ("row", 42)]
```

两个 row 可以复用局部 ID，因为它们的带 key 祖先路径不同。图中只展示手写的 ID：由 Entity 支撑的 View 还会将其 `EntityId` 加入路径，`RenderOnce` 组件则会加入类型名命名空间。这条路径属于 Window 的渲染树；`GlobalElementId` 是 GPUI 的内部路径，无须在调用处自行构造，也不是整个进程通用的字符串。若自定义绘制 API 需要为自身的 key 取得路径，可用 `window.with_global_id(key, |global_id, window| { … })` 在回调期间创建。

实际的唯一性规则是：**在同一个最近的带 key 祖先之下，各条带 key 的后代分支需要不同的 ID**。没有 key 的容器不会开辟新命名空间：

```rust
div().id("workspace")
    .child(div().child(div().id("item")))
    .child(div().child(div().id("item"))) // 带 key 的路径相同，发生冲突。
```

可以给两条分支赋予各自稳定的 ID，或让两个 item 使用不同 ID。路径重复会使保留状态和交互关联到错误的逻辑元素。

## 变化列表中的稳定 key

列表项可能插入、删除、过滤或重新排序时，每一行的 ID 应来自稳定的业务标识：

```rust
div().id("messages").children(messages.iter().map(|message| {
    div()
        .id(("message", message.id))
        .child(message.preview.clone())
}))
```

`("message", message.id)` 将 row 的用途与其他使用相同数字 ID 的控件区分开。调整顺序会改变绘制位置，但每条消息仍保持原来的 key 路径。相反，`.id(index)` 将状态绑定到**位置**：插入一行后，原来第一行的焦点、滚动、动画或其他带 key 状态，可能被另一个消息复用。只有位置本身就是身份且不会移动时，索引才合适。

如果重复的 row 中还有多个控件，先给 row 设置 key，再给子控件设置 `"edit"`、`"delete"` 等不同的局部 key。这样移动 row 时，它下面整棵带 key 的子树都会跟着移动。

## ID 如何保留状态

GPUI 会在 render 时重新构建元素。`div()` 或 `RenderOnce` 组件返回的 Rust 值是临时的；赋予 ID 不会让它变成持久的 `Entity<T>`。ID 的作用是让 GPUI 在连续帧之间重新关联元素局部状态。

`window.use_keyed_state(key, cx, init)` 使用当前带 key 的祖先路径加上 `key`。它返回 `Entity<S>`；只要连续渲染帧都使用这个 key，状态就会保留。`Window` 管理这份 keyed state；`cx` 提供应用访问（不同类型的说明见 [Context](./context)）。没有旧状态时才调用 `init`。GPUI 还会观察这份状态 Entity；状态变化时通知当前 View：

```rust
let focus_handle = window
    .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
    .read(cx)
    .clone();
```

GPUI Kit 的 `Button` 用这种方式保存焦点 handle。它的公开 ID 让每次重新构建的按钮都能找到相同的状态 key。焦点行为仍由 focus handle 承担；元素 ID 决定这个 handle 的状态归属何处。只有路径在连续帧中被访问，窗口才会保留其 keyed state；[缓存 View](./view-cache) 重用子树时会重放这些访问。单独保存的强 `Entity<S>` 句柄可以让该 Entity 在窗口状态条目消失后继续存活，但路径重新出现时仍会执行 `init`。

自定义 `Element` 的 `id()` 返回 ID 时，GPUI 会在 `request_layout`、`prepaint` 和 `paint` 中传入 `Option<&GlobalElementId>`。绘制期间，`window.with_element_state(global_id, ...)` 可以读取上一帧状态，并返回需要保存到下一帧的值：

```rust
let state = window.with_element_state(
    id.expect("this element always has an ID"),
    |previous: Option<AnimationState>, _window| {
        let state = previous.unwrap_or_default();
        (state.clone(), state)
    },
);
```

GPUI Kit 的 `ScrollBounce` 元素就采用这个模式，在 `prepaint` 中保留运动状态。`with_element_state` 是供元素作者在绘制阶段使用的 API；普通 View 应优先使用 Entity 状态或组件文档提供的 API。GPUI 按全局路径**以及状态类型**存储元素状态；元素不再参与后续渲染帧时，该状态会被释放。

:::info
`window.use_state(cx, init)` 使用调用位置作为局部 key，只要求**完整路径**唯一：如果每行已有稳定的带 key 祖先，同一个调用位置用于多行也是安全的。如果多次调用共享同一带 key 祖先路径，应改用带稳定条目 key 的 `use_keyed_state`，或为每个条目建立带 key 的命名空间。
:::

## 身份变化也会改变状态

- 改变元素 ID 或带 key 祖先的 ID，会生成新路径，并重置相应的元素状态。
- 移除元素会终止其连续帧状态的生命周期；以后重新创建时会再次初始化。
- ID 本身不会保留 View 的 `Entity<T>`；其生命周期由强 Entity owner 决定。
- 带 key 的状态属于 Window 的渲染上下文。不要依靠 `GlobalElementId` 在不同窗口之间转移状态。

布局、prepaint 和 paint 生命周期见 [Element](./element)；需要在元素离开渲染树后继续存在的状态见 [Entity](./entity)。

[Element]: /zh-CN/docs/element
[Entity]: /zh-CN/docs/entity
