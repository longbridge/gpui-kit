---
title: Entity
description: Create, share, read, update, and observe state with GPUI Entity.
order: -2.1
---

# Entity

When several Views, handlers, or async tasks need the same state, put that state in GPUI's `Entity<T>`. A Chat, for example, can keep its messages in an `Entity<Chat>`; any code holding a clone can access the same Chat through a GPUI context.

Create the Entity with `cx.new`, read it with `read`, and change it with `update`. If `Chat` implements `Render`, its `Entity<Chat>` can also render directly as a View. Otherwise, it works as a shared state model.

```text
Entity<Chat>
    ├── read(cx)       → &Chat
    ├── update(cx, …)  → &mut Chat + Context<Chat>
    └── downgrade()    → WeakEntity<Chat>
```

Cloning an Entity copies its handle, not the state inside it. Entity access always goes through a GPUI context, allowing GPUI to coordinate updates, rendering, subscriptions, and the Entity lifecycle.

## Create an Entity

Use `cx.new` in any GPUI context:

```rs
struct Chat {
    messages: Vec<String>,
}

let chat: Entity<Chat> = cx.new(|_cx| Chat {
    messages: Vec::new(),
});
```

The closure receives `Context<Chat>`, so initialization can also create child entities or register subscriptions.

An owner keeps a strong `Entity<T>` when the child should live as long as the owner:

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

This strong ownership pattern appears throughout gpui-kit and Zed: a parent View owns the child Views or models it renders and coordinates.

## Read state

Use `read` for direct, synchronous access:

```rs
let message_count = chat.read(cx).messages.len();
```

The returned reference is tied to `cx`; copy or clone the value you need instead of trying to store the reference.

Use `read_with` when code has a generic `AppContext`, or when a closure makes the read boundary clearer:

```rs
let last_message = chat.read_with(cx, |chat, _cx| {
    chat.messages.last().cloned()
});
```

## Update state

Use `update` to obtain mutable state and its `Context<T>`:

```rs
chat.update(cx, |chat, cx| {
    chat.messages.push("Hello".into());
    cx.notify();
});
```

`cx.notify()` reports that this Entity changed. Views that rendered or observed it can then update. Mutation alone does not imply a notification, so call it when the new state should be reflected by observers or rendering.

Always use the inner `cx` passed to the update closure. It is the `Context<Chat>` for the Entity currently being updated.

:::info
Do not call `read` or `update` on an Entity while that same Entity is already being updated or rendered. GPUI prevents re-entrant access and will panic. Use the `&mut T` already provided by the current callback, or finish the current update before starting another one.
:::

## Use a WeakEntity for back references and callbacks

Cloning `Entity<T>` creates another strong handle and keeps the Entity alive. Use `WeakEntity<T>` when a relationship should not own its target, such as a child pointing back to its parent or a long-running callback referring to a View.

```rs
struct ChatSidebar {
    workspace: WeakEntity<Workspace>,
}

let workspace = cx.weak_entity();
let sidebar = cx.new(|_cx| ChatSidebar { workspace });
```

A weak handle may outlive its target. Upgrade it, or use its fallible access methods:

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

`WeakEntity::upgrade` returns `Option<Entity<T>>`; `read_with` and `update` return a `Result` because the Entity may already have been released. Zed and gpui-kit use this pattern for async tasks, callbacks, delegates, and parent references so those relationships do not accidentally keep a View alive.

## Observe changes and subscribe to Events

An Entity can coordinate with another Entity in two related ways:

- `cx.observe(&entity, ...)` runs when that Entity calls `cx.notify()`. Use it when only “this state changed” matters.
- `cx.subscribe(&entity, ...)` receives a typed [Event]. Use it when the meaning and payload of the change matter.

Store the returned `Subscription` on the subscribing Entity:

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
        // Handle the typed Event.
        cx.notify();
    }
}
```

This is the pattern used by gpui-kit Views and by larger Zed and Longbridge Pro Views. Dropping `Workspace` also drops `_subscriptions`, disconnecting its callbacks. A local `let subscription = ...` is usually wrong because it is dropped at the end of the function. Detaching or storing a View subscription in a longer-lived global owner can keep callbacks and captured resources alive after the View disappears, causing a memory leak.

See [Event] for `EventEmitter`, `emit`, and typed subscription design.

## Lifecycle

An Entity remains alive while at least one strong `Entity<T>` handle exists. When the final strong handle is dropped, GPUI releases its state; `WeakEntity<T>` handles no longer upgrade successfully.

Most cleanup should follow normal ownership:

- own child entities with `Entity<T>`;
- use `WeakEntity<T>` for non-owning links;
- keep View-level subscriptions in the same View's `_subscriptions` field;
- let dropping the View release its subscriptions and captured resources.

For integration code that must react immediately before state is dropped, GPUI also provides `cx.on_release(...)` for the current Entity and `cx.observe_release(...)` for another Entity. Store those returned subscriptions for exactly as long as the release callback is needed.

[Entity]: https://docs.rs/gpui/latest/gpui/struct.Entity.html
[WeakEntity]: https://docs.rs/gpui/latest/gpui/struct.WeakEntity.html
[Event]: /docs/event
