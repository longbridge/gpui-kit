---
title: Image
description: Display embedded, local, and remote images with sizing, loading, and error states.
---

# Image

GPUI's `img()` draws an image, and `svg()` draws a single-color icon. GPUI Kit re-exports both from `gpui_kit`. This page shows the patterns an application uses most; [Images](../docs/image.md) explains sources, loading, sizing, `svg()`, caching, and HTTP caching in detail.

## Start with a working image

This complete native `src/main.rs` uses an icon already bundled by GPUI Kit, so it needs no extra asset file. Add `gpui-kit = "0.6"` to `Cargo.toml`. The same asset is shown as a color-preserving image and as a monochrome SVG.

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

`with_assets(Assets)` registers the default component icons; register your own `AssetSource` to embed your images (see [Icons & Assets](../docs/assets.md)). A string such as `"images/cover.png"` is an asset key, a `Path` reads a file, and a URL is fetched through the App's `HttpClient`; see [Sources](../docs/image.md#sources).

## Common patterns

A thumbnail that fills a fixed box and crops its edges:

```rust
img("images/cover.png")
    .w(px(320.))
    .h(px(180.))
    .object_fit(ObjectFit::Cover)
```

A banner that follows the parent width and reserves its height before the image arrives:

```rust
img("images/banner.webp")
    .w_full()
    .aspect_ratio(16. / 9.)
    .object_fit(ObjectFit::Cover)
```

A remote image with loading and failure states. The `.id(...)` is required for the loading state to appear:

```rust
img("https://example.com/avatar.png")
    .id("avatar")
    .size(px(48.))
    .rounded_full()
    .with_loading(|| div().child("Loading image...").into_any_element())
    .with_fallback(|| div().child("Image unavailable").into_any_element())
```

Use `ObjectFit::Contain` (the default) for logos and diagrams that must stay fully visible. To keep a multicolor SVG's colors, draw it with `img()`; for an icon that follows the theme, use [Icon](./icon.md). See [Size and fit](../docs/image.md#size-and-fit) and [img() or svg()](../docs/image.md#img-or-svg).

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

## Accessibility

- Pair an informative image with visible text or an accessible description in the surrounding UI. A file name is not a description. Decorative images need no narration.
- Make an image that triggers a command a real control, such as a `Button`, so it has a name, focus, and keyboard activation.
- Keep the loading and failure views in the same box as the image, so nearby content does not move.

## Learn more

- [Images](../docs/image.md): every `img()` source, loading and decoding, `svg()`, and troubleshooting.
- [Caches for decoded images](../docs/image.md#caches-for-decoded-images): scoped and custom `ImageCache`s.
- [Cache remote images over HTTP](../docs/image.md#cache-remote-images-over-http): reuse downloads across views and launches.
