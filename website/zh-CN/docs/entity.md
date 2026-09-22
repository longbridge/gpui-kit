---
title: Entity
description: 使用 GPUI Entity 创建、共享、读取、更新和观察状态。
order: -2.1
---

# Entity

当一份状态需要由多个 View、handler 或异步任务共同使用时，把它放进 GPUI 提供的 `Entity<T>`。例如，Chat 可以用 `Entity<Chat>` 保存消息；任何持有其 clone 的代码都能通过 GPUI Context 访问同一份 Chat。

使用 `cx.new` 创建 Entity，使用 `read` 读取状态，使用 `update` 修改状态。当 `Chat` 实现 `Render` 时，`Entity<Chat>` 还可以直接作为 View 渲染；不需要渲染时，它就是一个共享状态 model。

```text
Entity<Chat>
    ├── read(cx)       → &Chat
    ├── update(cx, …)  → &mut Chat + Context<Chat>
    └── downgrade()    → WeakEntity<Chat>
```

clone Entity 只会复制句柄，不会复制其中的状态。Entity 只能通过 GPUI Context 访问，因此 GPUI 可以统一协调状态更新、渲染、订阅和 Entity 生命周期。

## 创建 Entity

可以在任意 GPUI context 中使用 `cx.new`：

```rs
struct Chat {
    messages: Vec<String>,
}

let chat: Entity<Chat> = cx.new(|_cx| Chat {
    messages: Vec::new(),
});
```

闭包会收到 `Context<Chat>`，因此初始化时也可以创建子 Entity 或注册订阅。

当子 Entity 应该与 owner 同时存活时，owner 保存一个强引用 `Entity<T>`：

```rs
struct Workspace {
    chat: Entity<Chat>,
}

impl Workspace {
    fn new(cx: &mut Context<Self>) -> Self {
        let chat = cx.new(|_cx| Chat {
            messages: Vec::new(),
        });

        Self { chat }
    }
}
```

gpui-kit 和 Zed 广泛采用这种强所有权关系：父 View 持有它所渲染、协调的子 View 或 model。

## 读取状态

直接同步访问状态时使用 `read`：

```rs
let message_count = chat.read(cx).messages.len();
```

返回引用的生命周期受 `cx` 限制。需要长期使用某个值时，应复制或 clone 该值，不要保存这个引用。

当代码接收泛型 `AppContext`，或希望用闭包明确读取范围时，可以使用 `read_with`：

```rs
let last_message = chat.read_with(cx, |chat, _cx| {
    chat.messages.last().cloned()
});
```

## 更新状态

使用 `update` 获得可变状态和对应的 `Context<T>`：

```rs
chat.update(cx, |chat, cx| {
    chat.messages.push("Hello".into());
    cx.notify();
});
```

`cx.notify()` 表示这个 Entity 发生了变化，渲染过或正在观察它的 View 随后可以更新。仅修改字段不会自动产生通知；当新状态需要反映到观察者或界面上时，应调用它。

始终使用 `update` 闭包传入的内部 `cx`。它是当前 Entity 的 `Context<Chat>`。

:::info
同一个 Entity 正在 update 或 render 时，不要再次对它调用 `read` 或 `update`。GPUI 会阻止这种重入访问并 panic。应该直接使用当前回调已经提供的 `&mut T`，或者先结束本次 update，再开始下一次访问。
:::

## 使用 WeakEntity 表示反向引用和 callback

clone `Entity<T>` 会创建另一个强句柄，并让 Entity 继续存活。当一段关系不应该拥有目标时，应使用 `WeakEntity<T>`，例如子 View 反向引用父 View，或长时间运行的 callback 引用某个 View。

```rs
struct ChatSidebar {
    workspace: WeakEntity<Workspace>,
}

let workspace = cx.weak_entity();
let sidebar = cx.new(|_cx| ChatSidebar { workspace });
```

弱句柄可能比目标存活得更久。可以尝试 upgrade，或者使用它提供的可失败访问方法：

```rs
let workspace = cx.weak_entity();

cx.spawn(async move |_, cx| {
    let conversations = load_conversations().await;

    workspace
        .update(cx, |workspace, cx| {
            workspace.set_conversations(conversations);
            cx.notify();
        })
        .ok();
})
.detach();
```

`WeakEntity::upgrade` 返回 `Option<Entity<T>>`；`read_with` 和 `update` 返回 `Result`，因为目标 Entity 可能已经释放。Zed 和 gpui-kit 会在异步任务、callback、delegate 和父级引用中使用这一模式，避免这些关系意外延长 View 的生命周期。

## 观察变化与订阅 Event

一个 Entity 可以通过两种相关方式与另一个 Entity 协作：

- `cx.observe(&entity, ...)` 在目标 Entity 调用 `cx.notify()` 时执行。只关心“状态变了”时使用。
- `cx.subscribe(&entity, ...)` 接收类型化 [Event]。需要知道变化的含义和数据时使用。

应该把返回的 `Subscription` 保存在发起订阅的 Entity 上：

```rs
enum ChatEvent {
    MessageSent,
}

impl EventEmitter<ChatEvent> for Chat {}

struct Workspace {
    chat: Entity<Chat>,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    fn new(cx: &mut Context<Self>) -> Self {
        let chat = cx.new(|_cx| Chat {
            messages: Vec::new(),
        });

        let _subscriptions = vec![
            cx.observe(&chat, |_workspace, _chat, cx| {
                cx.notify();
            }),
            cx.subscribe(&chat, Self::on_chat_event),
        ];

        Self {
            chat,
            _subscriptions,
        }
    }

    fn on_chat_event(
        &mut self,
        _chat: Entity<Chat>,
        _event: &ChatEvent,
        cx: &mut Context<Self>,
    ) {
        // 处理类型化 Event。
        cx.notify();
    }
}
```

gpui-kit 的 View，以及 Zed 和 Longbridge Pro 中更复杂的 View，都在使用这一模式。`Workspace` 被释放时，`_subscriptions` 也会随之释放，callback 会自动断开。局部的 `let subscription = ...` 通常是错误写法，因为它会在函数结束时立即被 drop。把 View 的订阅 detach，或保存在生命周期更长的全局 owner 中，也可能导致 View 消失后 callback 与捕获的资源仍然存活，造成内存泄漏。

`EventEmitter`、`emit` 和类型化订阅的设计请继续阅读 [Event]。

## 生命周期

只要还有一个强引用 `Entity<T>`，Entity 就会继续存活。最后一个强句柄被 drop 后，GPUI 会释放其状态，此时 `WeakEntity<T>` 将无法再 upgrade。

大部分清理工作应该直接跟随所有权关系：

- 使用 `Entity<T>` 持有子 Entity；
- 使用 `WeakEntity<T>` 表示非拥有关系；
- 把 View 级 Subscription 放在同一个 View 的 `_subscriptions` 字段中；
- View 被 drop 时，一并释放订阅及 callback 捕获的资源。

如果集成代码必须在状态被 drop 前立即执行操作，GPUI 还提供了 `cx.on_release(...)` 来观察当前 Entity，以及 `cx.observe_release(...)` 来观察另一个 Entity。它们返回的 Subscription 也应该只保存到 release callback 所需的生命周期结束为止。

[Entity]: https://docs.rs/gpui/latest/gpui/struct.Entity.html
[WeakEntity]: https://docs.rs/gpui/latest/gpui/struct.WeakEntity.html
[Event]: /zh-CN/docs/event
