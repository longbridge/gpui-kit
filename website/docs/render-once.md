---
title: RenderOnce
description: Build reusable, declarative GPUI components from owned data.
order: -2.6
---

# RenderOnce

GPUI provides `RenderOnce` for reusable components that are described by owned data and rebuilt when their parent renders. It is a good fit for buttons, rows, badges, cards, and other declarative pieces that do not own a persistent lifecycle.

```rust
use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div};

#[derive(IntoElement)]
struct MessageRow {
    author: SharedString,
    body: SharedString,
}

impl MessageRow {
    fn new(author: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        Self {
            author: author.into(),
            body: body.into(),
        }
    }
}

impl RenderOnce for MessageRow {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .gap_2()
            .child(div().font_semibold().child(self.author))
            .child(self.body)
    }
}
```

`#[derive(IntoElement)]` generates the conversion that lets the value participate in GPUI's fluent Element tree:

```rust
div().child(MessageRow::new("You", "Explain RenderOnce"))
```

The derive does not render the component eagerly. It wraps the `RenderOnce` value as an Element; GPUI consumes and renders it as part of the surrounding tree.

## Why `render` consumes `self`

The signature is the main difference from [`Render`](./render):

```rust
fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement;
```

Because `self` is owned, rendering can move fields directly into the Element tree and its `'static` handlers. The component value is used once; its parent constructs a new value on the next render.

Destructuring first keeps ownership clear when several fields move into different parts of the tree:

```rust
fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
    let MessageRow { author, body } = self;

    div()
        .child(author)
        .child(body)
}
```

This does **not** mean the visible UI disappears after one frame. The resulting Element participates in that frame, and the parent supplies the next description when it renders again.

## State belongs outside the component value

Do not put changing application state in a `RenderOnce` value and expect mutations to survive. Store persistent state in an `Entity` whose type implements `Render`, then pass the current values or an `Entity` handle into the component.

```rust
#[derive(IntoElement)]
struct SendButton {
    chat: Entity<Chat>,
}

impl RenderOnce for SendButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let chat = self.chat;

        Button::new("send")
            .child("Send")
            .on_click(move |_, _, cx| {
                chat.update(cx, |chat, cx| {
                    chat.send_message(cx);
                });
            })
    }
}
```

Element handlers are `'static`, so use `move` and capture owned values such as `SharedString`, `Entity<T>`, or an Action. Clone a handle before moving it when the caller still needs it.

Capturing an `Entity<T>` keeps that entity alive as long as the rendered handler is retained. This is often correct for a child acting on its owner. Use `WeakEntity<T>` when the handler must not extend the target's lifetime, and handle the case where `weak.update(...)` can no longer reach it.

`RenderOnce::render` receives `&mut App`, not `&mut Context<Self>`. A `RenderOnce` component therefore has no entity context of its own: it cannot use `cx.listener`, retain subscriptions, or call `cx.notify()` for itself. Pass a handler, dispatch an [Action](./action), or update the state-owning `Entity` instead.

## Builder-style components

Owned fields make `RenderOnce` work naturally with builder APIs. GPUI Kit and Zed use this pattern for components such as buttons, list items, labels, and modal sections:

```rust
#[derive(IntoElement)]
struct StatusBadge {
    label: SharedString,
    muted: bool,
}

impl StatusBadge {
    fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            muted: false,
        }
    }

    fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }
}

impl RenderOnce for StatusBadge {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .rounded_full()
            .px_2()
            .when(self.muted, |this| this.opacity(0.6))
            .child(self.label)
    }
}
```

## Choose the right layer

| Use | When |
| --- | --- |
| `RenderOnce` | A reusable component is constructed from owned inputs and has no independent persistent state. |
| [`Render`](./render) | A stateful `Entity` owns data, subscriptions, tasks, Focus, or a lifecycle and must render repeatedly. |
| [`Element`](./element) | You need direct control over layout, prepaint, paint, hit testing, or other low-level rendering phases. |

A useful composition is: a `Render` view owns state, it creates `RenderOnce` components to describe reusable UI, and those components return built-in Elements. Implement `Element` only when the standard Element APIs cannot express the rendering behavior.

:::info
If a component starts accumulating mutable state, subscriptions, or background tasks, move that lifecycle into an `Entity` and implement `Render` for it. Keeping such state inside a value that is consumed on every render loses the ownership model that makes `RenderOnce` simple.
:::
