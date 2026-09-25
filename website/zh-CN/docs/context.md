---
title: Context
description: 了解 GPUI 如何提供应用、Entity、Window 与异步访问能力。
order: -2.2
---

# Context

GPUI callback 中经常出现 `window: &mut Window, cx: &mut Context<Self>`。这些参数由 GPUI 在调用期间提供，让代码访问当前窗口、当前 Entity 与整个应用，同时确保可变访问不会超出这次调用。

先区分各自的作用范围：

| 类型 | 作用范围 | 常见能力 |
| --- | --- | --- |
| `Window` | 当前系统窗口 | Focus、输入、窗口尺寸、绘制、Action 派发 |
| `Context<T>` | 当前正在更新的 `Entity<T>` | `self` 对应的 Entity、`notify`、订阅、Entity task |
| `App` | 整个应用 | Global、创建 Entity、打开窗口、应用级 Action 与 task |
| `AsyncApp` | 跨越 `await` 的前台任务句柄 | 通过一次简短的 update 重新进入 `App` 或 Entity |
| `AsyncWindowContext` | 指向某个窗口的异步句柄 | 重新进入 Entity 及其 Window |

`Context<T>` 会解引用为 `App`，所以拿到 `cx: &mut Context<T>` 时已经可以调用 App API，不需要再传一个 `&mut App`。它还知道当前是哪一个 Entity；普通的 `App` 不知道。`Window` 必须单独传入，因为同一个 Entity 可能显示在不同窗口中，而纯数据更新也可能不属于任何窗口。Window 还管理由 [ElementId](./element_id) 标识的窗口内状态。异步 context 是句柄，不是可以长期持有的 `&mut App` 或 `&mut Window` 引用。

