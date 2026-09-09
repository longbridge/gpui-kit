---
title: Toggle
description: A button-style toggle component for binary on/off or selected states.
---

# Toggle

A button-style toggle component that represents on/off or selected states. Unlike a traditional switch, toggles appear as buttons that can be pressed in or out. They're perfect for toolbar buttons, filter options, or any binary choice that benefits from a button-like appearance.

## Import

```rust
use gpui_kit::component::button::{Toggle, ToggleGroup};
```

## Usage

### Basic Toggle

```rust
Toggle::new("toggle1").
    .label("Toggle me")
    .checked(false)
    .on_click(|checked, _, _| {
        println!("Toggle is now: {}", checked);
    })
```

Here, we can use `on_click` to handle toggle state changes. The callback receives the **new checked state** as a `bool`.

### Icon Toggle

```rust
use gpui_kit::component::IconName;

Toggle::new("toggle2")
    .icon(IconName::Eye)
    .checked(true)
    .on_click(|checked, _, _| {
        println!("Visibility: {}", if *checked { "shown" } else { "hidden" });
    })
```

### Controlled Toggle

```rust
struct MyView {
    is_active: bool,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Toggle::new("active")
            .label("Active")
            .checked(self.is_active)
            .on_click(cx.listener(|view, checked, _, cx| {
                view.is_active = *checked;
                cx.notify();
            }))
    }
}
```

### Toggle Variants

```rust
// Ghost toggle (default)
Toggle::new("ghost-toggle")
    .ghost()
    .label("Ghost")

// Outline toggle
Toggle::new("outline-toggle")
    .outline()
    .label("Outline")
```

### Different Sizes

```rust
// Extra small
Toggle::new("xs-toggle")
    .icon(IconName::Star)
    .xsmall()

// Small
Toggle::new("small-toggle")
    .label("Small")
    .small()

// Medium (default)
Toggle::new("medium-toggle")
    .label("Medium")


// Large
Toggle::new("large-toggle")
    .label("Large")
    .large()
```

### Disabled State

```rust
// Disabled unchecked
Toggle::new("disabled-toggle")
    .label("Disabled")
    .disabled(true)
    .checked(false)

// Disabled checked
Toggle::new("disabled-checked-toggle")
    .label("Selected (Disabled)")
    .disabled(true)
    .checked(true)
```

## Toggle vs Switch

| Feature                | Toggle                                      | Switch                                    |
| ---------------------- | ------------------------------------------- | ----------------------------------------- |
| **Appearance**         | Button-like, can be pressed in/out          | Traditional switch with sliding indicator |
| **Use Cases**          | Toolbar buttons, filters, binary options    | Settings, preferences, on/off states      |
| **Visual Style**       | Rectangular button shape                    | Rounded switch track with thumb           |
| **State Indication**   | Background color change, pressed appearance | Position of sliding thumb                 |
| **Multiple Selection** | Supports groups with multiple selection     | Individual switches only                  |

**Use Toggle when you want:**

- Button-like appearance for binary states
- Grouping multiple related options
- Toolbar or filter interfaces
- Options that feel like "selections" rather than "settings"

**Use Switch when you want:**

- Traditional on/off control appearance
- Settings or preferences interface
- Clear visual indication of state with sliding animation
- Individual boolean controls

## Integration with ToggleGroup

Toggle buttons can be grouped together using `ToggleGroup` for related options:

### Basic Toggle Group

```rust
ToggleGroup::new("filter-group")
    .child(Toggle::new(0).icon(IconName::Bell))
    .child(Toggle::new(1).icon(IconName::Bot))
    .child(Toggle::new(2).icon(IconName::Inbox))
    .child(Toggle::new(3).label("Other"))
    .on_click(|checkeds, _, _| {
        println!("Selected toggles: {:?}", checkeds);
    })
```

The `on_click` callback receives a `Vec<bool>` representing the **new checked state** of each toggle in the group.

### Toggle Group with Controlled State

```rust
struct FilterView {
    notifications: bool,
    bots: bool,
    inbox: bool,
    other: bool,
}

impl Render for FilterView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ToggleGroup::new("filters")
            .child(Toggle::new(0).icon(IconName::Bell).checked(self.notifications))
            .child(Toggle::new(1).icon(IconName::Bot).checked(self.bots))
            .child(Toggle::new(2).icon(IconName::Inbox).checked(self.inbox))
            .child(Toggle::new(3).label("Other").checked(self.other))
            .on_click(cx.listener(|view, checkeds, _, cx| {
                view.notifications = checkeds[0];
                view.bots = checkeds[1];
                view.inbox = checkeds[2];
                view.other = checkeds[3];
                cx.notify();
            }))
    }
}
```

### Toggle Group Variants and Sizes

```rust
// Outline variant, small size
ToggleGroup::new("compact-filters")
    .outline()
    .small()
    .child(Toggle::new(0).icon(IconName::Filter))
    .child(Toggle::new(1).icon(IconName::Sort))
    .child(Toggle::new(2).icon(IconName::Search))

// Ghost variant (default), extra small
ToggleGroup::new("mini-toolbar")
    .xsmall()
    .child(Toggle::new(0).icon(IconName::Bold))
    .child(Toggle::new(1).icon(IconName::Italic))
    .child(Toggle::new(2).icon(IconName::Underline))
```

### Segmented Toggle Group

Use `segmented()` when a group should render as a connected segmented control. The
group still uses the same multi-toggle behavior: `on_click` receives a `Vec<bool>`
with the new checked state for each item.

