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

To reuse responses across views and launches, wrap the application's `HttpClient` in a client that applies HTTP caching rules, and install the wrapper once at startup. Every remote image then benefits, including images loaded by code the application does not own. Follow these rules:

- **Cache only a GET without a request body.** Pass every other request through unchanged.
- **Treat the cache as shared.** One client serves the whole application: its own views, extensions, and document views. Do not store a `no-store` or `private` response, or a response to a request carrying `Authorization`, unless the server explicitly allows a shared cache to keep it. A layer below the cache that adds cookies or tokens hides them from the cache, so add credentials above the cache, or leave those hosts uncached.
- **Revalidate instead of downloading again.** Serve a fresh response without the network. When it is stale, send `If-None-Match` or `If-Modified-Since`; on `304 Not Modified`, return the stored body with the refreshed headers.
- **Honor the caller's redirect policy.** `img()` follows redirects, but a loader that authorizes each hop itself requests `RedirectPolicy::NoFollow` and must receive the `3xx` response. Keep the policy in the cache key so a followed result never answers that caller, and cache the redirect response itself by the same rules.
- **Bound memory and disk use.** Cap each entry and the total size, evict old entries, and stream a response that is too large to keep straight through.
- **Authorize before the request.** The cache answers any caller that asks for the same URL. A check that decides whether a caller may reach a URL, such as the network grants of gpui-shell scripts, must run before `send`. The cache then never widens what a caller can reach; it only avoids repeating a request that was already allowed.

### Example

