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

**The complete catalog does not make existing applications embed every icon.**
`Assets` keeps the original 101 component icons. Applications provide additional
icons through their own `AssetSource`, as before; they do not need to redeclare
the component icons. Only explicitly registering `AllAssets` embeds all 1,830
SVGs on native platforms. Depending on the crate or using the shared `IconName`
alone does not reference every SVG payload.

| Native asset configuration | Embedded SVG data | Binary increase vs. default `Assets` |
| --- | ---: | ---: |
| Default component icons (101) | 44.28 KiB | 0 B (baseline) |
| Default + 2 application icons (103) | 45.04 KiB | +15.19 KiB |
| Default + 10 application icons (111) | 48.09 KiB | +19.19 KiB |
| Explicit `AllAssets` (1,830) | 731.45 KiB | +1.02 MiB |

**In this example, adding 10 application icons costs about 19 KiB, not the full
catalog.** Their SVGs total 3,903 bytes; the measured binary increase is 19,648
bytes, including the extra source's lookup/list-composition code, metadata and
alignment. These are not fixed per-icon costs or whole-application sizes.

Measured with Lucide 1.43.0 on Linux x86_64, Rust 1.98.0, `--release`, and stripped
symbols. Each program uses the same `IconName` lookup and runtime asset path.
The extra source falls back to `Assets`, and merges, sorts and deduplicates both
sources' lists. The 10 extras are `Accessibility`, `AlarmClock`, `Archive`,
`Award`, `Backpack`, `Bike`, `Bird`, `Camera`, `Coffee` and `Compass`; the two-icon
case uses the first two. SVG complexity, toolchain and source implementation
change the result.

Binary size is not RAM usage. Selected sources borrow static bytes without a
copy/cache; actual rendering still allocates for parsing, rasterization and
render caches. Runtime shared-name lookup can retain a name/path table, and
Cargo's downloaded package/build artifacts still contain the complete catalog.
On WASM, `Assets::new(endpoint)` and `AllAssets::new(endpoint)` use the existing
on-demand CDN loader instead of embedding the complete bundle.

:::

## Shared names and compatibility

`gpui_kit::assets::IconName` provides the complete shared catalog without a
Component dependency. `gpui_kit::component::IconName` remains the original
compatibility enum: existing imports, exhaustive matches and `.view(cx)` calls
continue to work without a new trait import. `Icon::new(...)` accepts either
type. A legacy name converts into the shared name with `.into()`.

For the new shared enum, use `Icon::new(name).view(cx)` when a component entity
is needed, or import `gpui_kit::component::IconNameExt` to call `name.view(cx)`.

`IconName::ALL` enumerates all 1,830 names; `IconName::Accessibility.path()`
returns `icons/accessibility.svg`. The default source contains only the original
101 component icons. Supply extra icons using the custom source below, or
explicitly register `AllAssets` to use the complete bundle.

## Use default bundled assets

The [gpui-kit-assets] crate provides a default bundled assets implementation that embeds the original 101 component icons listed in `crates/assets/default-icons.txt`.

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

Now, we can use `IconName` and `Icon` in our application as usual, the original component icons are loaded from the default bundle.

Continue [Use the icons](#use-the-icons) section to see how to use the icons in your application.

## Build you own assets

You may have a specific set of icons that you want to use in your application, or you may want to reduce the size of your application binary by including only the icons you need.

In this case, you can build your own assets by following these steps.

The [assets](https://github.com/longbridge/gpui-kit/tree/main/crates/assets/assets/) folder in source code contains all the available icons in SVG format, every file is that GPUI Component support, it matched with the [IconName] enum.

You can download the SVG files you need from the [assets] folder, or you can use your own SVG files by following the [IconName] naming convention.

In GPUI application, we can use the [rust-embed] crate to embed the SVG files into the application binary.

And GPUI Application providers an `AssetSource` trait to load the assets.

```rs
use gpui_kit::*;
use gpui_kit::assets::Assets as ComponentAssets;
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

        if let Some(file) = Self::get(path) {
            return Ok(Some(file.data));
        }
        ComponentAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = ComponentAssets.list(path)?;
        paths.extend(Self::iter().filter_map(|p| p.starts_with(path).then(|| p.into())));
        paths.sort();
        paths.dedup();
        Ok(paths)
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
