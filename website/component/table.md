---
title: Table
description: A basic table component for directly rendering tabular data.
---

# Table

A simple, stateless, composable table component for rendering tabular data. Unlike [DataTable], this component does not include virtual scrolling, sorting, or column management — it is designed for straightforward data display using a declarative API.

## Import

```rust
use gpui_kit::component::table::{
    Table, TableHeader, TableBody, TableFooter,
    TableRow, TableHead, TableCell, TableCaption,
};
```

## Usage

### Basic Table

```rust
Table::new()
    .child(TableHeader::new().child(
        TableRow::new()
            .child(TableHead::new().child("Name"))
            .child(TableHead::new().child("Email"))
            .child(TableHead::new().text_right().child("Amount"))
    ))
    .child(TableBody::new()
        .child(TableRow::new()
            .child(TableCell::new().child("John"))
            .child(TableCell::new().child("john@example.com"))
            .child(TableCell::new().text_right().child("$100.00")))
        .child(TableRow::new()
            .child(TableCell::new().child("Jane"))
            .child(TableCell::new().child("jane@example.com"))
            .child(TableCell::new().text_right().child("$200.00")))
    )
    .child(TableCaption::new().child("A list of recent invoices."))
```

### With Footer

```rust
Table::new()
    .child(TableHeader::new().child(
        TableRow::new()
            .child(TableHead::new().child("Invoice"))
            .child(TableHead::new().child("Status"))
            .child(TableHead::new().text_right().child("Amount"))
    ))
    .child(TableBody::new()
        .child(TableRow::new()
            .child(TableCell::new().child("INV001"))
            .child(TableCell::new().child("Paid"))
            .child(TableCell::new().text_right().child("$250.00")))
    )
    .child(TableFooter::new().child(
        TableRow::new()
            .child(TableCell::new().child("Total"))
            .child(TableCell::new().child(""))
            .child(TableCell::new().text_right().child("$250.00"))
    ))
```

### Column Widths

Use `.w()` on `TableHead` and `TableCell` to set fixed column widths:

```rust
TableRow::new()
    .child(TableHead::new().w(px(80.)).child("ID"))
    .child(TableHead::new().child("Name"))  // flex-1
    .child(TableHead::new().w(px(120.)).child("Date"))
```

### Text Alignment

```rust
// Center-aligned header
TableHead::new().text_center().child("Status")

// Right-aligned cell (e.g., for numbers)
TableCell::new().text_right().child("$1,000.00")
```

### Without Border (via Styled)

All table sub-components implement the `Styled` trait, so you can customize styles directly:

```rust
// Remove border and rounded corners
Table::new()
    .border_0()
    .rounded_none()
    .child(/* ... */)
```

### Custom Styling

Since all components implement `Styled`, you can apply any GPUI style:

```rust
// Custom row hover
TableRow::new()
    .bg(cx.theme().table_even)
    .child(/* ... */)

// Custom cell padding
TableCell::new()
    .px_4()
    .child("Custom padded content")
```

## Sub-components

| Component | Description |
|-----------|-------------|
| `Table` | Root container with border, rounded corners, and background |
| `TableHeader` | Header section with distinct background and font weight |
| `TableBody` | Body section wrapping data rows |
| `TableFooter` | Footer section with top border |
| `TableRow` | A flex row with bottom border |
| `TableHead` | Header cell with alignment and width options |
| `TableCell` | Data cell with alignment and width options |
| `TableCaption` | Caption text below the table |

## API Reference

### Table

- `new()` - Create a new table
- Implements `Styled`, `ParentElement`, `Sizable`, `RenderOnce`

### TableHead / TableCell

- `new()` - Create a new head/cell
- `w(width)` - Set fixed width (otherwise flex-1)
- `text_center()` - Center-align content
- `text_right()` - Right-align content
- Implements `Styled`, `ParentElement`, `RenderOnce`

### TableHeader / TableBody / TableFooter / TableRow / TableCaption

- `new()` - Create a new instance
- Implements `Styled`, `ParentElement`, `RenderOnce`

## Table vs DataTable

| Feature | Table | DataTable |
|---------|-------|-----------|
| Virtual scrolling | No | Yes |
| Column sorting | No | Yes |
| Column resizing | No | Yes |
| Column moving | No | Yes |
| Cell selection | No | Yes |
| Row selection | No | Yes |
| Infinite loading | No | Yes |
| Keyboard navigation | No | Yes |
| State management | Stateless | TableState |
| Best for | Small, static data | Large, interactive datasets |

[DataTable]: ./data-table.md
