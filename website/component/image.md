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

### Responsive width and a predictable ratio

Let the surrounding layout decide the available width, cap a large image, and reserve its height before the bytes arrive:

```rust
div()
    .w_full()
    .max_w(px(640.))
    .child(
        img("images/banner.webp")
            .w_full()
            .aspect_ratio(16. / 9.)
            .object_fit(ObjectFit::Cover),
    )
```

`w_full()` follows the parent width; `max_w(...)` limits the whole image region on a wide window. An explicit `aspect_ratio(...)` reserves a stable box before decoding. For square cards, use `.aspect_ratio(1.)`. Without an explicit ratio or height, GPUI can use the decoded image's intrinsic ratio, but the layout may change when loading finishes. If a flex child refuses to shrink in a narrow window, give its containing pane `.min_w_0()`.

### A small image grid

This is a scoped example for a view's `render` method after registering an `AssetSource` with these three keys. It intentionally uses a fixed three-column grid; for a narrow window, change the column count or use a different layout at your app's breakpoint.

```rust
div()
    .grid()
    .grid_cols(3)
    .gap_3()
    .children([
        "images/one.webp",
        "images/two.webp",
        "images/three.webp",
    ].into_iter().map(|source| {
        img(source)
            .w_full()
            .aspect_ratio(1.)
            .object_fit(ObjectFit::Cover)
    }))
```

Every tile reserves the same square area while loading. `Cover` can crop important content, so use `Contain` for diagrams or product images whose edges must remain visible. For a long, changing collection, use a scrolling or virtualized collection rather than constructing every tile in one frame.

## Gallery with selection

A thumbnail that changes the main image is a control. Give it a real `Button` so keyboard activation and an accessible name work, and keep the selected index in the view's retained state. This complete `src/main.rs` uses three icons already shipped with GPUI Kit; replace the source array with your own registered image keys for a photo gallery. Run it with the same `gpui-kit = "0.6"` dependency as the first example.

```rust
use gpui_kit::*;
use gpui_kit::assets::Assets;
use gpui_kit::base::Selectable;
use gpui_kit::component::button::Button;

const PICTURES: [(&str, &str); 3] = [
    ("icons/inbox.svg", "Inbox"),
    ("icons/book-open.svg", "Book"),
    ("icons/gallery-vertical-end.svg", "Gallery"),
];

struct Gallery {
    selected: usize,
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (source, name) = PICTURES[self.selected];

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .child(format!("Selected: {name}"))
            .child(
                img(source)
                    .id("gallery-main")
                    .w(px(360.))
                    .h(px(240.))
                    .object_fit(ObjectFit::Contain)
                    .with_fallback(|| div().child("Image unavailable").into_any_element()),
            )
            .child(
                div().flex().gap_2().children(
                    PICTURES.iter().enumerate().map(|(index, (source, name))| {
                        Button::new(format!("gallery-thumbnail-{index}"))
                            .accessibility_label(format!("Show {name}"))
                            .selected(index == self.selected)
                            .child(
                                img(*source)
                                    .size(px(40.))
                                    .object_fit(ObjectFit::Contain),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.selected = index;
                                cx.notify();
                            }))
                    }),
                ),
            )
    }
}

fn main() {
    gpui_kit::application().with_assets(Assets).run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| Gallery { selected: 0 })
        })
        .expect("Failed to open window");
    });
}
```

The selected button has an explicit selected state, and the visible `Selected: ...` text reports the current choice. In a product gallery, use meaningful labels such as `Show front view` rather than a file name. If the collection can be empty, render an empty-state message before indexing it. If image sources can be removed or reordered, store a stable domain ID instead of an index and resolve it during render.

## Hero image with readable copy

Place copy in a separate surface above the image rather than relying on pixels in the photo for contrast. This scoped example assumes `cx` is the view's context and `ActiveTheme` is imported.

```rust
use gpui_kit::component::ActiveTheme as _;

div()
    .relative()
    .w_full()
    .max_w(px(720.))
    .h(px(320.))
    .overflow_hidden()
    .child(
        img("images/hero.webp")
            .absolute()
            .inset_0()
            .size_full()
            .object_fit(ObjectFit::Cover),
    )
    .child(
        div()
            .absolute()
            .bottom_0()
            .left_0()
            .p_4()
            .bg(cx.theme().background)
            .child("Explore the collection"),
    )
```

The image is decorative here; the text carries the meaning. Keep any action outside a clipped or obscured region so its keyboard focus indication stays visible.

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

`with_loading` and `with_fallback` supply replacement elements; they do not expose a separate `ImageState` enum or automatically add a retry command. If a remote image is essential to the task, provide a nearby retry action in your application's state and rebuild the image with an updated source when the user retries. Avoid showing a fake loading skeleton after the source has already failed.

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

## API quick reference

| API | Purpose | Note |
| --- | --- | --- |
| `img(source)` | Create an image from an `ImageSource` | A non-URL string is an embedded asset key. |
| `.id(id)` | Give the element stable identity | Useful for image loading state across renders. |
| `.w(...)`, `.h(...)`, `.size(...)` | Set explicit dimensions | Reserve space before a remote image loads. |
| `.w_full()`, `.h_full()`, `.size_full()` | Fill the parent in that axis | The parent still needs useful bounds. |
| `.max_w(...)`, `.aspect_ratio(...)` | Cap width and reserve proportions | Helpful for responsive layouts. |
| `.object_fit(ObjectFit::...)` | Choose crop and scaling behavior | Defaults to `Contain`. |
| `.with_loading(...)`, `.with_fallback(...)` | Supply replacement elements | Each callback returns `AnyElement`. |
| `.grayscale(true)` | Render a desaturated image | Visual treatment does not change the source. |
| `.image_cache(&cache)` | Override cache for this image | Use only when a specific cache lifetime is needed. |

`rounded(...)`, borders, `overflow_hidden()`, and shadows are ordinary element/container styling, not image decoding options. Clip an image inside a rounded parent when the crop must follow that shape.

## Performance and accessibility

- Supply image dimensions close to the displayed size. A tiny thumbnail does not need the same encoded pixels as a full-screen photo.
- Compress source files and verify their appearance on your target displays. Use a suitable format for photos versus line art; do not assume every platform or decoder accepts every format.
- Keep a stable ratio or fixed bounds for images that arrive later. The loading and failure views should occupy the same region.
- GPUI's default image cache already handles ordinary repeated sources. Introduce a custom cache only for a measured lifetime or memory requirement.
- For a large scrolling gallery, create only the visible or nearby items using the collection APIs. The `img()` call alone is not a lazy-loading policy.
- Pair informative images with visible descriptive text or an accessible description in the surrounding UI. Decorative images need no duplicated narration. Make image-driven commands real controls with names, focus, and keyboard activation.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Blank embedded image | Confirm that the registered `AssetSource` includes the exact key and file bytes. `img("images/cover.png")` is an embedded lookup, not a file-system read. |
| Default icon missing | Register GPUI Kit's default `Assets` or compose it after your app source. |
| Full-color SVG changes to one color | Render it with `img()`; `svg().path(...)` intentionally uses an alpha mask. |
| Image appears cropped | Use `ObjectFit::Contain`, or increase the display bounds. |
| Fallback appears | Check the file path, network response, or decoded bytes for the chosen source type. |

For an image that conveys information, provide adjacent text or another accessible description in the surrounding UI. A filename is not a user-facing description.
