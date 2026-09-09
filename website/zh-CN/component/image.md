---
title: Image
description: 支持加载状态、回退内容和响应式尺寸的灵活图片展示组件。
---

# Image

Image 组件为图片展示提供了更稳健的封装，支持加载态、回退内容、响应式尺寸以及多种图片来源。它基于 GPUI 原生图片能力构建，可处理 URL、本地文件和 SVG 等资源，并便于结合主题与布局系统做统一样式控制。

## 导入

```rust
use gpui_kit::{img, ImageSource, ObjectFit};
use gpui_kit::component::{v_flex, h_flex, div, Icon, IconName};
```

## 用法

### 基础图片

```rust
// 来自 URL 的图片
img("https://example.com/image.jpg")

// 本地图片文件
img("assets/logo.png")

// SVG 图片
img("icons/star.svg")
```

### 设置尺寸

```rust
// 固定尺寸
img("https://example.com/photo.jpg")
    .w(px(300.))
    .h(px(200.))

// 响应式宽度并限制最大宽度
img("https://example.com/banner.jpg")
    .w(relative(1.))
    .max_w(px(800.))
    .h(px(400.))

// 正方形图片
img("https://example.com/avatar.jpg")
    .size(px(100.))
```

### Object Fit 选项

用于控制图片在容器中的缩放与定位方式：

```rust
// Cover：填满容器，可能裁剪
img("https://example.com/photo.jpg")
    .w(px(300.))
    .h(px(200.))
    .object_fit(ObjectFit::Cover)

// Contain：完整显示，保持比例
img("https://example.com/photo.jpg")
    .w(px(300.))
    .h(px(200.))
    .object_fit(ObjectFit::Contain)

// Fill：拉伸填满，可能变形
img("https://example.com/photo.jpg")
    .w(px(300.))
    .h(px(200.))
    .object_fit(ObjectFit::Fill)

// ScaleDown：类似 contain，但不会放大
img("https://example.com/photo.jpg")
    .w(px(300.))
    .h(px(200.))
    .object_fit(ObjectFit::ScaleDown)

// None：保持原始尺寸
img("https://example.com/photo.jpg")
    .w(px(300.))
    .h(px(200.))
    .object_fit(ObjectFit::None)
```

### 回退内容

```rust
fn image_with_fallback(src: &str, alt_text: &str) -> impl IntoElement {
    div()
        .w(px(300.))
        .h(px(200.))
        .bg(cx.theme().surface)
        .border_1()
        .border_color(cx.theme().border)
        .rounded(px(8.))
        .overflow_hidden()
        .child(
            img(src)
                .w_full()
                .h_full()
                .object_fit(ObjectFit::Cover)
                // 实际项目中可在这里补充错误处理
        )
}

fn image_with_icon_fallback(src: &str) -> impl IntoElement {
    div()
        .size(px(200.))
        .bg(cx.theme().surface)
        .border_1()
        .border_color(cx.theme().border)
        .rounded(px(8.))
        .flex()
        .items_center()
        .justify_center()
        .child(
            img(src)
                .size_full()
                .object_fit(ObjectFit::Cover)
                // 加载失败时可改为显示图标占位
        )
}
```

### 加载状态

```rust
fn image_with_loading(src: &str, is_loading: bool) -> impl IntoElement {
    div()
        .w(px(400.))
        .h(px(300.))
        .rounded(px(8.))
        .overflow_hidden()
        .map(|this| {
            if is_loading {
                this.bg(cx.theme().muted)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child("Loading...")
            } else {
                this.child(
                    img(src)
                        .w_full()
                        .h_full()
                        .object_fit(ObjectFit::Cover)
                )
            }
        })
}

fn progressive_image(src: &str, placeholder_src: &str) -> impl IntoElement {
    div()
        .relative()
        .w(px(400.))
        .h(px(300.))
        .rounded(px(8.))
        .overflow_hidden()
        .child(
            img(placeholder_src)
                .absolute()
                .inset_0()
                .w_full()
                .h_full()
                .object_fit(ObjectFit::Cover)
                .opacity(0.5)
        )
        .child(
            img(src)
                .absolute()
                .inset_0()
                .w_full()
                .h_full()
                .object_fit(ObjectFit::Cover)
        )
}
```

### 响应式图片

