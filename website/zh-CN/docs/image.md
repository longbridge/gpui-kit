---
title: 图片
description: img() 与 svg() 如何加载、解码、布局和缓存图片，以及如何在 HTTP 层缓存远程图片。
order: -6.9
---

# 图片

GPUI 用两个元素绘制图片。`img()` 绘制全彩图片：照片、截图、头像或多色 SVG。`svg()` 把 SVG 绘制成单色图形，用文字颜色填充，图标就是这样跟随主题变色的。两者都从 `gpui_kit` 重新导出。本页说明这两个元素如何查找、解码、布局和缓存图片，以及如何避免重复下载远程图片。现成的界面写法见 [Image](../component/image.md) 组件页；把文件打包进程序见[图标与资源](./assets.md)。

## 图片来源

`img(source)` 接受任何能转换为 `ImageSource` 的值，转换结果决定从哪里读取字节：

| 参数 | `ImageSource` | 字节来源 |
| --- | --- | --- |
| `"https://example.com/a.png"` 或任何能解析为 URL 的字符串；`SharedUri` | `Resource(Resource::Uri)` | `App` 上安装的 `HttpClient` |
| `"images/a.png"` 或其他字符串 | `Resource(Resource::Embedded)` | 已注册的 `AssetSource`，按这个键精确查找 |
| `&Path`、`PathBuf`、`Arc<Path>` | `Resource(Resource::Path)` | 文件系统 |
| `Arc<Image>` | `Image` | 已持有的编码字节及其 `ImageFormat` |
| `Arc<RenderImage>` | `Render` | 已解码的帧，直接绘制 |
| `Fn(&mut Window, &mut App) -> Option<Result<Arc<RenderImage>, ImageCacheError>>` | `Custom` | 自己的加载器 |

字符串只要能解析为 URL 就按 URL 处理，否则就是资源键。`"images/a.png"` 这样看起来像相对路径的字符串，不会从工作目录读取。磁盘上的文件请传 `Path`，这样既不会被当成 URL，也不会被当成资源键。

在原生平台上，默认的 `HttpClient` 会让所有请求失败，所以应用要先用 `cx.set_http_client(...)` 或 `Application::with_http_client(...)` 安装客户端，URL 图片才能加载。在 Web 上，`gpui_kit::application()` 会安装基于浏览器 Fetch API 的客户端。

`Custom` 加载器在每一帧的布局和绘制阶段都会运行。图片还在加载时返回 `None`，结果要自己保存，例如用 `window.use_asset::<YourAsset>(...)`，否则每一帧都会重新开始加载。

## 加载、解码与失败

加载是异步的。加载期间，`img()` 按自身样式布局，但不绘制任何内容。加载完成后，GPUI 会重绘绘制这张图片的视图。

```rust
img("https://example.com/cover.png")
    .id("cover")
    .w(px(320.))
    .h(px(180.))
    .with_loading(|| div().child("Loading image...").into_any_element())
    .with_fallback(|| div().child("Image unavailable").into_any_element())
```

