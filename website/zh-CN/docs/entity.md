---
title: Entity
description: 使用 GPUI Entity 创建、共享、读取、更新和观察状态。
order: -2.1
---

# Entity

当一份状态需要由多个 View、handler 或异步任务共同使用时，把它放进 GPUI 提供的 `Entity<T>`。例如，Chat 可以用 `Entity<Chat>` 保存消息；任何持有其 clone 的代码都能通过 GPUI Context 访问同一份 Chat。

使用 `cx.new` 创建 Entity，使用 `read` 读取状态，使用 `update` 修改状态。当 `Chat` 实现 [`Render`](./render) 时，`Entity<Chat>` 还可以直接作为 View 渲染；不需要渲染时，它就是一个共享状态 model。

```text
Entity<Chat>
    ├── read(cx)       → &Chat
    ├── update(cx, …)  → &mut Chat + Context<Chat>
    └── downgrade()    → WeakEntity<Chat>
```

clone Entity 会复制句柄，不会复制其中的状态。GPUI 会以同步方式增加强句柄计数；它不会 clone `T` 或其内部集合。句柄 clone 比深度复制模型轻得多，但并非零成本，而且每个保留的副本都会延长 Entity 的生命周期。Entity 只能通过 GPUI Context 访问，因此 GPUI 可以统一协调状态更新、渲染、订阅和 Entity 生命周期。

## 数据模型或持久 View

`Entity<T>` 是状态及身份的句柄；`T` **不需要**实现 `Render`。纯数据 Entity 可以保存模型或组件状态，供多个所有者读取或修改。若 `T` **实现了** `Render`，它的 `Entity<T>` 还可以放进元素树，成为持久的 View。View 的 Entity 会继续存活，而 `render` 返回的元素会在绘制时重新构建。

```text
Entity<MessageStore>（模型；无 Render）
          ↑ 读取/观察
Entity<MessagePanel>（Render View）
          ↓ render
        元素树
```

```rust
use gpui_kit::*;

struct MessageStore {
    messages: Vec<SharedString>,
}

struct MessagePanel {
    store: Entity<MessageStore>,
    _subscription: Subscription,
}

impl Render for MessagePanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.store.read(cx).messages.len();
        div().child(format!("{count} messages"))
    }
}

// 在 render 之外的 GPUI context 中：
let store = cx.new(|_| MessageStore { messages: vec![] });
let panel = cx.new(|cx| {
    let _subscription = cx.observe(&store, |_, _, cx| cx.notify());
    MessagePanel { store: store.clone(), _subscription }
});
store.update(cx, |store, cx| {
    store.messages.push("Hello".into());
    cx.notify();
});
// 将 `panel` 作为子 View；`store` 是数据，不是元素。
```

模型调用 `notify()` 后，保存的观察关系会通知 `MessagePanel`，后者再通知自己的 View。即使以后给该 View 加上缓存边界，这个显式连接也能保持内容更新。`RenderOnce` 是生成元素的另一条路径：它描述渲染时被消费的值类型组件，本身既不会创建也不要求 Entity。其生命周期见 [RenderOnce](./render-once)。

## 创建 Entity

可以在任意 GPUI context 中使用 `cx.new`：

```rs
use gpui_kit::*;

struct Chat {
    messages: Vec<SharedString>,
}

let chat: Entity<Chat> = cx.new(|_cx| Chat {
    messages: Vec::new(),
});

let same_chat = chat.clone();
assert_eq!(chat.entity_id(), same_chat.entity_id());
same_chat.update(cx, |chat, cx| {
    chat.messages.push("Hello".into());
    cx.notify();
});
assert_eq!(chat.read(cx).messages.len(), 1);
```

闭包会收到 [`Context<Chat>`](./context)，因此初始化时也可以创建子 Entity 或注册订阅。示例中的两个句柄指向同一份 `Chat`：经一个句柄修改，另一个也能读到。clone 句柄需要同步更新所有权计数，但无需复制模型及其消息。只有所有权或回调确实需要时才复制句柄，避免在高频循环里无故反复 clone。[`SharedString`](./shared-string) 也方便共享文本，但使用不同的存储机制。

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

