---
title: Radio
description: A set of checkable buttons—known as radio buttons—where no more than one of the buttons can be checked at a time.
---

# Radio

Radio buttons allow users to select a single option from a set of mutually exclusive choices. Use radio buttons when you want to give users a choice between multiple options and only one selection is allowed.

Use `on_change` for requested values. The owner stores the value and calls `cx.notify()`. The existing `on_click` name remains a compatibility alias; setting either replaces the same handler, so the last call wins.

## Import

```rust
use gpui_kit::component::radio::{Radio, RadioGroup};
```

## Usage

### Basic Radio Button

```rust
Radio::new("radio-option-1")
    .label("Option 1")
    .checked(false)
    .on_change(|checked, _, _| {
        println!("Radio is now: {}", checked);
    })
```

### Controlled Radio Button

```rust
struct MyView {
    radio_checked: bool,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Radio::new("radio")
            .label("Select this option")
            .checked(self.radio_checked)
            .on_change(cx.listener(|view, checked, _, cx| {
                view.radio_checked = *checked;
                cx.notify();
            }))
    }
}
```

### Radio Group (Recommended)

```rust
struct MyView {
    selected_option: Option<usize>,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        RadioGroup::horizontal("options")
            .children(["Option 1", "Option 2", "Option 3"])
            .selected_index(self.selected_option)
            .on_change(cx.listener(|view, selected_index: &usize, _, cx| {
                view.selected_option = Some(*selected_index);
                cx.notify();
            }))
    }
}
```

### Different Sizes

```rust
Radio::new("small").label("Small").xsmall()
Radio::new("medium").label("Medium") // default
Radio::new("large").label("Large").large()
```

### Disabled State

```rust
Radio::new("disabled")
    .label("Disabled option")
    .disabled(true)
    .checked(false)

Radio::new("disabled-checked")
    .label("Disabled and checked")
    .checked(true)
    .disabled(true)
```

### Multi-line Label with Custom Content

```rust
Radio::new("custom")
    .label("Primary option")
    .child(
        div()
            .text_color(cx.theme().muted_foreground)
            .child("This is additional descriptive text that provides more context.")
    )
    .w(px(300.))
```

### Custom Tab Order

```rust
Radio::new("radio")
    .label("Custom tab order")
    .tab_index(2)
    .tab_stop(true)
```

## Radio Group Usage

### Horizontal Layout

```rust
RadioGroup::horizontal("horizontal-group")
    .children(["First", "Second", "Third"])
    .selected_index(Some(0))
    .on_change(cx.listener(|view, index, _, cx| {
        println!("Selected index: {}", index);
        cx.notify();
    }))
```

### Vertical Layout

```rust
RadioGroup::vertical("vertical-group")
    .child(Radio::new("option1").label("United States"))
    .child(Radio::new("option2").label("Canada"))
    .child(Radio::new("option3").label("Mexico"))
    .selected_index(Some(1))
    .disabled(false)
```

### Styled Radio Group

```rust
RadioGroup::vertical("styled-group")
    .w(px(220.))
    .p_2()
    .border_1()
    .border_color(cx.theme().border)
    .rounded(cx.theme().radius)
    .child(Radio::new("option1").label("Option 1"))
    .child(Radio::new("option2").label("Option 2"))
    .child(Radio::new("option3").label("Option 3"))
    .selected_index(Some(0))
```

### Disabled Radio Group

```rust
RadioGroup::vertical("disabled-group")
    .children(["Option A", "Option B", "Option C"])
    .selected_index(Some(1))
    .disabled(true) // Disables all radio buttons in the group
```

## API Reference

### Radio

| Method             | Description                                                 |
| ------------------ | ----------------------------------------------------------- |
| `new(id)`          | Create a new radio button with the given ID                 |
| `label(text)`      | Set label text                                              |
| `checked(bool)`    | Set checked state                                           |
| `disabled(bool)`   | Set disabled state                                          |
| `on_change(fn)`     | Requested checked value, receives `&bool` |
| `tab_stop(bool)`   | Enable/disable tab navigation (default: true)               |
| `tab_index(isize)` | Set tab order index (default: 0)                            |

### RadioGroup

| Method                          | Description                                                         |
| ------------------------------- | ------------------------------------------------------------------- |
| `new(id)` | Create a vertical radio group with no selection |
| `horizontal(id)`                | Create a new horizontal radio group                                 |
| `vertical(id)`                  | Create a new vertical radio group                                   |
| `layout(Axis)`                  | Set layout direction (Vertical or Horizontal)                       |
| `child(Radio)`                  | Add a single radio button to the group                              |
| `children(items)`               | Add multiple radio buttons from an iterator                         |
| `selected_index(Option<usize>)` | Set the selected option by index                                    |
| `disabled(bool)`                | Disable all radio buttons in the group                              |
| `on_change(fn)`                 | Requested selected index, receives `&usize` |

