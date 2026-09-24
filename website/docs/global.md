---
title: Global
description: Share application-wide state with GPUI Global, and connect changes to Views and windows.
order: -2.632
---

# Global

`Global` marks a Rust type that GPUI can store once per [`App`](./context). It is useful for a setting or service shared by several features and windows. The value is keyed by its concrete Rust type, so `AppSettings` and [`Theme`](../component/theme) occupy different slots. `Global` is an empty marker trait with a `'static` bound; it does not make the value a View or create an event stream by itself.

```rust
use gpui_kit::*;

struct AppSettings {
    notifications_enabled: bool,
}

impl Global for AppSettings {}
```

The slot belongs to this `App`, not to a particular [`Window`](./window) or [`Entity`](./entity). Initialize it once at application startup, before Views read it. A second `set_global` for the same type replaces the previous value; it does not merge fields.

## Read and change a global

| API | Result | Use |
| --- | --- | --- |
| `cx.has_global::<T>()` | `bool` | Check whether the slot exists. |
| `cx.global::<T>()` | `&T` | Read a required value; panics if missing. |
| `cx.try_global::<T>()` | `Option<&T>` | Read an optional value. |
| `cx.set_global(value)` | `()` | Install or replace a value and notify global observers. |
| `cx.global_mut::<T>()` | `&mut T` | Edit an installed value and notify global observers. |
| `cx.update_global::<T, _>(\|value, cx\| …)` | closure result | Edit an installed value while also using its GPUI context; notifies observers when the edit finishes. |
| `cx.default_global::<T>()` | `&mut T` | Read or install `T::default()` mutably; requires `T: Default`. |
| `cx.update_default_global::<T, _>(\|value, cx\| …)` | closure result | Update a value, installing its default first if needed. |
| `cx.remove_global::<T>()` | `T` | Remove an installed value and notify global observers. |

`global_mut`, `update_global`, and `remove_global` require an existing value. A read does not notify observers. The mutable and defaulting APIs schedule a notification even if the code leaves the value unchanged, so compare values before writing when redundant work matters. `remove_global` also notifies: an observer that reads the slot after removal must use `try_global` or handle absence another way. References returned by `global`, `global_mut`, and `default_global` are tied to the current GPUI call; copy or clone data needed later.

`update_global` temporarily lends out the global so its closure can hold `&mut T` and `&mut cx` at the same time. Use the `value` passed to the closure. Do not try to read or update that same global again inside the closure.

```rust
cx.update_global::<AppSettings, _>(|settings, _cx| {
    settings.notifications_enabled = false;
});
```

[`Context<T>`](./context) can call these application APIs because it dereferences to `App`. Code outside an Entity, such as application initialization, receives `&mut App` directly.

## App scope and Window scope

Choose a global when all windows should see the same value: for example, an application preference, a theme, or a shared service handle. Keep focus, input dispatch, bounds, and other window-specific behavior in [Window](./window). A global is one slot for the entire app; putting a separate selection for each window into one global makes window ownership and cleanup harder.

Keep a feature's business state in an Entity owned by that feature's crate or view. Reserve `Global` for services, settings, and coordination genuinely shared across the application; difficulty passing data between modules is not a reason to move a large business collection into an app-wide slot. When features need to collaborate, pass a lightweight Entity handle where the ownership boundary permits it, or use an explicit interface, command, or event. The [Coding Guides](./coding-guides) explain how to keep each feature's model and workflow behind its module boundary.

GPUI Kit's `Theme` illustrates application-wide ownership. After `gpui_kit::init(cx)`, components read the active theme through `cx.theme()`. GPUI Kit also keeps derived theme data in sync for its lower layers and refreshes windows when the theme changes. Use `Theme::change(...)` for a mode change or `Theme::update(cx, |theme| { … })` for an edit; a raw `Theme::global_mut(cx)` edit does not perform that synchronization or refresh every window. This is a theme-specific rule on top of GPUI's ordinary `Global` behavior.

## Make changes visible

Changing a global notifies **global observers**. GPUI queues these notifications as effects and coalesces repeated touches of the same type while a notification is pending. The callback sees the current value, not an intermediate snapshot or a field-level change. A global update does not automatically call `cx.notify()` on every Entity that happened to read it. A View whose rendered output depends on a global can register `cx.observe_global::<T>(...)`, then notify itself in the callback. Keep the returned `Subscription` in that View so the observer lives as long as the View.

```rust
use gpui_kit::*;
use gpui_kit::assets::Assets;
use gpui_kit::component::button::Button;

struct AppSettings {
    notifications_enabled: bool,
}

impl Global for AppSettings {}

struct SettingsView {
    _settings_observer: Subscription,
}

impl SettingsView {
    fn new(cx: &mut Context<Self>) -> Self {
        let _settings_observer = cx.observe_global::<AppSettings>(|_this, cx| {
            cx.notify();
        });
        Self { _settings_observer }
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let enabled = cx.global::<AppSettings>().notifications_enabled;

        div().child(
            Button::new("toggle-notifications")
                .label(if enabled { "Disable notifications" } else { "Enable notifications" })
                .on_click(|_, _window, cx| {
                    cx.update_global::<AppSettings, _>(|settings, _cx| {
                        settings.notifications_enabled = !settings.notifications_enabled;
                    });
                }),
        )
    }
}

fn main() {
    application()
        .with_assets(Assets)
        .run(|cx| {
            init(cx);
            cx.set_global(AppSettings {
                notifications_enabled: true,
            });

            open_window(WindowOptions::default(), cx, |_, cx| {
                cx.new(SettingsView::new)
            })
            .expect("failed to open window");
        });
}
```

The button edits the app-level setting. The observer marks this View dirty, so its next render reads the new value. This notification is especially necessary if the View is [cached](./view-cache): a parent redraw alone can replay its old subtree. Other windows with their own `SettingsView` can observe the same global and update too. When `SettingsView` is dropped, its stored `Subscription` is dropped and the callback is disconnected. `observe_global` reports that the type was touched, without a typed change payload; use an [Event](./event) from an Entity when consumers need a specific operation or payload.

If a callback also needs its window, use `cx.observe_global_in::<T>(window, ...)` and store its `Subscription`. For a window-level observer without an owning Entity, `window.observe_global::<T>(cx, ...)` provides `&mut Window` and `&mut App` to the callback. Keep that subscription with a suitable owner as well. `window.refresh()` or `cx.refresh_windows()` explicitly requests rendering; neither is required for the View above because its observer calls `cx.notify()`.

## Common mistakes

- Calling `cx.global::<T>()` before installing `T`: it panics. Initialize first or use `try_global` for an optional slot.
- Assuming `set_global` or `update_global` redraws every reader: register a global observer and notify dependent Views, or use an API such as GPUI Kit's theme update that explicitly refreshes windows.
- Dropping the `Subscription` returned by `observe_global` at the end of a constructor: the observer stops immediately.
- Putting per-window focus, selection, or document state in a single app global: give it a window or Entity owner, or store explicitly keyed state when application-wide coordination is actually required.
- Writing GPUI Kit's theme through `global_mut` and expecting all theme projections and windows to update: use its theme APIs.
- Mutating a global during `render`: render can run again for many reasons. Handle input or other effects outside rendering, then let observation invalidate the View.
