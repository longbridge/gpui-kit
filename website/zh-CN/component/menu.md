---
title: Menu
description: 支持图标、快捷键、子菜单和多种菜单项类型的上下文菜单与弹出菜单。
---

# PopupMenu

Menu 组件同时提供上下文菜单和弹出菜单，支持图标、键盘快捷键、子菜单、分隔线、勾选项以及自定义元素，并内置可访问性与键盘导航支持。

## 导入

```rust
use gpui_kit::component::{
    menu::{PopupMenu, PopupMenuItem, ContextMenuExt, DropdownMenu},
    Button
};
use gpui_kit::{actions, Action};
```

## 用法

### ContextMenu

右键点击元素时显示上下文菜单：

```rust
use gpui_kit::component::menu::ContextMenuExt;

div()
    .id("my-element")
    .child("Right click me")
    .context_menu(|menu, window, cx| {
        menu.menu("Copy", Box::new(Copy))
            .menu("Paste", Box::new(Paste))
            .separator()
            .menu("Delete", Box::new(Delete))
    })
```

### DropdownMenu

下拉菜单通常由按钮或其它可交互元素触发：

```rust
use gpui_kit::component::popup_menu::{PopupMenuExt as _, PopupMenuItem};

let view = cx.entity();
Button::new("menu-btn")
    .label("Open Menu")
    .dropdown_menu(|menu, window, cx| {
        menu.menu("New File", Box::new(NewFile))
            .menu("Open File", Box::new(OpenFile))
            .link("Documentation", "https://gpui-kit.com/")
            .separator()
            .item(PopupMenuItem::new("Custom Action")
                .on_click(window.listener_for(&view, |this, _, window, cx| {
                    // Custom action logic here
                    this.
                })
            )
            .separator()
            .menu("Exit", Box::new(Exit))
    })
```

:::tip
每个菜单项都可以关联一个 [Action]。这种设计可以更好地接入 GPUI 的 action 与快捷键系统，在适用时自动显示对应快捷键。

因此，推荐优先使用 [Action] 定义菜单行为。

如果你不想使用 [Action]，也可以通过 `item` 方法配合 [PopupMenuItem] 创建自定义菜单项，并使用 `on_click` 直接处理点击事件。
:::

### 锚点位置

控制下拉菜单相对触发器的显示位置：

```rust
use gpui_kit::Anchor;

Button::new("menu-btn")
    .label("Options")
    .dropdown_menu_with_anchor(Anchor::TopRight, |menu, window, cx| {
        menu.menu("Option 1", Box::new(Action1))
            .menu("Option 2", Box::new(Action2))
    })
```

### 图标

```rust
use gpui_kit::component::IconName;

menu.menu_with_icon("Search", IconName::Search, Box::new(Search))
    .menu_with_icon("Settings", IconName::Settings, Box::new(OpenSettings))
    .separator()
    .menu_with_icon("Help", IconName::Help, Box::new(ShowHelp))
```

### 禁用状态

```rust
menu.menu("Available Action", Box::new(Action1))
    .menu_with_disabled("Disabled Action", Box::new(Action2), true)
    .menu_with_icon_and_disabled(
        "Unavailable",
        IconName::Lock,
        Box::new(Action3),
        true
    )
```

### 勾选状态

```rust
let is_enabled = true;

menu.menu_with_check("Enable Feature", is_enabled, Box::new(ToggleFeature))
    .menu_with_check("Show Sidebar", sidebar_visible, Box::new(ToggleSidebar))
```

默认情况下，勾选图标显示在菜单项左侧；如果菜单项已有图标，勾选图标会替换左侧图标。

也可以通过 `check_side` 将勾选图标放到右侧：

```rust
menu.check_size(Side::Right)
    .menu_with_check("Enable Feature", is_enabled, Box::new(ToggleFeature))
```

### 分隔线

```rust
menu.menu("New", Box::new(NewFile))
    .menu("Open", Box::new(OpenFile))
    .separator()
    .menu("Copy", Box::new(Copy))
    .menu("Paste", Box::new(Paste))
    .separator()
    .menu("Exit", Box::new(Exit))
```

