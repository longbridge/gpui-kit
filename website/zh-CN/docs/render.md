---
title: Render
description: 从 Entity 状态构建元素树，并理解 GPUI 何时重建 View。
order: -2.5
---

# Render

`Render` 连接持久存在的 [`Entity<T>`](./entity) 与它当前描述的界面。GPUI 需要某个 View 的[元素树](./element)时，会调用 `T::render`。Entity 保留数据和 identity；返回的元素则描述本次渲染的布局、外观与 handler。面板、页面、编辑器等需要拥有可变状态、订阅或子 Entity 的 View，适合实现 `Render`。

## 运行一个有状态 View

完成[入门教程](./getting-started)后，用下面的完整代码替换该项目的 `src/main.rs`。它沿用同一个 `gpui-kit` 依赖，无需新增示例 crate。

```rust
use gpui_kit::*;
use gpui_kit::component::button::Button;

#[derive(Default)]
struct Chat {
    messages: Vec<SharedString>,
}

impl Chat {
    fn add(&mut self, cx: &mut Context<Self>) {
        self.messages
            .push(format!("Message {}", self.messages.len() + 1).into());
        cx.notify();
    }

    fn clear(&mut self, cx: &mut Context<Self>) {
        if !self.messages.is_empty() {
            self.messages.clear();
            cx.notify();
        }
    }
}

impl Render for Chat {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_2()
            .p_4()
            .child(format!("Messages: {}", self.messages.len()))
            .child(
                Button::new("add-message")
                    .label("Add message")
                    .on_click(cx.listener(|this, _, _, cx| this.add(cx))),
            )
            .child(
                Button::new("clear-messages")
                    .label("Clear")
                    .on_click(cx.listener(|this, _, _, cx| this.clear(cx))),
            )
            .children(
                self.messages
                    .iter()
                    .cloned()
                    .map(|message| div().child(message)),
            )
    }
}

fn main() {
    application()
        .with_assets(assets::Assets)
        .run(|cx| {
            init(cx);
            open_window(WindowOptions::default(), cx, |_, cx| {
                cx.new(|_| Chat::default())
            })
            .expect("Failed to open window");
        });
}
```

运行 `cargo run`。窗口最初显示 **Messages: 0** 和两个按钮。点击 **Add message** 后，计数变为 1，并出现 **Message 1**；继续点击会添加更多行。点击 **Clear** 会清空行，计数回到 0。若列表已经为空，再点 **Clear** 不会改变状态。

`open_window` 保留 `cx.new` 返回的 `Entity<Chat>`；`Chat::default()` 只在创建该 Entity 时运行。`Render::render` 接收 View 的可变引用、`Window` 和该 View 的 [Context](./context)，返回 `impl IntoElement`；trait 要求 `Self: 'static + Sized`。返回类型无需写出具体、往往深度嵌套的 Element 类型。这里的 `div()` 是 GPUI Element，`Button` 是 GPUI Kit 组件。点击 closure 在渲染时只是被*注册*，发生输入后才通过 `cx.listener` 取得 `Chat` 的可变引用。构建元素树时，按钮并不会修改 `Chat`。

沿一次点击追踪所有权与渲染过程：

| 步骤 | 发生的事 |
| --- | --- |
| 1. 输入 | 按钮调用先前注册的回调。`cx.listener` 进入已有的 `Chat` Entity 并调用 `add`。 |
| 2. 状态 | `add` 向 `Chat::messages` 添加消息，然后通过该 Entity 的 `cx.notify()` 发出变化通知。 |
| 3. 窗口处理 | GPUI 使当前展示 `Chat` 的窗口失效；之后处理受影响的 View 时，`render` 描述新的计数和消息行。 |
| 4. 元素处理 | GPUI 按需计算布局、执行 prepaint 和 paint，随后才能呈现变化。 |

这是一次更新的路径，不代表固定的 `render` 调用次数或显示帧数。如果文字一直停在 **Messages: 0**，检查回调是否修改了已挂载的 `Chat` Entity，并调用它的 `Context<Chat>::notify()`。如果点击一次却重复发起请求或写日志，检查这类工作是否误放进了 `render`，而不是点击 handler。

