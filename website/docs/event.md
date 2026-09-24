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

## Define and emit an Event

Define the facts an Entity can report and implement [`EventEmitter`](https://docs.rs/gpui-pre/0.3.6/gpui/trait.EventEmitter.html):

```rust
#[derive(Clone, Debug)]
enum ChatEvent {
    DraftChanged,
    MessageSent { message_id: MessageId },
}

impl EventEmitter<ChatEvent> for Chat {}
```

Emit the Event after the state change succeeds:

```rust
fn finish_send(&mut self, message_id: MessageId, cx: &mut Context<Self>) {
    self.draft.clear();
    cx.emit(ChatEvent::MessageSent { message_id });
    cx.notify(); // Re-render Chat if its visible draft changed.
}
```

`cx.emit(...)` queues delivery as a GPUI effect; subscribers run after the current entity update can finish, not as a direct call inside `finish_send`. Emit only after the operation succeeds. Call `cx.notify()` separately when the emitter's rendered UI changed: emitting a notification is not a replacement for requesting a render. Name Events as facts in the past tense: `MessageSent`, `Saved`, or `Dismissed`. A command-style name such as `SendMessage` belongs to an Action.

## Subscribe from the owner

The owner stores subscriptions on the same View that subscribes. This follows the pattern used by GPUI Kit examples:

```rust
struct Workspace {
    chat: Entity<Chat>,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    fn new(cx: &mut Context<Self>) -> Self {
        let chat = cx.new(Chat::new);
        let _subscriptions = vec![cx.subscribe(&chat, |workspace, _chat, event, cx| {
            if matches!(event, ChatEvent::MessageSent { .. }) {
                workspace.refresh_conversation();
                cx.notify();
            }
        })];

        Self { chat, _subscriptions }
    }
}
```

Do not leave the returned `Subscription` in a local variable: it is dropped when the function returns, which disconnects the observer. Keeping `_subscriptions` on `Workspace` gives both the same lifetime. When the View is dropped, its subscriptions are dropped and disconnected too. Avoid storing View-scoped subscriptions in a longer-lived global owner: keeping callbacks and captured resources alive after the View is gone can cause a memory leak.

The callback receives the owner (`&mut Workspace`), the emitting `Entity<Chat>`, a borrowed `&ChatEvent`, and the owner's [Context](./context) (`Context<Workspace>`). It receives notifications from that **specific entity** and event type; another `Chat` instance does not share its subscribers. Use `cx.subscribe_in(&chat, window, |workspace, chat, event, window, cx| { ... })` when the callback needs a mutable [Window](./window); keep its returned `Subscription` in the same field. Use `cx.observe(...)` instead when the owner only needs to know that an entity changed and does not need a typed payload.

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
