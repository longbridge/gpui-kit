---
title: Toolbar
description: 用于标题栏、标签面板和自定义表面的透明、可调尺寸命令容器。
---

# Toolbar

Toolbar 是一个透明的水平操作容器，用于在面板标题、标签栏和自定义命令表面中排列按钮、分隔线和简短标签。背景和边框由外层容器负责。

其设计参考了原生 UI 框架中的工具栏：macOS 的 `NSToolbar` 和 Windows 的 `ToolStrip`。

## 引入

```rust
use gpui_kit::component::toolbar::Toolbar;
```

## 组合

使用 `child` 添加实现了 `Sizable` 的控件。Toolbar 会在渲染时把最终尺寸应用到这些控件，因此 `.small()` 写在控件之前或之后都得到相同结果。字符串、分隔线、弹性占位和需要保留自身尺寸的自定义布局使用 `content`。所有项目按源码顺序渲染。

- **命令**：直接传入 `Button`；Toolbar 会应用统一尺寸，并强制使用安静的 `ghost + compact` 外观。按需链式调用 `label`、`icon`、`tooltip`、`on_click` 等。
- **仅图标的按钮**：务必加上 `tooltip`，它同时也是无障碍名称。
- **分隔线**：通过 `content` 传入 `Separator::vertical()`，并指定高度。
- **不可交互的标签**：通过 content 方法传入字符串。

## 用法

### 命令

```rust
Toolbar::new("toolbar")
    .child(
        Button::new("new")
            .icon(IconName::Plus)
            .label("New")
            .on_click(|_, window, cx| { /* ... */ }),
    )
    .content(Separator::vertical().h_5())
    .child(
        Button::new("undo")
            .icon(IconName::Undo2)
            .tooltip("Undo")
            .on_click(|_, window, cx| { /* ... */ }),
    )
    .content(div().flex_1())
    .child(
        Button::new("more")
            .icon(IconName::Ellipsis)
            .tooltip("More options")
            .on_click(|_, window, cx| { /* ... */ }),
    )
```

### 尺寸

通过 `Sizable` 一起改变工具栏高度、间距、文字和内部控件尺寸：`xsmall`（28px）、`small`（32px，默认）和 `medium`（48px）。调用顺序不影响尺寸传播。

```rust
Toolbar::new("toolbar")
    .child(Button::new("new").icon(IconName::Plus).label("New"))
    .child(Button::new("find").icon(IconName::Search).tooltip("Find"))
    .small()
```

### 标签与自定义元素

```rust
Toolbar::new("toolbar")
    .content("Dashboard")
    .content(Separator::vertical().h_5())
    .content(
        h_flex()
            .items_center()
            .gap_1()
            .child(Icon::new(IconName::CircleCheck).xsmall())
            .child("Saved"),
    )
    .content(div().flex_1())
    .child(Button::new("settings").icon(IconName::Settings2).tooltip("Settings"))
```

### 自定义样式

`Toolbar` 默认透明且没有边框。它实现了 `Styled`，独立命令表面可以按需添加外观。

```rust
Toolbar::new("toolbar")
    .bg(cx.theme().secondary)
    .border_color(cx.theme().border)
    .content("Ready")
```

## 分组

用 `ToolbarGroup` 把相关控件包在一起并赋予可访问名称，辅助技术会把这一组控件读作一个整体。它实现了 `Sizable`，父 Toolbar 会把最终尺寸经由 group 传递给每个控件：

```rust
use gpui_kit::component::toolbar::ToolbarGroup;

Toolbar::new("document-toolbar")
    .child(
        ToolbarGroup::new("history-group")
            .label("History")
            .gap_2() // 与工具栏自身的项间距保持一致
            .child(Button::new("undo").icon(IconName::Undo2).tooltip("Undo"))
            .child(Button::new("redo").icon(IconName::Redo2).tooltip("Redo")),
    )
```

与 Base UI 的 `Toolbar.Group` 不同，group 无法禁用其子控件：该 API 通过 React context 传播到 Base UI 自己的按钮 primitive，GPUI 组合模型对任意子控件没有等价机制。禁用内部控件是调用方的职责。

分隔线等非尺寸化元素使用 `content`；可调尺寸控件使用 `child`，由 Toolbar 统一传播尺寸。

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
| `new()`          | 创建一个空的工具栏（small 尺寸）          |
| `child(c)` / `children(cs)` | 按源码顺序添加可调尺寸控件       |
| `content(c)` / `contents(cs)` | 按源码顺序添加非尺寸化内容       |
| `with_size(size)` | 设置工具栏尺寸 —— `xsmall`、`small` 或 `medium`         |
| `disabled(value)` | 禁用方向键导航；宿主同时负责禁用内部控件             |

控件方法要求 `Sizable + IntoElement`，content 方法接受通用元素。`Toolbar` 同时实现了 `Styled` 和 `Sizable`。

## 注意事项

- 需要把后续项目推到尾端时，插入 `content(div().flex_1())`。
- 保持主要命令始终可见；低频操作应放入下拉菜单或溢出菜单，不要藏在 hover 后面。
- Toolbar 默认没有背景和边框，由宿主表面提供。
