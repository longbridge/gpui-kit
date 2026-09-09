---
title: Kbd
description: Displays keyboard shortcuts with platform-specific formatting.
---

# Kbd

A component for displaying keyboard shortcuts and key combinations with proper platform-specific formatting. Automatically adapts the display to match the conventions of macOS (using symbols) or Windows/Linux (using text labels).

## Import

```rust
use gpui_kit::component::kbd::Kbd;
use gpui_kit::Keystroke;
```

## Usage

### Basic Keyboard Shortcut

```rust
// Create from a keystroke
let kbd = Kbd::new(Keystroke::parse("cmd-shift-p").unwrap());

// Or convert directly from keystroke
let kbd: Kbd = Keystroke::parse("escape").unwrap().into();
```

### Common Shortcuts

```rust
// Command palette
Kbd::new(Keystroke::parse("cmd-shift-p").unwrap())

// New tab
Kbd::new(Keystroke::parse("cmd-t").unwrap())

// Zoom controls
Kbd::new(Keystroke::parse("cmd--").unwrap())  // Zoom out
Kbd::new(Keystroke::parse("cmd-+").unwrap())  // Zoom in

// Navigation
Kbd::new(Keystroke::parse("escape").unwrap())
Kbd::new(Keystroke::parse("enter").unwrap())
Kbd::new(Keystroke::parse("backspace").unwrap())
```

### Multiple Modifiers

```rust
// Complex combinations
Kbd::new(Keystroke::parse("cmd-ctrl-shift-a").unwrap())
Kbd::new(Keystroke::parse("cmd-alt-backspace").unwrap())
Kbd::new(Keystroke::parse("ctrl-alt-shift-a").unwrap())
```

### Arrow Keys and Function Keys

```rust
// Arrow keys
Kbd::new(Keystroke::parse("left").unwrap())
Kbd::new(Keystroke::parse("right").unwrap())
Kbd::new(Keystroke::parse("up").unwrap())
Kbd::new(Keystroke::parse("down").unwrap())

// Function keys
Kbd::new(Keystroke::parse("f12").unwrap())
Kbd::new(Keystroke::parse("secondary-f12").unwrap())

// Page navigation
Kbd::new(Keystroke::parse("pageup").unwrap())
Kbd::new(Keystroke::parse("pagedown").unwrap())
```

### Without Visual Styling

```rust
// Display only the key text without the styled background
Kbd::new(Keystroke::parse("cmd-s").unwrap())
    .appearance(false)
```

### From Action Bindings

```rust
use gpui_kit::{Action, Window, FocusHandle};

// Get first keybinding for an action
if let Some(kbd) = Kbd::binding_for_action(&MyAction {}, None, window) {
    // Display the bound shortcut
}

// Get keybinding for action within a specific context
if let Some(kbd) = Kbd::binding_for_action(&MyAction {}, Some("Editor"), window) {
    // Display context-specific shortcut
}

// Get keybinding for action within a focus handle
if let Some(kbd) = Kbd::binding_for_action_in(&MyAction {}, &focus_handle, window) {
    // Display shortcut for focused element
}
```

## Platform Differences

The Kbd component automatically formats shortcuts according to platform conventions:

### macOS

- Uses symbols: ⌃ ⌥ ⇧ ⌘
- No separators between modifiers
- Order: Control, Option, Shift, Command
- Special keys: ⌫ (backspace), ⎋ (escape), ⏎ (enter), ← → ↑ ↓ (arrows)

### Windows/Linux

- Uses text labels: Ctrl, Alt, Shift, Win
- Plus sign (+) separators
- Order: Ctrl, Alt, Shift, Win
- Special keys: Backspace, Esc, Enter, Left, Right, Up, Down

### Examples by Platform

| Input               | macOS | Windows/Linux     |
| ------------------- | ----- | ----------------- |
| `cmd-a`             | ⌘A    | Win+A             |
| `ctrl-shift-a`      | ⌃⇧A   | Ctrl+Shift+A      |
| `cmd-alt-backspace` | ⌥⌘⌫   | Win+Alt+Backspace |
| `escape`            | ⎋     | Esc               |
| `enter`             | ⏎     | Enter             |
| `left`              | ←     | Left              |

## Examples

### Keyboard Shortcut Help

```rust
use gpui_kit::{div, h_flex, v_flex};

// Display common shortcuts
v_flex()
    .gap_2()
    .child(
        h_flex()
            .gap_2()
            .items_center()
            .child("Open command palette:")
            .child(Kbd::new(Keystroke::parse("cmd-shift-p").unwrap()))
    )
    .child(
        h_flex()
            .gap_2()
            .items_center()
            .child("Save file:")
            .child(Kbd::new(Keystroke::parse("cmd-s").unwrap()))
    )
    .child(
        h_flex()
            .gap_2()
            .items_center()
            .child("Find in files:")
            .child(Kbd::new(Keystroke::parse("cmd-shift-f").unwrap()))
    )
```

### Menu Item with Shortcut

```rust
h_flex()
    .justify_between()
    .items_center()
    .child("New File")
    .child(Kbd::new(Keystroke::parse("cmd-n").unwrap()))
```

### Inline Documentation

```rust
div()
    .child("Press ")
    .child(Kbd::new(Keystroke::parse("escape").unwrap()))
    .child(" to cancel or ")
    .child(Kbd::new(Keystroke::parse("enter").unwrap()))
    .child(" to confirm.")
```

### Custom Styling

```rust
Kbd::new(Keystroke::parse("cmd-k").unwrap())
    .text_color(cx.theme().accent)
    .border_color(cx.theme().accent)
    .bg(cx.theme().accent.opacity(0.1))
```

### Text-Only Format

```rust
// Get formatted text without styling
let shortcut_text = Kbd::format(&Keystroke::parse("cmd-shift-p").unwrap());
div().child(format!("Shortcut: {}", shortcut_text))
```

## Styling

The Kbd component uses the following default styles:

- Border with theme border color
- Muted foreground text color
- Background with theme background color
- Small rounded corners
- Centered text alignment
- Extra small font size
- Minimal padding (0.5px vertical, 1px horizontal)
- Minimum width of 5 units
- Flex shrink disabled to maintain size

All styles can be customized using the `Styled` trait methods.
