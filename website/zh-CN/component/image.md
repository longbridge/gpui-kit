---
title: Image
description: 展示嵌入资源、本地文件与远程图片，并处理尺寸、加载和失败状态。
---

# Image

GPUI 的 `img()` 绘制图片，`svg()` 绘制单色图标。GPUI Kit 从 `gpui_kit` 重新导出这两个函数。本页列出应用中最常用的写法；来源、加载、尺寸、`svg()`、缓存和 HTTP 缓存的细节见[图片](../docs/image.md)。

## 从可运行示例开始

以下是完整的原生桌面 `src/main.rs`。它使用 GPUI Kit 自带的图标，不需要额外图片文件。在 `Cargo.toml` 加入 `gpui-kit = "0.6"`。同一份资源分别以保留原色的图片和单色 SVG 显示。

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

`with_assets(Assets)` 注册默认组件图标；要嵌入自己的图片，请注册自己的 `AssetSource`（见[图标与资源](../docs/assets.md)）。`"images/cover.png"` 这样的字符串是资源键，`Path` 读取文件，URL 则通过 App 的 `HttpClient` 下载；见[图片来源](../docs/image.md#图片来源)。

## 常用写法

填满固定区域并裁掉边缘的缩略图：

```rust
img("images/cover.png")
    .w(px(320.))
    .h(px(180.))
    .object_fit(ObjectFit::Cover)
```

跟随父元素宽度、在图片加载前就占好高度的横幅：

```rust
img("images/banner.webp")
    .w_full()
    .aspect_ratio(16. / 9.)
    .object_fit(ObjectFit::Cover)
```

带加载中和加载失败状态的远程图片。必须设置 `.id(...)`，加载中的内容才会出现：

```rust
img("https://example.com/avatar.png")
    .id("avatar")
    .size(px(48.))
    .rounded_full()
    .with_loading(|| div().child("Loading image...").into_any_element())
    .with_fallback(|| div().child("Image unavailable").into_any_element())
```

必须完整显示的 Logo 和图表，使用 `ObjectFit::Contain`（默认值）。要保留多色 SVG 的颜色，用 `img()` 绘制；跟随主题变色的图标，使用 [Icon](./icon.md)。见[尺寸与适配](../docs/image.md#尺寸与适配)和[选择 img() 还是 svg()](../docs/image.md#选择-img-还是-svg)。

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

## 可访问性

- 传达信息的图片，要在周围界面配上可见文字或无障碍描述。文件名不是描述。装饰性图片不需要朗读。
- 点击后会执行命令的图片，要做成真正的控件，例如 `Button`，这样它才有名称、焦点和键盘操作。
- 加载中和加载失败的内容要占用与图片相同的区域，避免推动周围内容。

## 延伸阅读

- [图片](../docs/image.md)：`img()` 的所有来源、加载与解码、`svg()` 和常见问题。
- [解码后图片的缓存](../docs/image.md#解码后图片的缓存)：限定范围的缓存与自定义 `ImageCache`。
- [远程图片的 HTTP 缓存](../docs/image.md#远程图片的-http-缓存)：跨视图、跨启动复用下载结果。
