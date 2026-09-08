---
title: Tabs
description: 将内容拆分为多个标签面板并逐个切换显示的组件。
---

# Tabs

Tabs 用于把内容组织成多个独立分区，一次只显示一个标签面板。它支持多种外观、尺寸、导航控制，以及前后缀元素、滚动和菜单等交互能力。

## 导入

```rust
use gpui_kit::component::tab::{Tab, TabBar};
```

## 用法

### 基础 Tabs

```rust
TabBar::new("tabs")
    .selected_index(0)
    .on_click(|selected_index, _, _| {
        println!("Tab {} selected", selected_index);
    })
    .child(Tab::new().label("Account"))
    .child(Tab::new().label("Profile"))
    .child(Tab::new().label("Settings"))
```

### 标签样式

#### 默认样式

```rust
TabBar::new("default-tabs")
    .selected_index(0)
    .child(Tab::new().label("Account"))
    .child(Tab::new().label("Profile"))
    .child(Tab::new().label("Documents"))
```

#### Underline

```rust
TabBar::new("underline-tabs")
    .underline()
    .selected_index(0)
    .child(Tab::new().label("Account"))
    .child(Tab::new().label("Profile"))
    .child(Tab::new().label("Documents"))
```

#### Pill

```rust
TabBar::new("pill-tabs")
    .pill()
    .selected_index(0)
    .child(Tab::new().label("Account"))
    .child(Tab::new().label("Profile"))
    .child(Tab::new().label("Documents"))
```

#### Outline

```rust
TabBar::new("outline-tabs")
    .outline()
    .selected_index(0)
    .child(Tab::new().label("Account"))
    .child(Tab::new().label("Profile"))
    .child(Tab::new().label("Documents"))
```

#### Segmented

```rust
use gpui_kit::component::IconName;

TabBar::new("segmented-tabs")
    .segmented()
    .selected_index(0)
    .child(IconName::Bot)
    .child(IconName::Calendar)
    .child(IconName::Map)
    .children(vec!["Settings", "About"])
```

### 不同尺寸

```rust
TabBar::new("tabs").xsmall()
    .child(Tab::new().label("Small"))

TabBar::new("tabs").small()
    .child(Tab::new().label("Small"))

TabBar::new("tabs")
    .child(Tab::new().label("Medium"))

TabBar::new("tabs").large()
    .child(Tab::new().label("Large"))
```

### 带图标的标签

```rust
use gpui_kit::component::{Icon, IconName};

TabBar::new("icon-tabs")
    .child(Tab::default().icon(IconName::User).with_variant(TabVariant::Tab))
    .child(Tab::default().icon(IconName::Settings).with_variant(TabVariant::Tab))
    .child(Tab::default().icon(IconName::Mail).with_variant(TabVariant::Tab))
```

### 前缀和后缀

```rust
use gpui_kit::component::button::Button;
use gpui_kit::component::{h_flex, IconName};

TabBar::new("tabs-with-controls")
    .prefix(
        h_flex()
            .gap_1()
            .child(Button::new("back").ghost().xsmall().icon(IconName::ArrowLeft))
            .child(Button::new("forward").ghost().xsmall().icon(IconName::ArrowRight))
    )
    .suffix(
        h_flex()
            .gap_1()
            .child(Button::new("inbox").ghost().xsmall().icon(IconName::Inbox))
            .child(Button::new("more").ghost().xsmall().icon(IconName::Ellipsis))
    )
    .child(Tab::new().label("Account"))
    .child(Tab::new().label("Profile"))
    .child(Tab::new().label("Settings"))
```

### 禁用标签

```rust
TabBar::new("tabs-with-disabled")
    .child(Tab::new().label("Account"))
    .child(Tab::new().label("Profile").disabled(true))
    .child(Tab::new().label("Settings"))
```

### 动态标签

```rust
struct TabsView {
    active_tab: usize,
    tabs: Vec<String>,
}

impl Render for TabsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        TabBar::new("dynamic-tabs")
            .selected_index(self.active_tab)
            .on_click(cx.listener(|view, index, _, cx| {
                view.active_tab = *index;
                cx.notify();
            }))
            .children(
                self.tabs
                    .iter()
                    .map(|tab_name| Tab::new().label(tab_name.clone()))
            )
    }
}
```