```rust
ToggleGroup::new("formatting")
    .segmented()
    .outline()
    .child(Toggle::new(0).label("Bold").checked(self.bold))
    .child(Toggle::new(1).label("Italic").checked(self.italic))
    .child(Toggle::new(2).label("Code").checked(self.code))
    .on_click(cx.listener(|view, states, _, cx| {
        view.bold = states[0];
        view.italic = states[1];
        view.code = states[2];
        cx.notify();
    }))
```

By default, segmented groups use a zero gap so adjacent items share one outline.
Pass a non-zero gap when you want the segmented sizing and variants but separated
items:

```rust
use gpui_kit::px;

ToggleGroup::new("quick-actions")
    .segmented()
    .outline()
    .gap(px(8.))
    .small()
    .child(Toggle::new(0).label("Star"))
    .child(Toggle::new(1).label("Watch"))
    .child(Toggle::new(2).label("Pin"))
```

If you need mutually exclusive behavior, keep that state in your view model and
set only one child to `checked(true)` until a dedicated single-selection API is
available.

## Event Handling

### Individual Toggle Events

```rust
Toggle::new("subscribe-toggle")
    .label("Subscribe")
    .on_click(|checked, window, cx| {
        if *checked {
            // Handle subscription logic
            println!("Subscribed!");
        } else {
            // Handle unsubscription logic
            println!("Unsubscribed!");
        }
    })
```

## Examples

### Toolbar with Toggle Buttons

```rust
struct EditorToolbar {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
}

h_flex()
    .gap_1()
    .p_2()
    .bg(cx.theme().background)
    .border_1()
    .border_color(cx.theme().border)
    .child(
        ToggleGroup::new("formatting")
            .small()
            .child(Toggle::new(0).icon(IconName::Bold).checked(self.bold))
            .child(Toggle::new(1).icon(IconName::Italic).checked(self.italic))
            .child(Toggle::new(2).icon(IconName::Underline).checked(self.underline))
            .child(Toggle::new(3).icon(IconName::Strikethrough).checked(self.strikethrough))
            .on_click(cx.listener(|view, states, _, cx| {
                view.bold = states[0];
                view.italic = states[1];
                view.underline = states[2];
                view.strikethrough = states[3];
                cx.notify();
            }))
    )
```

### Filter Interface

```rust
struct FilterPanel {
    show_completed: bool,
    show_pending: bool,
    show_cancelled: bool,
    show_urgent: bool,
}

v_flex()
    .gap_3()
    .p_4()
    .child(Label::new("Filter by status"))
    .child(
        ToggleGroup::new("status-filters")
            .outline()
            .child(Toggle::new(0).label("Completed").checked(self.show_completed))
            .child(Toggle::new(1).label("Pending").checked(self.show_pending))
            .child(Toggle::new(2).label("Cancelled").checked(self.show_cancelled))
            .on_click(cx.listener(|view, states, _, cx| {
                view.show_completed = states[0];
                view.show_pending = states[1];
                view.show_cancelled = states[2];
                cx.notify();
            }))
    )
    .child(
        Toggle::new("urgent-filter")
            .label("Show urgent only")
            .checked(self.show_urgent)
            .on_click(cx.listener(|view, checked, _, cx| {
                view.show_urgent = *checked;
                cx.notify();
            }))
    )
```

### Settings with Individual Toggles

```rust
struct NotificationSettings {
    email_notifications: bool,
    push_notifications: bool,
    marketing_emails: bool,
}

v_flex()
    .gap_4()
    .child(
        h_flex()
            .items_center()
            .justify_between()
            .child(
                v_flex()
                    .child(Label::new("Email notifications"))
                    .child(
                        Label::new("Receive notifications via email")
                            .text_color(cx.theme().muted_foreground)
                            .text_sm()
                    )
            )
            .child(
                Toggle::new("email-notifications")
                    .icon(IconName::Mail)
                    .checked(self.email_notifications)
                    .on_click(cx.listener(|view, checked, _, cx| {
                        view.email_notifications = *checked;
                        cx.notify();
                    }))
            )
    )
    .child(
        h_flex()
            .items_center()
            .justify_between()
            .child(Label::new("Push notifications"))
            .child(
                Toggle::new("push-notifications")
                    .icon(IconName::Bell)
                    .checked(self.push_notifications)
                    .on_click(cx.listener(|view, checked, _, cx| {
                        view.push_notifications = *checked;
                        cx.notify();
                    }))
            )
    )
```

### Multi-select Options

```rust
struct SelectionView {
    selected_categories: Vec<bool>,
}

impl SelectionView {
    fn categories() -> Vec<&'static str> {
        vec!["Technology", "Design", "Business", "Science", "Art"]
    }
}

v_flex()
    .gap_3()
    .child(Label::new("Select categories of interest"))
    .child(
        ToggleGroup::new("categories")
            .children(
                Self::categories()
                    .into_iter()
                    .enumerate()
                    .map(|(i, category)| {
                        Toggle::new(i)
                            .label(category)
                            .checked(self.selected_categories.get(i).copied().unwrap_or(false))
                    })
            )
            .on_click(cx.listener(|view, states, _, cx| {
                view.selected_categories = states.clone();
                cx.notify();
            }))
    )
```

## Best Practices

1. **Use meaningful labels**: Choose clear, descriptive text for toggle labels
2. **Group related options**: Use ToggleGroup for logically related binary choices
3. **Provide visual feedback**: The checked state should be clearly distinguishable
4. **Consider context**: Use toggles for options that feel like "selections" rather than "settings"
5. **Maintain state consistency**: Ensure toggle state reflects the actual application state
6. **Accessible labels**: Provide tooltips or ARIA labels for icon-only toggles
