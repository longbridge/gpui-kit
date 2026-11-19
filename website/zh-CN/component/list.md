---
title: List
description: 支持分组、搜索、选择和无限滚动的灵活列表组件。
---

# List

List 是一个功能完整的列表组件，支持虚拟化展示、搜索、分组、头部和底部区域、选择状态以及无限滚动。它基于 delegate 模式构建，数据管理和项渲染都可以按业务场景自由扩展。

## 导入

```rust
use gpui_kit::component::list::{List, ListState, ListDelegate, ListItem, ListEvent, ListSeparatorItem};
use gpui_kit::component::IndexPath;
```

## 用法

### 基础列表

创建列表前，需要先为数据实现 `ListDelegate`：

```rust
struct MyListDelegate {
    items: Vec<String>,
    selected_index: Option<IndexPath>,
}

impl ListDelegate for MyListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.items.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        self.items.get(ix.row).map(|item| {
            ListItem::new(ix)
                .child(Label::new(item.clone()))
                .selected(Some(ix) == self.selected_index)
        })
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }
}

// 创建列表
let delegate = MyListDelegate {
    items: vec!["Item 1".into(), "Item 2".into(), "Item 3".into()],
    selected_index: None,
};

/// 创建列表状态。
let state = cx.new(|cx| ListState::new(delegate, window, cx));
```

渲染列表：

```rs
v_flex()
    .size_full()
    .child(List::new(&state).flex_1())
```

List 会撑满父容器，所以父容器需要有高度（例如 `size_full()` 或 `h(px(300.))`），否则列表高度为 0，什么都不显示。

### 分组列表

**注意：**`items_count` 为 `0` 的 section 会被自动隐藏，不会渲染 header 或 footer。

```rust
impl ListDelegate for MyListDelegate {
    type Item = ListItem;

    fn sections_count(&self, _cx: &App) -> usize {
        3 // section 数量
    }

    fn items_count(&self, section: usize, _cx: &App) -> usize {
        match section {
            0 => 5,
            1 => 3,
            2 => 7,
            _ => 0,
        }
    }

    fn render_section_header(
        &mut self,
        section: usize,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        let title = match section {
            0 => "Section 1",
            1 => "Section 2",
            2 => "Section 3",
            _ => return None,
        };

        Some(
            h_flex()
                .px_2()
                .py_1()
                .gap_2()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(Icon::new(IconName::Folder))
                .child(title)
        )
    }

    fn render_section_footer(
        &mut self,
        section: usize,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        Some(
            div()
                .px_2()
                .py_1()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!("End of section {}", section + 1))
        )
    }
}
```

### 带图标和操作的列表项

```rust
fn render_item(
    &mut self,
    ix: IndexPath,
    _window: &mut Window,
    cx: &mut Context<ListState<Self>>,
) -> Option<Self::Item> {
    self.items.get(ix.row).map(|item| {
        ListItem::new(ix)
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(Icon::new(IconName::File))
                    .child(Label::new(item.title.clone()))
            )
            .suffix(|_, _| {
                Button::new("action")
                    .ghost()
                    .small()
                    .icon(IconName::MoreHorizontal)
            })
            .selected(Some(ix) == self.selected_index)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.delegate_mut().select_item(ix, window, cx);
            }))
    })
}
```

### 可搜索列表

实现 `perform_search` 处理查询逻辑，并在 `ListState` 上启用 `searchable(true)`：

```rust
impl ListDelegate for MyListDelegate {
    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        // 按查询词过滤列表项
        self.filtered_items = self.all_items
            .iter()
            .filter(|item| item.to_lowercase().contains(&query.to_lowercase()))
            .cloned()
            .collect();

        Task::ready(())
    }
}

let state = cx.new(|cx| ListState::new(delegate, window, cx).searchable(true));
List::new(&state)
```

### 加载状态

```rust
impl ListDelegate for MyListDelegate {
    fn loading(&self, _cx: &App) -> bool {
        self.is_loading
    }

    fn render_loading(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        // 自定义加载视图
        v_flex()
            .justify_center()
            .items_center()
            .py_4()
            .child(Skeleton::new().h_4().w_full())
            .child(Skeleton::new().h_4().w_3_4())
    }
}
```

### 无限滚动

```rust
impl ListDelegate for MyListDelegate {
    fn has_more(&self, _cx: &App) -> bool {
        self.has_more_data
    }

    fn load_more_threshold(&self) -> usize {
        20 // 距底部还剩 20 项时触发
    }

    fn load_more(&mut self, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        if self.is_loading {
            return;
        }

        self.is_loading = true;
        cx.spawn_in(window, async move |view, window| {
            // 模拟 API 请求
            Timer::after(Duration::from_secs(1)).await;

            view.update_in(window, |view, _, cx| {
                // 追加列表项
                view.delegate_mut().load_more_items();
                view.delegate_mut().is_loading = false;
                cx.notify();
            });
        }).detach();
    }
}
```

### 列表事件

```rust
// 订阅列表事件
let _subscription = cx.subscribe(&state, |_, _, event: &ListEvent, _| {
    match event {
        ListEvent::Select(ix) => {
            println!("Item selected at: {:?}", ix);
        }
        ListEvent::Confirm(ix) => {
            println!("Item confirmed at: {:?}", ix);
        }
        ListEvent::Cancel => {
            println!("Selection cancelled");
        }
    }
});
```

### 列表项样式

