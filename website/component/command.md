---
title: Command
description: A command palette — a filtered list of commands and quick actions.
---

# Command

A command palette is a filtered list of commands with groups, Action-derived
keybinding hints, and keyboard navigation. Use it inline or compose it into an
existing dialog for a `⌘K`-style menu. On invalidation, Command creates and
layout-measures every flattened row; `v_virtual_list` then renders and paints
only viewport rows.

`Command` owns the entries and presentation policy. `CommandState` owns the
interaction state: query input, focus, selection, scrolling, and loading.

## Import

```rust
use gpui_kit::component::command::{Command, CommandEntry, CommandGroup, CommandItem, CommandState};
```

## Composition

Build the palette structure directly on `Command`; create an empty state once
and reuse it while the palette is shown.

```text
Command
├── CommandItem                 // ungrouped
├── CommandGroup
│   ├── CommandItem
│   └── CommandItem
├── separator
└── CommandGroup
    ├── CommandItem
    └── CommandItem

CommandState                    // query, focus, selection, scrolling
```

## Usage

### Inline

Define Actions and bindings in application setup. The default row resolves an
Action's active binding in the Command focus scope and then at application
scope, rendering a `Kbd` hint only when it finds one.

```rust
use gpui_kit::{actions, KeyBinding};

actions!(my_app, [OpenProfile, OpenBilling]);

// During application setup:
cx.bind_keys([
    KeyBinding::new("cmd-p", OpenProfile, Some("Command")),
    KeyBinding::new("cmd-b", OpenBilling, Some("Command")),
]);

let state = cx.new(|cx| CommandState::new(window, cx));

Command::new(&state)
    .group(
        CommandGroup::new().label("Suggestions")
            .item(CommandItem::new().label("Calendar").icon(IconName::Calendar))
            .item(CommandItem::new().label("Search Emoji").icon(IconName::Search))
            .item(CommandItem::new().label("Calculator").disabled(true)),
    )
    .separator()
    .group(
        CommandGroup::new().label("Settings")
            .item(
                CommandItem::new().label("Profile")
                    .icon(IconName::User)
                    .action(Box::new(OpenProfile)),
            )
            .item(
                CommandItem::new().label("Billing")
                    .action(Box::new(OpenBilling)),
            ),
    )
    .placeholder("Type a command or search...")
    .empty(|_, _, cx| {
        v_flex()
            .items_center()
            .gap_2()
            .child(Icon::new(IconName::Search).size_8())
            .child("No results found.")
    })
    .w(px(380.))
```

Do not provide a manually formatted shortcut string. `CommandItem::action`
provides both the executable behavior and, for the default row, the displayed
binding. A custom row owns its complete presentation, including any key hint.

### Quick Actions Without Search

Disable search for a compact action palette. It has no search field, retains
all entries, and `state.focus(window, cx)` focuses the Command frame so its
arrow, Enter, and Escape actions remain available.

```rust
let actions = cx.new(|cx| CommandState::new(window, cx));

Command::new(&actions)
    .searchable(false)
    .items([
        CommandItem::new().label("New File").icon(IconName::Plus),
        CommandItem::new().label("Duplicate").icon(IconName::Copy),
        CommandItem::new().label("Move to Trash").icon(IconName::Delete),
    ])
    .w(px(380.))
```

With the default `.searchable(true)`, `state.focus(window, cx)` and
[`Focusable::focus_handle`] target the search input instead. A non-searchable
palette never invokes `on_query`.

### In a Dialog

Compose the palette with the existing [`WindowExt::open_dialog`] API. `header`
renders above the optional search field and list; `footer` renders below the
list. In a searchable palette, Escape clears a non-empty query. Otherwise—
including a non-searchable palette with a hidden programmatic query—Command
calls `on_cancel` and then propagates Cancel. Let the hosting Dialog perform
dismissal—do not close it again in `on_cancel`.

```rust
use gpui_kit::component::WindowExt as _;

let state = self.command_state.clone();
window.open_dialog(cx, move |dialog, _, _| {
    let state = state.clone();
    dialog.close_button(false).p_0().content(move |content, _, _| {
        content.child(
            Command::new(&state)
                .bordered(false)
                .placeholder("Type a command or search...")
                .items([
                    CommandItem::new().label("Profile"),
                    CommandItem::new().label("Billing"),
                ])
                .on_confirm(|index, window, cx| {
                    window.push_notification(format!("Selected {index}"), cx);
                })
                // Record local cleanup only; Dialog handles the propagated Cancel.
                .on_cancel(|window, cx| {
                    window.push_notification("Command palette cancelled", cx);
                })
                .header(|state, _, cx| {
                    h_flex()
                        .justify_between()
                        .px_3()
                        .py_2()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child("Commands")
                        .child(format!("{} matches", state.matched_count()))
                })
                .footer(|_, _, cx| {
                    h_flex()
                        .gap_3()
                        .px_3()
                        .py_2()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .child("↑↓ Navigate")
                        .child("Enter Select")
                        .child("Escape Close")
                }),
        )
    })
});
```

