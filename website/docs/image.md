---
title: Images
description: How img() and svg() load, decode, size, and cache images, and how to cache remote images over HTTP.
order: -6.9
---

# Images

GPUI draws images with two elements. `img()` draws a full-color image: a photo, a screenshot, an avatar, or a multicolor SVG. `svg()` draws a single-color SVG as a mask filled with a text color, which is how icons follow the theme. Both are re-exported from `gpui_kit`. This page explains how each element finds, decodes, sizes, and caches its source, and how to keep remote images from being downloaded again. For ready-made UI patterns see the [Image](../component/image.md) component page; for bundling files into the binary see [Icons & Assets](./assets.md).

## Sources

`img(source)` takes anything that converts into `ImageSource`. The conversion decides where the bytes come from:

| Argument | `ImageSource` | Where the bytes come from |
| --- | --- | --- |
| `"https://example.com/a.png"`, or any string that parses as a URL; a `SharedUri` | `Resource(Resource::Uri)` | The `HttpClient` installed on the `App` |
| `"images/a.png"`, or any other string | `Resource(Resource::Embedded)` | The registered `AssetSource`, looked up by that exact key |
| `&Path`, `PathBuf`, `Arc<Path>` | `Resource(Resource::Path)` | The file system |
| `Arc<Image>` | `Image` | Encoded bytes you already hold, with their `ImageFormat` |
| `Arc<RenderImage>` | `Render` | Frames you already decoded; drawn as is |
| `Fn(&mut Window, &mut App) -> Option<Result<Arc<RenderImage>, ImageCacheError>>` | `Custom` | Your own loader |

A string is a URL whenever it parses as one, and an asset key otherwise. A relative-looking string such as `"images/a.png"` is never read from the working directory. Pass a `Path` for a file on disk, so it is never taken for a URL or an asset key.

On native platforms the default `HttpClient` fails every request, so URL images load only after the application installs a client with `cx.set_http_client(...)` or `Application::with_http_client(...)`. On the web, `gpui_kit::application()` installs a client backed by the browser's Fetch API.

A `Custom` loader runs during layout and paint of every frame. Return `None` while the image is loading and keep the result yourself, for example with `window.use_asset::<YourAsset>(...)`, so that each frame does not start a new load.

## Loading, decoding, and failures

Loading is asynchronous. While it is pending, `img()` lays out from its own style and draws nothing. When the load finishes, GPUI redraws the view that drew the image.

```rust
img("https://example.com/cover.png")
    .id("cover")
    .w(px(320.))
    .h(px(180.))
    .with_loading(|| div().child("Loading image...").into_any_element())
    .with_fallback(|| div().child("Image unavailable").into_any_element())
```

