---
title: Entity
description: Create, share, read, update, and observe state with GPUI Entity.
order: -2.1
---

# Entity

When several Views, handlers, or async tasks need the same state, put that state in GPUI's [`Entity<T>`][Entity]. A Chat, for example, can keep its messages in an `Entity<Chat>`; any code holding a clone can access the same Chat through a GPUI [Context](./context).

Create the Entity with `cx.new`, read it with `read`, and change it with `update`. If `Chat` implements [`Render`](./render), its `Entity<Chat>` can also render directly as a View. Otherwise, it works as a shared state model.

```text
Entity<Chat>
    ├── read(cx)       → &Chat
    ├── update(cx, …)  → &mut Chat + Context<Chat>
    └── downgrade()    → WeakEntity<Chat>
```

Cloning an Entity copies its handle, not the state inside it. GPUI increments a synchronized strong-handle count; it does not clone `T` or collections inside it. A handle clone is much smaller than a deep model clone, but it is not free, and every retained clone extends the Entity's lifetime. Entity access always goes through a GPUI context, allowing GPUI to coordinate updates, rendering, subscriptions, and the Entity lifecycle.

## Data model or persistent View

`Entity<T>` is a state and identity handle; `T` does **not** need to implement `Render`. A data-only Entity can hold a model or component state, and several owners can read or update it. When `T` **does** implement `Render`, its `Entity<T>` can also be placed in the element tree as a persistent View. The View's Entity survives while the elements returned by `render` are rebuilt for drawing.

```text
Entity<MessageStore> (model; no Render)
          ↑ read/observe
Entity<MessagePanel> (Render View)
          ↓ render
     element tree
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

// In a GPUI context, outside render:
let store = cx.new(|_| MessageStore { messages: vec![] });
let panel = cx.new(|cx| {
    let _subscription = cx.observe(&store, |_, _, cx| cx.notify());
    MessagePanel { store: store.clone(), _subscription }
});
store.update(cx, |store, cx| {
    store.messages.push("Hello".into());
    cx.notify();
});
// Add `panel` as a child View; `store` is data, not an element.
```

The model's `notify()` reaches `MessagePanel` through the stored observation, which notifies the View in turn. This explicit connection also keeps the View current if it is later placed behind a cache boundary. `RenderOnce` is another route to elements: it describes a value-like component consumed during rendering and does not, by itself, create or require an Entity. See [RenderOnce](./render-once) for that lifecycle.

## Create an Entity

Use `cx.new` in any GPUI context:

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

The closure receives [`Context<Chat>`](./context), so initialization can also create child entities or register subscriptions. Both handles in the example refer to the same `Chat`: an update through one is visible through the other. Cloning the handle performs synchronized ownership bookkeeping, but avoids copying the model and its messages. Reuse a handle when ownership or a callback needs it; avoid cloning it repeatedly inside a hot loop without a reason. [`SharedString`](./shared-string) also makes sharing text inexpensive, through a different storage mechanism.

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

This strong ownership pattern appears throughout GPUI Kit: a parent View owns the child Views or models it renders and coordinates.

### Pass a handle across a feature

A feature can keep its business data in one model Entity and pass cloned `Entity<Model>` handles to its panels, commands, and tasks. Those owners refer to **the same state**; they do not each receive a copy of a large message list or document. This makes a typed Entity handle a useful boundary between modules of one feature. A clone still updates GPUI's strong-handle count and extends the model's lifetime, so use `WeakEntity` for back references or work that should not retain it.

Keep document, workspace, and other feature-specific state owned by that feature's Entity. Use a [`Global`](./global) for a value or service whose scope really is the whole application, such as a shared preference read by multiple features or windows. A Global is one app-wide slot for its type; it is not a shortcut for passing a per-document Entity. Views also need an explicit global observation when changes must update their output.

Across crates, expose the feature's intended handle, command, or event through its public boundary rather than making sibling features depend on its internal model fields. Move a capability into a shared crate only when it has a coherent contract and more than one real owner. The [Coding Guides](./coding-guides) cover the full feature and crate layout.

### Retain component state outside `render`

