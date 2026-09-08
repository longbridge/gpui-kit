---
title: Input
description: Text input component with validation, masking, and various features.
---

# Input

A single-line text input with validation, masking, prefix/suffix elements, and
different visual states. Use [Textarea](./textarea.md) for ordinary multi-line
text and [Editor](./editor.md) for source code.

## Import

```rust
use gpui_kit::component::input::{Input, InputState};
```

## Usage

### Basic Input

```rust
let input = cx.new(|cx| InputState::new(window, cx));

Input::new(&input)
```

### With Placeholder

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Enter your name...")
);

Input::new(&input)
```

### With Default Value

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .default_value("John Doe")
);

Input::new(&input)
```

### Cleanable Input

```rust
Input::new(&input)
    .cleanable(true) // Show clear button when input has value
```

### With Prefix and Suffix

```rust
use gpui_kit::component::{Icon, IconName};

// With prefix icon
Input::new(&input)
    .prefix(Icon::new(IconName::Search).small())

// With suffix button
Input::new(&input)
    .suffix(
        Button::new("info")
            .ghost()
            .icon(IconName::Info)
            .xsmall()
    )

// With both
Input::new(&input)
    .prefix(Icon::new(IconName::Search).small())
    .suffix(Button::new("btn").ghost().icon(IconName::Info).xsmall())
```

### Password Input (Masked)

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .masked(true)
        .default_value("password123")
);

Input::new(&input)
    .content_type(InputContentType::Password)
    .mask_toggle() // Shows toggle button to reveal password
```

While the value is masked, the input keeps it out of the clipboard and out of
the selection: Copy and Cut do nothing (and are disabled in the context menu),
a word-wise delete takes everything before the caret, and a double click
selects the whole value instead of one word. Paste and Select All keep working,
and revealing the value with `mask_toggle` restores all of them.

### Input Sizes

```rust
Input::new(&input).large()
Input::new(&input) // medium (default)
Input::new(&input).small()
```

### Disabled Input

```rust
Input::new(&input).disabled(true)
```

### Read-only Input

Unlike `disabled`, a read-only input keeps the normal appearance and still can
be focused, selected and copied, it only rejects the changes made by the user.

```rust
Input::new(&input).readonly(true)
```

### Clean on ESC

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .clean_on_escape() // Clear input when ESC is pressed
);

Input::new(&input)
```

### Input Validation

```rust
// Validate float numbers
let input = cx.new(|cx|
    InputState::new(window, cx)
        .validate(|s, _| s.parse::<f32>().is_ok())
);

// Regex pattern validation
let input = cx.new(|cx|
    InputState::new(window, cx)
        .pattern(regex::Regex::new(r"^[a-zA-Z0-9]*$").unwrap())
);
```

### Input Masking

```rust
// Phone number mask
let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern("(999)-999-9999")
);

// Custom pattern: AAA-###-AAA (A=letter, #=digit, 9=digit optional)
let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern("AAA-###-AAA")
);

// Number with thousands separator
use gpui_kit::component::input::MaskPattern;

let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(3),
        })
);
```

### Handle Input Events

```rust
let input = cx.new(|cx| InputState::new(window, cx));

cx.subscribe_in(&input, window, |view, state, event, window, cx| {
    match event {
        InputEvent::Change => {
            let text = state.read(cx).value();
            println!("Input changed: {}", text);
        }
        InputEvent::PressEnter { secondary } => {
            println!("Enter pressed, secondary: {}", secondary);
        }
        InputEvent::Focus => println!("Input focused"),
        InputEvent::Blur => println!("Input blurred"),
    }
});
```

### Custom Appearance

```rust
// Without default styling
Input::new(&input).appearance(false)

// Use in custom container
div()
    .border_b_2()
    .px_6()
    .py_3()
    .border_color(cx.theme().border)
    .bg(cx.theme().secondary)
    .child(Input::new(&input).appearance(false))
```

### Context Menu

```rust
// The built-in context menu can be disabled.
let input = cx.new(|cx| InputState::new(window, cx).context_menu(false));

// Or you can define a custom context menu.
Input::new(&input).context_menu(|menu, window, cx| {
    // You can define your own actions and even utilize
    // built-in actions (cut, copy, paste, etc.)
    // to avoid having to re-implement that functionality.
    menu.menu("Custom Action", Box::new(CustomAction))
        .separator()
        .menu("Cut", Box::new(input::Cut))
        .menu("Copy", Box::new(input::Copy))
        .menu("Paste", Box::new(input::Paste))
})
```

## Examples

### Search Input

```rust
let search = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Search...")
);

Input::new(&search)
    .prefix(Icon::new(IconName::Search).small())
```

### Currency Input

```rust
let amount = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(2),
        })
);

div()
    .child(Input::new(&amount))
    .child(format!("Value: {}", amount.read(cx).value()))
```

### Form with Multiple Inputs

```rust
struct FormView {
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
}

v_flex()
    .gap_3()
    .child(Input::new(&self.name_input))
    .child(Input::new(&self.email_input))
```
