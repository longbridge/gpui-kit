---
title: KeyBinding
description: Bind GPUI Actions to keys, chords, and focused Key Contexts.
order: -2.625
---

# KeyBinding

A **KeyBinding** maps one or more keystrokes to a typed [Action](./action). GPUI Kit uses GPUI's keymap: register bindings on the application, then put a matching Key Context and Action handler on the focused [Element](./element)'s dispatch path. The Action guide explains that full Focus and dispatch path; this page concentrates on writing and resolving bindings.

## Bind a command

`KeyBinding::new(keys, action, context)` takes a keystroke string, an Action value, and an optional context predicate. Call `cx.bind_keys(...)` during initialization. A `None` context is eligible throughout the application; `Some("Editor")` requires an `Editor` context on the focused path.

```rust
use gpui_kit::*;

actions!(editor, [SaveDocument, MoveSelectionUp]);
const EDITOR_CONTEXT: &str = "Editor";

fn init_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("secondary-s", SaveDocument, Some(EDITOR_CONTEXT)),
        KeyBinding::new("up", MoveSelectionUp, Some(EDITOR_CONTEXT)),
    ]);
}
```

Call `gpui_kit::init(cx)` once before `init_keys(cx)` and before opening windows. GPUI Kit registers its component bindings during initialization; the application can then add its own bindings in a deliberate order.

Keep a [FocusHandle](./window) in the owning [Entity](./entity) and register it with the context and handler in its [Render](./render) implementation:

```rust
impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .key_context(EDITOR_CONTEXT)
            .on_action(cx.listener(Self::on_action_save_document))
            .on_action(cx.listener(Self::on_action_move_selection_up))
            .child("Editor")
    }
}
```

The handlers have the usual GPUI signature, for example `fn on_action_save_document(&mut self, _: &SaveDocument, window: &mut Window, cx: &mut Context<Self>)`. The `cx` argument is the owner's [Context](./context). A binding does not itself focus the element. Focus it when entering the region, or let pointer interaction focus its tracked handle. See [Action](./action) for focus ownership, dispatch, and propagation.

## Write key strings

A keystroke is a key name with optional modifiers separated by hyphens. Separate successive keystrokes with spaces to form a **chord**:

| Binding | Meaning |
| --- | --- |
| `"secondary-s"` | Primary shortcut modifier plus S: Command on macOS, Control on Linux and Windows. |
| `"ctrl-enter"`, `"alt-f4"`, `"shift-tab"` | Explicit modifiers. |
| `"cmd-shift-p"` | Platform modifier plus Shift and P. `cmd`, `super`, and `win` refer to the platform modifier; on Windows this is the Windows key, not Control. |
| `"up"`, `"escape"`, `"space"`, `"backspace"` | Named keys. |
| `"cmd-k left"` | Two keystrokes in sequence: the platform modifier plus K, then Left. |

GPUI also parses `fn`, `ctrl`, `alt`, `shift`, `cmd`, `super`, `win`, and `secondary` as modifiers. Use `secondary` for a conventional cross-platform Command/Control shortcut. Use `ctrl` when Control itself is required on every platform. A capital ASCII letter such as `"A"` implies Shift+A; writing `"shift-a"` makes that intent clearer. `KeyBinding::new` **panics** if its key string or context predicate cannot be parsed, so keep static definitions reviewable and validate user input before constructing bindings.

Chords can share a first keystroke. GPUI holds a prefix while a longer matching binding is possible; a following keystroke completes the chord or causes the prefix to be replayed. Avoid making a common text entry key a chord prefix in an editable region.

## Declare Key Contexts

The element's `.key_context(...)` declares facts about a node. The binding's third argument is a **predicate** tested against contexts along the current focus path. They use related but different syntax:

```rust
// Context attached to one element: identifier plus key/value fact.
div().key_context("Editor mode=normal")

// Predicates used as KeyBinding::new's third argument.
Some("Editor")
Some("Editor && mode == normal")
Some("Editor && !Modal")
Some("Workspace > Editor")
```

