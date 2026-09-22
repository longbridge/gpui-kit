---
title: Context
description: 了解 GPUI 如何提供应用、Entity、Window 与异步访问能力。
order: -2.3
---

# Context

GPUI 会在可以安全访问应用状态时把 Context 传给代码。它的类型表明当前可以访问什么：

```text
App
├─ Window                         当前窗口
├─ Context<T>                    App + 当前 Entity<T>
├─ AsyncApp                      跨 await 访问 App
└─ AsyncWindowContext            AsyncApp + 当前 Window
```

| 类型 | 用途 | 通常由哪里传入 |
| --- | --- | --- |
| `&mut App` | Global、创建 Entity、打开窗口 | 应用和窗口 callback |
| `&mut Context<T>` | 当前 `T`、通知、订阅和属于它的任务 | `cx.new`、`Entity::update`、`Render` |
| `&mut Window` | Focus、输入和单个窗口的操作 | render 和窗口 callback |
| `&mut AsyncApp` | 经过 `await` 后访问 App 或 Entity | `cx.spawn` |
| `&mut AsyncWindowContext` | 经过 `await` 后访问 Entity 和 Window | `cx.spawn_in` |

`Context<T>` 会解引用为 `App`，因此也能调用应用级 API。`Window` 是独立参数，因为一个应用可以拥有多个窗口。

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

:::info
`cx.notify()` 表示当前 Entity 已改变。它会安排依赖它的 View 重新 render，并触发 `observe` callback；只修改字段不会产生这些效果。
:::

不要让 `read` 返回的引用跨越 `await`，应先 clone 任务需要的少量数据。Entity 已经处于 `render` 或 `update` 时，也不要再次通过 handle 访问同一个 Entity；GPUI 会阻止重入并 panic。直接使用已经传入的 `self` 和内层 `cx`。

其他对象需要当前 Entity 的强 handle 时使用 `cx.entity()`。长期 callback 不应该阻止 View 释放时，使用 `cx.weak_entity()` 或 `downgrade()`。

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

如果从 `App` 启动 `spawn`，就没有当前 Entity handle。闭包只会收到 `AsyncApp`；经过 `await` 后，可以用 `cx.update(|cx| ...)` 执行一次简短的应用级修改。

任务结束时还需要同一个 Window，就使用 `spawn_in`。它提供 `AsyncWindowContext`，`update_in` 可以恢复 Window 和 Entity 访问：

```rs
cx.spawn_in(window, async move |this, cx| {
    let message = send_to_server().await?;
    this.update_in(cx, |chat, window, cx| {
        chat.messages.push(message);
        chat.input_focus.focus(window);
        cx.notify();
    })?;
    anyhow::Ok(())
})
.detach();
```

Zed 和 Longbridge Pro 都大量使用 `spawn_in` → `update_in`，在异步工作结束后更新 View 及其 Window。CPU 密集工作使用 `background_spawn`；它不能直接更新 GPUI 状态，需要先把结果带回前台任务。

## Task 生命周期

GPUI 的 `Task` handle 被 drop 时，任务会取消：

```rs
struct Chat {
    _load_task: Task<anyhow::Result<()>>,
}
```

- 属于 View 的工作保存在 View 上，释放 View 时任务也会取消。
- 只有工作需要脱离调用者继续运行时才调用 `.detach()`。
- 替换已保存的刷新或 debounce 任务会取消旧任务；Zed 和 Longbridge Pro 都使用这种模式。

## observe 与 subscribe

另一个 Entity 调用 `cx.notify()` 时，`observe` 会响应；Entity 发出类型化 [Event] 时，`subscribe` 会响应：

```rs
struct Chat {
    input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

// 位于 Chat::new 中：
let subscriptions = vec![
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

## 常见错误

- Task 被 drop，导致任务过早停止。保存它，或者明确调用 `.detach()`。
- Subscription 只存在于局部变量，导致 observer 不响应。
- Task 或 callback 捕获了强 Entity handle，导致 View 无法释放。改用弱 handle。
- 在 `render` 或 `update` 内重入同一个 Entity，导致 GPUI 报告 Entity 已被借用。
- 通过 `spawn` 启动的异步代码无法访问 Window。改用 `spawn_in` 和 `update_in`。
- 让 borrow 跨越了 `await`。先取出自有数据，之后通过 `update` 或 `update_in` 重新访问状态。
- 状态改变后没有调用 `cx.notify()`，导致界面没有更新。

无论 Context 的具体类型是什么，GPUI 通常都把参数命名为 `cx`，并把 Window 参数命名为 `window`。

[Entity]: /zh-CN/docs/entity
[Event]: /zh-CN/docs/event
