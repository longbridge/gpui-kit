---
title: Event
description: 使用 GPUI Event 发送类型化通知，并理解它与 Action 的关系。
order: -6
---

# Event

GPUI 提供 **Event**，用于在 Entity 之间发送类型明确的通知。Event 报告已经发生的事实；与 [**Action**](./action) 不同，它不经过 Focus、Key Context、KeyBinding 或 Dispatch Path。

## Action 进入，Event 返回

Action 可以触发状态变化，但 Event 的传递从状态变化之后开始：

```text
Chat 状态变化 → emit(MessageSent) → 订阅者收到 Event → Workspace 更新
```

<img class="architecture-light" src="/event-subscriptions-flow.svg?v=20260922-1" alt="Chat 发出一个 MessageSent Event，分别送达 Workspace、Activity Log 与 Telemetry 三个独立订阅者">
<img class="architecture-dark" src="/event-subscriptions-flow-dark.svg?v=20260922-1" alt="Chat 发出一个 MessageSent Event，分别送达 Workspace、Activity Log 与 Telemetry 三个独立订阅者">

- [**Action**](./action) 把意图向内传递：“发送这条消息”；
- **Event** 把结果向外报告：“这条消息已经发送”。

Command owner 处理 Action 并改变状态，再发出 Event，让 owner 或 service 响应结果，而不必依赖命令来自快捷键、按钮还是菜单。Focus、KeyBinding 与 Action 派发参见 [Action](./action)。

## 定义并发出 Event

先定义 Entity 可以报告的事实，并实现 `EventEmitter`：

```rust
#[derive(Clone, Debug)]
enum ChatEvent {
    DraftChanged,
    MessageSent { message_id: MessageId },
}

impl EventEmitter<ChatEvent> for Chat {}
```

状态变化成功后再发出 Event：

```rust
fn finish_send(&mut self, message_id: MessageId, cx: &mut Context<Self>) {
    self.draft.clear();
    cx.emit(ChatEvent::MessageSent { message_id });
}
```

Event 应按已经发生的事实命名，例如 `MessageSent`、`Saved`、`Dismissed`。`SendMessage` 这种命令式名称属于 Action。

## 由 owner 订阅

订阅方应把 Subscription 保存在发起订阅的同一个 View 上。GPUI Kit 的示例采用下面这种模式：

```rust
struct Workspace {
    chat: Entity<Chat>,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    fn new(cx: &mut Context<Self>) -> Self {
        let chat = cx.new(Chat::new);
        let _subscriptions = vec![cx.subscribe(&chat, |workspace, _, event, _cx| {
            if matches!(event, ChatEvent::MessageSent { .. }) {
                workspace.refresh_conversation();
            }
        })];

        Self { chat, _subscriptions }
    }
}
```

不要只用局部变量保存返回的 `Subscription`：函数结束后它会被 drop，观察者随即断开。把 `_subscriptions` 放在 `Workspace` 上，两者便拥有相同生命周期；View 释放时，Subscription 也会一起释放并取消订阅。也不要把 View 级 Subscription 存到生命周期更长的全局 owner，否则 View 消失后 callback 与捕获的资源仍然存活，可能造成内存泄漏。

回调需要 `&mut Window` 时使用 `cx.subscribe_in(..., window, ...)`，返回的 `Subscription` 同样保存在这个字段中。

:::info INFO — Event 不跟随 Focus 路由

Event 只发送给 source Entity 的订阅者。移动 Focus 或改变 Key Context 不会改变接收者。不要把 Event 当成绕过 Action routing 的全局命令总线。

:::

## 什么时候用 Action，什么时候用 Event

| 问题 | 使用 | 例子 |
| --- | --- | --- |
| 这是用户或调用者希望执行的指令吗？ | **Action** | 保存、删除、打开搜索 |
| 它需要绑定快捷键或出现在菜单里吗？ | **Action** | 复制、切换侧边栏、重命名 |
| 这是状态或生命周期变化后报告的事实吗？ | **Event** | ValueChanged、Saved、Dismissed |
| Owner 是否要独立于 UI tree 观察 child？ | **Event** | 输入变化、选择行、提交对话框 |
| 它只是鼠标手势且没有其他命令入口吗？ | callback | hover、拖动距离、指针位置 |

当一条命令产生了应用其他部分需要观察的事实时，两者一起使用：先处理 Action，提交状态变化，再发出 Event。
