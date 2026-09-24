---
title: Render
description: Turn Entity state into an element tree and understand when GPUI rebuilds a View.
order: -2.5
---

# Render

`Render` is the boundary between a persistent [`Entity<T>`](./entity) and the UI it currently describes. GPUI calls `T::render` when it needs that View's element tree. The Entity keeps its data and identity; the returned elements describe layout, appearance, and handlers for this rendering pass. Use `Render` for a panel, page, editor, or other View that owns changing state, subscriptions, or child entities.

```rust
use gpui_kit::*;
use gpui_kit::component::button::Button;

#[derive(Default)]
struct Chat {
    messages: Vec<SharedString>,
}

impl Chat {
    fn clear(&mut self, cx: &mut Context<Self>) {
        if !self.messages.is_empty() {
            self.messages.clear();
            cx.notify();
        }
    }
}

impl Render for Chat {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .child(
                Button::new("clear-messages")
                    .label("Clear")
                    .on_click(cx.listener(|this, _, _, cx| this.clear(cx))),
            )
            .children(
                self.messages
                    .iter()
                    .cloned()
                    .map(|message| div().child(message)),
            )
    }
}
```

The signature is `fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement`; the trait requires `Self: 'static + Sized`. `self` is the state inside the current Entity. `window` gives access to window specific operations; `cx` is this Entity's [Context](./context). `impl IntoElement` keeps the concrete, often deeply nested return type out of the signature. Here `div()` is a GPUI element and `Button` is a GPUI Kit component. The click closure is *registered* while rendering and runs later on input, with mutable access to `Chat` through `cx.listener`.

## The View owns state; the tree describes this pass

Create the Entity with `cx.new`. A parent can retain and render the same handle on every pass:

```rust
struct ConversationPage {
    chat: Entity<Chat>,
}

impl ConversationPage {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            chat: cx.new(|_| Chat::default()),
        }
    }
}

impl Render for ConversationPage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.chat.clone())
    }
}
```

`ConversationPage` owns the same `Chat` Entity across rendering passes; it does not recreate the message list whenever the page redraws. Cloning `Entity<Chat>` copies its handle, not its messages. A file list page can follow the same pattern: keep its file list View as a child Entity, with the list owning selection, loading state, and subscriptions. An `Entity<T: Render>` can be a child because GPUI converts it into a View element. Its Entity ID gives that View a reactive boundary and a distinct element ID space. See [Entity](./entity) for creation, sharing, and lifetime rules.

## Render or RenderOnce?

`Entity<T>` is a state container first: `T` does not need to implement `Render`. An `Entity<Model>` can hold data, receive updates, and notify observers without ever appearing in an element tree. When `T: Render`, the same container can be placed in the tree as a persistent View, keyed by its Entity ID. GPUI's `View` machinery also accepts `RenderOnce` values, but those have no Entity ID of their own. `Render` is the Entity-backed UI route, not a requirement for every Entity.

| Trait | Receiver and owner | Appropriate work |
| --- | --- | --- |
| `Render` | `render(&mut self, ..., &mut Context<Self>)` on a persistent `Entity<T>` View | Own changing or complex state, subscriptions, focus, tasks, and child entities across render passes. |
| [`RenderOnce`](./render-once) | `render(self, ..., &mut App)` consumes a value constructed by its parent | Describe a lightweight component from current inputs and callbacks; the parent supplies a new value when it renders again. |

The `Chat` View above implements `Render` because it owns a changing message collection. GPUI Kit's `SelectState<D>` is another state owner that implements `Render`. Its styled `Button` implements `RenderOnce`: the parent supplies its current props and handles the result of a click. A `RenderOnce` value can still be interactive, and GPUI may retain small keyed element state beneath it; it simply has no independent Entity lifecycle. This is a choice about **who owns persistent state**, not a promise that one trait runs once per display frame or that the other runs every frame.

Returning a tree does not paint it immediately. GPUI converts the values to elements, resolves layout, runs `prepaint`, then [paints](./paint). A View's `render` can be called as part of layout or prepaint, and GPUI can reuse a valid cached subtree. Do not assume exactly one `render` call per frame, or that every parent render calls every child View's `render`. The output is a description for GPUI's element pipeline, not a retained list of pixels. [Element](./element) covers those phases.

## Notify after a meaningful state change

An Entity update grants mutable access, but mutation alone does not report a visible change. Call the *updated Entity's* `Context<T>::notify()` when its rendered output or observers should change:

```rust
chat.update(cx, |chat, cx| {
    chat.messages.push("Hello".into());
    cx.notify(); // This is Context<Chat>, not the caller's context.
});
```

`notify` marks live windows displaying that Entity for invalidation and queues notification to observers. GPUI arranges a later rendering pass; `render` is not called synchronously at the `notify` line. A View shown in more than one live window may invalidate in each. GPUI can reuse unaffected View subtrees. Notifications are also meaningful for observed model entities that do not themselves render.

An event and a notification have different jobs: `cx.emit(event)` delivers a typed fact to subscribers; `cx.notify()` reports that the Entity changed. Emitting an event alone does not mean every View that reads the Entity will redraw. Conversely, a notification does not convey an event payload. Use both when the state and the event each need to be observed.

When one View reads another Entity's state, keep ownership and invalidation explicit. If the other Entity renders as a child, its own `notify` can invalidate that child View. If the parent copies the other's values into its own output, arrange for the parent to observe the source and call the parent's `cx.notify()` when those copied values change. Avoid assuming a parent rerender is the only way a retained child can change.

## Render builds values; handlers perform work

Reading state, choosing children, applying [styles](./style), and attaching handlers are ordinary render work. Rebuilding a tree may happen for reasons unrelated to a user action. Starting a request, registering a subscription, emitting an event, or mutating application state unconditionally in `render` would repeat that work whenever GPUI rebuilds the View. An unconditional `cx.notify()` from `render`, `prepaint`, `paint`, or a canvas callback can keep invalidating the window.

Place lifecycle work in Entity initialization, subscriptions, or an explicit input handler. Hold a `Subscription` on the owning Entity. When asynchronous work completes, update the owning Entity and notify from that update if its state changed. For a click, `cx.listener` turns the later GPUI Kit `Button` callback into an update of this `Render` owner, as in the Chat example. Captured `Entity<T>` handles can similarly update a different owner from a handler; never try to reenter an Entity already being rendered or updated.

GPUI Kit uses `Render` for state owners such as input and selection state, while controls such as `Button` use `RenderOnce` to turn their current props into elements. A custom `Element` is appropriate when layout, hitboxes, or painting need direct control. This composition lets a persistent View own behavior while short lived values describe its current interface.

## Common mistakes

| Symptom | Check |
| --- | --- |
| State changed but the screen stayed the same | Was the correct Entity updated and its context notified? |
| Work runs repeatedly while idle | Is `render` or a paint callback starting work or calling `notify` on every pass? |
| Child state resets when the parent changes | Is the child a newly created Entity inside `render` instead of a handle retained by an owner? |
| A click handler cannot borrow `self` | Register an owned callback with `cx.listener`, or capture an `Entity` handle for later update. |
| Repeated rows lose local UI state | Give repeated elements with keyed state stable IDs based on item identity, not array positions. See [ElementId](./element_id). |

For the context APIs used here, see [Context](./context). For input callback and event details, see [Event](./event).
