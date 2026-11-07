---
title: Event
description: Use GPUI Events for typed notifications and connect them to Actions.
order: -2.63
---

# Event

GPUI provides **Event** as a typed notification mechanism between [Entities](./entity). An Event reports something that already happened; unlike an [**Action**](./action), it does not use Focus, Key Contexts, KeyBindings, or the Dispatch Path. GPUI also calls raw mouse and keyboard input values “events”; those follow different rules, covered below.

## Action in, Event out

An Action can cause the state change, but Event delivery starts after that change:

```text
Chat changes state → emit(MessageSent) → subscribers receive Event → Workspace updates
```

<img class="architecture-light" src="/event-subscriptions-flow.svg?v=20260922-1" alt="Chat emits one MessageSent Event to independent Workspace, Activity Log, and Telemetry subscribers">
<img class="architecture-dark" src="/event-subscriptions-flow-dark.svg?v=20260922-1" alt="Chat emits one MessageSent Event to independent Workspace, Activity Log, and Telemetry subscribers">

- [**Action**](./action) carries intent inward: “send this message.”
- **Event** reports the result outward: “this message was sent.”

The command owner handles the Action and changes its state. It then emits an Event so owners or services can react without being coupled to the command's UI entry point. See [Action](./action) for Focus and command dispatch, and [KeyBinding](./keybinding) for shortcut matching.

## A complete first Event

This small application has two Entities. `Chat` owns the count and emits a typed fact. `Workspace` owns the `Chat` handle, subscribes to that exact Entity, and renders the latest count. Clicking the button updates `Chat`; the subscription then updates `Workspace`.

Use the project from [Getting Started](./getting-started), which already depends on `gpui-kit`. Replace its `src/main.rs` with this complete program and run `cargo run` from that project directory. The window should start at **Messages sent: 0**; each click on **Send** should increase the count by one.

```rust
use gpui_kit::*;
use gpui_kit::assets::Assets;
use gpui_kit::component::button::Button;

#[derive(Clone, Debug)]
enum ChatEvent {
    MessageSent { total: usize },
}

struct Chat {
    sent: usize,
}

impl EventEmitter<ChatEvent> for Chat {}

impl Chat {
    fn send(&mut self, cx: &mut Context<Self>) {
        self.sent += 1;
        cx.emit(ChatEvent::MessageSent { total: self.sent });
    }
}

struct Workspace {
    chat: Entity<Chat>,
    shown_total: usize,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    fn new(cx: &mut Context<Self>) -> Self {
        let chat = cx.new(|_| Chat { sent: 0 });
        let subscription = cx.subscribe(&chat, |workspace, _chat, event, cx| {
            match event {
                ChatEvent::MessageSent { total } => workspace.shown_total = *total,
            }
            cx.notify(); // Workspace's visible count changed.
        });

        Self {
            chat,
            shown_total: 0,
            _subscriptions: vec![subscription],
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_4()
            .child(format!("Messages sent: {}", self.shown_total))
            .child(Button::new("send").label("Send").on_click(cx.listener(
                |workspace, _, _, cx| {
                    workspace.chat.update(cx, |chat, cx| chat.send(cx));
                },
            )))
    }
}

fn main() {
    gpui_kit::application().with_assets(Assets).run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(Workspace::new)
        })
        .expect("failed to open window");
    });
}
```

The bootstrap calls `gpui_kit::init` before opening the window; `open_window` supplies the application window. The button's callback updates `Chat`, where the state mutation and `cx.emit` happen together. The subscriber reads the Event payload and calls `cx.notify()` because the value rendered by `Workspace` changed.

`cx.emit(...)` queues an effect. GPUI delivers it after the current entity update can finish; the subscription callback is not a direct call inside `Chat::send`. Emit only after the operation succeeds. If the emitting Entity also renders changed state, call `cx.notify()` in that Entity as well: `emit` does not request a render. A model Entity such as `Chat` above has no view to redraw. Name Event variants as facts, such as `MessageSent`, `Saved`, and `Dismissed`; `SendMessage` names a command.

### Connect the Action to this Event

The complete example above begins at a button callback. To exercise the full [Action](./action) → state → Event → subscription route, edit the **same** `src/main.rs` in four places. Keep `Chat`, `ChatEvent`, `Chat::send`, and the existing subscription callback.