### 菜单模式

当标签很多时，可以开启 `menu(true)`，在标签栏末尾显示下拉菜单按钮：

```rust
TabBar::new("tabs-with-menu")
    .menu(true)
    .selected_index(0)
    .child(Tab::new().label("Account"))
    .child(Tab::new().label("Profile"))
    .child(Tab::new().label("Documents"))
    .child(Tab::new().label("Mail"))
    .child(Tab::new().label("Settings"))
```

### 最大标签宽度

使用 `max_width` 限制每个标签的最大宽度。通过 `.label()` 创建的标签会自动截断超长文本并显示省略号。仅图标的标签（`.icon()`）不受此限制。前缀和后缀元素（例如关闭按钮）不会被截断——标签文本会优先让出空间。如果启用了溢出菜单，下拉项仍会显示完整标签文本。

通过 `.child()` 传入自定义内容的标签只会应用宽度限制，需要自行在应当收缩的部分调用 `truncate()`。

```rust
use gpui_kit::{div, px};

TabBar::new("tabs-with-max-width")
    .max_width(px(100.))
    .menu(true)
    .selected_index(0)
    .child(Tab::new().label("Account Settings & Preferences"))
    .child(Tab::new().label("Documents & Files"))
    .child(Tab::new().label("Appearance & Themes"))
    .child(
        Tab::new().child(
            h_flex()
                .gap_1()
                .child(Icon::new(IconName::Bot))
                .child(div().truncate().child("Custom Child Tab")),
        ),
    )
```

## API 参考

### TabBar

| 方法 | 说明 |
| --- | --- |
| `new(id)` | 创建新的标签栏 |
| `child(tab)` | 添加单个标签 |
| `children(tabs)` | 批量添加标签 |
| `selected_index(index)` | 设置当前选中索引 |
| `on_click(fn)` | 点击标签时触发，返回标签索引 |
| `prefix(element)` | 在标签前添加元素 |
| `suffix(element)` | 在标签后添加元素 |
| `last_empty_space(element)` | 自定义尾部空白区域 |
| `track_scroll(handle)` | 配合滚动句柄启用可滚动标签栏 |
| `with_menu(bool)` | 启用下拉菜单选择 |
| `max_width(width)` | 设置每个标签的最大宽度；超长文本自动截断 |

### TabBar 变体

| 方法 | 说明 |
| --- | --- |
| `with_variant(variant)` | 为所有子标签设置统一样式 |
| `underline()` | 下划线样式 |
| `pill()` | 胶囊样式 |
| `outline()` | 描边样式 |
| `segmented()` | 分段控制样式 |

### Tab

| 方法 | 说明 |
| --- | --- |
| `new(label)` | 创建带标签文本的 Tab |
| `empty()` | 创建空 Tab |
| `icon(icon)` | 创建仅图标的 Tab |
| `id(id)` | 设置自定义 ID |
| `with_variant(variant)` | 设置当前 Tab 的样式 |
| `prefix(element)` | 在标签内容前添加元素 |
| `suffix(element)` | 在标签内容后添加元素 |
| `disabled(bool)` | 设置禁用状态 |
| `selected(bool)` | 设置选中状态，通常由 `TabBar` 统一管理 |
| `on_click(fn)` | 为单个标签设置点击回调 |

### 样式

`TabBar` 和 `Tab` 都实现了 `Sizable` trait：

- `xsmall()`：超小尺寸
- `small()`：小尺寸
- `medium()`：中尺寸，默认值
- `large()`：大尺寸

## 说明

- `TabBar` 负责统一管理所有子标签的选中状态
- 当设置了 `TabBar.on_click` 时，单个 `Tab.on_click` 通常不会生效
- 子标签会自动继承父级 `TabBar` 的样式和尺寸
- 标签过多时可通过 `with_menu` 或滚动支持提升可用性
