---
title: Action
description: Define typed commands and route them through focus, key contexts, and GPUI's dispatch path.
order: -2.62
---

# Action

An **Action** represents an operation the application can perform. A shortcut, menu item, command palette, button, or another Action handler can all dispatch the same typed value. GPUI routes it to the part of the [Element](./element) tree that owns the command. An [Event](./event) serves the other direction: it reports something that happened after state changed.

The [GPUI Action source](https://docs.rs/crate/gpui-pre/{{gpui_pre_version}}/source/src/action.rs) defines the macro, trait, and registry described here.

This page explains command definition and dispatch. Start with [Focus](./focus) if you have not yet created a keyboard target; see [KeyBinding](./keybinding) for key notation, context matching, and keymap setup.

## Run an Action already in this repository

From the repository root, launch the Story Gallery directly on its Tree page:

```sh
cargo run -p gpui-component-story -- Tree
```

Select a file-tree row and press **Enter**. The process prints `Renaming item: ...` in the terminal. Select another row and repeat to see that the handler reads the current selection. If Enter does nothing, click a row first: the binding is scoped to the Tree story's focused path. This example does not rename a file.

Read the implementation in [`crates/story/src/stories/tree_story.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/story/src/stories/tree_story.rs):

1. `actions!(story, [Rename, OpenFile, Delete])` defines the typed commands.
2. `init` binds `enter` to `Rename` under the `TreeStory` context. [`stories::init`](https://github.com/longbridge/gpui-kit/blob/main/crates/story/src/stories/mod.rs) calls it during application setup.
3. `TreeStory::render` attaches `.key_context(CONTEXT)` and `.on_action(cx.listener(Self::on_action_rename))` to the story container. Its child Tree supplies the active focus path.
4. `on_action_rename` reads the selected entry from `tree_state`. Selection belongs to the Tree state; the Action expresses what the user requested.

The rest of this guide builds from that route. A menu or button can later dispatch the same Action without copying the handler.

## Build a runnable Action in `hello_world`

The Tree story is useful for tracing a real application. To build the route yourself, temporarily replace `examples/hello_world/src/main.rs` with the complete program below. It uses the existing `hello_world` package and needs no new crate or dependency.

```rust
use gpui_kit::component::button::Button;
use gpui_kit::*;

actions!(counter, [Increment]);

struct Counter {
    count: usize,
    focus: FocusHandle,
}

impl Counter {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle().tab_stop(true);
        focus.focus(window, cx);
        Self { count: 0, focus }
    }

    fn on_increment(&mut self, _: &Increment, _: &mut Window, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }
}

impl Render for Counter {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .track_focus(&self.focus)
            .key_context("Counter")
            .on_action(cx.listener(Self::on_increment))
            .child(format!("Count: {}", self.count))
            .child("Press Enter while this region has focus")
            .child(
                Button::new("increment")
                    .label("Increment")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.focus.dispatch_action(&Increment, window, cx);
                    })),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        cx.bind_keys([KeyBinding::new("enter", Increment, Some("Counter"))]);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| Counter::new(window, cx))
        })
        .expect("failed to open window");
    });
}
```

Run it from the repository root:

```sh
cargo run -p hello_world --bin hello_world
```

The window starts at `Count: 0`. Press **Enter** while the counter region has Focus, or click **Increment**; each operation increases the displayed count by one. The counter's `FocusHandle` is retained in its Entity and attached to the rendered container. `cx.bind_keys` maps Enter to `Increment` only on a path containing `Counter`. The container's `.on_action` calls `on_increment`, which changes retained state and calls `cx.notify()` so the new count renders. The button sends the *same* Action to that container through its saved handle, even if clicking the button moves Focus.

Check each link in the route with two small experiments, then restore the original source. First, remove `.key_context("Counter")`: Enter no longer selects this binding, while the button still works because direct dispatch does not consult Key Context. Next, restore the context and remove `.on_action(...)`: neither input changes the count because the route has no handler. If a handler runs but the label stays stale, check whether it mutates retained state and calls `cx.notify()`. Restore `examples/hello_world/src/main.rs` when finished.

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

Use a unit Action for a command whose handler can read all required state from its owner, as `Rename` does in the Tree story. Use a data Action when the caller must identify a target or supply a value. An Action payload is the command input, not a place to store the owner's changing UI state.

For a runtime command whose payload should never come from JSON, `no_json` retains typed dispatch but opts out of JSON construction:

```rust
#[derive(Action, Clone, PartialEq)]
#[action(namespace = workspace, no_json)]
struct OpenConversation {
    conversation_id: String,
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

### Trace one shortcut

Suppose the focused element is inside `Chat`, itself inside `Workspace`. A binding such as `KeyBinding::new("secondary-enter", SendMessage, Some("Chat"))` is eligible only while `Chat` appears on that focused path. GPUI chooses a matching binding, then dispatches its `SendMessage` value along that path. The `Chat` handler runs before a `Workspace` bubble handler. If no element on the route handles the Action, a global `cx.on_action` handler can receive it. The context chooses a **binding**; it does not select a handler by itself. [KeyBinding](./keybinding) explains competing bindings and predicate syntax.

For a command triggered by a click, `window.dispatch_action(Box::new(SendMessage), cx)` uses the focus captured when called. Ensure the intended route has focus, or dispatch through a retained `FocusHandle`. A button's callback can also call the owning entity's method directly when no shared command route is needed.

## Handler order and propagation

Action dispatch has two phases:

1. **Capture:** global capture listeners, then matching `.capture_action(...)` listeners from the root toward the target.
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

To inspect a command without consuming it, a capture listener can observe it and leave propagation enabled. Capture starts with propagation enabled. In bubble, each listener starts with propagation stopped. Call `cx.propagate()` when the next ancestor or global fallback should also receive that Action. Returning early alone does not pass it on.

`window.dispatch_action(Box::new(action), cx)` captures the current focus target and defers dispatch to the rendered frame. For an explicit owner, `focus_handle.dispatch_action(&action, window, cx)` starts at the element that rendered that handle, if it is present in the current frame. `cx.dispatch_action(&action)` targets the active window, or global handlers when no window is active. These choices matter when a popup or click changes focus before a command runs.

| Call | Target | When to use it |
| --- | --- | --- |
| `window.dispatch_action(Box::new(action), cx)` | Current focus in this window, captured at call time | A menu or button command for the focused region. Dispatch is deferred. |
| `focus_handle.dispatch_action(&action, window, cx)` | The element that rendered this handle in the current frame | A command for a specific rendered region, even if another control now has focus. No rendered handle means no dispatch. |
| `cx.dispatch_action(&action)` | Active window, or global handlers without one | An application-level command when the caller has an `App` context. |

None of these calls evaluates a Key Context predicate. Key Context participates when a **keystroke** selects an Action from the keymap. The dispatched Action still needs a handler reachable from its chosen target.

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

If a key does nothing, first distinguish **no binding match** from **no reachable handler**. Try dispatching the Action directly on the intended `FocusHandle`. If that reaches the handler, inspect the key string and context predicate. If it does not, inspect the rendered handle, dispatch path, and propagation. If the handler runs but the screen stays unchanged, verify that it updates the owning state and calls `cx.notify()` when a redraw is needed.

Keep the handle, context, and handler with the region that owns the command. Use a global handler only for an operation that truly applies across the application.

## Practice with the Tree story

1. **Follow the path.** Run `cargo run -p gpui-component-story -- Tree`, select a row, and press Enter. In `tree_story.rs`, find the binding, context, and handler. Which component owns the selected row, and which entity owns the command handler?
2. **Change the input.** In your own branch, temporarily change the Tree story binding from `"enter"` to `"secondary-r"`. Re-run the Story Gallery. Verify that the new key invokes the same `Rename` handler and that Enter no longer does. Restore the source after the experiment.
3. **Predict propagation.** Place `Rename` handlers on a child and its parent in a small view. Have the child call `cx.propagate()` only when it has no selection. Predict which handler runs in both cases, then test with visible output. A handler that returns without calling `cx.propagate()` consumes the Action.
4. **Add a target.** Model an operation that must carry a row identifier as a data Action with `#[action(namespace = story, no_json)]`. Dispatch it from a row callback; keep the mutation in the owning handler. Compare this with `Rename`, which reads the currently selected row instead.
