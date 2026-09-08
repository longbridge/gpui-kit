---
title: Icons & Assets
description: Configure bundled icons and custom assets for GPUI Component applications.
order: -4
---

# Icons & Assets

The [IconName] and [Icon] in GPUI Component provide a comprehensive set of icons and assets that can be easily integrated into your GPUI applications.

But for minimal size applications, **we have not embedded any icon assets by default** in `gpui-component` crate.

We split the icon assets into a separate crate [gpui-kit-assets] to allow developers to choose whether to include the icon assets in their applications or if you don't need the icons at all, you can build your own assets.


:::note NOTE — Depending on the crate does not embed every icon

`gpui-kit-assets` includes the full icon catalog, but **adding the dependency or
using `IconName` does not by itself put every SVG into your final binary or load
it into memory**. In optimized builds, unreferenced SVG data is discarded.

- Registering the full `Assets` source embeds **all** SVGs on native platforms.
- Registering `icon_assets!(AppAssets, [Search, Check])` embeds **only those two**
  SVGs, on native and WASM. Register `AppAssets` instead of `Assets`.
- Selected assets borrow static SVG bytes when loaded, without copying them or
  allocating a cache. Rendering still needs memory for SVG parsing,
  rasterization, and GPUI's render caches. This is not a zero-memory guarantee.

The complete catalog is still present in the downloaded crate and build
artifacts. Runtime `IconName` lookup can retain its name/path table; this does
not retain the SVG payloads. The full WASM `Assets::new(endpoint)` source fetches
icons on demand rather than embedding them.

:::

## Shared names and selected embedding

`IconName` and `IconNamed` are defined in `gpui_kit::assets` and reexported by
`gpui_kit::component`. Base and alternative presentation layers can use the
complete icon catalog without depending on GPUI Component. The bundle contains
all 1,818 Lucide 1.43.0 icons plus 12 retained GPUI Kit icons.

To embed only the icons your application needs:

```rust
use gpui_kit::assets::{icon_assets, IconName};
icon_assets!(AppAssets, [Search, Check]);
let app = gpui_kit::application().with_assets(AppAssets);
```

Register `AppAssets` instead of the full `Assets` source. Only the selected SVG
bytes are embedded, on both native and WASM; other paths return `Ok(None)`.
Loading borrows static bytes without a copy or runtime cache. Rendering still
uses GPUI's normal SVG parsing and rasterization. Selection is explicit because
runtime string paths cannot be inferred automatically. `IconName` alone does
not reference SVG bytes; runtime name lookup can retain the name/path table.

`IconName::ALL` lists the full catalog and `IconName::Search.path()` resolves its
asset path. Existing component imports still work. For `name.view(cx)`, also
import `gpui_kit::component::IconNameExt`, or use `Icon::new(name).view(cx)`.

## Use default bundled assets

The [gpui-kit-assets] crate provides a default bundled assets implementation that includes all the icon files in the `assets/icons` folder.

To use the default bundled assets, you need to add the `gpui-kit-assets` crate as a dependency in your `Cargo.toml`:

```toml
[dependencies]
gpui-component = { git = "https://github.com/longbridge/gpui-kit" }
gpui-kit-assets = { git = "https://github.com/longbridge/gpui-kit" }
```

Then we need call the `with_assets` method when creating the GPUI application to register the asset source:

```rs
use gpui_kit::*;
use gpui_kit::assets::Assets;

let app = gpui_kit::application().with_assets(Assets);
```

Now, we can use `IconName` and `Icon` in our application as usual, the all icon assets are loaded from the default bundled assets.

Continue [Use the icons](#use-the-icons) section to see how to use the icons in your application.

## Build you own assets

You may have a specific set of icons that you want to use in your application, or you may want to reduce the size of your application binary by including only the icons you need.

In this case, you can build your own assets by following these steps.

The [assets](https://github.com/longbridge/gpui-kit/tree/main/crates/assets/assets/) folder in source code contains all the available icons in SVG format, every file is that GPUI Component support, it matched with the [IconName] enum.

You can download the SVG files you need from the [assets] folder, or you can use your own SVG files by following the [IconName] naming convention.

In GPUI application, we can use the [rust-embed] crate to embed the SVG files into the application binary.

And GPUI Application providers an `AssetSource` trait to load the assets.

```rs
use anyhow::anyhow;
use gpui_kit::*;
use gpui_kit::component::{v_flex, IconName, Root};
use rust_embed::RustEmbed;
use std::borrow::Cow;

/// An asset source that loads assets from the `./assets` folder.
#[derive(RustEmbed)]
#[folder = "./assets"]
#[include = "icons/**/*.svg"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow!("could not find asset at path \"{path}\""))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}
```

We need call the `with_assets` method when creating the GPUI application to register the asset source:

```rs
fn main() {
    // Register Assets to GPUI application.
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        // We must initialize gpui_component before using it.
        gpui_kit::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| Example);
                // The first level on the window must be Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
```

## Use the icons

Now we can use the icons in our application:

```rs
pub struct Example;

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .text_center()
            .child(IconName::Inbox)
            .child(IconName::Bot)
    }
}
```

## Embed individual SVG icons

For custom icons, `Icon::data` accepts SVG bytes directly without an asset-path registry:

```rust
use gpui_kit::component::{Icon, button::Button};

Button::new("search")
    .icon(Icon::default().data(include_bytes!("search.svg")))
    .label("Search")
```

This only removes the asset lookup for that icon. Built-in `IconName` values and
other path-based component icons still need an asset source. See
[SVG Bytes](../component/icon.md#svg-bytes) for ownership, source replacement,
loading icons, and custom icon types.

## Resources

- [Lucide Icons](https://lucide.dev/) - The icon set used in GPUI Component is based on the open-source Lucide Icons library, which provides a wide range of customizable SVG icons.

[rust-embed]: https://docs.rs/rust-embed/latest/rust_embed/
[IconName]: https://docs.rs/gpui-kit-assets/latest/gpui_kit_assets/enum.IconName.html
[Icon]: https://docs.rs/gpui_component/latest/gpui_component/icon/struct.Icon.html
[assets]: https://github.com/longbridge/gpui-kit/tree/main/crates/assets/assets/
[gpui-kit-assets]: https://crates.io/crates/gpui-kit-assets
