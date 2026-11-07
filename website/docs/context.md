---
title: Context
description: Understand how GPUI provides application, Entity, Window, and async access.
order: -2.2
---

# Context

GPUI callbacks often receive `window: &mut Window, cx: &mut Context<Self>`. GPUI supplies these parameters for the duration of the call, giving code access to the current window, current Entity, and whole application while keeping mutable access within that call.

Start by separating the scopes:

| Type | Scope | Common capabilities |
| --- | --- | --- |
| `Window` | Current system window | Focus, input, window bounds, drawing, Action dispatch |
| `Context<T>` | The `Entity<T>` currently being updated | The Entity for `self`, `notify`, subscriptions, Entity tasks |
| `App` | The whole application | Globals, creating Entities, opening windows, application Actions and tasks |
| `AsyncApp` | A handle for foreground work across `await` | Re-enter `App` or an Entity in a short update |
| `AsyncWindowContext` | An async handle for one window | Re-enter an Entity together with its Window |

Read the table as a progression: `App` is available first in `application().run`; opening a window supplies `Window`; creating an Entity supplies its `Context<T>` whenever GPUI builds or updates that Entity. A foreground task receives an async handle instead of borrowing either synchronous context across an `await`.

| Where code runs | Parameters GPUI supplies | Use it for |
| --- | --- | --- |
| `application().run`, application callbacks | `&mut App` | Initialize the kit, set Globals, create Entities, open windows |
| Window builder, element or component callback | `&mut Window`, `&mut App` | Work with that window and application state; use `cx.listener` when the callback needs its owning View |
| `Render` or an Entity update | `&mut Self`, `&mut Context<Self>`; `Render` and `update_in` also receive `&mut Window` | Read or change the current View; call `notify` when its rendered state changes |
| `App::spawn` / `Context<T>::spawn` task | `&mut AsyncApp`, plus a `WeakEntity<T>` for Entity tasks | Resume short application or Entity operations after `await` |
| `Context<T>::spawn_in` task | `&mut AsyncWindowContext` and `WeakEntity<T>` | Resume an operation that also needs the original Window |

`Context<T>` dereferences to `App`, so code with `cx: &mut Context<T>` can already call App APIs and does not need a separate `&mut App`. It also knows which Entity is current; plain `App` does not. Window remains separate because the same Entity may appear in different windows, while a data-only update may not belong to any window. Window also owns per-window state keyed by [ElementId](./element_id). Async contexts are handles, not long-lived `&mut App` or `&mut Window` borrows.

