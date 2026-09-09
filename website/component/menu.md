---
title: Menu
description: Context menus and popup menus with support for icons, shortcuts, submenus, and various menu item types.
---

# PopupMenu

The Menu component provides both context menus (right-click menus) and popup menus with comprehensive features including icons, keyboard shortcuts, submenus, separators, checkable items, and custom elements. Built with accessibility and keyboard navigation in mind.

## Import

```rust
use gpui_kit::component::{
    menu::{PopupMenu, PopupMenuItem, ContextMenuExt, DropdownMenu},
    Button
};
use gpui_kit::{actions, Action};
```

## Usage

### ContextMenu

Context menus appear when right-clicking on an element:

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

Dropdown menus are triggered by buttons or other interactive elements:

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
As you see, the each menu item is associated with an [Action],
we choice this design to better integrate with GPUI's action and key binding system,
allowing menu items to automatically display keyboard shortcuts when applicable.

So, the [Action] is the recommended way to define menu item behaviors.

However, if you prefer not to use [Action]s, you can create custom menu items using the `item` method along with [PopupMenuItem].
There have a `on_click` callback to handle the click event directly.
:::

### Anchor Position

Control where the dropdown menu appears relative to the trigger:

```rust
use gpui_kit::Anchor;

Button::new("menu-btn")
    .label("Options")
    .dropdown_menu_with_anchor(Anchor::TopRight, |menu, window, cx| {
        menu.menu("Option 1", Box::new(Action1))
            .menu("Option 2", Box::new(Action2))
    })
```

### Icons

Add icons to menu items for better visual clarity:

```rust
use gpui_kit::component::IconName;

menu.menu_with_icon("Search", IconName::Search, Box::new(Search))
    .menu_with_icon("Settings", IconName::Settings, Box::new(OpenSettings))
    .separator()
    .menu_with_icon("Help", IconName::Help, Box::new(ShowHelp))
```

### Disabled State

Create disabled menu items that cannot be activated:

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

### Check state

Create menu items that show a check state:

```rust
let is_enabled = true;

menu.menu_with_check("Enable Feature", is_enabled, Box::new(ToggleFeature))
    .menu_with_check("Show Sidebar", sidebar_visible, Box::new(ToggleSidebar))
```

By default, the check icon will be shown on the left side of the menu item, if this menu item has an icon, the check icon will replace the icon on the left side.

There also have a `check_side` option for you to config the check icon to be shown on the right side:

```rust
menu.check_size(Side::Right)
    .menu_with_check("Enable Feature", is_enabled, Box::new(ToggleFeature))
```

### Separators

Use separators to group related menu items:

```rust
menu.menu("New", Box::new(NewFile))
    .menu("Open", Box::new(OpenFile))
    .separator()  // Groups file operations
    .menu("Copy", Box::new(Copy))
    .menu("Paste", Box::new(Paste))
    .separator()  // Groups edit operations
    .menu("Exit", Box::new(Exit))
```

### Labels

Add non-interactive labels to organize menu sections:

```rust
menu.label("File Operations")
    .menu("New", Box::new(NewFile))
    .menu("Open", Box::new(OpenFile))
    .separator()
    .label("Edit Operations")
    .menu("Copy", Box::new(Copy))
    .menu("Paste", Box::new(Paste))
```

### Link MenuItem

Create menu items that open external links:

```rust
menu.link("Documentation", "https://docs.example.com")
    .link_with_icon(
        "GitHub",
        IconName::GitHub,
        "https://github.com/example/repo"
    )
    .separator()
    .external_link_icon(false) // Hide external link icons
    .link("Support", "https://support.example.com")
```

### Custom Element

Create custom menu items with complex content:

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

### Keyboard Shortcuts

Menu items automatically display keyboard shortcuts if they're bound to actions:

```rust
// First define your actions and key bindings
actions!(my_app, [Copy, Paste, Cut]);

// In your app initialization
cx.bind_keys([
    KeyBinding::new("ctrl-c", Copy, Some("editor")),
    KeyBinding::new("ctrl-v", Paste, Some("editor")),
    KeyBinding::new("ctrl-x", Cut, Some("editor")),
]);

// The menu will automatically show shortcuts
menu.action_context(focus_handle) // Set context for shortcuts
    .menu("Copy", Box::new(Copy))     // Will show "Ctrl+C"
    .menu("Paste", Box::new(Paste))   // Will show "Ctrl+V"
    .menu("Cut", Box::new(Cut))       // Will show "Ctrl+X"
```

