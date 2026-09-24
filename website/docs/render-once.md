---
title: RenderOnce
description: Build reusable, declarative GPUI components from owned data.
order: -2.6
---

# RenderOnce

The core distinction is ownership. **`RenderOnce::render(self, ...)` consumes a component value**: its parent normally constructs a fresh, lightweight description when the parent renders. **[`Render::render(&mut self, ...)`](./render) borrows a retained View** stored in an [Entity](./entity). Use `RenderOnce` for declarative inputs that describe a reusable piece of UI for this render, and `Render` when a View itself must keep state and a lifecycle across renders. `Entity<T>` can also hold a model or other data that does not implement `Render`; an Entity becomes a renderable View when its type implements that trait.

```rust
// RenderOnce
fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement;
// Render
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement;
```

That is why most reusable GPUI Kit components are `RenderOnce`: a caller can supply props and a handler, and the component can return existing semantic elements without creating an Entity for every button, row, or badge. A file tree, chat list, chart workspace, or other feature View that owns a collection, subscriptions, async work, or coordinated selection usually needs a retained `Entity<T>` and `Render`. A complex feature View can still create many `RenderOnce` children.

```rust
use gpui_kit::*;
use gpui_kit::prelude::*;

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

`#[derive(IntoElement)]` generates the conversion that lets the value participate in GPUI's fluent [Element](./element) tree:

```rust
div().child(MessageRow::new("You", "Explain RenderOnce"))
```

The derive does not render the component eagerly. It generates an `IntoElement` implementation whose element is `ViewElement<Self>`; GPUI consumes and renders the value as part of the surrounding tree. Implementing `RenderOnce` alone gives the value a `View` implementation, but does not let you pass it directly to `.child(...)`: that also requires `IntoElement`, which this derive supplies. The derive does **not** implement `Styled`, `ParentElement`, focus, or accessibility semantics for your type. Those capabilities come from the elements you build or from additional traits you implement.

## Owned values and the render lifecycle

The [`Render`](./render) guide covers the retained View. `RenderOnce` requires `Self: 'static`, and the value component's actual signature is:

```rust
fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement;
```

Because `self` is owned, rendering can move fields directly into the Element tree and its `'static` handlers. The component value is used once; its parent constructs a new value the next time that parent renders. Builder methods may mutate the value while constructing it; the resulting props describe one render rather than a persistent mutable model.

Destructuring first keeps ownership clear when several fields move into different parts of the tree:

```rust
fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
    let MessageRow { author, body } = self;

    div()
        .child(author)
        .child(body)
}
```

This does **not** mean the visible UI disappears after one frame, nor that every display refresh constructs a new value. “Once” refers to one component instance: the parent makes another whenever its `render` runs. A normal window redraw may render even unchanged child views, while an explicit [cached view](./view-cache) can skip a clean subtree. GPUI retains Entities and keyed state across these render passes, so this is not simply a traditional immediate-mode loop that rebuilds the whole application on every screen refresh. Do not retain `&mut Window` or `&mut App` beyond this call. Repeated rows should use stable IDs derived from domain data when their children need identity; see [ElementId](./element_id).

## State belongs outside the component value

`RenderOnce` does **not** mean “no state.” The value holds props such as a label, disabled flag, or checked flag for this render. GPUI may retain small interaction details under a stable [ElementId](./element_id), and a `RenderOnce` component may hold an external `Entity<T>` handle. The boundary is that changing application state must have a durable owner outside the consumed value. That owner may be a `Render` View, a model-only Entity, or another application state holder; pass current values, a callback, or a handle into the component.

GPUI Kit's styled `Button` and `Checkbox` both implement `RenderOnce`. The Button takes a label and click handler. Checkbox takes a controlled `checked` bool and reports the requested next value through `on_click`; the owner writes that value and calls `cx.notify()`. Their Base layer supplies focus, keyboard, and accessibility behavior. The parent can recreate them cheaply while keeping the actual state in one place.

```rust
use gpui_kit::component::checkbox::Checkbox;

// Inside the owner's Render::render, with self.show_hidden and cx available:
Checkbox::new("show-hidden")
    .checked(self.show_hidden)
    .label("Show hidden files")
    .on_click(cx.listener(|this, checked, _, cx| {
        this.show_hidden = *checked;
        cx.notify();
    }))
```

