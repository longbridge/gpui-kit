---
title: VirtualList
description: 用于大数据集渲染的高性能虚拟列表组件，支持可变尺寸项。
---

# VirtualList

VirtualList 是一个面向大规模数据集的高性能列表组件。它只渲染当前可见区域的项目，因此非常适合长列表、动态高度内容以及类似表格的复杂布局。

与普通均匀列表不同，VirtualList 支持每一项拥有不同尺寸。

## 导入

```rust
use gpui_kit::component::{
    v_virtual_list, h_virtual_list, VirtualListScrollHandle,
    scroll::{Scrollbar, ScrollbarState, ScrollbarAxis},
};
use std::rc::Rc;
use gpui_kit::{px, size, ScrollStrategy, Size, Pixels};
```

## 用法

### 基础纵向列表

```rust
pub struct ListViewExample {
    items: Vec<String>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    scroll_handle: VirtualListScrollHandle,
}
```

```rust
v_virtual_list(
    cx.entity().clone(),
    "my-list",
    self.item_sizes.clone(),
    |view, visible_range, _, cx| {
        visible_range
            .map(|ix| {
                div()
                    .h(px(30.))
                    .w_full()
                    .bg(cx.theme().secondary)
                    .child(format!("Item {}", ix))
            })
            .collect()
    },
)
.track_scroll(&self.scroll_handle)
```

### 横向列表

```rust
h_virtual_list(
    cx.entity().clone(),
    "horizontal-list",
    item_sizes.clone(),
    |view, visible_range, _, cx| {
        visible_range
            .map(|ix| {
                div()
                    .w(px(120.))
                    .h_full()
                    .bg(cx.theme().accent)
                    .child(format!("Card {}", ix))
            })
            .collect()
    },
)
```

### 可变尺寸项

```rust
let item_sizes = Rc::new(
    (0..1000)
        .map(|i| {
            let height = if i % 5 == 0 {
                px(60.)
            } else if i % 3 == 0 {
                px(45.)
            } else {
                px(30.)
            };
            size(px(300.), height)
        })
        .collect::<Vec<_>>()
);
```

## 滚动控制

### 基础滚动

```rust
pub struct ScrollableList {
    scroll_handle: VirtualListScrollHandle,
    scroll_state: ScrollbarState,
}
```

### 编程式滚动

```rust
impl ScrollableList {
    fn scroll_to_item(&self, index: usize) {
        self.scroll_handle.scroll_to_item(index, ScrollStrategy::Top);
    }

    fn center_item(&self, index: usize) {
        self.scroll_handle.scroll_to_item(index, ScrollStrategy::Center);
    }

    fn scroll_to_bottom(&self) {
        self.scroll_handle.scroll_to_bottom();
    }
}
```

### 双轴滚动

```rust
Scrollbar::both(&scroll_state, &scroll_handle)
    .axis(ScrollbarAxis::Both)
```

## 性能说明

VirtualList 的核心优势在于：

- 只渲染可见范围内的项
- 滚动时复用已渲染元素
- 数据量很大时内存占用仍然稳定

因此只要列表项超过几十个，尤其是达到上百上千个时，就值得优先考虑 VirtualList。

## 示例

### 文件浏览器

```rust
pub struct FileExplorer {
    files: Vec<FileEntry>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    scroll_handle: VirtualListScrollHandle,
    selected_index: Option<usize>,
}
```

### 聊天窗口

```rust
pub struct ChatWindow {
    messages: Vec<ChatMessage>,
    scroll_handle: VirtualListScrollHandle,
    auto_scroll: bool,
}
```

### 固定表头的数据网格

```rust
pub struct DataGrid {
    headers: Vec<String>,
    data: Vec<Vec<String>>,
    column_widths: Vec<Pixels>,
    scroll_handle: VirtualListScrollHandle,
}
```

## 最佳实践

1. 尽量预先计算 item 尺寸
2. 渲染函数里避免重计算
3. 列表项数量超过 50 时优先考虑 VirtualList
4. 将项状态与渲染逻辑拆开管理
5. 测试不同数据量和滚动位置下的表现
