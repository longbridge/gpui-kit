---
title: Spinner
description: 显示旋转加载动画，用于反馈任务或异步操作的进行中状态。
---

# Spinner

Spinner 用于显示旋转中的加载动画，适合异步请求、处理中状态和其他需要即时反馈的场景。它支持自定义图标、颜色、尺寸以及内置旋转动画。

## 导入

```rust
use gpui_kit::component::spinner::Spinner;
```

## 用法

### 基础用法

```rust
Spinner::new()
```

### 自定义颜色

```rust
use gpui_kit::component::ActiveTheme;

Spinner::new()
    .color(cx.theme().blue)

Spinner::new()
    .color(cx.theme().green)

Spinner::new()
    .color(cx.theme().cyan)
```

### 不同尺寸

```rust
Spinner::new().xsmall()
Spinner::new().small()
Spinner::new()
Spinner::new().large()
Spinner::new().with_size(px(64.))
```

### 自定义图标

```rust
use gpui_kit::component::IconName;

Spinner::new()
    .icon(IconName::LoaderCircle)

Spinner::new()
    .icon(IconName::LoaderCircle)
    .large()
    .color(cx.theme().cyan)

Spinner::new()
    .icon(IconName::Loader)
    .color(cx.theme().primary)
```

## 可用图标

### 加载图标

- `Loader`，默认的线形旋转图标
- `LoaderCircle`，圆形加载图标

### 其他兼容图标

- 理论上可使用 `IconName` 中任意图标，但带旋转语义的图标效果最好

## 动画

Spinner 内置旋转动画：

- 时长：`0.8` 秒
- 缓动：ease-in-out
- 循环：无限重复
- 变换：360 度旋转

## 尺寸参考

| 尺寸 | 方法 | 近似像素 |
| --- | --- | --- |
| 超小 | `.xsmall()` | ~12px |
| 小 | `.small()` | ~14px |
| 中 | 默认 | ~16px |
| 大 | `.large()` | ~24px |
| 自定义 | `.with_size(px(n))` | `n` px |

## 示例

### 加载状态

```rust
Spinner::new()

Spinner::new()
    .color(cx.theme().blue)

Spinner::new()
    .large()
    .color(cx.theme().primary)
```

### 不同加载图标

```rust
Spinner::new()
    .color(cx.theme().muted_foreground)

Spinner::new()
    .icon(IconName::LoaderCircle)
    .color(cx.theme().blue)

Spinner::new()
    .icon(IconName::LoaderCircle)
    .large()
    .color(cx.theme().green)
```

### 状态型 Spinner

```rust
Spinner::new()
    .small()
    .color(cx.theme().muted_foreground)

Spinner::new()
    .icon(IconName::LoaderCircle)
    .color(cx.theme().blue)

Spinner::new()
    .icon(IconName::LoaderCircle)
    .color(cx.theme().green)
```

### 在界面组件中使用

```rust
Button::new("submit-btn")
    .loading(true)
    .icon(
        Spinner::new()
            .small()
            .color(cx.theme().primary_foreground)
    )
    .label("Loading...")

// 在卡片头部
div()
    .flex()
    .items_center()
    .gap_2()
    .child("Processing...")
    .child(
        Spinner::new()
            .small()
            .color(cx.theme().muted_foreground)
    )
```

## 性能说明

- 动画基于 transform，性能开销较低
- 多个 Spinner 可共享相同动画节奏
- 组件本身较轻，适合频繁更新的界面
- 大量同时显示时，优先使用更小尺寸

## 常见模式

### 条件加载

```rust
.when(is_loading, |this| {
    this.child(
        Spinner::new()
            .small()
            .color(cx.theme().muted_foreground)
    )
})
```

### 文字配合加载图标

```rust
h_flex()
    .items_center()
    .gap_2()
    .child(
        Spinner::new()
            .small()
            .color(cx.theme().primary)
    )
    .child("Loading data...")
```