```rust
use gpui_kit::component::button::{Button, ButtonVariants};

struct Editor {
    saved: bool,
}

impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(if self.saved { "Saved" } else { "Unsaved" })
            .child(
                Button::new("save")
                    .label("Save")
                    .primary()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.saved = true;
                        cx.notify();
                    })),
            )
    }
}
```

This example assumes the first example's `use gpui_kit::*;` import. The `Editor` is mounted in an `Entity<Editor>`; `Button::new` establishes a stable element ID. The button's label, focus, keyboard activation, and accessible role come from the component and Base layers. Element handlers are `'static`, so a child that captures a callback or `Entity<T>` needs an owned value. Clone a handle before moving it when the caller still needs it.

Text editing shows the other side of the boundary. `Input::new(&state)` returns a `RenderOnce` visual component, but `state` is an `Entity<InputState>` retained by the owning View. `InputState` implements `Render` and keeps text, selection, focus, editing history, and input behavior across parent renders. Recreating `Input::new(&state)` does not recreate the editor state:

```rust
use gpui_kit::component::input::{Input, InputState};

struct SearchView {
    query: Entity<InputState>,
}

impl Render for SearchView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Input::new(&self.query).id("search-query")
    }
}
```

Create `query` once when constructing `SearchView`, for example with `cx.new(|cx| InputState::new(window, cx))`; do not create it inside `render`. A collection-heavy View follows the same ownership rule: keep records, filters, selected IDs, and subscriptions in the retained owner, then render value-like rows and controls from them.

Capturing an `Entity<T>` keeps that entity alive as long as the rendered handler is retained. This is often correct for a child acting on its owner. Use `WeakEntity<T>` when the handler must not extend the target's lifetime, and handle the case where `weak.update(...)` can no longer reach it.

`RenderOnce::render` receives `&mut App`, not `&mut Context<Self>`. A `RenderOnce` component therefore has no entity [Context](./context) of its own: it cannot use `cx.listener` for itself, retain its own subscriptions or tasks, or call `cx.notify()` to schedule itself. Pass a handler, dispatch an [Action](./action), or update the state-owning `Entity` instead. Keyed element state can retain local interaction details, but it does not replace an owner for durable application data.

## Builder-style components

Owned, private fields make `RenderOnce` work naturally with builder APIs. GPUI Kit uses this pattern in its components. A builder takes and returns `Self`, preserving a valid component while callers refine it:

```rust
use gpui_kit::component::ActiveTheme;

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
            .text_color(cx.theme().muted_foreground)
            .when(!self.muted, |this| this.text_color(cx.theme().foreground))
            .child(self.label)
    }
}
```

This example also uses `use gpui_kit::*;`. The returned `div()` implements [`Styled`](./style) and `ParentElement`, enabling `.rounded_full()`, `.text_color()`, and `.child()`. If callers need those methods **on `StatusBadge` itself**, implement `Styled` and/or `ParentElement` for the type, store the styles or children, and apply them in `render`. `IntoElement` derive does not forward the returned element's fluent traits. GPUI Kit's `Button` explicitly implements both. Use `.when(...)` and `.when_some(...)` for small refinements; use ordinary Rust branches when the UI structure differs substantially.

## Choose the right layer

| Use | When |
| --- | --- |
| [`RenderOnce`](./render-once) + `IntoElement` | A reusable component consumes caller-supplied props and handlers for a render. It may use keyed state or an external Entity; rendering receives `&mut Window` and `&mut App`. |
| [`Render`](./render) | A retained `Entity` owns changing data, collections, subscriptions, tasks, or a lifecycle; rendering receives `&mut Context<Self>` to update and notify it. |
| [`Element`](./element) | Built-in elements cannot express required layout, prepaint, paint, hit testing, or other low-level phases. |

A useful composition is: a `Render` view owns state, it creates `RenderOnce` components to describe reusable UI, and those components return built-in Elements. This gives most GPUI Kit components a small, explicit API while keeping complex state in a few meaningful owners. Implement `Element` only when the standard Element APIs cannot express the rendering behavior.

:::info
If a component starts accumulating mutable state, subscriptions, or background tasks, move that lifecycle into an `Entity` and implement `Render` for it. Keeping such state inside a value that is consumed on every render loses the ownership model that makes `RenderOnce` simple.
:::