### 标签

```rust
menu.label("File Operations")
    .menu("New", Box::new(NewFile))
    .menu("Open", Box::new(OpenFile))
    .separator()
    .label("Edit Operations")
    .menu("Copy", Box::new(Copy))
    .menu("Paste", Box::new(Paste))
```

### 链接菜单项

```rust
menu.link("Documentation", "https://docs.example.com")
    .link_with_icon(
        "GitHub",
        IconName::GitHub,
        "https://github.com/example/repo"
    )
    .separator()
    .external_link_icon(false)
    .link("Support", "https://support.example.com")
```

### 自定义元素

```rust
use gpui_kit::component::{h_flex, v_flex};

menu.menu_element(Box::new(CustomAction), |window, cx| {
        v_flex()
            .child("Custom Element")
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("This is a subtitle")
            )
    })
    .menu_element_with_icon(
        IconName::Info,
        Box::new(InfoAction),
        |window, cx| {
            h_flex()
                .gap_1()
                .child("Status")
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().success)
                        .child("✓ Connected")
                )
        }
    )
```

### 键盘快捷键

```rust
actions!(my_app, [Copy, Paste, Cut]);

cx.bind_keys([
    KeyBinding::new("ctrl-c", Copy, Some("editor")),
    KeyBinding::new("ctrl-v", Paste, Some("editor")),
    KeyBinding::new("ctrl-x", Cut, Some("editor")),
]);

menu.action_context(focus_handle)
    .menu("Copy", Box::new(Copy))
    .menu("Paste", Box::new(Paste))
    .menu("Cut", Box::new(Cut))
```

### 子菜单

```rust
menu.submenu("File", window, cx, |submenu, window, cx| {
        submenu.menu("New", Box::new(NewFile))
            .menu("Open", Box::new(OpenFile))
            .separator()
            .menu("Recent", Box::new(ShowRecent))
    })
    .submenu("Edit", window, cx, |submenu, window, cx| {
        submenu.menu("Undo", Box::new(Undo))
            .menu("Redo", Box::new(Redo))
    })
```

### 带图标的子菜单

```rust
menu.submenu_with_icon(
        Some(IconName::Folder.into()),
        "Project",
        window,
        cx,
        |submenu, window, cx| {
            submenu.menu("Open Project", Box::new(OpenProject))
                .menu("Close Project", Box::new(CloseProject))
        }
    )
```

### 可滚动菜单

:::warning
启用 `scrollable()` 后，尽量不要在同一个菜单里再使用子菜单，否则容易影响可用性。
:::

```rust
Button::new("large-menu")
    .label("Many Options")
    .dropdown_menu(|menu, window, cx| {
        let mut menu = menu
            .scrollable(true)
            .max_h(px(300.))
            .label("Select an option");

        for i in 0..100 {
            menu = menu.menu(
                format!("Option {}", i),
                Box::new(SelectOption(i))
            );
        }
        menu
    })
```

### 菜单尺寸

```rust
menu.min_w(px(200.))
    .max_w(px(400.))
    .max_h(px(300.))
    .scrollable(true)
```

### Action 上下文

```rust
let focus_handle = cx.focus_handle();

menu.action_context(focus_handle)
    .menu("Copy", Box::new(Copy))
    .menu("Paste", Box::new(Paste))
```

## API 参考

- [PopupMenu]
- [context_menu]
- [PopupMenuItem]

## 示例

### 文件管理器上下文菜单

```rust
div()
    .id("file-manager")
    .child("Right-click for options")
    .context_menu(|menu, window, cx| {
        menu.menu_with_icon("Open", IconName::FolderOpen, Box::new(Open))
            .separator()
            .menu_with_icon("Copy", IconName::Copy, Box::new(Copy))
            .menu_with_icon("Cut", IconName::Scissors, Box::new(Cut))
            .menu_with_icon("Paste", IconName::Clipboard, Box::new(Paste))
            .separator()
            .submenu("New", window, cx, |submenu, window, cx| {
                submenu.menu_with_icon("File", IconName::File, Box::new(NewFile))
                    .menu_with_icon("Folder", IconName::Folder, Box::new(NewFolder))
            })
            .separator()
            .menu_with_icon("Delete", IconName::Trash, Box::new(Delete))
            .separator()
            .menu("Properties", Box::new(ShowProperties))
    })
```

