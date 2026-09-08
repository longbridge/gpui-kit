---
title: VirtualList
description: High-performance virtualized list component for rendering large datasets with variable item sizes.
---

# VirtualList

VirtualList is a high-performance component designed for efficiently rendering large datasets by only rendering visible items. Unlike uniform lists, VirtualList supports variable item sizes, making it perfect for complex layouts like tables with different row heights or dynamic content.

## Import

```rust
use gpui_kit::component::{
    v_virtual_list, h_virtual_list, VirtualListScrollHandle,
    scroll::{Scrollbar, ScrollbarState, ScrollbarAxis},
};
use std::rc::Rc;
use gpui_kit::{px, size, ScrollStrategy, Size, Pixels};
```

## Usage

### Basic Vertical Virtual List

```rust
use std::rc::Rc;
use gpui_kit::{px, size, Size, Pixels};

pub struct ListViewExample {
    items: Vec<String>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    scroll_handle: VirtualListScrollHandle,
}

impl ListViewExample {
    fn new(cx: &mut Context<Self>) -> Self {
        let items = (0..5000).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
        let item_sizes = Rc::new(items.iter().map(|_| size(px(200.), px(30.))).collect());

        Self {
            items,
            item_sizes,
            scroll_handle: VirtualListScrollHandle::new(),
        }
    }
}

impl Render for ListViewExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
    }
}
```

### Horizontal Virtual List

```rust
h_virtual_list(
    cx.entity().clone(),
    "horizontal-list",
    item_sizes.clone(),
    |view, visible_range, _, cx| {
        visible_range
            .map(|ix| {
                div()
                    .w(px(120.))  // Width is used for horizontal lists
                    .h_full()
                    .bg(cx.theme().accent)
                    .child(format!("Card {}", ix))
            })
            .collect()
    },
)
.track_scroll(&scroll_handle)
```

### Variable Item Sizes

VirtualList excels at handling items with different sizes:

```rust
let item_sizes = Rc::new(
    (0..1000)
        .map(|i| {
            // Different heights based on index
            let height = if i % 5 == 0 {
                px(60.)  // Header items are taller
            } else if i % 3 == 0 {
                px(45.)  // Some items are medium
            } else {
                px(30.)  // Regular items
            };
            size(px(300.), height)
        })
        .collect::<Vec<_>>()
);

v_virtual_list(
    cx.entity().clone(),
    "variable-list",
    item_sizes.clone(),
    |view, visible_range, _, cx| {
        visible_range
            .map(|ix| {
                let content = if ix % 5 == 0 {
                    format!("Header {}", ix / 5)
                } else {
                    format!("Item {}", ix)
                };

                let bg_color = if ix % 5 == 0 {
                    cx.theme().accent
                } else {
                    cx.theme().secondary
                };

                div()
                    .w_full()
                    .h(item_sizes[ix].height)
                    .bg(bg_color)
                    .flex()
                    .items_center()
                    .px_4()
                    .child(content)
            })
            .collect()
    },
)
```

### Table-like Layout with Multiple Columns

VirtualList can render complex layouts like tables:

```rust
v_virtual_list(
    cx.entity().clone(),
    "table-list",
    item_sizes.clone(),
    |view, visible_range, _, cx| {
        visible_range
            .map(|row_ix| {
                h_flex()
                    .w_full()
                    .h(px(40.))
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .children(
                        // Multiple columns per row
                        (0..5).map(|col_ix| {
                            div()
                                .flex_1()
                                .h_full()
                                .px_3()
                                .flex()
                                .items_center()
                                .child(format!("R{}C{}", row_ix, col_ix))
                        })
                    )
            })
            .collect()
    },
)
```

## Scroll Handling

### Basic Scroll Control

```rust
pub struct ScrollableList {
    scroll_handle: VirtualListScrollHandle,
    scroll_state: ScrollbarState,
}

impl Render for ScrollableList {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .size_full()
            .child(
                v_virtual_list(/* ... */)
                    .track_scroll(&self.scroll_handle)
                    .p_4()
                    .border_1()
                    .border_color(cx.theme().border)
            )
            .child(
                // Add scrollbars
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .child(
                        Scrollbar::both(&self.scroll_state, &self.scroll_handle)
                            .axis(ScrollbarAxis::Vertical)
                    )
            )
    }
}
```

### Programmatic Scrolling

