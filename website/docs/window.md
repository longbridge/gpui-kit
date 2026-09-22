---
title: Window
description: Use GPUI Window for window-local input, focus, rendering, and asynchronous work.
order: -2.3
---

# Window

GPUI provides `Window` as the context for one native window. It connects the rendered Element tree to platform input, Focus, Action dispatch, drawing, and window controls. A View receives it only while GPUI is updating or rendering that window:

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

Keep application state in an [Entity](./entity). Use `Window` when an operation belongs to the current window or needs its current interaction state.

## What belongs to Window

Common window-local operations include:

| Need | API |
| --- | --- |
| Inspect geometry and state | `bounds`, `viewport_size`, `scale_factor`, `is_window_active` |
| Manage Focus | `focused`, `focus`, `blur`, `focus_next`, `focus_prev` |
| Send a command from code | `dispatch_action` |
| Request another frame | `refresh`, `on_next_frame` |
| Control the native window | `set_window_title`, `activate_window`, `remove_window` |
| Continue work later | `defer`, `spawn` |

`Window` also carries layout, text, hit testing, input, and drawing state internally. Most Views do not manipulate those systems directly; Elements and GPUI use them during rendering.

## Focus and Action dispatch

Focus is local to a Window. `window.focus(...)` selects a `FocusHandle`, and `window.focused(cx)` returns the current one. Keyboard input then uses the focused Element's Dispatch Path to match a KeyBinding and dispatch its Action.

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

Use `window.dispatch_action(action.boxed_clone(), cx)` when a button, command palette, native menu, or another piece of code should issue the same command as a KeyBinding. GPUI captures the current Focus and defers the actual dispatch until the current effect cycle completes.

```rust
Button::new("open-conversation")
    .label("Open")
    .on_click(|_, window, cx| {
        window.dispatch_action(Box::new(OpenConversation { id }), cx);
    })
```

The Action still follows the focused Dispatch Path. Place its `on_action_*` handler on that path, usually on the focused region or a common owner. See [Action](./action) for the complete routing model.

## Run work after the current update

Use `window.defer` when work must wait until entities currently being updated have been released. This is common when closing an overlay changes Focus, or when the next operation updates another part of the same UI tree.

```rust
fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let composer = self.composer.clone();

    window.defer(cx, move |window, cx| {
        let focus_handle = composer.read(cx).focus_handle(cx);
        window.focus(&focus_handle, cx);
    });
}
```

Inside an Entity, `cx.defer_in(window, ...)` is often more convenient because GPUI supplies that Entity again:

```rust
cx.defer_in(window, |this, window, cx| {
    this.rebuild_results(window, cx);
});
```

The callback already receives `&mut Self`. Do not call `update` on the same Entity from inside it; that attempts to update an Entity which is already being updated. Zed and Longbridge Pro use `defer` and `defer_in` for Focus changes and UI-tree mutations that cannot safely happen in the current callback.

Use `window.on_next_frame(...)` only when the operation specifically belongs to the next rendered frame, such as an animation step. `defer` means “after the current effect cycle,” which is a different boundary.

## Async work with a Window

Use `cx.spawn_in(window, ...)` when a task belongs to the current Entity and later needs both Entity and Window access:

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

`cx.spawn_in` provides a `WeakEntity<Self>` and an `AsyncWindowContext`. If the Entity or Window has gone away, `update_in` returns an error; propagate or handle it instead of assuming they still exist.

Use `window.spawn(cx, ...)` when the task needs the Window but does not belong to one Entity. Use `cx.spawn(...)` when no Window access is needed, and `cx.background_spawn(...)` for CPU-heavy work. A `Task` is cancelled when dropped, so store it on the owning View when its lifetime should follow that View, or call `.detach()` only for work that should continue independently.

## Subscribe with Window access

Use `cx.subscribe_in` when an Event callback needs `&mut Window`, for example to restore Focus after a child finishes:

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

Store the returned `Subscription` on the subscribing View. Dropping a local variable immediately cancels the subscription. Storing it on a longer-lived global owner can keep the callback and captured resources alive after the View disappears, causing a memory leak. See [Event](./event) for subscription ownership and multiple subscribers.

## Window lifetime

Do not store `&mut Window`; it is a temporary context supplied by GPUI. For later work, use `defer`, `spawn_in`, or obtain `window.window_handle()` and update it through GPUI. A handle does not keep a closed window alive, so handle-based updates can fail and should be treated accordingly.

Keep these ownership rules together:

- persistent UI state belongs to an Entity;
- window-specific work receives `&mut Window` only for the duration of a callback;
- `Task` and `Subscription` fields tie background work and observers to the owning View;
- Focus and Action dispatch always use the state of the specific Window.

[Entity]: ./entity
