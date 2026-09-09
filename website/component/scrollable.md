---
title: Scrollable
description: Scrollable container with custom scrollbars, scroll tracking, and virtualization support.
---

# Scrollable

A comprehensive scrollable container component that provides custom scrollbars, scroll tracking, and virtualization capabilities. Supports both vertical and horizontal scrolling with customizable appearance and behavior.

## Import

```rust
use gpui_kit::component::{
    scroll::{ScrollableElement, ScrollbarAxis, ScrollbarMode},
    StyledExt as _,
};
```

## Usage

### Basic Scrollable Container

The simplest way to make any element scrollable is using the `overflow_scrollbar()` method from `ScrollableElement` trait.

This method is almost like the `overflow_scroll()` method, but it adds scrollbars.

- `overflow_scrollbar()` - Adds scrollbars for both axes as needed.
- `overflow_x_scrollbar()` - Adds horizontal scrollbar as needed.
- `overflow_y_scrollbar()` - Adds vertical scrollbar as needed.

```rust
use gpui_kit::{div, Axis};
use gpui_kit::component::ScrollableElement;

div()
    .id("scrollable-container")
    .size_full()
    .child("Your content here")
    .overflow_scrollbar()
```

### Vertical Scrolling

```rust
v_flex()
    .id("scrollable-container")
    .overflow_y_scrollbar()
    .gap_2()
    .p_4()
    .child("Scrollable Content")
    .children((0..100).map(|i| {
        div()
            .h(px(40.))
            .w_full()
            .bg(cx.theme().secondary)
            .child(format!("Item {}", i))
    }))
```

### Horizontal Scrolling

```rust
h_flex()
    .id("scrollable-container")
    .overflow_x_scrollbar()
    .gap_2()
    .p_4()
    .children((0..50).map(|i| {
        div()
            .min_w(px(120.))
            .h(px(80.))
            .bg(cx.theme().accent)
            .child(format!("Card {}", i))
    }))
```

### Both Directions

```rust
div()
    .id("scrollable-container")
    .size_full()
    .overflow_scrollbar()
    .child(
        div()
            .w(px(2000.))  // Wide content
            .h(px(2000.))  // Tall content
            .bg(cx.theme().background)
            .child("Large content area")
    )
```

## Custom Scrollbars

### Manual Scrollbar Creation

For more control, you can create scrollbars manually:

```rust
use gpui_kit::component::scroll::{ScrollableElement};

pub struct ScrollableView {
    scroll_handle: ScrollHandle,
}

impl Render for ScrollableView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .size_full()
            .child(
                div()
                    .id("content")
                    .track_scroll(&self.scroll_handle)
                    .overflow_scroll()
                    .size_full()
                    .child("Your scrollable content")
            )
            .vertical_scrollbar(&self.scroll_handle)
    }
}
```

## Virtualization

### VirtualList for Large Datasets

For rendering large lists efficiently, use `VirtualList`:

```rust
use gpui_kit::component::{VirtualList, VirtualListScrollHandle};

pub struct LargeListView {
    items: Vec<String>,
    scroll_handle: VirtualListScrollHandle,
}

impl Render for LargeListView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let item_count = self.items.len();

        VirtualList::new(
            self.scroll_handle.clone(),
            item_count,
            |ix, window, cx| {
                // Item sizes - can be different for each item
                size(px(300.), px(40.))
            },
            |ix, bounds, selected, window, cx| {
                // Render each item
                div()
                    .size(bounds.size)
                    .bg(if selected {
                        cx.theme().accent
                    } else {
                        cx.theme().background
                    })
                    .child(format!("Item {}: {}", ix, self.items[ix]))
                    .into_any_element()
            },
        )
    }
}
```

### Scrolling to Specific Items

```rust
impl LargeListView {
    fn scroll_to_item(&mut self, index: usize) {
        self.scroll_handle.scroll_to_item(index, ScrollStrategy::Top);
    }

    fn scroll_to_item_centered(&mut self, index: usize) {
        self.scroll_handle.scroll_to_item(index, ScrollStrategy::Center);
    }
}
```

### Variable Item Sizes

```rust
VirtualList::new(
    scroll_handle,
    items.len(),
    |ix, window, cx| {
        // Different heights based on content
        let height = if items[ix].len() > 50 {
            px(80.)  // Tall items for long content
        } else {
            px(40.)  // Normal height
        };
        size(px(300.), height)
    },
    |ix, bounds, selected, window, cx| {
        // Render logic
    },
)
```

## Theme Customization

### Scrollbar Appearance

Customize scrollbar appearance through theme configuration:

```rust
// In your theme JSON
{
    "scrollbar.background": "#ffffff20",
    "scrollbar.thumb.background": "#00000060",
    "scrollbar.thumb.hover.background": "#000000"
}
```

### Scrollbar Show Modes

Control when scrollbars are visible:

```rust
use gpui_kit::component::{Theme, scroll::ScrollbarMode};

// In theme initialization
Theme::set_scrollbar_mode(ScrollbarMode::Scrolling, cx); // Only while scrolling
Theme::set_scrollbar_mode(ScrollbarMode::Hover, cx);     // On hover
Theme::set_scrollbar_mode(ScrollbarMode::Always, cx);    // Always visible
```

### System Integration

Sync scrollbar behavior with system preferences:

```rust
// Automatically sync with system settings
Theme::sync_scrollbar_appearance(cx);
```

## Examples

### File Browser with Scrolling

```rust
pub struct FileBrowser {
    files: Vec<String>,
}

impl Render for FileBrowser {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .border_1()
            .border_color(cx.theme().border)
            .size_full()
            .child(
                v_flex()
                    .gap_1()
                    .p_2()
                    .overflow_y_scrollbar()
                    .children(self.files.iter().map(|file| {
                        div()
                            .h(px(32.))
                            .w_full()
                            .px_2()
                            .flex()
                            .items_center()
                            .hover(|style| style.bg(cx.theme().secondary_hover))
                            .child(file.clone())
                    }))
            )
    }
}
```

### Chat Messages with Auto-scroll

```rust
pub struct ChatView {
    messages: Vec<String>,
    scroll_handle: ScrollHandle,
    should_auto_scroll: bool,
}

impl ChatView {
    fn add_message(&mut self, message: String) {
        self.messages.push(message);

        if self.should_auto_scroll {
            // Scroll to bottom for new messages
            let max_offset = self.scroll_handle.max_offset();
            self.scroll_handle.set_offset(point(px(0.), max_offset.y));
        }
    }
}
```

### Data Table with Virtual Scrolling

```rust
pub struct DataTable {
    data: Vec<Vec<String>>,
    scroll_handle: VirtualListScrollHandle,
}

impl Render for DataTable {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        VirtualList::new(
            self.scroll_handle.clone(),
            self.data.len(),
            |_ix, _window, _cx| size(px(800.), px(32.)), // Fixed row height
            |ix, bounds, _selected, _window, cx| {
                h_flex()
                    .size(bounds.size)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .children(self.data[ix].iter().map(|cell| {
                        div()
                            .flex_1()
                            .px_2()
                            .flex()
                            .items_center()
                            .child(cell.clone())
                    }))
                    .into_any_element()
            },
        )
    }
}
```
