---
title: Tooltip
description: Display helpful information on hover or focus, with support for keyboard shortcuts and custom content.
---

# Tooltip

A versatile tooltip component that displays helpful information when hovering over or focusing on elements. Supports text content, custom elements, keyboard shortcuts, different trigger methods, and positioning options.

## Import

```rust
use gpui_kit::component::tooltip::Tooltip;
```

## Usage

### Basic Tooltip with Text

```rust
// Simple text tooltip
div()
    .child("Hover me")
    .id("basic-tooltip")
    .tooltip(|window, cx| {
        Tooltip::new("This is a helpful tooltip").build(window, cx)
    })
```

### Button with Tooltip

```rust
Button::new("save-btn")
    .label("Save")
    .tooltip("Save the current document")
```

### Tooltip with Action/Keybinding

```rust
actions!(my_actions, [SaveDocument]);

Button::new("save-btn")
    .label("Save")
    .tooltip_with_action(
        "Save the current document",
        &SaveDocument,
        Some("MyContext")
    )
```

### Custom Element Tooltip

```rust
div()
    .child("Hover for rich content")
    .id("rich-tooltip")
    .tooltip(|window, cx| {
        Tooltip::element(|_, cx| {
            h_flex()
                .gap_x_1()
                .child(IconName::Info)
                .child(
                    div()
                        .child("Muted Text")
                        .text_color(cx.theme().muted_foreground)
                )
                .child(
                    div()
                        .child("Danger Text")
                        .text_color(cx.theme().danger)
                )
                .child(IconName::ArrowUp)
        })
        .build(window, cx)
    })
```

### Tooltip with Manual Keybinding

```rust
div()
    .child("Custom keybinding")
    .id("custom-kb")
    .tooltip(|window, cx| {
        Tooltip::new("Delete item")
            .key_binding(Some(Kbd::new("Delete")))
            .build(window, cx)
    })
```

## Advanced Usage

### Components with Built-in Tooltip Support

Many components have built-in tooltip methods:

```rust
// Button
Button::new("btn")
    .label("Click me")
    .tooltip("This button performs an action")

// Switch
Switch::new("toggle")
    .label("Enable notifications")
    .tooltip("Toggle push notifications on/off")

// Checkbox
Checkbox::new("check")
    .label("Remember me")
    .tooltip("Keep me logged in for 30 days")

// Radio
Radio::new("option")
    .label("Option 1")
    .tooltip("Select this option to enable feature X")
```

### Complex Tooltip Content

```rust
div()
    .child("Hover for details")
    .id("complex-tooltip")
    .tooltip(|window, cx| {
        Tooltip::element(|_, cx| {
            v_flex()
                .gap_2()
                .child(
                    h_flex()
                        .gap_1()
                        .child(IconName::User)
                        .child("User Information")
                        .text_sm()
                        .font_semibold()
                )
                .child(
                    div()
                        .child("Last login: 2 hours ago")
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                )
                .child(
                    div()
                        .child("Status: Active")
                        .text_xs()
                        .text_color(cx.theme().success)
                )
        })
        .build(window, cx)
    })
```

### Tooltip in Form Elements

```rust
v_flex()
    .gap_4()
    .child(
        Input::new("email")
            .placeholder("Enter your email")
            .tooltip("We'll never share your email address")
    )
    .child(
        Input::new("password")
            .input_type(InputType::Password)
            .placeholder("Password")
            .tooltip("Must be at least 8 characters with special characters")
    )
```

## API Reference

### Tooltip

| Method                    | Description                                  |
| ------------------------- | -------------------------------------------- |
| `new(text)`               | Create a tooltip with text content           |
| `element(builder)`        | Create a tooltip with custom element content |
| `action(action, context)` | Set action to display keybinding information |
| `key_binding(kbd)`        | Set manual keybinding information            |
| `build(window, cx)`       | Build and return the tooltip as AnyView      |

### Built-in Tooltip Methods

Components with tooltip support typically provide these methods:

| Method                                       | Description                             |
| -------------------------------------------- | --------------------------------------- |
| `tooltip(text)`                              | Add simple text tooltip                 |
| `tooltip_with_action(text, action, context)` | Add tooltip with action keybinding      |
| `tooltip(closure)`                           | Add custom tooltip with builder closure |

### Tooltip Styling

