---
title: SystemNotification
description: Send OS notifications with GPUI, handle activation, and understand GPUI Kit's Notification integration and platform limits.
order: -9.05
maturity: [platform-dependent]
---

# SystemNotification

`SystemNotification` posts a message to the **operating system's notification center**. It is an application-level platform service, not an Element rendered inside a [Window](./window). GPUI Kit's [Notification component](../component/notification) defaults to an in-app toast and can also post to the system with `.system()` or to both places with `.in_app_and_system()`.

| Need | Use |
| --- | --- |
| Immediate feedback while the user is working in the window | `window.push_notification(Notification::info(...), cx)` |
| A completed background task that may matter while the app is inactive | `window.push_notification(Notification::info(...).system(), cx)` |
| The same status in the window and the OS notification center | `window.push_notification(Notification::info(...).in_app_and_system(), cx)` |
| GPUI payload fields, a chosen tag, or OS action buttons | `cx.show_system_notification(SystemNotification { ... })` |

Use an in-app toast for a result the user is already watching; use a system notification when they may have switched away. Keep important state in the application itself: delivery may be disabled, suppressed, or delayed by the OS. For a failure requiring a decision, show an in-app error or dialog when the user returns instead of depending on a transient system message.

## Try the existing story first

From this repository, run the notification story and press **System only** or **In-app and system**:

```sh
cargo run -p gpui-component-story -- NotificationStory
```

The buttons are in `crates/story/src/stories/notification_story.rs`. The story startup sets an app identity in `crates/story/src/main.rs`. This exercises the existing GPUI Kit path without adding a crate. On macOS, a `cargo run` binary is not a bundled `.app`, so the system post is suppressed; use a bundled build for a real delivery check. The in-app half of **In-app and system** can still appear.

Walk through the **System notification** section in this order:

| Action | Expected observation |
| --- | --- |
| Press **System only**. | No toast appears inside the story. On a supported, permitted desktop installation, the OS receives **Build finished** with the body **Delivered straight to the notification center.** |
| Press **In-app and system**. | A toast remains in the story because this button sets `autohide(false)`. The OS post uses **Build finished** with the body **Shown as a toast and in the notification center.** |
| Switch to another application, then activate the OS notification. | The application and the original story window are requested to activate. The terminal running the story prints `[notification] system notification clicked`; an in-app counterpart, if present, closes. Window activation and notification presentation remain subject to OS policy. |

Both buttons use the same `SystemNotificationKind` ID. In-app pushes with that ID replace the existing toast; a later OS post with the same tag replaces an earlier one only where the platform supports tag replacement. The exercise checks the **component route**. It does not exercise raw `SystemNotification` action buttons. On macOS, the `cargo run` exercise can verify only the in-app portion; use a packaged, authorized app to check OS delivery and activation. If your desktop suppresses banners, also inspect its notification center before treating the post as missing.

## Initialize and identify the application

Call `gpui_kit::init(cx)` once at startup and set a stable identifier and display name before opening windows or posting. Windows needs an app identity for an unpackaged app; Linux can use its display name. The GPUI test platform also requires an identity, so set one in notification tests.

```rust
gpui_kit::application().run(|cx| {
    gpui_kit::init(cx);
    cx.set_app_identity("com.example.exporter", "Exporter");
    // Open application windows here.
});
```

| Platform | Delivery conditions and behavior |
| --- | --- |
| macOS | Run from a real `.app` bundle in a trusted location such as `/Applications`. A plain `cargo run` process cannot post. The first post can request notification permission; denial suppresses delivery. |
| Windows | Set the identity early, especially for an unpackaged app. Windows toast delivery and visibility also depend on OS notification settings. |
| Linux | A working session D-Bus and XDG notification daemon are needed. The adapter logs a warning if it cannot show a notification. Its current implementation neither retracts nor replaces by tag. |

GPUI's `show_system_notification` returns `()`, not a delivery result. A successful call therefore means only that the request was submitted to the platform adapter. A missing toast is not evidence that the application's background work failed.

Treat permission as part of the product flow: keep the task result in application state, and make it reachable when the user returns. The OS notification is a prompt to revisit that state, not the state itself. A denied permission, muted application, quiet mode, missing daemon, or a closed source window must not erase a completed export or its error.

## Post with the raw GPUI API

