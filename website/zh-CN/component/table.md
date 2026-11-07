---
title: Table
description: 一个用于直接渲染表格数据的基础表格组件。
---

# Table

Table 是一个简单、无状态、可组合的表格组件，用于渲染表格型数据。与 [DataTable] 不同，它不包含虚拟滚动、排序或列管理能力，更适合直接用声明式 API 展示较小且静态的数据。

## 导入

```rust
use gpui_kit::component::table::{
    Table, TableHeader, TableBody, TableFooter,
    TableRow, TableHead, TableCell, TableCaption,
};
```

## 用法

### 基础表格

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

### 带 Footer

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

### 列宽

可以在 `TableHead` 和 `TableCell` 上使用 `.w()` 设置固定列宽：

```rust
TableRow::new()
    .child(TableHead::new().w(px(80.)).child("ID"))
    .child(TableHead::new().child("Name"))
    .child(TableHead::new().w(px(120.)).child("Date"))
```

### 文本对齐

```rust
TableHead::new().text_center().child("Status")

TableCell::new().text_right().child("$1,000.00")
```

### 去掉边框

所有表格子组件都实现了 `Styled`，可以直接自定义样式：

```rust
Table::new()
    .border_0()
    .rounded_none()
    .child(/* ... */)
```

### 自定义样式

```rust
TableRow::new()
    .bg(cx.theme().table_even)
    .child(/* ... */)

TableCell::new()
    .px_4()
    .child("Custom padded content")
```

## 子组件

| 组件 | 说明 |
| --- | --- |
| `Table` | 根容器，带边框、圆角和背景 |
| `TableHeader` | 表头区域 |
| `TableBody` | 表体区域 |
| `TableFooter` | 表尾区域 |
| `TableRow` | 一行数据 |
| `TableHead` | 表头单元格 |
| `TableCell` | 数据单元格 |
| `TableCaption` | 表格下方说明文字 |

## API 摘要

### Table

- `new()` - 创建新表格
- 实现了 `Styled`、`ParentElement`、`Sizable`、`RenderOnce`

### TableHead / TableCell

- `new()` - 创建单元格
- `w(width)` - 设置固定宽度
- `text_center()` - 居中对齐
- `text_right()` - 右对齐

### TableHeader / TableBody / TableFooter / TableRow / TableCaption

- `new()` - 创建实例
- 实现了 `Styled`、`ParentElement`、`RenderOnce`

## Table 和 DataTable 的区别

| 特性 | Table | DataTable |
| --- | --- | --- |
| 虚拟滚动 | No | Yes |
| 列排序 | No | Yes |
| 列宽调整 | No | Yes |
| 列拖动 | No | Yes |
| 单元格选择 | No | Yes |
| 行选择 | No | Yes |
| 无限加载 | No | Yes |
| 键盘导航 | No | Yes |
| 状态管理 | Stateless | TableState |
| 适用场景 | 小型静态数据 | 大型交互式数据集 |

[DataTable]: ./data-table.md
