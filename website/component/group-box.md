---
title: GroupBox
description: A styled container element with an optional title to group related content together.
---

# GroupBox

The GroupBox component is a versatile container that groups related content together with optional borders, backgrounds, and titles. It provides visual organization and semantic grouping for form controls, settings panels, and other related UI elements.

## Import

```rust
use gpui_kit::component::group_box::{GroupBox, GroupBoxVariant, GroupBoxVariants as _};
```

## Usage

### Basic GroupBox

```rust
GroupBox::new()
    .child("Subscriptions")
    .child(Checkbox::new("all").label("All"))
    .child(Checkbox::new("newsletter").label("Newsletter"))
    .child(Button::new("save").primary().label("Save"))
```

### GroupBox Variants

```rust
// Normal variant (default) - no background or border
GroupBox::new()
    .child("Content without visual container")

// Fill variant - with background color
GroupBox::new()
    .fill()
    .title("Settings")
    .child("Content with background")

// Outline variant - with border, no background
GroupBox::new()
    .outline()
    .title("Preferences")
    .child("Content with border")
```

### With Title

```rust
GroupBox::new()
    .fill()
    .title("Account Settings")
    .child(
        h_flex()
            .justify_between()
            .child("Make profile private")
            .child(Switch::new("privacy").checked(false))
    )
    .child(Button::new("save").primary().label("Save Changes"))
```

### Custom ID

```rust
GroupBox::new()
    .id("user-preferences")
    .outline()
    .title("User Preferences")
    .child("Preference controls...")
```

### Custom Title Styling

```rust
use gpui_kit::{StyleRefinement, relative};

GroupBox::new()
    .outline()
    .title("Custom Title")
    .title_style(
        StyleRefinement::default()
            .font_semibold()
            .line_height(relative(1.0))
            .px_3()
            .text_color(cx.theme().accent)
    )
    .child("Content with custom title styling")
```

### Custom Content Styling

```rust
GroupBox::new()
    .fill()
    .title("Custom Content Area")
    .content_style(
        StyleRefinement::default()
            .rounded_xl()
            .py_3()
            .px_4()
            .border_2()
            .border_color(cx.theme().accent)
    )
    .child("Content with custom styling")
```

### Complex Example

```rust
GroupBox::new()
    .id("notification-settings")
    .outline()
    .bg(cx.theme().group_box)
    .rounded_xl()
    .p_5()
    .title("Notification Preferences")
    .title_style(
        StyleRefinement::default()
            .font_semibold()
            .line_height(relative(1.0))
            .px_3()
    )
    .content_style(
        StyleRefinement::default()
            .rounded_xl()
            .py_3()
            .px_4()
            .border_2()
    )
    .child(
        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .child("Email notifications")
                    .child(Switch::new("email").checked(true))
            )
            .child(
                h_flex()
                    .justify_between()
                    .child("Push notifications")
                    .child(Switch::new("push").checked(false))
            )
            .child(
                h_flex()
                    .justify_between()
                    .child("SMS notifications")
                    .child(Switch::new("sms").checked(false))
            )
    )
    .child(
        h_flex()
            .justify_end()
            .gap_2()
            .child(Button::new("cancel").label("Cancel"))
            .child(Button::new("save").primary().label("Save Settings"))
    )
```

## Examples

### Form Section

```rust
GroupBox::new()
    .fill()
    .title("Personal Information")
    .child(
        v_flex()
            .gap_4()
            .child(
                h_flex()
                    .gap_2()
                    .child(Input::new("first-name").placeholder("First Name"))
                    .child(Input::new("last-name").placeholder("Last Name"))
            )
            .child(Input::new("email").placeholder("Email Address"))
            .child(
                h_flex()
                    .justify_end()
                    .child(Button::new("update").primary().label("Update Profile"))
            )
    )
```

### Settings Panel

```rust
GroupBox::new()
    .outline()
    .title("Display Settings")
    .child(
        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .child(Label::new("Theme"))
                    .child(
                        RadioGroup::horizontal("theme")
                            .child(Radio::new("light").label("Light"))
                            .child(Radio::new("dark").label("Dark"))
                            .child(Radio::new("auto").label("Auto"))
                    )
            )
            .child(
                h_flex()
                    .justify_between()
                    .child(Label::new("Font Size"))
                    .child(
                        Select::new("font-size")
                            .option("small", "Small")
                            .option("medium", "Medium")
                            .option("large", "Large")
                    )
            )
    )
```

### Subscription Management

```rust
GroupBox::new()
    .title("Email Subscriptions")
    .child(
        v_flex()
            .gap_2()
            .child(Checkbox::new("newsletter").label("Weekly Newsletter"))
            .child(Checkbox::new("updates").label("Product Updates"))
            .child(Checkbox::new("security").label("Security Alerts"))
            .child(Checkbox::new("marketing").label("Marketing Communications"))
    )
    .child(
        h_flex()
            .justify_between()
            .mt_4()
            .child(Button::new("unsubscribe-all").link().label("Unsubscribe All"))
            .child(Button::new("save").primary().label("Update Preferences"))
    )
```

### Without Title

```rust
GroupBox::new()
    .outline()
    .child(
        h_flex()
            .justify_between()
            .items_center()
            .child("Enable two-factor authentication")
            .child(Switch::new("2fa").checked(false))
    )
```

## Styling

The GroupBox component supports extensive customization through both built-in variants and custom styling:

### Theme Integration

```rust
// Using theme colors
GroupBox::new()
    .fill()
    .bg(cx.theme().group_box)
    .title("Themed Group Box")
```

### Custom Appearance

```rust
GroupBox::new()
    .outline()
    .border_2()
    .border_color(cx.theme().accent)
    .rounded(cx.theme().radius_lg)
    .title("Custom Styled Group Box")
    .title_style(
        StyleRefinement::default()
            .text_color(cx.theme().accent)
            .font_bold()
    )
```

## Best Practices

1. **Use titles for clarity** - Always include a descriptive title when grouping form controls
2. **Choose appropriate variants** - Use `fill()` for primary content groups, `outline()` for secondary groupings
3. **Maintain visual hierarchy** - Use GroupBox to create clear sections without overwhelming the interface
4. **Group related content** - Only group logically related controls and information
5. **Consider spacing** - The component automatically handles internal spacing, but consider external margins
6. **Responsive design** - GroupBox adapts well to different screen sizes and container widths

## Related Components

- **Form**: Use GroupBox within forms to organize sections
- **Dialog**: GroupBox works well within dialogs for organizing content
- **Accordion**: For collapsible grouped content, consider using Accordion instead
- **Card**: For elevated content containers with more visual weight
