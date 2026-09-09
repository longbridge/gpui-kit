---
title: Dialog
description: A dialog dialog for displaying content in a layer above the app.
---

# Dialog

Dialog component for creating dialogs, confirmations, and alerts. Supports overlay, keyboard shortcuts, and various customizations.

## Import

```rust
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::WindowExt;
```

## Usage

### Setup application root view for display of dialogs

You need to set up your application's root view to render the dialog layer. This is typically done in your main application struct's render method.

The [Root::render_dialog_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_dialog_layer) function handles rendering any active dialogs on top of your app content.

```rust
use gpui_kit::component::TitleBar;

struct MyApp {
    view: AnyView,
}

impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(window, cx);

        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(TitleBar::new())
                    .child(div().flex_1().overflow_hidden().child(self.view.clone())),
            )
            // Render the dialog layer on top of the app content
            .children(dialog_layer)
    }
}
```

### Basic Dialog

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("Welcome")
        .child("This is a dialog dialog.")
})
```

### Form Dialog

```rust
let input = cx.new(|cx| InputState::new(window, cx));

window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("User Information")
        .child(
            v_flex()
                .gap_3()
                .child("Please enter your details:")
                .child(Input::new(&input))
        )
        .footer(|_, _, _, _| {
            vec![
                Button::new("ok")
                    .primary()
                    .label("Submit")
                    .on_click(|_, window, cx| {
                        window.close_dialog(cx);
                    }),
                Button::new("cancel")
                    .label("Cancel")
                    .on_click(|_, window, cx| {
                        window.close_dialog(cx);
                    }),
            ]
        })
})
```

### Dialog with Icon

```rust
window.open_dialog(cx, |dialog, _, cx| {
    dialog
        .child(
            h_flex()
                .gap_3()
                .child(Icon::new(IconName::TriangleAlert)
                    .size_6()
                    .text_color(cx.theme().warning))
                .child("This action cannot be undone.")
        )
})
```

### Scrollable Dialog

```rust
use gpui_kit::component::text::markdown;

window.open_dialog(cx, |dialog, window, cx| {
    dialog
        .h(px(450.))
        .title("Long Content")
        .child(markdown(long_markdown_text))
})
```

### Dialog Options

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("Custom Dialog")
        .overlay(true)              // Show overlay (default: true)
        .overlay_closable(true)     // Click overlay to close (default: true)
        .keyboard(true)             // ESC to close (default: true)
        .close_button(false)        // Show close button (default: true)
        .child("Dialog content")
})
```

### Nested Dialogs

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("First Dialog")
        .child("This is the first dialog")
        .footer(|_, _, _, _| {
            vec![
                Button::new("open-another")
                    .label("Open Another Dialog")
                    .on_click(|_, window, cx| {
                        window.open_dialog(cx, |dialog, _, _| {
                            dialog
                                .title("Second Dialog")
                                .child("This is nested")
                        });
                    }),
            ]
        })
})
```

### Custom Styling

```rust
window.open_dialog(cx, |dialog, _, cx| {
    dialog
        .rounded(cx.theme().radius_lg)
        .bg(cx.theme().cyan)
        .text_color(cx.theme().info_foreground)
        .title("Custom Style")
        .child("Styled dialog content")
})
```

### Custom Padding

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .p_3()                      // Custom padding
        .title("Custom Padding")
        .child("Dialog with custom spacing")
})
```

### Close Dialog Programmatically

The `close_dialog` method can be used to close the active dialog from anywhere within the window context.

```rust
// Close top level active dialog.
window.close_dialog(cx);

// Close and perform action
Button::new("submit")
    .primary()
    .label("Submit")
    .on_click(|_, window, cx| {
        // Do something
        window.close_dialog(cx);
    })
```

## Declarative API

The Dialog component now supports a declarative API that provides a more React-like component composition pattern using dedicated header, title, description, and footer components.

### Import