1. After the imports, add `actions!(chat, [SendMessage]);`. Add `focus: FocusHandle,` to `Workspace` beside its `chat` field.
2. Change `Workspace::new` to accept `window: &mut Window` before `cx`. Before the existing `let chat = ...` line, create and focus the handle, then add `focus,` to the returned `Self`:

   ```rust
   let focus = cx.focus_handle().tab_stop(true);
   focus.focus(window, cx);
   ```

3. Add this method to `impl Workspace`:

   ```rust
   fn on_send_message(&mut self, _: &SendMessage, _: &mut Window, cx: &mut Context<Self>) {
       self.chat.update(cx, |chat, cx| chat.send(cx));
   }
   ```

   In `render`, add the following three calls to the **outer** `div()` before its children, then replace only the button's `.on_click(...)` with the callback below:

   ```rust
   .track_focus(&self.focus)
   .key_context("Chat")
   .on_action(cx.listener(Self::on_send_message))

   .on_click(cx.listener(|workspace, _, window, cx| {
       workspace.focus.dispatch_action(&SendMessage, window, cx);
   }))
   ```

4. In `main`, after `gpui_kit::init(cx)`, bind Enter. Change the `open_window` closure to pass its window to the constructor:

   ```rust
   cx.bind_keys([KeyBinding::new("enter", SendMessage, Some("Chat"))]);
   gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
       cx.new(|cx| Workspace::new(window, cx))
   })
   .expect("failed to open window");
   ```

Run `cargo run` again. Press **Enter** while the Workspace region has focus, then click **Send**. Each input dispatches the same `SendMessage` Action to `Workspace::on_send_message`. That handler updates `Chat`; `Chat::send` increments `sent` and emits `MessageSent`; the retained subscription updates `Workspace::shown_total`. The visible count should rise once per input. As a check, temporarily remove `cx.emit(...)` from `Chat::send`: the internal `sent` value still rises, but the displayed count stops changing because `Workspace` listens for the Event. Restore the emit call before the next exercise. If Enter does nothing, check that the focus handle is attached to the rendered container and its `Chat` Key Context is present; if the button does nothing, inspect the Action handler and dispatch path.

## Subscription lifetime and ownership

`cx.subscribe(&chat, callback)` returns a `Subscription`. Keep it in the subscribing owner, as `Workspace` does above. Dropping the handle disconnects the callback, so a local handle that disappears at the end of `new` silently stops delivery. Dropping `Workspace` also drops its handles. An `Entity<Chat>` clone only identifies the source; retaining it does not retain a subscription. Keep view-specific subscriptions with the view instead of placing them in long-lived global state.

The callback arguments are `&mut Workspace`, the emitting `Entity<Chat>`, `&ChatEvent`, and `&mut Context<Workspace>`. The payload is borrowed for the callback; copy or clone data that must outlive it. The subscription is tied to one source Entity and one Event type, not every `Chat` or every event in the application. Multiple owners may subscribe independently to the same source. To stop one subscription early, remove or drop its handle from the owner's collection.

Use `cx.subscribe_in(&chat, window, callback)` when the handler also needs `&mut Window`. Its callback receives five arguments: owner, `&Entity<Chat>`, `&ChatEvent`, window, and context; keep its returned `Subscription` just as above. Use `cx.observe(&chat, ...)` for a generic entity-change notification when no typed Event payload is needed. An Event is deliberate semantic information; `cx.notify()` reports that an Entity needs an update and does not produce a `ChatEvent`.

If a subscription seems silent, check that the source is the same Entity, the `EventEmitter` implementation matches the emitted type, the handle is still stored, and `cx.emit` is reached after a successful state change. If the callback runs but the screen stays stale, check which Entity renders the changed value and call `cx.notify()` on its context.

### Try `observe` without an Event payload

Restore the original complete `Chat`/`Workspace` example above if you made the Action continuation, then run it: each click raises the visible count by one. Now make these three changes to that application:

1. Remove the `ChatEvent` enum and `impl EventEmitter<ChatEvent> for Chat {}`. Replace `Chat::send` with the following method. This version changes state and calls `notify`; it emits no Event.

   ```rust
   fn send(&mut self, cx: &mut Context<Self>) {
       self.sent += 1;
       cx.notify();
   }
   ```

