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

## 排查问题

| 现象 | 检查方向 |
| --- | --- |
| 嵌入图片空白 | 确认已注册的 `AssetSource` 包含完全相同的键名和文件字节。`img("images/cover.png")` 是资源查找，不会读取当前目录的文件。 |
| 默认图标缺失 | 注册 GPUI Kit 默认的 `Assets`，或把它作为应用资源源的回退来源。 |
| 多色 SVG 变成单色 | 使用 `img()` 显示；`svg().path(...)` 会有意使用透明度蒙版。 |
| 图片被裁切 | 改用 `ObjectFit::Contain`，或增加显示区域的尺寸。 |
| 出现错误回退内容 | 按所选来源检查文件路径、网络响应或图片字节能否解码。 |

如果图片承载信息，应在周边界面提供文字说明或其他可访问描述；文件名不能代替面向用户的描述。
