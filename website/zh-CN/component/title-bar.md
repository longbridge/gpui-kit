---
title: TitleBar
description: 支持窗口控制和自定义内容的自定义标题栏组件。
---

# TitleBar

TitleBar 用于替换系统默认标题栏，提供可定制的窗口标题区域。它内置平台相关的窗口控制按钮，并支持插入菜单栏、状态信息和自定义操作区。组件会根据 macOS、Windows 和 Linux 自动调整行为和视觉样式。

## 导入

```rust
use gpui_kit::component::TitleBar;
```

## 用法

### 基础标题栏

```rust
TitleBar::new()
    .child(div().child("My Application"))
```

### 带自定义内容的标题栏

```rust
TitleBar::new()
    .child(
        div()
            .flex()
            .items_center()
            .gap_3()
            .child("App Name")
            .child(Badge::new().count(5))
    )
    .child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(Button::new("settings").icon(IconName::Settings))
            .child(Button::new("profile").icon(IconName::User))
    )
```

### 带菜单栏

```rust
TitleBar::new()
    .child(
        div()
            .flex()
            .items_center()
            .child(AppMenuBar::new(window, cx))
    )
    .child(
        div()
            .flex()
            .items_center()
            .justify_end()
            .gap_2()
            .child(Button::new("github").icon(IconName::GitHub))
            .child(Button::new("notifications").icon(IconName::Bell))
    )
```

### Linux 自定义关闭行为

```rust
TitleBar::new()
    .on_close_window(|_, window, cx| {
        window.push_notification("Saving before close...", cx);
        window.remove_window();
    })
    .child(div().child("Custom Close Behavior"))
```

### 自定义样式

```rust
TitleBar::new()
    .bg(cx.theme().primary)
    .border_color(cx.theme().primary_border)
    .child(
        div()
            .text_color(cx.theme().primary_foreground)
            .child("Styled Title Bar")
    )
```

### 窗口配置

推荐以 `TitleBar::window_options()` 作为窗口配置的基础，它会配置好标题栏所需的
全部选项，其中包括让标题栏（而不是系统）来处理拖动和双击。

```rust
use gpui_kit::WindowOptions;

WindowOptions {
    window_bounds: Some(window_bounds),
    ..TitleBar::window_options()
}
```

如果自行构造 [`WindowOptions`]，需要同时设置这两个字段：

```rust
use gpui_kit::WindowOptions;

WindowOptions {
    titlebar: Some(TitleBar::title_bar_options()),
    // macOS 上必须设置，否则系统也会处理标题栏双击，
    // 并且会为了判定双击而延迟投递标题栏点击。
    app_owns_titlebar_drag: true,
    ..Default::default()
}
```

## 平台差异

### macOS

- 使用原生红黄绿窗口按钮
- traffic light 默认位置为 `(9px, 9px)`
- 双击标题栏会调用 `window.titlebar_double_click()`
- 左侧默认预留 `80px`
- 默认表现为透明标题栏

### Windows

- 使用自定义窗口控制按钮并接入系统窗口管理
- 通过 `WindowControlArea` 处理交互
- 支持 hover 和 active 状态
- 每个控制按钮宽度固定为 `34px`
- 左侧默认内边距为 `12px`

### Linux

- 使用手动事件处理的自定义窗口控制按钮
- 支持通过 `on_close_window()` 覆盖关闭逻辑
- 支持双击最大化 / 还原
- 支持右键弹出窗口菜单
- 支持在标题栏区域拖动窗口

## API 参考

### TitleBar

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建标题栏 |
| `child(element)` | 向标题栏中添加子元素 |
| `on_close_window(fn)` | 自定义关闭行为，仅 Linux 有效 |
| `title_bar_options()` | 获取窗口可用的默认标题栏配置 |
| `window_options()` | 获取标题栏配套的默认窗口配置 |

### 窗口配置项

| 属性 | 说明 |
| --- | --- |
| `appears_transparent` | 标题栏透明（默认 true） |
| `traffic_light_position` | macOS 红黄绿按钮位置 |
| `title` | 窗口标题（使用自定义标题栏时可选） |
| `app_owns_titlebar_drag` | 由标题栏自行处理拖动与双击（仅 macOS） |

### 常量

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `TITLE_BAR_HEIGHT` | `34px` | 标准标题栏高度 |
| `TITLE_BAR_LEFT_PADDING` | `80px`（macOS），`12px`（其他） | 内容区域左侧留白 |

## 说明

- 组件会自动处理平台相关的标题栏行为
- Windows 和 Linux 才会渲染自定义窗口控制按钮
- 拖拽窗口的逻辑已内置在合适区域中
- 自定义样式时应考虑各平台对标题栏的交互习惯
