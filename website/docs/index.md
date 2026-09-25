---
title: GPUI Kit
description: A comprehensive Rust framework for building fantastic, high-performance desktop applications with GPUI.
---

# GPUI Kit

GPUI Kit is a Rust desktop application framework built on GPUI.
The `gpui-pre` dependency name refers to the published, version-pinned GPUI snapshot used by Kit, not a different rendering framework; see [Installation](./installation#why-the-dependency-is-named-gpui-pre).

GPUI Kit's core architecture has five layers:

- **`gpui`**: The underlying UI runtime, entity model, windows, elements,
  layout, and rendering.
- **`gpui-kit`**: The application entry point that re-exports GPUI and brings
  together Base, Component, and default assets through one dependency.
- **`gpui-base`**: Unstyled behavior, controlled state, focus, overlays,
  virtual lists, dock infrastructure, and semantic design tokens.
- **`gpui-component`**: GPUI Component, the complete styled component library
  with 75+ documented components and primitives, themes, data tables, dock
  layout, and a code editor.
- **`gpui-shell`**: Opens a Rust host to JavaScript extensions, one granted
  capability at a time.

An application normally depends on `gpui-kit` for GPUI, Base, Component, and
assets. Add `gpui-shell` separately when the application hosts JavaScript
extensions; it remains part of the framework's core architecture.

Use `gpui-component` for polished controls with one coherent visual language,
or build your own design system on the reusable behavior and infrastructure in
`gpui-base`. This section covers GPUI Kit setup, shared design and coding guides, and
application development. For library APIs, see [GPUI Component](/component),
[GPUI Base](/base), and [GPUI Shell](/shell).

Read [Focus](./focus) for `FocusHandle`, Tab order, and the keyboard target, then
[Action](./action) for command dispatch. [KeyBinding](./keybinding) explains how to bind actions and
display the active shortcut. Continue with [Event](./event) for typed
notifications and the relationship between Actions and Events.

For the core rendering model, start with [Entity](./entity) and
[Context](./context), then read [Render](./render), [RenderOnce](./render-once),
and [ElementId](./element_id). [Style](./style) covers GPUI's fluent styling
methods; [Element](./element) and [Paint](./paint) explain lower-level drawing.
[Task](./task) covers work that continues after a callback returns.

## Learn GPUI in a working order

Use these stages as a learning path. Each stage has a small application task to
try before moving on:

1. **Open a window:** follow [Installation](./installation) and [Getting Started](./getting-started), run the button example, and confirm its click reaches the terminal.
2. **Own and redraw state:** make an [Entity](./entity), update it through [Context](./context), and use [Render](./render) to show the new value. Then read [Window](./window) to understand which window receives the update.
3. **Draw a custom control:** follow [Element](./element), [Geometry](./geometry), and [Paint](./paint) with the existing Brush example. Use [ElementId](./element_id) and [View Cache](./view-cache) when the drawing needs stable state or reuse.
4. **Handle input and ongoing work:** establish a keyboard target with [Focus](./focus), then connect an [Action](./action) or [Event](./event) to the View; use [Task](./task) for asynchronous work and [Animation](./animation) for motion that ends cleanly.
5. **Check the application:** use [Accessibility](./accessibility) and [Testing](./test) for interaction checks. Read [FPS Monitor](./fps) before making frame-rate claims, then choose a target such as [WebAssembly](./webassembly) or [Mobile](./mobile) if the application needs one.

Each core page distinguishes a code example from its runtime result and links
to the next concept. The [Coding Guides](./coding-guides) collect ownership and
architecture conventions once the first window works.

## Features

- **75+ Components and Primitives**: Forms, navigation, overlays, data display, editing, feedback, layout, and more.
- **Production Ready**: Refined through production desktop applications and continuously tested across GPUI Kit's components and examples.
- **WebAssembly**: Applications and component showcases run on the web through `wasm32-unknown-unknown`.
- **Accessibility**: AccessKit roles, names, states, relationships, and actions are built into the interaction layer.
- **UI Integration Testing**: Headless windows exercise real pointer, keyboard, focus, layout, and accessibility behavior.
- **Native Feel**: Modern controls inspired by macOS and Windows.
- **High refresh support**: GPUI can target a 120 Hz display when the complete workload fits its roughly 8.3 ms frame budget; actual smoothness depends on the application, device, and presentation path. See [Frames, refresh rates, and rendering modes](./fps#120-hz-is-a-frame-budget-not-a-refresh-promise).
- **Data Tables**: Virtual scrolling, fixed and resizable columns, sorting, and cell selection across hundreds of thousands of rows.
- **Virtual Lists**: Render only the visible range, including differently sized items.
- **Code Editor**: 200K lines, Tree-sitter highlighting, diagnostics, completion, and hover.
- **Dock Layout**: Resizable panels, draggable tabs, nested splits, and edge docks.
- **Rich Content**: Native Markdown and HTML, syntax highlighting, and charts.
- **Design Freedom**: Use the complete visual system or build your own on `gpui-base`.
- **Typed Motion**: CSS-aligned easing, timing, keyframes, springs, presence, and measured reveal with allocation-free steady sampling.
- **Cross Platform**: Ship one Rust codebase to macOS, Windows, and Linux.

## Quick Example

After preparing the platform libraries in [Installation](./installation), create a Rust project with `cargo new gpui-hello` and enter it with `cd gpui-hello`. Add `gpui-kit` to its `Cargo.toml`:

```toml
[dependencies]
gpui-kit = "0.6"
```

Replace `src/main.rs` with this complete "Hello, World!" application:

```rust
use gpui_kit::component::button::{Button, ButtonVariants};
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

Run `cargo run` from the project directory. The window shows a label and button; clicking the button prints `Clicked!` in the terminal. Continue with [Getting Started](./getting-started) for the initialization sequence, `Render`, and retained state.

## Community & Support

Learn how to build interruptible animation in the [GPUI Base Motion guide](/base/motion).

- [GitHub Repository](https://github.com/longbridge/gpui-kit)
- [Issue Tracker](https://github.com/longbridge/gpui-kit/issues)
- [Contributing Guide](https://github.com/longbridge/gpui-kit/blob/main/CONTRIBUTING.md)

## License

Apache-2.0
