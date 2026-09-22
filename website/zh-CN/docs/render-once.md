---
title: RenderOnce
description: 使用持有数据的方式构建可复用、声明式的 GPUI 组件。
order: -2.6
---

# RenderOnce

GPUI 提供 `RenderOnce`，用于根据持有的数据构建可复用组件。父级每次 render 时会重新创建这些组件，因此它很适合按钮、列表行、Badge、Card 等不需要独立持久生命周期的声明式 UI。

```rust
use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div};

#[derive(IntoElement)]
struct MessageRow {
    author: SharedString,
    body: SharedString,
}

impl MessageRow {
    fn new(author: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        Self {
            author: author.into(),
            body: body.into(),
        }
    }
}

impl RenderOnce for MessageRow {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .gap_2()
            .child(div().font_semibold().child(self.author))
            .child(self.body)
    }
}
```

`#[derive(IntoElement)]` 会生成转换实现，让这个值可以直接加入 GPUI 的 Element tree：

```rust
div().child(MessageRow::new("You", "Explain RenderOnce"))
```

这个 derive 不会立即 render 组件。它会把 `RenderOnce` 值包装成 Element，GPUI 在处理外层 Element tree 时消费并 render 它。

## 为什么 `render` 会消费 `self`

这个方法签名是它与 [`Render`](./render) 最主要的区别：

```rust
fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement;
```

因为 `self` 的所有权属于 `render`，所以可以直接把字段移动到 Element tree 和 `'static` handler 中。组件值只使用一次；父级下一次 render 时会创建一个新的值。

多个字段需要移动到不同位置时，可以先解构，让所有权关系更清楚：

```rust
fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
    let MessageRow { author, body } = self;

    div()
        .child(author)
        .child(body)
}
```

这并不表示界面显示一帧后就会消失。生成的 Element 会参与当前帧；下次 render 时，父级再提供新的 UI 描述。

## 状态应放在组件值之外

不要把会变化的应用状态放进 `RenderOnce` 值，并期待修改能够保留。持久状态应该存放在实现了 `Render` 的 `Entity` 中，再把当前值或 `Entity` handle 传给组件。

```rust
#[derive(IntoElement)]
struct SendButton {
    chat: Entity<Chat>,
}

impl RenderOnce for SendButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let chat = self.chat;

        Button::new("send")
            .child("Send")
            .on_click(move |_, _, cx| {
                chat.update(cx, |chat, cx| {
                    chat.send_message(cx);
                });
            })
    }
}
```

Element handler 要求 `'static`，因此应使用 `move`，捕获 `SharedString`、`Entity<T>` 或 Action 等持有的值。如果调用方还需要某个 handle，应先 clone，再把副本移入 handler。

捕获 `Entity<T>` 会让该 Entity 在生成的 handler 存续期间保持存活。子组件操作其 owner 时，这通常符合预期。如果 handler 不应该延长目标的生命周期，应使用 `WeakEntity<T>`，并处理 `weak.update(...)` 已经无法访问目标的情况。

`RenderOnce::render` 得到的是 `&mut App`，而不是 `&mut Context<Self>`。因此 `RenderOnce` 组件没有自己的 Entity Context：它不能为自己使用 `cx.listener`、保存 Subscription，也不能调用 `cx.notify()`。此时应传入 handler、派发 [Action](./action)，或更新真正持有状态的 `Entity`。

## Builder 风格的组件

持有字段的方式很适合 Builder API。GPUI Kit 和 Zed 的 Button、List Item、Label、Modal Section 等组件都广泛使用这种模式：

```rust
#[derive(IntoElement)]
struct StatusBadge {
    label: SharedString,
    muted: bool,
}

impl StatusBadge {
    fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            muted: false,
        }
    }

    fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }
}

impl RenderOnce for StatusBadge {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .rounded_full()
            .px_2()
            .when(self.muted, |this| this.opacity(0.6))
            .child(self.label)
    }
}
```

## 选择合适的层级

| 使用 | 适用情况 |
| --- | --- |
| `RenderOnce` | 可复用组件由持有的输入构建，并且没有独立的持久状态。 |
| [`Render`](./render) | 有状态的 `Entity` 持有数据、Subscription、Task、Focus 或生命周期，需要反复 render。 |
| [`Element`](./element) | 需要直接控制 layout、prepaint、paint、hit testing 等底层渲染阶段。 |

常见的组合方式是：`Render` view 持有状态，由它创建 `RenderOnce` 组件来描述可复用 UI，而这些组件返回内置 Element。只有标准 Element API 无法表达所需渲染行为时，才需要直接实现 `Element`。

:::info
如果一个组件开始积累可变状态、Subscription 或后台 Task，应把这些生命周期移入 `Entity`，并为它实现 `Render`。把它们放在每次 render 都会被消费的值里，会破坏 `RenderOnce` 简单明确的所有权模型。
:::
