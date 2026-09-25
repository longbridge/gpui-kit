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
Chat state changes → emit(MessageSent) → Subscriber receives event → Workspace updates
```

<img class="architecture-light" src="/event-subscriptions-flow.svg?v=20260922-1" alt="Chat 发出一个 MessageSent Event，分别送达 Workspace、Activity Log 与 Telemetry 三个独立订阅者">
<img class="architecture-dark" src="/event-subscriptions-flow-dark.svg?v=20260922-1" alt="Chat 发出一个 MessageSent Event，分别送达 Workspace、Activity Log 与 Telemetry 三个独立订阅者">

- [**Action**](./action) 把意图向内传递：“发送这条消息”；
- **Event** 把结果向外报告：“这条消息已经发送”。

Command owner 处理 Action 并改变状态，再发出 Event，让 owner 或 service 响应结果，而不必依赖命令来自快捷键、按钮还是菜单。Focus 与命令派发见 [Action](./action)，快捷键匹配见 [KeyBinding](./keybinding)。

## 从零完成一次 Event 传递

下面的小程序有两个 Entity。`Chat` 保存发送数量并发出带类型的事实；`Workspace` 持有 `Chat`，订阅这个具体的 Entity，并显示最新数量。点击按钮后，`Chat` 更新状态，订阅回调再更新 `Workspace`。

沿用[入门指南](./getting-started)中已经依赖 `gpui-kit` 的项目，把以下完整程序写入该项目的 `src/main.rs`，然后在项目目录运行 `cargo run`。窗口开始显示 **Messages sent: 0**；每次点击 **Send**，数量应增加一。

```rust
use gpui_kit::*;
use gpui_kit::assets::Assets;
use gpui_kit::component::button::Button;

#[derive(Clone, Debug)]
enum ChatEvent {
    MessageSent { total: usize },
}

struct Chat {
    sent: usize,
}

impl EventEmitter<ChatEvent> for Chat {}

impl Chat {
    fn send(&mut self, cx: &mut Context<Self>) {
        self.sent += 1;
        cx.emit(ChatEvent::MessageSent { total: self.sent });
    }
}

