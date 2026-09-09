---
title: Switch
description: 用于在选中和未选中之间切换的开关控件。
---

# Switch

Switch 是一个二元开关组件，适合表示开启 / 关闭状态。它支持平滑动画、不同尺寸、标签、禁用状态和自定义颜色。

使用 `on_change` 接收请求的新值，由状态所有者保存并调用 `cx.notify()`。原有的 `on_click` 保留为兼容名称；两者设置的是同一个回调，最后一次设置生效。

## 导入

```rust
use gpui_kit::component::switch::Switch;
```

## 用法

### 基础 Switch

```rust
Switch::new("my-switch")
    .checked(false)
    .on_change(|checked, _, _| {
        println!("Switch is now: {}", checked);
    })
```

### 受控 Switch

```rust
struct MyView {
    is_enabled: bool,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Switch::new("switch")
            .checked(self.is_enabled)
            .on_change(cx.listener(|view, checked, _, cx| {
                view.is_enabled = *checked;
                cx.notify();
            }))
    }
}
```

### 带标签

```rust
Switch::new("notifications")
    .label("Enable notifications")
    .checked(true)
    .on_change(|checked, _, _| {
        println!("Notifications: {}", if *checked { "enabled" } else { "disabled" });
    })
```

### 不同尺寸

```rust
Switch::new("small-switch")
    .small()
    .label("Small switch")

Switch::new("medium-switch")
    .label("Medium switch")

Switch::new("custom-switch")
    .with_size(Size::Small)
    .label("Custom size")
```

### 禁用状态

```rust
Switch::new("disabled-off")
    .label("Disabled (off)")
    .disabled(true)
    .checked(false)

Switch::new("disabled-on")
    .label("Disabled (on)")
    .disabled(true)
    .checked(true)
```

### 自定义颜色

`color()` 用于覆盖选中状态下的背景色；禁用态透明度会自动叠加：

```rust
Switch::new("switch")
    .label("Success")
    .checked(true)
    .color(cx.theme().success)

Switch::new("switch")
    .label("Danger")
    .checked(true)
    .color(cx.theme().danger)

Switch::new("switch")
    .label("Disabled")
    .checked(true)
    .color(cx.theme().success)
    .disabled(true)
```

### 带 Tooltip

```rust
Switch::new("switch")
    .label("Airplane mode")
    .tooltip("Enable airplane mode to disable all wireless connections")
    .checked(false)
```

## API 参考

### Switch

| 方法 | 说明 |
| --- | --- |
| `new(id)` | 使用给定 ID 创建开关 |
| `checked(bool)` | 设置当前选中状态 |
| `label(text)` | 设置标签文本 |
| `label_side(side)` | 设置标签位置，`Side::Left` 或 `Side::Right` |
| `disabled(bool)` | 设置禁用状态 |
| `tooltip(text)` | 添加提示文本 |
| `color(color)` | 设置选中时的背景色，默认 `theme.primary` |
| `on_change(fn)` | 点击回调，参数为新的 `&bool` 状态 |

### 样式

实现了 `Sizable` 和 `Disableable` trait：

- `small()`：小尺寸，开关区域约 `28x16px`
- `medium()`：中尺寸，默认，开关区域约 `36x20px`
- `with_size(size)`：显式设置尺寸
- `disabled(bool)`：禁用状态

## 示例

### 设置面板

```rust
struct SettingsView {
    marketing_emails: bool,
    security_emails: bool,
    push_notifications: bool,
}

v_flex()
    .gap_4()
    .child(
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex()
                            .child(Label::new("Marketing emails").text_lg())
                            .child(
                                Label::new("Receive emails about new products and features")
                                    .text_color(theme.muted_foreground)
                            )
                    )
                    .child(
                        Switch::new("marketing")
                            .checked(self.marketing_emails)
                            .on_change(cx.listener(|view, checked, _, cx| {
                                view.marketing_emails = *checked;
                                cx.notify();
                            }))
                    )
            )
    )
```

### 紧凑设置列表

```rust
v_flex()
    .gap_3()
    .child(
        Switch::new("wifi")
            .label("Wi-Fi")
            .label_side(Side::Left)
            .checked(true)
            .small()
    )
    .child(
        Switch::new("bluetooth")
            .label("Bluetooth")
            .label_side(Side::Left)
            .checked(false)
            .small()
    )
```

## 动画

Switch 包含平滑切换动画：

- 切换动画时长约 150ms
- 背景色会在关闭色与激活色之间过渡
- 圆点位置会平滑移动
- 禁用状态下不会触发交互动效
