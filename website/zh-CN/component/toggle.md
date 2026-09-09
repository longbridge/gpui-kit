---
title: Toggle
description: 以按钮形态表示开关或选中状态的切换组件。
---

# Toggle

Toggle 是一种按钮式的二元切换组件，用于表示选中 / 未选中、开启 / 关闭等状态。与传统 Switch 不同，Toggle 更像可按下或弹起的按钮，适合工具栏、筛选器和多选项场景。

## 导入

```rust
use gpui_kit::component::button::{Toggle, ToggleGroup};
```

## 用法

### 基础 Toggle

```rust
Toggle::new("toggle1")
    .label("Toggle me")
    .checked(false)
    .on_click(|checked, _, _| {
        println!("Toggle is now: {}", checked);
    })
```

`on_click` 回调接收的是切换后的新状态。

### 图标 Toggle

```rust
use gpui_kit::component::IconName;

Toggle::new("toggle2")
    .icon(IconName::Eye)
    .checked(true)
    .on_click(|checked, _, _| {
        println!("Visibility: {}", if *checked { "shown" } else { "hidden" });
    })
```

### 受控 Toggle

```rust
struct MyView {
    is_active: bool,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Toggle::new("active")
            .label("Active")
            .checked(self.is_active)
            .on_click(cx.listener(|view, checked, _, cx| {
                view.is_active = *checked;
                cx.notify();
            }))
    }
}
```

### 样式变体

```rust
Toggle::new("ghost-toggle")
    .ghost()
    .label("Ghost")

Toggle::new("outline-toggle")
    .outline()
    .label("Outline")
```

### 不同尺寸

```rust
Toggle::new("xs-toggle")
    .icon(IconName::Star)
    .xsmall()

Toggle::new("small-toggle")
    .label("Small")
    .small()

Toggle::new("medium-toggle")
    .label("Medium")

Toggle::new("large-toggle")
    .label("Large")
    .large()
```

### 禁用状态

```rust
Toggle::new("disabled-toggle")
    .label("Disabled")
    .disabled(true)
    .checked(false)

Toggle::new("disabled-checked-toggle")
    .label("Selected (Disabled)")
    .disabled(true)
    .checked(true)
```

## Toggle 与 Switch 的区别

| 特性 | Toggle | Switch |
| --- | --- | --- |
| 外观 | 按钮式，可按下 / 弹起 | 传统滑块式开关 |
| 场景 | 工具栏、筛选器、二元选择 | 设置项、偏好项、开关状态 |
| 状态表达 | 背景和按压感变化 | 滑块位置变化 |
| 分组能力 | 支持 `ToggleGroup` | 主要单独使用 |

## 与 ToggleGroup 配合使用

### 基础分组

```rust
ToggleGroup::new("filter-group")
    .child(Toggle::new(0).icon(IconName::Bell))
    .child(Toggle::new(1).icon(IconName::Bot))
    .child(Toggle::new(2).icon(IconName::Inbox))
    .child(Toggle::new(3).label("Other"))
    .on_click(|checkeds, _, _| {
        println!("Selected toggles: {:?}", checkeds);
    })
```

### 受控分组

```rust
struct FilterView {
    notifications: bool,
    bots: bool,
    inbox: bool,
    other: bool,
}

impl Render for FilterView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ToggleGroup::new("filters")
            .child(Toggle::new(0).icon(IconName::Bell).checked(self.notifications))
            .child(Toggle::new(1).icon(IconName::Bot).checked(self.bots))
            .child(Toggle::new(2).icon(IconName::Inbox).checked(self.inbox))
            .child(Toggle::new(3).label("Other").checked(self.other))
            .on_click(cx.listener(|view, checkeds, _, cx| {
                view.notifications = checkeds[0];
                view.bots = checkeds[1];
                view.inbox = checkeds[2];
                view.other = checkeds[3];
                cx.notify();
            }))
    }
}
```

### 分段样式 ToggleGroup

使用 `segmented()` 可以把一组 Toggle 渲染成贴合的分段控件。这个样式只影响外观，
交互语义仍然是当前的多选模型：`on_click` 会收到每一项最新状态组成的 `Vec<bool>`。

```rust
ToggleGroup::new("formatting")
    .segmented()
    .outline()
    .child(Toggle::new(0).label("Bold").checked(self.bold))
    .child(Toggle::new(1).label("Italic").checked(self.italic))
    .child(Toggle::new(2).label("Code").checked(self.code))
    .on_click(cx.listener(|view, states, _, cx| {
        view.bold = states[0];
        view.italic = states[1];
        view.code = states[2];
        cx.notify();
    }))
```

分段样式默认使用 `0px` 间距，相邻项会共享一个连续边框。需要保留间距时可以传入
非零 `gap`：

```rust
use gpui_kit::px;

ToggleGroup::new("quick-actions")
    .segmented()
    .outline()
    .gap(px(8.))
    .small()
    .child(Toggle::new(0).label("Star"))
    .child(Toggle::new(1).label("Watch"))
    .child(Toggle::new(2).label("Pin"))
```

如果业务需要互斥选择，请继续在视图状态中自行保证只有一项 `checked(true)`，
直到后续提供专门的单选 API。

## 最佳实践

1. 需要按钮式反馈时优先使用 Toggle，而不是 Switch。
2. 一组相关选项应使用 `ToggleGroup` 统一管理。
3. 图标型 Toggle 最好补充 tooltip 或可访问标签。
4. Toggle 状态应与实际业务状态保持同步，避免视觉与数据不一致。
