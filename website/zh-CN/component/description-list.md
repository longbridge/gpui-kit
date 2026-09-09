---
title: DescriptionList
description: 用于以整齐布局展示键值对详情信息的组件。
---

# DescriptionList

DescriptionList 是一个用于展示键值对的通用组件，支持横向和纵向布局、多列、分隔线和不同尺寸，适合展示元数据、规格信息和摘要数据。

## 导入

```rust
use gpui_kit::component::description_list::{DescriptionList, DescriptionItem, DescriptionText};
```

## 用法

### 基础列表

```rust
DescriptionList::new()
    .item("Name", "GPUI Kit", 1)
    .item("Version", "0.1.0", 1)
    .item("License", "Apache-2.0", 1)
```

### 使用 DescriptionItem Builder

```rust
DescriptionList::new()
    .children([
        DescriptionItem::new("Name").value("GPUI Kit"),
        DescriptionItem::new("Description").value("UI components for building desktop applications"),
        DescriptionItem::new("Version").value("0.1.0"),
    ])
```

### 不同布局

```rust
DescriptionList::horizontal()
    .item("Platform", "macOS, Windows, Linux", 1)
    .item("Repository", "https://github.com/longbridge/gpui-kit", 1)

DescriptionList::vertical()
    .item("Name", "GPUI Kit", 1)
    .item("Description", "A comprehensive Rust desktop framework", 1)
```

### 多列和跨列

```rust
DescriptionList::new()
    .columns(3)
    .child(DescriptionItem::new("Name").value("GPUI Kit").span(1))
    .children([
        DescriptionItem::new("Version").value("0.1.0").span(1),
        DescriptionItem::new("License").value("Apache-2.0").span(1),
        DescriptionItem::new("Description")
            .value("Full-featured UI components for desktop applications")
            .span(3),
        DescriptionItem::new("Repository")
            .value("https://github.com/longbridge/gpui-kit")
            .span(2),
    ])
```

### 分隔线

```rust
DescriptionList::new()
    .item("Name", "GPUI Kit", 1)
    .item("Version", "0.1.0", 1)
    .separator()
    .item("Author", "Longbridge", 1)
    .item("License", "Apache-2.0", 1)
```

### 不同尺寸

```rust
DescriptionList::new()
    .large()
    .item("Title", "Large Description List", 1)

DescriptionList::new()
    .item("Title", "Medium Description List", 1)

DescriptionList::new()
    .small()
    .item("Title", "Small Description List", 1)
```

### 无边框

```rust
DescriptionList::new()
    .bordered(false)
    .item("Name", "GPUI Kit", 1)
    .item("Type", "UI Library", 1)
```

### 自定义标签宽度

```rust
use gpui_kit::px;

DescriptionList::horizontal()
    .label_width(px(200.0))
    .item("Very Long Label Name", "Short Value", 1)
    .item("Short", "Very long value that needs more space", 1)
```

### 富文本内容

```rust
use gpui_kit::component::text::markdown;

DescriptionList::new()
    .columns(2)
    .children([
        DescriptionItem::new("Name").value("GPUI Kit"),
        DescriptionItem::new("Description").value(
            markdown(
                "UI components for building **fantastic** desktop applications.",
            ).into_any_element()
        ),
    ])
```

### 混合内容示例

```rust
DescriptionList::new()
    .columns(3)
    .label_width(px(150.0))
    .children([
        DescriptionItem::new("Project Name").value("GPUI Kit").span(1),
        DescriptionItem::new("Version").value("0.1.0").span(1),
        DescriptionItem::new("Status").value("Active").span(1),

        DescriptionItem::Separator,

        DescriptionItem::new("Description").value(
            "A comprehensive Rust desktop framework built on GPUI"
        ).span(3),

        DescriptionItem::new("Repository").value(
            "https://github.com/longbridge/gpui-kit"
        ).span(2),
        DescriptionItem::new("License").value("Apache-2.0").span(1),

        DescriptionItem::new("Platforms").value("macOS, Windows, Linux").span(2),
        DescriptionItem::new("Language").value("Rust").span(1),
    ])
```

## 示例

### 用户资料信息

```rust
DescriptionList::new()
    .columns(2)
    .bordered(true)
    .children([
        DescriptionItem::new("Full Name").value("John Doe"),
        DescriptionItem::new("Email").value("john@example.com"),
        DescriptionItem::new("Phone").value("+1 (555) 123-4567"),
        DescriptionItem::new("Department").value("Engineering"),
        DescriptionItem::Separator,
        DescriptionItem::new("Bio").value(
            "Senior software engineer with 10+ years of experience in Rust and system programming."
        ).span(2),
    ])
```

### 系统信息

```rust
DescriptionList::vertical()
    .small()
    .bordered(false)
    .children([
        DescriptionItem::new("Operating System").value("macOS 14.0"),
        DescriptionItem::new("Architecture").value("Apple Silicon (M2)"),
        DescriptionItem::new("Memory").value("16 GB"),
        DescriptionItem::new("Storage").value("512 GB SSD"),
        DescriptionItem::new("GPU").value("Apple M2 10-core GPU"),
    ])
```

### 产品规格

```rust
DescriptionList::new()
    .columns(3)
    .large()
    .children([
        DescriptionItem::new("Model").value("MacBook Pro").span(1),
        DescriptionItem::new("Year").value("2023").span(1),
        DescriptionItem::new("Screen Size").value("14-inch").span(1),

        DescriptionItem::new("Processor").value("Apple M2 Pro").span(2),
        DescriptionItem::new("Base Price").value("$1,999").span(1),

        DescriptionItem::Separator,

        DescriptionItem::new("Key Features").value(
            "Liquid Retina XDR display, ProMotion technology, P3 wide color gamut"
        ).span(3),
    ])
```

### 配置项展示

```rust
DescriptionList::horizontal()
    .label_width(px(180.0))
    .bordered(false)
    .children([
        DescriptionItem::new("Theme").value("Dark Mode"),
        DescriptionItem::new("Font Size").value("14px"),
        DescriptionItem::new("Auto Save").value("Enabled"),
        DescriptionItem::new("Backup Frequency").value("Every 30 minutes"),
        DescriptionItem::new("Language").value("English (US)"),
    ])
```

## 设计建议

- 简单键值对优先使用横向布局。
- 值较长或结构较复杂时优先使用纵向布局。
- 列数尽量控制在 3 到 4 列以内。
- 使用分隔线对相关信息分组。
- 标签保持简洁且语义明确。
- 使用尺寸属性统一间距和密度。
- 嵌入式场景下可考虑关闭边框。
