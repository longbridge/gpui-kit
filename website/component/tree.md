---
title: Tree
description: A hierarchical tree view component for displaying and navigating tree-structured data.
---

# Tree

A versatile tree component for displaying hierarchical data with expand/collapse functionality, keyboard navigation, and custom item rendering. Perfect for file explorers, navigation menus, or any nested data structure.

## Import

```rust
use gpui_kit::component::tree::{tree, TreeState, TreeItem, TreeEntry};
```

## Usage

### Basic Tree

```rust
// Create tree state
let tree_state = cx.new(|cx| {
    TreeState::new(cx).items(vec![
        TreeItem::new("src", "src")
            .expanded(true)
            .child(TreeItem::new("src/lib.rs", "lib.rs"))
            .child(TreeItem::new("src/main.rs", "main.rs")),
        TreeItem::new("Cargo.toml", "Cargo.toml"),
        TreeItem::new("README.md", "README.md"),
    ])
});

// Render tree
tree(&tree_state, |ix, entry, selected, window, cx| {
    ListItem::new(ix)
        .child(
            h_flex()
                .gap_2()
                .child(entry.item().label.clone())
        )
})
```

### File Tree with Icons

```rust
use gpui_kit::component::{ListItem, IconName, h_flex};

tree(&tree_state, |ix, entry, selected, window, cx| {
    let item = entry.item();
    let icon = if !entry.is_folder() {
        IconName::File
    } else if entry.is_expanded() {
        IconName::FolderOpen
    } else {
        IconName::Folder
    };

    ListItem::new(ix)
        .selected(selected)
        .pl(px(16.) * entry.depth() + px(12.)) // Indent based on depth
        .child(
            h_flex()
                .gap_2()
                .child(icon)
                .child(item.label.clone())
        )
        .on_click(cx.listener(move |_, _, _, _| {
            // Handle item click
        }))
})
```

### Dynamic Tree Loading

```rust
impl MyView {
    fn load_files(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let tree_state = self.tree_state.clone();
        cx.spawn(async move |cx| {
            let items = build_file_items(&path).await;
            tree_state.update(cx, |state, cx| {
                state.set_items(items, cx);
            })
        }).detach();
    }
}

fn build_file_items(path: &Path) -> Vec<TreeItem> {
    let mut items = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            if path.is_dir() {
                let children = build_file_items(&path);
                items.push(TreeItem::new(path.to_string_lossy(), name)
                    .children(children));
            } else {
                items.push(TreeItem::new(path.to_string_lossy(), name));
            }
        }
    }
    items
}
```

### Tree with Selection Handling

```rust
struct MyTreeView {
    tree_state: Entity<TreeState>,
    selected_item: Option<TreeItem>,
}

impl MyTreeView {
    fn handle_selection(&mut self, item: TreeItem, cx: &mut Context<Self>) {
        self.selected_item = Some(item.clone());
        println!("Selected: {} ({})", item.label, item.id);
        cx.notify();
    }
}

// In render method
tree(&self.tree_state, {
    let view = cx.entity();
    move |ix, entry, selected, window, cx| {
        view.update(cx, |this, cx| {
            ListItem::new(ix)
                .selected(selected)
                .child(entry.item().label.clone())
                .on_click(cx.listener({
                    let item = entry.item().clone();
                    move |this, _, _, cx| {
                        this.handle_selection(item.clone(), cx);
                    }
                }))
        })
    }
})
```

### Disabled Items

```rust
TreeItem::new("protected", "Protected Folder")
    .disabled(true)
    .child(TreeItem::new("secret.txt", "secret.txt"))
```

### Programmatic Tree Control

```rust
// Get current selection
if let Some(entry) = tree_state.read(cx).selected_entry() {
    println!("Current selection: {}", entry.item().label);
}

// Set selection programmatically (by selected_index)
tree_state.update(cx, |state, cx| {
    state.set_selected_index(Some(2), cx); // Select third item
});

// Set selection programmatically (by tree item)
tree_state.update(cx, |state, cx| {
    state.set_selected_item(Some(item), cx); // Select third item
});

// Scroll to specific item
tree_state.update(cx, |state, _| {
    state.scroll_to_item(5, gpui_kit::ScrollStrategy::Center);
});

// Clear selection (by selected_index)
tree_state.update(cx, |state, cx| {
    state.set_selected_index(None, cx);
});

// Clear selection (by tree item)
tree_state.update(cx, |state, cx| {
    state.set_selected_item(None, cx);
});
```

## API Reference

### TreeState

| Method                         | Description                      |
|--------------------------------|----------------------------------|
| `new(cx)`                      | Create a new tree state          |
| `items(items)`                 | Set initial tree items           |
| `set_items(items, cx)`         | Update tree items and notify     |
| `selected_index()`             | Get currently selected index     |
| `set_selected_index(ix, cx)`   | Set selected index               |
| `set_selected_item(item, cx)`  | Set selected by tree item        |
| `selected_item(item, cx)`      | Get currently selected tree item |
| `selected_entry()`             | Get currently selected entry     |
| `scroll_to_item(ix, strategy)` | Scroll to specific item          |

### TreeItem

