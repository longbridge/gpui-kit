---
title: Tag
description: A short item that can be used to categorize or label content.
---

# Tag

A versatile tag component for categorizing and labeling content. Tags are compact visual indicators that help organize information and display metadata like categories, status, or properties.

## Import

```rust
use gpui_kit::component::tag::Tag;
```

## Usage

### Basic Tags

```rust
// Primary tag (default filled style)
Tag::primary().child("Primary")

// Secondary tag
Tag::secondary().child("Secondary")

// Status tags
Tag::danger().child("Danger")
Tag::success().child("Success")
Tag::warning().child("Warning")
Tag::info().child("Info")
```

### Tag Variants

```rust
// Semantic variants
Tag::primary().child("Featured")
Tag::secondary().child("Category")
Tag::danger().child("Critical")
Tag::success().child("Completed")
Tag::warning().child("Pending")
Tag::info().child("Information")
```

### Outline Tags

```rust
// Outline style variants
Tag::primary().outline().child("Primary Outline")
Tag::secondary().outline().child("Secondary Outline")
Tag::danger().outline().child("Error Outline")
Tag::success().outline().child("Success Outline")
Tag::warning().outline().child("Warning Outline")
Tag::info().outline().child("Info Outline")
```

### Tag Sizes

```rust
// Small size
Tag::primary().small().child("Small Tag")

// Medium size (default)
Tag::primary().child("Medium Tag")
```

### Custom Colors

```rust
use gpui_kit::component::ColorName;

// Using predefined color names
Tag::color(ColorName::Blue).child("Blue Tag")
Tag::color(ColorName::Green).child("Green Tag")
Tag::color(ColorName::Purple).child("Purple Tag")
Tag::color(ColorName::Pink).child("Pink Tag")
Tag::color(ColorName::Indigo).child("Indigo Tag")
Tag::color(ColorName::Yellow).child("Yellow Tag")
Tag::color(ColorName::Red).child("Red Tag")
```

### Custom HSLA Colors

```rust
use gpui_kit::{hsla, Hsla};

// Custom colors with HSLA values
let color = hsla(220.0 / 360.0, 0.8, 0.5, 1.0);
let foreground = hsla(0.0, 0.0, 1.0, 1.0);
let border = hsla(220.0 / 360.0, 0.8, 0.4, 1.0);

Tag::custom(color, foreground, border).child("Custom Color")
```

### Rounded Corners

```rust
use gpui_kit::px;

// Fully rounded tags
Tag::primary().rounded_full().child("Rounded Full")

// Custom border radius
Tag::primary().rounded(px(4.0)).child("Custom Radius")

// Square corners
Tag::primary().rounded(px(0.0)).child("Square Tag")
```

### Combined Styles

```rust
// Small tags with full rounding
Tag::primary().small().rounded_full().child("Small Pill")
Tag::success().small().rounded_full().child("Success Pill")

// Outline tags with custom rounding
Tag::warning().outline().rounded(px(2.0)).child("Custom Outline")

// Color tags with outline style
Tag::color(ColorName::Purple).outline().child("Purple Outline")
```

## Tag Categories and Use Cases

### Status Tags

```rust
// Task or item status
Tag::success().child("Completed")
Tag::warning().child("In Progress")
Tag::danger().child("Failed")
Tag::info().child("Pending Review")
```

### Category Labels

```rust
// Content categorization
Tag::secondary().child("Technology")
Tag::color(ColorName::Blue).child("Design")
Tag::color(ColorName::Green).child("Development")
Tag::color(ColorName::Purple).child("Marketing")
```

### Priority Indicators

```rust
// Priority levels
Tag::danger().child("High Priority")
Tag::warning().child("Medium Priority")
Tag::secondary().child("Low Priority")
```

### Feature Tags

```rust
// Feature flags or attributes
Tag::primary().small().child("New")
Tag::success().small().child("Popular")
Tag::info().small().child("Beta")
Tag::warning().small().child("Limited")
```

## API Reference

### Tag Creation Methods

| Method                      | Description                                |
| --------------------------- | ------------------------------------------ |
| `primary()`                 | Create a primary tag (blue theme)          |
| `secondary()`               | Create a secondary tag (gray theme)        |
| `danger()`                  | Create a danger tag (red theme)            |
| `success()`                 | Create a success tag (green theme)         |
| `warning()`                 | Create a warning tag (yellow/orange theme) |
| `info()`                    | Create an info tag (blue theme)            |
| `color(ColorName)`          | Create a tag with predefined color         |
| `custom(color, fg, border)` | Create a tag with custom HSLA colors       |