The tooltip automatically applies theme-appropriate styling:

- Background: `theme.popover`
- Text color: `theme.popover_foreground`
- Border: `theme.border`
- Shadow: Medium drop shadow
- Border radius: 6px
- Font: System UI font

You can apply additional styling using the `Styled` trait:

```rust
Tooltip::new("Custom styled tooltip")
    .bg(cx.theme().accent)
    .text_color(cx.theme().accent_foreground)
    .build(window, cx)
```

## Examples

### Toolbar with Tooltips

```rust
h_flex()
    .gap_1()
    .child(
        Button::new("new")
            .icon(IconName::Plus)
            .tooltip_with_action("Create new file", &NewFile, Some("Editor"))
    )
    .child(
        Button::new("open")
            .icon(IconName::FolderOpen)
            .tooltip_with_action("Open file", &OpenFile, Some("Editor"))
    )
    .child(
        Button::new("save")
            .icon(IconName::Save)
            .tooltip_with_action("Save file", &SaveFile, Some("Editor"))
    )
```

### Status Indicators with Tooltips

```rust
h_flex()
    .gap_2()
    .child(
        div()
            .size_3()
            .rounded_full()
            .bg(cx.theme().success)
            .tooltip(|window, cx| {
                Tooltip::new("Connected to server").build(window, cx)
            })
    )
    .child(
        div()
            .size_3()
            .rounded_full()
            .bg(cx.theme().warning)
            .tooltip(|window, cx| {
                Tooltip::new("Limited connectivity").build(window, cx)
            })
    )
```

### Interactive Elements with Rich Tooltips

```rust
v_flex()
    .gap_3()
    .child(
        div()
            .p_2()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child("File: document.txt")
            .id("file-item")
            .tooltip(|window, cx| {
                Tooltip::element(|_, cx| {
                    v_flex()
                        .gap_1()
                        .child(
                            h_flex()
                                .gap_2()
                                .child(IconName::File)
                                .child("document.txt")
                                .text_sm()
                                .font_medium()
                        )
                        .child(
                            div()
                                .child("Size: 2.4 KB")
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                        )
                        .child(
                            div()
                                .child("Modified: 2 hours ago")
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                        )
                        .child(
                            h_flex()
                                .gap_1()
                                .child(Kbd::new("Enter"))
                                .child("to open")
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                        )
                })
                .build(window, cx)
            })
    )
```

### Form Validation with Tooltips

```rust
struct FormView {
    email_error: Option<String>,
    password_error: Option<String>,
}

v_flex()
    .gap_4()
    .child(
        Input::new("email")
            .placeholder("Email address")
            .when_some(self.email_error.clone(), |this, error| {
                this.tooltip(move |window, cx| {
                    Tooltip::element(|_, cx| {
                        h_flex()
                            .gap_1()
                            .child(IconName::AlertCircle)
                            .child(error.clone())
                            .text_color(cx.theme().destructive)
                    })
                    .build(window, cx)
                })
            })
    )
```

## Best Practices

### Content Guidelines

- **Be concise**: Keep tooltip text short and to the point
- **Be helpful**: Provide additional context, not redundant information
- **Use proper tone**: Match your application's voice and tone
- **Avoid critical info**: Don't put essential information only in tooltips

### Usage Guidelines

- **Progressive disclosure**: Use tooltips for additional context, not primary information
- **Consistency**: Use consistent tooltip patterns throughout your application
- **Performance**: Avoid complex content in frequently triggered tooltips
- **Testing**: Test tooltips with both mouse and keyboard interaction

### Examples of Good Tooltip Content

```rust
// Good: Provides helpful context
Button::new("delete")
    .icon(IconName::Trash)
    .tooltip("Delete this item permanently")

// Good: Explains abbreviation
div()
    .child("CPU: 45%")
    .tooltip("Central Processing Unit usage")

// Good: Describes action with keybinding
Button::new("undo")
    .icon(IconName::Undo)
    .tooltip_with_action("Undo last action", &Undo, Some("Editor"))
```

### Examples to Avoid

```rust
// Avoid: Redundant information
Button::new("save")
    .label("Save")
    .tooltip("Save") // Doesn't add value

// Avoid: Critical information
Button::new("delete")
    .tooltip("This will permanently delete all your files") // Too important for tooltip only
```