- `with_loading` replaces the image only when the load is still pending 200 ms after it started, so a fast load never flashes a placeholder. GPUI tracks that time in the element's state, so the placeholder appears only on an image with an `.id(...)`.
- `with_fallback` replaces the image when loading fails: a missing asset key or file, a network error, a response that is not `2xx`, or bytes that do not decode.
- Neither callback retries. To retry, keep the attempt in your view state and draw the image again with a different source, or remove the cached failure (see [Caches for decoded images](#caches-for-decoded-images)).

GPUI detects the format from the bytes, not from the file extension. PNG, JPEG, WebP, GIF, BMP, TIFF, ICO and the other formats listed by `Img::extensions()` decode through the `image` crate. Bytes that are not a known raster format are parsed as SVG and rasterized once at twice their intrinsic size, so `img()` of an SVG stays sharp at normal scales but blurs when it is enlarged far beyond its own size.

Animated GIF and WebP images play only on an image with an `.id(...)`, because the current frame is kept in element state. They advance while the window is active and stop when the system asks to reduce motion.

## Size and fit

Layout decides the image's bounds; `object_fit` decides how the image is drawn inside them.

When a dimension is `auto`, GPUI fills it in after the image has decoded. If the other dimension has an absolute length, the image's ratio gives this one; otherwise the image's own size is used. The image's ratio also becomes the element's `aspect_ratio` unless you set one. Before decoding finishes there is nothing to measure, so an image without a size moves the surrounding layout when it arrives. Reserve its box with an explicit size, or with a width and `aspect_ratio(...)`:

```rust
img("images/banner.webp")
    .w_full()
    .aspect_ratio(16. / 9.)
    .object_fit(ObjectFit::Cover)
```

| `ObjectFit` | Result |
| --- | --- |
| `Contain` (default) | The whole image, aspect ratio kept; empty space may remain. |
| `Cover` | Fills the bounds, aspect ratio kept; edges may be cropped. |
| `Fill` | Stretches to the bounds; may distort. |
| `ScaleDown` | Like `Contain`, but never enlarges the image. |
| `None` | The image's own size, centered. |

`.rounded(...)` on an `img()` rounds the drawn image itself. `.grayscale(true)` draws it without color. Borders, shadows and backgrounds are ordinary element styles.

## svg()

`svg()` draws an SVG as a single-color shape. GPUI rasterizes the SVG's alpha channel at the element's size and fills it with a color, so the SVG's own colors are discarded. Choose the source with one of three builders:

| Builder | Where the bytes come from |
| --- | --- |
| `.path("icons/check.svg")` | The registered `AssetSource`, by key |
| `.external_path("/path/to/check.svg")` | The file system, read asynchronously and cached by path |
| `.data(bytes)` | Bytes you pass in, cached by a hash of the bytes |

```rust
svg()
    .path("icons/check.svg")
    .size(px(16.))
    .text_color(cx.theme().foreground)
```

- **Set the color on the element.** `svg()` paints only when the element itself has a text color. A color inherited from a parent does not count, so an `svg()` without `.text_color(...)` draws nothing.
- **Set the size.** Layout never reads the SVG, so `svg()` has no intrinsic size and collapses to zero without one.
- **Transform at paint time.** `.with_transformation(Transformation::rotate(percentage(0.25)))`, and the `scale` and `translate` variants, move only the drawing. Layout and the hit area stay where they were.

In GPUI Kit, prefer the [Icon](../component/icon.md) component for icons: it picks the size from the component size scale and the color from the theme.

## img() or svg()

| | `img()` | `svg()` |
| --- | --- | --- |
| Colors | Keeps the source's colors | One color, from `.text_color(...)` |
| Formats | Raster formats and SVG | SVG only |
| Size | Intrinsic size from the image | Must be set |
| Sources | URL, asset key, path, bytes, decoded frames, custom loader | Asset key, path, bytes |
| Loading and failure | `with_loading`, `with_fallback` | Draws nothing until ready or on failure |
| Use for | Photos, avatars, logos, illustrations | Icons and glyphs that follow the theme or a state |

`.text_color(...)` does not recolor an `img()`, and `svg()` cannot keep a multicolor logo's colors.

## Caches for decoded images

Decoded images are kept so that drawing the same source again does not load it again. Which cache keeps them decides when they are released.

- **Default.** An `img()` of a `Resource` (URL, asset key, or path) uses the App's asset cache, keyed by the source. One entry serves every window and view and stays until you remove it with `ImageSource::remove_asset(cx)`. Failed loads are cached too, so remove the entry before retrying the same source. An `Arc<Image>` is cached the same way.
- **A scoped cache.** `image_cache(provider)` wraps children in an element whose `img()` descendants use that cache instead. `image_cache(retain_all("preview"))` keeps a `RetainAllImageCache` in element state: it holds everything it loaded and releases it when the element stops being drawn. `img(...).image_cache(&cache)` picks a cache for one image.
- **Your own cache.** Implement `ImageCache` to decide what to keep. `ImageCacheItem::new(resource, cx)` starts a load through GPUI's image loader, and `item.use_image(window)` returns the result and redraws the current view when it finishes. When you evict an image, release its GPU texture with `cx.drop_image(image, Some(window))`.

The cache below keeps the most recently drawn images and releases the rest. Its capacity must exceed the number of images on screen at once, or visible images will be evicted and reloaded every frame.

```rust
use std::{collections::VecDeque, sync::Arc};

use gpui_kit::*;

/// Keeps the `capacity` most recently drawn images and releases the rest.
pub struct RecentImageCache {
    capacity: usize,
    items: VecDeque<(Resource, ImageCacheItem)>,
}

impl RecentImageCache {
    pub fn new(capacity: usize, cx: &mut App) -> Entity<Self> {
        let cache = cx.new(|_| Self {
            capacity,
            items: VecDeque::new(),
        });
        // Release the GPU textures when the cache itself is dropped.
        cx.observe_release(&cache, |cache, cx| {
            for (_, item) in cache.items.drain(..) {
                if let Some(Ok(image)) = item.get() {
                    cx.drop_image(image, None);
                }
            }
        })
        .detach();
        cache
    }
}

impl ImageCache for RecentImageCache {
    fn load(
        &mut self,
        resource: &Resource,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Result<Arc<RenderImage>, ImageCacheError>> {
        let item = match self.items.iter().position(|(source, _)| source == resource) {
            Some(ix) => self.items.remove(ix).expect("index is in bounds"),
            None => (resource.clone(), ImageCacheItem::new(resource, cx)),
        };
        self.items.push_front(item);
        while self.items.len() > self.capacity {
            if let Some((_, evicted)) = self.items.pop_back()
                && let Some(Ok(image)) = evicted.get()
            {
                cx.drop_image(image, Some(window));
            }
        }
        self.items[0].1.use_image(window)
    }
}
```

Create it once, keep the `Entity` in your view, and wrap the images that should use it:

```rust
image_cache(self.images.clone())
    .flex()
    .gap_2()
    .children(self.urls.iter().map(|url| img(url.clone()).size(px(96.))))
```

These caches hold decoded images in memory. Every one of them still loads a URL through the App's `HttpClient`, and none of them remembers what the server said about the response. That is the job of the next section.

## Cache remote images over HTTP

The previous sections cover what `img()` keeps in memory. This section covers the network: how to reuse responses across views and launches on native platforms. On the web the browser's HTTP cache already applies.

### Why GPUI's image cache is not enough

GPUI's image cache sits above the App's `HttpClient`. It keeps decoded images in memory, keyed by source, which is enough for repeated sources in a running view but not for network traffic:

- It ends with the process, so every launch downloads every image again.
- It ignores `Cache-Control`, `ETag`, and `Last-Modified`. It cannot keep a response the server allows to be reused, or ask the server whether an older copy is still current.
- It lives only as long as the cache that holds it. An image with its own `.image_cache(...)`, or a loader that keeps a cache per view, requests the image again each time a new view is created.

### Cache at the HTTP layer

To reuse responses across views and launches, install an `HttpClient` that applies HTTP caching rules once at startup. Every remote image then benefits, including images loaded by code the application does not own. Follow these rules:

- **Cache only a GET without a request body.** Pass every other request through unchanged.
- **Treat the cache as shared.** One client serves the whole application: its own views, extensions, and document views. Do not store a `no-store` or `private` response, or a response to a request carrying `Authorization`, unless the server explicitly allows a shared cache to keep it. A layer below the cache that adds cookies or tokens hides them from the cache, so add credentials above the cache, or leave those hosts uncached.
- **Revalidate instead of downloading again.** Serve a fresh response without the network. When it is stale, send `If-None-Match` or `If-Modified-Since`; on `304 Not Modified`, return the stored body with the refreshed headers.
- **Honor the caller's redirect policy.** `img()` follows redirects, but a loader that authorizes each hop itself requests `RedirectPolicy::NoFollow` and must receive the `3xx` response. Keep a separate cache for each policy, so a followed result never answers that caller.
- **Bound memory and disk use.** Cap the cache's size and remove old entries.
- **Authorize before the request.** The cache answers any caller that asks for the same URL. A check that decides whether a caller may reach a URL, such as the network grants of gpui-shell scripts, must run before `send`. The cache then never widens what a caller can reach; it only avoids repeating a request that was already allowed.

### Example

[http-cache-reqwest](https://crates.io/crates/http-cache-reqwest) implements these rules as [reqwest-middleware](https://crates.io/crates/reqwest-middleware): freshness, `ETag` and `Last-Modified` revalidation, and the restrictions on a shared cache, with a disk store. The application only adapts it to GPUI's `HttpClient`. Add these dependencies:

```toml
[dependencies]
anyhow = "1"
futures = "0.3"
http-cache-reqwest = "0.15"
reqwest = { version = "0.12", features = ["stream"] }
reqwest-middleware = "0.4"
tokio = { version = "1", features = ["rt-multi-thread"] }
```

```rust
use std::{path::Path, sync::LazyLock};

use futures::{AsyncReadExt as _, FutureExt as _, TryStreamExt as _, future::BoxFuture};
use gpui_kit::http_client::{
    AsyncBody, HttpClient, RedirectPolicy, Request, Response, Url, http::HeaderValue,
};
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest::redirect;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};

/// reqwest needs a tokio runtime; GPUI's executors are not one.
static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("failed to start the HTTP runtime")
});

/// A reqwest client with an HTTP cache on disk.
pub struct CachedHttpClient {
    follow: ClientWithMiddleware,
    no_follow: ClientWithMiddleware,
}

impl CachedHttpClient {
    pub fn new(cache_dir: &Path) -> anyhow::Result<Self> {
        let client = |policy, dir| -> anyhow::Result<_> {
            let client = reqwest::Client::builder().redirect(policy).build()?;
            Ok(ClientBuilder::new(client)
                .with(Cache(HttpCache {
                    mode: CacheMode::Default,
                    manager: CACacheManager {
                        path: cache_dir.join(dir),
                    },
                    options: HttpCacheOptions::default(),
                }))
                .build())
        };
        // A caller that disables redirects must receive the 3xx itself. It
        // gets a client that never follows them, with a cache of its own.
        Ok(Self {
            follow: client(redirect::Policy::default(), "follow")?,
            no_follow: client(redirect::Policy::none(), "no-follow")?,
        })
    }
}

impl HttpClient for CachedHttpClient {
    fn user_agent(&self) -> Option<&HeaderValue> {
        None
    }

    fn proxy(&self) -> Option<&Url> {
        None
    }

    fn send(
        &self,
        req: Request<AsyncBody>,
    ) -> BoxFuture<'static, anyhow::Result<Response<AsyncBody>>> {
        let (head, mut body) = req.into_parts();
        let client = match head.extensions.get::<RedirectPolicy>() {
            Some(RedirectPolicy::NoFollow) => self.no_follow.clone(),
            _ => self.follow.clone(),
        };
        async move {
            let mut bytes = Vec::new();
            body.read_to_end(&mut bytes).await?;
            let request = client
                .request(head.method, head.uri.to_string())
                .headers(head.headers)
                .body(bytes);
            let response = RUNTIME.spawn(request.send()).await??;

            let mut builder = Response::builder()
                .status(response.status())
                .version(response.version());
            *builder.headers_mut().unwrap() = response.headers().clone();
            let body = response
                .bytes_stream()
                .map_err(std::io::Error::other)
                .into_async_read();
            Ok(builder.body(AsyncBody::from_reader(body))?)
        }
        .boxed()
    }
}
```

GPUI's own reqwest client applies the caller's `RedirectPolicy` to each request, but a `reqwest-middleware` client fixes the policy when it is built. The example therefore builds two clients: one that follows redirects, for `img()`, and one that never does, for callers that request `RedirectPolicy::NoFollow`. Each has its own cache directory, because a cache in front of a following client stores the final response under the original URL and would answer the other caller with it. `RedirectPolicy::FollowLimit` uses reqwest's default limit of 10.

### Install the client

Install the client once at startup, before any window loads a remote image. Use your platform's cache directory, for example from the [dirs](https://crates.io/crates/dirs) crate, instead of the temporary directory used here:

```rust
use std::sync::Arc;

gpui_kit::application().run(|cx| {
    gpui_kit::init(cx);

    let cache_dir = std::env::temp_dir().join("my-app/http-cache");
    let client = CachedHttpClient::new(&cache_dir).expect("failed to create the HTTP client");
    cx.set_http_client(Arc::new(client));

    // Open windows here.
});
```

### Extend the example

- **Size.** The disk store has no size limit. Remove old entries at startup, or clear the directory when it grows past a limit.
- **Proxy and user agent.** Configure them on the `reqwest::Client::builder()`, and return them from `proxy()` and `user_agent()` when other code reads them.
- **Request bodies.** The example reads a request body into memory before sending it. That is fine for images and small requests; wrap the stream with `reqwest::Body::wrap_stream` for large uploads.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| An embedded image is blank | The registered `AssetSource` must contain the exact key. `img("images/a.png")` is an asset lookup, not a file read. |
| Every URL image shows the fallback on native | Install an `HttpClient`; the default one fails every request. |
| `with_loading` never appears, or a GIF does not play | Give the `img()` an `.id(...)`. |
| An `svg()` is invisible | Set `.text_color(...)` and a size on the `svg()` element itself. |
| A multicolor SVG turns into one color | Draw it with `img()`; `svg()` always draws a single color. |
| The layout jumps when an image arrives | Reserve its box with a size, or a width and `aspect_ratio(...)`. |
| A fixed image still shows the old failure | The failure is cached; call `ImageSource::remove_asset(cx)` before drawing it again. |
| Images download again in every new view or after a restart | Cache at the HTTP layer; see [Cache remote images over HTTP](#cache-remote-images-over-http). |