### Callbacks and Actions

Callbacks are configured on `Command`, not subscribed from `CommandState`.
They notify the palette owner directly:

```rust
Command::new(&state)
    .items(entries)
    .on_query(|query, window, cx| {
        // Start or update an application-owned search.
    })
    .on_select(|index, window, cx| {
        // Preview the newly highlighted IndexPath.
    })
    .on_confirm(|index, window, cx| {
        // Finish with this IndexPath, whether or not it has an Action.
    })
    .on_cancel(|window, cx| {
        // Clean up local palette state before Cancel propagates.
    })
```

An `IndexPath` always addresses the model supplied by the latest `Command`
render, before local filtering. Items passed to `.items(...)` are in section 0,
with `row` equal to their position in that iterator. Explicit groups use their
group and item positions; when both forms are mixed, they follow the implicit
ungrouped section. Filtering changes what is visible, not these coordinates.

`on_query` runs only when a searchable query actually changes. Refiltering can
move the highlight, so its `on_select` runs first when the selected `IndexPath`
changes; then `on_query` runs. These callbacks, and `on_confirm`, are delivered
after the current `CommandState` update releases its lease. Keyboard and
pointer highlight changes run `on_select` but never dispatch an Action. While
the source window remains live, confirming an enabled item dispatches its
Action first and then invokes `on_confirm`; if that Action closes the window,
the callback cannot be delivered. An item without an Action still invokes
`on_confirm`. In a searchable palette, Escape clears a non-empty query.
Otherwise—including a non-searchable palette with a hidden programmatic
query—it invokes `on_cancel`, then propagates Cancel.

### Dynamic Entries

Keep asynchronous or changing entries in the owner view, then reconstruct the
Command from the owner's current data when that view renders. Do not mutate the
state with an entry builder or `set_entries`.

```rust
struct StockSearch {
    state: Entity<CommandState>,
    results: Vec<CommandItem>,
}

impl StockSearch {
    fn render_palette(&self, owner: WeakEntity<Self>) -> Command {
        let results = self.results.clone();

        Command::new(&self.state)
            .items(results)
            .on_query(move |query, window, cx| {
                _ = owner.update(cx, |this, cx| this.search(query, window, cx));
            })
    }
}
```

The installed model remains in `CommandState` while query, selection, and
scrolling change, so those interactions do not need an owner rerender. A later
owner render installs the new model, preserves the selected `IndexPath` when it is
still present, and remeasures rows.

## Searching

Command uses a case-insensitive substring match against each item's label and
keywords. Empty queries match every item. A group whose items all filter out
hides its heading; a separator left leading, trailing, or adjacent to another
separator is omitted.

```rust
CommandItem::new().label("Profile")
    .keywords(["account", "user"])
```

For custom or remote search, update owner-held entries in `on_query` and call
`state.set_loading(true, window, cx)` while waiting so the
empty message is suppressed. Render the new entries when the response arrives.

## Custom Rows and Virtualization

`CommandItem::child` replaces an item's icon and label content with a lazy
child factory. The factory can run more than once for measurement, viewport
entry, and typography or width invalidation, so it must be side-effect-free.

On invalidation, Command creates and layout-measures every flattened row before
supplying independent sizes to `v_virtual_list`. Custom rows may therefore have
different intrinsic heights; `v_virtual_list` still renders and paints only
viewport rows. Build them for the available list width and keep their rendered
content stable until the owner updates the entries.

```rust
Command::new(&state)
    .item(CommandItem::new().label("compact").child(|_, _| {
        h_flex().w_full().py_1().child("Compact custom row")
    }))
    .item(CommandItem::new().label("expanded").child(|_, cx| {
        v_flex()
            .w_full()
            .py_4()
            .child("Expanded custom row")
            .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Extra detail"))
    }))
```

## Command

