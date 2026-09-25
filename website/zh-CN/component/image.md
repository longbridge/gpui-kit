---
title: Image
description: 展示嵌入资源、本地文件与远程图片，并处理尺寸、加载和失败状态。
---

# Image

GPUI 的 `img()` 会创建图片元素。GPUI Kit 从 `gpui_kit` 重新导出这一 API，使用整合 crate 的应用无需另加 GPUI 依赖。图片可以来自嵌入式资源键名、文件系统 `Path`、HTTP URL 或内存中的图片数据。参数类型决定 GPUI 从哪里取字节；布局样式决定图片如何显示。

## 从可运行示例开始

以下是完整的原生桌面 `src/main.rs`。它使用 GPUI Kit 自带的图标，不需要额外图片文件。在 `Cargo.toml` 加入 `gpui-kit = "0.6"`。同一份 SVG 分别以保留原色的图片和随主题着色的单色图标显示；两者区别见下文。

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

`with_assets(Assets)` 在打开窗口前注册默认组件图标。如果应用还需要嵌入自己的图片，可以改为注册组合后的 `AssetSource`；见[图标与资源](../docs/assets.md)。只有加载在短暂延迟后仍未完成时，`with_loading` 的内容才会显示；加载失败时显示 `with_fallback` 的内容。稳定的 `.id(...)` 让 GPUI 能跨帧保存图片的加载状态。

## 选择图片来源

| `img(...)` 参数 | GPUI 的加载方式 | 打包时需要考虑 |
| --- | --- | --- |
| `"images/cover.png"` | 把键名原样交给已注册的 `AssetSource` | 在资源源中嵌入或提供该键名。 |
| `std::path::Path::new("/absolute/cover.png")` | 读取文件系统路径 | 随应用安装文件，或由用户选择文件。 |
| `"https://example.com/cover.png"` | 通过配置的 HTTP 客户端请求 URL | 处理网络连接、加载中与 HTTP 错误。 |
| `Arc<Image>` | 解码调用方提供的编码字节和格式 | 提供字节与格式相符的 `Image`。 |
| `Arc<RenderImage>` | 使用已经可供绘制的图片数据 | 调用方创建或持有可绘制数据。 |

不是 URL 的**字符串**会作为资源键名处理，即使看起来像相对文件名，也不会按进程当前工作目录解析。要嵌入应用的 `images/cover.png`，将文件包含在应用自己的 `AssetSource` 中，再通过 `with_assets(...)` 注册。例如原生 `rust-embed` 的根目录是 `./assets` 时，文件 `assets/images/cover.png` 对应的键名是 `images/cover.png`。资源源的实现和默认组件图标的回退方式见[应用资源示例](../docs/assets.md)。

## 尺寸与填充方式

为图片设定合适的布局尺寸。`object_fit` 决定内容如何放进这块区域；默认值是 `Contain`。

```rust
img("images/cover.png")
    .w(px(320.))
    .h(px(180.))
    .object_fit(ObjectFit::Cover)
```

| 模式 | 效果 |
| --- | --- |
| `Contain` | 保持比例，完整显示图片；可能留下空白。 |
| `Cover` | 保持比例并填满区域；边缘可能被裁掉。 |
| `Fill` | 拉伸到指定宽高；可能变形。 |
| `ScaleDown` | 类似 `Contain`，但不放大原图。 |
| `None` | 保持图片原始尺寸。 |

可能裁剪的缩略图通常用 `Cover`；需要完整显示的 Logo 和图示通常用 `Contain`。如果加载期间周围布局必须稳定，请同时指定宽和高。

### 响应式宽度与稳定比例

让父容器决定可用宽度，限制宽窗口中的图片最大尺寸，并在图片字节到达之前预留高度：

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

`w_full()` 跟随父容器的宽度；`max_w(...)` 限制整个图片区域在宽窗口中的尺寸。显式设置 `aspect_ratio(...)`，可以在解码前预留稳定的空间。正方形卡片可使用 `.aspect_ratio(1.)`。如果没有显式比例或高度，GPUI 可以在解码后使用图片的固有比例，但布局可能在加载完成时改变。如果 flex 子元素在窄窗口中无法收缩，可为其所在容器设置 `.min_w_0()`。

