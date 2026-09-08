---
title: Pagination
description: 提供页码、上一页和下一页导航的分页组件。
---

# Pagination

[Pagination] 组件用于在多页内容之间切换，支持显示页码、上一页和下一页操作，适合表格、列表和搜索结果等需要分页浏览的场景。

## 导入

```rust
use gpui_kit::component::pagination::Pagination;
```

## 用法

### 基础分页

```rust
Pagination::new("my-pagination")
    .current_page(5)
    .total_pages(10)
    .on_click(|page, _, cx| {
        println!("Navigated to page: {}", page);
    })
```

### 自定义可见页数

默认最多显示 5 个页码按钮，可以通过 `visible_pages()` 调整：

```rust
Pagination::new("my-pagination")
    .current_page(1)
    .total_pages(50)
    .visible_pages(10)
    .on_click(|page, _, cx| {
        // 处理页码切换
    })
```

### 紧凑模式

紧凑模式只显示上一页和下一页按钮，不显示具体页码：

```rust
Pagination::new("my-pagination")
    .compact()
    .current_page(3)
    .total_pages(10)
    .on_click(|page, _, cx| {
        // 处理页码切换
    })
```

### 不同尺寸

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
    .total_pages(10)

Pagination::new("my-pagination")
    .large()
    .current_page(1)
    .total_pages(10)
```

### 禁用状态

```rust
Pagination::new("my-pagination")
    .current_page(4)
    .total_pages(10)
    .disabled(true)
    .on_click(|_, _, _| {})
```

### 处理页码变化

`on_click` 会在用户点击页码、上一页或下一页时返回新的页码：

```rust
Pagination::new("my-pagination")
    .current_page(current_page)
    .total_pages(total_pages)
    .on_click(|page, _, cx| {
        // 用新的页码更新状态
        // 页码从 1 开始
    })
```

## API 参考

### 尺寸

实现了 [Sizable] trait：

- `xsmall()`：超小尺寸
- `small()`：小尺寸
- `medium()`：中尺寸，默认值
- `large()`：大尺寸
- `with_size(size)`：设置自定义尺寸

### 方法

- `current_page(page: usize)`：设置当前页，页码从 1 开始，超出范围时会自动限制到 `1..=total_pages`
- `total_pages(pages: usize)`：设置总页数
- `visible_pages(max: usize)`：设置最多显示多少个页码按钮，默认 `5`
- `compact()`：启用紧凑模式，仅显示前后翻页按钮
- `disabled(bool)`：设置禁用状态
- `on_click(handler)`：设置页码切换回调

## 示例

### 结合状态管理

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

### 大数据集分页

```rust
Pagination::new("large-pagination")
    .current_page(25)
    .total_pages(100)
    .visible_pages(10)
    .on_click(|page, _, cx| {
        // 加载新页的数据
    })
```

[Pagination]: https://docs.rs/gpui-component/latest/gpui_component/pagination/struct.Pagination.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html
