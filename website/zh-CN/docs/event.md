---
title: Event
description: 使用 GPUI Event 发送类型化通知，并理解它与 Action 的关系。
order: -2.63
---

# Event

GPUI 提供 **Event**，用于在 [Entity](./entity) 之间发送类型明确的通知。Event 报告已经发生的事实；与 [**Action**](./action) 不同，它不经过 Focus、Key Context、KeyBinding 或 Dispatch Path。GPUI 也把原始鼠标和键盘输入称为 event；它们遵循另一套规则，下文分别说明。

## Action 进入，Event 返回

Action 可以触发状态变化，但 Event 的传递从状态变化之后开始：

```text
Chat 状态变化 → emit(MessageSent) → 订阅者收到 Event → Workspace 更新
```

<img class="architecture-light" src="/event-subscriptions-flow.svg?v=20260922-1" alt="Chat 发出一个 MessageSent Event，分别送达 Workspace、Activity Log 与 Telemetry 三个独立订阅者">
<img class="architecture-dark" src="/event-subscriptions-flow-dark.svg?v=20260922-1" alt="Chat 发出一个 MessageSent Event，分别送达 Workspace、Activity Log 与 Telemetry 三个独立订阅者">

- [**Action**](./action) 把意图向内传递：“发送这条消息”；
- **Event** 把结果向外报告：“这条消息已经发送”。

Command owner 处理 Action 并改变状态，再发出 Event，让 owner 或 service 响应结果，而不必依赖命令来自快捷键、按钮还是菜单。Focus 与命令派发见 [Action](./action)，快捷键匹配见 [KeyBinding](./keybinding)。

## 定义并发出 Event

先定义 Entity 可以报告的事实，并实现 [`EventEmitter`](https://docs.rs/gpui-pre/0.3.6/gpui/trait.EventEmitter.html)：

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
    cx.notify(); // 可见的草稿变化时重新渲染 Chat。
}
```

`cx.emit(...)` 会把通知放入 GPUI 的 effect 队列；当前 Entity 更新可以先结束，订阅者不是在 `finish_send` 内被直接调用。操作成功后再发出 Event。发出通知不能代替重新渲染：emitter 的可见 UI 发生变化时，还要单独调用 `cx.notify()`。Event 应按已经发生的事实命名，例如 `MessageSent`、`Saved`、`Dismissed`。`SendMessage` 这种命令式名称属于 Action。

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
        let _subscriptions = vec![cx.subscribe(&chat, |workspace, _chat, event, cx| {
            if matches!(event, ChatEvent::MessageSent { .. }) {
                workspace.refresh_conversation();
                cx.notify();
            }
        })];

        Self { chat, _subscriptions }
    }
}
```

不要只用局部变量保存返回的 `Subscription`：函数结束后它会被 drop，观察者随即断开。把 `_subscriptions` 放在 `Workspace` 上，两者便拥有相同生命周期；View 释放时，Subscription 也会一起释放并取消订阅。也不要把 View 级 Subscription 存到生命周期更长的全局 owner，否则 View 消失后 callback 与捕获的资源仍然存活，可能造成内存泄漏。

回调依次得到 owner（`&mut Workspace`）、发出通知的 `Entity<Chat>`、借用的 `&ChatEvent` 和 owner 的 [Context](./context)（`Context<Workspace>`）。它只接收**这个 Entity** 发出的对应 Event 类型；另一个 `Chat` 实例不会共用订阅者。回调需要可变的 [Window](./window) 时使用 `cx.subscribe_in(&chat, window, |workspace, chat, event, window, cx| { ... })`，并把返回的 `Subscription` 存在同一字段。只需要知道某个 Entity 发生变化、不需要有类型的 payload 时，可以使用 `cx.observe(...)`。

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

## 鼠标与键盘输入事件

`MouseDownEvent`、`MouseUpEvent`、`MouseMoveEvent`、`ScrollWheelEvent`、`KeyDownEvent` 和 `KeyUpEvent` 表示原始输入，与上文由 `EventEmitter` 发出的业务通知不同。普通 `div()` 可以用 `.on_mouse_down(MouseButton::Left, ...)`、`.on_key_down(...)` 注册处理函数；其 `InteractiveElement` 实现负责标准命中区域和派发机制。需要快捷键或菜单入口的命令使用 [Action](./action)；需要位置、按钮、修饰键或手势位移时使用原始输入事件。原始输入 callback 不会自动创建 Entity Event，除非 owner 主动调用 `cx.emit(...)`。

