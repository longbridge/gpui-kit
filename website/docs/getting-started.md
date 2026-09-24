---
title: Getting Started
description: Build your first GPUI Kit desktop application with one dependency and one view.
order: -2
---

# Getting Started

This guide builds a small desktop window with a GPUI Kit button. You need Rust and Cargo plus the system libraries for your platform; see [Installation](./installation.md) for macOS, Windows and Linux requirements. For a browser target, start with [WebAssembly](./webassembly.md) after learning the view model here.

## Create a project

```sh
cargo new gpui-hello
cd gpui-hello
```

Add GPUI Kit to the generated `Cargo.toml`:

```toml
[dependencies]
gpui-kit = "0.6"
```

This single dependency includes GPUI, GPUI Base, the styled GPUI Component library and its default icon assets. Application code accesses GPUI through `use gpui_kit::*;` and components through `gpui_kit::component`. You can change the feature selection later; see [Icons & Assets](./assets.md).

## Add a view

Replace `src/main.rs` with:

```rust
use gpui_kit::component::button::Button;
use gpui_kit::*;

struct HelloWorld;

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .gap_2()
            .child("Hello, World!")
            .child(
                Button::new("hello")
                    .primary()
                    .label("Click me")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}

fn main() {
    application()
        .with_assets(assets::Assets)
        .run(|cx| {
            init(cx);

            open_window(WindowOptions::default(), cx, |_, cx| {
                cx.new(|_| HelloWorld)
            })
            .expect("Failed to open window");
        });
}
```

Run `cargo run` from the project directory. A window shows the label and button; clicking the button prints `Clicked!` in the terminal.

The startup sequence has three parts:

1. `gpui_kit::application()` creates the desktop application; `.with_assets(...)` registers the default icon source.
2. `gpui_kit::init(cx)` initializes the enabled Kit layers, including component themes. Call it once before opening application windows or constructing components.
3. `gpui_kit::open_window(...)` creates an `Entity<HelloWorld>` from the closure and wraps it in a [`Root`](./window). `Root` owns the window's overlay layers, including dialogs, sheets and notifications. Return your content view from the closure, not another `Root`.

`HelloWorld` implements GPUI's [`Render`](./render) trait. When GPUI renders the view, `render` returns an [element tree](./element): a `div` containing text and a `Button`. The button is a value built for that render; when a control needs lasting state, such as an input's text, the owning view keeps an `Entity` for that state instead of recreating it in `render`.

## A small mental model

An [Entity<T>](./entity) holds state across frames. It can own a model without drawing anything; when `T` implements `Render` and is mounted, the entity is a persistent **View** that builds a fresh element tree each time it renders. A [RenderOnce](./render-once) component takes its inputs as a value and describes a reusable piece of that tree. Use one where the caller supplies its state and handlers; it can still use small keyed element state. Give complex state, subscriptions, and tasks a lasting owner.

```text
app shell → feature (model, commands, view)
              ├─ Entity<Model>        retained state
              └─ Entity<View>         retained view; View implements Render
                    └─ element tree   rebuilt for each render
                         └─ RenderOnce values for reusable pieces
```

As an app grows, a feature with its own workflow can keep its model and views together in a feature crate, with a private `Global` only when it needs truly application-wide state. Let features cooperate through small public interfaces, events, or `Entity` handles. This keeps reusable pieces inexpensive to adopt and gives teammates or AI agents a clear boundary for parallel changes. The [Coding Guides](./coding-guides) explain when to make that split and how to keep ownership and dependencies clear.

## Where to go next

Read these in order as your app grows:

1. [Element](./element.md) and [RenderOnce](./render-once.md): understand the frame's tree and value-like components.
2. [Entity](./entity.md) and [Context](./context.md): retain state and update it outside rendering.
3. [Window](./window.md): open windows and use the `Root` overlay layer.
4. [Event](./event.md) and [Action](./action.md): connect state changes, keyboard shortcuts and commands.
5. [Component catalog](../component/index.md): choose controls; then read [Icons & Assets](./assets.md) and [Fonts](./fonts.md) as your interface needs them.

For a complete application with retained input state and subscriptions, use the [application recipes](https://github.com/longbridge/gpui-kit/tree/main/examples/ai_recipes). The [Coding Guides](./coding-guides.md) explain the conventions behind those examples.
