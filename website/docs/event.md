---
title: Event
description: Use GPUI Events for typed notifications and connect them to Actions.
order: -6
---

# Event

GPUI provides **Event** as a typed notification mechanism between Entities. An Event reports something that already happened; it does not use Focus, Key Contexts, KeyBindings, or the Dispatch Path.

## Action in, Event out

Action and Event often form one complete interaction:

```text
⌘ Enter → SendMessage Action → Chat sends → MessageSent Event → Workspace updates
```

- **Action** carries intent inward: “send this message.”
- **Event** reports the result outward: “this message was sent.”

The command owner handles the Action and changes its state. It then emits an Event so owners or services can react without being coupled to the command's UI entry point. See [Action](./action) for Focus, KeyBindings, and Action dispatch.

## Define and emit an Event

Define the facts an Entity can report and implement `EventEmitter`:

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
    cx.notify();
}
```

Name Events as facts in the past tense: `MessageSent`, `Saved`, or `Dismissed`. A command-style name such as `SendMessage` belongs to an Action.

## Subscribe from the owner

The owner subscribes while wiring its Entities together:

```rust
let chat = cx.new(Chat::new);
let subscription = cx.subscribe(&chat, |workspace, _, event, cx| {
    if matches!(event, ChatEvent::MessageSent { .. }) {
        workspace.refresh_conversation();
        cx.notify();
    }
});
```

Keep the returned `Subscription` alive when required, commonly in `_subscriptions: Vec<Subscription>`. Dropping it disconnects the observer. Use `window.subscribe(...)` when the callback also needs `&mut Window`.

:::info INFO — Event delivery does not follow Focus

An Event goes to subscribers of its source Entity. Moving Focus or changing a Key Context does not change who receives it. Do not use Events as a global command bus to bypass Action routing.

:::

## `emit` and `notify` are different

`cx.emit(...)` sends a typed semantic fact with a payload. `cx.notify()` tells observers to reread Entity state, usually causing a rerender. A state change may need one or both; emitting an Event does not automatically request a render.

## Action or Event?

| Question | Use | Examples |
| --- | --- | --- |
| Is this an instruction a user or caller wants performed? | **Action** | Save, Delete, Open Search |
| Should it be bindable to a key or shown in a menu? | **Action** | Copy, Toggle Sidebar, Rename |
| Is this a fact reported after state or lifecycle changed? | **Event** | ValueChanged, Saved, Dismissed |
| Should an owner observe a child independently of its UI tree? | **Event** | Input changed, row selected, dialog submitted |
| Is it only a pointer gesture with no other command entry point? | callback | hover, drag delta, pointer position |

Use both when a command produces a fact other parts of the application need to observe: handle the Action first, commit the state change, then emit the Event.