```rust
// 带悬停效果的基础列表项
ListItem::new(ix)
    .child(Label::new("Basic Item"))
    .selected(is_selected)

// 带勾选图标的列表项
ListItem::new(ix)
    .child(Label::new("Checkable Item"))
    .check_icon(IconName::Check)
    .confirmed(is_confirmed)

// 禁用的列表项
ListItem::new(ix)
    .child(Label::new("Disabled Item"))
    .disabled(true)

// 分隔线
ListSeparatorItem::new()
    .child(
        div()
            .h_px()
            .w_full()
            .bg(cx.theme().border)
    )
```

### 拖拽重排

`ListItem` 实现了 GPUI 的 `InteractiveElement` 和 `StatefulInteractiveElement` trait，
因此 `on_drag`、`on_drop`、`drag_over`、`on_hover` 等原生交互 API 均可直接使用：

```rust
#[derive(Clone)]
struct DragItem {
    ix: IndexPath,
    name: SharedString,
}

impl Render for DragItem {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 拖拽时跟随光标的预览元素。
        div()
            .px_2()
            .py_1()
            .bg(cx.theme().accent)
            .text_color(cx.theme().accent_foreground)
            .rounded(cx.theme().radius)
            .child(self.name.clone())
    }
}

// 在 `ListDelegate` 的 `render_item` 中：
ListItem::new(ix)
    .child(Label::new(item.name.clone()))
    .on_drag(DragItem { ix, name: item.name.clone() }, |drag, _, _, cx| {
        cx.new(|_| drag.clone())
    })
    .drag_over::<DragItem>(|style, _, _, cx| style.bg(cx.theme().drop_target))
    .on_drop(cx.listener(move |this, drag: &DragItem, _, cx| {
        this.delegate_mut().move_item(drag.ix, ix);
        cx.notify();
    }))
```

### 自定义空状态

```rust
impl ListDelegate for MyListDelegate {
    fn render_empty(&mut self, _window: &mut Window, cx: &mut Context<ListState<Self>>) -> impl IntoElement {
        v_flex()
            .size_full()
            .justify_center()
            .items_center()
            .gap_2()
            .child(Icon::new(IconName::Search).size_16().text_color(cx.theme().muted_foreground))
            .child(
                Label::new("No items found")
                    .text_color(cx.theme().muted_foreground)
            )
            .child(
                Label::new("Try adjusting your search terms")
                    .text_sm()
                    .text_color(cx.theme().muted_foreground.opacity(0.7))
            )
    }
}
```

## 配置选项

### 列表配置

```rust
List::new(&state)
    .max_h(px(400.))                    // 设置最大高度
    .scrollbar_visible(false)           // 隐藏滚动条
    .paddings(Edges::all(px(8.)))       // 设置内边距
```

### 滚动控制

```rust
// 滚动到指定列表项
state.update(cx, |state, cx| {
    state.scroll_to_item(
        IndexPath::new(0).section(1),  // section 1 的第 0 行
        ScrollStrategy::Center,
        window,
        cx,
    );
});

// 滚动到选中项
state.update(cx, |state, cx| {
    state.scroll_to_selected_item(window, cx);
});

// 只设置选中项，不滚动
state.update(cx, |state, cx| {
    state.set_selected_index(Some(IndexPath::new(5)), window, cx);
});
```

## 示例

### 文件浏览器

```rust
struct FileBrowserDelegate {
    files: Vec<FileInfo>,
    selected: Option<IndexPath>,
}

#[derive(Clone)]
struct FileInfo {
    name: String,
    is_directory: bool,
    size: Option<u64>,
}

impl ListDelegate for FileBrowserDelegate {
    type Item = ListItem;

    fn render_item(&mut self, ix: IndexPath, window: &mut Window, cx: &mut Context<ListState<Self>>) -> Option<Self::Item> {
        self.files.get(ix.row).map(|file| {
            let icon = if file.is_directory {
                IconName::Folder
            } else {
                IconName::File
            };

            ListItem::new(ix)
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .w_full()
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child(Icon::new(icon))
                                .child(Label::new(file.name.clone()))
                        )
                        .when_some(file.size, |this, size| {
                            this.child(
                                Label::new(format_size(size))
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                            )
                        })
                )
                .selected(Some(ix) == self.selected)
        })
    }
}
```

### 按首字母分组的通讯录

```rust
struct ContactListDelegate {
    contacts_by_letter: BTreeMap<char, Vec<Contact>>,
    selected: Option<IndexPath>,
}

impl ListDelegate for ContactListDelegate {
    type Item = ListItem;

    fn sections_count(&self, _cx: &App) -> usize {
        self.contacts_by_letter.len()
    }

    fn render_section_header(&mut self, section: usize, _window: &mut Window, cx: &mut Context<ListState<Self>>) -> Option<impl IntoElement> {
        let letter = self.contacts_by_letter.keys().nth(section)?;

        Some(
            div()
                .px_3()
                .py_2()
                .bg(cx.theme().background)
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    Label::new(letter.to_string())
                        .text_lg()
                        .text_color(cx.theme().accent_foreground)
                        .font_weight(FontWeight::BOLD)
                )
        )
    }
}
```

## 滚动回弹

```rust
List::new(&state).scroll_bounce(true)
```

开启 `scroll_bounce(true)` 后，超出视口高度的内容在精细像素滚动到达顶部或底部时越过边缘并弹回，例如 macOS 上的触控板或 Magic Mouse。普通鼠标滚轮产生按行滚动，不触发回弹。新的精细滚动手势或反向精细滚动会接管回弹；普通滚轮输入会取消回弹并正常滚动。逻辑滚动位置始终限制在正常范围内，滚动条保持固定。内容未超出视口时不回弹。iOS、Android 和 macOS 默认开启（与系统自带的滚动视图一致），其他平台默认关闭；系统要求减弱动态效果时不产生位移。
