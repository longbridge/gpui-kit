---
title: Clipboard
description: A button component that helps you copy text or other content to your clipboard.
---

# Clipboard

The Clipboard component provides an easy way to copy text or other data to the user's clipboard. It renders as a button with a copy icon that changes to a checkmark when content is successfully copied. The component supports both static values and dynamic content through callback functions.

## Import

```rust
use gpui_kit::component::clipboard::Clipboard;
```

## Usage

### Basic Clipboard

```rust
Clipboard::new("my-clipboard")
    .value("Text to copy")
    .on_copied(|value, window, cx| {
        window.push_notification(format!("Copied: {}", value), cx)
    })
```

### Using Dynamic Values

The `value_fn` method allows you to provide a closure that generates the content to be copied at the time of the copy action.

- This is useful when the content to be copied depends on the current state of the application.
- And in some cases, it may have a larger overhead to compute, so you only want to do it when the user actually clicks the copy button.

```rust
let state = some_state.clone();
Clipboard::new("dynamic-clipboard")
    .value_fn(move |_, cx| {
        state.read(cx).get_current_value()
    })
    .on_copied(|value, window, cx| {
        window.push_notification(format!("Copied: {}", value), cx)
    })
```

### With Custom Content

```rust
use gpui_kit::component::label::Label;

 h_flex()
     .gap_2()
     .child(Label::new("Share URL"))
     .child(Icon::new(IconName::Share))
     .child(
        Clipboard::new("custom-clipboard")
        .value("https://example.com")
     )
```

### In Input Fields

The Clipboard component is commonly used as a suffix in input fields:

```rust
use gpui_kit::component::input::{InputState, Input};

let url_state = cx.new(|cx| InputState::new(window, cx).default_value("https://github.com"));

Input::new(&url_state)
    .suffix(
        Clipboard::new("url-clipboard")
            .value_fn({
                let state = url_state.clone();
                move |_, cx| state.read(cx).value()
            })
            .on_copied(|value, window, cx| {
                window.push_notification(format!("URL copied: {}", value), cx)
            })
    )
```

## API Reference

- [Clipboard]

## Examples

### Simple Text Copy

```rust
Clipboard::new("simple")
    .value("Hello, World!")
```

### With User Feedback

```rust
h_flex()
    .gap_2()
    .child(Label::new("Your API Key:"))
    .child(
        Clipboard::new("feedback")
            .value("sk-1234567890abcdef")
            .on_copied(|_, window, cx| {
                window.push_notification("API key copied to clipboard", cx)
            })
    )
```

### Form Field Integration

```rust
use gpui_kit::component::{
    input::{InputState, Input},
    h_flex, label::Label
};

let api_key = "sk-1234567890abcdef";

h_flex()
    .gap_2()
    .items_center()
    .child(Label::new("API Key:"))
    .child(
        Input::new(&input_state)
            .value(api_key)
            .readonly(true)
            .suffix(
                Clipboard::new("api-key-copy")
                    .value(api_key)
                    .on_copied(|_, window, cx| {
                        window.push_notification("API key copied!", cx)
                    })
            )
    )
```

### Dynamic Content Copy

```rust
struct AppState {
    current_url: String,
}

let app_state = cx.new(|_| AppState {
    current_url: "https://example.com".to_string()
});

Clipboard::new("current-url")
    .value_fn({
        let state = app_state.clone();
        move |_, cx| {
            SharedString::from(state.read(cx).current_url.clone())
        }
    })
    .on_copied(|url, window, cx| {
        window.push_notification(format!("Shared: {}", url), cx)
    })
```

## Data Types

The Clipboard component currently supports copying text strings to the clipboard. It uses GPUI's `ClipboardItem::new_string()` method, which handles:

- Plain text strings
- UTF-8 encoded content
- Cross-platform clipboard integration

[Clipboard]: https://docs.rs/gpui-component/latest/gpui_component/clipboard/struct.Clipboard.html