### 小型图片网格

下面是视图 `render` 方法中的局部示例，假设已经注册包含三个键名的 `AssetSource`。示例固定为三列；当窗口变窄时，应用应在自己的布局断点更改列数或布局方式。

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

每个图片格在加载中都会保留相同的正方形区域。`Cover` 可能裁掉重要内容，因此图表或需要显示完整边缘的产品图片应使用 `Contain`。对于长而持续变化的集合，使用滚动或虚拟列表，而不是一次构建所有图片格。

## 可选择图片的画廊

能够切换主图的缩略图是一种操作控件。使用真正的 `Button`，让键盘激活和可访问名称都有效；将选中项保存在视图的持久状态中。下面完整的 `src/main.rs` 使用 GPUI Kit 自带的三个图标；图片画廊可以换成自己注册的资源键名。依赖与前面的示例相同，仍是 `gpui-kit = "0.6"`。

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

被选中的按钮有明确的选中状态，可见的 `Selected: ...` 文本也报告当前选择。在实际画廊中，使用 `Show front view` 等有意义的名称，不要只用文件名。如果图片集合可能为空，先显示空状态，再读取选中项。如果图片来源会被删除或重新排序，应保存稳定的业务 ID，而不是数组索引，并在渲染时解析它。

## 叠字主图

把文字放在图片上方的独立表面，让可读性不依赖照片本身的像素。下面的局部示例假设 `cx` 是视图上下文，并已导入 `ActiveTheme`。

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

这里的图片只作装饰，文字负责传达信息。任何操作控件都应避开被裁切或遮挡的区域，让键盘焦点始终可见。

## 加载中与加载失败

`img()` 通过 `StyledImage` 提供真正的加载态与错误回退 API。回调返回用于替代图片显示的元素。下例使用嵌入资源键名；URL 和文件系统路径也适用。

```rust
img("images/cover.png")
    .id("cover-image")
    .w(px(320.))
    .h(px(180.))
    .object_fit(ObjectFit::Cover)
    .with_loading(|| div().child("Loading image...").into_any_element())
    .with_fallback(|| div().child("Image unavailable").into_any_element())
```

加载很快时，加载提示可能来不及出现。资源键名不存在、本地文件缺失、图片字节无法解码或 HTTP 请求失败，都可能触发回退内容。为外层布局保留固定尺寸，避免替代内容出现时推动周边界面。GPUI 会缓存加载的图片资源；只有需要特定缓存生命周期时才需要自定义图片缓存。

`with_loading` 和 `with_fallback` 只负责提供替代元素；它们不会暴露独立的 `ImageState` 枚举，也不会自动添加重试命令。如果远程图片是任务必需内容，应在应用状态中提供相邻的重试操作，并在用户重试时使用更新后的来源重新构建图片。来源已经失败时，不要继续显示假装正在加载的骨架屏。

## 远程图片的 HTTP 缓存

`img("https://...")` 这类 URL 来源，以及 `TextView` 文档中的远程图片，都由 GPUI 通过安装在 `App` 上的 `HttpClient` 加载。在原生平台上，这个客户端由应用决定：默认客户端会让所有请求失败，因此要先用 `cx.set_http_client(...)` 或 `Application::with_http_client(...)` 安装一个客户端，远程图片才能加载。在 Web 上，`gpui_kit::application()` 会安装基于浏览器 Fetch API 的客户端，浏览器自身的 HTTP 缓存已经生效；本节只讨论原生应用。

GPUI 的图片缓存位于这个客户端之上。它在内存中按来源保存解码后的图片，足以应付运行中视图里的重复来源，却无法减少网络请求：

- 进程退出后缓存随之消失，每次启动都要重新下载所有图片。
- 它不理会 `Cache-Control`、`ETag` 和 `Last-Modified`，既不能保留服务器允许复用的响应，也不能向服务器确认旧副本是否仍然有效。
- 它的寿命取决于持有它的缓存。使用独立 `.image_cache(...)` 的图片，或按视图分别缓存的加载器，每创建一个新视图都会重新请求图片。

