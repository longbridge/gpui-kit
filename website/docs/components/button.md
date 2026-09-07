---
title: Button
description: Displays a button or a component that looks like a button.
---

# Button

The [Button] element with multiple variants, sizes, and states. Supports icons, loading states, and can be grouped together.

## Import

```rust
use gpui_kit::component::button::{Button, ButtonGroup};
```

## Usage

### Basic Button

```rust
Button::new("my-button")
    .label("Click me")
    .on_click(|_, _, _| {
        println!("Button clicked!");
    })
```

### Variants

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

### Outline Buttons

Outline style is not a variant itself, but can be combined with other variants.

```rust
Button::new("btn").primary().outline().label("Primary Outline")
Button::new("btn").danger().outline().label("Danger Outline")
```

### Compact Button

The `compact` method reduces the padding of the button for a more condensed appearance.

```rust
// Compact (reduced padding)
Button::new("btn")
    .label("Compact")
    .compact()
```

### Sizeable

The Button supports the [Sizable] trait for different sizes.

```rust
Button::new("btn").xsmall().label("Extra Small")
Button::new("btn").small().label("Small")
Button::new("btn").label("Medium") // default
Button::new("btn").large().label("Large")
```

### With Icons

The `icon` method supports multiple types, allowing you to use different visual indicators:

- **[Icon] / [IconName]** - Static icons for actions and visual cues
- **[Spinner]** - Animated loading indicator for async operations
- **[ProgressCircle]** - Circular progress indicator showing completion percentage

All icon types automatically adapt to the button's size and can be customized with colors and other properties.

#### Icon Types

```rust
use gpui_kit::component::{Icon, IconName};

// Using IconName (simplest)
Button::new("btn")
    .icon(IconName::Check)
    .label("Confirm")

// Using Icon with custom size
Button::new("btn")
    .icon(Icon::new(IconName::Heart))
    .label("Like")

// Icon only (no label)
Button::new("btn")
    .icon(IconName::Search)
```

#### Spinner Icon

Use a [Spinner] to indicate loading or processing state:

```rust
use gpui_kit::component::spinner::Spinner;

// Basic spinner
Button::new("btn")
    .icon(Spinner::new())
    .label("Loading...")

// Spinner with custom color
Button::new("btn")
    .icon(Spinner::new().color(cx.theme().blue))
    .label("Processing")

// Spinner with icon
Button::new("btn")
    .icon(Spinner::new().icon(IconName::LoaderCircle))
    .label("Syncing")
```

#### ProgressCircle Icon

Use a [ProgressCircle] to show progress percentage:

```rust
use gpui_kit::component::progress::ProgressCircle;

// Basic progress circle
Button::new("btn")
    .icon(ProgressCircle::new("install-progress").value(45.0))
    .label("Installing...")

// Progress circle with custom color
Button::new("btn")
    .primary()
    .icon(
        ProgressCircle::new("download-progress")
            .value(75.0)
            .color(cx.theme().primary_foreground)
    )
    .label("Downloading")

// Different sizes
Button::new("btn")
    .small()
    .icon(ProgressCircle::new("progress-1").value(60.0))
    .label("Installing...")

Button::new("btn")
    .large()
    .icon(ProgressCircle::new("progress-2").value(80.0))
    .label("Installing...")
```

#### Dynamic Icon Updates

Icons can be updated dynamically based on component state:

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

#### Loading State with Icons

When a button is in loading state, it automatically handles icon transitions:

```rust
// If icon is already a Spinner or ProgressCircle, it will be shown during loading
Button::new("btn")
    .icon(Spinner::new())
    .label("Processing")
    .loading(true) // Spinner will continue to show

// If icon is a regular Icon, it will be replaced with a Spinner during loading
Button::new("btn")
    .icon(IconName::Save)
    .label("Saving")
    .loading(true) // Icon will be replaced with Spinner
```

### With a dropdown caret icon

The `.dropdown_caret` method can allows adding a dropdown caret icon to end of the button.

```rust
Button::new("btn")
    .label("Options")
    .dropdown_caret(true)
```

### Button States

There have `disabled`, `loading`, `selected` state for buttons to indicate different statuses.

```rust
// Disabled
Button::new("btn")
    .label("Disabled")
    .disabled(true)

// Loading
Button::new("btn")
    .label("Loading")
    .loading(true)

// Selected
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

### Toggle Button Group

```rust
ButtonGroup::new("toggle-group")
    .multiple(true) // Allow multiple selections
    .child(Button::new("btn1").label("Option 1").selected(true))
    .child(Button::new("btn2").label("Option 2"))
    .child(Button::new("btn3").label("Option 3"))
    .on_click(|selected_indices, _, _| {
        println!("Selected: {:?}", selected_indices);
    })
```

## Custom Variant

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

## API Reference

- [Button]
- [ButtonGroup]
- [ButtonCustomVariant]

## Examples

### With Tooltip

```rust
Button::new("btn")
    .label("Hover me")
    .tooltip("This is a helpful tooltip")
    .tooltip_placement(Placement::Bottom)
```

Use `.tooltip_placement(...)` to prefer a side for either `.tooltip(...)` or
`.tooltip_with_action(...)`. The tooltip still flips when that side does not fit.
Omit placement to keep automatic positioning.

### Custom Children

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