### Submenus

Create nested menus with submenu support:

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

### Submenus with Icons

Add icons to submenu headers:

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

### Scrollable Menus

:::warning
When you have enabled `scrollable()` on a menu, avoid using submenus within it, as this can lead to usability issues.
:::

For menus with many items, enable scrolling:

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

### Menu Sizing

Control menu dimensions:

```rust
menu.min_w(px(200.))      // Minimum width
    .max_w(px(400.))      // Maximum width
    .max_h(px(300.))      // Maximum height
    .scrollable(true)         // Enable scrolling when content exceeds max height
```

### Action Context

Set the focus context for handling menu actions:

```rust
let focus_handle = cx.focus_handle();

menu.action_context(focus_handle)
    .menu("Copy", Box::new(Copy))
    .menu("Paste", Box::new(Paste))
```

## API Reference

- [PopupMenu]
- [context_menu]
- [PopupMenuItem]

## Examples

### File Manager Context Menu

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

### Add MenuItem without action

Sometimes you may not like to define an action for a menu item, you just want add a `on_click` handler, in this case, the `item` and [PopupMenuItem] can help you:

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
                    // Custom click handler logic
                    println!("Custom Action Clicked!");
                })
        )
        .separator()
        .menu("Standard Action", Box::new(StandardAction))
    })
```

### Editor Menu with Shortcuts

```rust
// Define actions with keyboard shortcuts
actions!(editor, [Save, SaveAs, Find, Replace, ToggleWordWrap]);

// Set up key bindings
cx.bind_keys([
    KeyBinding::new("ctrl-s", Save, Some("editor")),
    KeyBinding::new("ctrl-shift-s", SaveAs, Some("editor")),
    KeyBinding::new("ctrl-f", Find, Some("editor")),
    KeyBinding::new("ctrl-h", Replace, Some("editor")),
]);

// Create menu with automatic shortcuts
let editor_focus = cx.focus_handle();

Button::new("editor-menu")
    .label("Edit")
    .dropdown_menu(|menu, window, cx| {
        menu.action_context(editor_focus)
            .menu("Save", Box::new(Save))           // Shows "Ctrl+S"
            .menu("Save As...", Box::new(SaveAs))   // Shows "Ctrl+Shift+S"
            .separator()
            .menu("Find", Box::new(Find))           // Shows "Ctrl+F"
            .menu("Replace", Box::new(Replace))     // Shows "Ctrl+H"
            .separator()
            .menu_with_check("Word Wrap", true, Box::new(ToggleWordWrap))
    })
```

### Settings Menu with Custom Elements

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

## Keyboard Shortcuts

| Key               | Action                            |
| ----------------- | --------------------------------- |
| `↑` / `↓`         | Navigate menu items               |
| `←` / `→`         | Navigate submenus                 |
| `Enter` / `Space` | Activate menu item                |
| `Escape`          | Close menu                        |
| `Tab`             | Close menu and focus next element |

## Best Practices

1. **Group Related Items**: Use separators to group related functionality
2. **Consistent Icons**: Use consistent iconography across your application
3. **Logical Order**: Place most common actions at the top
4. **Keyboard Shortcuts**: Provide shortcuts for frequently used actions
5. **Context Awareness**: Show only relevant items for the current context
6. **Progressive Disclosure**: Use submenus for complex hierarchies
7. **Clear Labels**: Use descriptive, action-oriented labels
8. **Reasonable Limits**: Use scrollable menus for more than 10-15 items

[PopupMenu]: https://docs.rs/gpui-component/latest/gpui_component/menu/struct.PopupMenu.html
[PopupMenuItem]: https://docs.rs/gpui-component/latest/gpui_component/menu/struct.PopupMenuItem.html
[context_menu]: https://docs.rs/gpui-component/latest/gpui_component/menu/trait.ContextMenuExt.html#method.context_menu
[Action]: https://docs.rs/gpui/latest/gpui/trait.Action.html