`Editor mode=normal` declares two facts on one node; `Editor && mode == normal` tests those facts. Predicates support identifiers, `==`, `!=`, `!`, `&&`, `||`, parentheses, and `>` for an ancestor followed by a descendant context. For example, `Workspace > Editor` requires an `Editor` context below a `Workspace` context. Use parentheses when combining operators so the intended grouping is explicit. `!Modal` excludes a path containing `Modal`; it is useful for a workspace shortcut that should not run while a modal owns focus.

Contexts only participate when they lie on the **focused** dispatch path. A sibling's `Editor` context does not activate an editor binding. A binding can match while its handler remains unreachable if that handler is on another branch. Put both context and handler on the focused region or an ancestor that owns the command.

## Understand precedence

When several bindings match the same keys, GPUI ranks them by the depth of the matching context on the focused path. A binding for an inner `Editor` ranks ahead of one for its `Workspace` ancestor. At the same depth, the binding added **later** ranks first. A binding with `None` context is treated as matching at the deepest context depth, so it is not automatically a weak fallback: a later `None` binding can rank ahead of an `Editor` binding on the same keys. Prefer an explicit workspace context for a shortcut that should yield to an inner control.

GPUI can try more than one matching binding. It dispatches each candidate Action along the focused path until a handler consumes one; Action handlers stop propagation by default. A handler that declines an Action can call `cx.propagate()` to let dispatch continue. Keep competing bindings intentional: order, context depth, and handler availability all affect the result.

```rust
cx.bind_keys([
    KeyBinding::new("escape", ClearWorkspaceSelection, Some("Workspace")),
    KeyBinding::new("escape", CloseEditorSearch, Some("Editor")),
]);
```

With focus in an Editor inside Workspace, `CloseEditorSearch` has the more specific match. With focus elsewhere in Workspace, only `ClearWorkspaceSelection` matches. This is how GPUI Kit components such as Tree and TimeField keep arrow-key behavior local to their own contexts.

## Look up a shortcut from its Action

An Action is also the key for **reverse lookup**: ask the window which binding currently invokes that Action. GPUI Kit's [Kbd component](../component/kbd) displays the result. Use the focus handle of the command's intended target when rendering a button, menu, or command palette. This matters when another control or an overlay currently owns Focus.

```rust
use gpui_kit::component::kbd::Kbd;

let binding = window.highest_precedence_binding_for_action_in(
    &SaveDocument,
    &self.editor_focus,
);

// A GPUI Kit hint for the same Action and target.
let hint = Kbd::binding_for_action_in(
    &SaveDocument,
    &self.editor_focus,
    window,
);
div().children(hint)
```

The first call returns `Option<KeyBinding>`; the second returns `Option<Kbd>`, ready to render beside a label. Both account for the target's context, shadowing by a higher priority binding, and the active keymap, including user overrides. `None` means there is no visible binding for that Action on that resolved path. These lookups use the **previously rendered frame**, so a focus handle first drawn in the current frame may have no result yet. A binding lookup does not check the command's runtime enabled state or prove that an Action handler is on the target path; the owner must still decide whether the command is allowed. `window.is_action_available_in(&SaveDocument, &self.editor_focus)` checks for an element Action handler on that path.

For the current window context, GPUI also offers `window.highest_precedence_binding_for_action(&action)` and `window.bindings_for_action(&action)`; the latter returns all visible bindings. For a known single context, use `window.highest_precedence_binding_for_action_in_context(&action, KeyContext::parse("Editor")?)`. GPUI Kit offers `Kbd::binding_for_action(&action, Some("Editor"), window)` for the same simple context lookup, or `None` for the window's current context. Its `Some(...)` argument uses **Key Context declaration syntax**, such as `"Editor mode=normal"`, not predicate syntax such as `"Editor && mode == normal"`. An invalid context string silently falls back to the window lookup, so validate dynamic input before passing it. A concrete focus handle is the safer choice when contexts are nested or a menu has moved Focus. GPUI Kit's `Kbd::global_binding_for_action(&action, window)` offers a last-resort lookup against an empty Key Context, without reconstructing a nested focus path.