| Method            | Description                            |
| ----------------- | -------------------------------------- |
| `new(id, label)`  | Create new tree item with ID and label |
| `child(item)`     | Add single child item                  |
| `children(items)` | Add multiple child items               |
| `expanded(bool)`  | Set expanded state                     |
| `disabled(bool)`  | Set disabled state                     |
| `is_folder()`     | Check if item has children             |
| `is_expanded()`   | Check if item is expanded              |
| `is_disabled()`   | Check if item is disabled              |

### TreeEntry

| Method          | Description                 |
| --------------- | --------------------------- |
| `item()`        | Get the source TreeItem     |
| `depth()`       | Get item depth in tree      |
| `is_folder()`   | Check if entry has children |
| `is_expanded()` | Check if entry is expanded  |
| `is_disabled()` | Check if entry is disabled  |

### tree() Function

| Parameter     | Description                           |
| ------------- | ------------------------------------- |
| `state`       | `Entity<TreeState>` for managing tree |
| `render_item` | Closure for rendering each item       |

#### Render Item Closure

```rust
Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem
```

- `usize`: Item index in flattened tree
- `&TreeEntry`: Tree entry with item and metadata
- `bool`: Whether item is currently selected
- `&mut Window`: Current window context
- `&mut App`: Application context
- Returns: `ListItem` for rendering

## Examples

### Lazy Loading Tree

```rust
struct LazyTreeView {
    tree_state: Entity<TreeState>,
    loaded_paths: HashSet<String>,
}

impl LazyTreeView {
    fn load_children(&mut self, item_id: &str, cx: &mut Context<Self>) {
        if self.loaded_paths.contains(item_id) {
            return;
        }

        let path = PathBuf::from(item_id);
        if path.is_dir() {
            let tree_state = self.tree_state.clone();
            let item_id = item_id.to_string();

            cx.spawn(async move |cx| {
                let children = load_directory_children(&path).await;
                tree_state.update(cx, |state, cx| {
                    // Update specific item with loaded children
                    state.update_item_children(&item_id, children, cx);
                })
            }).detach();

            self.loaded_paths.insert(item_id.to_string());
        }
    }
}
```

### Search and Filter

```rust
struct SearchableTree {
    tree_state: Entity<TreeState>,
    original_items: Vec<TreeItem>,
    search_query: String,
}

impl SearchableTree {
    fn filter_tree(&mut self, query: &str, cx: &mut Context<Self>) {
        self.search_query = query.to_string();

        let filtered_items = if query.is_empty() {
            self.original_items.clone()
        } else {
            filter_tree_items(&self.original_items, query)
        };

        self.tree_state.update(cx, |state, cx| {
            state.set_items(filtered_items, cx);
        });
    }
}

fn filter_tree_items(items: &[TreeItem], query: &str) -> Vec<TreeItem> {
    items.iter()
        .filter_map(|item| {
            if item.label.to_lowercase().contains(&query.to_lowercase()) {
                Some(item.clone().expanded(true)) // Auto-expand matches
            } else {
                // Check if any children match
                let filtered_children = filter_tree_items(&item.children, query);
                if !filtered_children.is_empty() {
                    Some(item.clone()
                        .children(filtered_children)
                        .expanded(true))
                } else {
                    None
                }
            }
        })
        .collect()
}
```

### Multi-Select Tree

```rust
struct MultiSelectTree {
    tree_state: Entity<TreeState>,
    selected_items: HashSet<String>,
}

impl MultiSelectTree {
    fn toggle_selection(&mut self, item_id: &str, cx: &mut Context<Self>) {
        if self.selected_items.contains(item_id) {
            self.selected_items.remove(item_id);
        } else {
            self.selected_items.insert(item_id.to_string());
        }
        cx.notify();
    }

    fn is_selected(&self, item_id: &str) -> bool {
        self.selected_items.contains(item_id)
    }
}

// In render method
tree(&self.tree_state, {
    let view = cx.entity();
    move |ix, entry, _selected, window, cx| {
        view.update(cx, |this, cx| {
            let item = entry.item();
            let is_multi_selected = this.is_selected(&item.id);

            ListItem::new(ix)
                .selected(is_multi_selected)
                .child(
                    h_flex()
                        .gap_2()
                        .child(checkbox().checked(is_multi_selected))
                        .child(item.label.clone())
                )
                .on_click(cx.listener({
                    let item_id = item.id.clone();
                    move |this, _, _, cx| {
                        this.toggle_selection(&item_id, cx);
                    }
                }))
        })
    }
})
```

## Keyboard Navigation

The Tree component supports comprehensive keyboard navigation:

| Key     | Action                                    |
| ------- | ----------------------------------------- |
| `↑`     | Select previous item                      |
| `↓`     | Select next item                          |
| `←`     | Collapse current folder or move to parent |
| `→`     | Expand current folder                     |
| `Enter` | Toggle expand/collapse for folders        |
| `Space` | Custom action (configurable)              |

```rust
// Custom keyboard handling
tree(&tree_state)
    .key_context("MyTree")
    .on_action(cx.listener(|this, action: &MyCustomAction, _, cx| {
        // Handle custom actions
    }))
```
