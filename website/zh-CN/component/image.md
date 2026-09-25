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
- GPUI 默认的图片缓存已经处理普通的重复来源。只有测得明确的生命周期或内存需求时才引入自定义缓存。
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