要跨视图、跨启动复用响应，可以用一个遵循 HTTP 缓存规则的客户端包装应用原有的 `HttpClient`，并在启动时安装一次。此后所有远程图片都会受益，包括不由应用自己编写的代码加载的图片。实现时遵循以下规则：

- **只缓存不带请求体的 GET。** 其他请求原样转发。
- **按共享缓存处理。** 整个应用共用一个客户端：应用自己的视图、扩展和文档视图都经过它。`no-store` 或 `private` 响应，以及对带 `Authorization` 请求的响应，除非服务器明确允许共享缓存保存，否则都不能存储。如果缓存下层会附加 Cookie 或令牌，缓存看不到这些凭据，因此应在缓存之上添加凭据，或者不缓存这些主机。
- **用重新验证代替重新下载。** 新鲜的响应直接返回，不访问网络。过期后发送 `If-None-Match` 或 `If-Modified-Since`；收到 `304 Not Modified` 时，用更新后的响应头返回已存储的响应体。
- **遵守调用方的重定向策略。** `img()` 会跟随重定向，但自行授权每一跳的加载器会请求 `RedirectPolicy::NoFollow`，并且必须拿到 `3xx` 响应。把策略放进缓存键，跟随重定向得到的结果就永远不会回应这类调用方；重定向响应本身也按同样的规则缓存。
- **限制内存和磁盘用量。** 同时限制单个条目和总大小，淘汰旧条目；过大而不宜保存的响应直接以流的形式转发。
- **先授权，再请求。** 同一个 URL，缓存会回应任何请求它的调用方。判断调用方能否访问某个 URL 的检查，例如 gpui-shell 脚本的网络授权，必须在 `send` 之前完成。这样缓存不会扩大调用方的访问范围，只是省去重复一次已经获准的请求。

