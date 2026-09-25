---
title: SystemNotification
description: Send OS notifications with GPUI, handle activation, and understand GPUI Kit's Notification integration and platform limits.
order: -9.05
---

# SystemNotification

`SystemNotification` posts a message to the **operating system's notification center**. It is an application-level platform service, not an Element rendered inside a [Window](./window). GPUI Kit's [Notification component](../component/notification) normally shows an in-app toast, but it can also send to the system with `.system()` or to both destinations with `.in_app_and_system()`.

| Need | Use |
| --- | --- |
| Feedback visible in the current window | `window.push_notification(Notification::info(...), cx)` |
| OS notification with GPUI Kit's window activation and `on_click` handling | `window.push_notification(Notification::info(...).system(), cx)` |
| Direct access to the GPUI payload, tag, or action buttons | `cx.show_system_notification(SystemNotification { ... })` |

The raw GPUI API in the version pinned by this project (`gpui-pre 0.3.6`) is [`App::show_system_notification`](https://docs.rs/gpui-pre/0.3.6/gpui/struct.App.html#method.show_system_notification), `App::dismiss_system_notification`, and `App::on_system_notification_response`. The payload contains `tag`, `title`, `body`, and `actions`. `SystemNotificationAction` contains an `id` and user-visible `label`; a `SystemNotificationResponse` returns the `tag` and either the action's `id` or `None` when the notification body was activated.

## Set an application identity

Set a stable identifier and display name once, early in startup, before opening windows or posting notifications. Windows requires it for system notifications; other platforms can use the display name:

```rust
gpui_kit::application().run(|cx| {
    gpui_kit::init(cx);
    cx.set_app_identity("com.example.exporter", "Exporter");
    // Open application windows here.
});
```

On macOS, run a bundled `.app` from a trusted location such as `/Applications`; a plain `cargo run` process cannot deliver these notifications. The first post may request system permission. Linux needs an XDG notification daemon. OS settings and authorization can suppress delivery without an error from `show_system_notification`.

## Send and replace a notification

From code with an `&mut App` (usually called `cx`), send a payload with a stable tag:

```rust
use gpui_kit::SystemNotification;

cx.show_system_notification(SystemNotification {
    tag: "export/report-42".into(),
    title: "Export complete".into(),
    body: "report.pdf is ready".into(),
    actions: Vec::new(),
});
```

Posting another notification with the same `tag` replaces the previous one **where the platform supports replacement**. `cx.dismiss_system_notification("export/report-42")` requests retraction of a pending or delivered notification; retraction is best-effort. In the current Linux adapter, retraction is unsupported and notifications age out. Its XDG implementation also does not implement tag-based replacement, so do not use a tag as a cross-platform guarantee of one visible notification.

The low-level `actions` field accepts `Vec<SystemNotificationAction>`. Some platforms may display a notification without action buttons. If an action is offered, its `id` is returned in `SystemNotificationResponse::action_id` when pressed.

## Handle a click in GPUI Kit

For a GPUI Kit application, use the component entry point when clicking the notification must activate the right window or invoke application code:

```rust
use gpui_kit::component::{WindowExt, notification::Notification};

struct ExportNotice;

window.push_notification(
    Notification::success("report.pdf is ready")
        .title("Export complete")
        .id::<ExportNotice>()
        .system(),
    cx,
);
```

The component maps its ID to a system tag and activates the application and originating window on click. Add `.on_click(...)` to run application code when that happens. Use `.in_app_and_system()` when the same message should also appear as an in-app toast. Removing the component notification explicitly requests system retraction; a toast's automatic timeout does not retract its system copy. With `.system()`, there is no toast, so `on_close` is not called.

**One callback owner:** GPUI keeps only one `App::on_system_notification_response` handler; registering a new handler replaces the old one. `gpui_kit::init(cx)` installs the component's handler. Do not register another handler afterward if you use component system notifications, or their click behavior will stop working. Raw `cx.show_system_notification(...)` calls can still post messages, but the component handler intentionally ignores their tags. GPUI Kit currently has no public hook for a second raw response listener alongside that handler. If you need raw action-button responses, you must own a single response handler and coordinate it with the component integration, or use only one of the two paths.

## Test the boundary

GPUI's `TestAppContext` exposes `shown_system_notifications`, `delivered_system_notifications`, `dismissed_system_notifications`, and `simulate_system_notification_response`. Use those to verify tag and response routing; then check real delivery under each target OS. The [Native Extensions](./native-extension) guide explains the same platform boundary pattern, and the [Notification component](../component/notification) documents toast presentation and delivery modes.