## View 持有状态，元素树描述本次界面

使用 `cx.new` 创建 Entity。父 View 可以保留同一个 handle，并在每次渲染时将它放入树中：

```rust
struct ConversationPage {
    chat: Entity<Chat>,
}

impl ConversationPage {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            chat: cx.new(|_| Chat::default()),
        }
    }
}

impl Render for ConversationPage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.chat.clone())
    }
}
```

`ConversationPage` 在多次渲染之间持有同一个 `Chat` Entity，不会因为页面重绘就重新创建消息列表。克隆 `Entity<Chat>` 只复制 handle，不复制消息。文件列表页面也可以采用同样的结构：以子 Entity 持有文件列表 View，由它自己管理选择、加载状态和订阅。`Entity<T: Render>` 可以作为 child，因为 GPUI 会把它转换为 View Element。Entity ID 为这个 View 提供响应式边界及独立的 Element ID 空间。创建、共享及生命周期规则见 [Entity](./entity)。

## Render 还是 RenderOnce？

`Entity<T>` 首先是状态容器，`T` 不必实现 `Render`。`Entity<Model>` 可以只保存数据、接受更新并通知观察者，完全不出现在元素树中。当 `T: Render` 时，同一个容器也能作为持久 View 加入元素树，并以 Entity ID 作为身份。GPUI 的 `View` 机制也接受 `RenderOnce` 值，但这些值没有自己的 Entity ID。`Render` 是 Entity 承载 UI 的路径，并非所有 Entity 的必备条件。

| Trait | 接收者与所有者 | 适合承担的工作 |
| --- | --- | --- |
| [`Render`](./render) | 持久 `Entity<T>` View 上的 `render(&mut self, ..., &mut Context<Self>)` | 跨渲染过程持有变化或复杂的状态、订阅、焦点、Task 与子 Entity。 |
| [`RenderOnce`](./render-once) | 父级构造的值由 `render(self, ..., &mut App)` 消费 | 根据当前输入和回调描述轻量组件；父级下次渲染时提供新的值。 |

上面的 `Chat` View 持有会变化的消息集合，因此实现 `Render`。GPUI Kit 的 `SelectState<D>` 是另一种实现 `Render` 的状态所有者。带样式的 `Button` 则实现 `RenderOnce`：父级提供当前 props，并处理点击结果。`RenderOnce` 值仍然可以交互；GPUI 也可能在其内部保留少量 keyed element state，只是这个值没有独立的 Entity 生命周期。两者区别在于**持久状态由谁拥有**，不是承诺一个 trait 每显示帧只运行一次，或另一个 trait 每帧都会运行。

返回元素树不等于立即绘制。GPUI 会把值转换成元素，计算布局，执行 `prepaint`，最后[绘制](./paint)。View 的 `render` 可能发生在布局或 prepaint 过程中；有效的子树也可能直接复用。因此不能假设每帧恰好调用一次 `render`，也不能假设父 View 每次渲染都会调用所有子 View 的 `render`。输出是供 GPUI Element 管线处理的描述，不是持久保存的像素列表。各阶段见 [Element](./element)。

