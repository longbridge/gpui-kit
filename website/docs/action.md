---
title: Action
description: Understand how GPUI routes Focus, keyboard shortcuts, Actions, and Events.
order: -5
---

# Action

GPUI provides **Focus**, **Key Context**, **Action**, **KeyBinding**, and [**Event**](./event) as its core interaction mechanisms. Together they let an application route commands to the active part of a window and communicate typed state changes between entities.

This guide shows how to use those mechanisms together:

- **Focus** says where keyboard interaction is happening;
- **`track_focus`** registers a stable `FocusHandle` on an Element so pointer input and command routing can use it;
- an **Action** expresses a command and can come from a `KeyBinding`, menu, button, or code;
- an [**Event**](./event) reports what an entity did or experienced to its subscribers.

## How a shortcut works

<img class="architecture-light" src="/focus-action-flow.svg?v=20260922-3" alt="Focus builds a Dispatch Path, its key contexts match a KeyBinding, and the resulting Action is dispatched to the most specific handler first">
<img class="architecture-dark" src="/focus-action-flow-dark.svg?v=20260922-3" alt="Focus builds a Dispatch Path, its key contexts match a KeyBinding, and the resulting Action is dispatched to the most specific handler first">

Imagine a window split into a Sidebar on the left and Chat on the right. Clicking the Sidebar produces a focus path containing `Sidebar`; clicking the chat composer produces one containing `Chat`. A binding scoped to `Chat` is therefore active only on the right.

The layout can make both keyboard regions explicit in one place:

```rust
h_flex()
    .size_full()
    .child(
        // Left: clicking here activates the Sidebar context.
        div()
            .w_64()
            .track_focus(&self.sidebar_focus)
            .key_context("Sidebar")
            .child("Sidebar"),
    )
    .child(
        // Right: clicking here activates the Chat context.
        div()
            .flex_1()
            .track_focus(&self.chat_focus)
            .key_context("Chat")
            .on_action(cx.listener(Self::on_action_send_message))
            .child("Chat"),
    )
```

Each region has its own stable `FocusHandle` and Key Context. Clicking Chat moves Focus to `chat_focus`, so `Chat` joins the active Dispatch Path and the `SendMessage` handler can receive the matched Action. Clicking Sidebar activates `Sidebar` instead, so the Chat-only binding does not match.

When a key is pressed, GPUI:

1. starts at the focused element and builds a path through its ancestors;
2. collects the `key_context` values on that path and matches a `KeyBinding`;
3. dispatches the matched Action along the same path, starting with the most specific handler.

The active focus path makes the same keystroke mean different things in different parts of a window without introducing a global shortcut switchboard.

## Focus is a location

A `FocusHandle` is a stable identity for a keyboard target. Keep it on the entity that owns the interaction:

```rust
struct Chat {
    focus_handle: FocusHandle,
}

impl Chat {
    fn new(cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle() }
    }
}

impl Focusable for Chat {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
```

- `handle.is_focused(window)` checks this exact target.
- `handle.contains_focused(window, cx)` also accepts a focused descendant.
- `handle.focus(window, cx)` deliberately moves focus here.

Use the exact check for an input caret or selected control. Use containment when a panel remains active while one of its controls has focus.

## What `track_focus` does

`track_focus` registers the `FocusHandle` on the Element's dispatch node and marks that Element as able to receive Focus:

```rust
div()
    .track_focus(&self.focus_handle)
    .key_context("Chat")
    .on_action(cx.listener(Self::on_action_send_message))
```

That registration has several connected effects:

- mouse down inside the Element moves Focus to the handle by default;
- `focus`, `in_focus`, and `focus_visible` styles can read its state;
- GPUI can calculate Focus containment and the Focus Path;
- Key Contexts and Action handlers on that path participate in key matching and Action dispatch.

A nested control can call `cx.prevent_default()` when it must keep the parent Element from taking Focus on mouse down.

`track_focus` does **not** immediately give the Element Focus during render. Call `focus_handle.focus(window, cx)` when opening a view or entering an interaction. Never request Focus unconditionally from `render`, because every render would steal it back.

### Focus and Tab order are separate

A tracked handle is not automatically reachable with Tab. Declare Tab behavior on the handle itself:

```rust
let focus_handle = cx.focus_handle().tab_stop(true);
```

Use `tab_index(...)` for an intentional order. Calling `.tab_stop(...)` on the element does not change a handle passed to `track_focus`.

For a stateless component, retain the handle across renders with keyed state:

```rust
let focus_handle = window.use_keyed_state(id, cx, |_, cx| {
    cx.focus_handle().tab_stop(true)
});
```

## An Action is a command protocol

An Action is GPUI's core representation of an application operation: a typed command value that can be dispatched without coupling the sender to the receiver. GPUI routes it through the active focus path. The same Action serves three layers:

1. **Input mapping:** a key binding maps a keystroke to an Action.
2. **Command dispatch:** a command palette, button, popup menu, or another handler dispatches the Action.
3. **Configuration:** a Keymap serializes a command as a stable Action name plus an optional JSON payload, and GPUI's Action registry deserializes it back into the typed Action. This is the basis of Zed-style user keymaps.

For example, a command palette stores Actions rather than a callback for every row:

```rust
let commands: Vec<(&str, Box<dyn Action>)> = vec![
    ("Send message", Box::new(SendMessage)),
    ("Toggle sidebar", Box::new(ToggleSidebar)),
];

// When the user confirms the selected command:
window.dispatch_action(commands[selected].1.boxed_clone(), cx);
```

In application code, use whatever owned or cloneable command entry your palette model provides; the important boundary is that selection produces an Action and dispatches it. The focused owner still decides how to handle it.