```rust
impl ScrollableList {
    // Scroll to specific item
    fn scroll_to_item(&self, index: usize) {
        self.scroll_handle.scroll_to_item(index, ScrollStrategy::Top);
    }

    // Center item in view
    fn center_item(&self, index: usize) {
        self.scroll_handle.scroll_to_item(index, ScrollStrategy::Center);
    }

    // Scroll to bottom
    fn scroll_to_bottom(&self) {
        self.scroll_handle.scroll_to_bottom();
    }

    // Get current scroll position
    fn get_scroll_offset(&self) -> Point<Pixels> {
        self.scroll_handle.offset()
    }

    // Set scroll position manually
    fn set_scroll_position(&self, offset: Point<Pixels>) {
        self.scroll_handle.set_offset(offset);
    }
}
```

### Both Axis Scrolling

For content that scrolls in both directions:

```rust
v_virtual_list(
    cx.entity().clone(),
    "both-axis",
    item_sizes.clone(),
    |view, visible_range, _, cx| {
        visible_range
            .map(|ix| {
                // Wide content that requires horizontal scrolling
                h_flex()
                    .gap_2()
                    .children((0..20).map(|col| {
                        div()
                            .min_w(px(100.))
                            .h(px(30.))
                            .bg(cx.theme().secondary)
                            .child(format!("R{}C{}", ix, col))
                    }))
            })
            .collect()
    },
)
.track_scroll(&scroll_handle)
.child(
    Scrollbar::both(&scroll_state, &scroll_handle)
        .axis(ScrollbarAxis::Both)
)
```

## Performance Optimization

### Efficient Item Rendering

Only visible items are rendered, making VirtualList highly performant:

```rust
// The render function is only called for visible items
v_virtual_list(
    cx.entity().clone(),
    "efficient-list",
    item_sizes.clone(),
    |view, visible_range, _, cx| {
        // visible_range contains only the items currently visible
        // This typically contains 10-20 items, not all 10,000
        println!("Rendering {} items out of {}",
                visible_range.len(),
                view.total_items);

        visible_range
            .map(|ix| {
                // Complex rendering logic here
                // Only executed for visible items
                expensive_item_renderer(ix, cx)
            })
            .collect()
    },
)
```

### Memory Management

VirtualList automatically manages memory by:

- Only rendering visible items
- Reusing rendered elements when scrolling
- Calculating precise visible ranges

```rust
// Large dataset - only visible items use memory
let large_dataset = (0..1_000_000).map(|i| format!("Item {}", i)).collect();

// Memory usage remains constant regardless of dataset size
v_virtual_list(/* render only visible items */)
```

### Variable Heights with Caching

For dynamic content with calculated heights:

```rust
struct DynamicItem {
    content: String,
    calculated_height: Option<Pixels>,
}

impl MyView {
    fn calculate_item_size(&mut self, ix: usize) -> Size<Pixels> {
        if let Some(height) = self.items[ix].calculated_height {
            return size(px(300.), height);
        }

        // Calculate height based on content
        let content_lines = self.items[ix].content.lines().count();
        let height = px(20. + content_lines as f32 * 16.);

        // Cache the calculated height
        self.items[ix].calculated_height = Some(height);

        size(px(300.), height)
    }
}
```

## Examples

### File Explorer with Virtual Scrolling

```rust
pub struct FileExplorer {
    files: Vec<FileEntry>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    scroll_handle: VirtualListScrollHandle,
    selected_index: Option<usize>,
}

impl FileExplorer {
    fn calculate_item_heights(&mut self) {
        let sizes = self.files.iter().map(|file| {
            // Different heights for different file types
            let height = match file.file_type {
                FileType::Directory => px(40.),
                FileType::Image => px(60.),  // Larger for thumbnails
                FileType::Document => px(35.),
                _ => px(30.),
            };
            size(px(400.), height)
        }).collect();

        self.item_sizes = Rc::new(sizes);
    }
}

impl Render for FileExplorer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_virtual_list(
            cx.entity().clone(),
            "file-list",
            self.item_sizes.clone(),
            |view, visible_range, _, cx| {
                visible_range
                    .map(|ix| {
                        let file = &view.files[ix];
                        let is_selected = view.selected_index == Some(ix);

                        div()
                            .w_full()
                            .h(view.item_sizes[ix].height)
                            .px_3()
                            .py_1()
                            .flex()
                            .items_center()
                            .gap_2()
                            .bg(if is_selected {
                                cx.theme().accent
                            } else {
                                Color::transparent()
                            })
                            .hover(|style| style.bg(cx.theme().secondary_hover))
                            .child(file_icon(&file.file_type))
                            .child(file.name.clone())
                            .child(
                                div()
                                    .flex_1()
                                    .text_right()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format_file_size(file.size))
                            )
                            .on_click(cx.listener(move |view, _, _, cx| {
                                view.selected_index = Some(ix);
                                cx.notify();
                            }))
                    })
                    .collect()
            },
        )
        .track_scroll(&self.scroll_handle)
    }
}
```