struct Workspace {
    chat: Entity<Chat>,
    shown_total: usize,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    fn new(cx: &mut Context<Self>) -> Self {
        let chat = cx.new(|_| Chat { sent: 0 });
        let subscription = cx.subscribe(&chat, |workspace, _chat, event, cx| {
            match event {
                ChatEvent::MessageSent { total } => workspace.shown_total = *total,
            }
            cx.notify(); // The count shown by Workspace changed.
        });

        Self {
            chat,
            shown_total: 0,
            _subscriptions: vec![subscription],
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_4()
            .child(format!("Messages sent: {}", self.shown_total))
            .child(Button::new("send").label("Send").on_click(cx.listener(
                |workspace, _, _, cx| {
                    workspace.chat.update(cx, |chat, cx| chat.send(cx));
                },
            )))
    }
}

fn main() {
    gpui_kit::application().with_assets(Assets).run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(Workspace::new)
        })
        .expect("failed to open window");
    });
}
```

启动代码先调用 `gpui_kit::init`，再由 `open_window` 创建窗口。按钮回调更新 `Chat`，状态变化和 `cx.emit` 都发生在 `Chat` 内；订阅者读取 Event 的内容，并在修改了 `Workspace` 显示的数量后调用 `cx.notify()`。

`cx.emit(...)` 将 Event 放进 effect 队列；当前 Entity 更新可以先结束，订阅回调并不是 `Chat::send` 内部的一次直接函数调用。操作成功后再 emit。如果发出 Event 的 Entity 自己也显示了变化后的状态，还要在那个 Entity 中调用 `cx.notify()`；`emit` 不会请求重绘。上例的 `Chat` 是不渲染 UI 的状态 Entity。Event 名称应表达事实，例如 `MessageSent`、`Saved`、`Dismissed`；`SendMessage` 则表示命令。

### 把 Action 接到这个 Event

上面的完整示例从按钮 callback 开始。要验证 [Action](./action) → 状态 → Event → 订阅的整条路径，请在**同一个** `src/main.rs` 中作四处改动。原有的 `Chat`、`ChatEvent`、`Chat::send` 和订阅回调保持不变。

1. 在 import 后加入 `actions!(chat, [SendMessage]);`。在 `Workspace` 的 `chat` 字段旁增加 `focus: FocusHandle,`。
2. 让 `Workspace::new` 在 `cx` 之前接收 `window: &mut Window`。在原有的 `let chat = ...` 前创建并聚焦句柄，再把 `focus,` 放入返回的 `Self`：

   ```rust
   let focus = cx.focus_handle().tab_stop(true);
   focus.focus(window, cx);
   ```

3. 在 `impl Workspace` 中增加方法：

   ```rust
   fn on_send_message(&mut self, _: &SendMessage, _: &mut Window, cx: &mut Context<Self>) {
       self.chat.update(cx, |chat, cx| chat.send(cx));
   }
   ```

   在 `render` 的**最外层** `div()` 上、添加子元素之前接上三个调用，然后只用下面的 callback 替换按钮原有的 `.on_click(...)`：

   ```rust
   .track_focus(&self.focus)
   .key_context("Chat")
   .on_action(cx.listener(Self::on_send_message))

   .on_click(cx.listener(|workspace, _, window, cx| {
       workspace.focus.dispatch_action(&SendMessage, window, cx);
   }))
   ```

4. 在 `main` 中调用 `gpui_kit::init(cx)` 后绑定 Enter，并让 `open_window` 闭包把窗口传入构造函数：

   ```rust
   cx.bind_keys([KeyBinding::new("enter", SendMessage, Some("Chat"))]);
   gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
       cx.new(|cx| Workspace::new(window, cx))
   })
   .expect("failed to open window");
   ```

再次运行 `cargo run`。当 Workspace 区域拥有 Focus 时按 **Enter**，再点击 **Send**。两种输入都会把同一个 `SendMessage` Action 派发给 `Workspace::on_send_message`；该 handler 更新 `Chat`，`Chat::send` 增加 `sent` 并发出 `MessageSent`，保留的订阅再更新 `Workspace::shown_total`。可见计数应随每次输入增加一次。作为核对，可暂时删去 `Chat::send` 中的 `cx.emit(...)`：内部 `sent` 仍增加，但由于 `Workspace` 订阅的是 Event，可见计数不再变化。继续下一练习前恢复 `emit`。若 Enter 无效，检查焦点句柄是否挂在渲染出的容器上，以及 `Chat` Key Context 是否存在；若按钮无效，检查 Action handler 与派发路径。

## Subscription 的生命周期与归属

`cx.subscribe(&chat, callback)` 返回一个 `Subscription`。应像上例一样由订阅方保存它。丢弃该句柄会断开回调，因此只存在于 `new` 内的局部句柄会在函数返回时悄悄停止接收 Event。`Workspace` 被释放时，它保存的句柄也会释放。克隆 `Entity<Chat>` 只是保留事件源的引用，不会保留订阅关系。View 专用的订阅应随 View 保存，而不是放进生命周期更长的全局状态。

回调参数依次是 `&mut Workspace`、发出通知的 `Entity<Chat>`、`&ChatEvent`、`&mut Context<Workspace>`。Event 内容只在回调期间借用；需要之后使用的数据应复制或克隆。订阅绑定到一个事件源 Entity 和一种 Event 类型，不会自动接收其他 `Chat` 或应用中所有 Event。多个 owner 可以分别订阅同一个事件源。要提前停止某项订阅，从 owner 的集合中移除并丢弃其句柄。

如果回调还需要 `&mut Window`，使用 `cx.subscribe_in(&chat, window, callback)`。它的回调有五个参数：owner、`&Entity<Chat>`、`&ChatEvent`、window 和 context；返回的 `Subscription` 同样需要保存。只需要知道 Entity 有变化，不需要类型化 Event 内容时，使用 `cx.observe(&chat, ...)`。Event 表达主动发出的业务事实；`cx.notify()` 表示 Entity 需要更新，不会产生 `ChatEvent`。

订阅没有响应时，检查发出 Event 的是否为同一个 Entity、`EventEmitter` 实现的类型是否与发出的类型一致、句柄是否还在，以及成功修改状态后是否执行了 `cx.emit`。如果回调执行了但界面仍是旧值，检查哪个 Entity 负责渲染该值，并在它的 context 上调用 `cx.notify()`。

### 动手比较 `observe` 与 Event 订阅

如果做过上面的 Action 延伸练习，先恢复原始的完整 `Chat`/`Workspace` 示例，再运行它：每次点击，界面上的计数增加一。然后在这个程序中做三处改动：

1. 删除 `ChatEvent` 枚举和 `impl EventEmitter<ChatEvent> for Chat {}`，将 `Chat::send` 改成下面的方法。这个版本更新状态并调用 `notify`，不发出 Event。

   ```rust
   fn send(&mut self, cx: &mut Context<Self>) {
       self.sent += 1;
       cx.notify();
   }
   ```

2. 在 `Workspace::new` 中，用下面的 observer 替换 `cx.subscribe(...)` 代码块。返回的句柄继续保存在现有的 `_subscriptions` 字段中。

   ```rust
   let subscription = cx.observe(&chat, |workspace, chat, cx| {
       workspace.shown_total = chat.read(cx).sent;
       cx.notify();
   });
   ```

3. 再运行程序并点击 **Send**，计数仍然增加。observer 收到发生变化的 `Entity<Chat>`，再读取当前状态；它没有 `ChatEvent` 或 Event payload。暂时删掉 `Chat::send` 中的 `cx.notify()` 并重新运行：虽然 `Chat.sent` 变化了，但 observer 不会执行，显示的计数保持零。最后恢复 `cx.notify()`。

只需知道某个 Entity 有变化，且可以从中读取当前状态时，适合使用 `observe`；生产者要主动报告一种带类型的事实时，适合使用 `subscribe`。两者都返回需要由 owner 保存的 `Subscription`。单独调用 `cx.emit` 不会通知 `observe` callback；单独调用 `cx.notify()` 也不会发出类型化 Event。

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
        return; // Let the UI handle other keys.
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

### 动手观察父子区域的鼠标处理顺序

回到上面的完整 Event 版本示例。在 `Workspace` 中加入 `parent_hits: usize`、`child_hits: usize` 两个字段，并在 `Workspace::new` 中把它们都初始化为 `0`。然后将 `Render` 实现替换为下面的代码：

```rust
impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_4()
            .child(format!("Messages sent: {}", self.shown_total))
            .child(Button::new("send").label("Send").on_click(cx.listener(
                |workspace, _, _, cx| {
                    workspace.chat.update(cx, |chat, cx| chat.send(cx));
                },
            )))
            .child(format!(
                "Parent: {} | Child: {}",
                self.parent_hits, self.child_hits
            ))
            .child(
                div()
                    .p_4()
                    .bg(rgb(0xd0d7de))
                    .on_mouse_down(MouseButton::Left, cx.listener(|workspace, _, _, cx| {
                        workspace.parent_hits += 1;
                        cx.notify();
                    }))
                    .child("Parent surface")
                    .child(
                        div()
                            .p_4()
                            .bg(rgb(0x8ecae6))
                            .on_mouse_down(MouseButton::Left, cx.listener(
                                |workspace, _, _, cx| {
                                    workspace.child_hits += 1;
                                    cx.notify();
                                    // Uncomment to stop the parent handler:
                                    // cx.stop_propagation();
                                },
                            ))
                            .child("Child surface"),
                    ),
            )
    }
}
```

点击蓝色的 **Child surface** 一次，两个计数都会增加。child listener 在绘制阶段较晚注册，所以在鼠标 bubble 阶段先运行；parent 的 hitbox 同样包含该位置，因而随后运行。点击子区域外的灰色部分，只增加 parent 计数。取消 child 回调中 `cx.stop_propagation()` 那一行的注释并重新运行，再点击 child 时只有 child 计数增加；点击灰色部分仍然只增加 parent 计数。点击 **Send** 只改变消息计数，不影响这两个命中计数。

这里的两个区域为了练习而嵌套，但 GPUI 根据窗口当前帧的 listener 顺序及各自的 hitbox 派发鼠标事件，**不会**像 DOM 一样沿祖先链路由。即使两个重叠区域在元素树上没有父子关系，后绘制的 surface 仍可能是第一个 bubble listener。只有子区域确实需要独占这次输入时，才调用 `stop_propagation()` 阻止后续 listener。

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