GPUI Kit's stateful components follow the same rule. Construct an input's state once, store its Entity, and pass the handle to the value-like `Input` element on each render:

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

Creating `InputState` inside `render` would reset the draft, selection, and focus-related behavior as the View redraws. The `Input` value can be rebuilt; its state Entity should survive.

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

`read` takes `&App` (a `Context<T>` dereferences to `App`). `read_with` accepts any `AppContext` and returns what its closure produces. Neither creates a second copy of the Entity. Keep either read short: an Entity already leased for an update or render cannot be read again until that access ends.

## Update state

Use `update` to obtain mutable state and its `Context<T>`:

```rs
chat.update(cx, |chat, cx| {
    chat.messages.push("Hello".into());
    cx.notify();
});
```

`cx.notify()` reports that this Entity changed. Views that rendered or observed it can then update. Mutation alone does not imply a notification, so call it when the new state should be reflected by observers or rendering.

An `update` closure may return a result. This lets a handler change one Entity, finish that borrow, and then use the result to update another without creating a callback cycle:

```rust
let new_count = chat.update(cx, |chat, cx| {
    chat.messages.push("Hello".into());
    cx.notify();
    chat.messages.len()
});
// The Chat update has ended here; `new_count` is an ordinary usize.
```

Always use the inner `cx` passed to the update closure. It is the `Context<Chat>` for the Entity currently being updated.

:::info
Do not call `read` or `update` on an Entity while that **same** Entity is already being updated or rendered. GPUI prevents re-entrant access and will panic. Updating a different Entity inside an update can work, but an observer or callback that reaches back to the first Entity creates the same problem indirectly. Use the `&mut T` already provided by the callback, copy out a small result, or finish the first update before starting the next. This also applies to deferred callbacks that run with their owning Entity already borrowed.
:::

## Use a WeakEntity for back references and callbacks

Cloning `Entity<T>` creates another strong handle and keeps the Entity alive. That matters when a parent View owns a child Entity and passes its **strong** `Entity<ParentView>` to the child. If the child stores it, both sides own each other:

```text
outside owner → ParentView ──strong──→ ChildView
                  ↑                    │
                  └──────strong────────┘
```

Dropping the outside owner does not remove either remaining strong handle, so neither Entity is released. Passing a parent handle to a child is safe when the child only uses it temporarily; the cycle appears when the child retains that strong handle while the parent retains the child. Store a [`WeakEntity<ParentView>`][WeakEntity] for the back reference instead:

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

