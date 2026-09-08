---
title: Tree
description: 用于显示和导航树形结构数据的层级树组件。
---

# Tree

Tree 是一个用于展示层级数据的通用组件，支持展开/折叠、键盘导航、自定义项渲染以及编程式选中控制。它非常适合文件浏览器、菜单树和嵌套数据结构。

## 导入

```rust
use gpui_kit::component::tree::{tree, TreeState, TreeItem, TreeEntry};
```

## 用法

### 基础树

```rust
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

tree(&tree_state, |ix, entry, selected, window, cx| {
    ListItem::new(ix)
        .child(
            h_flex()
                .gap_2()
                .child(entry.item().label.clone())
        )
})
```

### 文件树与图标

```rust
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
        .pl(px(16.) * entry.depth() + px(12.))
        .child(
            h_flex()
                .gap_2()
                .child(icon)
                .child(item.label.clone())
        )
})
```

### 动态加载

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
```

### 选择处理

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
```

### 禁用项

```rust
TreeItem::new("protected", "Protected Folder")
    .disabled(true)
    .child(TreeItem::new("secret.txt", "secret.txt"))
```

### 编程式控制

```rust
if let Some(entry) = tree_state.read(cx).selected_entry() {
    println!("Current selection: {}", entry.item().label);
}

tree_state.update(cx, |state, cx| {
    state.set_selected_index(Some(2), cx);
});
```

## API 参考

### TreeState

- `new(cx)`
- `items(items)`
- `set_items(items, cx)`
- `selected_index()`
- `set_selected_index(ix, cx)`
- `set_selected_item(item, cx)`
- `selected_entry()`
- `scroll_to_item(ix, strategy)`

### TreeItem

- `new(id, label)`
- `child(item)`
- `children(items)`
- `expanded(bool)`
- `disabled(bool)`

### TreeEntry

- `item()`
- `depth()`
- `is_folder()`
- `is_expanded()`
- `is_disabled()`

## 键盘导航

| 按键 | 行为 |
| --- | --- |
| `↑` | 选中上一个节点 |
| `↓` | 选中下一个节点 |
| `←` | 折叠当前节点或移动到父级 |
| `→` | 展开当前节点 |
| `Enter` | 切换展开/折叠 |
| `Space` | 自定义动作 |
