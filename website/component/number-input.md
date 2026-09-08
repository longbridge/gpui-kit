---
title: NumberInput
description: Number input component with increment/decrement controls and numeric formatting.
---

# NumberInput

A specialized input component for numeric values with built-in increment/decrement buttons and support for min/max values, step values, and number formatting with thousands separators.

## Import

```rust
use gpui_kit::component::input::{InputState, NumberInput, NumberInputEvent, StepAction};
```

## Usage

### Basic Number Input

```rust
let number_input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Enter number")
        .default_value("1")
);

NumberInput::new(&number_input)
```

### Input Restriction and Normalization

By default, the NumberInput only accepts a valid number: an optional leading
`+`/`-` sign, digits and a single decimal point (e.g. `-1.5`), other characters
are rejected on typing and pasting.

Full-width number characters are normalized into their ASCII equivalents
automatically, for CJK IME users:

- Full-width digits: `１２３` → `123`
- Full-width signs: `＋` → `+`, `－` → `-`
- Full-width dot and ideographic full stop: `．`, `。` → `.`

A bare leading decimal point is kept as-is (e.g. `.5`, parsed as `0.5`), matching the web behavior, so deleting the integer part of `1.2` keeps `.2` and stays editable.

To opt out of the default restriction, set an explicit mask:
`state.set_mask_pattern(MaskPattern::None, window, cx)`.

To further restrict the input (e.g. positive integers only), use `pattern`:

```rust
// Integer input with validation
let integer_input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Integer value")
        .pattern(Regex::new(r"^\d+$").unwrap()) // Only positive integers
);

NumberInput::new(&integer_input)
```

### With Min/Max/Step

By default, the NumberInput updates the value internally with `step(1.)`:
the `↑`/`↓` keys and the `+`/`-` buttons step the value by 1 and emit
`InputEvent::Change`. Set `min`/`max` to clamp the range, or set a custom step.

To fall back to emitting `NumberInputEvent::Step` only (the subscriber is
responsible for updating the value), call
`state.set_step(None, window, cx)`.

A typed out-of-range value is kept while typing, and clamped on blur.
Stepping follows the web behavior: a step that cannot move the value in the
pressed direction (e.g. `↓` on a value at or below the `min`) does nothing.

```rust
let stepper_input = cx.new(|cx|
    InputState::new(window, cx)
        .default_value("50")
        .step(5.)
        .min(0.)
        .max(100.)
);

NumberInput::new(&stepper_input)
```

### Dynamic Step

Use `step_by` to calculate the step value from the current value and the step
direction, e.g. a step size that varies by range. Because the step can differ
by direction at a boundary, the closure receives the `StepAction`; here `1.0`
steps by `0.1` going down and `0.5` going up. The closure also receives a
`Context` for reading or updating other entities:

```rust
let price_input = cx.new(|cx|
    InputState::new(window, cx)
        .step_by(|value, action, _cx| match action {
            StepAction::Increment => if value < 1.0 { 0.1 } else { 0.5 },
            StepAction::Decrement => if value <= 1.0 { 0.1 } else { 0.5 },
        })
        .min(0.)
);

NumberInput::new(&price_input)
```

The step strategy can also be updated at runtime via `set_step`:

```rust
use gpui_kit::component::input::NumberStep;

state.set_step(NumberStep::Fixed(0.01), window, cx);
state.set_step(NumberStep::by_value(|v, _, _cx| if v < 1. { 0.01 } else { 0.1 }), window, cx);
state.set_step(None, window, cx); // Fall back to NumberInputEvent::Step
```

### With Number Formatting

```rust
use gpui_kit::component::input::MaskPattern;

// Currency input with thousands separator
let currency_input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Amount")
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(2), // 2 decimal places
        })
);

NumberInput::new(&currency_input)
```

### Different Sizes

```rust
// Large size
NumberInput::new(&input).large()

// Medium size (default)
NumberInput::new(&input)

// Small size
NumberInput::new(&input).small()
```

