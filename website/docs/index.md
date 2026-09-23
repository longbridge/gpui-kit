---
title: GPUI Kit
description: A comprehensive Rust framework for building fantastic, high-performance desktop applications with GPUI.
---

# GPUI Kit

GPUI Kit (aka: GPUI Component) is a comprehensive Rust desktop application framework built on GPUI.

It combines a complete UI system with application-grade data, layout, content,
and editing capabilities, and it ships as three crates that build on each other,
all reachable through the single `gpui-kit` dependency:

- **`gpui-base`**: Unstyled behavior, controlled state, focus, overlays,
  virtual lists, dock infrastructure, and semantic design tokens.
- **`gpui-component`**: GPUI Component, the complete styled component library
  with 75+ documented components and primitives, themes, data tables, dock
  layout, and a code editor.
- **`gpui-shell`**: Opens a Rust host to JavaScript extensions, one granted
  capability at a time.

Use `gpui-component` for polished controls with one coherent visual language,
or build your own design system on the reusable behavior and infrastructure in
`gpui-base`. This section covers GPUI Kit setup, shared design and coding guides, and
application development. For library APIs, see [GPUI Component](/component),
[GPUI Base](/base), and [GPUI Shell](/shell).

Read [Action](./action) for GPUI Focus, `track_focus`, Key Contexts,
KeyBindings, and command dispatch. Continue with [Event](./event) for typed
notifications and the relationship between Actions and Events.

## Features

- **75+ Components and Primitives**: Forms, navigation, overlays, data display, editing, feedback, layout, and more.
- **Production Ready**: Used to build Longbridge Pro from day one and refined in a publicly shipped commercial desktop application.
- **WebAssembly**: Applications and component showcases run on the web through `wasm32-unknown-unknown`.
- **Accessibility**: AccessKit roles, names, states, relationships, and actions are built into the interaction layer.
- **UI Integration Testing**: Headless windows exercise real pointer, keyboard, focus, layout, and accessibility behavior.
- **Native Feel**: Modern controls inspired by macOS and Windows.
- **120 FPS**: GPU-accelerated interfaces that remain smooth under load.
- **Data Tables**: Virtual scrolling, fixed and resizable columns, sorting, and cell selection across hundreds of thousands of rows.
- **Virtual Lists**: Render only the visible range, including differently sized items.
- **Code Editor**: 200K lines, Tree-sitter highlighting, diagnostics, completion, and hover.
- **Dock Layout**: Resizable panels, draggable tabs, nested splits, and edge docks.
- **Rich Content**: Native Markdown and HTML, syntax highlighting, and charts.
- **Design Freedom**: Use the complete visual system or build your own on `gpui-base`.
- **Typed Motion**: CSS-aligned easing, timing, keyframes, springs, presence, and measured reveal with allocation-free steady sampling.
- **Cross Platform**: Ship one Rust codebase to macOS, Windows, and Linux.

## Quick Example

Add `gpui-kit` to your `Cargo.toml`:

```toml
[dependencies]
gpui-kit = "0.6"
```

Then create a simple "Hello, World!" application with a button:

```rust
use gpui_kit::*;
use gpui_kit::component::button::*;
use gpui_kit::component::*;

pub struct HelloWorld;
impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}

fn main() {
    gpui_kit::application().run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_kit::init(cx);

        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| HelloWorld)
        })
        .expect("Failed to open window");
    });
}
```

## Community & Support

Learn how to build interruptible 120 FPS animation in the [GPUI Base Motion guide](/base/motion).

- [GitHub Repository](https://github.com/longbridge/gpui-kit)
- [Issue Tracker](https://github.com/longbridge/gpui-kit/issues)
- [Contributing Guide](https://github.com/longbridge/gpui-kit/blob/main/CONTRIBUTING.md)

## License

Apache-2.0
