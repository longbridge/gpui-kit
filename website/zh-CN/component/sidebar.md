---
title: Sidebar
description: 一个可组合、可主题化、可自定义的侧边栏组件。
---

# Sidebar

Sidebar 是一个灵活的应用导航组件，支持折叠状态、嵌套菜单项、头部与底部区域以及响应式布局。它非常适合文件浏览器、后台管理面板和多层级导航界面。

## 导入

```rust
use gpui_kit::component::sidebar::{
    Sidebar, SidebarHeader, SidebarFooter, SidebarGroup,
    SidebarMenu, SidebarMenuItem, SidebarToggleButton
};
```

## 用法

### 基础 Sidebar

```rust
use gpui_kit::component::{sidebar::*, Side};

Sidebar::new()
    .header(
        SidebarHeader::new()
            .child("My Application")
    )
    .child(
        SidebarGroup::new("Navigation")
            .child(
                SidebarMenu::new()
                    .child(
                        SidebarMenuItem::new("Dashboard")
                            .icon(IconName::LayoutDashboard)
                            .on_click(|_, _, _| println!("Dashboard clicked"))
                    )
                    .child(
                        SidebarMenuItem::new("Settings")
                            .icon(IconName::Settings)
                            .on_click(|_, _, _| println!("Settings clicked"))
                    )
            )
    )
    .footer(
        SidebarFooter::new()
            .child("User Profile")
    )
```

### 可折叠 Sidebar

```rust
let mut collapsed = false;

Sidebar::new()
    .collapsed(collapsed)
    .collapsible(true)
    .header(
        SidebarHeader::new()
            .child(
                h_flex()
                    .child(Icon::new(IconName::Home))
                    .when(!collapsed, |this| this.child("Home"))
            )
    )
```

配合切换按钮：

```rust
SidebarToggleButton::new()
    .collapsed(collapsed)
    .on_click(|_, _, _| {
        collapsed = !collapsed;
    })
```

### 嵌套菜单

```rust
SidebarMenuItem::new("Projects")
    .icon(IconName::FolderOpen)
    .active(true)
    .children([
        SidebarMenuItem::new("Web App").active(false),
        SidebarMenuItem::new("Mobile App").active(true),
        SidebarMenuItem::new("Desktop App"),
    ])
```

### 多分组

```rust
Sidebar::new()
    .child(
        SidebarGroup::new("Main")
            .child(
                SidebarMenu::new()
                    .child(SidebarMenuItem::new("Dashboard").icon(IconName::Home))
                    .child(SidebarMenuItem::new("Analytics").icon(IconName::BarChart))
            )
    )
```

### Badge 与后缀

```rust
use gpui_kit::component::{Badge, Switch};

SidebarMenuItem::new("Notifications")
    .icon(IconName::Bell)
    .suffix(
        Badge::new()
            .count(5)
            .child("5")
    )
```

### 右侧放置

```rust
Sidebar::new()
    .side(Side::Right)
    .width(300)
    .header(
        SidebarHeader::new()
            .child("Right Panel")
    )
```

### 右键菜单

```rust
use gpui_kit::component::menu::PopupMenu;

SidebarMenuItem::new("Project Files")
    .icon(IconName::Folder)
    .context_menu(|menu, _, _| {
        menu.link("Open in Editor", "https://editor.example.com")
            .separator()
            .menu_with_description("Rename", "Rename this project", Box::new(RenameAction))
            .menu_with_description("Delete", "Delete this project", Box::new(DeleteAction))
    })
```

### 自定义宽度与样式

```rust
Sidebar::new()
    .width(280)
    .border_width(2)
    .header(
        SidebarHeader::new()
            .p_4()
            .rounded(cx.theme().radius)
            .child("Custom Styled Sidebar")
    )
```

## 主题

Sidebar 使用一组独立的主题颜色：

```rust
cx.theme().sidebar
cx.theme().sidebar_foreground
cx.theme().sidebar_border
cx.theme().sidebar_accent
cx.theme().sidebar_accent_foreground
cx.theme().sidebar_primary
cx.theme().sidebar_primary_foreground
```

## 示例

### 文件浏览器

```rust
Sidebar::new()
    .header(
        SidebarHeader::new()
            .child(
                h_flex()
                    .gap_2()
                    .child(IconName::Folder)
                    .child("Explorer")
            )
    )
```

### 管理后台

```rust
Sidebar::new()
    .header(
        SidebarHeader::new()
            .child(
                h_flex()
                    .gap_2()
                    .child("Admin Panel")
            )
    )
```

### 设置侧栏

```rust
Sidebar::new()
    .width(300)
    .header(
        SidebarHeader::new()
            .child("Settings")
    )
```