GPUI Kit 广泛采用这种强所有权关系：父 View 持有它所渲染、协调的子 View 或 model。

### 在功能边界间传递句柄

一个功能可以把业务数据保存在一个模型 Entity 中，再将 clone 后的 `Entity<Model>` 句柄交给面板、命令和任务。这些所有者引用的是**同一份状态**，不会各自获得一份大型消息列表或文档的副本。因此，有类型的 Entity 句柄适合在同一功能的模块间传递。clone 仍会更新 GPUI 的强句柄计数，并延长模型生命周期；反向引用或不应持有模型的工作应使用 `WeakEntity`。

文档、工作区等功能特有的状态，应由该功能自己的 Entity 持有。只有值或服务的范围确实覆盖整个应用时，才使用 [`Global`](./global)，例如多个功能或窗口共同读取的偏好设置。一个类型的 Global 在应用中只有一个存储位置，不能把它当作传递逐文档 Entity 的捷径。若 Global 变化需要更新 View 输出，View 还需显式观察变化。

跨 crate 时，通过功能的公开边界暴露预期的句柄、命令或事件，不要让兄弟功能依赖其模型内部字段。只有能力具有清晰契约、且确有多个所有者时，才提取共享 crate。完整的功能与 crate 布局规则见 [Coding Guides](./coding-guides)。

### 在 `render` 外保留组件状态

GPUI Kit 的有状态组件也遵循这一原则。输入框状态只构造一次，把 Entity 存在字段中；每次 render 再将句柄传给值类型的 `Input` 元素：

```rust
use gpui_kit::*;
use gpui_kit::component::input::{Input, InputState};

struct SearchPane {
    query: Entity<InputState>,
}

impl SearchPane {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx));
        Self { query }
    }
}

impl Render for SearchPane {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child(Input::new(&self.query))
    }
}
```

如果在 `render` 里新建 `InputState`，View 重绘时就会重置草稿、选区及与焦点相关的行为。`Input` 这个值可以重建；它的状态 Entity 应继续存活。

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

`read` 接收 `&App`（`Context<T>` 可以解引用为 `App`）；`read_with` 接收任意 `AppContext`，返回闭包的结果。它们都不会再复制一份 Entity。读取范围应尽量短：正在被 update 或 render 借用的 Entity，必须等本次访问结束后才能再次读取。

## 更新状态

使用 `update` 获得可变状态和对应的 `Context<T>`：

```rs
chat.update(cx, |chat, cx| {
    chat.messages.push("Hello".into());
    cx.notify();
});
```

`cx.notify()` 表示这个 Entity 发生了变化，渲染过或正在观察它的 View 随后可以更新。仅修改字段不会自动产生通知；当新状态需要反映到观察者或界面上时，应调用它。

`update` 闭包可以返回结果。handler 可以先修改一个 Entity，结束对它的借用，再用返回值修改另一个，避免形成回调循环：

```rust
let new_count = chat.update(cx, |chat, cx| {
    chat.messages.push("Hello".into());
    cx.notify();
    chat.messages.len()
});
// 到这里 Chat 的 update 已结束；`new_count` 是普通的 usize。
```

始终使用 `update` 闭包传入的内部 `cx`。它是当前 Entity 的 `Context<Chat>`。

:::info
同一个 Entity 正在 update 或 render 时，不要再次对**同一个** Entity 调用 `read` 或 `update`。GPUI 会阻止这种重入访问并 panic。在一个 update 内更新另一个 Entity 可以正常工作，但若观察者或回调又绕回第一个 Entity，同样会间接触发重入问题。应使用回调已经提供的 `&mut T`，把少量结果复制出来，或先结束第一次 update 再开始下一次访问。以所属 Entity 已被借用的状态运行的 deferred callback 同样遵循此规则。
:::

## 使用 WeakEntity 表示反向引用和 callback

clone `Entity<T>` 会创建另一个强句柄，并让 Entity 继续存活。若父 View 持有子 Entity，又把**强引用** `Entity<ParentView>` 交给子级保存，两边就会相互持有：