2. In `Workspace::new`, replace the `cx.subscribe(...)` block with this observer. Keep the returned handle in the existing `_subscriptions` field.

   ```rust
   let subscription = cx.observe(&chat, |workspace, chat, cx| {
       workspace.shown_total = chat.read(cx).sent;
       cx.notify();
   });
   ```

3. Run the application again and click **Send**. The count still rises. The observer receives the changed `Entity<Chat>` and reads its state; it receives no `ChatEvent` or payload. Remove `cx.notify()` from `Chat::send` once and rerun: the observer no longer fires, so the count stays at zero even though `Chat.sent` changes. Restore `cx.notify()` afterward.

`observe` is useful when any notification from a particular Entity is enough and the observer can read current state. `subscribe` is useful when the producer deliberately reports a typed fact. Both return a `Subscription` that the owner must retain; `cx.emit` alone does not notify `observe` callbacks, and `cx.notify()` alone does not emit a typed Event.

:::info INFO — Event delivery does not follow Focus

An Event goes to subscribers of its source Entity. Moving Focus or changing a Key Context does not change who receives it. Do not use Events as a global command bus to bypass Action routing.

:::

## Action or Event?

| Question | Use | Examples |
| --- | --- | --- |
| Is this an instruction a user or caller wants performed? | **Action** | Save, Delete, Open Search |
| Should it be bindable to a key or shown in a menu? | **Action** | Copy, Toggle Sidebar, Rename |
| Is this a fact reported after state or lifecycle changed? | **Event** | ValueChanged, Saved, Dismissed |
| Should an owner observe a child independently of its UI tree? | **Event** | Input changed, row selected, dialog submitted |
| Is it only a pointer gesture with no other command entry point? | callback | hover, drag delta, pointer position |

Use both when a command produces a fact other parts of the application need to observe: handle the Action first, commit the state change, then emit the Event.

## Pointer and keyboard input are also events

`MouseDownEvent`, `MouseUpEvent`, `MouseMoveEvent`, `ScrollWheelEvent`, `KeyDownEvent`, and `KeyUpEvent` describe raw input. They are distinct from the typed `EventEmitter` notifications above. A normal `div()` can register `.on_mouse_down(MouseButton::Left, ...)` or `.on_key_down(...)`; its `InteractiveElement` implementation handles the underlying hitbox and dispatch registration. Use [Action](./action) for an operation that needs a shortcut or menu entry, and raw events when positions, buttons, modifiers, or gesture deltas matter. Raw input callbacks do not create a typed entity Event unless the owning entity calls `cx.emit(...)`.

