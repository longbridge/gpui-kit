---
title: Action
description: Define typed commands and route them through focus, key contexts, and GPUI's dispatch path.
order: -2.62
---

# Action

An **Action** represents an operation the application can perform. A shortcut, menu item, command palette, button, or another Action handler can all dispatch the same typed value. GPUI routes it to the part of the [Element](./element) tree that owns the command. An [Event](./event) serves the other direction: it reports something that happened after state changed.

The [GPUI Action source](https://github.com/zed-industries/zed/blob/main/crates/gpui/src/action.rs) defines the macro, trait, and registry described here.

This page explains command definition and dispatch. See [KeyBinding](./keybinding) for key notation, context matching, and keymap setup.

## One command, several entry points

Define a unit Action with a namespace. `actions!` generates the type and registers its stable name, here `chat::SendMessage`:

```rust
use gpui_kit::*;

actions!(chat, [SendMessage]);
```

An element handler receives the typed Action, the window, and the owning entity's [Context](./context). `cx.listener` adapts the method to the element callback:

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

// In Chat::render:
div()
    .track_focus(&self.focus_handle)
    .key_context("Chat")
    .on_action(cx.listener(Self::on_action_send_message))
    .child("Chat")
```

Bind a shortcut to `SendMessage`, and use the same Action elsewhere:

```rust
window.dispatch_action(Box::new(SendMessage), cx); // Button or command palette
MenuItem::action("Send Message", SendMessage)      // Native application menu
```

The command logic stays in one handler. A direct `window.dispatch_action(...)` does **not** need a KeyBinding or match a Key Context; those participate when a keystroke is translated into an Action. Use [KeyBinding](./keybinding) for the binding itself.

## Define data carrying Actions

An Action can include data. Deriving `Action` requires `Clone` and `PartialEq`. If the Action should be constructible from a named JSON keymap entry, also derive `Deserialize` and `JsonSchema`:

```rust
#[derive(Action, Clone, PartialEq, serde::Deserialize, schemars::JsonSchema)]
#[action(namespace = chat)]
struct InsertPrompt {
    text: String,
}
```

The Action registry uses the namespace and type name to build a typed value from an action name and optional JSON payload. This lets a configurable keymap and a command UI refer to the same command. Names must be unique; duplicate registration panics during application creation.

The JSON-capable example requires `serde` with its `derive` feature and `schemars` as application dependencies for those two derives.

For a runtime command whose payload should never come from JSON, `no_json` retains typed dispatch but opts out of JSON construction:

```rust
#[derive(Action, Clone, PartialEq)]
#[action(namespace = workspace, no_json)]
struct OpenConversation {
    conversation_id: ConversationId,
}
```

Choose a stable verb based name for each command. Use one Action type for all entry points that mean the same operation; put state changes in its owner rather than in each input callback.

## Focus selects the route

<svg class="focus-action-diagram" viewBox="0 0 1120 250" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="focus-action-title-en focus-action-desc-en">
  <title id="focus-action-title-en">GPUI shortcut dispatch in three steps</title>
  <desc id="focus-action-desc-en">Focus builds a dispatch path, Key Context selects a binding, and its Action reaches a handler on that path.</desc>
  <defs><marker id="focus-action-arrow-en" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto"><path d="M0 0 10 5 0 10z" class="fa-arrow-head" /></marker></defs>
  <rect x="24" y="24" width="310" height="202" rx="14" class="fa-box" />
  <text x="50" y="58" class="fa-step">1 · FOCUS</text><text x="50" y="88" class="fa-title">Build the Dispatch Path</text>
  <rect x="50" y="111" width="258" height="58" rx="9" class="fa-active" /><text x="70" y="136" class="fa-code">Chat → Workspace</text><text x="70" y="157" class="fa-body">focused Element → ancestors</text>
  <text x="50" y="201" class="fa-body">Start at the Element with Focus.</text>
  <path d="M348 125H393" class="fa-arrow" marker-end="url(#focus-action-arrow-en)" />
  <rect x="407" y="24" width="310" height="202" rx="14" class="fa-box" />
  <text x="433" y="58" class="fa-step">2 · KEY CONTEXT</text><text x="433" y="88" class="fa-title">Match a KeyBinding</text>
  <rect x="433" y="111" width="258" height="58" rx="9" class="fa-active" /><text x="453" y="136" class="fa-code">⌘ Enter + "Chat"</text><text x="453" y="157" class="fa-code">→ SendMessage</text>
  <text x="433" y="201" class="fa-body">Use key_context values on the path.</text>
  <path d="M731 125H776" class="fa-arrow" marker-end="url(#focus-action-arrow-en)" />
  <rect x="790" y="24" width="306" height="202" rx="14" class="fa-box" />
  <text x="816" y="58" class="fa-step">3 · ACTION</text><text x="816" y="88" class="fa-title">Dispatch along the path</text>
  <rect x="816" y="111" width="254" height="58" rx="9" class="fa-action" /><text x="836" y="136" class="fa-code">Chat handler</text><text x="836" y="157" class="fa-body">then parents if propagated</text>
  <text x="816" y="201" class="fa-body">The most specific handler runs first.</text>
</svg>

A [FocusHandle](./window) identifies a keyboard target. Keep the handle on the entity that owns the interaction, then attach it to an element each time that entity renders:

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

// In Chat::render:
div()
    .track_focus(&self.focus_handle)
    .key_context("Chat")
    .on_action(cx.listener(Self::on_action_send_message))
```

`track_focus` registers the handle on the element's dispatch node. Mouse down inside the element focuses that handle by default. If an inner control must retain its own focus, its mouse down handler can call `window.prevent_default()` to suppress the ancestor's default focus transfer. Tracking does not focus the element during render; call `self.focus_handle.focus(window, cx)` when the view opens or the user enters it.

`handle.is_focused(window)` tests the exact target. `handle.contains_focused(window, cx)` also accepts a focused descendant, useful while a child control is active. A tracked handle is not automatically a Tab stop: configure `cx.focus_handle().tab_stop(true)` when creating it. For a stateless component, retain a handle across renders with `window.use_keyed_state(...)`.

GPUI builds a **dispatch path** from the focused element through its ancestors. A `key_context("Chat")` on that path makes contextual bindings eligible; the matching key produces an Action. The Action then travels on the path. A handler on a sibling is not reachable from this route.

## Handler order and propagation

Action dispatch has two phases:

1. **Capture:** matching `.capture_action(...)` listeners from the root toward the target.
2. **Bubble:** matching `.on_action(...)` listeners from the target toward the root, then global `cx.on_action(...)` listeners if propagation continues.

The closest bubble handler therefore gets the first chance to handle a command. An Action handler stops bubble propagation by default. Call `cx.propagate()` when this handler declines the Action and a parent or global handler should try it:

```rust
fn on_action_close(
    &mut self,
    _: &ClosePanel,
    _: &mut Window,
    cx: &mut Context<Self>,
) {
    if !self.can_close() {
        cx.propagate();
        return;
    }
    self.close();
    cx.notify();
}
```

Capture listeners can call `cx.stop_propagation()` to stop dispatch before it reaches the target. Global bubble handlers also stop propagation by default, so a global fallback that declines a command should call `cx.propagate()`. These are Action dispatch controls; `window.prevent_default()` controls a default input behavior such as mouse focus transfer. See [Event](./event) for pointer and keyboard event propagation.

`window.dispatch_action(Box::new(action), cx)` captures the current focus target and defers dispatch to the rendered frame. For an explicit owner, `focus_handle.dispatch_action(&action, window, cx)` starts at the element that rendered that handle, if it is present in the current frame. `cx.dispatch_action(&action)` targets the active window, or global handlers when no window is active. These choices matter when a popup or click changes focus before a command runs.

## Coordinate sibling regions through an owner

Suppose a conversation selected in a Sidebar should open in Chat. The Sidebar describes the intent with `OpenConversation`; the common owner, `Workspace`, handles it and updates the Chat entity:

```rust
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

// Workspace renders both regions under its handler.
h_flex()
    .on_action(cx.listener(Self::on_action_open_conversation))
    .child(self.sidebar.clone())
    .child(self.chat.clone())

// From a Sidebar interaction, while its focus path is active:
window.dispatch_action(Box::new(OpenConversation { conversation_id }), cx);
```

The route is **Sidebar → Workspace**. `Workspace` then updates Chat through the [Entity](./entity) API. Attaching the handler only to Chat would not work for an Action dispatched from Sidebar because Chat is a sibling, outside Sidebar's dispatch path. For commands that must target a specific rendered region regardless of current focus, retain that region's `FocusHandle` and use its `dispatch_action` method. See [Coding Guides](./coding-guides) for larger feature ownership patterns.

GPUI Kit uses the same pattern in its Command palette and Popup Menu: a selected item supplies a boxed Action, then the window dispatches it. The Command palette also keeps its own focus handle, key context, and navigation Action handlers on the palette element. The framework component owns selection and keyboard mechanics; the application owner handles the command's meaning.

## Diagnose a missing command

When a shortcut works only after clicking a region, inspect the route in order:

1. Which `FocusHandle` is focused, and is it attached with `track_focus` in the rendered tree?
2. Is the required `key_context` on that element or an ancestor? See [KeyBinding](./keybinding) for binding matching.
3. Is the typed `.on_action(...)` handler on the resulting dispatch path?
4. Did a closer handler consume the Action, or did a declining handler forget `cx.propagate()`?
5. If a direct dispatch runs after focus changes, should it use an explicit `FocusHandle` target?

Keep the handle, context, and handler with the region that owns the command. Use a global handler only for an operation that truly applies across the application.