| Method | Signature and description |
| --- | --- |
| `new` | `new(&Entity<CommandState>) -> Command` creates a palette for a state. |
| `item` / `items` | `item(CommandItem) -> Self` and `items(impl IntoIterator<Item = CommandItem>) -> Self` add ungrouped entries. |
| `group` / `separator` | `group(CommandGroup) -> Self` adds a group; `separator() -> Self` adds a divider. |
| `searchable` | `searchable(bool) -> Self` shows or hides the search field and local filtering. Default: `true`. |
| `on_query` | `on_query<F>(F) -> Self`, where `F: Fn(&str, &mut Window, &mut App) + 'static`, runs after a searchable query changes. |
| `on_select` | `on_select<F>(F) -> Self`, where `F: Fn(IndexPath, &mut Window, &mut App) + 'static`, runs when the highlighted path changes. |
| `on_confirm` | `on_confirm<F>(F) -> Self`, with the same `IndexPath` callback bounds; while the source window remains live, runs after the confirmed Action dispatches. |
| `on_cancel` | `on_cancel<F>(F) -> Self`, where `F: Fn(&mut Window, &mut App) + 'static`, runs before Cancel propagates when Escape does not clear a searchable query. |
| `placeholder` | `placeholder(impl Into<SharedString>) -> Self` sets the search-field placeholder. |
| `empty` | `empty<F, E>(F) -> Self` renders custom content when there are no matches. |
| `max_h` | `max_h(impl Into<DefiniteLength>) -> Self` sets the list maximum. Default: `18.75rem` (300px). |
| `bordered` | `bordered(bool) -> Self` draws the surrounding border and rounding. Default: `true`. |
| `header` | `header<F, E>(F) -> Self`, where `F: Fn(&CommandState, &mut Window, &mut App) -> E + 'static` and `E: IntoElement`; renders above search and list. |
| `footer` | `footer<F, E>(F) -> Self`, with the same callback bounds; renders below the list. |

`Command` implements [`Styled`], so `w`, `max_w`, `bg`, and other styles apply
to the palette frame.

## CommandItem

| Method | Description |
| --- | --- |
| `new` | Creates an item; Command generates its internal rendering identity. |
| `label` | Sets the visible label and default search text. |
| `icon` | Sets the leading icon for the default row. |
| `action` | `action(Box<dyn Action>) -> Self` sets the behavior dispatched on click or confirm. The default row displays its resolved binding. |
| `checked` | Draws a trailing check. A resolved Action binding uses that position instead. |
| `keywords` | Adds default-match terms. |
| `disabled` | `Disableable::disabled(bool) -> Self` makes the item non-interactive and skips it during keyboard navigation. |
| `child` | `child<F, E>(F) -> Self`, where `F: Fn(&mut Window, &mut App) -> E + 'static` and `E: IntoElement`; lazily replaces the default row content. |

## CommandGroup

| Method | Description |
| --- | --- |
| `new` | Creates an unlabeled group. |
| `label` | Sets the group heading, which hides when all items filter out. |
| `item` / `items` | Add one or many `CommandItem`s to the group. |
| `heading` | Returns the optional heading. |

`CommandEntry` is the public enum for an item, group, or separator. It is useful
when an owner stores a mixed dynamic entry collection; replay each variant onto
a newly constructed `Command` during rendering.

## CommandState

| Method | Signature and description |
| --- | --- |
| `new` | `new(&mut Window, &mut Context<Self>) -> Self` creates empty interaction state. |
| `query` / `set_query` | Read the query, or `set_query(query, window, cx)` as if it were typed. |
| `selected_index` | Returns the highlighted item's original `IndexPath`; section identifies the top-level entry and row identifies the item within a group. |
| `matched_count` | Returns the number of matching items. |
| `focus` | `focus(&self, &mut Window, &mut App)` focuses the input when searchable, otherwise the Command frame. |
| `set_loading` / `is_loading` | Show or read the search spinner; loading suppresses the empty message. |

## Keyboard Shortcuts

| Key | Action |
| --- | --- |
| `↑` / `↓` | Move the highlight, wrapping around and skipping disabled items. |
| `Enter` | Confirm the highlighted item. |
| `Escape` | In a searchable palette, clear a non-empty query; otherwise call `on_cancel` and propagate `Cancel`. |

## Best Practices

1. Build static entries, groups, separators, searchability, and filters on `Command`.
2. Keep dynamic entries and asynchronous results in the palette owner; rebuild `Command` from them when rendering.
3. Bind real `Action`s instead of supplying shortcut text, so hints and dispatch stay in sync.
4. Keep `child` factories side-effect-free and use them for rows that need custom presentation or variable heights.
5. Let a hosting Dialog own cancellation after `on_cancel`; use header and footer for application-owned status and hints.
6. Give each independently rendered palette its own [`CommandState`].

[Command]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.Command.html
[CommandState]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.CommandState.html
[CommandGroup]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.CommandGroup.html
[WindowExt::open_dialog]: https://docs.rs/gpui-component/latest/gpui_component/trait.WindowExt.html#tymethod.open_dialog
[Focusable::focus_handle]: https://docs.rs/gpui/latest/gpui/trait.Focusable.html#tymethod.focus_handle
[Styled]: https://docs.rs/gpui/latest/gpui/trait.Styled.html