### Style Methods

| Method            | Description                                  |
| ----------------- | -------------------------------------------- |
| `outline()`       | Apply outline style (transparent background) |
| `rounded(radius)` | Set custom border radius                     |
| `rounded_full()`  | Apply full rounding (pill shape)             |

### Size Methods (from Sizable trait)

| Method            | Description                      |
| ----------------- | -------------------------------- |
| `small()`         | Small tag size (reduced padding) |
| `with_size(size)` | Set custom size                  |

### Content Methods (from ParentElement trait)

| Method           | Description                  |
| ---------------- | ---------------------------- |
| `child(element)` | Add child content to the tag |

## Examples

### Tag Collections

```rust
use gpui_kit::component::{h_flex, v_flex};

// Horizontal tag group
h_flex()
    .gap_2()
    .child(Tag::primary().child("React"))
    .child(Tag::success().child("TypeScript"))
    .child(Tag::info().child("Next.js"))
    .child(Tag::warning().child("Beta"))

// Vertical tag stack
v_flex()
    .gap_1()
    .child(Tag::danger().small().child("Critical"))
    .child(Tag::warning().small().child("Important"))
    .child(Tag::secondary().small().child("Normal"))
```

### Status Dashboard Tags

```rust
// System status indicators
h_flex()
    .gap_3()
    .child(
        v_flex()
            .child("API Status:")
            .child(Tag::success().child("Operational"))
    )
    .child(
        v_flex()
            .child("Database:")
            .child(Tag::warning().child("Maintenance"))
    )
    .child(
        v_flex()
            .child("Cache:")
            .child(Tag::danger().child("Down"))
    )
```

### Interactive Tag Lists

```rust
// Note: Event handling would require additional state management
// Tags themselves are display components

// Filter tags (would need click handlers)
h_flex()
    .gap_2()
    .child(Tag::primary().small().child("All"))
    .child(Tag::secondary().outline().small().child("Active"))
    .child(Tag::secondary().outline().small().child("Completed"))
    .child(Tag::secondary().outline().small().child("Archived"))
```

### Color-Coded Categories

```rust
use gpui_kit::component::ColorName;

// Content type tags
h_flex()
    .gap_2()
    .flex_wrap()
    .child(Tag::color(ColorName::Red).child("Bug"))
    .child(Tag::color(ColorName::Blue).child("Feature"))
    .child(Tag::color(ColorName::Green).child("Enhancement"))
    .child(Tag::color(ColorName::Purple).child("Documentation"))
    .child(Tag::color(ColorName::Yellow).child("Question"))
    .child(Tag::color(ColorName::Pink).child("Discussion"))
```

### Pill-Style Tags

```rust
// Skill tags with pill styling
h_flex()
    .gap_2()
    .flex_wrap()
    .child(Tag::color(ColorName::Blue).rounded_full().small().child("Rust"))
    .child(Tag::color(ColorName::Green).rounded_full().small().child("JavaScript"))
    .child(Tag::color(ColorName::Purple).rounded_full().small().child("Python"))
    .child(Tag::color(ColorName::Red).rounded_full().small().child("Go"))
```

## Behavior Notes

- Tags automatically adjust their appearance based on the current theme
- Outline tags maintain border visibility across different backgrounds
- Small tags use reduced padding and border radius for compact layouts
- Custom colors support both light and dark theme adaptations
- Tags are display components and don't include built-in interaction handlers
- Multiple tags can be combined in flex layouts for tag clouds or lists
- Border radius automatically scales based on tag size unless explicitly overridden

## Design Guidelines

### When to Use Tags

- **Categorization**: Group content by type, topic, or theme
- **Status Indication**: Show state, progress, or health status
- **Metadata Display**: Present attributes, properties, or classifications
- **Filtering**: Visual indicators for active filters or selections
- **Feature Flags**: Highlight new, beta, or special features

### Color Usage

- **Semantic Colors**: Use danger (red) for errors, success (green) for completion, warning (yellow) for caution, info (blue) for information
- **Category Colors**: Use the ColorName variants for content categorization where color coding helps with recognition
- **Custom Colors**: Reserve for brand colors or specific design system requirements

### Size Guidelines

- **Small Tags**: Use for compact layouts, metadata, or when space is limited
- **Medium Tags**: Default size for most use cases, provides good readability and click targets
- **Rounding**: Use `rounded_full()` for pill-style tags, custom `rounded()` for specific design requirements
