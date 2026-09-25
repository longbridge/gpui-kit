---
title: Focus
description: Build keyboard reachable GPUI views with stable focus handles, focus events, and safe focus traps.
order: -2.4
---

# Focus

**Focus** identifies the target for keyboard input in one [Window](./window). GPUI uses the focused element's path through the rendered tree to route [Actions and key bindings](./action). A pointer press may move Focus, but drawing a control or giving it an `ElementId` does not. This guide uses the GPUI API published as `gpui-pre` 0.3.6 through `gpui-kit`; `gpui-pre` is the snapshot publishing and version alignment name, not a different rendering engine.

## Try the existing example

From the repository root, run:

```sh
cargo run -p focus_trap
```

The source is [`examples/focus_trap/src/main.rs`](https://github.com/longbridge/gpui-kit/blob/main/examples/focus_trap/src/main.rs). Click an outside button and press Tab: Focus follows the window's ordinary Tab order. Click a button in either numbered area and press Tab or Shift+Tab: Focus cycles among that area's buttons. The example calls `.focus_trap(id, &handle)` on each container; the buttons are real GPUI Kit `Button`s with their own focus handles. It demonstrates containment after entering an area, not a complete modal lifecycle.

For a new application, initialize Kit before opening a window with `gpui_kit::init(cx)`, as the example does. `gpui_kit::open_window` supplies the Base `Root` that handles normal Tab and Shift+Tab navigation and participates in Kit's trap handling. The [Getting Started](./getting-started) guide covers the application setup.

## Four separate operations

| Operation | What it does | What it does not do |
| --- | --- | --- |
| `cx.focus_handle()` | Creates a stable target that an owner can retain. | It does not register a rendered element or move Focus. Its default `tab_stop` is `false`. |
| `.track_focus(&handle)` | Registers that handle on a rendered interactive element. The element can then participate in Focus dispatch and receive its focused style. | It does not move Focus or automatically opt the handle into Tab order. |
| `handle.focus(window, cx)` or `window.focus(&handle, cx)` | Makes the handle the window's current Focus target. | It does not create a rendered node or a keyboard handler. |
| `cx.focus_handle().tab_stop(true)` | Includes a tracked handle in Tab navigation. | It does not focus the handle immediately. |

The handle belongs to the long lived owner, usually an [`Entity<T>`](./entity). The `Element` is rebuilt for each frame; attach the *same* handle again from [`Render::render`](./render). If you create a fresh handle on every render, Focus identity and Tab behavior can change underneath the user. For a stateless Kit component, retain a handle through `window.use_keyed_state(...)`; Kit's `Button` uses that pattern. An `ElementId` can key retained state, but it is not itself a Focus target.

### A minimal focusable view

The following view can be placed in the window builder shown in [Getting Started](./getting-started). It creates one Tab stop and displays whether it has exact Focus:

```rust
use gpui_kit::*;

struct FocusPanel {
    focus_handle: FocusHandle,
}

impl FocusPanel {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle().tab_stop(true),
        }
    }
}

impl Focusable for FocusPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FocusPanel {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let label = if self.focus_handle.is_focused(window) {
            "Focused"
        } else {
            "Press Tab or click here"
        };

        div()
            .track_focus(&self.focus_handle)
            .p_4()
            .child(label)
    }
}
```

Create it with `cx.new(FocusPanel::new)` in a window builder. `Focusable` exposes the handle to code that owns the Entity; it does not automatically call `.track_focus(...)`. A view can implement `Focusable` and still fail keyboard dispatch if its rendered tree omits the tracked element. To move Focus from a callback, call `self.focus_handle.focus(window, cx)`. Do that when opening or entering a region, not unconditionally in `render`: repeated requests from rendering can steal Focus from a child or another control.

## Pointer, Tab, and keyboard commands

For an element with `.track_focus(&handle)`, GPUI's default mouse down behavior focuses that handle when the pointer hits it. A nested tracked child gets the first opportunity to focus and suppresses an ancestor's default transfer; a custom mouse handler can use `window.prevent_default()` when it deliberately handles Focus itself. A pointer callback that only changes application state is not a keyboard interaction.

Tab and Shift+Tab visit *tab stops* registered in the most recently rendered frame. The default `FocusHandle` is not one. Set `.tab_stop(true)` when constructing a handle that users should reach by Tab. GPUI also has `FocusHandle::tab_index(n)` for explicit ordering, but prefer the rendered order for ordinary forms. When passing a handle to `.track_focus(...)`, set Tab configuration on the **handle**; an element's separate `.tab_index(...)` or `.tab_stop(...)` does not change that handle's settings.

Once Focus reaches a tracked element, its dispatch path determines which `key_context`, `on_action`, and key bindings apply. For example, an editor's Save binding works when the editor or a descendant owns Focus and the matching handler lies on that path. A sibling's handler is not on the path. See [Action](./action) and [KeyBinding](./keybinding) for a complete command example. A custom control must also provide a visible Focus treatment and the expected keyboard activation or navigation behavior; `track_focus` only supplies the routing target. See [Accessibility](./accessibility) for semantics and testing limits.

## Exact Focus versus Focus within a region

Use `handle.is_focused(window)` when only that handle should count as active. Use `handle.contains_focused(window, cx)` when the handle or a tracked descendant should count—for example, keeping a panel visually active while a text field inside it has Focus. Containment comes from the **most recently rendered dispatch tree**; both handles must be attached to elements in the expected parent and child relationship. A stored Entity relationship alone does not establish it.

```rust
let panel_itself = self.focus_handle.is_focused(window);
let panel_or_child = self.focus_handle.contains_focused(window, cx);
```

`window.focused(cx)` returns an `Option<FocusHandle>` for the current window. It is useful for diagnostics and for saving the previous target before opening an overlay. The returned handle may later outlive its rendered element, so validate the UI lifecycle before restoring it.

## Observe transitions without putting subscriptions in `render`

`Context<T>` provides `on_focus`, `on_blur`, `on_focus_in`, and `on_focus_out`. Register them while constructing the owner, and retain the returned [`Subscription`](./event) on that owner. Dropping a subscription unregisters it. The exact pair fires when the named handle gains or loses Focus; the `in`/`out` pair considers tracked descendants as well.

```rust
struct SearchPanel {
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
    active: bool,
}

impl SearchPanel {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let entered = cx.on_focus_in(&focus_handle, window, |this, _, cx| {
            this.active = true;
            cx.notify();
        });
        let left = cx.on_focus_out(&focus_handle, window, |this, _, _, cx| {
            this.active = false;
            cx.notify();
        });

        Self {
            focus_handle,
            _subscriptions: vec![entered, left],
            active: false,
        }
    }
}
```

Attach `focus_handle` to the panel's rendered root with `.track_focus(&self.focus_handle)`. This pattern is appropriate when another state change must follow Focus; a simple Focus style can often read `is_focused` during rendering instead. Registering listeners during every render would accumulate duplicate callbacks.

## Trap and restore Focus in an overlay

GPUI Kit adds `.focus_trap(id, &container_handle)` to an interactive container. The Base `Root` checks whether the current Focus is inside that container when it handles Tab or Shift+Tab. If normal navigation would leave, it searches for another Tab stop inside the container. The trap's `id` must be stable, its handle must survive rendering, and the container must contain usable child Tab stops.

```rust
div()
    .child(first_button)
    .child(second_button)
    .focus_trap("settings-dialog", &self.dialog_focus_handle)
```

**A trap alone does not open a modal, enter its first control, block pointer focus outside it, or restore Focus after dismissal.** For a modal flow:

1. Save `window.focused(cx)` when the user opens the overlay. Keep the saved handle with the overlay owner.
2. Render the overlay and move Focus to an enabled control inside it when that control exists. Choose a predictable first target; do not rely on the trap to perform this step.
3. On dismissal, remove the overlay and restore the saved target only if it is still an appropriate rendered target. If it has disappeared, choose a current fallback such as the control that opens the overlay. Coordinate the update and restoration so a closing overlay does not immediately take Focus back.
4. Test both Tab directions, Escape or explicit close, pointer clicks, disabled controls, nested overlays, and the case where the previous target disappears.

GPUI Kit's `Dialog` and `Sheet` components provide their own modal focus behavior; use them for ordinary modal UI. The manual trap is useful for a custom surface whose entry, dismissal, and restoration lifecycle you explicitly own. An `on_focus_out` listener on a container can help observe Focus leaving; it is not a substitute for the modal lifecycle.

## Verify and debug

Start with the `focus_trap` example to see pointer entry and both Tab directions in a running window. In an application test, render the real view, click its intended control, send Tab or the command key, and assert the resulting owner state. The [Testing](./test) guide covers Kit's UI integration test helpers; focus scopes need an explicit tracked handle for reliable inspection.

| Symptom | Check |
| --- | --- |
| Tab skips the custom view | The retained handle has `.tab_stop(true)` and its element calls `.track_focus(&handle)` in the current frame. |
| Click focuses a parent instead of a child | The child has its own tracked handle; inspect mouse handlers that call `prevent_default`. |
| A shortcut works only after clicking | The intended handle is focused, the `key_context` matches, and the handler lies on the focused dispatch path. |
| Panel appears inactive when a child is focused | Use `contains_focused`, and verify the child's tracked element is nested under the panel's tracked element. |
| Tab escapes a custom trap | Focus entered the trap first; the container is rendered through Kit's Base `Root`; children are actual Tab stops. |
| Focus disappears after closing an overlay | Save the previous target and restore an existing rendered target after dismissal. |

Related guides: [Window](./window), [Action](./action), [KeyBinding](./keybinding), [Accessibility](./accessibility), and [Testing](./test).
