---
title: List
description: A flexible list component that displays a series of items with support for sections, search, selection, and infinite scrolling.
---

# List

A powerful List component that provides a virtualized, searchable list interface with support for sections, headers, footers, selection states, and infinite scrolling. The component is built on a delegate pattern that allows for flexible data management and custom item rendering.

## Import

```rust
use gpui_kit::component::list::{List, ListState, ListDelegate, ListItem, ListEvent, ListSeparatorItem};
use gpui_kit::component::IndexPath;
```

## Usage

### Basic List

To create a list, you need to implement the `ListDelegate` trait for your data:

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

// Create the list
let delegate = MyListDelegate {
    items: vec!["Item 1".into(), "Item 2".into(), "Item 3".into()],
    selected_index: None,
};

/// Create a list state.
let state = cx.new(|cx| ListState::new(delegate, window, cx));
```

Now use [List] to render list:

```rs
div().child(List::new(&state))
```

### List with Sections

**Note:** Sections with `items_count` of 0 will be automatically hidden (no header or footer will be rendered for empty sections).

```rust
impl ListDelegate for MyListDelegate {
    type Item = ListItem;

    fn sections_count(&self, _cx: &App) -> usize {
        3 // Number of sections
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

### List Items with Icons and Actions

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

### List with Search

The list automatically includes a search input by default. Implement `perform_search` to handle queries:

And you should use `searchable(true)` when creating the `ListState` to show search input.

```rust
impl ListDelegate for MyListDelegate {
    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        // Filter items based on query
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

### List with Loading State

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
        // Custom loading view
        v_flex()
            .justify_center()
            .items_center()
            .py_4()
            .child(Skeleton::new().h_4().w_full())
            .child(Skeleton::new().h_4().w_3_4())
    }
}
```

### Infinite Scrolling

```rust
impl ListDelegate for MyListDelegate {
    fn has_more(&self, _cx: &App) -> bool {
        self.has_more_data
    }

    fn load_more_threshold(&self) -> usize {
        20 // Trigger when 20 items from bottom
    }

    fn load_more(&mut self, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        if self.is_loading {
            return;
        }

        self.is_loading = true;
        cx.spawn_in(window, async move |view, window| {
            // Simulate API call
            Timer::after(Duration::from_secs(1)).await;

            view.update_in(window, |view, _, cx| {
                // Add more items
                view.delegate_mut().load_more_items();
                view.delegate_mut().is_loading = false;
                cx.notify();
            });
        }).detach();
    }
}
```

### List Events

```rust
// Subscribe to list events
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

### Different Item Styles

```rust
// Basic item with hover effect
ListItem::new(ix)
    .child(Label::new("Basic Item"))
    .selected(is_selected)

// Item with check icon
ListItem::new(ix)
    .child(Label::new("Checkable Item"))
    .check_icon(IconName::Check)
    .confirmed(is_confirmed)

// Disabled item
ListItem::new(ix)
    .child(Label::new("Disabled Item"))
    .disabled(true)

// Separator item
ListSeparatorItem::new()
    .child(
        div()
            .h_px()
            .w_full()
            .bg(cx.theme().border)
    )
```

### Drag and Drop Reordering

`ListItem` implements GPUI's `InteractiveElement` and `StatefulInteractiveElement`
traits, so all native interaction APIs such as `on_drag`, `on_drop`, `drag_over`
and `on_hover` are directly available:

```rust
#[derive(Clone)]
struct DragItem {
    ix: IndexPath,
    name: SharedString,
}

impl Render for DragItem {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The preview element that follows the cursor while dragging.
        div()
            .px_2()
            .py_1()
            .bg(cx.theme().accent)
            .text_color(cx.theme().accent_foreground)
            .rounded(cx.theme().radius)
            .child(self.name.clone())
    }
}

// In `render_item` of your `ListDelegate`:
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

### Custom Empty State

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

## Configuration Options

### List Configuration

```rust
List::new(&state)
    .max_h(px(400.))                    // Set maximum height
    .scrollbar_visible(false)           // Hide scrollbar
    .paddings(Edges::all(px(8.)))       // Set internal padding
```

### Scrolling Control

```rust
// Scroll to specific item
state.update(cx, |state, cx| {
    state.scroll_to_item(
        IndexPath::new(0).section(1),  // Row 0 of section 1
        ScrollStrategy::Center,
        window,
        cx,
    );
});

// Scroll to selected item
state.update(cx, |state, cx| {
    state.scroll_to_selected_item(window, cx);
});

// Set selected index without scrolling
state.update(cx, |state, cx| {
    state.set_selected_index(Some(IndexPath::new(5)), window, cx);
});
```

## Examples

### File Browser List

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

### Contact List with Sections

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
