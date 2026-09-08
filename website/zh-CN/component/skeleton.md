---
title: Skeleton
description: 在内容加载时显示占位骨架。
---

# Skeleton

Skeleton 会在真实内容尚未加载完成时显示带动画的占位块，为用户提供加载反馈，并尽量保持界面布局稳定。

## 导入

```rust
use gpui_kit::component::skeleton::Skeleton;
```

## 用法

### 基础 Skeleton

```rust
Skeleton::new()
```

### 文本行占位

```rust
Skeleton::new()
    .w(px(250.))
    .h_4()
    .rounded_md()

v_flex()
    .gap_2()
    .child(Skeleton::new().w(px(250.)).h_4().rounded_md())
    .child(Skeleton::new().w(px(200.)).h_4().rounded_md())
    .child(Skeleton::new().w(px(180.)).h_4().rounded_md())
```

### 圆形占位

```rust
Skeleton::new()
    .size_12()
    .rounded_full()

Skeleton::new()
    .w(px(64.))
    .h(px(64.))
    .rounded_full()
```

### 矩形占位

```rust
Skeleton::new()
    .w(px(250.))
    .h(px(125.))
    .rounded_md()

Skeleton::new()
    .w(px(120.))
    .h(px(40.))
    .rounded_md()
```

### 不同形状

```rust
Skeleton::new().w(px(200.)).h_4().rounded_sm()
Skeleton::new().size_20().rounded_md()
Skeleton::new().w_full().h(px(200.)).rounded_lg()
Skeleton::new().size_6().rounded_md()
```

### Secondary 变体

```rust
Skeleton::new()
    .secondary()
    .w(px(200.))
    .h_4()
    .rounded_md()
```

## 动画

Skeleton 内置脉冲动画，行为如下：

- 持续循环播放，周期为 2 秒
- 使用 bounce easing，并带有 ease-in-out 变化
- 透明度会在 100% 和 50% 之间往返变化
- 自动重复，以持续表达“内容正在加载”

该动画不可关闭，因为它本身就是加载状态的重要视觉提示。

## 尺寸

Skeleton 没有预设的尺寸枚举，通常配合 gpui 的尺寸工具使用：

```rust
Skeleton::new().h_3()
Skeleton::new().h_4()
Skeleton::new().h_5()
Skeleton::new().h_6()

Skeleton::new().w(px(100.))
Skeleton::new().w(px(200.))
Skeleton::new().w_full()
Skeleton::new().w_1_2()

Skeleton::new().size_4()
Skeleton::new().size_8()
Skeleton::new().size_12()
Skeleton::new().size_16()
```

## 示例

### 资料卡片加载中

```rust
v_flex()
    .gap_4()
    .p_4()
    .border_1()
    .border_color(cx.theme().border)
    .rounded(cx.theme().radius_lg)
    .child(
        h_flex()
            .gap_3()
            .items_center()
            .child(Skeleton::new().size_12().rounded_full())
            .child(
                v_flex()
                    .gap_2()
                    .child(Skeleton::new().w(px(120.)).h_4().rounded_md())
                    .child(Skeleton::new().w(px(100.)).h_3().rounded_md())
            )
    )
    .child(
        v_flex()
            .gap_2()
            .child(Skeleton::new().w_full().h_4().rounded_md())
            .child(Skeleton::new().w(px(200.)).h_4().rounded_md())
    )
```

### 文章列表加载中

```rust
v_flex()
    .gap_6()
    .children((0..3).map(|_| {
        h_flex()
            .gap_4()
            .child(Skeleton::new().w(px(120.)).h(px(80.)).rounded_md())
            .child(
                v_flex()
                    .gap_2()
                    .flex_1()
                    .child(Skeleton::new().w_full().h_5().rounded_md())
                    .child(Skeleton::new().w(px(300.)).h_4().rounded_md())
                    .child(Skeleton::new().w(px(250.)).h_4().rounded_md())
                    .child(Skeleton::new().w(px(100.)).h_3().rounded_md())
            )
    }))
```

### 表格行加载中

```rust
v_flex()
    .gap_2()
    .children((0..5).map(|_| {
        h_flex()
            .gap_4()
            .p_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(Skeleton::new().size_8().rounded_full())
            .child(Skeleton::new().w(px(150.)).h_4().rounded_md())
            .child(Skeleton::new().w(px(200.)).h_4().rounded_md())
            .child(Skeleton::new().w(px(80.)).h_4().rounded_md())
            .child(Skeleton::new().w(px(60.)).h_4().rounded_md())
    }))
```

### 按钮加载态

```rust
h_flex()
    .gap_3()
    .child(Skeleton::new().w(px(80.)).h(px(36.)).rounded_md())
    .child(Skeleton::new().w(px(70.)).h(px(36.)).rounded_md())
    .child(Skeleton::new().size_9().rounded_md())
```

### 表单字段加载态

```rust
v_flex()
    .gap_4()
    .child(
        v_flex()
            .gap_1()
            .child(Skeleton::new().w(px(60.)).h_4().rounded_md())
            .child(Skeleton::new().w_full().h(px(40.)).rounded_md())
    )
    .child(
        v_flex()
            .gap_1()
            .child(Skeleton::new().w(px(80.)).h_4().rounded_md())
            .child(Skeleton::new().w_full().h(px(120.)).rounded_md())
    )
```

### 条件加载

```rust
if loading {
    Skeleton::new().w(px(200.)).h_4().rounded_md()
} else {
    div().child("Actual content here")
}
```

## 主题

Skeleton 默认使用主题中的 `skeleton` 颜色；如果未配置，则回退到 `secondary`。你可以在主题中这样覆盖：

```json
{
  "skeleton.background": "#e2e8f0"
}
```

`secondary(true)` 变体会对骨架颜色应用 50% 透明度，让占位效果更柔和。
