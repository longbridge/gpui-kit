---
title: Icons & Assets
description: Configure bundled icons and custom assets for GPUI Kit applications.
order: -7
---

# Icons & Assets

GPUI Kit exposes [IconName] and [Icon] through `gpui_kit::assets` and `gpui_kit::component`. The default `gpui-kit` features make both available, but the application must register an `AssetSource` to load icons by path. The underlying [gpui-kit-assets] crate keeps the SVG payloads separate from the component code: register the default `Assets`, select extra icons, or provide your own source.


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
On [WebAssembly](./webassembly), `Assets::new(endpoint)` and `AllAssets::new(endpoint)` use the existing
on-demand CDN loader instead of embedding the complete bundle.

:::

## Shared names and compatibility

`gpui_kit::assets::IconName` provides the complete shared catalog without a
Component dependency. `gpui_kit::component::IconName` remains the original
compatibility enum: existing imports, exhaustive matches and `.view(cx)` calls
continue to work without a new trait import. `Icon::new(...)` accepts either
type. A legacy name converts into the shared name with `.into()`.

For the new shared enum, use `Icon::new(name).view(cx)` when a component [Entity](./entity)
is needed, or import `gpui_kit::component::IconNameExt` to call `name.view(cx)`.

`IconName::ALL` enumerates all 1,830 names; `IconName::Accessibility.path()`
returns `icons/accessibility.svg`. The default source contains only the original
101 component icons. Supply extra icons using the custom source below, or
explicitly register `AllAssets` to use the complete bundle.

## Start with the default source

