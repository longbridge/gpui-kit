---
title: Button
description: 显示一个按钮，或外观表现为按钮的组件。
---

# Button

[Button] 是一个支持多种变体、尺寸和状态的按钮组件。它支持图标、加载态，也可以与 [ButtonGroup] 组合使用。

## 导入

```rust
use gpui_kit::component::button::{Button, ButtonGroup};
```

## 用法

### 基础按钮

```rust
Button::new("my-button")
    .label("Click me")
    .on_click(|_, _, _| {
        println!("Button clicked!");
    })
```

### 变体

```rust
// Primary button
Button::new("btn-primary").primary().label("Primary")

// Secondary button (default)
Button::new("btn-secondary").label("Secondary")

// Danger button
Button::new("btn-danger").danger().label("Delete")

// Warning button
Button::new("btn-warning").warning().label("Warning")

// Success button
Button::new("btn-success").success().label("Success")

// Info button
Button::new("btn-info").info().label("Info")

// Ghost button
Button::new("btn-ghost").ghost().label("Ghost")

// Link button
Button::new("btn-link").link().label("Link")

// Text button
Button::new("btn-text").text().label("Text")
```

### Outline 按钮

`outline` 不是独立变体，而是可以和其它变体叠加使用：

```rust
Button::new("btn").primary().outline().label("Primary Outline")
Button::new("btn").danger().outline().label("Danger Outline")
```

### 紧凑模式

`compact` 会减少按钮内边距，使按钮更紧凑：

```rust
Button::new("btn")
    .label("Compact")
    .compact()
```

### 尺寸

Button 支持 [Sizable] trait：

```rust
Button::new("btn").xsmall().label("Extra Small")
Button::new("btn").small().label("Small")
Button::new("btn").label("Medium") // default
Button::new("btn").large().label("Large")
```

### 图标

`icon` 方法支持多种图标类型：

- **[Icon] / [IconName]** - 静态图标
- **[Spinner]** - 加载中的旋转图标
- **[ProgressCircle]** - 环形进度图标

这些图标会自动适配按钮尺寸，也可以继续定制颜色和其他属性。

#### 基础图标

```rust
use gpui_kit::component::{Icon, IconName};

Button::new("btn")
    .icon(IconName::Check)
    .label("Confirm")

Button::new("btn")
    .icon(Icon::new(IconName::Heart))
    .label("Like")

Button::new("btn")
    .icon(IconName::Search)
```

#### Spinner

```rust
use gpui_kit::component::spinner::Spinner;

Button::new("btn")
    .icon(Spinner::new())
    .label("Loading...")

Button::new("btn")
    .icon(Spinner::new().color(cx.theme().blue))
    .label("Processing")

Button::new("btn")
    .icon(Spinner::new().icon(IconName::LoaderCircle))
    .label("Syncing")
```

#### ProgressCircle

```rust
use gpui_kit::component::progress::ProgressCircle;

Button::new("btn")
    .icon(ProgressCircle::new("install-progress").value(45.0))
    .label("Installing...")

Button::new("btn")
    .primary()
    .icon(
        ProgressCircle::new("download-progress")
            .value(75.0)
            .color(cx.theme().primary_foreground)
    )
    .label("Downloading")
```

### 动态更新图标

图标可以随组件状态动态变化：

```rust
struct InstallButton {
    progress: f32,
    is_installing: bool,
}

impl InstallButton {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let button = Button::new("install-btn")
            .label(if self.is_installing {
                "Installing..."
            } else {
                "Install"
            });

        if self.is_installing {
            button.icon(
                ProgressCircle::new("install-progress")
                    .value(self.progress)
            )
        } else {
            button.icon(IconName::Download)
        }
    }
}
```

### 加载态

按钮进入 `loading(true)` 时，会自动处理图标切换：

```rust
Button::new("btn")
    .icon(Spinner::new())
    .label("Processing")
    .loading(true)

Button::new("btn")
    .icon(IconName::Save)
    .label("Saving")
    .loading(true)
```

### 下拉箭头

`.dropdown_caret(true)` 可以在按钮右侧增加一个下拉箭头：

```rust
Button::new("btn")
    .label("Options")
    .dropdown_caret(true)
```

### 状态

Button 常见状态包括 `disabled`、`loading` 和 `selected`：

```rust
Button::new("btn")
    .label("Disabled")
    .disabled(true)

Button::new("btn")
    .label("Loading")
    .loading(true)

Button::new("btn")
    .label("Selected")
    .selected(true)
```

## Button Group

```rust
ButtonGroup::new("btn-group")
    .child(Button::new("btn1").label("One"))
    .child(Button::new("btn2").label("Two"))
    .child(Button::new("btn3").label("Three"))
```

### 切换式按钮组

```rust
ButtonGroup::new("toggle-group")
    .multiple(true)
    .child(Button::new("btn1").label("Option 1").selected(true))
    .child(Button::new("btn2").label("Option 2"))
    .child(Button::new("btn3").label("Option 3"))
    .on_click(|selected_indices, _, _| {
        println!("Selected: {:?}", selected_indices);
    })
```

## 自定义变体

```rust
use gpui_kit::component::button::ButtonCustomVariant;

let custom = ButtonCustomVariant::new(cx)
    .color(cx.theme().magenta)
    .foreground(cx.theme().primary_foreground)
    .border(cx.theme().magenta)
    .hover(cx.theme().magenta.opacity(0.1))
    .active(cx.theme().magenta);

Button::new("custom-btn")
    .custom(custom)
    .label("Custom Button")
```

## 示例

### Tooltip

```rust
Button::new("btn")
    .label("Hover me")
    .tooltip("This is a helpful tooltip")
```

### 自定义子内容

```rust
Button::new("btn")
    .child(
        h_flex()
            .items_center()
            .gap_2()
            .child("Custom Content")
            .child(IconName::ChevronDown)
            .child(IconName::Eye)
    )
```

[Button]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.Button.html
[ButtonGroup]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.ButtonGroup.html
[ButtonCustomVariant]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.ButtonCustomVariant.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html
[Spinner]: https://docs.rs/gpui-component/latest/gpui_component/spinner/struct.Spinner.html
[ProgressCircle]: https://docs.rs/gpui-component/latest/gpui_component/progress/struct.ProgressCircle.html
[Icon]: https://docs.rs/gpui-component/latest/gpui_component/icon/struct.Icon.html
[IconName]: https://docs.rs/gpui-component/latest/gpui_component/icon/enum.IconName.html