### With Prefix and Suffix

```rust
use gpui_kit::component::{button::{Button, ButtonVariants}, IconName};

// With currency prefix
NumberInput::new(&input)
    .prefix(div().child("$"))

// With info button suffix
NumberInput::new(&input)
    .suffix(
        Button::new("info")
            .ghost()
            .icon(IconName::Info)
            .xsmall()
    )
```

### Disabled State

```rust
NumberInput::new(&input).disabled(true)
```

### Without Default Styling

```rust
// For custom container styling
div()
    .w_full()
    .bg(cx.theme().secondary)
    .rounded(cx.theme().radius)
    .child(NumberInput::new(&input).appearance(false))
```

### Handle Number Input Events

By default, the NumberInput updates the value internally. To fall back to
`NumberInputEvent::Step` (the subscriber is responsible for updating the
value), call `state.set_step(None, window, cx)`:

```rust
let number_input = cx.new(|cx| InputState::new(window, cx));
let mut value: i64 = 0;

// Subscribe to input changes
cx.subscribe_in(&number_input, window, |view, state, event, window, cx| {
    match event {
        InputEvent::Change => {
            let text = state.read(cx).value();
            if let Ok(new_value) = text.parse::<i64>() {
                view.value = new_value;
            }
        }
        _ => {}
    }
});

// Subscribe to increment/decrement actions
cx.subscribe_in(&number_input, window, |view, state, event, window, cx| {
    match event {
        NumberInputEvent::Step(step_action) => {
            match step_action {
                StepAction::Increment => {
                    view.value += 1;
                    state.update(cx, |input, cx| {
                        input.set_value(view.value.to_string(), window, cx);
                    });
                }
                StepAction::Decrement => {
                    view.value -= 1;
                    state.update(cx, |input, cx| {
                        input.set_value(view.value.to_string(), window, cx);
                    });
                }
            }
        }
    }
});
```

### Programmatic Control

```rust
// Increment programmatically
NumberInput::increment(&number_input, window, cx);

// Decrement programmatically
NumberInput::decrement(&number_input, window, cx);
```

## API Reference

### NumberInput

| Method                         | Description                                |
| ------------------------------ | ------------------------------------------ |
| `new(state)`                   | Create number input with InputState entity |
| `placeholder(str)`             | Set placeholder text                       |
| `size(size)`                   | Set input size (small, medium, large)      |
| `prefix(el)`                   | Add prefix element                         |
| `suffix(el)`                   | Add suffix element                         |
| `appearance(bool)`             | Enable/disable default styling             |
| `disabled(bool)`               | Set disabled state                         |
| `increment(state, window, cx)` | Increment value programmatically           |
| `decrement(state, window, cx)` | Decrement value programmatically           |

### NumberInputEvent

| Event              | Description                        |
| ------------------ | ---------------------------------- |
| `Step(StepAction)` | Increment/decrement pressed. Only emitted when `step` is `None` (opt out via `set_step(None, ...)`). |

### StepAction

| Action      | Description               |
| ----------- | ------------------------- |
| `Increment` | Value should be increased |
| `Decrement` | Value should be decreased |

### InputState (Number-specific methods)

| Method                              | Description                                             |
| ----------------------------------- | ------------------------------------------------------- |
| `step(impl Into<NumberStep>)`       | Set step value for built-in increment/decrement (default: 1) |
| `step_by(fn(f64, StepAction, &mut Context) -> f64)` | Calculate step value based on the current value and direction |
| `min(f64)`                          | Set minimum value, clamped on stepping and blur          |
| `max(f64)`                          | Set maximum value, clamped on stepping and blur          |
| `set_step(Option<NumberStep>, ...)` | Update step strategy after construction                  |
| `set_min(Option<f64>, ...)`         | Update minimum value after construction                  |
| `set_max(Option<f64>, ...)`         | Update maximum value after construction                  |
| `pattern(regex)`                    | Set regex pattern for validation (e.g., digits only)    |
| `mask_pattern(MaskPattern::Number)` | Set number formatting with separator and decimal places |
| `value()`                           | Get current display value (formatted)                   |
| `unmask_value()`                    | Get actual numeric value (unformatted)                  |