For example, [GPUI Kit's TimeField](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/time_field.rs) binds arrow-key **Actions** inside its own Key Context, handles a typed `KeyDownEvent` for digit input, and emits `TimeFieldEvent::Change` only after the time value changes. Its owner can subscribe to that event without knowing whether the change came from a key or another control. A matching KeyBinding can consume a key before a raw `on_key_down` handler receives it, so commands belong in Actions rather than duplicate raw key handlers. A digit handler follows this shape:

```rust
fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
    let stroke = &event.keystroke;
    if stroke.modifiers.modified() || stroke.key.chars().count() != 1 {
        return;
    }
    let Some(digit) = stroke.key.chars().next().and_then(|c| c.to_digit(10)) else {
        return; // Leave unrelated keys to the rest of the UI.
    };
    window.prevent_default();
    cx.stop_propagation();
    if self.editor.input_digit(digit) {
        cx.emit(TimeFieldEvent::Change(self.editor.time));
    }
    cx.notify();
}
```

### Capture and bubble

Raw input has two dispatch phases. **Keyboard** listeners follow the focused element's path: capture walks from root to focused node; bubble returns from focused node to root. **Mouse** listeners are registered in paint order rather than on that ancestry path: capture runs back to front, and bubble runs front to back. The dispatcher calls matching mouse listeners in that order; a low-level listener must check its own hitbox before acting. Normal `.on_mouse_down(...)` and `.on_key_down(...)` callbacks run in bubble; `.capture_any_mouse_down(...)` is an element-level capture hook.

### Try a nested pointer handler

Return to the original Event version of the complete example above. Add `parent_hits: usize` and `child_hits: usize` to `Workspace`, initialize both to `0` in `Workspace::new`, and replace its `Render` implementation with this one:

```rust
impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_4()
            .child(format!("Messages sent: {}", self.shown_total))
            .child(Button::new("send").label("Send").on_click(cx.listener(
                |workspace, _, _, cx| {
                    workspace.chat.update(cx, |chat, cx| chat.send(cx));
                },
            )))
            .child(format!(
                "Parent: {} | Child: {}",
                self.parent_hits, self.child_hits
            ))
            .child(
                div()
                    .p_4()
                    .bg(rgb(0xd0d7de))
                    .on_mouse_down(MouseButton::Left, cx.listener(|workspace, _, _, cx| {
                        workspace.parent_hits += 1;
                        cx.notify();
                    }))
                    .child("Parent surface")
                    .child(
                        div()
                            .p_4()
                            .bg(rgb(0x8ecae6))
                            .on_mouse_down(MouseButton::Left, cx.listener(
                                |workspace, _, _, cx| {
                                    workspace.child_hits += 1;
                                    cx.notify();
                                    // Uncomment to stop the parent handler:
                                    // cx.stop_propagation();
                                },
                            ))
                            .child("Child surface"),
                    ),
            )
    }
}
```

Click the blue **Child surface** once: both counters increase. The child listener is registered later in paint order and runs first in mouse bubble; the parent hitbox also contains that point, so its listener runs next. Click the gray area outside the child: only the parent counter increases. Uncomment `cx.stop_propagation()` in the child callback and rerun. Clicking the child now increases only its counter; clicking the gray area still increases only the parent counter. A click on **Send** changes the message count without changing either hit counter.

The two surfaces are nested for this exercise, but GPUI mouse dispatch uses the window frame's ordered listener list and each listener's hitbox. It does **not** route mouse input along a DOM-style ancestor chain. An overlapping surface painted later can be the first bubble listener even when it is not a child in the element tree. Stop propagation only when the child interaction must keep later listeners from acting on the same input.

Custom [`Element`](./element#the-three-phases) code can register a listener during `paint` with `window.on_mouse_event` and inspect `DispatchPhase`:

```rust
window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
    if phase.capture() && hitbox.is_hovered(window) {
        // Decide whether this surface owns the gesture.
        if event.button == MouseButton::Left {
            cx.stop_propagation();
        }
    }
});
```

This listener is registered during `paint` and is replaced when the next frame is rendered. A `Hitbox` should have been inserted during `prepaint`. GPUI Kit's [Carousel scroll mask](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/carousel/scroll_mask.rs) uses capture for pointer and wheel gestures: it consumes movement on the carousel's axis while letting movement on the other axis reach an outer scroller. `Hitbox::is_hovered` tests pointer location; `should_handle_scroll` also accounts for scroll occlusion. Prefer fluent element handlers for ordinary controls; use `window.on_mouse_event` when building a custom Element that needs its own hitbox or phase handling.

### `stop_propagation` versus `prevent_default`

| Call | Meaning | Typical use |
| --- | --- | --- |
| `cx.stop_propagation()` | Stop delivery to later listeners in the **current dispatch**. In mouse bubble this blocks surfaces behind the current one; in keyboard bubble it blocks ancestors. In capture it also prevents the remaining capture listeners and bubble phase. | A nested control consumed a drag or key. |
| `window.prevent_default()` | Mark the current input event's default behavior as prevented. GPUI uses this for built-in behavior such as parent focus acquisition on mouse down. | A child handles mouse down but should keep the parent's focus from moving. |

They are independent. Stopping propagation does not itself cancel the focus default; preventing the default does not itself stop another handler. GPUI resets both flags for each input dispatch. `prevent_default` controls GPUI behavior that checks this flag; do not treat it as a general browser-style or operating-system event cancellation. Call either only after deciding that this input belongs to the control. An Action has the reverse initial propagation policy from raw input: its handler stops bubbling by default, and `cx.propagate()` explicitly lets an ancestor try it. Input callbacks receive `&mut Window`; call `window.prevent_default()` there.

When debugging an input handler, check the event phase, focused path, content mask, hitbox behavior, and z order. A handler may be registered correctly but never see a point because a higher surface occludes it or because the event follows a different focus path.
