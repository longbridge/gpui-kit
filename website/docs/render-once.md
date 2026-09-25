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
    action: Option<AnyElement>,
}

impl MessageRow {
    fn new(author: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        Self {
            author: author.into(),
            body: body.into(),
            action: None,
        }
    }

    fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }
}

impl RenderOnce for MessageRow {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let MessageRow { author, body, action } = self;

        div()
            .flex()
            .gap_2()
            .child(div().font_semibold().child(author))
            .child(body)
            .when_some(action, |row, action| row.child(action))
    }
}
```

`#[derive(IntoElement)]` generates the conversion that lets the value participate in GPUI's fluent [Element](./element) tree:

```rust
div().child(MessageRow::new("You", "Explain RenderOnce"))
```

The derive does not render the component eagerly. It generates an `IntoElement` implementation whose element is `ViewElement<Self>`; GPUI consumes and renders the value as part of the surrounding tree. Implementing `RenderOnce` alone gives the value a `View` implementation, but does not let you pass it directly to `.child(...)`: that also requires `IntoElement`, which this derive supplies. The derive does **not** implement `Styled`, `ParentElement`, focus, or accessibility semantics for your type. Those capabilities come from the elements you build or from additional traits you implement.

## Try it: rebuild a value, keep the count

In an app that depends on `gpui-kit`, replace `src/main.rs` with this complete example and run `cargo run`. The `CounterLabel` is a `RenderOnce` value. `CounterView` is the retained `Entity` that owns the count.

```rust
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;

#[derive(IntoElement)]
struct CounterLabel {
    count: u32,
}

impl RenderOnce for CounterLabel {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child(format!("Count: {}", self.count))
    }
}

struct CounterView {
    count: u32,
}

impl Render for CounterView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .gap_2()
            .child(CounterLabel { count: self.count })
            .child(
                Button::new("increment")
                    .primary()
                    .label("Add one")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.count += 1;
                        cx.notify();
                    })),
            )
    }
}

fn main() {
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);
        open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| CounterView { count: 0 })
        })
        .expect("Failed to open window");
    });
}
```

The window starts at `Count: 0`. Click **Add one** twice: it should show `Count: 1`, then `Count: 2`. Each `CounterView::render` call constructs a new `CounterLabel` from the current count; its earlier value is consumed. The count survives because the same `CounterView` Entity owns it. `cx.notify()` requests another render after the click changes that owner. The component's name does not promise exactly one render per display frame; see [Render](./render) for when GPUI renders.

If the text stays at zero, check that the listener writes `this.count` and calls `cx.notify()`. If it always returns to one, check that `CounterView { count: 0 }` is created in the window closure, not inside `render`. If `.child(CounterLabel { ... })` does not compile, keep `#[derive(IntoElement)]` and `use gpui_kit::*;` in scope. The later snippets in this page illustrate separate variations; use this full example as the copyable starting point.

## Build and use the component

`MessageRow` has a small builder surface: `new` supplies required text, while `action` is an optional **slot**. `AnyElement` stores whichever concrete element the caller supplies. The slot is rendered after the body; calling `.action(...)` twice replaces the earlier element. This follows the repository's [`Empty` component](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/empty.rs), which uses named optional slots and renders them in a fixed order. If the component must accept several children, store `Vec<AnyElement>` and implement `ParentElement`; one named slot does not need that trait.

The caller can put a semantic control in the slot and keep the behavior in its retained View. In this example the `Editor` below owns `opened`; its click listener changes that field and requests another render:

```rust
// Inside Editor::render, where cx: &mut Context<Self> is available.
MessageRow::new("You", "Explain RenderOnce").action(
    Button::new("open-message")
        .label("Open")
        .on_click(cx.listener(|this, _, _, cx| {
            this.opened = true;
            cx.notify();
        })),
)
```

Import `Button` with `use gpui_kit::component::button::Button;` and give `Editor` an `opened: bool` field. For repeated messages, pass each button a stable ID derived from that message's domain ID (for example, `("open-message", message.id)` if that ID is a supported `ElementId` part). The label is display text, not an identity. `MessageRow` itself has no Entity, listener context, or callback ownership: it consumes the already-built action. A custom component that owns its own callback can instead store an owned `'static` handler in a private field and forward it to a semantic control during `render`; use that design when the callback is part of the component's contract.

## Owned values and the render lifecycle

The [`Render`](./render) guide covers the retained View. `RenderOnce` requires `Self: 'static`, and the value component's actual signature is:

```rust
fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement;
```

Because `self` is owned, rendering can move fields directly into the Element tree and its `'static` handlers. The component value is used once; its parent constructs a new value the next time that parent renders. Builder methods may mutate the value while constructing it; the resulting props describe one render rather than a persistent mutable model.

Destructuring first keeps ownership clear when several fields move into different parts of the tree. The complete `MessageRow::render` above does this for its text and action slot.

The slot conversion happens in the builder, before `render`; `.when_some(...)` adds it only when present. A `SharedString` owns reusable text, and `AnyElement` owns the type-erased child, so neither borrows a short-lived local variable.

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

## Common compile errors and a quick check

| Symptom | Check |
| --- | --- |
| `MessageRow` does not implement `IntoElement` at `.child(...)` | Add `#[derive(IntoElement)]` as well as `impl RenderOnce`; keep `use gpui_kit::*;` in scope. |
| `no method named when_some` (or a fluent style method) | Import `gpui_kit::prelude::*;` for `FluentBuilder`, and check whether the method belongs to the returned `div()` rather than your component type. |
| A borrowed local value “does not live long enough” in a slot or handler | Move owned text (`SharedString`), an owned `AnyElement`, or a cloned `Entity` handle into the component. GPUI handlers must be `'static`. |
| `cx.listener` or `cx.notify()` is unavailable in `RenderOnce::render` | That method gets `&mut App`, not an entity `Context<Self>`. Create the listener in the owner View's `Render::render` and pass it in, as above. |
| Calling `.child(...)` on `MessageRow` fails | The derive provides `IntoElement`, not `ParentElement`. Add a named builder such as `.action(...)`, or implement `ParentElement` and store children explicitly. |

To check a copied example, first put the type, builders, and `RenderOnce` implementation in the same module with both imports shown above. Use it as a child of a retained View and run `cargo check -p your-app` in that app's workspace. The `Editor` snippets are separate illustrations; add the mentioned fields and imports before compiling them together. For the repository's real API, compare [`Empty`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/empty.rs), [`Checkbox`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/checkbox.rs), and [`Button`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/button/button.rs).

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
