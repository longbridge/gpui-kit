---
title: Event
description: Use GPUI Events for typed notifications and connect them to Actions.
order: -6
---

# Event

GPUI provides **Event** as a typed notification mechanism between Entities. An Event reports something that already happened; unlike an [**Action**](./action), it does not use Focus, Key Contexts, KeyBindings, or the Dispatch Path.

## Action in, Event out

An Action can cause the state change, but Event delivery starts after that change:

```text
Chat changes state → emit(MessageSent) → subscribers receive Event → Workspace updates
```

<img class="architecture-light" src="/event-subscriptions-flow.svg?v=20260922-1" alt="Chat emits one MessageSent Event to independent Workspace, Activity Log, and Telemetry subscribers">
<img class="architecture-dark" src="/event-subscriptions-flow-dark.svg?v=20260922-1" alt="Chat emits one MessageSent Event to independent Workspace, Activity Log, and Telemetry subscribers">

- [**Action**](./action) carries intent inward: “send this message.”
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
}
```

Name Events as facts in the past tense: `MessageSent`, `Saved`, or `Dismissed`. A command-style name such as `SendMessage` belongs to an Action.

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
        let _subscriptions = vec![cx.subscribe(&chat, |workspace, _, event, _cx| {
            if matches!(event, ChatEvent::MessageSent { .. }) {
                workspace.refresh_conversation();
            }
        })];

        Self { chat, _subscriptions }
    }
}
```

Do not leave the returned `Subscription` in a local variable: it is dropped when the function returns, which disconnects the observer. Keeping `_subscriptions` on `Workspace` gives both the same lifetime. When the View is dropped, its subscriptions are dropped and disconnected too. Avoid storing View-scoped subscriptions in a longer-lived global owner: keeping callbacks and captured resources alive after the View is gone can cause a memory leak.

Use `cx.subscribe_in(..., window, ...)` when the callback needs `&mut Window`; keep that returned `Subscription` in the same field as well.

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