- `with_loading` 只在加载开始 200 ms 后仍未完成时才替换图片，所以加载很快时不会闪出占位内容。GPUI 在元素状态里记录这个时间，因此只有带 `.id(...)` 的图片才会显示占位内容。
- 加载失败时，`with_fallback` 会替换图片。失败的情况包括：资源键或文件不存在、网络错误、响应不是 `2xx`、字节无法解码。
- 两个回调都不会重试。要重试，在视图状态里记录重试，再换一个来源重新绘制图片，或者先删掉缓存的失败结果（见[解码后图片的缓存](#解码后图片的缓存)）。

GPUI 根据字节内容判断格式，而不是根据文件扩展名。PNG、JPEG、WebP、GIF、BMP、TIFF、ICO 以及 `Img::extensions()` 列出的其他格式，都通过 `image` crate 解码。不属于已知位图格式的字节会按 SVG 解析，并按其固有尺寸的两倍光栅化一次。所以用 `img()` 显示的 SVG 在正常缩放下是清晰的，但放大到远超自身尺寸时会变模糊。

GIF 和 WebP 动图只有在图片带 `.id(...)` 时才会播放，因为当前帧保存在元素状态里。窗口处于活动状态时动图才会播放，系统要求减少动态效果时则停止。

## 尺寸与适配

布局决定图片的边界，`object_fit` 决定图片在边界内如何绘制。

某个维度为 `auto` 时，GPUI 会在图片解码后补上它。如果另一个维度是绝对长度，就按图片比例算出这个维度；否则使用图片自身的尺寸。除非另外设置，图片比例也会成为元素的 `aspect_ratio`。解码完成前没有尺寸可以测量，所以没有设置尺寸的图片在加载完成时会推动周围的布局。可以用明确的尺寸，或者宽度加 `aspect_ratio(...)`，预先占好位置：

```rust
img("images/banner.webp")
    .w_full()
    .aspect_ratio(16. / 9.)
    .object_fit(ObjectFit::Cover)
```

| `ObjectFit` | 效果 |
| --- | --- |
| `Contain`（默认） | 显示完整图片，保持宽高比；可能留有空白。 |
| `Cover` | 填满边界，保持宽高比；边缘可能被裁掉。 |
| `Fill` | 拉伸到边界大小；可能变形。 |
| `ScaleDown` | 与 `Contain` 相同，但不会放大图片。 |
| `None` | 按图片自身尺寸居中显示。 |

在 `img()` 上设置 `.rounded(...)` 会让绘制出的图片本身带圆角。`.grayscale(true)` 以灰度绘制。边框、阴影和背景都是普通的元素样式。

## svg()

`svg()` 把 SVG 绘制成单色图形。GPUI 按元素尺寸光栅化 SVG 的透明度通道，再用一种颜色填充，所以 SVG 自带的颜色会被丢弃。用以下三个方法之一指定来源：

| 方法 | 字节来源 |
| --- | --- |
| `.path("icons/check.svg")` | 已注册的 `AssetSource`，按键查找 |
| `.external_path("/path/to/check.svg")` | 文件系统，异步读取并按路径缓存 |
| `.data(bytes)` | 传入的字节，按字节的哈希缓存 |

```rust
svg()
    .path("icons/check.svg")
    .size(px(16.))
    .text_color(cx.theme().foreground)
```

- **在元素本身上设置颜色。** 只有元素自己设置了文字颜色，`svg()` 才会绘制。从父元素继承的颜色不算，所以没有 `.text_color(...)` 的 `svg()` 什么也不画。
- **设置尺寸。** 布局阶段不会读取 SVG，所以 `svg()` 没有固有尺寸，不设尺寸就会缩成零。
- **变换只影响绘制。** `.with_transformation(Transformation::rotate(percentage(0.25)))` 以及 `scale`、`translate` 只移动绘制结果，布局和点击区域保持不变。

在 GPUI Kit 中，图标优先使用 [Icon](../component/icon.md) 组件：它按组件尺寸体系确定大小，并从主题取颜色。

## 选择 img() 还是 svg()

| | `img()` | `svg()` |
| --- | --- | --- |
| 颜色 | 保留原有颜色 | 单色，来自 `.text_color(...)` |
| 格式 | 位图格式和 SVG | 只支持 SVG |
| 尺寸 | 使用图片的固有尺寸 | 必须设置 |
| 来源 | URL、资源键、路径、字节、已解码的帧、自定义加载器 | 资源键、路径、字节 |
| 加载与失败 | `with_loading`、`with_fallback` | 准备好之前或失败时不绘制 |
| 适用于 | 照片、头像、Logo、插画 | 跟随主题或状态变色的图标和符号 |

`.text_color(...)` 不能给 `img()` 改色，`svg()` 也无法保留多色 Logo 的颜色。

## 解码后图片的缓存

解码后的图片会被保留，再次绘制同一来源时不必重新加载。由哪个缓存保留，决定了图片何时被释放。

- **默认缓存。** `Resource` 来源（URL、资源键或路径）的 `img()` 使用 App 的资源缓存，按来源作为键。一个条目供所有窗口和视图共用，直到用 `ImageSource::remove_asset(cx)` 删除为止。加载失败的结果也会被缓存，所以重试同一来源前要先删除这个条目。`Arc<Image>` 也以同样的方式缓存。
- **限定范围的缓存。** `image_cache(provider)` 包裹子元素，其中的 `img()` 改用这个缓存。`image_cache(retain_all("preview"))` 在元素状态里保存一个 `RetainAllImageCache`：它保留加载过的所有图片，元素不再绘制时一并释放。`img(...).image_cache(&cache)` 为单张图片指定缓存。
- **自定义缓存。** 实现 `ImageCache` 来决定保留哪些图片。`ImageCacheItem::new(resource, cx)` 通过 GPUI 的图片加载器开始加载，`item.use_image(window)` 返回结果，并在加载完成时重绘当前视图。淘汰图片时，用 `cx.drop_image(image, Some(window))` 释放它的 GPU 纹理。

下面的缓存保留最近绘制过的图片，释放其余的。容量必须大于同时显示在屏幕上的图片数量，否则可见的图片会被淘汰，每一帧都要重新加载。

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

创建一次，把 `Entity` 保存在视图里，再用它包裹需要使用这个缓存的图片：

```rust
image_cache(self.images.clone())
    .flex()
    .gap_2()
    .children(self.urls.iter().map(|url| img(url.clone()).size(px(96.))))
```

这些缓存都在内存中保存解码后的图片。它们仍然通过 App 的 `HttpClient` 加载 URL，也都不会记住服务器对响应的缓存要求，这正是下一节要解决的问题。

## 远程图片的 HTTP 缓存

前面几节讲的是 `img()` 在内存中保存什么。本节讲网络层：在原生平台上如何跨视图、跨启动复用响应。在 Web 上，浏览器的 HTTP 缓存已经生效。

### GPUI 图片缓存的局限

GPUI 的图片缓存位于 App 的 `HttpClient` 之上。它在内存中按来源保存解码后的图片，足以应付运行中视图里的重复来源，却无法减少网络请求：

- 进程退出后缓存随之消失，每次启动都要重新下载所有图片。
- 它不理会 `Cache-Control`、`ETag` 和 `Last-Modified`，既不能保留服务器允许复用的响应，也不能向服务器确认旧副本是否仍然有效。
- 它的寿命取决于持有它的缓存。使用独立 `.image_cache(...)` 的图片，或按视图分别缓存的加载器，每创建一个新视图都会重新请求图片。

### 在 HTTP 层缓存

要跨视图、跨启动复用响应，可以在启动时安装一个遵循 HTTP 缓存规则的 `HttpClient`。此后所有远程图片都会受益，包括不由应用自己编写的代码加载的图片。实现时遵循以下规则：

- **只缓存不带请求体的 GET。** 其他请求原样转发。
- **按共享缓存处理。** 整个应用共用一个客户端：应用自己的视图、扩展和文档视图都经过它。`no-store` 或 `private` 响应，以及对带 `Authorization` 请求的响应，除非服务器明确允许共享缓存保存，否则都不能存储。如果缓存下层会附加 Cookie 或令牌，缓存看不到这些凭据，因此应在缓存之上添加凭据，或者不缓存这些主机。
- **用重新验证代替重新下载。** 新鲜的响应直接返回，不访问网络。过期后发送 `If-None-Match` 或 `If-Modified-Since`；收到 `304 Not Modified` 时，用更新后的响应头返回已存储的响应体。
- **遵守调用方的重定向策略。** `img()` 会跟随重定向，但自行授权每一跳的加载器会请求 `RedirectPolicy::NoFollow`，并且必须拿到 `3xx` 响应。每种策略使用各自的缓存，跟随重定向得到的结果就永远不会回应这类调用方。
- **限制内存和磁盘用量。** 限制缓存大小，并清理旧条目。
- **先授权，再请求。** 同一个 URL，缓存会回应任何请求它的调用方。判断调用方能否访问某个 URL 的检查，例如 gpui-shell 脚本的网络授权，必须在 `send` 之前完成。这样缓存不会扩大调用方的访问范围，只是省去重复一次已经获准的请求。

### 示例

[http-cache-reqwest](https://crates.io/crates/http-cache-reqwest) 以 [reqwest-middleware](https://crates.io/crates/reqwest-middleware) 中间件的形式实现了这些规则：新鲜度判断、基于 `ETag` 和 `Last-Modified` 的重新验证、共享缓存的存储限制，并自带磁盘存储。应用只需把它适配到 GPUI 的 `HttpClient`。添加以下依赖：

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

GPUI 自带的 reqwest 客户端会按每个请求的 `RedirectPolicy` 处理跳转，而 `reqwest-middleware` 客户端的跳转策略在构建时就固定了。所以示例构建了两个客户端：一个跟随跳转，供 `img()` 使用；另一个从不跟随，供请求 `RedirectPolicy::NoFollow` 的调用方使用。两者的缓存目录也要分开，因为跟随跳转的客户端前面的缓存，会把最终响应存在原始 URL 下，再拿它回应另一类调用方。`RedirectPolicy::FollowLimit` 使用 reqwest 默认的 10 次上限。

### 安装客户端

在启动时安装一次，要在任何窗口加载远程图片之前。缓存目录请使用平台的缓存目录，例如用 [dirs](https://crates.io/crates/dirs) crate 获取，不要像这里一样放在临时目录：

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

### 扩展示例

- **大小。** 磁盘存储没有大小上限。可以在启动时删除旧条目，或在目录超过上限时清空它。
- **代理与 User-Agent。** 在 `reqwest::Client::builder()` 上配置，其他代码需要读取时，再从 `proxy()` 和 `user_agent()` 返回。
- **请求体。** 示例在发送前把请求体读入内存，这对图片和小请求没有问题；上传大文件时，改用 `reqwest::Body::wrap_stream` 包装数据流。

## 常见问题

| 现象 | 检查 |
| --- | --- |
| 内嵌图片显示为空白 | 已注册的 `AssetSource` 必须包含完全相同的键。`img("images/a.png")` 是资源查找，不是读取文件。 |
| 原生平台上所有 URL 图片都显示失败内容 | 安装一个 `HttpClient`；默认客户端会让所有请求失败。 |
| `with_loading` 从不出现，或 GIF 不播放 | 给 `img()` 加上 `.id(...)`。 |
| `svg()` 看不见 | 在 `svg()` 元素本身上设置 `.text_color(...)` 和尺寸。 |
| 多色 SVG 变成了单色 | 用 `img()` 绘制；`svg()` 总是单色。 |
| 图片加载完成时布局跳动 | 用尺寸，或宽度加 `aspect_ratio(...)` 预先占位。 |
| 修好的图片仍显示之前的失败内容 | 失败结果被缓存了；重新绘制前调用 `ImageSource::remove_asset(cx)`。 |
| 每个新视图或每次重启都重新下载图片 | 在 HTTP 层缓存，见[远程图片的 HTTP 缓存](#远程图片的-http-缓存)。 |
