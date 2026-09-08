---
title: AlertDialog
description: A modal dialog that interrupts the user with important content and expects a response.
---

# AlertDialog

AlertDialog is a modal dialog component that interrupts the user with important content and expects a response. It is built on top of the [Dialog] component with opinionated defaults and a simplified API.

## Differences from Dialog

AlertDialog provides these defaults on top of Dialog:

- Not overlay closable by default (can be changed with `overlay_closable(true)`)
- No close button by default (can be changed with `close_button(true)`)
- Footer buttons are center-aligned (Dialog uses right-alignment)
- Simplified API focused on alert and confirmation scenarios

## Import

```rust
use gpui_kit::component::dialog::{AlertDialog, DialogAction, DialogClose};
use gpui_kit::component::WindowExt;
```

## Usage

### Setup Application Root View

Like Dialog, you need to set up your application's root view to render the dialog layer. See [Dialog documentation](./dialog.md#setup-application-root-view) for details.

### Basic AlertDialog (Declarative API)

Create a fully declarative AlertDialog using `trigger` and `content`:

```rust
use gpui_kit::component::dialog::{AlertDialog, DialogHeader, DialogTitle, DialogDescription, DialogFooter};

AlertDialog::new(cx)
    .trigger(
        Button::new("show-alert")
            .outline()
            .label("Show Alert")
    )
    .content(|content, _, cx| {
        content
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Are you absolutely sure?"))
                    .child(DialogDescription::new().child(
                        "This action cannot be undone. \
                        This will permanently delete your account from our servers."
                    ))
            )
            .child(
                DialogFooter::new()
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
                            .label("Continue")
                            .on_click(|_, window, cx| {
                                window.push_notification("Confirmed", cx);
                                window.close_dialog(cx);
                            })
                    )
            )
    })
```

### Using DialogAction and DialogClose

`DialogAction` and `DialogClose` are wrapper components that simplify button click handling by automatically triggering the appropriate actions:

- **DialogClose**: Wraps a button to trigger the `Cancel` action, invoking `on_cancel` callback
- **DialogAction**: Wraps a button to trigger the `Confirm` action, invoking `on_ok` callback

These components eliminate the need to manually call `window.close_dialog(cx)`:

```rust
AlertDialog::new(cx)
    .trigger(Button::new("show-alert").outline().label("Show Alert"))
    .on_ok(|_, window, cx| {
        window.push_notification("You confirmed!", cx);
        true  // Return true to close dialog
    })
    .on_cancel(|_, window, cx| {
        window.push_notification("You cancelled!", cx);
        true
    })
    .content(|content, _, cx| {
        content
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Confirm Action"))
                    .child(DialogDescription::new().child("Do you want to proceed?"))
            )
            .child(
                DialogFooter::new()
                    .child(
                        DialogClose::new().child(
                            Button::new("cancel").outline().label("Cancel")
                        )
                    )
                    .child(
                        DialogAction::new().child(
                            Button::new("ok").primary().label("Confirm")
                        )
                    )
            )
    })
```

**Benefits:**
- No need to manually close the dialog
- Automatically connects to `on_ok` and `on_cancel` callbacks
- Cleaner, more declarative code
- Supports returning `false` from callbacks to prevent closing

### Basic AlertDialog (Imperative API)

Open a dialog imperatively using `WindowExt::open_alert_dialog`:

```rust
window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .title("Delete File")
        .description("Are you sure you want to delete this file? This action cannot be undone.")
        .show_cancel(true)
        .on_ok(|_, window, cx| {
            window.push_notification("File deleted", cx);
            true // Return true to close dialog
        })
})
```

### Custom Button Props

Use `button_props` to customize button text and styles:

```rust
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::button::ButtonVariant;

window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .title("Delete Account")
        .description("This will permanently delete your account and all associated data.")
        .button_props(
            DialogButtonProps::default()
                .ok_text("Delete")
                .ok_variant(ButtonVariant::Danger)
                .cancel_text("Keep")
                .show_cancel(true)
        )
        .on_ok(|_, window, cx| {
            window.push_notification("Account deleted", cx);
            true
        })
})
```

### AlertDialog with Icon

Using icon in declarative API:

```rust
use gpui_kit::component::{Icon, IconName, ActiveTheme};

AlertDialog::new(cx)
    .w(px(320.))
    .trigger(Button::new("permission").outline().label("Request Permission"))
    .on_ok(|_, window, cx| {
        window.push_notification("Permission granted", cx);
        true
    })
    .content(|content, _, cx| {
        content
            .child(
                DialogHeader::new()
                    .items_center()
                    .child(
                        Icon::new(IconName::TriangleAlert)
                            .size_10()
                            .text_color(cx.theme().warning)
                    )
                    .child(DialogTitle::new().child("Network Permission Required"))
                    .child(DialogDescription::new().child(
                        "We need your permission to access the network to provide better services."
                    ))
            )
            .child(
                DialogFooter::new()
                    .v_flex()
                    .child(
                        DialogAction::new().child(
                            Button::new("allow").w_full().primary().label("Allow")
                        )
                    )
                    .child(
                        DialogClose::new().child(
                            Button::new("deny").w_full().outline().label("Don't Allow")
                        )
                    )
            )
    })
```

