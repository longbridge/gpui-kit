---
title: Context
description: Understand how GPUI provides application, Entity, Window, and async access.
order: -2.2
---

# Context

GPUI callbacks often receive `window: &mut Window, cx: &mut Context<Self>`. GPUI supplies these parameters for the duration of the call, giving code access to the current window, current Entity, and whole application while keeping mutable access within that call.

Start by separating the three scopes:

| Type | Scope | Common capabilities |
| --- | --- | --- |
| `Window` | Current system window | Focus, input, window bounds, drawing, Action dispatch |
| `Context<T>` | The `Entity<T>` currently being updated | The Entity for `self`, `notify`, subscriptions, Entity tasks |
| `App` | The whole application | Globals, creating Entities, opening windows, application Actions and tasks |

`Context<T>` dereferences to `App`, so code with `cx: &mut Context<T>` can already call App APIs and does not need a separate `&mut App`. Window remains separate because the same Entity may appear in different windows, while a data-only update may not belong to any window.

## `window, cx` or only `cx`

View state belongs to its Entity, while window interaction belongs to Window. A method receives both when it changes View state and operates on the window displaying that View. GPUI style places runtime parameters last, in `window, cx` order:

```rust
fn focus_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.composer_open = true;
    self.input_focus.focus(window);
    cx.notify();
}
```

An Action, Event, or pointer callback may put `action`, `event`, or similar arguments first, while keeping runtime parameters last:

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

If a method only changes data and does not read Focus, input, window bounds, or other window state, keep only the final `cx` argument:

```rust
fn clear_messages(&mut self, cx: &mut Context<Self>) {
    self.messages.clear();
    cx.notify();
}
```

When there is no current Entity and the work is application-wide, a callback receives `&mut App` directly. Application initialization, registering global state, and opening the first window are common examples. Do not add an unused Window for signature consistency; parameters should expose the scope the logic actually needs.

Async code uses the corresponding `AsyncApp` or `AsyncWindowContext` to re-enter GPUI after an `await`. See [Window](./window) for Window-specific capabilities.

## Create, read, and update

`cx.new` creates an [Entity]. Its closure receives the new Entity's own `Context<T>`:

```rs
let chat = cx.new(|cx| Chat::new(cx));

let count = chat.read(cx).message_count();

chat.update(cx, |chat, cx| {
    chat.clear_draft();
    cx.notify();
});
```

`read` provides synchronous read-only access. `update` provides `&mut T` and its Context. Always use the inner `cx` passed to the update closure.

:::info
`cx.notify()` reports that the current Entity changed. It schedules dependent views and `observe` callbacks; changing a field alone does not.
:::

Do not retain a reference from `read` across an `await`; clone the small piece of data the task needs first. Also do not read or update an Entity again while it is already inside its `render` or `update`. GPUI prevents re-entrant access and will panic. Use the `self` and inner `cx` already provided.

Use `cx.entity()` when another object needs a strong handle to the current Entity. Prefer `cx.weak_entity()` or `downgrade()` in long-lived callbacks that should not keep a View alive.

## Async work

`cx.spawn` starts a foreground task. From `Context<T>`, it supplies the current Entity as a `WeakEntity<T>` and an `AsyncApp`:

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

The weak handle does not keep the View alive. Its `update` returns an error if the View was released, so handle or propagate that result.

When `spawn` starts from `App`, there is no current Entity handle. The closure receives only `AsyncApp`; use `cx.update(|cx| ...)` to run a short application-level mutation after an `await`.

Use `spawn_in` when completion also needs the same Window. Its `AsyncWindowContext` lets `update_in` restore both Window and Entity access:

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

Zed and Longbridge Pro use this `spawn_in` → `update_in` pattern for async work that updates a View and its Window. Use `background_spawn` for CPU-heavy work; it cannot update GPUI state directly, so bring its result back to a foreground task first.

## Task lifetime

A GPUI `Task` is cancelled when its handle is dropped:

```rs
struct Chat {
    _load_task: Task<anyhow::Result<()>>,
}
```

- Store View-owned work on the View, so releasing the View cancels it.
- Call `.detach()` only when work should continue independently.
- Replacing a stored refresh or debounce task cancels the previous one, a pattern used in Zed and Longbridge Pro.

## Observe and subscribe

`observe` reacts when another Entity calls `cx.notify()`. `subscribe` reacts to a typed [Event]:

```rs
struct Chat {
    input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

// In Chat::new:
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

Both return a `Subscription`. Store it on the subscribing View so it remains active for exactly that View's lifetime. Dropping it cancels the callback immediately. Keeping it in a longer-lived owner may retain its callback and captured resources after the View disappears, causing a memory leak. Use `observe_in` or `subscribe_in` when the callback also needs `&mut Window`.

## Common mistakes

- A task stops early because its `Task` was dropped. Store it or intentionally detach it.
- An observer never runs because its `Subscription` was only a local variable.
- A View stays alive because a task or callback captured a strong Entity handle. Capture a weak handle.
- GPUI reports an Entity is already borrowed because code re-entered the same Entity during `render` or `update`.
- Async code cannot access Window because it used `spawn`; use `spawn_in` and `update_in`.
- A borrow is held across `await`; extract owned data first, then reacquire access with `update` or `update_in`.
- The UI stays stale because state changed without `cx.notify()`.

GPUI convention names every context parameter `cx`, regardless of its concrete type, and names the Window parameter `window`.

[Entity]: /docs/entity
[Event]: /docs/event