### 不使用 action 添加菜单项

```rust
use gpui_kit::component::{menu::PopupMenuItem, Button};

Button::new("custom-item-menu")
    .label("Options")
    .dropdown_menu(|menu, window, cx| {
        menu.item(
            PopupMenuItem::new("Custom Action")
                .disabled(false)
                .icon(IconName::Star)
                .on_click(|window, cx| {
                    println!("Custom Action Clicked!");
                })
        )
        .separator()
        .menu("Standard Action", Box::new(StandardAction))
    })
```

### 带快捷键的编辑器菜单

```rust
actions!(editor, [Save, SaveAs, Find, Replace, ToggleWordWrap]);

cx.bind_keys([
    KeyBinding::new("ctrl-s", Save, Some("editor")),
    KeyBinding::new("ctrl-shift-s", SaveAs, Some("editor")),
    KeyBinding::new("ctrl-f", Find, Some("editor")),
    KeyBinding::new("ctrl-h", Replace, Some("editor")),
]);

let editor_focus = cx.focus_handle();

Button::new("editor-menu")
    .label("Edit")
    .dropdown_menu(|menu, window, cx| {
        menu.action_context(editor_focus)
            .menu("Save", Box::new(Save))
            .menu("Save As...", Box::new(SaveAs))
            .separator()
            .menu("Find", Box::new(Find))
            .menu("Replace", Box::new(Replace))
            .separator()
            .menu_with_check("Word Wrap", true, Box::new(ToggleWordWrap))
    })
```

### 带自定义元素的设置菜单

```rust
Button::new("settings")
    .label("Settings")
    .dropdown_menu(|menu, window, cx| {
        menu.label("Display")
            .menu_element_with_check(dark_mode, Box::new(ToggleDarkMode), |window, cx| {
                h_flex()
                    .gap_2()
                    .child("Dark Mode")
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(if dark_mode { "On" } else { "Off" })
                    )
            })
            .separator()
            .label("Account")
            .menu_element_with_icon(
                IconName::User,
                Box::new(ShowProfile),
                |window, cx| {
                    v_flex()
                        .child("John Doe")
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("john@example.com")
                        )
                }
            )
            .separator()
            .link_with_icon("Help Center", IconName::Help, "https://help.example.com")
            .menu("Sign Out", Box::new(SignOut))
    })
```

## 键盘快捷键

| 按键 | 行为 |
| --- | --- |
| `↑` / `↓` | 在菜单项之间移动 |
| `←` / `→` | 在子菜单之间移动 |
| `Enter` / `Space` | 激活当前菜单项 |
| `Escape` | 关闭菜单 |
| `Tab` | 关闭菜单并聚焦下一个元素 |

## 最佳实践

1. 使用分隔线对相关操作分组。
2. 在整个应用中保持图标语义一致。
3. 将最常用的操作放在靠前位置。
4. 为高频操作提供快捷键。
5. 根据上下文只展示相关菜单项。
6. 对复杂层级使用子菜单而不是把所有项堆在一起。
7. 使用清晰、动作导向的文案。
8. 菜单项很多时开启滚动并设置合理高度。

[PopupMenu]: https://docs.rs/gpui-component/latest/gpui_component/menu/struct.PopupMenu.html
[PopupMenuItem]: https://docs.rs/gpui-component/latest/gpui_component/menu/struct.PopupMenuItem.html
[context_menu]: https://docs.rs/gpui-component/latest/gpui_component/menu/trait.ContextMenuExt.html#method.context_menu
[Action]: https://docs.rs/gpui/latest/gpui/trait.Action.html