下文使用的 Entity 专用方法定义在 [GPUI `Context<T>`](https://github.com/zed-industries/zed/blob/main/crates/gpui/src/app/context.rs) 源码中。

GPUI Kit 应用只依赖 `gpui-kit`，通过 `use gpui_kit::*;` 导入 GPUI API。创建基于组件的 View 之前调用 `gpui_kit::init(cx)`。应用级 [Global](./global) 属于 `App`；组件或功能 View 的持久状态放在 [Entity] 中。

## `window, cx` 与只有 `cx`

View 的状态属于 Entity，窗口交互属于 Window。一个方法既要修改 View 状态，又要操作这个 View 所在的窗口时，就同时接收两者。按照 GPUI 风格，它们放在参数列表最后，并保持 `window, cx` 的顺序：

```rust
fn focus_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.composer_open = true;
    self.input_focus.focus(window, cx);
    cx.notify();
}
```

Action、Event 或鼠标 callback 可能在前面带有 `action`、`event` 等参数，但运行时参数仍放在最后：

```rust
fn on_action_send_message(
    &mut self,
    action: &SendMessage,
    window: &mut Window,
    cx: &mut Context<Self>,
) {
    // ...
}
```

如果方法只处理数据，不读取 Focus、输入、窗口尺寸或其他窗口状态，就只保留最后一个 `cx` 参数：

```rust
fn clear_messages(&mut self, cx: &mut Context<Self>) {
    self.messages.clear();
    cx.notify();
}
```

当前没有 Entity，只执行应用级工作时，callback 会直接接收 `&mut App`。例如应用初始化、注册全局状态或打开第一个窗口。不要为了统一签名加入未使用的 Window；函数参数应该直接反映逻辑真正依赖的范围。

异步代码使用对应的 `AsyncApp` 或 `AsyncWindowContext`，在 `await` 以后重新进入 GPUI。Window 的具体能力见 [Window](./window)。

## 需要访问 View 的 callback

GPUI Kit `Button` 的点击 handler 接收 `(&ClickEvent, &mut Window, &mut App)`，不会直接收到 View 的 `&mut Self`。在 View 的[渲染](./render)代码中用 `cx.listener` 创建 handler；GPUI 随后会更新该 View，并把它的 `Context<Self>` 交给内层闭包。组件还提供键盘与[无障碍](./accessibility)行为：

```rust
use gpui_kit::*;
use gpui_kit::assets::Assets;
use gpui_kit::component::button::Button;

struct Counter {
    count: usize,
}

impl Render for Counter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            Button::new("increment")
                .label(format!("计数：{}", self.count))
                .on_click(cx.listener(|this, _event, _window, cx| {
                    this.count += 1;
                    cx.notify();
                })),
        )
    }
}

fn main() {
    application()
        .with_assets(Assets)
        .run(|cx| {
            init(cx);
            open_window(WindowOptions::default(), cx, |_, cx| {
                cx.new(|_| Counter { count: 0 })
            })
            .expect("failed to open window");
        });
}
```

外层 callback 的类型由按钮决定。内层闭包收到 `&mut Counter` 和 `&mut Context<Counter>`。`cx.listener` 使用 View 的弱句柄，因此保存的 handler 不会让已关闭的 View 继续存活。计数属于 Entity 状态，`cx.notify()` 告诉 GPUI 渲染新值。不要在 `render` 中反复创建 View；拥有它的 `Entity<Counter>` 在窗口打开时创建。

## 创建、读取和更新

`cx.new` 创建 [Entity]，构造闭包会拿到新 Entity 自己的 `Context<T>`：

```rs
let chat = cx.new(|cx| Chat::new(cx));

let count = chat.read(cx).message_count();

chat.update(cx, |chat, cx| {
    chat.clear_draft();
    cx.notify();
});
```

`read` 用于同步只读访问；`update` 提供 `&mut T` 和它的 Context。在 update 闭包中始终使用内层传入的 `cx`。

`App` 和 `Context<T>` 都提供 GPUI 的 `AppContext` API，因此都可以调用 `cx.new`。从 `Context<Parent>` 创建子 Entity 时，构造闭包收到的是**新建子 Entity** 的 `Context<Child>`，并非父 Entity 的 context。若子 Entity 应跨越多次渲染存活，owner 应保存返回的强 `Entity<Child>` 句柄。

:::info
`cx.notify()` 表示当前 Entity 已改变。它会安排依赖它的 View 重新 render，并触发 `observe` callback；只修改字段不会产生这些效果。
:::

不要让 `read` 返回的引用跨越 `await`，应先 clone 任务需要的少量数据。不要把 `&mut App`、`&mut Window` 或 `&mut Context<T>` 保存在 View 或 task 中；它们只在当前 GPUI 调用期间有效。

Entity 已经处于 `render` 或 `update` 时，不要再次通过 handle 访问同一个 Entity；GPUI 会阻止重入并 panic。直接使用已经传入的 `self` 和内层 `cx`。在 `cx.listener` 内也是如此：`this` 已经是当前 View。若某项工作必须等当前 callback 完成后才能更新这个 Entity，就推迟执行，不要立即通过它的句柄重入。

其他对象需要当前 Entity 的强 handle 时使用 `cx.entity()`。长期 callback 不应该阻止 View 释放时，使用 `cx.weak_entity()` 或 `downgrade()`。

## 在当前更新结束后继续

`App::defer` 在当前 effect cycle 结束后运行应用级闭包。`window.defer(cx, ...)` 在稍后提供同一个 Window。对于 Entity，`cx.defer_in(window, ...)` 会等当前更新释放借用后，重新提供同一个 View、Window 和 `Context<Self>`：

```rust
fn finish_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    cx.defer_in(window, |this, window, cx| {
        this.finish_edit_after_update(window, cx);
    });
}
```

延迟闭包也已经收到 `this: &mut Self`，不要在其中再次对同一个 Entity 调用 `update`。当 UI 树变化后需要恢复 Focus 等操作必须等当前更新结束时，再使用 defer。`window.on_next_frame(...)` 表示下一次渲染帧，是另一种时机。窗口关闭或 View 释放后，延迟工作可能不会执行，因此不要把它当成持久任务队列。

## 异步任务

`cx.spawn` 启动前台任务。从 `Context<T>` 调用时，它会提供当前 Entity 的 `WeakEntity<T>` 和 `AsyncApp`：

```rs
self._load_task = cx.spawn(async move |this, cx| {
    let messages = fetch_messages().await?;
    this.update(cx, |chat, cx| {
        chat.messages = messages;
        cx.notify();
    })?;
    anyhow::Ok(())
});
```

弱 handle 不会让 View 一直存活。如果 View 已释放，它的 `update` 会返回错误，因此需要处理或传播结果。

如果从 `App` 启动 `spawn`，就没有当前 Entity handle。闭包只会收到 `AsyncApp`；经过 `await` 后，可以用 `cx.update(|cx| ...)` 执行一次简短的应用级修改。`AsyncApp` 也提供基于闭包的 Global 访问，例如 `read_global` 和 `update_global`。它不会让异步任务持有跨越 `await` 的 `&mut App`。

任务结束时还需要同一个 Window，就使用 `spawn_in`。它提供 `AsyncWindowContext`，`update_in` 可以恢复 Window 和 Entity 访问：

```rs
cx.spawn_in(window, async move |this, cx| {
    let message = send_to_server().await?;
    this.update_in(cx, |chat, window, cx| {
        chat.messages.push(message);
        chat.input_focus.focus(window, cx);
        cx.notify();
    })?;
    anyhow::Ok(())
})
.detach();
```

异步工作结束后要更新 View 及其 Window 时，使用 `spawn_in` → `update_in`。CPU 密集工作使用 `background_spawn`；它不能直接更新 GPUI 状态，需要先把结果带回前台任务。

等待期间，`WeakEntity` 指向的 View 可能已释放，Window 也可能已关闭，因此 `update` 和 `update_in` 返回 `Result`，应予以处理。如果旧请求可能晚于新请求完成，应在应用结果之前比较 ID 或版本号。

## Task 生命周期

GPUI 的 [Task](./task) handle 被 drop 时，任务会取消：

```rs
struct Chat {
    _load_task: Task<anyhow::Result<()>>,
}
```

- 属于 View 的工作保存在 View 上，释放 View 时任务也会取消。
- 只有工作需要脱离调用者继续运行时才调用 `.detach()`。
- 替换已保存的刷新或 debounce 任务会取消旧任务。

## observe 与 subscribe

另一个 Entity 调用 `cx.notify()` 时，`observe` 会响应；Entity 发出类型化 [Event] 时，`subscribe` 会响应：

```rs
struct Chat {
    input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

// 位于 Chat::new 中：
let _subscriptions = vec![
    cx.observe(&input, |_, _, cx| cx.notify()),
    cx.subscribe(&input, |chat, input, event: &InputEvent, cx| {
        if matches!(event, InputEvent::Change) {
            chat.draft = input.read(cx).value().to_string();
            cx.notify();
        }
    }),
];
```

两者都会返回 `Subscription`。把它保存在订阅者 View 上，订阅就会与 View 拥有相同生命周期。`Subscription` 被 drop 会立即取消 callback；如果把它保存在生命周期更长的 owner 上，View 消失后 callback 与捕获的资源仍可能被保留，并造成内存泄漏。callback 还需要 `&mut Window` 时，使用 `observe_in` 或 `subscribe_in`。

发送应用事件时，先为发送事件的 Entity 类型实现 `EventEmitter<EventType>`，再从它的 `Context` 调用 `cx.emit(event)`：

```rust
struct SaveRequested;
struct Draft;

impl EventEmitter<SaveRequested> for Draft {}

impl Draft {
    fn request_save(&mut self, cx: &mut Context<Self>) {
        cx.emit(SaveRequested);
    }
}
```

父 Entity 可以用 `cx.subscribe(&draft, ...)` 处理 `SaveRequested`；订阅指定事件类型，因此同一 Entity 可以发送多种事件。`observe` 由 `cx.notify()` 触发，`subscribe` 由 `cx.emit(...)` 触发。发送事件**不会**通知 observer 或重绘发送者；若发送者的可见状态也改变，仍要调用 `cx.notify()`。反过来，`notify` 也不会发送事件。观察应用级的值应使用 `observe_global` 并保存其 `Subscription`；参见 [Global]。

## 常见错误

- Task 被 drop，导致任务过早停止。保存它，或者明确调用 `.detach()`。
- Subscription 只存在于局部变量，导致 observer 不响应。
- Task 或 callback 捕获了强 Entity handle，导致 View 无法释放。改用弱 handle。
- 在 `render` 或 `update` 内重入同一个 Entity，导致 GPUI 报告 Entity 已被借用。
- callback 只收到普通 `App`，却需要修改 View；应使用 `cx.listener`，不要手动寻找 View 再重入。
- UI 树变化后还需要操作当前 Entity；应使用 `cx.defer_in`，不要立即更新正在被借用的 Entity。
- 通过 `spawn` 启动的异步代码无法访问 Window。改用 `spawn_in` 和 `update_in`。
- 让 borrow 跨越了 `await`。先取出自有数据，之后通过 `update` 或 `update_in` 重新访问状态。
- 状态改变后没有调用 `cx.notify()`，导致界面没有更新。

无论 Context 的具体类型是什么，GPUI 通常都把参数命名为 `cx`，并把 Window 参数命名为 `window`。

[Entity]: /zh-CN/docs/entity
[Event]: /zh-CN/docs/event
[Global]: /zh-CN/docs/global
