---
title: Badge
description: 用于显示未读数、状态点或图标提示的小徽标组件。
---

# Badge

Badge 是一个通用徽标组件，可在头像、图标或其他元素上显示数字、圆点或图标。适合用来表示通知数、状态或上下文提示信息。

## 导入

```rust
use gpui_kit::component::badge::Badge;
```

## 用法

### 显示数字

使用 `count` 显示数字徽标。只有当数字大于 0 时才会显示；否则自动隐藏。

默认最大值是 `99`，超过后显示为 `99+`。你也可以通过 `max` 自定义上限。

```rust
Badge::new()
    .count(3)
    .child(Icon::new(IconName::Bell))
```

### 不同变体

- 默认：显示数字
- Dot：显示状态圆点
- Icon：显示图标

```rust
Badge::new()
    .count(5)
    .child(Avatar::new().src("https://example.com/avatar.jpg"))

Badge::new()
    .dot()
    .child(Icon::new(IconName::Inbox))

Badge::new()
    .icon(IconName::Check)
    .child(Avatar::new().src("https://example.com/avatar.jpg"))
```

### 不同尺寸

Badge 也实现了 [Sizable] trait：

```rust
Badge::new()
    .small()
    .count(1)
    .child(Avatar::new().small())

Badge::new()
    .count(5)
    .child(Avatar::new())

Badge::new()
    .large()
    .count(10)
    .child(Avatar::new().large())
```

### 颜色

```rust
use gpui_kit::component::ActiveTheme;

Badge::new()
    .count(3)
    .color(cx.theme().blue)
    .child(Avatar::new())

Badge::new()
    .icon(IconName::Star)
    .color(cx.theme().yellow)
    .child(Avatar::new())

Badge::new()
    .dot()
    .color(cx.theme().green)
    .child(Icon::new(IconName::Bell))
```

### 用在图标上

```rust
use gpui_kit::component::{Icon, IconName};

Badge::new()
    .count(3)
    .child(Icon::new(IconName::Bell).large())

Badge::new()
    .count(103)
    .child(Icon::new(IconName::Inbox).large())

Badge::new()
    .count(150)
    .max(999)
    .child(Icon::new(IconName::Mail))
```

### 用在头像上

```rust
use gpui_kit::component::avatar::Avatar;

Badge::new()
    .count(5)
    .child(Avatar::new().src("https://example.com/avatar.jpg"))

Badge::new()
    .icon(IconName::Check)
    .color(cx.theme().green)
    .child(Avatar::new().src("https://example.com/avatar.jpg"))

Badge::new()
    .dot()
    .color(cx.theme().green)
    .child(Avatar::new().src("https://example.com/avatar.jpg"))
```

### 复杂嵌套

```rust
Badge::new()
    .count(212)
    .large()
    .child(
        Badge::new()
            .icon(IconName::Check)
            .large()
            .color(cx.theme().cyan)
            .child(Avatar::new().large().src("https://example.com/avatar.jpg"))
    )

Badge::new()
    .count(2)
    .color(cx.theme().green)
    .large()
    .child(
        Badge::new()
            .icon(IconName::Star)
            .large()
            .color(cx.theme().yellow)
            .child(Avatar::new().large().src("https://example.com/avatar.jpg"))
    )
```

## API 参考

- [Badge]

## 示例

### 通知提示

```rust
Badge::new()
    .count(12)
    .child(Icon::new(IconName::Mail).large())

Badge::new()
    .count(3)
    .color(cx.theme().red)
    .child(Icon::new(IconName::Bell).large())

Badge::new()
    .count(1234)
    .max(999)
    .color(cx.theme().orange)
    .child(Icon::new(IconName::AlertTriangle))
```

### 状态提示

```rust
Badge::new()
    .dot()
    .color(cx.theme().green)
    .child(Avatar::new().src("https://example.com/user.jpg"))

Badge::new()
    .icon(IconName::CheckCircle)
    .color(cx.theme().blue)
    .child(Avatar::new().src("https://example.com/verified-user.jpg"))

Badge::new()
    .icon(IconName::AlertTriangle)
    .color(cx.theme().yellow)
    .child(Avatar::new().src("https://example.com/user.jpg"))
```

### 显示位置

```rust
// Badge 会根据变体自动选择位置：
// - Dot：右上角小圆点
// - Number：右上角数字徽标
// - Icon：右下角图标徽标
```

### 数字格式

```rust
Badge::new().count(5)
Badge::new().count(99)

Badge::new().count(100)
Badge::new().count(1000).max(999)

Badge::new().count(0)
```

[Badge]: https://docs.rs/gpui_component/latest/gpui_component/badge/struct.Badge.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html