```rust
use gpui_kit::component::dialog::{
    Dialog, DialogHeader, DialogTitle, DialogDescription, DialogFooter,
};
```

### Trigger-based Dialog

The trigger-based approach allows you to create a dialog that opens when a trigger element is clicked. The dialog is defined inline with the trigger.

```rust
Dialog::new(cx)
    .trigger(
        Button::new("open-dialog")
            .outline()
            .label("Open Dialog")
    )
    .content(|content, _, cx| {
        content
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Account Created"))
                    .child(DialogDescription::new().child(
                        "Your account has been created successfully!",
                    ))
            )
            .child(
                DialogFooter::new()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().muted)
                    .child(
                        Button::new("cancel")
                            .outline()
                            .label("Cancel")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            })
                    )
                    .child(
                        Button::new("ok")
                            .primary()
                            .label("Save Changes")
                    )
            )
    })
```

### Content Builder Pattern

Use the content builder pattern with `window.open_dialog` for more control over dialog creation:

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .w(px(400.))
        .content(|content, _, _| {
            content
                .child(
                    DialogHeader::new()
                        .child(DialogTitle::new().child("Custom Width"))
                        .child(DialogDescription::new().child(
                            "This dialog has a custom width of 400px.",
                        ))
                )
                .child(div().child(
                    "Content area with custom width configuration."
                ))
                .child(
                    DialogFooter::new()
                        .justify_center()
                        .child(
                            Button::new("cancel")
                                .flex_1()
                                .outline()
                                .label("Cancel")
                                .on_click(|_, window, cx| {
                                    window.close_dialog(cx);
                                })
                        )
                        .child(
                            Button::new("done")
                                .flex_1()
                                .primary()
                                .label("Done")
                                .on_click(|_, window, cx| {
                                    window.close_dialog(cx);
                                })
                        )
                )
        })
})
```

### Declarative Components

#### DialogHeader

Container for the dialog's title and description section.

```rust
DialogHeader::new()
    .child(DialogTitle::new().child("Title"))
    .child(DialogDescription::new().child("Description"))
```

#### DialogTitle

Displays the main title of the dialog with semantic styling.

```rust
DialogTitle::new()
    .child("Account Settings")
```

#### DialogDescription

Displays descriptive text below the title with muted styling.

```rust
DialogDescription::new()
    .child("Update your account settings and preferences here.")
```

#### DialogFooter

Container for action buttons and footer content. Automatically applies proper spacing and alignment.

```rust
DialogFooter::new()
    .bg(cx.theme().muted)
    .border_t_1()
    .border_color(cx.theme().border)
    .child(Button::new("cancel").outline().label("Cancel"))
    .child(Button::new("save").primary().label("Save"))
```

### Form Dialog with Declarative API

```rust
let name_input = cx.new(|cx| InputState::new(window, cx));
let email_input = cx.new(|cx| InputState::new(window, cx));

Dialog::new(cx)
    .trigger(Button::new("edit-profile").label("Edit Profile"))
    .content(|content, _, cx| {
        content
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Edit Profile"))
                    .child(DialogDescription::new().child(
                        "Make changes to your profile here. Click save when done."
                    ))
            )
            .child(
                v_flex()
                    .gap_4()
                    .py_4()
                    .child(
                        v_flex()
                            .gap_2()
                            .child("Name")
                            .child(Input::new(&name_input).placeholder("Enter your name"))
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child("Email")
                            .child(Input::new(&email_input).placeholder("Enter your email"))
                    )
            )
            .child(
                DialogFooter::new()
                    .child(Button::new("cancel").outline().label("Cancel"))
                    .child(Button::new("save").primary().label("Save Changes"))
            )
    })
```

### Styled Footer

Customize the footer appearance with background colors, borders, and alignment:

```rust
DialogFooter::new()
    .justify_center()        // Center align buttons
    .bg(cx.theme().muted)    // Background color
    .border_t_1()            // Top border
    .border_color(cx.theme().border)
    .child(Button::new("btn1").flex_1().label("Cancel"))
    .child(Button::new("btn2").flex_1().primary().label("Confirm"))
