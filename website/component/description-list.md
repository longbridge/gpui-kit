---
title: DescriptionList
description: Use to display details with a tidy layout for key-value pairs.
---

# DescriptionList

A versatile component for displaying key-value pairs in a structured, organized layout. Supports both horizontal and vertical layouts, multiple columns, borders, and different sizes. Perfect for showing detailed information like metadata, specifications, or summary data.

## Import

```rust
use gpui_kit::component::description_list::{DescriptionList, DescriptionItem, DescriptionText};
```

## Usage

### Basic Description List

```rust
DescriptionList::new()
    .item("Name", "GPUI Kit", 1)
    .item("Version", "0.1.0", 1)
    .item("License", "Apache-2.0", 1)
```

### Using DescriptionItem Builder

```rust
DescriptionList::new()
    .children([
        DescriptionItem::new("Name").value("GPUI Kit"),
        DescriptionItem::new("Description").value("UI components for building desktop applications"),
        DescriptionItem::new("Version").value("0.1.0"),
    ])
```

### Different Layouts

```rust
// Horizontal layout (default)
DescriptionList::horizontal()
    .item("Platform", "macOS, Windows, Linux", 1)
    .item("Repository", "https://github.com/longbridge/gpui-kit", 1)

// Vertical layout
DescriptionList::vertical()
    .item("Name", "GPUI Kit", 1)
    .item("Description", "A comprehensive Rust desktop framework", 1)
```

### Multiple Columns with Spans

```rust
DescriptionList::new()
    .columns(3)
    .child(DescriptionItem::new("Name").value("GPUI Kit").span(1))
    .children([
        DescriptionItem::new("Version").value("0.1.0").span(1),
        DescriptionItem::new("License").value("Apache-2.0").span(1),
        DescriptionItem::new("Description")
            .value("Full-featured UI components for desktop applications")
            .span(3), // Spans all 3 columns
        DescriptionItem::new("Repository")
            .value("https://github.com/longbridge/gpui-kit")
            .span(2), // Spans 2 columns
    ])
```

### With Separators

```rust
DescriptionList::new()
    .item("Name", "GPUI Kit", 1)
    .item("Version", "0.1.0", 1)
    .separator() // Add a visual separator
    .item("Author", "Longbridge", 1)
    .item("License", "Apache-2.0", 1)
```

### Different Sizes

```rust
// Large size
DescriptionList::new()
    .large()
    .item("Title", "Large Description List", 1)

// Medium size (default)
DescriptionList::new()
    .item("Title", "Medium Description List", 1)

// Small size
DescriptionList::new()
    .small()
    .item("Title", "Small Description List", 1)
```

### Without Borders

```rust
DescriptionList::new()
    .bordered(false) // Remove borders for a cleaner look
    .item("Name", "GPUI Kit", 1)
    .item("Type", "UI Library", 1)
```

### Custom Label Width (Horizontal Layout)

```rust
use gpui_kit::px;

DescriptionList::horizontal()
    .label_width(px(200.0)) // Set custom label width
    .item("Very Long Label Name", "Short Value", 1)
    .item("Short", "Very long value that needs more space", 1)
```

### Rich Content with Custom Elements

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

### Complex Example with Mixed Content

```rust
DescriptionList::new()
    .columns(3)
    .label_width(px(150.0))
    .children([
        DescriptionItem::new("Project Name").value("GPUI Kit").span(1),
        DescriptionItem::new("Version").value("0.1.0").span(1),
        DescriptionItem::new("Status").value("Active").span(1),

        DescriptionItem::Separator, // Full-width separator

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

## Examples

### User Profile Information

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

### System Information

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

### Product Specifications

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

### Configuration Settings

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

## Design Guidelines

- Use horizontal layout for simple key-value pairs
- Use vertical layout when values are lengthy or complex
- Limit columns to 3-4 for optimal readability
- Use separators to group related information
- Keep labels concise and descriptive
- Use consistent spacing with the size prop
- Consider removing borders for embedded contexts