例如，[GPUI Kit 的 TimeField](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/time_field.rs) 把方向键绑定为自己 Key Context 中的 **Action**，用 `KeyDownEvent` 处理数字输入，并且只有时间值发生变化时才发出 `TimeFieldEvent::Change`。Owner 可以订阅这个 Event，无须知道变化来自键盘还是其他控件。匹配的 KeyBinding 可能先消费按键，使原始 `on_key_down` handler 收不到它；命令应交给 Action，而不是再写一套重复的原始按键 handler。数字输入 handler 的结构如下：

```rust
fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
    let stroke = &event.keystroke;
    if stroke.modifiers.modified() || stroke.key.chars().count() != 1 {
        return;
    }
    let Some(digit) = stroke.key.chars().next().and_then(|c| c.to_digit(10)) else {
        return; // 其他按键交给 UI 的后续处理。
    };
    window.prevent_default();
    cx.stop_propagation();
    if self.editor.input_digit(digit) {
        cx.emit(TimeFieldEvent::Change(self.editor.time));
    }
    cx.notify();
}
```

### Capture 与 bubble

原始输入有两个派发阶段。**键盘** listener 沿 Focus 对应的路径运行：capture 从根节点走向 focused node，bubble 从 focused node 返回根节点。**鼠标** listener 按绘制顺序注册，而不是沿祖先路径运行：capture 从后向前，bubble 从前向后。派发器会按这个顺序调用匹配类型的鼠标 listener；底层 listener 必须自行检查 hitbox，确认输入是否落在自身区域。普通 `.on_mouse_down(...)` 和 `.on_key_down(...)` callback 在 bubble 阶段运行；`.capture_any_mouse_down(...)` 是元素级的 capture 接口。

编写自定义 [`Element`](./element#三个阶段) 时，可在 `paint` 中用 `window.on_mouse_event` 注册 listener，再检查 `DispatchPhase`：

```rust
window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
    if phase.capture() && hitbox.is_hovered(window) {
        if event.button == MouseButton::Left {
            cx.stop_propagation();
        }
    }
});
```

这里的 `Hitbox` 应在 `prepaint` 插入。listener 在 `paint` 注册，下次渲染时会重新建立。GPUI Kit 的 [Carousel scroll mask](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/carousel/scroll_mask.rs) 使用 capture 处理指针与滚轮手势：在 carousel 的轴上消费移动，另一方向则交给外层滚动容器。`Hitbox::is_hovered` 用于普通指针命中，`should_handle_scroll` 还会考虑滚动遮挡。普通控件优先使用元素提供的 fluent handler；需要自己的 hitbox 或阶段处理时，再使用 `window.on_mouse_event`。

### `stop_propagation` 与 `prevent_default`

| 调用 | 作用 | 典型场景 |
| --- | --- | --- |
| `cx.stop_propagation()` | 停止**当前派发**中后续 listener：鼠标 bubble 阶段不再交给后方 surface，键盘 bubble 阶段不再交给祖先；在 capture 阶段调用时，还会阻止剩余的 capture listener 与 bubble 阶段。 | 子控件已经消费拖动或按键。 |
| `window.prevent_default()` | 标记当前输入事件的默认行为已被阻止。GPUI 用它控制 mouse down 时父元素获得 Focus 等内置行为。 | 子控件处理鼠标按下，但不想改变父元素 Focus。 |

两者互不替代：停止传播不会自动取消默认 Focus 行为；取消默认行为也不会自动停止其他 handler。GPUI 在每次输入派发开始时重置这两个标志。`prevent_default` 只控制检查该标志的 GPUI 内置行为，不代表普遍的浏览器式或操作系统事件取消。确定当前控件确实需要处理该输入后再调用它们。Action 刚好采用另一种默认策略：Action handler 默认停止向上冒泡，需要上层继续尝试时才调用 `cx.propagate()`。输入 callback 接收 `&mut Window`，在其中调用 `window.prevent_default()`。

排查 handler 没有生效时，依次查看事件阶段、Focus path、content mask、hitbox 行为和 z 顺序。注册成功的 listener 仍可能因为前方元素遮挡，或事件走了另一条 Focus path 而收不到输入。