Using icon in imperative API:

```rust
window.open_alert_dialog(cx, |alert, _, cx| {
    alert
        .title("Warning")
        .description("This action requires confirmation.")
        .icon(
            Icon::new(IconName::AlertTriangle)
                .size_8()
                .text_color(cx.theme().warning)
        )
})
```

### Destructive Action Confirmation

```rust
AlertDialog::new(cx)
    .trigger(
        Button::new("delete-account")
            .outline()
            .danger()
            .label("Delete Account")
    )
    .on_ok(|_, window, cx| {
        window.push_notification("Account deletion initiated", cx);
        true
    })
    .content(|content, _, _| {
        content
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Delete Account"))
                    .child(DialogDescription::new().child(
                        "This will permanently delete your account \
                        and all associated data. This action cannot be undone."
                    ))
            )
            .child(
                DialogFooter::new()
                    .child(
                        DialogClose::new().child(
                            Button::new("cancel").flex_1().outline().label("Cancel")
                        )
                    )
                    .child(
                        DialogAction::new().child(
                            Button::new("delete")
                                .flex_1()
                                .outline()
                                .danger()
                                .label("Delete Forever")
                        )
                    )
            )
    })
```

### Custom Width

```rust
AlertDialog::new(cx)
    .width(px(500.))
    .trigger(Button::new("custom-width").label("Custom Width"))
    .content(|content, _, _| {
        // ... dialog content
    })
```

### Controlling Dialog Close Behavior

#### Allow Overlay Click to Close

```rust
window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .title("Notice")
        .description("Click outside this dialog or press ESC to close it.")
        .overlay_closable(true)
})
```

#### Disable Keyboard ESC to Close

```rust
window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .title("Important Notice")
        .description("Please read this carefully before proceeding.")
        .keyboard(false)
})
```

#### Show Close Button

```rust
window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .title("Information")
        .description("Some information...")
        .close_button(true)
})
```

### Prevent Dialog from Closing

Return `false` from `on_ok` or `on_cancel` callbacks to prevent the dialog from closing:

```rust
use gpui_kit::component::dialog::DialogButtonProps;

window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .title("Processing")
        .description("A process is running. Click Continue to stop it or Cancel to keep waiting.")
        .button_props(
            DialogButtonProps::default()
                .ok_text("Continue")
                .show_cancel(true)
        )
        .on_ok(|_, window, cx| {
            // Return false to prevent closing
            window.push_notification("Cannot close: Process still running", cx);
            false
        })
        .on_cancel(|_, window, cx| {
            window.push_notification("Waiting...", cx);
            false
        })
})
```

### Dialog Close Callback

Use `on_close` to execute actions after the dialog closes (called after `on_ok` or `on_cancel`):

```rust
window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .title("Confirm")
        .description("Are you sure?")
        .on_close(|_, window, cx| {
            window.push_notification("Dialog closed", cx);
        })
})
```

## API Reference

### AlertDialog

| Method                   | Description                                                   |
| ------------------------ | ------------------------------------------------------------- |
| `new(cx)`                | Create a new AlertDialog                                      |
| `trigger(element)`       | Set trigger element that opens the dialog when clicked        |
| `content(builder)`       | Set dialog content using a builder function (declarative API) |
| `title(title)`           | Set dialog title (imperative API)                             |
| `description(desc)`      | Set dialog description (imperative API)                       |
| `icon(icon)`             | Set dialog icon (imperative API)                              |
| `button_props(props)`    | Set button properties (text, style, visibility)               |
| `show_cancel(bool)`      | Show/hide cancel button, default `false`                      |
| `width(px)`              | Set dialog width, default `420px`                             |
| `overlay_closable(bool)` | Allow clicking overlay to close, default `false`              |
| `close_button(bool)`     | Show/hide close button, default `false`                       |
| `keyboard(bool)`         | Support ESC key to close, default `true`                      |
| `on_ok(callback)`        | Set OK button callback, return `true` to close dialog         |
| `on_cancel(callback)`    | Set cancel button callback, return `true` to close dialog     |
| `on_close(callback)`     | Set callback after dialog closes                              |

### DialogButtonProps

| Method                    | Description                              |
| ------------------------- | ---------------------------------------- |
| `ok_text(text)`           | Set OK button text, default "OK"         |
| `cancel_text(text)`       | Set cancel button text, default "Cancel" |
| `ok_variant(variant)`     | Set OK button variant                    |
| `cancel_variant(variant)` | Set cancel button variant                |
| `show_cancel(bool)`       | Show/hide cancel button                  |
| `on_ok(callback)`         | Set OK callback                          |
| `on_cancel(callback)`     | Set cancel callback                      |

### DialogAction

A wrapper component that automatically triggers the `Confirm` action when its child element is clicked. This invokes the `on_ok` callback set on the AlertDialog.

**Usage:**
```rust
DialogAction::new().child(
    Button::new("ok").primary().label("Confirm")
)
```