### Styling

Both Radio and RadioGroup implement `Styled` trait for custom styling:

Radio also implements `Sizable` trait:

- `xsmall()` - Extra small size
- `small()` - Small size
- `medium()` - Medium size (default)
- `large()` - Large size

## Examples

### Settings Panel

```rust
struct SettingsView {
    theme: Option<usize>, // 0: Light, 1: Dark, 2: Auto
    language: Option<usize>, // 0: English, 1: Spanish, 2: French
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                v_flex()
                    .gap_2()
                    .child(div().text_sm().font_semibold().child("Theme"))
                    .child(
                        RadioGroup::vertical("theme")
                            .child(Radio::new("light").label("Light"))
                            .child(Radio::new("dark").label("Dark"))
                            .child(Radio::new("auto").label("Auto"))
                            .selected_index(self.theme)
                            .on_change(cx.listener(|view, index, _, cx| {
                                view.theme = Some(*index);
                                cx.notify();
                            }))
                    )
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(div().text_sm().font_semibold().child("Language"))
                    .child(
                        RadioGroup::horizontal("language")
                            .children(["English", "Español", "Français"])
                            .selected_index(self.language)
                            .on_change(cx.listener(|view, index, _, cx| {
                                view.language = Some(*index);
                                cx.notify();
                            }))
                    )
            )
    }
}
```

### Survey Form

```rust
struct SurveyView {
    satisfaction: Option<usize>,
    recommendation: Option<usize>,
}

impl Render for SurveyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_8()
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .text_base()
                            .font_medium()
                            .child("How satisfied are you with our service?")
                    )
                    .child(
                        RadioGroup::vertical("satisfaction")
                            .child(Radio::new("very-satisfied").label("Very satisfied"))
                            .child(Radio::new("satisfied").label("Satisfied"))
                            .child(Radio::new("neutral").label("Neutral"))
                            .child(Radio::new("dissatisfied").label("Dissatisfied"))
                            .child(Radio::new("very-dissatisfied").label("Very dissatisfied"))
                            .selected_index(self.satisfaction)
                            .on_change(cx.listener(|view, index, _, cx| {
                                view.satisfaction = Some(*index);
                                cx.notify();
                            }))
                    )
            )
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .text_base()
                            .font_medium()
                            .child("How likely are you to recommend us?")
                    )
                    .child(
                        RadioGroup::horizontal("recommendation")
                            .children((0..=10).map(|i| i.to_string()))
                            .selected_index(self.recommendation)
                            .on_change(cx.listener(|view, index, _, cx| {
                                view.recommendation = Some(*index);
                                cx.notify();
                            }))
                    )
            )
    }
}
```

### Payment Method Selection

```rust
struct PaymentView {
    payment_method: Option<usize>,
}

impl Render for PaymentView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                div()
                    .text_lg()
                    .font_semibold()
                    .child("Select Payment Method")
            )
            .child(
                RadioGroup::vertical("payment")
                    .child(
                        Radio::new("credit-card")
                            .label("Credit Card")
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Visa, MasterCard, American Express")
                            )
                    )
                    .child(
                        Radio::new("paypal")
                            .label("PayPal")
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Pay with your PayPal account")
                            )
                    )
                    .child(
                        Radio::new("bank-transfer")
                            .label("Bank Transfer")
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Direct bank account transfer")
                            )
                    )
                    .selected_index(self.payment_method)
                    .on_change(cx.listener(|view, index, _, cx| {
                        view.payment_method = Some(*index);
                        cx.notify();
                    }))
            )
    }
}
```

## Best Practices

1. **Use RadioGroup**: Always prefer `RadioGroup` over individual `Radio` components for mutually exclusive choices
2. **Clear Labels**: Provide descriptive labels that clearly indicate what each option represents
3. **Default Selection**: Consider providing a sensible default selection, especially for required fields
4. **Logical Order**: Arrange options in a logical order (alphabetical, frequency of use, or importance)
5. **Limit Options**: Keep the number of radio options reasonable (typically 2-7 options)
6. **Group Related Options**: Use visual grouping and clear headings for multiple radio groups
7. **Responsive Design**: Consider using horizontal layout for fewer options and vertical for more options