// In a GPUI context, outside render:
let parent = cx.new(|_| ParentView { child: None, clicks: 0 });
let child = cx.new(|_| ChildView { parent: parent.downgrade() });
parent.update(cx, |parent, cx| {
    parent.child = Some(child);
    cx.notify();
});
```

Now the parent owns the child, but the child's link back does not keep the parent alive: `ParentView ──strong──→ ChildView ──weak──→ ParentView`. `upgrade()` returns `Option<Entity<ParentView>>`; when the parent has gone away, the click handler simply returns. The handler runs after rendering, so it can update the parent without re-entering the parent's `render` borrow. GPUI Kit's nested popup menus use the same strong child, weak parent ownership direction.

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

`WeakEntity::upgrade` returns `Option<Entity<T>>`; `read_with` and `update` return a `Result` because the Entity may already have been released. GPUI Kit uses this pattern for async tasks, callbacks, delegates, and parent references so those relationships do not accidentally keep a View alive.

`Context<Self>::spawn` already supplies a `WeakEntity<Self>` as the first async argument. Hold the returned [`Task`](./task) in the owning Entity when dropping that owner should cancel the work; call `.detach()` when it should continue independently. In either case, handle a failed weak update after an `await` as normal cancellation, since the View may have closed while the work was running.

Retained closures can close an ownership cycle too, but only when their owner chain leads back to a strongly captured Entity. GPUI's `cx.observe`, `cx.subscribe`, and `cx.listener` use weak subscriber/View handles internally; storing one of their `Subscription`s does not by itself make a strong Entity cycle. An additional `move` capture of a strong Entity can still complete that cycle. `cx.processor` differs from `cx.listener`: it captures a strong handle to its View, so do not store its closure back in that same View without breaking the ownership loop. When investigating retained state, check both strong Entity cycles and subscriptions or tasks whose lifetime was detached from their intended owner; these are common ownership paths to inspect, not an exhaustive list of causes.

## Observe changes and subscribe to Events

An Entity can coordinate with another Entity in two related ways:

- `cx.observe(&entity, ...)` runs when that Entity calls `cx.notify()`. Use it when only “this state changed” matters.
- `cx.subscribe(&entity, ...)` receives a typed [Event]. Use it when the meaning and payload of the change matter.

They are separate signals: `notify()` does not emit an Event, and `emit(event)` does not by itself notify renderers. A state change that needs both a redraw and a semantic event can do both deliberately, usually once each. An observer can inspect the observed Entity with the handle it receives; it must still avoid re-entering an Entity already borrowed by the callback chain.

Store the [`Subscription`](https://docs.rs/gpui-pre/0.3.6/gpui/struct.Subscription.html) returned by `observe` or `subscribe` on the subscribing Entity, in a `_subscription` field or a `_subscriptions: Vec<Subscription>` field:

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

Calling `chat.update(cx, |chat, cx| chat.send_message(cx))` now sends both signals: the observer invalidates `Workspace` for the model change, while the Event subscriber increments `sent_count`. This is the pattern used by GPUI Kit Views. Dropping `Workspace` also drops `_subscriptions` and cancels its callbacks. Dropping a local `let subscription = ...` at the end of the function **cancels too early**; it is not a leak. `Subscription::detach()` has the opposite effect: it consumes the handle without canceling the registration, which can remain until the subscribed Entity is dropped. Repeatedly detaching subscriptions to a long-lived Entity can accumulate callbacks and captured resources. Detach only when that longer lifetime is intended; it differs from `Task::detach()`, which lets asynchronous work continue independently. A callback's strong Entity capture becomes a cycle only if the ownership chain leads back to the owner.

`Context<Self>::observe` and `subscribe` use a weak handle to the subscriber, so the registration does not itself own that View. Keep the `Subscription` in the subscriber for an explicit lifetime. If both callbacks above call `cx.notify()` for one operation, GPUI can coalesce invalidations, but decide whether both responses are actually needed.

See [Event] for `EventEmitter`, `emit`, and typed subscription design.

## Lifecycle

An Entity remains alive while at least one strong `Entity<T>` handle exists. When the final strong handle is dropped, GPUI releases its state; `WeakEntity<T>` handles no longer upgrade successfully.

Most cleanup should follow normal ownership:

- own child entities with `Entity<T>`;
- use `WeakEntity<T>` for non-owning links;
- keep View-level subscriptions in the same View's `_subscription` or `_subscriptions` field;
- let dropping the View release its subscriptions and captured resources.

For integration code that must react immediately before state is dropped, [Context](./context) also provides `cx.on_release(...)` for the current Entity and `cx.observe_release(...)` for another Entity. Store those returned subscriptions for exactly as long as the release callback is needed.

## Entity identity and view caching

An `Entity<T: Render>` can be embedded directly as a child View. Its `EntityId` gives that View a stable identity across frames; `cx.notify()` invalidates the views that display it. The state persists in the Entity, while the ordinary element tree is rebuilt for a draw. Keeping an Entity is therefore different from caching its rendered subtree.

For an expensive child that often stays unchanged while its parent redraws, GPUI also exposes `child.clone().cached(style)` and the equivalent `AnyView::cached(style)`. The parent must retain the same child Entity, and `style` must provide a definite outer size because GPUI can skip rendering the contents during layout. A clean cached child may replay its previous subtree; notifications, changed bounds or inherited drawing context cause a rebuild. See [View Cache](./view-cache) for the exact boundary and how it differs from element state and virtualization.

[Entity]: https://docs.rs/gpui-pre/0.3.6/gpui/struct.Entity.html
[WeakEntity]: https://docs.rs/gpui-pre/0.3.6/gpui/struct.WeakEntity.html
[Event]: /docs/event