Native application menus use the same protocol. On macOS, menu commands are Actions rather than ordinary element click callbacks:

```rust
MenuItem::action("Send Message", SendMessage)
```

Unit Actions declared with `actions!` are registered by name. For an Action carrying configuration data, derive `Action` and `Deserialize` and give it a namespace:

```rust
#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = chat)]
struct InsertPrompt {
    text: SharedString,
}
```

A keymap can then identify the command by its stable action name and, when needed, a JSON payload. `#[action(no_json)]` deliberately opts an Action out of JSON construction; use it for runtime-only commands that should never appear in user configuration.

### Coordinate sibling components through their owner

Suppose selecting a conversation in the Sidebar should open it in Chat. The Sidebar should describe that intent with `OpenConversation`; it does not need a callback or direct reference to Chat. Their nearest common owner, `Workspace`, handles the Action and updates Chat:

```rust
#[derive(Action, Clone, PartialEq)]
#[action(namespace = workspace, no_json)]
struct OpenConversation {
    conversation_id: ConversationId,
}

impl Workspace {
    fn on_action_open_conversation(
        &mut self,
        action: &OpenConversation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.chat.update(cx, |chat, cx| {
            chat.open(action.conversation_id.clone(), window, cx);
        });
    }
}

// Workspace is an ancestor of both Sidebar and Chat.
h_flex()
    .on_action(cx.listener(Self::on_action_open_conversation))
    .child(self.sidebar.clone())
    .child(self.chat.clone())

// A conversation row in Sidebar dispatches the command.
window.dispatch_action(
    Box::new(OpenConversation { conversation_id }),
    cx,
);
```

The dispatch route is now explicit: **Sidebar → Workspace → Chat**. The Action travels upward on Sidebar's current Dispatch Path until `Workspace` handles it. `Workspace` then calls Chat through the Entity API. The Action itself never travels sideways from Sidebar into Chat.

:::info INFO — A sibling is not on the Dispatch Path

If `on_action_open_conversation` is attached only to Chat, an Action dispatched while Sidebar has Focus cannot reach it: Chat is a sibling, not an ancestor on the current Dispatch Path. The same mistake can make a shortcut appear unresponsive when its `on_action` handler sits outside the path selected by Focus. Put a cross-region handler on the nearest common owner, register the `KeyBinding`, and place its `key_context` and handler on the path where the shortcut should work.

:::

Reserve global handlers for commands that are truly application-wide.

## Build a command end to end

Define and bind the command once:

```rust
actions!(chat, [SendMessage]);
const CHAT_CONTEXT: &str = "Chat";

fn init(cx: &mut App) {
    cx.bind_keys([
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-enter", SendMessage, Some(CHAT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-enter", SendMessage, Some(CHAT_CONTEXT)),
    ]);
}
```

Attach focus, context, and handler to the same owning region:

```rust
impl Chat {
    fn on_action_send_message(
        &mut self,
        _: &SendMessage,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.submit_draft();
        cx.notify();
    }
}

impl Render for Chat {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .key_context(CHAT_CONTEXT)
            .on_action(cx.listener(Self::on_action_send_message))
            .child("Chat")
    }
}
```

Every entry point dispatches the same `SendMessage` Action: `KeyBinding` covers the shortcut, the button calls `window.dispatch_action(...)` from its click handler, and popup or macOS native menus hold the same Action. Message submission is implemented once in the Action handler.

Action handlers stop bubbling by default. If an inner handler declines the Action and a parent should try it, call `cx.propagate()`. A global handler registered with `cx.on_action(...)` must also propagate whenever it does not apply.

Register key bindings before `cx.set_menus(...)`. Native menus capture displayed shortcuts when built, so changing bindings later does not update an existing menu automatically.

## How Action and Event work together

Action and Event describe opposite directions in the same interaction:

```text
⌘ Enter → SendMessage Action → Chat sends → MessageSent Event → Workspace updates
```

An Action carries **intent inward** to the command owner. After the operation changes state, an Event carries **what happened outward** to interested owners. Read [Event](./event) for `EventEmitter`, `emit`, subscriptions, lifetime management, and the complete Action-or-Event decision guide.

## Contextual and global shortcuts

Most editing and navigation shortcuts should be contextual: they only make sense while a region contains focus and should yield to a more specific child.

Use `cx.on_action(...)` for a true application-wide fallback or service command. For dangerous shortcuts, also check runtime state in the owner. Longbridge Pro scopes trading bindings to a workspace and separately rejects them while an input or dialog has focus. Context decides **where a command is eligible**; the handler decides **whether it is currently allowed**.

## Debug “works only after clicking”

If a shortcut only works after clicking a particular region, the click has usually moved focus into the path containing the required key context and handler. Treat this as a routing problem and inspect the same pipeline GPUI uses.

Check the pipeline in order:

1. **Binding:** Is the key bound to the expected Action and context?
2. **Focus:** Which `FocusHandle` is focused before and after the click?
3. **Tracking:** Is that same handle passed to `track_focus` on a rendered element?
4. **Context:** Is the required `key_context` on the focused element or an ancestor?
5. **Handler:** Is `on_action` on the same dispatch path?
6. **Propagation:** Did a more specific handler consume the Action?
7. **Lifetime:** Was a global handler or Event subscription dropped, or did a stale handler fail to propagate?

The usual fix is to make one region own the retained handle, `track_focus`, `key_context`, and `on_action`, then move focus into that region when the user enters it.

These patterns follow GPUI's dispatch implementation and are exercised in GPUI Kit's Menu, Tree, Input, Dialog, and Color Picker code, Zed's panels and editors, and Longbridge Pro's workspace and trading shortcuts.