### MaskPattern::Number

| Field       | Type            | Description                            |
| ----------- | --------------- | -------------------------------------- |
| `separator` | `Option<char>`  | Thousands separator (e.g., ',' or ' ') |
| `fraction`  | `Option<usize>` | Number of decimal places               |

## Keyboard Navigation

| Key         | Action                     |
| ----------- | -------------------------- |
| `↑`         | Increment value            |
| `↓`         | Decrement value            |
| `Tab`       | Navigate to next field     |
| `Shift+Tab` | Navigate to previous field |
| `Enter`     | Submit/confirm value       |
| `Escape`    | Clear input (if enabled)   |

## Examples

### Integer Counter

```rust
struct CounterView {
    counter_input: Entity<InputState>,
    counter_value: i32,
}

impl CounterView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let counter_input = cx.new(|cx|
            InputState::new(window, cx)
                .placeholder("Count")
                .default_value("0")
                .pattern(Regex::new(r"^-?\d+$").unwrap()) // Allow negative integers
        );

        let _subscription = cx.subscribe_in(&counter_input, window, Self::on_number_event);

        Self {
            counter_input,
            counter_value: 0,
        }
    }

    fn on_number_event(
        &mut self,
        state: &Entity<InputState>,
        event: &NumberInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            NumberInputEvent::Step(StepAction::Increment) => {
                self.counter_value += 1;
                state.update(cx, |input, cx| {
                    input.set_value(self.counter_value.to_string(), window, cx);
                });
            }
            NumberInputEvent::Step(StepAction::Decrement) => {
                self.counter_value -= 1;
                state.update(cx, |input, cx| {
                    input.set_value(self.counter_value.to_string(), window, cx);
                });
            }
        }
    }
}

// Usage
NumberInput::new(&self.counter_input)
```

### Currency Input

```rust
struct PriceInput {
    price_input: Entity<InputState>,
    price_value: f64,
}

impl PriceInput {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let price_input = cx.new(|cx|
            InputState::new(window, cx)
                .placeholder("0.00")
                .mask_pattern(MaskPattern::Number {
                    separator: Some(','),
                    fraction: Some(2),
                })
        );

        Self {
            price_input,
            price_value: 0.0,
        }
    }
}

// Usage with currency prefix
h_flex()
    .gap_2()
    .child(div().child("$"))
    .child(NumberInput::new(&self.price_input))
```

### Quantity Selector with Limits

```rust
struct QuantitySelector {
    quantity_input: Entity<InputState>,
}

impl QuantitySelector {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Step by 1 and clamp to 1..=99, no event handling needed.
        let quantity_input = cx.new(|cx|
            InputState::new(window, cx)
                .default_value("1")
                .min(1.)
                .max(99.)
        );

        Self { quantity_input }
    }
}

// Usage
NumberInput::new(&self.quantity_input).small()
```

### Floating Point Input

```rust
// Step by 0.1, the fraction digits of the value are kept on stepping,
// e.g. 0.2 -> 0.3 (not 0.30000000000000004).
let float_input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("0.0")
        .step(0.1)
);

NumberInput::new(&float_input)
```

## Best Practices

1. **Validation**: Always validate numeric input on both client and server side
2. **Range Limits**: Use `min`/`max` to clamp values for user safety
3. **Step Size**: Choose appropriate `step` values for your use case
4. **Error Handling**: Provide clear feedback for invalid input
5. **Formatting**: Use consistent number formatting across your application
6. **Performance**: Debounce rapid increment/decrement actions if needed
7. **Accessibility**: Always provide proper labels and descriptions
