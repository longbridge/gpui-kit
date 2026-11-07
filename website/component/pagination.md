---
title: Pagination
description: Pagination with page navigation, next and previous links.
---

# Pagination

The [Pagination] component provides page navigation with next and previous links. It displays page numbers and allows users to navigate through multiple pages of content.

## Import

```rust
use gpui_kit::component::pagination::Pagination;
```

## Usage

### Basic Pagination

```rust
Pagination::new("my-pagination")
    .current_page(5)
    .total_pages(10)
    .on_click(|page, _, cx| {
        println!("Navigated to page: {}", page);
    })
```

### With Visible Pages

By default, the pagination shows up to 5 visible page buttons. You can customize this with `visible_pages()`:

```rust
Pagination::new("my-pagination")
    .current_page(1)
    .total_pages(50)
    .visible_pages(10)
    .on_click(|page, _, cx| {
        // Handle page change
    })
```

### Compact Style

The compact style only shows the previous and next buttons with icons, without displaying page numbers.

Use `compact` method to enable compact style:

```rust
Pagination::new("my-pagination")
    .compact()
    .current_page(3)
    .total_pages(10)
    .on_click(|page, _, cx| {
        // Handle page change
    })
```

### Different Sizes

The Pagination supports the [Sizable] trait for different sizes:

```rust
use gpui_kit::component::{Sizable as _, Size};

Pagination::new("my-pagination")
    .xsmall()
    .current_page(1)
    .total_pages(10)

Pagination::new("my-pagination")
    .small()
    .current_page(1)
    .total_pages(10)

Pagination::new("my-pagination")
    .current_page(1)
    .total_pages(10) // Medium (default)

Pagination::new("my-pagination")
    .large()
    .current_page(1)
    .total_pages(10)
```

### Disabled State

```rust
Pagination::new("my-pagination")
    .current_page(4)
    .total_pages(10)
    .disabled(true)
    .on_click(|_, _, _| {})
```

### Handle Page Change Events

The `on_click` callback receives the new page number when users click on page numbers, previous, or next buttons:

```rust
Pagination::new("my-pagination")
    .current_page(current_page)
    .total_pages(total_pages)
    .on_click(|page, _, cx| {
        // Update your state with the new page
        // The page number is 1-based
    })
```

## API Reference

- [Pagination]

### Sizing

Implements [Sizable] trait:

- `xsmall()` - Extra small size
- `small()` - Small size
- `medium()` - Medium size (default)
- `large()` - Large size
- `with_size(size)` - Set custom size

### Methods

- `current_page(page: usize)` - Set the current page number (1-based). The value will be clamped between 1 and total_pages.
- `total_pages(pages: usize)` - Set the total number of pages.
- `visible_pages(max: usize)` - Set the maximum number of visible page buttons (default: 5).
- `compact()` - Enable compact style (only shows prev/next buttons with icons).
- `disabled(bool)` - Set the disabled state.
- `on_click(handler)` - Set the handler for page change events.

## Examples

### With State Management

```rust
let mut current_page = 1;
let total_pages = 20;

Pagination::new("pagination")
    .current_page(current_page)
    .total_pages(total_pages)
    .on_click({
        let entity = entity.clone();
        move |page, _, cx| {
            entity.update(cx, |this, cx| {
                this.current_page = *page;
                cx.notify();
            });
        }
    })
```

### Large Dataset Pagination

For large datasets, use `visible_pages()` to show more page options:

```rust
Pagination::new("large-pagination")
    .current_page(25)
    .total_pages(100)
    .visible_pages(10)
    .on_click(|page, _, cx| {
        // Load data for the new page
    })
```

[Pagination]: https://docs.rs/gpui-component/latest/gpui_component/pagination/struct.Pagination.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html