The example below applies these rules in memory. The [http-cache-semantics](https://crates.io/crates/http-cache-semantics) crate implements the HTTP caching rules: freshness, validators, `Vary`, and the restrictions on a shared cache. Add `http-cache-semantics = "2"`, `futures`, `bytes`, and `anyhow` to `Cargo.toml`.

```rust
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::SystemTime,
};

use bytes::Bytes;
use futures::{AsyncReadExt as _, FutureExt as _, future::BoxFuture, io::Cursor};
use gpui_kit::http_client::{
    AsyncBody, HttpClient, Inner, Method, RedirectPolicy, Request, Response, Url,
    http::{HeaderValue, response},
};
use http_cache_semantics::{AfterResponse, BeforeRequest, CachePolicy};

/// An [`HttpClient`] that answers GET requests from memory when HTTP caching
/// rules allow it, and forwards everything else to `inner`.
pub struct CachingHttpClient {
    inner: Arc<dyn HttpClient>,
    store: Arc<Mutex<Store>>,
}

impl CachingHttpClient {
    pub fn new(inner: Arc<dyn HttpClient>, max_bytes: usize) -> Self {
        let store = Store {
            max_bytes,
            ..Default::default()
        };
        Self {
            inner,
            store: Arc::new(Mutex::new(store)),
        }
    }
}

/// The URI plus the caller's redirect policy. A caller that disables
/// redirects must receive the 3xx itself, never a followed result.
type Key = (String, Option<RedirectPolicy>);

struct Entry {
    policy: CachePolicy,
    body: Bytes,
}

#[derive(Default)]
struct Store {
    entries: HashMap<Key, Entry>,
    /// Keys in insertion order; the oldest is evicted first.
    order: VecDeque<Key>,
    bytes: usize,
    max_bytes: usize,
}

impl Store {
    /// A single response may use at most an eighth of the budget.
    fn max_entry_bytes(&self) -> usize {
        self.max_bytes / 8
    }

    fn get(&self, key: &Key) -> Option<(CachePolicy, Bytes)> {
        let entry = self.entries.get(key)?;
        Some((entry.policy.clone(), entry.body.clone()))
    }

    fn insert(&mut self, key: Key, policy: CachePolicy, body: Bytes) {
        self.remove(&key);
        self.bytes += body.len();
        self.order.push_back(key.clone());
        self.entries.insert(key, Entry { policy, body });
        while self.bytes > self.max_bytes {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(entry) = self.entries.remove(&oldest) {
                self.bytes -= entry.body.len();
            }
        }
    }

    fn remove(&mut self, key: &Key) {
        if let Some(entry) = self.entries.remove(key) {
            self.bytes -= entry.body.len();
            self.order.retain(|k| k != key);
        }
    }
}

fn respond(head: response::Parts, body: Bytes) -> Response<AsyncBody> {
    Response::from_parts(head, AsyncBody::from_bytes(body))
}

impl HttpClient for CachingHttpClient {
    fn user_agent(&self) -> Option<&HeaderValue> {
        self.inner.user_agent()
    }

    fn proxy(&self) -> Option<&Url> {
        self.inner.proxy()
    }

    fn send(
        &self,
        req: Request<AsyncBody>,
    ) -> BoxFuture<'static, anyhow::Result<Response<AsyncBody>>> {
        // Only a GET without a body is cacheable; pass everything else through.
        if req.method() != Method::GET || !matches!(req.body().0, Inner::Empty) {
            return self.inner.send(req);
        }

        let inner = self.inner.clone();
        let store = self.store.clone();
        async move {
            let (request, _) = req.into_parts();
            let redirects = request.extensions.get::<RedirectPolicy>().cloned();
            let key = (request.uri.to_string(), redirects);
            let cached = store.lock().unwrap().get(&key);

            // Fresh: answer without the network. Stale: send the conditional
            // headers (If-None-Match / If-Modified-Since) the policy computed,
            // keeping the caller's extensions such as its redirect policy.
            let mut outgoing = request.clone();
            if let Some((policy, body)) = &cached {
                match policy.before_request(&request, SystemTime::now()) {
                    BeforeRequest::Fresh(head) => return Ok(respond(head, body.clone())),
                    BeforeRequest::Stale {
                        request: revalidation,
                        ..
                    } => {
                        outgoing.headers = revalidation.headers;
                    }
                }
            }

            let (head, body) = inner
                .send(Request::from_parts(outgoing, AsyncBody::empty()))
                .await?
                .into_parts();

            // 304: keep the cached body under the refreshed headers.
            if let Some((policy, cached_body)) = cached {
                match policy.after_response(&request, &head, SystemTime::now()) {
                    AfterResponse::NotModified(policy, head) => {
                        let mut store = store.lock().unwrap();
                        store.insert(key, policy, cached_body.clone());
                        return Ok(respond(head, cached_body));
                    }
                    AfterResponse::Modified(..) => {}
                }
            }

            // `CachePolicy::new` evaluates the response as a shared cache:
            // `no-store` and `private` responses, and most responses to requests
            // carrying `Authorization`, are not storable.
            let policy = CachePolicy::new(&request, &head);
            if !policy.is_storable() {
                store.lock().unwrap().remove(&key);
                return Ok(Response::from_parts(head, body));
            }

            let limit = store.lock().unwrap().max_entry_bytes();
            let mut bytes = Vec::new();
            let mut reader = body.take(limit as u64 + 1);
            reader.read_to_end(&mut bytes).await?;
            let body = reader.into_inner();
            if bytes.len() > limit {
                // Too large to keep: return what was read, then the rest of the stream.
                store.lock().unwrap().remove(&key);
                let rest = Cursor::new(bytes).chain(body);
                return Ok(Response::from_parts(head, AsyncBody::from_reader(rest)));
            }

            let bytes = Bytes::from(bytes);
            store.lock().unwrap().insert(key, policy, bytes.clone());
            Ok(respond(head, bytes))
        }
        .boxed()
    }
}
```

### Install the client

Install the wrapper around the client the application already uses. Here `reqwest_client` is the `gpui-pre-reqwest-client` crate at the GPUI snapshot version your `gpui-kit` release pins:

```rust
use std::sync::Arc;

gpui_kit::application().run(|cx| {
    gpui_kit::init(cx);

    let network = reqwest_client::ReqwestClient::user_agent("my-app/1.0")
        .expect("failed to create the HTTP client");
    let cached = CachingHttpClient::new(Arc::new(network), 64 * 1024 * 1024);
    cx.set_http_client(Arc::new(cached));

    // Open windows here.
});
```

### Extend the example

The example keeps its scope small. Extend it where your application needs more:

- **Persistence.** Entries disappear when the application quits. To keep images across launches, store each body with its `CachePolicy` on disk (`CachePolicy` implements `serde` traits), write files atomically, and remove entries past a size or age limit at startup. A client built on `reqwest` can instead use caching middleware with a disk store, such as [http-cache-reqwest](https://crates.io/crates/http-cache-reqwest).
- **Variants.** Each URL keeps one response. A response that differs by `Vary` replaces the previous variant.
- **Duplicate requests.** Concurrent misses for the same URL each reach the network. GPUI's image loader already shares one load per source.
- **Eviction.** The oldest entry is removed first, whether or not it was used recently. Use an LRU structure when access patterns matter.

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