The project pins `gpui-pre {{gpui_pre_version}}`. Its [`App::show_system_notification`](https://docs.rs/gpui-pre/{{gpui_pre_version}}/gpui/struct.App.html#method.show_system_notification) accepts a `SystemNotification` with `tag`, `title`, `body`, and `actions`:

```rust
use gpui_kit::SystemNotification;

// In code that has an &mut App named cx:
cx.show_system_notification(SystemNotification {
    tag: "export/report-42".into(),
    title: "Export complete".into(),
    body: "report.pdf is ready".into(),
    actions: Vec::new(),
});
```

Choose a tag for the underlying task or item. Reposting that tag replaces the earlier notification **where the platform supports replacement**. `cx.dismiss_system_notification("export/report-42")` requests removal of a pending or delivered notification, also on a best-effort basis. In particular, the current Linux adapter does not implement either tag replacement or retraction, so duplicate or stale entries may remain until the daemon ages them out.

For OS buttons, put `SystemNotificationAction { id, label }` values in `actions`. `id` is returned as `SystemNotificationResponse::action_id` when that button is pressed; activating the body returns `None`. A platform may present the notification without its buttons. Register `cx.on_system_notification_response(|response, cx| { ... })` once to route raw responses by `response.tag` and `response.action_id`. The handler runs only for delivered user activations, not for posting failures or ordinary dismissal. Raw GPUI posting has no originating window to activate or domain callback to run automatically; the application must own that routing and ensure the target window still exists.

For a raw response handler, keep the tag tied to a durable task key, look up that task when the response arrives, and handle both a body click (`None`) and each action ID. A response can arrive after the task or window has gone away. Do not treat the handler as a completion callback for the background operation.

## Use GPUI Kit's window integration

When a click should return to the originating window, push a `Notification` through that window's `WindowExt`. Open the window through `gpui_kit::open_window` or wrap its view in `Root::new` so the notification overlay exists.

```rust
use gpui_kit::component::{notification::Notification, WindowExt};

struct ExportNotice;

window.push_notification(
    Notification::success("report.pdf is ready")
        .title("Export complete")
        .id::<ExportNotice>()
        .system()
        .on_click(|_, window, cx| {
            // Open the completed export in the owning window.
        }),
    cx,
);
```

The component maps its ID to a namespaced system tag and remembers the window that posted it. Use `.id1::<ExportNotice>(task_key)` when separate tasks need separate identities. It sends the title and message as OS text; if only a message is set, that message becomes the OS title. A content-only notification with neither title nor message does not post. Custom content and the component's `.action(...)` button belong to the in-app toast; component system posts have no OS action buttons.

On a recognized click, GPUI Kit requests retraction, activates the app, and then activates the originating window, closes its in-app counterpart if present, and calls `on_click`. The callback needs a live originating window. If that window has closed, the app can still activate, but there is no window callback. A response after restart or after the component's bounded routing entries have been pruned can likewise activate the app without invoking `on_click`; keep essential task state outside the notification callback. The component handles a response once, and ignores raw tags outside its namespace.

`.system()` creates no in-app toast, so `on_close` does not run. `.in_app_and_system()` creates both; automatic toast timeout does **not** retract the OS copy. Explicit `window.remove_notification::<ExportNotice>(cx)` or `window.clear_notifications(cx)` requests system retraction for notifications owned by that window, including system-only ones. If another window has since posted the same component ID, the first window's removal does not retract the newer post. OS retraction remains best-effort.

## Keep one response handler

GPUI keeps one `App::on_system_notification_response` handler: registering another replaces the previous one. With the default component feature, `gpui_kit::init(cx)` installs GPUI Kit's handler. Do not overwrite it if component system notifications need click behavior. Raw calls can still post, but that handler ignores their tags, so raw action responses will not reach application code. GPUI Kit does not expose a public way to chain a raw handler onto its component handler; choose one response ownership path for the application.

## Test and troubleshoot

`TestAppContext` exposes `shown_system_notifications()`, `delivered_system_notifications()`, `dismissed_system_notifications()`, and `simulate_system_notification_response(...)`. Set an identity in the test, inspect the posted title/body/tag, simulate a click, and assert the intended window callback or raw handler result. The test platform models replacement and retraction; it does **not** prove that a real OS daemon, permission prompt, or click activation works. Check those on every target OS.

If nothing appears, verify the application identity, bundle and permission status on macOS, Windows notification settings, or the Linux session daemon. If a click does nothing, check whether the notification was posted through the component or raw GPUI path, whether another response handler replaced GPUI Kit's handler, and whether the originating window still exists. For toast styling and lifecycle, see the [Notification component](../component/notification); for platform boundary design, see [Native Extensions](./native-extension).

| Symptom | Check next |
| --- | --- |
| **In-app and system** shows a toast but no OS entry. | The window integration is mounted. Check OS permission, quiet mode, and platform delivery prerequisites above. On macOS under `cargo run`, this is the expected result. |
| **System only** appears to do nothing. | This mode deliberately has no in-app toast. Check the notification center and the same platform prerequisites; the API has no delivery-result callback. |
| Clicking an OS entry activates the app but not the intended view. | Check that the originating window is still open and the component's in-memory routing entry still exists. Restore the view from application state on return. |
| An old entry remains after a newer post or explicit removal. | Check whether the platform supports tag replacement and retraction. The current Linux adapter supports neither. |
| A raw notification's action button has no application effect. | Check the single app-global response handler. The handler installed by `gpui_kit::init(cx)` routes GPUI Kit component tags and ignores raw tags. |
