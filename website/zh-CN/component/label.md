---
title: Label
description: 支持高亮、次要文本和样式定制的文本标签组件。
---

# Label

Label 是一个灵活的文本标签组件，可用于表单标签、说明文字和通用文本展示。它支持次要文本、高亮、掩码显示以及丰富的样式定制。

## 导入

```rust
use gpui_kit::component::label::{Label, HighlightsMatch};
```

## 用法

### 基础标签

```rust
Label::new("This is a label")
```

### 带次要文本

```rust
Label::new("Company Address")
    .secondary("(optional)")

Label::new("Email Address")
    .secondary("(required)")
```

### 文本对齐

```rust
Label::new("Text align left")

Label::new("Text align center")
    .text_center()

Label::new("Text align right")
    .text_right()
```

### 文本高亮

```rust
Label::new("Hello World Hello")
    .highlights("Hello")

Label::new("Hello World")
    .highlights(HighlightsMatch::Prefix("Hello".into()))

Label::new("Company Name")
    .secondary("(optional)")
    .highlights("Company")
```

### 颜色与字体样式

```rust
use gpui_kit::component::green_500;

Label::new("Color Label")
    .text_color(green_500())

Label::new("Font Size Label")
    .text_size(px(20.))
    .font_semibold()
    .line_height(rems(1.8))
```

### 掩码文本

```rust
Label::new("9,182,1 USD")
    .text_2xl()
    .masked(true)

Label::new("500 USD")
    .text_xl()
    .masked(self.masked)
```

### 多行文本

```rust
div().w(px(200.)).child(
    Label::new(
        "Label should support text wrap in default, \
        if the text is too long, it should wrap to the next line."
    )
    .line_height(rems(1.8))
)
```

### 不同尺寸

```rust
Label::new("Extra Large").text_2xl()
Label::new("Large").text_xl()
Label::new("Medium").text_base()
Label::new("Small").text_sm()
Label::new("Extra Small").text_xs()
```

## API 参考

### Label

| 方法 | 说明 |
| ------------------- | ------------------------------------------------------------- |
| `new(text)` | 使用文本创建标签 |
| `secondary(text)` | 添加次要文本，常用于 optional 或 required 标识 |
| `masked(bool)` | 使用圆点字符隐藏文本 |
| `highlights(match)` | 高亮匹配内容 |

### HighlightsMatch

| 变体 | 说明 |
| -------------- | ------------------------------------------------ |
| `Full(text)` | 高亮所有匹配内容 |
| `Prefix(text)` | 仅在文本开头匹配时高亮 |

| 方法 | 说明 |
| ------------- | ------------------------------- |
| `as_str()` | 获取匹配字符串 |
| `is_prefix()` | 判断是否为前缀匹配 |

### 样式方法（来自 Styled trait）

| 方法 | 说明 |
| --------------------- | --------------------------- |
| `text_color(color)` | 设置文字颜色 |
| `text_size(size)` | 设置字体大小 |
| `text_center()` | 居中对齐 |
| `text_right()` | 右对齐 |
| `font_semibold()` | 半粗体 |
| `font_bold()` | 粗体 |
| `line_height(height)` | 设置行高 |
| `text_xs()` | 超小字号 |
| `text_sm()` | 小字号 |
| `text_base()` | 默认字号 |
| `text_lg()` | 大字号 |
| `text_xl()` | 超大字号 |
| `text_2xl()` | 2 倍大字号 |

## 示例

### 表单标签

```rust
Label::new("Email Address")
    .secondary("*")
    .text_color(cx.theme().destructive)

Label::new("Phone Number")
    .secondary("(optional)")

Label::new("Password")
    .secondary("(minimum 8 characters)")
```

### 搜索高亮

```rust
let search_term = "Hello";
Label::new("Hello World Hello Universe")
    .highlights(search_term)
```

### 敏感信息

```rust
h_flex()
    .child(
        Label::new("$9,182.50 USD")
            .text_2xl()
            .masked(self.is_masked)
    )
    .child(
        Button::new("toggle-mask")
            .ghost()
            .icon(if self.is_masked { IconName::EyeOff } else { IconName::Eye })
            .on_click(|this, _, _, _| {
                this.is_masked = !this.is_masked;
            })
    )
```

### 多语言支持

```rust
Label::new("这是一个标签")
Label::new("こんにちは世界")
Label::new("🌍 Hello World 🚀")
```

### 状态提示

```rust
Label::new("✓ Verified")
    .text_color(cx.theme().success)

Label::new("⚠ Pending Review")
    .text_color(cx.theme().warning)

Label::new("✗ Failed")
    .text_color(cx.theme().destructive)
```

### 自定义布局

```rust
h_flex()
    .justify_between()
    .child(Label::new("Total Amount"))
    .child(Label::new("$1,234.56").font_semibold())

v_flex()
    .gap_2()
    .child(Label::new("Name:").font_semibold())
    .child(Label::new("John Doe"))
    .child(Label::new("Email:").font_semibold())
    .child(Label::new("john@example.com"))
```