```

### DialogContent Container

The `DialogContent` component provides a flexible container for dialog body content:

```rust
use gpui_kit::component::dialog::DialogContent;

window.open_dialog(cx, |dialog, _, _| {
    dialog.content(|content, _, cx| {
        content
            .child(DialogHeader::new()
                .child(DialogTitle::new().child("Settings"))
                .child(DialogDescription::new().child("Configure your preferences"))
            )
            .child(
                div()
                    .py_4()
                    .child("Main content area")
            )
            .child(DialogFooter::new()
                .child(Button::new("close").label("Close"))
            )
    })
})
```

## API Reference - Declarative Components

### Dialog

| Method                   | Description                                           |
| ------------------------ | ----------------------------------------------------- |
| `new(cx)`                | Create a new Dialog (no longer requires window param) |
| `trigger(element)`       | Set trigger element that opens the dialog             |
| `content(builder)`       | Set content using a builder function                  |
| `w(px)` / `width(px)`    | Set dialog width                                      |
| `max_w(px)`              | Set maximum width                                     |
| `margin_top(px)`         | Set top margin                                        |
| `overlay(bool)`          | Show/hide overlay (default: true)                     |
| `overlay_closable(bool)` | Allow closing by clicking overlay (default: true)     |
| `keyboard(bool)`         | Allow closing with ESC key (default: true)            |
| `close_button(bool)`     | Show/hide close button (default: true)                |

### DialogContent

Container for dialog body content. Automatically applies padding and flex layout.

```rust
DialogContent::new()
    .child(DialogHeader::new()...)
    .child(/* your content */)
    .child(DialogFooter::new()...)
```

### DialogHeader

Container for title and description. Automatically applies vertical flex layout with proper gap.

```rust
DialogHeader::new()
    .child(DialogTitle::new().child("Title"))
    .child(DialogDescription::new().child("Description"))
```

### DialogTitle

Displays the dialog title with semantic styling (font-semibold, proper line-height).

```rust
DialogTitle::new()
    .child("Dialog Title")
```

### DialogDescription

Displays descriptive text with muted foreground color and proper text sizing.

```rust
DialogDescription::new()
    .child("This is a description text that provides more context.")
```

### DialogFooter

Container for footer buttons with automatic spacing and alignment.

```rust
DialogFooter::new()
    .justify_end()  // Right align (default)
    .child(Button::new("btn1").label("Cancel"))
    .child(Button::new("btn2").primary().label("OK"))
```

## Breaking Changes

### Dialog::new() Signature Change

The `Dialog::new()` constructor no longer requires a `window` parameter:

```rust
// Old API (deprecated)
Dialog::new(window, cx)

// New API
Dialog::new(cx)
```

### Content Builder Function

The `.content()` method now accepts a builder function instead of a pre-built `DialogContent`:

```rust
// Old approach (still works)
dialog.child(DialogHeader::new()...)

// New declarative API
dialog.content(|content, window, cx| {
    content
        .child(DialogHeader::new()...)
        .child(DialogFooter::new()...)
})
```

## Best Practices

1. **Use Declarative Components**: Prefer `DialogHeader`, `DialogTitle`, `DialogDescription`, and `DialogFooter` for consistent styling
2. **Trigger-based for Simple Cases**: Use the trigger pattern for straightforward dialogs that open from a button
3. **Builder Pattern for Complex Dialogs**: Use `window.open_dialog` with content builder for dialogs requiring complex logic or state
4. **Semantic Structure**: Always include `DialogHeader` with title and description for accessibility
5. **Consistent Footer**: Use `DialogFooter` for all action buttons to maintain visual consistency
6. **Proper Sizing**: Explicitly set dialog width when content requires specific dimensions
