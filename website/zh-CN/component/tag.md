---
title: Tag
description: 用于分类、标记和显示元数据的紧凑标签组件。
---

# Tag

Tag 是一个轻量但灵活的标签组件，适合展示分类、状态、优先级和其他元数据。它体积紧凑，适合在列表、卡片和详情页中重复使用。

## 导入

```rust
use gpui_kit::component::tag::Tag;
```

## 用法

### 基础标签

```rust
Tag::primary().child("Primary")
Tag::secondary().child("Secondary")
Tag::danger().child("Danger")
Tag::success().child("Success")
Tag::warning().child("Warning")
Tag::info().child("Info")
```

### 语义变体

```rust
Tag::primary().child("Featured")
Tag::secondary().child("Category")
Tag::danger().child("Critical")
Tag::success().child("Completed")
Tag::warning().child("Pending")
Tag::info().child("Information")
```

### Outline 风格

```rust
Tag::primary().outline().child("Primary Outline")
Tag::secondary().outline().child("Secondary Outline")
Tag::danger().outline().child("Error Outline")
Tag::success().outline().child("Success Outline")
```

### 尺寸

```rust
Tag::primary().small().child("Small Tag")
Tag::primary().child("Medium Tag")
```

### 预设颜色

```rust
use gpui_kit::component::ColorName;

Tag::color(ColorName::Blue).child("Blue Tag")
Tag::color(ColorName::Green).child("Green Tag")
Tag::color(ColorName::Purple).child("Purple Tag")
Tag::color(ColorName::Pink).child("Pink Tag")
```

### 自定义 HSLA 颜色

```rust
use gpui_kit::{hsla, Hsla};

let color = hsla(220.0 / 360.0, 0.8, 0.5, 1.0);
let foreground = hsla(0.0, 0.0, 1.0, 1.0);
let border = hsla(220.0 / 360.0, 0.8, 0.4, 1.0);

Tag::custom(color, foreground, border).child("Custom Color")
```

### 圆角

```rust
use gpui_kit::px;

Tag::primary().rounded_full().child("Rounded Full")
Tag::primary().rounded(px(4.0)).child("Custom Radius")
Tag::primary().rounded(px(0.0)).child("Square Tag")
```

## 常见场景

### 状态标签

```rust
Tag::success().child("Completed")
Tag::warning().child("In Progress")
Tag::danger().child("Failed")
Tag::info().child("Pending Review")
```

### 分类标签

```rust
Tag::secondary().child("Technology")
Tag::color(ColorName::Blue).child("Design")
Tag::color(ColorName::Green).child("Development")
Tag::color(ColorName::Purple).child("Marketing")
```

### 优先级标签

```rust
Tag::danger().child("High Priority")
Tag::warning().child("Medium Priority")
Tag::secondary().child("Low Priority")
```

## API 参考

### 创建方法

| 方法 | 说明 |
| --- | --- |
| `primary()` | 主色标签 |
| `secondary()` | 次级标签 |
| `danger()` | 危险状态标签 |
| `success()` | 成功状态标签 |
| `warning()` | 警告状态标签 |
| `info()` | 信息标签 |
| `color(ColorName)` | 使用预设颜色创建标签 |
| `custom(color, fg, border)` | 使用自定义 HSLA 颜色创建标签 |

### 样式方法

| 方法 | 说明 |
| --- | --- |
| `outline()` | 使用描边风格 |
| `rounded(radius)` | 自定义圆角 |
| `rounded_full()` | 完整圆角，胶囊样式 |

### 尺寸方法

| 方法 | 说明 |
| --- | --- |
| `small()` | 小尺寸标签 |
| `with_size(size)` | 设置自定义尺寸 |

## 设计建议

- 状态类信息优先使用语义颜色，如 success、warning、danger
- 分类标签可结合 `ColorName` 做稳定的颜色映射
- 空间有限时优先使用 `small()`
- 纯展示标签不应默认承担交互职责
