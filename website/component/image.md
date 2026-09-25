---
title: Image
description: Display embedded, local, and remote images with sizing, loading, and error states.
---

# Image

GPUI's `img()` creates an image element. GPUI Kit re-exports it from `gpui_kit`, so an application using the umbrella crate needs no separate GPUI dependency. An image can come from an embedded asset key, a file-system `Path`, an HTTP URL, or in-memory image data. The source type determines where GPUI loads the bytes; styling determines how those bytes are displayed.

## Start with a working image

This complete native `src/main.rs` uses an icon already bundled by GPUI Kit, so it needs no extra asset file. Add `gpui-kit = "0.6"` to `Cargo.toml`. The same asset is shown as a color-preserving image and as a theme-colored monochrome SVG; the difference is explained below.

```rust
use gpui_kit::*;
use gpui_kit::assets::Assets;

struct ImageExample;

impl Render for ImageExample {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .gap_4()
            .child(
                img("icons/inbox.svg")
                    .id("inbox-image")
                    .size(px(64.))
                    .object_fit(ObjectFit::Contain)
                    .with_loading(|| div().child("Loading image...").into_any_element())
                    .with_fallback(|| div().child("Image unavailable").into_any_element()),
            )
            .child(svg().path("icons/inbox.svg").size(px(64.)))
    }
}

fn main() {
    gpui_kit::application().with_assets(Assets).run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| ImageExample)
        })
        .expect("Failed to open window");
    });
}
```

`with_assets(Assets)` registers the default component icons before opening a window. Replace the source with a composite `AssetSource` when your app also embeds its own images; see [Icons & Assets](../docs/assets.md). The example's `with_loading` callback appears only if loading is still pending after a short delay. `with_fallback` runs when loading fails. The image's stable `.id(...)` lets GPUI keep its loading state across frames.

## Choose the source

| Argument to `img(...)` | How GPUI loads it | Packaging implication |
| --- | --- | --- |
| `"images/cover.png"` | Calls the registered `AssetSource` with that exact key | Embed or provide that key in your source. |
| `std::path::Path::new("/absolute/cover.png")` | Reads a file-system path | Ship the file where the app will run, or let the user select it. |
| `"https://example.com/cover.png"` | Fetches a URL through the configured HTTP client | Handle connectivity, loading, and HTTP errors. |
| `Arc<Image>` | Decodes caller-supplied encoded bytes and format | Supply an `Image` with matching bytes and format. |
| `Arc<RenderImage>` | Uses already renderable image data | The caller creates or retains the renderable data. |

A non-URL **string** is an asset key, even if it looks like a relative file name. It is not resolved against the process's current directory. To bundle your own `images/cover.png`, include it in an application `AssetSource` and register that source with `with_assets(...)`. For a native `rust-embed` source rooted at `./assets`, the file `assets/images/cover.png` has the key `images/cover.png`. See the [application asset walkthrough](../docs/assets.md) for the source implementation and fallback to component icons.

## Size and fit

Give the image a useful layout size. `object_fit` determines how its content fits inside those bounds; the default is `Contain`.

```rust
img("images/cover.png")
    .w(px(320.))
    .h(px(180.))
    .object_fit(ObjectFit::Cover)
```

| Fit | Result |
| --- | --- |
| `Contain` | Show the whole image and preserve its aspect ratio; unused space may remain. |
| `Cover` | Fill the bounds and preserve aspect ratio; edges may be cropped. |
| `Fill` | Stretch to both dimensions, possibly distorting the image. |
| `ScaleDown` | Fit like `Contain` but do not enlarge the source. |
| `None` | Keep the source's original size. |

Use `Cover` for a cropped thumbnail and `Contain` for a logo or diagram that must remain fully visible. Set both width and height when the surrounding layout needs stable dimensions during loading.

## Loading and failures

`img()` exposes real loading and error hooks through `StyledImage`. The callbacks return an element to display in the image's place. This example uses an embedded key; the same pattern works for URLs and file-system paths.

```rust
img("images/cover.png")
    .id("cover-image")
    .w(px(320.))
    .h(px(180.))
    .object_fit(ObjectFit::Cover)
    .with_loading(|| div().child("Loading image...").into_any_element())
    .with_fallback(|| div().child("Image unavailable").into_any_element())
```

A quick load may finish before the loading view appears. A missing asset key, invalid image bytes, a missing local file, or an unsuccessful HTTP response can lead to the fallback. Keep the outer layout size stable so a replacement view does not unexpectedly move nearby content. GPUI caches loaded image resources; use a dedicated image cache only when your view has a specific cache lifetime requirement.

## SVG as an image or an icon

Both forms can load a key from the same `AssetSource`, but they render differently:

| API | Rendering | Use it for |
| --- | --- | --- |
| `img("images/brand.svg")` | Rasterizes the SVG as an image and keeps its source colors. `object_fit` applies. | Multicolor logos, illustrations, and diagrams. |
| `svg().path("icons/check.svg")` | Renders the SVG's alpha as a monochrome mask, colored by the element's text color. | Single-color icons that follow a theme or state. |

```rust
img("images/brand.svg")
    .size(px(96.))
    .object_fit(ObjectFit::Contain);

svg().path("icons/check.svg")
    .size(px(20.))
    .text_color(rgb(0x2563eb));
```

Applying `.text_color(...)` to `img("images/brand.svg")` does **not** recolor the image. Use `svg().path(...)` or GPUI Kit's [`Icon`](./icon.md) for a monochrome icon. For a small SVG provided directly as bytes, `svg().data(include_bytes!("check.svg"))` skips the asset-path lookup. A full-color SVG still belongs in `img()`.

`img()` supports common raster formats such as PNG, JPEG, WebP, and GIF, plus SVG. Format support comes from the pinned GPUI image decoder; check your target platform with representative files before shipping.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Blank embedded image | Confirm that the registered `AssetSource` includes the exact key and file bytes. `img("images/cover.png")` is an embedded lookup, not a file-system read. |
| Default icon missing | Register GPUI Kit's default `Assets` or compose it after your app source. |
| Full-color SVG changes to one color | Render it with `img()`; `svg().path(...)` intentionally uses an alpha mask. |
| Image appears cropped | Use `ObjectFit::Contain`, or increase the display bounds. |
| Fallback appears | Check the file path, network response, or decoded bytes for the chosen source type. |

For an image that conveys information, provide adjacent text or another accessible description in the surrounding UI. A filename is not a user-facing description.
