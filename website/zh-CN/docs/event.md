---
title: Event
description: 使用 GPUI Event 发送类型化通知，并理解它与 Action 的关系。
order: -6
---

# Event

GPUI 原生提供 **Event**，用于在 Entity 之间发送类型明确的通知。Event 报告已经发生的事实，不经过 Focus、Key Context、KeyBinding 或 Dispatch Path。

## Action 进入，Event 返回

Action 与 Event 经常组成一次完整交互：

```text
⌘ Enter → SendMessage Action → Chat 发送 → MessageSent Event → Workspace 更新
```

- **Action** 把意图向内传递：“发送这条消息”；
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
    cx.notify();
}
```

Event 应按已经发生的事实命名，例如 `MessageSent`、`Saved`、`Dismissed`。`SendMessage` 这种命令式名称属于 Action。

## 由 owner 订阅

Owner 在组装 Entity 时订阅：

```rust
let chat = cx.new(Chat::new);
let subscription = cx.subscribe(&chat, |workspace, _, event, cx| {
    if matches!(event, ChatEvent::MessageSent { .. }) {
        workspace.refresh_conversation();
        cx.notify();
    }
});
```

API 要求时必须保留返回的 `Subscription`，通常存进 `_subscriptions: Vec<Subscription>`。丢弃它就会断开订阅。回调还需要 `&mut Window` 时使用 `window.subscribe(...)`。

:::info INFO — Event 不跟随 Focus 路由

Event 只发送给 source Entity 的订阅者。移动 Focus 或改变 Key Context 不会改变接收者。不要把 Event 当成绕过 Action routing 的全局命令总线。

:::

## `emit` 与 `notify` 不同

`cx.emit(...)` 携带 payload 发送类型化语义事实；`cx.notify()` 告诉观察者重新读取 Entity state，通常会触发重绘。一次状态变化可能只需要其中一个，也可能两个都需要；发出 Event 不会自动请求 render。

## 什么时候用 Action，什么时候用 Event

| 问题 | 使用 | 例子 |
| --- | --- | --- |
| 这是用户或调用者希望执行的指令吗？ | **Action** | 保存、删除、打开搜索 |
| 它需要绑定快捷键或出现在菜单里吗？ | **Action** | 复制、切换侧边栏、重命名 |
| 这是状态或生命周期变化后报告的事实吗？ | **Event** | ValueChanged、Saved、Dismissed |
| Owner 是否要独立于 UI tree 观察 child？ | **Event** | 输入变化、选择行、提交对话框 |
| 它只是鼠标手势且没有其他命令入口吗？ | callback | hover、拖动距离、指针位置 |

当一条命令产生了应用其他部分需要观察的事实时，两者一起使用：先处理 Action，提交状态变化，再发出 Event。