```rust
fn responsive_image_grid() -> impl IntoElement {
    div()
        .grid()
        .grid_cols(3)
        .gap_4()
        .child(
            img("https://example.com/photo1.jpg")
                .w_full()
                .aspect_ratio(1.0)
                .object_fit(ObjectFit::Cover)
                .rounded(px(8.))
        )
        .child(
            img("https://example.com/photo2.jpg")
                .w_full()
                .aspect_ratio(1.0)
                .object_fit(ObjectFit::Cover)
                .rounded(px(8.))
        )
        .child(
            img("https://example.com/photo3.jpg")
                .w_full()
                .aspect_ratio(1.0)
                .object_fit(ObjectFit::Cover)
                .rounded(px(8.))
        )
}

fn hero_image() -> impl IntoElement {
    div()
        .relative()
        .w_full()
        .h(px(500.))
        .rounded(px(12.))
        .overflow_hidden()
        .child(
            img("https://example.com/hero-image.jpg")
                .absolute()
                .inset_0()
                .w_full()
                .h_full()
                .object_fit(ObjectFit::Cover)
        )
        .child(
            div()
                .absolute()
                .inset_0()
                .bg(rgba(0, 0, 0, 0.4))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    v_flex()
                        .items_center()
                        .gap_4()
                        .child("Hero Title")
                        .child("Subtitle text here")
                )
        )
}
```

### 图片画廊

```rust
fn image_gallery(images: Vec<&str>) -> impl IntoElement {
    v_flex()
        .gap_6()
        .child(
            div()
                .w_full()
                .h(px(400.))
                .rounded(px(12.))
                .overflow_hidden()
                .child(
                    img(images[0])
                        .w_full()
                        .h_full()
                        .object_fit(ObjectFit::Cover)
                )
        )
        .child(
            h_flex()
                .gap_3()
                .children(
                    images.iter().map(|src| {
                        div()
                            .size(px(80.))
                            .rounded(px(6.))
                            .overflow_hidden()
                            .border_2()
                            .border_color(cx.theme().border)
                            .cursor_pointer()
                            .hover(|this| this.border_color(cx.theme().primary))
                            .child(
                                img(*src)
                                    .size_full()
                                    .object_fit(ObjectFit::Cover)
                            )
                    })
                )
        )
}
```

### SVG 图片

```rust
img("assets/icons/logo.svg")
    .size(px(64.))
    .text_color(cx.theme().primary)

img("data:image/svg+xml;base64,...")
    .w(px(32.))
    .h(px(32.))

img("assets/spinner.svg")
    .size(px(24.))
    .text_color(cx.theme().primary)
    // 实际使用中可叠加旋转动画
```

## API 参考

### 核心函数

| 函数 | 说明 |
| --- | --- |
| `img(source)` | 基于 `ImageSource` 创建图片元素 |

### 图片来源（ImageSource）

| 类型 | 说明 | 示例 |
| --- | --- | --- |
| String / &str | URL 或文件路径 | `"https://example.com/image.jpg"` |
| SharedUri | 共享 URI 引用 | `SharedUri::from("file://path")` |
| Local Path | 本地文件系统路径 | `"assets/logo.png"` |
| Data URI | Base64 编码图片 | `"data:image/png;base64,..."` |

### 尺寸方法

| 方法 | 说明 |
| --- | --- |
| `w(length)` | 设置宽度 |
| `h(length)` | 设置高度 |
| `size(length)` | 同时设置宽高 |
| `w_full()` | 占满容器宽度 |
| `h_full()` | 占满容器高度 |
| `size_full()` | 占满容器尺寸 |
| `max_w(length)` | 设置最大宽度 |
| `max_h(length)` | 设置最大高度 |
| `min_w(length)` | 设置最小宽度 |
| `min_h(length)` | 设置最小高度 |

### Object Fit 选项

| 值 | 说明 |
| --- | --- |
| `ObjectFit::Cover` | 填满容器，可能裁剪 |
| `ObjectFit::Contain` | 完整显示在容器内 |
| `ObjectFit::Fill` | 拉伸填满容器 |
| `ObjectFit::ScaleDown` | 类似 contain，但不会放大 |
| `ObjectFit::None` | 保持原始尺寸 |

### 样式方法

| 方法 | 说明 |
| --- | --- |
| `rounded(radius)` | 设置圆角 |
| `border_1()` | 1px 边框 |
| `border_color(color)` | 设置边框颜色 |
| `opacity(value)` | 设置透明度（0.0-1.0） |
| `shadow_sm()` | 小阴影 |
| `shadow_lg()` | 大阴影 |

## 最佳实践

### 图片优化

- 根据展示尺寸提供合适分辨率的图片
- 在保证质量的前提下尽量压缩资源
- 优先考虑 WebP、AVIF 等现代格式
- 为不同屏幕尺寸准备响应式图片

### 错误处理

- 为加载失败提供明确回退内容
- 使用骨架屏保持布局稳定
- 对临时网络错误考虑重试机制
- 对永久失败给出可理解的用户提示

### 性能

- 对首屏外图片使用懒加载
- 配合缓存策略减少重复请求
- 加载期间可使用低质量占位图
- 图片尺寸应与实际展示上下文匹配

### 用户体验

- 图片网格中保持一致的宽高比
- 使用平滑的加载过渡
- 根据内容类型选择合适的 `object-fit`
- 细节图可考虑提供缩放能力