`Kbd::binding_for_action_in` currently displays **only the first keystroke** of a chord. To show the entire chord, format every stroke of the returned `KeyBinding`:

```rust
use gpui_kit::AsKeystroke;
use gpui_kit::component::kbd::Kbd;

let shortcut = binding.map(|binding| {
    binding.keystrokes().iter()
        .map(|stroke| Kbd::format(stroke.as_keystroke()))
        .collect::<Vec<_>>()
        .join(" ")
});
```

`Kbd::format` chooses platform-specific modifier symbols and key names: `secondary-s` displays as `⌘S` on macOS and `Ctrl+S` on Linux and Windows. `AsKeystroke` exposes each binding stroke as a `Keystroke`. Keep `shortcut` optional: an unbound Action should not show a made-up accelerator.

## Keep menus and displayed shortcuts in sync

Use one Action for the shortcut, button, command palette, and menu item. A menu can hold the same value with `MenuItem::action("Save Document", SaveDocument)`, while a button can dispatch it with `window.dispatch_action(Box::new(SaveDocument), cx)`. The focused owner then runs the same handler.

Register bindings **before** `cx.set_menus(...)`. Native menus read the current keymap when built and retain the displayed shortcut. If an application changes bindings later, rebuild the menus with `cx.set_menus(...)` so their shortcut labels and native accelerators reflect the new keymap. For in-window menus and tooltips, query the Action's current binding as above rather than hard-coding `⌘A` or `Ctrl+A`: the platform, target context, and user keymap can each change the displayed shortcut. GPUI Kit's Popup Menu resolves shortcuts against the command target or trigger Focus and displays them with `Kbd`; a tooltip can use `Tooltip::new("Save Document").action(&SaveDocument, Some("Editor"))` to resolve a simple context when it renders.

## User keymaps and named Actions

GPUI's `actions!` macro registers unit Actions under stable names such as `editor::SaveDocument`. For an Action with configuration data, derive `Action` and the required deserialization and schema traits; `#[action(no_json)]` marks an Action that cannot be constructed from JSON. `cx.all_action_names()` lists registered names, and `cx.build_action(name, data)` builds a registered Action from an optional JSON value.

GPUI provides this Action registry and keymap machinery, but an application owns its **user keymap file format**, validation, loading, and reload policy. A loader can resolve an Action name and payload with `cx.build_action(...)`, parse the context with `KeyBindingContextPredicate::parse(...)`, and construct a binding with the fallible `KeyBinding::load(...)`. Report parse or unknown-Action errors to the user rather than passing untrusted strings to the panic-on-error `KeyBinding::new(...)`. `cx.bind_keys(...)` appends bindings; `cx.clear_key_bindings()` clears the entire application keymap, including component defaults, so a reload must restore every required binding in the desired order. Rebuild native menus after a keymap change.

## Troubleshoot a shortcut

1. **Keys:** Check the actual modifier on this platform. `secondary-s` means Command+S on macOS and Control+S elsewhere; `cmd-s` does not mean Control+S on Windows.
2. **Focus:** Check which `FocusHandle` is focused and that a rendered element calls `track_focus` with it. A shortcut that works only after a click often has a focus path problem.
3. **Context:** Check that the predicate matches a `key_context` on that path. Use `mode=normal` when declaring a context and `mode == normal` when testing it.
4. **Competition:** Check deeper contexts, later bindings at the same depth, and chord prefixes. A `None` binding can outrank a shallower scoped binding.
5. **Handler:** Check that the matching Action has an `on_action` handler on the focused path and that a more specific handler does not consume it first.
6. **Menus and reloads:** If a menu shows an old shortcut, call `cx.set_menus(...)` after installing the new keymap. If a reload removed a component shortcut, confirm that `clear_key_bindings()` was followed by all component initialization.

For the full route from Focus through Action handling, continue with [Action](./action).
