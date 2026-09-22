---
title: Render
description: 将有状态的 GPUI Entity 渲染为元素树。
order: -2.5
---

# Render

GPUI 提供 `Render` trait，用于把 `Entity` 的当前状态转换成元素树。对于 Chat 面板、设置页面、Workspace 这类状态会持续变化、生命周期较长的 View，应使用 `Render`。

```rust
use gpui::{div, prelude::*, Context, IntoElement, Render, Window};

struct Chat {
    messages: Vec<String>,
}

impl Render for Chat {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .children(
                self.messages
                    .iter()
                    .cloned()
                    .map(|message| div().child(message)),
            )
    }
}
```

`render` 会收到：

- `&mut self`：保存在当前 Entity 中的状态；
- `&mut Window`：窗口级状态与操作；
- `&mut Context<Self>`：当前 Entity 的 GPUI Context；
- 返回 `impl IntoElement`：可被 GPUI 转换成元素树的值。

使用 `impl IntoElement` 后，函数签名不需要写出通常很深的具体元素类型。`div()`、GPUI Kit 组件以及子级 `Entity<T>` 都可以加入这棵树。

## 更新 View

只修改 Entity 并不会告诉 GPUI 它的可见内容已经变化。应在 Entity update 中修改状态，然后调用 `cx.notify()`：

```rust
impl Chat {
    fn push_message(&mut self, message: String, cx: &mut Context<Self>) {
        self.messages.push(message);
        cx.notify();
    }
}
```

它们的关系是：

```text
Entity 状态变化
       ↓
   cx.notify()
       ↓
GPUI 使展示该 Entity 的 View 失效
       ↓
Render 构建下一棵元素树
```

`cx.notify()` 还会通知该 Entity 的观察者。GPUI 会安排后续渲染，并不会在调用 `notify` 的这一行同步执行 `render`。同一个 Entity 同时显示在多个窗口或位置时，GPUI 会使正在展示它的 View 失效。

## 让 Render 保持声明式

把 `render` 看作“根据当前状态描述 UI”。读取状态、选择子元素和绑定 handler 都是正常操作；不要仅仅因为 `render` 被调用，就启动工作或修改应用状态：

- 不要发起网络请求或后台任务；
- 不要创建 Subscription 或注册应用级观察者；
- 不要派发命令或发送 Event；
- 不要无条件调用 `cx.notify()`、`window.refresh()` 或 `window.request_animation_frame()`。

View 失效时，GPUI 可能再次调用 `render`。如果在 `render`、`prepaint`、`paint` 或 canvas callback 中无条件调用 `cx.notify()`，窗口可能被持续标记为需要重绘，形成空闲重绘循环。应在初始化阶段或明确的 handler 中启动工作；结果返回后更新 Entity，并且只在可见状态确实变化时调用 `cx.notify()`。

在渲染时绑定输入 handler 是另一回事：closure 只是作为元素树的一部分被注册，之后发生输入时才会执行。

```rust
div()
    .child("Clear")
    .on_click(cx.listener(|this, _, _, cx| {
        this.messages.clear();
        cx.notify();
    }))
```

## Render、RenderOnce 与 Element

选择能够满足需求的最小抽象：

| API | 适用场景 | 方法接收者 | Context |
| --- | --- | --- | --- |
| `Render` | 由 `Entity<T>` 承载、有状态且长期存在的 View | `&mut self` | `Context<Self>` |
| [`RenderOnce`](./render-once) | 根据输入数据组合出的可复用组件 | `self` | `App` |
| [`Element`](./element) | 自定义布局、prepaint、hitbox 或绘制 | 各阶段的 `&mut self` | `App` |

当 `T: Render` 时，`Entity<T>` 可以直接作为 child。Entity ID 为 View 提供 identity，`notify` 可以使对应的 View 子树失效。`RenderOnce` 组件会在构建元素树时消耗自身，没有独立的 Entity identity。只有组合已有元素无法满足需求时，才需要直接实现 `Element`。

## 相关文档

- [`Entity`](./entity) 介绍状态的所有权、读取和更新。
- [`Context`](./context) 介绍 `App`、`Window` 和 `Context<T>`。
- [`RenderOnce`](./render-once) 介绍如何根据 owned props 构建可复用组件。
- [`Element`](./element) 介绍 GPUI 的布局与绘制阶段。
