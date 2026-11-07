---
title: Window
description: Use GPUI Window for window-local input, focus, rendering, and asynchronous work.
order: -2.3
---

# Window

GPUI provides [`Window`](https://docs.rs/gpui-pre/{{gpui_pre_version}}/gpui/struct.Window.html) as the context for one platform window. It connects the rendered Element tree to [platform input](./event#pointer-and-keyboard-input-are-also-events), Focus, Action dispatch, drawing, and window controls. A View receives it only while GPUI is updating or rendering that window:

```rust
impl Render for Chat {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active = window.is_window_active();

        div()
            .track_focus(&self.focus_handle)
            .when(!active, |this| this.opacity(0.8))
            .child("Chat")
    }
}
```

Keep application state in an [Entity](./entity). Use `Window` when an operation belongs to the current window or needs its current interaction state.

`App` gives access to application-wide services, [globals](./global), and entities. [`Context<Self>`](./context) adds operations tied to the current Entity, including `cx.notify()`, listeners, events, and tasks. `Window` carries focus, dispatch, input, [keyed element state](./element_id), measurement, and drawing for **one** window. These are temporary callback contexts; store an `Entity`, `FocusHandle`, task, subscription, or window handle for later work, never `&mut Window` or `&mut Context<_>`.

## Open and own a window

Call `gpui_kit::init(cx)` before opening windows. `gpui_kit::open_window` creates a GPUI window with a Base `Root` around the view returned by the builder. It returns both a window handle and the application content Entity, so the app can retain the part it owns:

```rust
use gpui_kit::*;

application().run(|cx| {
    init(cx);
    let (window_handle, workspace) = open_window(
        WindowOptions::default(),
        cx,
        |window, cx| cx.new(|cx| Workspace::new(window, cx)),
    )
    .expect("open workspace window");

    // Retain the handles in an application owner if later work needs them.
});
```

[`WindowOptions`](https://docs.rs/gpui-pre/{{gpui_pre_version}}/gpui/struct.WindowOptions.html) controls initial bounds, focus, visibility, window kind, minimum size, and other platform-facing choices. The builder receives the `Window` only for construction. A window handle lets later code request an update, but handle-based updates can fail after the window closes. In a multi-window app, use the handle for the particular window whose focus or geometry you mean; an Entity handle alone does not select a window.

`open_window` returns an `AnyWindowHandle` because the actual GPUI root is `gpui_kit::base::Root`, not `Workspace`. From a later callback with `&mut App`, use the handle to enter that window, and check the result before assuming it is still open:

```rust
if window_handle
    .update(cx, |_, window, _| window.activate_window())
    .is_err()
{
    // The window has already closed.
}
```

The first callback argument is the Base `Root` view; keep the `workspace` Entity returned by `open_window` for application content updates. A window handle selects the window, while an Entity selects the state to update.

## Try it: update and close one window

This exercise uses the existing `hello_world` package. Replace `examples/hello_world/src/main.rs` with the following code, then run `cargo run -p hello_world` from the repository root:

```rust
use gpui_kit::component::button::*;
use gpui_kit::component::*;
use gpui_kit::*;

struct WindowPractice {
    renamed: bool,
}

impl Render for WindowPractice {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = if self.renamed {
            "Title: Second title"
        } else {
            "Title: First title"
        };

        div()
            .v_flex()
            .gap_2()
            .p_4()
            .child(status)
            .child(
                Button::new("rename")
                    .label("Change title")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.renamed = !this.renamed;
                        let title = if this.renamed {
                            "Second title"
                        } else {
                            "First title"
                        };
                        window.set_window_title(title);
                        cx.notify();
                    })),
            )
            .child(
                Button::new("close")
                    .label("Close window")
                    .on_click(|_, window, _| window.remove_window()),
            )
    }
}

fn main() {
    application().run(|cx| {
        gpui_kit::init(cx);
        open_window(WindowOptions::default(), cx, |window, cx| {
            window.set_window_title("First title");
            cx.new(|_| WindowPractice { renamed: false })
        })
        .expect("open practice window");
    });
}
```

Click **Change title** twice. The native window title and the text inside the View should alternate together. `renamed` is persistent Entity state; `cx.notify()` makes its new text visible. `window.set_window_title(...)` changes the current native window directly and does not need that Entity notification. Click **Close window** to request removal of this window. The button callback's `&mut Window` is valid only during that callback; after removal, a saved window handle may return an error on update.

If the content changes but the title does not, check whether your desktop displays native window titles and whether `set_window_title` runs in the button callback. If neither changes, confirm `init(cx)` ran before `open_window`, the button has its `on_click` listener, and the listener calls `cx.notify()` after changing `renamed`. Restore the original `main.rs` after the exercise. For more than one window and ownership across them, continue with [Multi Window](./multi-window).

## What belongs to Window

Common window-local operations include:

| Need | API |
| --- | --- |
| Inspect geometry and state | `bounds`, `viewport_size`, `scale_factor`, `is_window_active` |
| Manage Focus | `focused`, `focus`, `blur`, `focus_next`, `focus_prev` |
| Send a command from code | `dispatch_action` |
| Redraw or schedule a frame callback | `refresh`, `request_animation_frame`, `on_next_frame` |
| Control the native window | `set_window_title`, `activate_window`, `remove_window` |
| Continue work later | `defer`, `spawn` |

`Window` also carries layout, text, hit testing, input, and drawing state internally. Most Views do not manipulate those systems directly; Elements and GPUI use them during rendering.

## Geometry and scale

`window.bounds()` returns the native window rectangle in **global** coordinates, potentially spanning displays. `window.viewport_size()` returns the drawable content area's size in window-local logical `Pixels`. For a local overlay that must fit inside the content, use the viewport size; for saved placement, use `window.window_bounds()`, which includes the window's restorable state. `window.inner_window_bounds()` excludes platform insets where supported.

`window.scale_factor()` converts logical pixels to physical display pixels: a factor of `2.0` means one logical pixel covers two device pixels along each axis. It may change when the window moves between displays. Do not multiply GPUI layout sizes by it; use it at a boundary that actually needs device pixels, such as a native platform integration. `visual_viewport_bounds()` can shrink or move when a mobile keyboard appears, while `viewport_size()` remains the layout area.

These values have different origins: `bounds().origin` is global display space, while pointer events and `visual_viewport_bounds()` use window-local logical coordinates. Do not compare a pointer position directly with a saved global window origin. For an overlay that must stay clear of system insets or the software keyboard, `window.fully_visible_bounds()` gives a conservative window-local rectangle; it cannot account for obscuring surfaces the platform does not report.

The [Dialog implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/dialog/dialog.rs) uses `window.viewport_size()` and window border padding to keep a surface within available content. The [native menu integration](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/native_menu/windows.rs) reads `scale_factor()` at its platform coordinate boundary. Let standard components perform these calculations when they already own the overlay or native control.

## Focus and Action dispatch

Focus is local to a Window. `window.focus(...)` selects a `FocusHandle`, and `window.focused(cx)` returns the current one. Attach that handle to a rendered Element with `.track_focus(&handle)` so it has a node on the Dispatch Path; a handle alone does not create a keyboard target. A tracked handle is not automatically in Tab order: opt in with `cx.focus_handle().tab_stop(true)` when creating it. Keyboard input then uses the focused Element's Dispatch Path to match a [KeyBinding](./keybinding) and dispatch its Action.

Follow the [Focus tutorial](./focus) to build and verify the target, Tab order, and overlay restoration before adding shortcuts.

```rust
fn focus_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    window.focus(&self.composer_focus, cx);
}

fn on_action_open_conversation(
    &mut self,
    action: &OpenConversation,
    window: &mut Window,
    cx: &mut Context<Self>,
) {
    self.open(action.id, cx);
    window.focus(&self.composer_focus, cx);
}
```

Use `window.dispatch_action(action.boxed_clone(), cx)` when a button, command palette, native menu, or another piece of code should issue the same command as a KeyBinding. GPUI captures the current Focus and defers the actual dispatch until the current effect cycle completes.

```rust
Button::new("open-conversation")
    .label("Open")
    .on_click(|_, window, cx| {
        window.dispatch_action(Box::new(OpenConversation { id }), cx);
    })
```

The Action still follows the focused Dispatch Path. Place its `on_action_*` handler on that path, usually on the focused region or a common owner. See [Action](./action) for the complete routing model.

## Run work after the current update

Use `window.defer` when work must wait until entities currently being updated have been released. This is common when closing an overlay changes Focus, or when the next operation updates another part of the same UI tree.

```rust
fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let composer = self.composer.clone();

    window.defer(cx, move |window, cx| {
        let focus_handle = composer.read(cx).focus_handle(cx);
        window.focus(&focus_handle, cx);
    });
}
```

Inside an Entity, `cx.defer_in(window, ...)` is often more convenient because GPUI supplies that Entity again:

```rust
cx.defer_in(window, |this, window, cx| {
    this.rebuild_results(window, cx);
});
```

The callback already receives `&mut Self`. Do not call `update` on the same Entity from inside it; that attempts to update an Entity which is already being updated. Use `defer` and `defer_in` for Focus changes and UI-tree mutations that cannot safely happen in the current callback.

Use `window.on_next_frame(...)` only when the operation specifically belongs to the next frame callback, such as an animation step. `defer` means “after the current effect cycle,” which is a different boundary.

## Redraw and frame lifecycle

An Entity mutation followed by `cx.notify()` marks that Entity for rendering. `window.refresh()` marks the **whole window** dirty for its next draw; use it for window-local changes that lack an Entity notification, such as a platform or overlay state change. Neither belongs in an unconditional render path.

`window.on_next_frame(callback)` runs the callback at the next platform frame tick, before that tick's optional draw. It creates frame demand but does not itself mark the window dirty. `window.request_animation_frame()` captures the currently rendering View and notifies it on the next tick. In the pinned `gpui-pre` {{gpui_pre_version}} implementation, it calls `current_view()` immediately, so use it only while GPUI has a current View; outside that render path, use `on_next_frame` and explicitly notify an Entity or call `window.refresh()`. Call it only while the motion still needs another sample. GPUI's `AnimationExt::with_animation` and [Base Motion](./animation) already manage frame requests and reduced motion for their animations.

For a frame callback that changes external window state, call `window.refresh()` in that callback so the change reaches a draw. For Entity state, update the Entity and call `cx.notify()` in its update callback. A frame tick alone does not redraw an unchanged window.

Rendering builds a fresh Element tree from retained Entity state, then GPUI resolves layout, prepaints input geometry, and paints the scene. `Window` has methods for all these stages, but application views should normally derive elements in `render`; custom Elements need later-stage hooks only when resolved bounds are required. An unconditional `cx.notify()`, `window.refresh()`, or `window.request_animation_frame()` in `render` creates continuous work even when the UI is idle. See [Render](./render) and [Element](./element).

## Async work with a Window

Use `cx.spawn_in(window, ...)` when a [Task](./task) belongs to the current Entity and later needs both Entity and Window access:

```rust
struct Chat {
    load_task: Option<Task<()>>,
}

fn load_conversation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.load_task = Some(cx.spawn_in(window, async move |this, cx| {
        let Ok(messages) = fetch_messages().await else { return };

        this.update_in(cx, |this, _window, cx| {
            this.messages = messages;
            cx.notify();
        })
        .ok();
    }));
}
```

`cx.spawn_in` provides a `WeakEntity<Self>` and an `AsyncWindowContext`. If the Entity or Window has gone away, `update_in` returns an error; propagate or handle it instead of assuming they still exist.

Use `window.spawn(cx, ...)` when the task needs the Window but does not belong to one Entity. Use `cx.spawn(...)` when no Window access is needed, and `cx.background_spawn(...)` for CPU-heavy work. A `Task` is cancelled when dropped, so store it on the owning View when its lifetime should follow that View, or call `.detach()` only for work that should continue independently.

## Subscribe with Window access

Use `cx.subscribe_in` when an Event callback needs `&mut Window`, for example to restore Focus after a child finishes:

```rust
struct Workspace {
    chat: Entity<Chat>,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    fn new(chat: Entity<Chat>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _subscriptions = vec![
            cx.subscribe_in(&chat, window, |_this, chat, event, window, cx| {
                if let ChatEvent::ConversationOpened = event {
                    window.focus(&chat.read(cx).focus_handle(cx), cx);
                }
            }),
        ];

        Self { chat, _subscriptions }
    }
}
```

Store the returned `Subscription` on the subscribing View. Dropping a local variable immediately cancels the subscription. Storing it on a longer-lived global owner can keep the callback and captured resources alive after the View disappears, causing a memory leak. See [Event](./event) for subscription ownership and multiple subscribers.

## Window lifetime

Do not store `&mut Window`; it is a temporary context supplied by GPUI. For later work, use `defer`, `spawn_in`, or obtain `window.window_handle()` and update it through GPUI. A handle does not keep a closed window alive, so handle-based updates can fail and should be treated accordingly.

`window.remove_window()` requests removal from the current update. To decide whether a platform close request may proceed, register `window.on_window_should_close(cx, callback)` and return `false` to cancel it; the application owns any unsaved-work confirmation flow. To observe a completed close, `cx.on_window_closed(callback)` returns a `Subscription` whose callback takes `&mut App` and `WindowId`, in that order. Retain that subscription on an application owner. The closed `Window` is already inaccessible when this callback runs, so gather any needed window state before closing it.

Register the close guard while that window is available, typically in the `open_window` builder. It applies to the platform's close request; `remove_window()` is an explicit programmatic removal. If the app should exit when its last window closes, use the closed callback to check `cx.windows().is_empty()` and call `cx.quit()`. The [FPS monitor example](https://github.com/longbridge/gpui-kit/blob/main/examples/fps_monitor/src/main.rs) shows this single-window quit pattern and a View requesting animation frames. Run it from this repository with `cargo run -p fps_monitor`; it uses GPUI Kit without the optional Component layer.

If a window action appears to do nothing, first check that the handle still names an open window and that the focused Element's Dispatch Path contains the Action handler. If an Entity mutation is not visible, verify that its update calls `cx.notify()`; if a window-level change is not visible, call `window.refresh()`. If a frame callback fires without a draw, remember that `on_next_frame` creates frame demand but does not dirty the window. If an Event callback stops firing, check that its `Subscription` is retained.

Keep these ownership rules together:

- persistent UI state belongs to an Entity;
- window-specific work receives `&mut Window` only for the duration of a callback;
- `Task` and `Subscription` fields tie background work and observers to the owning View;
- Focus and Action dispatch always use the state of the specific Window.

Continue with [Multi Window](./multi-window) when an application owns several windows. [Native Extensions](./native-extension) covers OS APIs at a window boundary; [WebView](./webview) explains the native child-view integration and its limits. For OS-delivered notifications, see [SystemNotification](./system-notification).
