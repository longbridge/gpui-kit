---
title: Window
description: 使用 GPUI Window 处理窗口内的输入、Focus、绘制与异步任务。
order: -2.3
---

# Window

GPUI 提供 `Window` 作为单个系统窗口的上下文。它把渲染后的 Element 树与平台输入、Focus、Action 派发、绘制和窗口控制连接起来。GPUI 只会在更新或渲染这个窗口时，把它传给 View：

```rust
impl Render for Chat {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active = window.is_window_active();

        div()
            .track_focus(&self.focus_handle)
            .when(!active, |this| this.opacity(0.8))
            .child("Chat")
    }
}
```

应用状态应保存在 [Entity](./entity) 中。一个操作属于当前窗口，或者依赖窗口当前的交互状态时，才使用 `Window`。

## Window 负责什么

常见的窗口级操作包括：

| 需求 | API |
| --- | --- |
| 读取尺寸和状态 | `bounds`、`viewport_size`、`scale_factor`、`is_window_active` |
| 管理 Focus | `focused`、`focus`、`blur`、`focus_next`、`focus_prev` |
| 从代码下达命令 | `dispatch_action` |
| 请求下一帧 | `refresh`、`on_next_frame` |
| 控制原生窗口 | `set_window_title`、`activate_window`、`remove_window` |
| 稍后继续工作 | `defer`、`spawn` |

`Window` 内部还承载布局、文本、hit testing、输入和绘制状态。大多数 View 不需要直接操作这些系统；Element 和 GPUI 会在渲染期间使用它们。

## Focus 与 Action 派发

Focus 属于某个具体的 Window。`window.focus(...)` 选择一个 `FocusHandle`，`window.focused(cx)` 返回当前 handle。键盘输入随后从获得 Focus 的 Element 建立 Dispatch Path，用它匹配 KeyBinding 并派发 Action。

```rust
fn focus_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    window.focus(&self.composer_focus, cx);
}

fn on_action_open_conversation(
    &mut self,
    action: &OpenConversation,
    window: &mut Window,
    cx: &mut Context<Self>,
) {
    self.open(action.id, cx);
    window.focus(&self.composer_focus, cx);
}
```

当按钮、命令面板、原生菜单或其他代码要下达与 KeyBinding 相同的命令时，使用 `window.dispatch_action(action.boxed_clone(), cx)`。GPUI 会记录当前 Focus，并把真正的派发推迟到当前 effect cycle 结束以后。

```rust
Button::new("open-conversation")
    .label("打开")
    .on_click(|_, window, cx| {
        window.dispatch_action(Box::new(OpenConversation { id }), cx);
    })
```

这个 Action 仍沿当前 Focus 对应的 Dispatch Path 传递。它的 `on_action_*` handler 应位于这条路径上，通常挂在当前 region 或共同 owner 上。完整的路由机制见 [Action](./action)。

## 在当前更新结束后执行

当一个操作必须等当前正在更新的 Entity 被释放以后才能执行时，使用 `window.defer`。关闭 overlay 后调整 Focus，或继续修改同一棵 UI 树的其他部分，通常都需要这样处理。

```rust
fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let composer = self.composer.clone();

    window.defer(cx, move |window, cx| {
        let focus_handle = composer.read(cx).focus_handle(cx);
        window.focus(&focus_handle, cx);
    });
}
```

在 Entity 内部，`cx.defer_in(window, ...)` 通常更方便，因为 GPUI 会再次把这个 Entity 传给 callback：

```rust
cx.defer_in(window, |this, window, cx| {
    this.rebuild_results(window, cx);
});
```

callback 已经获得 `&mut Self`，不要在里面通过 handle 对同一个 Entity 调用 `update`，否则会再次更新一个正在更新中的 Entity。Zed 与 Longbridge Pro 都会用 `defer` 和 `defer_in` 安排不能在当前 callback 中安全完成的 Focus 变化和 UI 树修改。

只有操作明确属于下一次渲染帧时才使用 `window.on_next_frame(...)`，例如推进一帧动画。`defer` 表示“当前 effect cycle 结束后”，两者不是同一个时机。

## 需要 Window 的异步任务

当任务属于当前 Entity，并且完成后需要同时访问 Entity 与 Window 时，使用 `cx.spawn_in(window, ...)`：

```rust
struct Chat {
    load_task: Option<Task<()>>,
}

fn load_conversation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.load_task = Some(cx.spawn_in(window, async move |this, mut cx| {
        let Ok(messages) = fetch_messages().await else { return };

        this.update_in(&mut cx, |this, _window, cx| {
            this.messages = messages;
            cx.notify();
        })
        .ok();
    }));
}
```

`cx.spawn_in` 提供 `WeakEntity<Self>` 与 `AsyncWindowContext`。如果 Entity 或 Window 已经消失，`update_in` 会返回错误；应传播或处理错误，不能假定它们仍然存在。

任务需要 Window、但不属于某个 Entity 时，使用 `window.spawn(cx, ...)`；不需要 Window 时使用 `cx.spawn(...)`；CPU 密集型工作使用 `cx.background_spawn(...)`。`Task` 被 drop 时任务会取消，因此当任务生命周期应跟随 View 时，把它存进 View；只有任务确实应该独立继续时才调用 `.detach()`。

## 订阅中访问 Window

Event callback 需要 `&mut Window` 时使用 `cx.subscribe_in`。例如，child 完成操作后让 owner 恢复 Focus：

```rust
struct Workspace {
    chat: Entity<Chat>,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    fn new(chat: Entity<Chat>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _subscriptions = vec![
            cx.subscribe_in(&chat, window, |_this, chat, event, window, cx| {
                if let ChatEvent::ConversationOpened = event {
                    window.focus(&chat.read(cx).focus_handle(cx), cx);
                }
            }),
        ];

        Self { chat, _subscriptions }
    }
}
```

返回的 `Subscription` 应保存在订阅它的 View 上。只存在局部变量会立刻 drop，订阅也会取消；把它存在生命周期更长的全局 owner 上，则可能在 View 消失后仍然保留 callback 与捕获的资源，造成内存泄漏。订阅所有权和多个订阅者见 [Event](./event)。

## Window 生命周期

不要保存 `&mut Window`；它是 GPUI 临时提供的上下文。需要稍后执行时，使用 `defer`、`spawn_in`，或者取得 `window.window_handle()` 后通过 GPUI 更新。handle 不会让已经关闭的窗口继续存活，因此通过 handle 执行的更新可能失败，代码应正确处理这一情况。

可以用下面几条规则判断所有权：

- 持久 UI 状态属于 Entity；
- 窗口级工作只在 callback 执行期间获得 `&mut Window`；
- 把 `Task` 和 `Subscription` 存在 View 上，让后台工作和订阅跟随 owner 生命周期；
- Focus 与 Action 派发始终使用当前这个 Window 的状态。

[Entity]: ./entity
