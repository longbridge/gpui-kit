---
title: Checkbox
description: A control that allows the user to toggle between checked and not checked.
---

# Checkbox

A checkbox component for binary choices. Supports labels, disabled state, and different sizes.

Use `on_change` for requested values. The owner stores the value and calls `cx.notify()`. The existing `on_click` name remains a compatibility alias; setting either replaces the same handler, so the last call wins.

## Import

```rust
use gpui_kit::component::checkbox::Checkbox;
```

## Usage

### Basic Checkbox

```rust
Checkbox::new("my-checkbox")
    .label("Accept terms and conditions")
    .checked(false)
    .on_change(|checked, _, _| {
        println!("Checkbox is now: {}", checked);
    })
```

The `on_change` callback is triggered when the user toggles the checkbox, receiving the **new checked state**.

### Controlled Checkbox

```rust
struct MyView {
    is_checked: bool,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Checkbox::new("checkbox")
            .label("Option")
            .checked(self.is_checked)
            .on_change(cx.listener(|view, checked, _, cx| {
                view.is_checked = *checked;
                cx.notify();
            }))
    }
}
```

### Different Sizes

```rust
Checkbox::new("cb").text_xs().label("Extra Small")
Checkbox::new("cb").text_sm().label("Small")
Checkbox::new("cb").label("Medium") // default
Checkbox::new("cb").text_lg().label("Large")
```

### Disabled State

```rust
Checkbox::new("checkbox")
    .label("Disabled checkbox")
    .disabled(true)
    .checked(false)
```

### Without Label

```rust
Checkbox::new("checkbox")
    .checked(true)
```

### Custom Tab Order

```rust
Checkbox::new("checkbox")
    .label("Custom tab order")
    .tab_index(2)
    .tab_stop(true)
```

## API Reference

- [Checkbox]

### Styling

Implements `Sizable` and `Disableable` traits:

- `text_xs()` - Extra small text
- `text_sm()` - Small text
- `text_base()` - Base text (default)
- `text_lg()` - Large text
- `disabled(bool)` - Disabled state

## Examples

### Checkbox List

```rust
v_flex()
    .gap_2()
    .child(Checkbox::new("cb1").label("Option 1").checked(true))
    .child(Checkbox::new("cb2").label("Option 2").checked(false))
    .child(Checkbox::new("cb3").label("Option 3").checked(false))
```

### Form Integration

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
