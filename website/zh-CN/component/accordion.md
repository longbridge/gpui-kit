---
title: Accordion
description: 内部基于 collapse 实现的可折叠面板组件。
---

# Accordion

Accordion 是一个可折叠内容组件，允许用户展开和收起多个内容区块。它内部基于 collapse 功能实现，适合 FAQ、设置分组和分段内容展示。

## 导入

```rust
use gpui_kit::component::accordion::Accordion;
```

## 用法

### 基础 Accordion

```rust
Accordion::new("my-accordion")
    .item(|item| {
        item.title("Section 1")
            .child("Content for section 1")
    })
    .item(|item| {
        item.title("Section 2")
            .child("Content for section 2")
    })
    .item(|item| {
        item.title("Section 3")
            .child("Content for section 3")
    })
```

### 允许同时展开多个项

默认情况下，一次只能展开一个项。使用 `multiple()` 可以允许多个项同时展开：

```rust
Accordion::new("my-accordion")
    .multiple(true)
    .item(|item| item.title("Section 1").child("Content 1"))
    .item(|item| item.title("Section 2").child("Content 2"))
```

### 带边框

```rust
Accordion::new("my-accordion")
    .bordered(true)
    .item(|item| item.title("Section 1").child("Content 1"))
```

### 不同尺寸

```rust
use gpui_kit::component::{Sizable as _, Size};

Accordion::new("my-accordion")
    .small()
    .item(|item| item.title("Small Section").child("Content"))

Accordion::new("my-accordion")
    .large()
    .item(|item| item.title("Large Section").child("Content"))
```

### 处理切换事件

```rust
Accordion::new("my-accordion")
    .on_toggle_click(|open_indices, window, cx| {
        println!("Open items: {:?}", open_indices);
    })
    .item(|item| item.title("Section 1").child("Content 1"))
```

### 禁用状态

```rust
Accordion::new("my-accordion")
    .disabled(true)
    .item(|item| item.title("Disabled Section").child("Content"))
```

## API 参考

- [Accordion]
- [AccordionItem]

### 尺寸

实现了 [Sizable] trait：

- `small()`：小尺寸
- `medium()`：中尺寸，默认值
- `large()`：大尺寸
- `xsmall()`：超小尺寸

## 示例

### 自定义图标标题

```rust
Accordion::new("my-accordion")
    .item(|item| {
        item.title(
            h_flex()
                .gap_2()
                .child(Icon::new(IconName::Settings))
                .child("Settings")
        )
        .child("Settings content here")
    })
```

### 嵌套 Accordion

```rust
Accordion::new("outer")
    .item(|item| {
        item.title("Parent Section")
            .child(
                Accordion::new("inner")
                    .item(|item| item.title("Child 1").child("Content"))
                    .item(|item| item.title("Child 2").child("Content"))
            )
    })
```

[Accordion]: https://docs.rs/gpui-component/latest/gpui_component/accordion/struct.Accordion.html
[AccordionItem]: https://docs.rs/gpui-component/latest/gpui_component/accordion/struct.AccordionItem.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html