下面的示例在内存中实现这些规则。[http-cache-semantics](https://crates.io/crates/http-cache-semantics) crate 负责 HTTP 缓存规则的判断：新鲜度、验证器、`Vary` 以及共享缓存的限制。在 `Cargo.toml` 中加入 `http-cache-semantics = "2"`、`futures`、`bytes` 和 `anyhow`。

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

在应用原有的客户端外层安装这个包装。这里的 `reqwest_client` 是 `gpui-pre-reqwest-client` crate，版本与所用 `gpui-kit` 固定的 GPUI 快照一致：

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

这个示例刻意保持简短，应用有需要时可以在以下方面扩展：

- **持久化。** 应用退出后条目随之消失。要跨启动保留图片，可以把每个响应体连同其 `CachePolicy` 存到磁盘（`CachePolicy` 实现了 `serde` 的 trait），以原子方式写入文件，并在启动时清理超出大小或时间限制的条目。基于 `reqwest` 的客户端也可以改用带磁盘存储的缓存中间件，例如 [http-cache-reqwest](https://crates.io/crates/http-cache-reqwest)。
- **变体。** 每个 URL 只保留一个响应。因 `Vary` 而不同的响应会替换之前的变体。
- **重复请求。** 同一 URL 的并发未命中会各自访问网络。GPUI 的图片加载器本身已经让同一来源只加载一次。
- **淘汰策略。** 总是先移除最早存入的条目，不管它最近是否被使用。如果访问模式很重要，可以改用 LRU 结构。

## SVG 图片与单色图标

两种形式都能从同一 `AssetSource` 读取键名，但渲染方式不同：

| API | 渲染方式 | 适用场景 |
| --- | --- | --- |
| `img("images/brand.svg")` | 将 SVG 光栅化为图片，保留文件原有颜色；支持 `object_fit`。 | 多色 Logo、插画和图示。 |
| `svg().path("icons/check.svg")` | 使用 SVG 的透明度蒙版，按元素文字颜色绘制成单色。 | 跟随主题或状态变化的单色图标。 |

```rust
img("images/brand.svg")
    .size(px(96.))
    .object_fit(ObjectFit::Contain);

svg().path("icons/check.svg")
    .size(px(20.))
    .text_color(rgb(0x2563eb));
```

给 `img("images/brand.svg")` 加 `.text_color(...)` **不会**改变图片颜色。单色图标应使用 `svg().path(...)` 或 GPUI Kit 的 [`Icon`](./icon.md)。少量自定义 SVG 也可以用 `svg().data(include_bytes!("check.svg"))` 直接提供字节，省去资源键名查找。全彩 SVG 仍应通过 `img()` 显示。

`img()` 支持 PNG、JPEG、WebP、GIF 等常见位图格式，也支持 SVG。具体格式能力来自当前固定版本的 GPUI 图片解码器；发布前应在目标平台用实际文件检查效果。

## API 速查

| API | 用途 | 注意 |
| --- | --- | --- |
| `img(source)` | 根据 `ImageSource` 创建图片 | 非 URL 字符串是嵌入资源键名。 |
| `.id(id)` | 提供稳定的元素标识 | 有助于跨帧保存图片加载状态。 |
| `.w(...)`、`.h(...)`、`.size(...)` | 指定明确的尺寸 | 在远程图片加载前预留空间。 |
| `.w_full()`、`.h_full()`、`.size_full()` | 填满父容器相应方向 | 父容器仍需有明确的可用尺寸。 |
| `.max_w(...)`、`.aspect_ratio(...)` | 限制宽度并预留比例 | 适用于响应式布局。 |
| `.object_fit(ObjectFit::...)` | 选择裁切与缩放方式 | 默认是 `Contain`。 |
| `.with_loading(...)`、`.with_fallback(...)` | 提供替代元素 | 每个回调都返回 `AnyElement`。 |
| `.grayscale(true)` | 用灰度显示图片 | 视觉效果不会改变图片来源。 |
| `.image_cache(&cache)` | 覆盖此图片使用的缓存 | 仅在确实需要特定缓存生命周期时使用。 |

`rounded(...)`、边框、`overflow_hidden()` 和阴影属于普通元素或容器的样式，不是图片解码选项。需要圆角裁切时，将图片放进带圆角的父容器并裁切溢出内容。

## 性能与可访问性

- 图片尺寸应接近实际显示尺寸。很小的缩略图不需要全屏照片那么多编码像素。
- 压缩源文件，并在目标显示设备上检查效果。照片和线条插画应选择合适的格式；不要假设所有平台或解码器支持所有格式。
- 为稍后才到达的图片保留固定区域或稳定比例。加载态和失败态应占据同一块区域。
- GPUI 默认的图片缓存已经处理普通的重复来源。只有测得明确的生命周期或内存需求时才引入自定义缓存。要避免新视图或重启后重新下载远程图片，应在 HTTP 层缓存，见[远程图片的 HTTP 缓存](#远程图片的-http-缓存)。
- 对于大型滚动画廊，用集合 API 只构建可见或邻近的条目。单独调用 `img()` 并不等于实施了懒加载策略。
- 对传达信息的图片提供可见说明或周围界面中的可访问描述。装饰性图片不需重复朗读。由图片驱动的操作应使用带名称、焦点和键盘激活能力的真实控件。

## 排查问题

| 现象 | 检查方向 |
| --- | --- |
| 嵌入图片空白 | 确认已注册的 `AssetSource` 包含完全相同的键名和文件字节。`img("images/cover.png")` 是资源查找，不会读取当前目录的文件。 |
| 默认图标缺失 | 注册 GPUI Kit 默认的 `Assets`，或把它作为应用资源源的回退来源。 |
| 多色 SVG 变成单色 | 使用 `img()` 显示；`svg().path(...)` 会有意使用透明度蒙版。 |
| 图片被裁切 | 改用 `ObjectFit::Contain`，或增加显示区域的尺寸。 |
| 出现错误回退内容 | 按所选来源检查文件路径、网络响应或图片字节能否解码。 |

如果图片承载信息，应在周边界面提供文字说明或其他可访问描述；文件名不能代替面向用户的描述。