**Behavior:**
- Dispatches `Confirm` action on click
- Invokes the `on_ok` callback
- Dialog closes if callback returns `true`
- Dialog stays open if callback returns `false`

### DialogClose

A wrapper component that automatically triggers the `Cancel` action when its child element is clicked. This invokes the `on_cancel` callback set on the AlertDialog.

**Usage:**
```rust
DialogClose::new().child(
    Button::new("cancel").outline().label("Cancel")
)
```

**Behavior:**
- Dispatches `Cancel` action on click
- Invokes the `on_cancel` callback
- Dialog closes if callback returns `true` (or if no callback is set)
- Dialog stays open if callback returns `false`

## Examples

### Delete Confirmation

Using imperative API with button props:

```rust
Button::new("delete")
    .danger()
    .label("Delete")
    .on_click(|_, window, cx| {
        window.open_alert_dialog(cx, |alert, _, _| {
            alert
                .title("Delete File?")
                .description("This action cannot be undone.")
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("Delete")
                        .ok_variant(ButtonVariant::Danger)
                        .show_cancel(true)
                )
                .on_ok(|_, window, cx| {
                    // Perform delete operation
                    window.push_notification("File deleted", cx);
                    true
                })
        });
    })
```

Or using declarative API with DialogAction/DialogClose:

```rust
AlertDialog::new(cx)
    .trigger(Button::new("delete").danger().label("Delete"))
    .on_ok(|_, window, cx| {
        window.push_notification("File deleted", cx);
        true
    })
    .content(|content, _, cx| {
        content
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Delete File?"))
                    .child(DialogDescription::new().child("This action cannot be undone."))
            )
            .child(
                DialogFooter::new()
                    .child(
                        DialogClose::new().child(
                            Button::new("cancel").outline().label("Cancel")
                        )
                    )
                    .child(
                        DialogAction::new().child(
                            Button::new("delete-confirm").danger().label("Delete")
                        )
                    )
            )
    })
```

### Session Timeout

```rust
window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .content(|content, _, _| {
            content
                .child(
                    DialogHeader::new()
                        .items_center()
                        .child(DialogTitle::new().child("Session Expired"))
                        .child(DialogDescription::new().child(
                            "Your session has expired due to inactivity. \
                            Please log in again to continue."
                        ))
                )
                .child(
                    DialogFooter::new()
                        .child(
                            Button::new("sign-in")
                                .label("Sign in")
                                .primary()
                                .flex_1()
                                .on_click(|_, window, cx| {
                                    window.push_notification("Redirecting to login...", cx);
                                    window.close_dialog(cx);
                                })
                        )
                )
        })
})
```

### Update Available

```rust
AlertDialog::new(cx)
    .trigger(Button::new("update").outline().label("Update Available"))
    .on_cancel(|_, window, cx| {
        window.push_notification("Update postponed", cx);
        true
    })
    .on_ok(|_, window, cx| {
        window.push_notification("Starting update...", cx);
        true
    })
    .content(|content, _, _| {
        content
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Update Available"))
                    .child(DialogDescription::new().child(
                        "A new version (v2.0.0) is available. \
                        This update includes new features and bug fixes."
                    ))
            )
            .child(
                DialogFooter::new()
                    .child(
                        DialogClose::new().child(
                            Button::new("later").flex_1().outline().label("Later")
                        )
                    )
                    .child(
                        DialogAction::new().child(
                            Button::new("update-now").flex_1().primary().label("Update Now")
                        )
                    )
            )
    })
```

## Best Practices

1. **Choose the Right API**: Use imperative API (`open_alert_dialog`) for simple confirmations; use declarative API (`trigger` + `content`) for complex layouts or integration with other components
2. **Use DialogAction and DialogClose**: Prefer wrapping buttons with `DialogAction` and `DialogClose` over manual `window.close_dialog()` calls for cleaner, more declarative code
3. **Clarify Intent**: Use appropriate button variants (e.g., `ButtonVariant::Danger` for delete operations) to communicate the importance of actions
4. **Provide Clear Descriptions**: Ensure users understand the consequences of their actions, especially for destructive operations
5. **Use Icons Wisely**: Icons can enhance attention for warnings and errors, but use them appropriately
6. **Prevent Closing Carefully**: Only prevent dialog closing when user confirmation is truly necessary (e.g., a process is running)
7. **Maintain Consistency**: Keep dialog button order and styles consistent throughout your application

## Related Components

- [Dialog] - More flexible dialog component
- [DialogHeader] - Dialog header component
- [DialogTitle] - Dialog title component
- [DialogDescription] - Dialog description component
- [DialogFooter] - Dialog footer component
- [DialogAction] - Wrapper component for confirm/OK buttons
- [DialogClose] - Wrapper component for cancel/close buttons

[AlertDialog]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.AlertDialog.html
[Dialog]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.Dialog.html
[DialogHeader]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.DialogHeader.html
[DialogTitle]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.DialogTitle.html
[DialogDescription]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.DialogDescription.html
[DialogFooter]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.DialogFooter.html
[DialogAction]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.DialogAction.html
[DialogClose]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.DialogClose.html
