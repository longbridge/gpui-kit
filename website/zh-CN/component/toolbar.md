---
title: Toolbar
description: 一条承载命令的工具栏，通常放置在窗口或面板顶部。
---

# Toolbar

Toolbar 是一条水平工具栏，用于承载一排操作 —— 按钮、分隔线和简短标签 —— 通常放置在窗口、面板或区块的顶部。它与上方的 `TitleBar`、底部的 `StatusBar` 搭配使用。

其设计参考了原生 UI 框架中的工具栏：macOS 的 `NSToolbar` 和 Windows 的 `ToolStrip`。

## 引入

```rust
use gpui_kit::component::toolbar::Toolbar;
```

## 区域

向区域传入任意 `impl IntoElement` —— 字符串、`Icon`、`Button`、自定义布局等。`left` 和 `right` 把项固定在两端；`child` / `children` 添加到中间区域，其对齐方式取决于固定了哪一端 —— 同时有 `left` 和 `right` 时居中，只有 `left` 时右对齐，否则左对齐（只有 `right`，或两者都没有时，像普通工具栏一样）。多次调用即可追加更多。

- **命令**：传入一个与工具栏尺寸匹配的 ghost `Button` —— `Button::new(id).ghost()` —— 可链式调用 `label`、`icon`、`tooltip`、`on_click` 等。
- **仅图标的按钮**：务必加上 `tooltip`，它同时也是无障碍名称。
- **分隔线**：传入 `Separator::vertical()`，并指定高度（例如 `.h_5()`）。
- **不可交互的标签**：直接传字符串 —— 它会继承工具栏的文字样式，且没有 hover。

## 用法

### 命令

```rust
Toolbar::new("toolbar")
    .left(
        Button::new("new").ghost()
            .icon(IconName::Plus)
            .label("New")
            .on_click(|_, window, cx| { /* ... */ }),
    )
    .left(Separator::vertical().h_5())
    .left(
        Button::new("undo").ghost()
            .icon(IconName::Undo2)
            .tooltip("Undo")
            .on_click(|_, window, cx| { /* ... */ }),
    )
    .right(
        Button::new("more").ghost()
            .icon(IconName::Ellipsis)
            .tooltip("More options")
            .on_click(|_, window, cx| { /* ... */ }),
    )
```

### 尺寸

通过 `Sizable` 一起改变工具栏高度、间距和文字大小：`xsmall`（28px）、`small`（32px）、`medium`（40px，默认）和 `large`（48px）。内部按钮请使用匹配的尺寸。

```rust
Toolbar::new("toolbar").small()
    .left(Button::new("new").ghost().small().icon(IconName::Plus).label("New"))
    .right(Button::new("find").ghost().small().icon(IconName::Search).tooltip("Find"))
```

### 标签与自定义元素

```rust
Toolbar::new("toolbar")
    .left("Dashboard")
    .left(Separator::vertical().h_5())
    .child(
        h_flex()
            .items_center()
            .gap_1()
            .child(Icon::new(IconName::CircleCheck).xsmall())
            .child("Saved"),
    )
    .right(Button::new("settings").ghost().icon(IconName::Settings2).tooltip("Settings"))
```

### 自定义样式

`Toolbar` 实现了 `Styled`，因此任意样式方法都会覆盖默认值。

```rust
Toolbar::new("toolbar")
    .bg(cx.theme().secondary)
    .border_color(cx.theme().border)
    .left("Ready")
```

## 分组

用 `ToolbarGroup`（从 `gpui_base` re-export）把相关控件包在一起并赋予可访问名称，辅助技术会把这一组控件读作一个整体：

```rust
use gpui_kit::component::toolbar::ToolbarGroup;

Toolbar::new("document-toolbar")
    .left(
        ToolbarGroup::new("history-group")
            .label("History")
            .gap_2() // 与工具栏自身的项间距保持一致
            .child(Button::new("undo").ghost().icon(IconName::Undo2).tooltip("Undo"))
            .child(Button::new("redo").ghost().icon(IconName::Redo2).tooltip("Redo")),
    )
```

与 Base UI 的 `Toolbar.Group` 不同，group 无法禁用其子控件：该 API 通过 React context 传播到 Base UI 自己的按钮 primitive，GPUI 组合模型对任意子控件没有等价机制。禁用内部控件是调用方的职责。

`Separator` 和 `Link` 不需要工具栏专用封装 —— 直接传 `Separator::vertical().h_5()` 和现有的 `Link` 组件即可。

## 键盘

工具栏向辅助技术暴露 `Toolbar` 语义，并拥有漫游键盘焦点，符合 ARIA toolbar 模式，与 Base UI 的 `Toolbar` 一致：

| 按键 | 行为 |
| --- | --- |
| `←` / `→` | 将焦点移动到上一个 / 下一个控件（水平工具栏） |
| `↑` / `↓` | 将焦点移动到上一个 / 下一个控件（垂直工具栏） |
| `Tab` | 进入或离开工具栏；工具栏本身不是 tab 停靠点 |

焦点在两端环绕。内部输入框保留自己的方向键光标行为；请把输入框放在工具栏的末尾。该行为来自无样式的 `gpui_base::Toolbar` primitive，因此在 base 层上构建自定义工具栏的应用也能获得同样的契约。

## API 参考

### Toolbar

| 方法             | 说明                                       |
| ---------------- | ------------------------------------------ |
| `new()`          | 创建一个空的工具栏（medium 尺寸）         |
| `left(child)`    | 向左侧区域追加一个元素（可多次调用）      |
| `right(child)`   | 向右侧区域追加一个元素                     |
| `child(c)` / `children(cs)` | 向中间区域添加元素              |
| `with_size(size)` | 设置工具栏尺寸 —— `xsmall`、`small`、`medium`、`large` |

每个区域方法接受 `impl IntoElement`。`Toolbar` 同时实现了 `Styled` 和 `Sizable`，样式方法（`bg`、`border_color`、`py` 等）可以覆盖默认值。

## 注意事项

- 中间区域（通过 `child` / `children`）在同时有 `left` 和 `right` 时居中，只有 `left` 时右对齐，否则左对齐（只有 `right`，或两者都没有时，像普通工具栏一样）。
- 保持主要命令始终可见；低频操作应放入下拉菜单或溢出菜单，不要藏在 hover 后面。
- 颜色取自 `toolbar`（背景）和 `toolbar_border`（边框）主题变量，缺省回退到标题栏颜色。
