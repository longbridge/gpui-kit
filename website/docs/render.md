---
title: Render
description: Render a stateful GPUI Entity into an element tree.
order: -2.5
---

# Render

GPUI provides the `Render` trait for turning the current state of an `Entity` into an element tree. Use it for a long-lived View whose state changes over time, such as a Chat panel, settings page, or workspace.

```rust
use gpui::{div, prelude::*, Context, IntoElement, Render, Window};

struct Chat {
    messages: Vec<String>,
}

impl Render for Chat {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .children(
                self.messages
                    .iter()
                    .cloned()
                    .map(|message| div().child(message)),
            )
    }
}
```

`render` receives:

- `&mut self`: the state stored in this Entity;
- `&mut Window`: window-level state and operations;
- `&mut Context<Self>`: the current Entity's GPUI context;
- and returns `impl IntoElement`: a value GPUI can convert into an element tree.

Returning `impl IntoElement` keeps the concrete, often deeply nested element type out of the function signature. Common values such as `div()`, GPUI Kit components, and child `Entity<T>` values can all participate in that tree.

## Updating the View

Changing an Entity does not by itself tell GPUI that its visible output changed. Mutate the state inside an Entity update and call `cx.notify()`:

```rust
impl Chat {
    fn push_message(&mut self, message: String, cx: &mut Context<Self>) {
        self.messages.push(message);
        cx.notify();
    }
}
```

The relationship is:

```text
Entity state changes
        ↓
    cx.notify()
        ↓
GPUI invalidates Views displaying that Entity
        ↓
Render builds their next element trees
```

`cx.notify()` also notifies observers of the Entity. Rendering is scheduled by GPUI; it does not call `render` synchronously at the line where `notify` runs. When the same Entity is shown in multiple windows or locations, GPUI invalidates the live Views that display it.

## Keep Render Declarative

Treat `render` as a description of the UI for the current state. Reading state, choosing children, and attaching handlers are normal. Avoid starting work or changing application state merely because `render` ran:

- do not start network requests or background tasks;
- do not create subscriptions or register application observers;
- do not dispatch commands or emit Events;
- do not unconditionally call `cx.notify()`, `window.refresh()`, or `window.request_animation_frame()`.

GPUI may render again whenever the View is invalidated. An unconditional `cx.notify()` inside `render`, `prepaint`, `paint`, or a canvas callback can continually dirty the window and create an idle redraw loop. Start work in initialization or an explicit handler, update the Entity when results arrive, then call `cx.notify()` only when visible state actually changed.

Input handlers attached while rendering are different: the closure is registered as part of the element tree and runs later, when input occurs.

```rust
div()
    .child("Clear")
    .on_click(cx.listener(|this, _, _, cx| {
        this.messages.clear();
        cx.notify();
    }))
```

## Render, RenderOnce, and Element

Choose the narrowest abstraction that fits:

| API | Use it for | Method receiver | Context |
| --- | --- | --- | --- |
| `Render` | A stateful, long-lived View backed by an `Entity<T>` | `&mut self` | `Context<Self>` |
| [`RenderOnce`](./render-once) | A reusable component assembled from owned input | `self` | `App` |
| [`Element`](./element) | Custom layout, prepaint, hitboxes, or painting | phase-specific `&mut self` | `App` |

An `Entity<T>` where `T: Render` can be added directly as a child. Its Entity ID gives the View identity, and notifications can invalidate its View subtree. A `RenderOnce` component is consumed as it builds a tree and has no Entity identity of its own. Implement `Element` only when composing existing elements is not enough.

## Related Guides

- [`Entity`](./entity) explains ownership, reading, and updating state.
- [`Context`](./context) explains `App`, `Window`, and `Context<T>`.
- [`RenderOnce`](./render-once) covers reusable components made from owned props.
- [`Element`](./element) covers GPUI's layout and paint phases.