`gpui_kit::assets::Assets` provides the default resource source with the original 101 component icons listed in [`default-icons.txt`](https://github.com/longbridge/gpui-kit/blob/main/crates/assets/default-icons.txt).

Add the umbrella crate to `Cargo.toml`; its default features include `component` and `assets`:

```toml
[dependencies]
gpui-kit = "0.6"
```

For a native desktop app, register the source before opening a window. This complete `src/main.rs` renders a default icon:

```rust
use gpui_kit::*;
use gpui_kit::assets::Assets;
use gpui_kit::component::{Icon, IconName};

struct Example;

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child(Icon::new(IconName::Inbox))
    }
}

fn main() {
    gpui_kit::application().with_assets(Assets).run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| Example)
        })
        .expect("Failed to open window");
    });
}
```

`with_assets` installs one application-wide source. `gpui_kit::init(cx)` initializes the component layer before the window is constructed. `IconName::Inbox` resolves to `icons/inbox.svg`; GPUI passes that exact path to the source. Merely adding the crate dependency does not register a source. See [Getting Started](./getting-started.md) for the full application setup.

## Pick additional catalog icons with `icon_assets!`

If an app already depends on `gpui-kit`, use `gpui_kit::assets::icon_assets!`; its default `assets` feature exposes the macro and the shared catalog. **Do not add a second `gpui-kit-assets` dependency just to use it.** A crate that intentionally uses the asset layer without the umbrella can depend on `gpui-kit-assets` directly and call `gpui_kit_assets::icon_assets!` instead.

Choose the source according to the icons the app renders:

| Need | Register with `with_assets` | Native icon data |
| --- | --- | --- |
| Only the default component icons | `Assets` | 101 default SVGs |
| Default icons plus a few catalog icons | A composite of `Assets` and `icon_assets!` | Defaults plus named selections |
| Every catalog icon | `AllAssets` | All 1,830 SVGs |
| Only a few catalog icons, without Component | The generated source alone | Named selections only |

The macro arguments are **`IconName` variant identifiers**, not strings, paths, or your own SVG filenames. For example, `Accessibility` selects `icons/accessibility.svg`. The generated `ExtraIcons` is a unit struct implementing `AssetSource`: `load` returns borrowed embedded bytes for selected paths and `Ok(None)` otherwise; `list(prefix)` lists selected paths. Selection happens at compile time. A misspelled or nonexistent variant fails compilation. The macro accepts an optional visibility modifier, such as `icon_assets!(pub ExtraIcons, [Accessibility]);`, when the source must be used from another module.

The following complete `src/main.rs` selects two extra Lucide icons while retaining the default component icons. It targets native desktop applications. Use the `gpui-kit` dependency shown above; no SVG copying or new crate is needed.

```rust
use gpui_kit::*;
use gpui_kit::assets::{Assets as ComponentAssets, icon_assets};
use gpui_kit::component::Icon;
use std::borrow::Cow;

icon_assets!(ExtraIcons, [Accessibility, AlarmClock]);

struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = ExtraIcons.load(path)? {
            return Ok(Some(bytes));
        }
        ComponentAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = ComponentAssets.list(path)?;
        paths.extend(ExtraIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

struct Example;

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(Icon::new(gpui_kit::assets::IconName::Accessibility))
            .child(Icon::new(gpui_kit::assets::IconName::AlarmClock))
    }
}

fn main() {
    gpui_kit::application().with_assets(AppAssets).run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| Example)
        })
        .expect("Failed to open window");
    });
}
```

`with_assets` takes one source, so `AppAssets` owns both lookups. It checks `ExtraIcons` first because native `ComponentAssets.load` returns an error for an unknown path; that error would prevent a later fallback. `list` merges, sorts, and deduplicates both sources so callers see the same keys that `load` can serve. The generated source alone is suitable only when the app does not need the default component icons.

To migrate a hand-written asset lookup for bundled Lucide icons, replace its copied SVG bytes or filename match arms with one `icon_assets!(ExtraIcons, [...]);` declaration. Keep any unrelated app files in their own source, query that source and `ExtraIcons` before the default `Assets` fallback, and merge all three `list` results. Remove the old SVG copies only after verifying that no other code loads those files by path. Your own files such as `icons/brand-mark.svg` are outside the catalog and still need the custom source described below. `IconName::ALL` lists names; it does not make every name loadable from the currently registered source.

For a quick check, call `AppAssets.load("icons/accessibility.svg")` and `AppAssets.load("icons/inbox.svg")`; both should return `Some` on native. `AppAssets.list("icons/")` should contain both paths. If the extra icon is blank, check that the rendered `IconName` appears in the macro list and that `AppAssets`, rather than `ComponentAssets`, was registered. On WebAssembly, the macro source can still embed the selection, but the built-in `Assets` is constructed with an endpoint and must be stored as a field in the composite source.

## Add your own asset files

`icon_assets!` selects from GPUI Kit's named catalog. For an application's own logo, illustration, photo, or other image, use an application `AssetSource`. The [assets] folder in this repository supplies catalog SVGs; placing a new file in your app does **not** add an `IconName` variant. A key such as `icons/brand-mark.svg` is relative to the embedded folder, not a URL or a path relative to the running process's working directory.

The example below embeds one custom SVG and an image folder. Use this layout next to your application's `Cargo.toml`:

```text
Cargo.toml
src/main.rs
assets/icons/brand-mark.svg
assets/images/cover.png
```

Add [rust-embed] alongside `gpui-kit`:

```toml
[dependencies]
gpui-kit = "0.6"
rust-embed = { version = "8.7", features = ["include-exclude"] }
```

This complete native `src/main.rs` serves app files first and falls back to GPUI Kit's default component icons. The `images/**/*` pattern embeds **every file** under that folder, so keep only intended assets there; you can replace it with narrower `#[include]` patterns. The example registers one source before opening the window.

```rs
use gpui_kit::*;
use gpui_kit::assets::Assets as ComponentAssets;
use gpui_kit::component::Icon;
use rust_embed::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "./assets"]
#[include = "icons/**/*.svg"]
#[include = "images/**/*"]
struct AppFiles;

struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        if let Some(file) = AppFiles::get(path) {
            return Ok(Some(file.data));
        }
        ComponentAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = ComponentAssets.list(path)?;
        paths.extend(AppFiles::iter().filter_map(|p| p.starts_with(path).then(|| p.into())));
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

struct Example;

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(Icon::default().path("icons/brand-mark.svg"))
            .child(img("images/cover.png").w(px(240.)).h(px(160.)).object_fit(ObjectFit::Cover))
    }
}

fn main() {
    gpui_kit::application().with_assets(AppAssets).run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| Example)
        })
        .expect("Failed to open window");
    });
}
```

`AssetSource::load(path)` supplies bytes for an exact key; `list(prefix)` reports matching keys. In this example app files win on a key collision, and unknown keys reach `ComponentAssets`. On native, that built-in source errors on an unknown nonempty path. If the app also uses `icon_assets!`, check its generated source before this fallback and include its paths in `list` as shown above. On WASM, construct `Assets::new(endpoint)` and store that value in the composite source instead of using the native unit struct.

## Use an asset path

Use the path exposed by the registered source. `Icon::path` is for SVG icons; `IconName` maps known names to those paths.

```rust
use gpui_kit::component::{Icon, IconName};

let built_in = Icon::new(IconName::Inbox);
let custom = Icon::default().path("icons/brand-mark.svg");
let extra = Icon::new(gpui_kit::assets::IconName::Accessibility);
```

`extra` needs the selected-icons source above or `AllAssets`. `custom` needs a source that contains `icons/brand-mark.svg`. Use `Icon::path` or `svg().path(...)` for a **monochrome SVG** that should follow the text color; GPUI paints it as a tinted alpha mask. Set an explicit size and `text_color` when needed. Use `img()` for photographs and other raster graphics, or for a **multicolor SVG** whose original colors should be preserved:

```rust
use gpui_kit::{ObjectFit, StyledImage, img, px, svg};

let tinted_mark = svg().path("icons/brand-mark.svg").size(px(24.));
let cover = img("images/cover.png")
    .w(px(240.))
    .h(px(160.))
    .object_fit(ObjectFit::Cover);
let color_artwork = img("images/illustration.svg").size(px(160.));
```

The `img()` source supports common PNG, JPEG, WebP, GIF, and SVG files (and other formats listed by GPUI's `Img::extensions()`). It detects raster formats from the bytes; SVG takes a separate decoding path. Use a real image file with a supported format, not only a matching extension. `ObjectFit::Contain` is the default; `Cover` fills the bounds and may crop, while `Fill` can distort. `object_fit` controls `img()`, not the monochrome `svg()` element. `img()` loads and decodes asynchronously through GPUI's image cache; embedded source bytes are copied into its loader, so embedding alone does not eliminate decode or runtime image memory. Animated GIF and WebP can contain multiple frames.

For a string like `"images/cover.png"`, GPUI calls the registered `AssetSource` with that key. `img(std::path::Path::new("/absolute/file.png"))` reads a filesystem path, and a URL string uses the HTTP loader. They have different deployment and error behavior. See [Images](./image.md) for how each source loads, sizes, and caches.

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

## Diagnose a missing asset

| Symptom | Check |
| --- | --- |
| A default component icon is blank | Confirm `.with_assets(Assets)` (or a composed source) runs before opening windows. Check that the path starts with `icons/` and ends in `.svg`. |
| A shared catalog name is blank | Confirm that name belongs to the registered source. `Assets` has 101 defaults; register selected icons or `AllAssets` for other names. |
| A custom icon is blank | Compare the exact `Icon::path` key with the path below the source's `#[folder]`; check `#[include]` and case. `list("icons/")` can reveal what the source exposes. |
| An image is blank | Check whether the argument is an asset key, filesystem `Path`, or URL; verify that the source includes the raster file and that the bytes are a supported image format. |

On native, a missing path in the built-in `Assets` and `AllAssets` returns an error; an empty path returns `Ok(None)`. In a composed source, test custom matches before the built-in fallback. `Icon::data` avoids path lookup for that one SVG, but malformed SVG bytes can still fail to render.

## Packaging and WebAssembly

Native `Assets` embeds only the default SVGs; `AllAssets` embeds the full catalog. `icon_assets!` embeds selected SVG bytes, and `rust-embed` embeds files matched by your include patterns. Source paths are resolved at build time, so packaged native apps do not need the source `assets/` directory at runtime. A filesystem `Path` passed to `img()` is different: that file must exist where the app runs. Use the size measurements above as examples, then measure your own release binary.

On WebAssembly, `Assets::new(endpoint)` and `AllAssets::new(endpoint)` use the same on-demand HTTP source. It requests `endpoint + "/assets/" + path` for `icons/*.svg`, caches successful responses, and reports a temporary loading error while a request is in flight. Host the exact files and use an endpoint without a trailing slash. Check the browser Network and Console panels for status, CORS, or URL problems; the loader does not itself request a repaint after a download. `list()` returns an empty list on this source. The native `rust-embed` example above is a separate choice for explicitly embedding app files, including on WASM. See [WebAssembly](./webassembly.md) for the gallery's deployment setup.

## Resources

- [Lucide Icons](https://lucide.dev/) - GPUI Kit's icon catalog is based on the open-source Lucide collection.

[rust-embed]: https://docs.rs/rust-embed/latest/rust_embed/
[IconName]: https://docs.rs/gpui-kit-assets/0.6.5/gpui_kit_assets/enum.IconName.html
[Icon]: https://docs.rs/gpui-component/latest/gpui_component/struct.Icon.html
[assets]: https://github.com/longbridge/gpui-kit/tree/main/crates/assets/assets/icons
[gpui-kit-assets]: https://docs.rs/crate/gpui-kit-assets/0.6.5
