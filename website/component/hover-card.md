---
title: HoverCard
description: A floating overlay that displays rich content when hovering over a trigger element.
---

# HoverCard

HoverCard component for displaying rich content that appears when the mouse hovers over a trigger element. Ideal for previewing user profiles, link previews, and other contextual information without requiring a click. Features configurable delays for both opening and closing to prevent flickering during quick mouse movements.

This is most like the [Popover] component, but triggered by hover instead of click, and with timing controls for a smoother user experience.

## Import

```rust
use gpui_kit::component::hover_card::HoverCard;
```

## Usage

### Basic HoverCard

```rust
use gpui_kit::{ParentElement as _, Styled as _};
use gpui_kit::component::{hover_card::HoverCard, v_flex};

HoverCard::new("basic")
    .trigger(
        div()
            .child("Hover over me")
            .text_color(cx.theme().primary)
            .cursor_pointer()
            .text_sm()
    )
    .child(
        v_flex()
            .gap_2()
            .child(
                div()
                    .child("This is a hover card")
                    .font_semibold()
                    .text_sm()
            )
            .child(
                div()
                    .child("You can display rich content when hovering over a trigger element.")
                    .text_color(cx.theme().muted_foreground)
                    .text_sm()
            )
    )
```

### User Profile Preview

A common use case is showing user profiles when hovering over a username, similar to GitHub or Twitter:

```rust
use gpui_kit::{px, relative, Styled as _};
use gpui_kit::component::{
    avatar::Avatar,
    hover_card::HoverCard,
    h_flex,
    v_flex,
};

h_flex()
    .child("Hover over ")
    .text_sm()
    .child(
        HoverCard::new("user-profile")
            .trigger(
                div()
                    .child("@huacnlee")
                    .cursor_pointer()
                    .text_color(cx.theme().link)
            )
            .child(
                h_flex()
                    .w(px(320.))
                    .gap_4()
                    .items_start()
                    .child(
                        Avatar::new()
                            .src("https://avatars.githubusercontent.com/u/5518?s=64")
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .line_height(relative(1.))
                            .child(div().child("Jason Lee").font_semibold())
                            .child(
                                div()
                                    .child("@huacnlee")
                                    .text_color(cx.theme().muted_foreground)
                                    .text_sm()
                            )
                            .child("The author of GPUI Kit.")
                    )
            )
    )
    .child(" to see their profile")
```

### Custom Timing

Adjust the opening and closing delays to suit your needs:

```rust
use std::time::Duration;
use gpui_kit::Styled as _;
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    h_flex,
};

h_flex()
    .gap_4()
    .child(
        HoverCard::new("fast-open")
            .open_delay(Duration::from_millis(200))
            .close_delay(Duration::from_millis(100))
            .trigger(Button::new("fast").label("Fast Open (200ms)").outline())
            .child(div().child("This hover card opens after 200ms").text_sm())
    )
    .child(
        HoverCard::new("slow-open")
            .open_delay(Duration::from_secs(1))
            .close_delay(Duration::from_secs_f32(0.5))
            .trigger(Button::new("slow").label("Slow Open (1000ms)").outline())
            .child(div().child("This hover card opens after 1000ms").text_sm())
    )
```

### Positioning

HoverCard supports 6 positioning options using the [Anchor] type:

- TopLeft
- TopCenter
- TopRight
- BottomLeft
- BottomCenter
- BottomRight