关于渲染过程、屏幕刷新与已呈现帧的区别，以及“120 FPS”和 GPUI hybrid 模型的含义，参阅 [FPS Monitor](./fps#120-hz-是帧预算不是刷新承诺)。

## 有意义的状态变化后调用 notify

Entity update 提供可变访问，但仅修改字段并不会报告界面变化。当某个 Entity 的渲染结果或观察者应该更新时，调用*该 Entity* 的 `Context<T>::notify()`：

```rust
chat.update(cx, |chat, cx| {
    chat.messages.push("Hello".into());
    cx.notify(); // This is Context<Chat>, not the caller's Context.
});
```

`notify` 使正在展示该 Entity 的窗口失效，并把通知排给观察者。GPUI 会安排后续渲染，不会在 `notify` 这一行同步调用 `render`。同一个 View 若展示在多个活动窗口，可能让各窗口都失效。GPUI 会将变化的 View 及其已渲染的祖先标记为 dirty，随后可能复用符合条件的缓存子树。不要依赖固定的 `render` 调用次数。对于自身不渲染、但有人观察的模型 Entity，通知也有意义。

[Event](./event) 与通知承担不同职责：`cx.emit(event)` 向订阅者传递带类型的事实；`cx.notify()` 报告 Entity 已变化。只发送 Event 不代表读取该 Entity 的所有 View 都会重绘；通知也不携带 Event 的数据。若状态变化和事件事实都需要对外可见，可分别使用两者。

一个 View 读取另一个 Entity 的状态时，要明确谁拥有数据、谁需要失效。若另一个 Entity 作为子 View 渲染，其自身 `notify` 可以使该子 View 失效。若父 View 把别的 Entity 的值复制到自己的输出中，应观察源 Entity，并在这些值变化时调用父 View 的 `cx.notify()`。不要假设保留状态的子 View 只能靠父 View 重渲染才能变化。

### 沿示例追踪一次更新

1. `ConversationPage::new` 创建一个 `Chat` Entity 并保留它的 handle。页面每次渲染，都把同一个 handle 放进元素树。
2. 调用方可以用 `chat.update(cx, |chat, cx| { chat.messages.push("Hello".into()); cx.notify(); });` 添加消息。内层 `cx` 属于 `Chat`；这次更新无需另行通知 `ConversationPage`。
3. 在之后的窗口渲染过程中，GPUI 按需构建受影响的元素树，并执行布局、prepaint 和 paint，新消息随之出现。点击 **Clear** 后，先前注册的 listener 清空 `Chat` 的消息，再通知同一个 Entity。
4. 再点一次 **Clear** 不会改变集合，所以 `clear` 不调用 notify。这两步都不保证固定的 `render` 调用次数：窗口刷新和缓存是否可用也会影响调用次数。

在运行的 View 中检查所有权边界：通过保留的 `Chat` handle 添加消息，确认消息出现。改变与 `Chat` 无关的父级状态，确认子级仍保留消息；如果列表重置，检查是否在父级的 `render` 中重新创建了 `Chat` Entity。随后点击 **Clear**，确认消息消失。

## Render 构建值，handler 执行工作

读取状态、选择子元素、应用[样式](./style)与注册 handler，都是正常的渲染工作。GPUI 重新构建元素树的原因未必是用户操作。如果在 `render` 中无条件发起请求、注册订阅、发送 Event 或修改应用状态，每次重建都可能重复执行。在 `render`、`prepaint`、`paint` 或 canvas callback 中无条件调用 `cx.notify()`，还可能让窗口持续失效。

把生命周期工作放在 Entity 初始化、订阅或明确的输入 handler 中。`Subscription` 应由订阅的 Entity 持有。异步工作完成后，通过 update 修改状态；状态确有变化时，从该 update 调用 notify。上面 Chat 示例中的 `cx.listener` 会把 GPUI Kit `Button` 日后发出的点击回调转为当前 `Render` 所有者的更新。handler 也可以捕获 `Entity<T>` handle，以更新另一位所有者；不要在 Entity 正在渲染或更新时重新进入同一 Entity。

GPUI Kit 的输入与选择状态等状态所有者实现 `Render`；`Button` 等控件用 `RenderOnce` 把当前 props 转成元素。需要直接控制布局、hitbox 或绘制时才实现自定义 `Element`。这种组合让持久 View 负责行为，生命周期较短的值描述当前界面。

## 常见问题

| 现象 | 检查方向 |
| --- | --- |
| 状态已改变，画面却没更新 | 是否更新了正确的 Entity，并通过它的 Context 调用 notify？ |
| 空闲时仍不断执行工作 | `render` 或绘制回调是否每次都启动工作或调用 notify？ |
| 父 View 变化后子状态重置 | 是否在 `render` 中每次新建子 Entity，而没有由所有者保留 handle？ |
| 点击 handler 无法借用 `self` | 用 `cx.listener` 注册拥有自身数据的回调，或捕获 Entity handle 供日后 update。 |
| 重复列表项的局部 UI 状态错位 | 使用 keyed state 的重复元素应采用基于条目 identity 的稳定 ID，不要直接用数组下标。见 [ElementId](./element_id)。 |

本文使用的 Context API 见 [Context](./context)，输入回调及事件详见 [Event](./event)。