```text
外部 owner → ParentView ──强引用──→ ChildView
                 ↑                    │
                 └──────强引用────────┘
```

外部 owner 被释放后，环中的强句柄仍然存在，所以两个 Entity 都不会释放。把父级句柄传给子级临时使用没有问题；当子级长期保存强句柄、父级又长期持有子级时才形成环。反向引用应保存为 `WeakEntity<ParentView>`：

```rust
use gpui_kit::*;

struct ParentView {
    child: Option<Entity<ChildView>>,
    clicks: usize,
}

struct ChildView {
    parent: WeakEntity<ParentView>,
}

impl Render for ParentView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(format!("Clicks: {}", self.clicks))
            .children(self.child.clone())
    }
}

impl Render for ChildView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().child("Notify parent").on_click(cx.listener(|this, _, _, cx| {
            let Some(parent) = this.parent.upgrade() else { return; };
            parent.update(cx, |parent, cx| {
                parent.clicks += 1;
                cx.notify();
            });
        }))
    }
}

// 在 render 之外的 GPUI context 中：
let parent = cx.new(|_| ParentView { child: None, clicks: 0 });
let child = cx.new(|_| ChildView { parent: parent.downgrade() });
parent.update(cx, |parent, cx| {
    parent.child = Some(child);
    cx.notify();
});
```

现在父级拥有子级，但子级的反向引用不会让父级继续存活：`ParentView ──强引用──→ ChildView ──弱引用──→ ParentView`。`upgrade()` 返回 `Option<Entity<ParentView>>`；父级已释放时，点击处理器直接返回。处理器在渲染之后运行，所以可以更新父级，而不会重入父级 `render` 时的借用。GPUI Kit 的嵌套弹出菜单也采用父级强持有子级、子级弱返指父级的所有权方向。

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

`WeakEntity::upgrade` 返回 `Option<Entity<T>>`；`read_with` 和 `update` 返回 `Result`，因为目标 Entity 可能已经释放。GPUI Kit 会在异步任务、callback、delegate 和父级引用中使用这一模式，避免这些关系意外延长 View 的生命周期。

`Context<Self>::spawn` 已经将 `WeakEntity<Self>` 作为异步闭包的第一个参数传入。若希望 owner 被释放时取消工作，就把返回的 [`Task`](./task) 存在所属 Entity 中；若工作应独立继续，就调用 `.detach()`。两种情况下，`await` 之后的弱引用更新失败都是正常的取消情形：等待期间 View 可能已关闭。

长期保存的闭包也可能闭合所有权环，但前提是其所有者链最终回到被强引用捕获的 Entity。GPUI 的 `cx.observe`、`cx.subscribe` 和 `cx.listener` 内部使用订阅者或 View 的弱句柄；仅仅保存它们返回的 `Subscription` 并不会自动形成强 Entity 环。额外用 `move` 捕获强 Entity 仍可能闭合这个环。`cx.processor` 与 `cx.listener` 不同：它会捕获当前 View 的强句柄，因此不要不加处理就把生成的闭包存回同一个 View。排查长期未释放的状态时，先检查强 Entity 环，以及生命周期脱离预期 owner 的订阅或任务；这些是常见的所有权线索，而非全部可能原因。

## 观察变化与订阅 Event

一个 Entity 可以通过两种相关方式与另一个 Entity 协作：

- `cx.observe(&entity, ...)` 在目标 Entity 调用 `cx.notify()` 时执行。只关心“状态变了”时使用。
- `cx.subscribe(&entity, ...)` 接收类型化 [Event]。需要知道变化的含义和数据时使用。

它们是两种独立的信号：`notify()` 不会发送 Event，`emit(event)` 本身也不会通知渲染者。某次状态变化如果既需要重绘，又需要语义事件，可以明确地各触发一次。观察者可以用收到的句柄读取目标 Entity，但仍须避免重入回调链中已被借用的 Entity。

应把 `observe` 或 `subscribe` 返回的 `Subscription` 保存在发起订阅的 Entity 上，放在 `_subscription` 字段或 `_subscriptions: Vec<Subscription>` 字段中：

