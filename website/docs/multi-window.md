---
title: Multi Window
description: Open multiple GPUI Kit windows, share application state, route work to the right window, and handle closing and restoration.
order: -2.301
---

# Multi Window

A GPUI application can own several windows. Each window has its own `Window` context, Focus, input dispatch, geometry, and GPUI Kit `Root`. Application data can be shared across them. Read [Window](./window) for the single-window API; this guide covers the ownership decisions that appear when more than one window is open.

## Open two windows over one model

Call `gpui_kit::init` once. Each call to `gpui_kit::open_window` creates a window and wraps the returned view in its own Base `Root`. The function returns `(AnyWindowHandle, Entity<V>)`, where `V` is the application view passed to the builder. Do not wrap `V` in another `Root`.

This example shares one counter Entity but creates a separate `Workspace` Entity for each window. The observer makes changes to the shared Entity visible in both windows; `local_clicks` remains independent.

```rust
use gpui_kit::component::button::Button;
use gpui_kit::*;

struct SharedCounter {
    count: usize,
}

struct Workspace {
    shared: Entity<SharedCounter>,
    _shared_observer: Subscription,
    local_clicks: usize,
    name: &'static str,
}

impl Workspace {
    fn new(shared: Entity<SharedCounter>, name: &'static str, cx: &mut Context<Self>) -> Self {
        let _shared_observer = cx.observe(&shared, |_, _, cx| cx.notify());
        Self { shared, _shared_observer, local_clicks: 0, name }
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.shared.read(cx).count;

        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .child(format!("{}: shared count {count}", self.name))
            .child(format!("Clicks in this window: {}", self.local_clicks))
            .child(
                Button::new("increment")
                    .label("Increment")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.local_clicks += 1;
                        this.shared.update(cx, |shared, cx| {
                            shared.count += 1;
                            cx.notify();
                        });
                        cx.notify();
                    })),
            )
    }
}

fn main() {
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);
        let shared = cx.new(|_| SharedCounter { count: 0 });

        for name in ["First window", "Second window"] {
            let shared = shared.clone();
            open_window(WindowOptions::default(), cx, move |_, cx| {
                cx.new(|cx| Workspace::new(shared, name, cx))
            })
            .expect("open workspace window");
        }
    });
}
```

Keep document or session data in a feature-owned Entity shared by the windows that need it. Keep window selection, Focus handles, overlay state, and window-scoped tasks in that window's view. Use an application [Global](./global) for genuinely app-wide settings or services, not as a catch-all owner for every window's UI state. A shared Entity notification reaches only views that observe it; reading shared state in `render` alone does not subscribe that view to changes.

## Address the intended window

`gpui_kit::open_window` returns an `AnyWindowHandle` because its actual root view is GPUI Kit's `Root`, not the content view. Keep the returned content `Entity<V>` when you need its state. Keep the window handle when later work needs that window's Focus, Action dispatch, activation, or geometry. `window.window_handle()` obtains the current window's handle inside a callback.

An `AnyWindowHandle` exposes `window_id()` and `update(...)`. `cx.windows()` enumerates open handles; `cx.active_window()` returns the platform-focused one when available. If you already know the target window, retain its handle instead of choosing one by iteration order. A handle can also be downcast to `WindowHandle<gpui_kit::base::Root>` when a typed root handle is required; downcasting it to `WindowHandle<Workspace>` fails because `Workspace` is content inside `Root`.

```rust
// `target` is the AnyWindowHandle returned by open_window.
target.update(cx, |_, window, _cx| {
    window.activate_window();
    window.set_window_title("Document");
})?;
```

The `update` result must be handled because the target may have closed. Focus and Action dispatch run in the selected window's context. For an Entity update that also needs its window, use `cx.update_window(target, |_, window, cx| { ... })` and update the content Entity inside that callback. For an async task bound to one window, use [`cx.spawn_in`](./task) and handle a failed `update_in` after the window or Entity disappears.

## Close and clean up

`window.remove_window()` closes the current window; it does not express a whole-app quit policy. Register `window.on_window_should_close(cx, ...)` in that window when unsaved work may cancel a platform close request. Capture or persist window-specific data before closing: `cx.on_window_closed(...)` runs after the `Window` is inaccessible and receives its `WindowId` for registry cleanup.

For a desktop app that should quit when the last window closes:

```rust
cx.on_window_closed(|cx, _closed_id| {
    if cx.windows().is_empty() {
        cx.quit();
    }
})
.detach(); // Deliberately keep this app-wide observer for the app lifetime.
```

Alternatively, retain the returned `Subscription` in an application owner and drop it with that owner. If your app can reopen a window from the dock or tray, choose that policy instead of quitting. A registry keyed by `WindowId` can remove the closed handle in this callback. Window-owned `Task` and `Subscription` fields should drop with the corresponding view; do not keep a closed window's view alive through an app-wide collection accidentally.

## Restore placement

Before a window closes, read `window.window_bounds()` to obtain its restorable `WindowBounds` (`Windowed`, `Maximized`, or `Fullscreen`). Persist that value in your application settings, then pass it to the next `WindowOptions`:

```rust
let options = WindowOptions {
    window_bounds: Some(saved_bounds),
    ..Default::default()
};
open_window(options, cx, |window, cx| cx.new(|cx| Workspace::new(shared, "Restored", cx)))?;
```

Here `saved_bounds` is a `WindowBounds` captured from a prior window; saving and loading it is application code. Check restored placement against currently attached displays before using it, because display topology and scale may have changed. See [Window geometry](./window#geometry-and-scale) for the difference between global bounds and window-local viewport size.

## Test the boundaries

In a [`TestAppContext` test](./test), open two windows through `gpui_kit::open_window`, update the shared Entity, and assert both views change while window-local state stays independent. Close the first with `window.remove_window()`; its handle update should return an error, while the second window should still render and accept input. The repository's [multi-window lifecycle test](https://github.com/longbridge/gpui-kit/blob/main/crates/kit/tests/lifecycle.rs) exercises this closing behavior.
