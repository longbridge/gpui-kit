---
title: Multi Window
description: Open multiple GPUI Kit windows, share application state, route work to the right window, and handle closing and restoration.
order: -2.301
---

# Multi Window

A GPUI application can own several windows. Each window has its own `Window` context, focus, input dispatch, geometry, and GPUI Kit `Root`. Application data can be shared across them. Read [Window](./window) for the single-window API; this guide builds a second window and explains what each window owns.

## Start from an empty project

Install the platform requirements in [Installation](./installation.md), then create an application:

```sh
cargo new gpui-multi-window
cd gpui-multi-window
```

In `Cargo.toml`, add the same single dependency used by [Getting Started](./getting-started.md):

```toml
[dependencies]
gpui-kit = "0.6"
```

Replace `src/main.rs` with the complete example below. If you already have a single-window application, keep its `application().run(...)` and `init(cx)` calls; create the shared model once in that startup closure, then call `open_window` a second time with a new content view. The loop below performs both calls explicitly, once for each name.

## Open two windows over one model

Call `gpui_kit::init` once before either window. Each call to `gpui_kit::open_window` creates a window and wraps the returned view in its own Base `Root`. The function returns `(AnyWindowHandle, Entity<V>)`, where `V` is the application view passed to the builder. Return the content Entity from the builder; do not wrap it in another `Root`. The `Root` supplies window-level overlays, menus, notifications, and focus coordination.

This example shares one counter Entity but creates a separate `Workspace` Entity for each window. The observer makes changes to the shared Entity visible in both windows; `local_clicks` remains independent.

```text
Application
  SharedCounter (one Entity)
    ├── First window  → Root → Workspace (first Entity, first observer)
    └── Second window → Root → Workspace (second Entity, second observer)
```

Each `Workspace` holds a clone of the same Entity handle. Cloning an `Entity<T>` handle does not copy its `T` value. Both observers watch the same model, but each observer belongs to one window's content view.

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

Run `cargo run` in the new project. Two windows should open. Click **Increment** in either window: the shared count changes in both, while each window's click count changes only when its own button is clicked. Close one window and use the other; its count and button remain available. The `SharedCounter` was created once, outside the loop, while `Workspace::new` runs once per window.

Use this sequence to check which state changed:

| Action | First window | Second window |
| --- | --- | --- |
| Start | Shared 0; local 0 | Shared 0; local 0 |
| Click **Increment** in the first window | Shared 1; local 1 | Shared 1; local 0 |
| Click **Increment** in the second window | Shared 2; local 1 | Shared 2; local 1 |
| Close the first window, then click in the second | Closed | Shared 3; local 2 |

Trace the first click through the code: the button listener increments the first `Workspace.local_clicks`; `shared.update` mutates the one `SharedCounter` and calls its `cx.notify()`; both stored observers receive that notification and call `cx.notify()` for their own views. Their next renders read the same count. The first view also calls `cx.notify()` after changing its local count. Reading `self.shared.read(cx)` in `render` supplies a value for that render; the observer is the connection that schedules later renders when the model changes.

To open a third independent workspace over the same counter, add `"Third window"` to the `for name in [...]` array and rerun. All three shared labels should advance after one click, while only the clicked window's local label advances. If you instead create `SharedCounter` inside the loop, each window gets its own model and this observable behavior changes.

Keep document or session data in a feature-owned Entity shared by the windows that need it. Keep window selection, focus handles, overlay state, and window-scoped tasks in that window's view. Use an application [Global](./global) for genuinely app-wide settings or services, not as a catch-all owner for every window's UI state. A shared Entity notification reaches only views that observe it; reading shared state in `render` alone does not subscribe that view to changes. Store the returned `Subscription` on the observing view: dropping it would stop updates, and retaining it in an application owner would outlive the window unnecessarily.

## Address the intended window

`gpui_kit::open_window` returns an `AnyWindowHandle` because its actual root view is GPUI Kit's `Root`, not the content view. Keep the returned content `Entity<V>` when you need its state. Keep the window handle when later work needs that window's focus, Action dispatch, activation, or geometry. `window.window_handle()` obtains the current window's handle inside a callback. The startup example discards both return values because its buttons act on their own views; a document switcher or window registry would retain them.

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

`window.remove_window()` requests removal of the current window; it does not express a whole-app quit policy. Register `window.on_window_should_close(cx, |window, cx| { ... })` on each window that may need to cancel a platform close request; return `false` to prevent that close. A direct `remove_window()` call is an application decision to remove the window, so perform any confirmation before calling it. Capture or persist window-specific data before closing: `cx.on_window_closed(...)` runs after the `Window` is inaccessible and receives its `WindowId` for registry cleanup.

For a desktop app that should quit when the last window closes:

```rust
cx.on_window_closed(|cx, _closed_id| {
    if cx.windows().is_empty() {
        cx.quit();
    }
})
.detach(); // Deliberately keep this app-wide observer for the app lifetime.
```

Alternatively, retain the returned `Subscription` in an application owner and drop it with that owner. The basic example needs no close observer: closing either window leaves the other usable. If your app can reopen a window from the dock or tray, choose that policy instead of quitting. A registry keyed by `WindowId` can remove the closed handle in this callback. Window-owned `Task` and `Subscription` fields should drop with the corresponding view; do not keep a closed window's view alive through an app-wide collection accidentally.

The close policy has two separate decision points. A platform close request reaches `on_window_should_close` while the window is still available; return `false` if the user must resolve unsaved work first. Once a window has actually closed, `on_window_closed` can remove its `WindowId` from a registry or decide whether the app should quit. If a button calls `remove_window()` directly, ask for confirmation before that call; the should-close callback is not a substitute for the button's own confirmation flow.

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

In a [`TestAppContext` test](./test), open two windows through `gpui_kit::open_window`, update the shared Entity, and assert both views change while window-local state stays independent. Close the first with `window.remove_window()`; its handle update should return an error, while the second window should still render and accept input. Run the repository's existing lifecycle coverage with `cargo test -p gpui-kit --features test-support --test lifecycle closing_one_window_preserves_other_window_and_owned_snapshot`; the [test source](https://github.com/longbridge/gpui-kit/blob/main/crates/kit/tests/lifecycle.rs) exercises the close boundary. For this example, also verify the two visible counts and close behavior with `cargo run` in the new project.

## Troubleshoot the example

| Observation | Check |
| --- | --- |
| Only one window opens | Check that `open_window` runs once per loop item and that each call succeeds. A panic from `.expect(...)` identifies an open failure. |
| The clicked window changes, but the other shared label stays stale | Keep the `Subscription` returned by `cx.observe` in each `Workspace`, and call `cx.notify()` inside the `SharedCounter` update. Reading the Entity alone does not register an observer. |
| Local click counts change in both windows | Create a distinct `Workspace` with `cx.new(...)` for each window; only `SharedCounter` should be constructed outside the loop. |
| An action updates the wrong window or fails after close | Keep the `AnyWindowHandle` returned for the intended window, use it for window-scoped work, and handle its `update` error after closure. Do not rely on `cx.windows()` order. |
| Closing one window exits the whole app | Inspect the app's last-window quit policy and any `on_window_closed` callback. The two-window sample registers no such callback. |