Imagine the card has a pointer tip (like a speech bubble's tail). The anchor is where that tip sits relative to the trigger — `TopCenter` places it at the trigger's top center, `BottomRight` at the bottom-right, and so on. The card then hangs off that point.

For example, `Anchor::TopLeft` places the card just below the trigger, left-aligned to it:

```text
[ Trigger ]
┌──────────────┐
│  Hover Card  │
└──────────────┘
```

### Custom Content Builder

For performance optimization, you can provide a content builder function for more complex case, which only calls when the HoverCard is opened:

```rust
HoverCard::new("complex")
    .trigger(Button::new("btn").label("Hover me"))
    .content(|state, window, cx| {
        v_flex()
            .child("Dynamic content")
            .child(format!("Open: {}", state.is_open()))
    })
```

### Styling

HoverCard inherits all `Styled` trait methods:

```rust
HoverCard::new("styled")
    .trigger(Button::new("btn").label("Styled"))
    .w(px(400.))
    .max_h(px(500.))
    .text_sm()
    .gap_2()
    .child("Styled content")
```

Disable default appearance and apply custom styles:

```rust
HoverCard::new("custom-styled")
    .appearance(false)  // Disable default popover styling
    .trigger(Button::new("btn").label("Custom"))
    .bg(cx.theme().background)
    .border_2()
    .border_color(cx.theme().primary)
    .rounded(px(12.))
    .p_4()
    .child("Custom styled content")
```

## API Reference

### HoverCard Methods

- `new(id: impl Into<ElementId>)` - Create a new HoverCard with a unique ID
- `trigger<T: IntoElement>(trigger: T)` - Set the element that triggers the hover
- `content<F>(content: F)` - Set a content builder function that receives `(&mut HoverCardState, &mut Window, &mut Context<HoverCardState>)`
- `open_delay(duration: Duration)` - Set delay before showing (default: 600ms)
- `close_delay(duration: Duration)` - Set delay before hiding (default: 300ms)
- `anchor(anchor: impl Into<Anchor>)` - Set positioning (default: TopCenter)
- `on_open_change<F>(callback: F)` - Callback when open state changes, receives `(&bool, &mut Window, &mut App)`
- `appearance(appearance: bool)` - Enable/disable default styling (default: true)

### HoverCardState Methods

- `is_open() -> bool` - Check if the hover card is currently open

## Behavior Details

### Hover Timing

The HoverCard uses a sophisticated timing system to provide a smooth user experience:

1. **Open Delay (600ms default)**: Prevents the card from flickering when the mouse quickly passes over the trigger
2. **Close Delay (300ms default)**: Gives users time to move their mouse from the trigger to the content area without the card closing
3. **Interactive Content**: Users can move their mouse into the content area, and the card will remain open as long as the mouse is either on the trigger or in the content

### Edge Cases Handled

- **Quick Mouse Sweep**: If the mouse quickly moves across the trigger, the card won't open (cancelled by the open delay)
- **Trigger to Content Movement**: The card stays open when moving the mouse from the trigger to the content area
- **Rapid Hovering**: Multiple rapid hover events are debounced using an epoch-based timer system
- **Multiple HoverCards**: Each HoverCard has independent state, so multiple cards can coexist without interfering

## Best Practices

1. **Use appropriate delays**:
   - Standard content: 600ms open, 300ms close
   - Quick previews: 500ms open, 200ms close
   - Tooltips: 300ms open, 100ms close

2. **Keep content concise**: HoverCards should provide preview information, not full content

3. **Make triggers visually distinct**: Use colors, underlines, or cursor changes to indicate hoverable elements

4. **Consider accessibility**: HoverCards are visual-only and don't support keyboard navigation. For keyboard-accessible content, consider using a Popover instead

5. **Avoid nested HoverCards**: They can create confusing user experiences

## Differences from [Popover]

| Feature                  | HoverCard        | Popover            |
| ------------------------ | ---------------- | ------------------ |
| Trigger                  | Mouse hover      | Click/right-click  |
| Keyboard navigation      | No               | Yes (with focus)   |
| Dismiss on outside click | No               | Yes (configurable) |
| Timing delays            | Yes (open/close) | No                 |
| Primary use case         | Previews         | Actions/forms      |

[Popover]: ./popover.md
[Anchor]: https://docs.rs/gpui-component/latest/gpui_component/enum.Anchor.html
[Avatar]: ./avatar.md