### Chat Messages with Auto-scroll

```rust
pub struct ChatWindow {
    messages: Vec<ChatMessage>,
    scroll_handle: VirtualListScrollHandle,
    auto_scroll: bool,
}

impl ChatWindow {
    fn add_message(&mut self, message: ChatMessage, cx: &mut Context<Self>) {
        self.messages.push(message);

        // Recalculate item sizes
        self.update_item_sizes();

        if self.auto_scroll {
            // Scroll to bottom for new messages
            self.scroll_handle.scroll_to_bottom();
        }

        cx.notify();
    }

    fn update_item_sizes(&mut self) {
        let sizes = self.messages.iter().map(|msg| {
            // Calculate height based on message content
            let lines = msg.content.lines().count().max(1);
            let height = px(40. + (lines.saturating_sub(1)) as f32 * 16.);
            size(px(350.), height)
        }).collect();

        self.item_sizes = Rc::new(sizes);
    }
}

impl Render for ChatWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                v_virtual_list(
                    cx.entity().clone(),
                    "chat-messages",
                    self.item_sizes.clone(),
                    |view, visible_range, _, cx| {
                        visible_range
                            .map(|ix| {
                                let msg = &view.messages[ix];

                                div()
                                    .w_full()
                                    .px_4()
                                    .py_2()
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(
                                                h_flex()
                                                    .justify_between()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .font_weight(FontWeight::SEMIBOLD)
                                                            .child(msg.author.clone())
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(format_timestamp(msg.timestamp))
                                                    )
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .child(msg.content.clone())
                                            )
                                    )
                            })
                            .collect()
                    },
                )
                .track_scroll(&self.scroll_handle)
                .flex_1()
            )
            .child(
                // Chat input at bottom
                div()
                    .w_full()
                    .h(px(60.))
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child("Chat input here...")
            )
    }
}
```

### Data Grid with Fixed Headers

```rust
pub struct DataGrid {
    headers: Vec<String>,
    data: Vec<Vec<String>>,
    column_widths: Vec<Pixels>,
    scroll_handle: VirtualListScrollHandle,
}

impl Render for DataGrid {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                // Fixed header
                h_flex()
                    .w_full()
                    .h(px(40.))
                    .bg(cx.theme().secondary)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .children(
                        self.headers.iter().zip(&self.column_widths).map(|(header, &width)| {
                            div()
                                .w(width)
                                .h_full()
                                .px_3()
                                .flex()
                                .items_center()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(header.clone())
                        })
                    )
            )
            .child(
                // Virtual list for data rows
                v_virtual_list(
                    cx.entity().clone(),
                    "data-rows",
                    Rc::new(vec![size(px(800.), px(32.)); self.data.len()]),
                    |view, visible_range, _, cx| {
                        visible_range
                            .map(|row_ix| {
                                h_flex()
                                    .w_full()
                                    .h(px(32.))
                                    .border_b_1()
                                    .border_color(cx.theme().border.opacity(0.5))
                                    .children(
                                        view.data[row_ix].iter().zip(&view.column_widths).map(|(cell, &width)| {
                                            div()
                                                .w(width)
                                                .h_full()
                                                .px_3()
                                                .flex()
                                                .items_center()
                                                .child(cell.clone())
                                        })
                                    )
                            })
                            .collect()
                    },
                )
                .track_scroll(&self.scroll_handle)
                .flex_1()
            )
    }
}
```

## Best Practices

1. **Item Sizing**: Pre-calculate item sizes when possible for best performance
2. **Memory Management**: Use VirtualList for any list with >50 items
3. **Scroll Performance**: Avoid heavy computations in render functions
4. **State Management**: Keep item state separate from rendering logic
5. **Error Handling**: Handle edge cases like empty lists gracefully
6. **Testing**: Test with various data sizes and scroll positions

## Performance Tips

1. **Pre-calculate Sizes**: Calculate item sizes upfront rather than during render
2. **Minimize Re-renders**: Use stable item keys and avoid recreating render functions
3. **Batch Updates**: Group multiple data changes together
4. **Efficient Rendering**: Keep item render functions lightweight
5. **Memory Monitoring**: Monitor memory usage with very large datasets
