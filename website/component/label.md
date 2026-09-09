---
title: Label
description: Text labels for form elements with highlighting and styling options.
---

# Label

A versatile label component for displaying text with support for secondary text, highlighting, masking, and customizable styling. Perfect for form labels, captions, and general text display with optional/required indicators.

## Import

```rust
use gpui_kit::component::label::{Label, HighlightsMatch};
```

## Usage

### Basic Label

```rust
Label::new("This is a label")
```

### Label with Secondary Text

```rust
// Label with optional indicator
Label::new("Company Address")
    .secondary("(optional)")

// Label with required indicator
Label::new("Email Address")
    .secondary("(required)")
```

### Text Alignment

```rust
// Left aligned (default)
Label::new("Text align left")

// Center aligned
Label::new("Text align center")
    .text_center()

// Right aligned
Label::new("Text align right")
    .text_right()
```

### Text Highlighting

```rust
// Full text highlighting (finds all matches)
Label::new("Hello World Hello")
    .highlights("Hello")

// Prefix highlighting (only matches at start)
Label::new("Hello World")
    .highlights(HighlightsMatch::Prefix("Hello".into()))

// Highlight with secondary text
Label::new("Company Name")
    .secondary("(optional)")
    .highlights("Company")
```

### Color and Styling

```rust
use gpui_kit::component::green_500;

// Custom text color
Label::new("Color Label")
    .text_color(green_500())

// Font styling
Label::new("Font Size Label")
    .text_size(px(20.))
    .font_semibold()
    .line_height(rems(1.8))
```

### Masked Labels

```rust
// For sensitive information
Label::new("9,182,1 USD")
    .text_2xl()
    .masked(true) // Shows as "•••••••••••"

// Toggle masking programmatically
Label::new("500 USD")
    .text_xl()
    .masked(self.masked)
```

### Multi-line Text

```rust
// Text wrapping with line height
div().w(px(200.)).child(
    Label::new(
        "Label should support text wrap in default, \
        if the text is too long, it should wrap to the next line."
    )
    .line_height(rems(1.8))
)
```

### Different Sizes

```rust
// Using text size utilities
Label::new("Extra Large").text_2xl()
Label::new("Large").text_xl()
Label::new("Medium").text_base() // default
Label::new("Small").text_sm()
Label::new("Extra Small").text_xs()
```

## API Reference

### Label

| Method              | Description                                                   |
| ------------------- | ------------------------------------------------------------- |
| `new(text)`         | Create a new label with text                                  |
| `secondary(text)`   | Add secondary text (usually for optional/required indicators) |
| `masked(bool)`      | Show/hide text with bullet characters                         |
| `highlights(match)` | Highlight matching text                                       |

### HighlightsMatch

| Variant        | Description                                      |
| -------------- | ------------------------------------------------ |
| `Full(text)`   | Highlights all occurrences of the text           |
| `Prefix(text)` | Highlights only if text appears at the beginning |

| Method        | Description                     |
| ------------- | ------------------------------- |
| `as_str()`    | Get the search text as string   |
| `is_prefix()` | Check if this is a prefix match |

### Styling Methods (via Styled trait)

| Method                | Description                 |
| --------------------- | --------------------------- |
| `text_color(color)`   | Set text color              |
| `text_size(size)`     | Set font size               |
| `text_center()`       | Center align text           |
| `text_right()`        | Right align text            |
| `font_semibold()`     | Set font weight to semibold |
| `font_bold()`         | Set font weight to bold     |
| `line_height(height)` | Set line height             |
| `text_xs()`           | Extra small text size       |
| `text_sm()`           | Small text size             |
| `text_base()`         | Base text size (default)    |
| `text_lg()`           | Large text size             |
| `text_xl()`           | Extra large text size       |
| `text_2xl()`          | 2x large text size          |

## Examples

### Form Labels

```rust
// Required field
Label::new("Email Address")
    .secondary("*")
    .text_color(cx.theme().destructive)

// Optional field
Label::new("Phone Number")
    .secondary("(optional)")

// Field with description
Label::new("Password")
    .secondary("(minimum 8 characters)")
```

### Search Highlighting

```rust
// Interactive search highlighting
let search_term = "Hello";
Label::new("Hello World Hello Universe")
    .highlights(search_term) // Highlights all "Hello" occurrences
```

### Sensitive Information

```rust
// Financial data with toggle
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

### Multi-language Support

```rust
// Supports Unicode text
Label::new("这是一个标签") // Chinese text
Label::new("こんにちは世界") // Japanese text
Label::new("🌍 Hello World 🚀") // Emojis
```

### Status Indicators

```rust
// Success status
Label::new("✓ Verified")
    .text_color(cx.theme().success)

// Warning status
Label::new("⚠ Pending Review")
    .text_color(cx.theme().warning)

// Error status
Label::new("✗ Failed")
    .text_color(cx.theme().destructive)
```

### Custom Layouts

```rust
// Flex layout with labels
h_flex()
    .justify_between()
    .child(Label::new("Total Amount"))
    .child(Label::new("$1,234.56").font_semibold())

// Grid layout
v_flex()
    .gap_2()
    .child(Label::new("Name:").font_semibold())
    .child(Label::new("John Doe"))
    .child(Label::new("Email:").font_semibold())
    .child(Label::new("john@example.com"))
```
