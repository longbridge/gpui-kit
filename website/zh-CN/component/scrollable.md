---
title: Scrollable
description: 支持自定义滚动条、滚动跟踪与虚拟列表的可滚动容器。
---

# Scrollable

Scrollable 是一个功能完整的可滚动容器组件，支持自定义滚动条、滚动位置跟踪以及虚拟化渲染。它同时支持纵向和横向滚动，并可按需定制显示行为。

## 导入

```rust
use gpui_kit::component::{
    scroll::{ScrollableElement, ScrollbarAxis, ScrollbarMode},
    StyledExt as _,
};
```

## 用法

### 基础可滚动容器

让任意元素具备滚动能力的最简单方式，是使用 `ScrollableElement` trait 提供的 `overflow_scrollbar()`：

- `overflow_scrollbar()`：按需为两个方向都添加滚动条。
- `overflow_x_scrollbar()`：按需添加横向滚动条。
- `overflow_y_scrollbar()`：按需添加纵向滚动条。

```rust
use gpui_kit::{div, Axis};
use gpui_kit::component::ScrollableElement;

div()
    .id("scrollable-container")
    .size_full()
    .child("Your content here")
    .overflow_scrollbar()
```

### 纵向滚动

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

### 横向滚动

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

### 双向滚动

```rust
div()
    .id("scrollable-container")
    .size_full()
    .overflow_scrollbar()
    .child(
        div()
            .w(px(2000.))
            .h(px(2000.))
            .bg(cx.theme().background)
            .child("Large content area")
    )
```

## 自定义滚动条

### 手动创建滚动条

如果你需要更高的控制粒度，可以手动创建滚动条：

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

## 虚拟化

### 使用 VirtualList 处理大数据集

渲染超长列表时，推荐使用 `VirtualList`：

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
                size(px(300.), px(40.))
            },
            |ix, bounds, selected, window, cx| {
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

### 滚动到指定项

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

### 可变高度项

```rust
VirtualList::new(
    scroll_handle,
    items.len(),
    |ix, window, cx| {
        let height = if items[ix].len() > 50 {
            px(80.)
        } else {
            px(40.)
        };
        size(px(300.), height)
    },
    |ix, bounds, selected, window, cx| {
        // Render logic
    },
)
```

## 主题定制

### 滚动条外观

可以通过主题配置自定义滚动条样式：

```rust
// In your theme JSON
{
    "scrollbar.background": "#ffffff20",
    "scrollbar.thumb.background": "#00000060",
    "scrollbar.thumb.hover.background": "#000000"
}
```

### 滚动条显示模式

控制滚动条何时显示：

```rust
use gpui_kit::component::{Theme, scroll::ScrollbarMode};

Theme::set_scrollbar_mode(ScrollbarMode::Scrolling, cx);
Theme::set_scrollbar_mode(ScrollbarMode::Hover, cx);
Theme::set_scrollbar_mode(ScrollbarMode::Always, cx);
```

### 跟随系统设置

```rust
Theme::sync_scrollbar_appearance(cx);
```

## 示例

### 带滚动的文件浏览器

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

### 自动滚动到底部的聊天列表

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
            let max_offset = self.scroll_handle.max_offset();
            self.scroll_handle.set_offset(point(px(0.), max_offset.y));
        }
    }
}
```

### 带虚拟滚动的数据表格

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
            |_ix, _window, _cx| size(px(800.), px(32.)),
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

## 滚动回弹

```rust
div()
    .child(content)
    .overflow_y_scrollbar()
    .scroll_bounce(true)
```

开启 `scroll_bounce(true)` 后，超出视口高度的内容在精细像素滚动到达顶部或底部时越过边缘并弹回，例如 macOS 上的触控板或 Magic Mouse。普通鼠标滚轮产生按行滚动，不触发回弹。新的精细滚动手势或反向精细滚动会接管回弹；普通滚轮输入会取消回弹并正常滚动。逻辑滚动位置始终限制在正常范围内，滚动条保持固定。内容未超出视口时不回弹。iOS、Android 和 macOS 默认开启（与系统自带的滚动视图一致），其他平台默认关闭；系统要求减弱动态效果时不产生位移。

嵌套的滚动区域与系统行为一致：指针下最内层的滚动区域接管整个手势，到达自身两端时自己回弹，不会把剩余的滚动交给外层。文本输入框、List、Table 和 Tree 遵循同一规则。

自定义视口可以用 `gpui_kit::base::ScrollBounce::new(id, &handle, child)` 包裹。包裹层负责纵向滚轮滚动，默认值跟随平台；`.enabled(false)` 保留滚动、去掉回弹。固定控件应放在包裹层外。`.scroll_wheel(false)` 让子元素自己滚动，包裹层只对未用完的位移做回弹，适用于触摸输入。仅横向滚动的 Scrollable 容器不回弹。
