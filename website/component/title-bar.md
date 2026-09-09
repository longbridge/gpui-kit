---
title: TitleBar
description: A custom window title bar component with window controls and custom content support.
---

# TitleBar

TitleBar provides a customizable window title bar that can replace the default OS title bar. It includes platform-specific window controls (minimize, maximize, close) and supports custom content and styling. The component automatically adapts to different operating systems (macOS, Windows, Linux) with appropriate behaviors and visual styles.

## Import

```rust
use gpui_kit::component::TitleBar;
```

## Usage

### Basic Title Bar

```rust
TitleBar::new()
    .child(div().child("My Application"))
```

### Title Bar with Custom Content

```rust
TitleBar::new()
    .child(
        div()
            .flex()
            .items_center()
            .gap_3()
            .child("App Name")
            .child(Badge::new().count(5))
    )
    .child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(Button::new("settings").icon(IconName::Settings))
            .child(Button::new("profile").icon(IconName::User))
    )
```

### Title Bar with Menu Bar

```rust
TitleBar::new()
    .child(
        div()
            .flex()
            .items_center()
            .child(AppMenuBar::new(window, cx))
    )
    .child(
        div()
            .flex()
            .items_center()
            .justify_end()
            .gap_2()
            .child(Button::new("github").icon(IconName::GitHub))
            .child(Button::new("notifications").icon(IconName::Bell))
    )
```

### Title Bar with Window Controls (Linux only)

```rust
TitleBar::new()
    .on_close_window(|_, window, cx| {
        // Custom close behavior
        window.push_notification("Saving before close...", cx);
        // Perform cleanup
        window.remove_window();
    })
    .child(div().child("Custom Close Behavior"))
```

### Styled Title Bar

```rust
TitleBar::new()
    .bg(cx.theme().primary)
    .border_color(cx.theme().primary_border)
    .child(
        div()
            .text_color(cx.theme().primary_foreground)
            .child("Styled Title Bar")
    )
```

### Title Bar Options for Window

Use `TitleBar::window_options()` as the base of the window options, it sets up
everything the title bar needs, including letting the title bar own dragging and
double clicking instead of the system.

```rust
use gpui_kit::WindowOptions;

WindowOptions {
    window_bounds: Some(window_bounds),
    ..TitleBar::window_options()
}
```

If you build the [`WindowOptions`] yourself, set both fields:

```rust
use gpui_kit::WindowOptions;

WindowOptions {
    titlebar: Some(TitleBar::title_bar_options()),
    // Required on macOS, otherwise the system also handles title bar double
    // clicks and delays title bar clicks to disambiguate double clicks.
    app_owns_titlebar_drag: true,
    ..Default::default()
}
```

## Platform Differences

### macOS

- Uses native traffic light buttons (minimize, maximize, close)
- Traffic light position is automatically set to `(9px, 9px)`
- Double-click behavior calls `window.titlebar_double_click()`
- Left padding accounts for traffic light buttons (80px)
- Appears transparent by default

### Windows

- Custom window control buttons with system integration
- Uses `WindowControlArea` for proper window management
- Control buttons have hover and active states
- Fixed button width of 34px each
- Left padding is 12px

### Linux

- Custom window control buttons with manual event handling
- Supports custom close window callback via `on_close_window()`
- Double-click to maximize/restore window
- Right-click shows window context menu
- Window dragging supported in title bar area

## API Reference

### TitleBar

| Method                | Description                              |
| --------------------- | ---------------------------------------- |
| `new()`               | Create a new title bar                   |
| `child(element)`      | Add child element to the title bar       |
| `on_close_window(fn)` | Custom close window handler (Linux only) |
| `title_bar_options()` | Get default titlebar options for window  |
| `window_options()`    | Get default window options for the title bar |

### Window Configuration

| Property                 | Description                                                    |
| ------------------------ | -------------------------------------------------------------- |
| `appears_transparent`    | Make title bar transparent (default: true)                     |
| `traffic_light_position` | Position of macOS traffic lights                               |
| `title`                  | Window title (optional when using custom title bar)            |
| `app_owns_titlebar_drag` | Let the title bar own dragging and double clicking (macOS only) |

### Title Bar Element (Internal)

The `TitleBarElement` provides window dragging functionality on Linux platforms.

### Constants

| Constant                 | Value                           | Description               |
| ------------------------ | ------------------------------- | ------------------------- |
| `TITLE_BAR_HEIGHT`       | `34px`                          | Standard title bar height |
| `TITLE_BAR_LEFT_PADDING` | `80px` (macOS), `12px` (others) | Left padding for content  |

## Examples

### Application Title Bar

```rust
use gpui_kit::component::{TitleBar, button::Button, menu::AppMenuBar};

struct AppTitleBar {
    app_menu_bar: Entity<AppMenuBar>,
}

impl Render for AppTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new()
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(self.app_menu_bar.clone())
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("settings")
                            .ghost()
                            .icon(IconName::Settings)
                    )
                    .child(
                        Button::new("help")
                            .ghost()
                            .icon(IconName::HelpCircle)
                    )
            )
    }
}
```

### Title Bar with Breadcrumbs

```rust
TitleBar::new()
    .child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child("Home")
            .child(IconName::ChevronRight)
            .child("Documents")
            .child(IconName::ChevronRight)
            .child("Project")
    )
    .child(
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(Button::new("search").icon(IconName::Search).ghost())
            .child(Button::new("more").icon(IconName::MoreHorizontal).ghost())
    )
```

### Custom Themed Title Bar

```rust
TitleBar::new()
    .h(px(40.)) // Custom height
    .bg(cx.theme().accent)
    .border_b_2()
    .border_color(cx.theme().accent_border)
    .child(
        div()
            .flex()
            .items_center()
            .text_color(cx.theme().accent_foreground)
            .font_weight_semibold()
            .child("Custom Theme App")
    )
```

### Title Bar with Status

```rust
TitleBar::new()
    .child(
        div()
            .flex()
            .items_center()
            .gap_3()
            .child("My Editor")
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("● Unsaved changes")
            )
    )
    .child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Line 42, Col 12")
            )
            .child(
                Button::new("sync")
                    .small()
                    .ghost()
                    .icon(IconName::RotateCcw)
                    .tooltip("Sync changes")
            )
    )
```

### Minimal Title Bar

```rust
TitleBar::new()
    .child(
        div()
            .text_center()
            .flex_1()
            .child("Document.txt")
    )
```

### Title Bar with Search

```rust
TitleBar::new()
    .child(
        div()
            .flex()
            .items_center()
            .gap_3()
            .child("File Explorer")
            .child(
                Input::new("search")
                    .placeholder("Search files...")
                    .w(px(200.))
                    .small()
            )
    )
```

## Notes

- The title bar automatically handles platform-specific styling and behavior
- Window controls are only rendered on Windows and Linux platforms
- The component integrates with GPUI's window management system
- Custom styling should consider platform conventions
- Window dragging is handled automatically in appropriate areas