The [GPUI `Context<T>` source](https://docs.rs/crate/gpui-pre/{{gpui_pre_version}}/source/src/app/context.rs) defines the entity-specific methods used below.

GPUI Kit applications depend on `gpui-kit` and import GPUI through `use gpui_kit::*;`. Call `gpui_kit::init(cx)` before creating component-backed Views. An application-wide [Global](./global) lives on `App`; a component or feature View keeps retained state in an [Entity].

## Open a URL in the default browser

Use `cx.open_url(...)` to hand a URL to the platform's default browser. It is an `App` API, so it also works when `cx` is a `Context<T>` or the `&mut App` supplied to a button callback:

```rust
use gpui_kit::component::button::Button;

Button::new("open-docs")
    .label("Open docs")
    .on_click(|_, _, cx| cx.open_url("https://gpui-kit.com/docs"))
```

This opens an external browser; use [WebView](./webview) when browser content must live inside a GPUI window. `open_url` returns no completion result. GPUI Kit's `Link` with an `href` also calls `cx.open_url(...)` when clicked.

## `window, cx` or only `cx`

View state belongs to its Entity, while window interaction belongs to Window. A method receives both when it changes View state and operates on the window displaying that View. GPUI style places runtime parameters last, in `window, cx` order:

```rust
fn focus_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.composer_open = true;
    self.input_focus.focus(window, cx);
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

## A callback that needs its View

A GPUI Kit `Button` click handler receives `(&ClickEvent, &mut Window, &mut App)`. It does not receive the View as `&mut Self`. Build the callback with `cx.listener` while [rendering](./render) a View; GPUI will update that View and pass its `Context<Self>` to the inner closure. The component also supplies keyboard and [accessibility](./accessibility) behavior:

To try the complete example, replace `examples/hello_world/src/main.rs` in this repository's existing `hello_world` package with the following code. From the repository root, run `cargo run -p hello_world --bin hello_world`.

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
                .label(format!("Count: {}", self.count))
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

The window first shows `Count: 0`. Click the button once and its label should become `Count: 1`; each further click increments it by one. This checks that the listener mutates the existing `Counter` Entity and that `cx.notify()` makes the new value visible. Restore your original `main.rs` after the exercise if you want to keep the package's previous example.

The outer callback type belongs to the button. The inner closure receives `&mut Counter` and `&mut Context<Counter>`. `cx.listener` uses a weak handle to the View, so a stored handler does not keep a closed View alive. The count is Entity state, and `cx.notify()` tells GPUI to render its new value. Do not create the View again inside `render`; the owning `Entity<Counter>` is created when the window opens.

## Create, read, and update

`cx.new` creates an [Entity]. Its closure receives the new Entity's own `Context<T>`. A strong `Entity<T>` handle keeps it alive. Use the handle with `read` for a synchronous borrowed view of its data, or `update` to receive `&mut T` and that Entity's own context:

```rs
struct Draft {
    text: String,
}

fn edit_draft(cx: &mut App) -> Entity<Draft> {
    let draft = cx.new(|_| Draft { text: String::new() });
    let was_empty = draft.read(cx).text.is_empty();

    if was_empty {
        draft.update(cx, |draft, cx| {
            draft.text.push_str("Hello");
            cx.notify();
        });
    }

    draft
}
```

The `read` reference cannot outlive the `App` borrow. Complete the read before requesting mutable access with `update`; copy or clone the small amount of data you need later. A strong handle's `update` returns the closure's value directly. A `WeakEntity<T>` can disappear, so its `update` and `update_in` return `Result` instead. Always use the inner `cx` passed to the update closure when notifying or using Entity-specific methods.

`cx.new` can be called through `App` or `Context<T>` because both implement GPUI's `AppContext` API. From a `Context<Parent>`, the construction closure receives a **new** `Context<Child>`. It is not the parent's context. Keep the returned strong `Entity<Child>` in its owner when the child should survive across renders.

:::info
`cx.notify()` reports that the current Entity changed. It schedules dependent views and `observe` callbacks; changing a field alone does not.
:::

`notify` is unnecessary for a read. Avoid calling it unconditionally from `render`, which can schedule repeated renders. When one action changes several related fields, finish the change and notify once.

Do not retain a reference from `read` across an `await`; clone the small piece of data the task needs first. Do not store `&mut App`, `&mut Window`, or `&mut Context<T>` in a View or task. Those are short-lived access granted for the current GPUI call.

Do not read or update an Entity again while it is already inside its `render` or `update`. GPUI prevents re-entrant access and will panic. Use the `self` and inner `cx` already provided. The same applies inside `cx.listener`: its `this` argument is already the View. If code must update that Entity *after* the current callback, defer the work instead of trying to access it through its handle immediately.

Use `cx.entity()` when another object needs a strong handle to the current Entity. Prefer `cx.weak_entity()` or `downgrade()` in long-lived callbacks that should not keep a View alive.

## Defer until the current update ends

`App::defer` runs an app-level closure after the current effect cycle. `window.defer(cx, ...)` supplies that Window later. From an Entity, `cx.defer_in(window, ...)` supplies the same View, its Window, and `Context<Self>` after the current update has released its borrow:

```rust
fn finish_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    cx.defer_in(window, |this, window, cx| {
        this.finish_edit_after_update(window, cx);
    });
}
```

The deferred closure still receives `this: &mut Self`; do not call `update` on that same Entity from inside it. Deferral is for work that needs the current update to finish, such as focus restoration after changing the UI tree. [`window.on_next_frame(...)`](https://docs.rs/crate/gpui-pre/{{gpui_pre_version}}/source/src/window.rs) instead queues a callback for the next platform frame request and wakes the frame source. The callback runs before any drawing for that request; registering it does not mark the window dirty or cause a render. If its work changes visible Entity state, call `cx.notify()` from the callback. Use `window.request_animation_frame()` when the intent is to request a redraw on the next frame. A closed window or released View may prevent deferred work from running, so do not use it as a durable job queue.

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

When `spawn` starts from `App`, there is no current Entity handle. The closure receives only `AsyncApp`; use `cx.update(|cx| ...)` to run a short application-level mutation after an `await`. `AsyncApp` also offers closure-based access to globals, such as `read_global` and `update_global`. It does not give an async task a `&mut App` to hold across awaits.

Use `spawn_in` when completion also needs the same Window. Its `AsyncWindowContext` lets `update_in` restore both Window and Entity access:

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

Use this `spawn_in` → `update_in` pattern for async work that updates a View and its Window. Use `background_spawn` for CPU-heavy work; it cannot update GPUI state directly, so bring its result back to a foreground task first.

The `WeakEntity` may have been released, and the Window may have closed while the task waited. `update` and `update_in` therefore return a `Result`; handle it. If a request can finish after a newer request, compare an ID or revision before applying its result.

## Task lifetime

A GPUI [Task](./task) is cancelled when its handle is dropped:

```rs
struct Chat {
    _load_task: Task<anyhow::Result<()>>,
}
```

- Store View-owned work on the View, so releasing the View cancels it.
- Call `.detach()` only when work should continue independently.
- Replacing a stored refresh or debounce task cancels the previous one.

## Observe and subscribe

`observe` reacts when another Entity calls `cx.notify()`. `subscribe` reacts to a typed [Event]:

```rs
struct Chat {
    input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

// In Chat::new:
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

Both return a `Subscription`. Store it on the subscribing View so it remains active for exactly that View's lifetime. Dropping it cancels the callback immediately. Keeping it in a longer-lived owner may retain its callback and captured resources after the View disappears, causing a memory leak. Use `observe_in` or `subscribe_in` when the callback also needs `&mut Window`.

To publish an application event, implement `EventEmitter<EventType>` on the emitting Entity type, then call `cx.emit(event)` from its `Context`:

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

A parent can use `cx.subscribe(&draft, ...)` to handle `SaveRequested`; the event type is part of that subscription, so an Entity can emit more than one type. An observer is tied to `cx.notify()`, while a subscription is tied to `cx.emit(...)`. Emitting an event does **not** notify observers or redraw the emitter: call `cx.notify()` as well if its rendered state changed. Conversely, `notify` does not emit an event. For an application-wide value, use `observe_global` and keep its `Subscription`; see [Global].

## Common mistakes

- A task stops early because its `Task` was dropped. Store it or intentionally detach it.
- An observer never runs because its `Subscription` was only a local variable.
- A View stays alive because a task or callback captured a strong Entity handle. Capture a weak handle.
- GPUI reports an Entity is already borrowed because code re-entered the same Entity during `render` or `update`.
- A callback receives plain `App` but needs its View; create it with `cx.listener` instead of trying to find and re-enter the View manually.
- A later operation needs the current Entity after a UI-tree change; use `cx.defer_in` rather than updating the borrowed Entity immediately.
- Async code cannot access Window because it used `spawn`; use `spawn_in` and `update_in`.
- A borrow is held across `await`; extract owned data first, then reacquire access with `update` or `update_in`.
- The UI stays stale because state changed without `cx.notify()`.

GPUI convention names every context parameter `cx`, regardless of its concrete type, and names the Window parameter `window`.

[Entity]: ./entity.md
[Event]: ./event.md
[Global]: ./global.md