```rs
enum ChatEvent {
    MessageSent,
}

impl EventEmitter<ChatEvent> for Chat {}

impl Chat {
    fn send_message(&mut self, cx: &mut Context<Self>) {
        self.messages.push("Hello".into());
        cx.notify();
        cx.emit(ChatEvent::MessageSent);
    }
}

struct Workspace {
    chat: Entity<Chat>,
    sent_count: usize,
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
            sent_count: 0,
            _subscriptions,
        }
    }

    fn on_chat_event(
        &mut self,
        _chat: Entity<Chat>,
        _event: &ChatEvent,
        cx: &mut Context<Self>,
    ) {
        self.sent_count += 1;
        cx.notify();
    }
}
```

调用 `chat.update(cx, |chat, cx| chat.send_message(cx))` 会同时发出两种信号：观察者因模型变化使 `Workspace` 失效，Event 订阅者则递增 `sent_count`。GPUI Kit 的 View 使用这一模式。`Workspace` 被释放时，`_subscriptions` 也会随之释放，callback 随即取消。局部的 `let subscription = ...` 若在函数结束时被 drop，问题是**过早取消**，并非泄漏。`Subscription::detach()` 则相反：它消耗 handle 但不取消注册，订阅可一直保留到被订阅的 Entity 释放。对长寿命 Entity 反复 detach 订阅，callback 和捕获资源可能不断累积。只有确实需要这种更长的生命周期时才 detach；它与让异步工作独立继续的 `Task::detach()` 不同。callback 强捕获 Entity 只有在所有权链回到 owner 时才形成环。

`Context<Self>::observe` 和 `subscribe` 使用订阅者的弱句柄，因此注册本身不会持有该 View。把 `Subscription` 放在订阅者中，明确它的生命周期。如果上例中两个回调都为一次操作调用 `cx.notify()`，GPUI 可以合并失效请求，但仍应判断是否真的需要两种响应。

`EventEmitter`、`emit` 和类型化订阅的设计请继续阅读 [Event]。

## 生命周期

只要还有一个强引用 `Entity<T>`，Entity 就会继续存活。最后一个强句柄被 drop 后，GPUI 会释放其状态，此时 `WeakEntity<T>` 将无法再 upgrade。

大部分清理工作应该直接跟随所有权关系：

- 使用 `Entity<T>` 持有子 Entity；
- 使用 `WeakEntity<T>` 表示非拥有关系；
- 把 View 级 Subscription 放在同一个 View 的 `_subscription` 或 `_subscriptions` 字段中；
- View 被 drop 时，一并释放订阅及 callback 捕获的资源。

如果集成代码必须在状态被 drop 前立即执行操作，GPUI 还提供了 `cx.on_release(...)` 来观察当前 Entity，以及 `cx.observe_release(...)` 来观察另一个 Entity。它们返回的 Subscription 也应该只保存到 release callback 所需的生命周期结束为止。

## Entity 身份与 View 缓存

`Entity<T: Render>` 可以直接作为子 View 嵌入。它的 `EntityId` 让 View 跨帧保有稳定身份；`cx.notify()` 会使展示它的 View 失效。状态保存在 Entity 中，而普通元素树会在绘制时重新构建。因此，保留 Entity 与缓存其渲染子树是两回事。

对于父级频繁重绘、自己却经常不变的昂贵子 View，GPUI 还提供 `child.clone().cached(style)` 和等价的 `AnyView::cached(style)`。父级必须保留同一个子 Entity，`style` 也必须给出确定的外层尺寸，因为 GPUI 在布局阶段可能跳过内容的 render。未变化的缓存子级可以重放先前的子树；通知、边界尺寸或继承的绘图上下文变化则会触发重建。缓存边界与元素状态、虚拟列表的区别，见 [View Cache](./view-cache)。

[Entity]: https://docs.rs/gpui/latest/gpui/struct.Entity.html
[WeakEntity]: https://docs.rs/gpui/latest/gpui/struct.WeakEntity.html
[Event]: /zh-CN/docs/event
