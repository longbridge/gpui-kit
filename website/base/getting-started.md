---
title: Getting Started
description: Install, initialize, and render your first gpui-base control.
order: 2
---

# Getting Started

## Install

Use the repository revision of GPUI that matches `gpui-base`:

```toml
[dependencies]
gpui-base = { git = "https://github.com/longbridge/gpui-kit" }
gpui = { git = "https://github.com/zed-industries/zed" }
gpui_platform = { git = "https://github.com/zed-industries/zed", features = ["font-kit"] }
```

## Initialize

Call `gpui_kit::base::init` once before opening windows. If the application already calls `gpui_kit::component::init`, base initialization is included.

```rust
use gpui_kit::AppContext as _;

fn main() {
    gpui_platform::application().run(|cx| {
        gpui_kit::base::init(cx);
        // Open your application window here.
    });
}
```

## Render and style a control

Base controls intentionally have no product-specific padding, colors, or radius. Style them with ordinary GPUI methods:

```rust
use gpui_kit::prelude::*;
use gpui_kit::{px, rgb};
use gpui_kit::base::Button;

Button::new("save")
    .px_3()
    .py_2()
    .rounded(px(6.))
    .bg(rgb(0x2563eb))
    .text_color(rgb(0xffffff))
    .on_click(|_, _, _| println!("save"))
    .child("Save")
```

Keep each `ElementId` stable across renders so GPUI can preserve element and focus state. Controlled components such as Checkbox, Switch, Radio, and Toggle report the next value through callbacks; store that value in your view and pass it back on the next render.

## Default color tokens

`gpui-base` provides readable light and dark semantic palettes through
`ColorTokens::light()` and `ColorTokens::dark()`. `ColorTokens::default()` uses
the light palette. Both palettes use `Hsla` values and match the semantic roles
of the default `gpui-component` themes.

```rust
use gpui_kit::base::{ColorTokens, SemanticThemeTokens, Theme};

// Pick the palette that matches the application's current appearance.
let colors = if is_dark {
    ColorTokens::dark()
} else {
    ColorTokens::light()
};

Theme::global_mut(cx).tokens = SemanticThemeTokens {
    colors,
    ..Default::default()
};
```

The palette contains semantic roles rather than component-specific colors:
`background` and `foreground`, `surface` and `surface_foreground`, `primary`,
`secondary`, `muted`, `accent`, `destructive`, `border`, `input`, `ring`, and
`selection`, including the corresponding foreground roles. Base components
derive what they can from these roles — a link takes `primary`, for instance —
rather than adding a component-specific token for it. `selection` is its own
role because no other one can stand in for it: it is painted under the glyphs
and has to stay legible there, which neither `accent` nor `ring` guarantees.

Calling `gpui_kit::component::init` projects its active light or dark theme into the
same Base tokens automatically. Applications that use only `gpui-base` should
install the matching palette when their appearance mode changes.

## Run the shared examples

The examples used by this website also run as a native GPUI application:

```sh
cargo run -p gpui-base-examples -- button
```

Replace `button` with a primitive slug from the [primitive catalog](./primitives/index.md). The website compiles the same showcase for `wasm32-unknown-unknown` and loads it on each primitive page.
