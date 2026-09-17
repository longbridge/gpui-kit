---
title: Checkbox
description: 用于切换选中与未选中状态的复选框组件。
---

# Checkbox

Checkbox 是一个用于二元选择的复选框组件，支持标签、禁用状态和不同文字尺寸。

使用 `on_change` 接收请求的新值，由状态所有者保存并调用 `cx.notify()`。原有的 `on_click` 保留为兼容名称；两者设置的是同一个回调，最后一次设置生效。

## 导入

```rust
use gpui_kit::component::checkbox::Checkbox;
```

## 用法

### 基础 Checkbox

```rust
Checkbox::new("my-checkbox")
    .label("Accept terms and conditions")
    .checked(false)
    .on_change(|checked, _, _| {
        println!("Checkbox is now: {}", checked);
    })
```

`on_change` 会在用户切换状态时触发，接收到的是切换后的新状态。

### 受控 Checkbox

这份完整的 **Tested consumer recipe** 将值保留在渲染所有者上，从 `on_change` 接收请求的新值、保存后再通知：

<!-- recipe:controlled-value:start -->
```rust
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::{Context, IntoElement, Render, Window};

pub struct ControlledCheckbox {
    checked: bool,
}

impl ControlledCheckbox {
    pub fn new() -> Self {
        Self { checked: false }
    }

    pub fn is_checked(&self) -> bool {
        self.checked
    }
}

impl Render for ControlledCheckbox {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Checkbox::new("marketing-emails")
            .label("Receive product updates")
            .checked(self.checked)
            .on_change(cx.listener(|this, checked, _, cx| {
                this.checked = *checked;
                cx.notify();
            }))
    }
}
```
<!-- recipe:controlled-value:end -->

### 不同尺寸

```rust
use gpui_kit::component::Sizable as _;

Checkbox::new("cb").text_xs().label("Extra Small")
Checkbox::new("cb").text_sm().label("Small")
Checkbox::new("cb").label("Medium")
Checkbox::new("cb").text_lg().label("Large")
```

### 禁用状态

```rust
use gpui_kit::component::Disableable as _;

Checkbox::new("checkbox")
    .label("Disabled checkbox")
    .disabled(true)
    .checked(false)
```

### 不带标签

```rust
Checkbox::new("checkbox")
    .checked(true)
```

### 自定义 Tab 顺序

```rust
Checkbox::new("checkbox")
    .label("Custom tab order")
    .tab_index(2)
    .tab_stop(true)
```

## API 参考

- [Checkbox]

### 样式

实现了 `Sizable` 和 `Disableable` trait：

- `text_xs()`：超小字号
- `text_sm()`：小字号
- `text_base()`：默认字号
- `text_lg()`：大字号
- `disabled(bool)`：禁用状态

## 示例

### 复选框列表

```rust
v_flex()
    .gap_2()
    .child(Checkbox::new("cb1").label("Option 1").checked(true))
    .child(Checkbox::new("cb2").label("Option 2").checked(false))
    .child(Checkbox::new("cb3").label("Option 3").checked(false))
```

### 表单集成

```rust
struct FormView {
    agree_terms: bool,
    subscribe: bool,
}

v_flex()
    .gap_3()
    .child(
        Checkbox::new("terms")
            .label("I agree to the terms and conditions")
            .checked(self.agree_terms)
            .on_change(cx.listener(|view, checked, _, cx| {
                view.agree_terms = *checked;
                cx.notify();
            }))
    )
    .child(
        Checkbox::new("subscribe")
            .label("Subscribe to newsletter")
            .checked(self.subscribe)
            .on_change(cx.listener(|view, checked, _, cx| {
                view.subscribe = *checked;
                cx.notify();
            }))
    )
```

[Checkbox]: https://docs.rs/gpui-component/latest/gpui_component/checkbox/struct.Checkbox.html
