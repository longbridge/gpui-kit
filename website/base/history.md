---
title: History
description: Browser-style navigation trails and grouped undo/redo transactions for application state.
order: 7
---

# History

`History<T>` and `UndoHistory<T>` keep two different kinds of application state. Both are independent of GPUI and leave applying a returned value to the caller, but their operations intentionally have different meanings:

- `History<T>` is a browser-style linear trail of locations, with back and forward navigation.
- `UndoHistory<T>` records changes as undo transactions, including changes grouped into one user action.

## Import

```rust
use gpui_kit::base::{History, UndoHistory};
```

## Which one to use

Choose the type from the meaning of the state, not from the names of the UI
commands that operate on it:

- Use `History<T>` when each entry is a location and moving backward or
  forward returns the location reached. The current root must remain available.
- Use `UndoHistory<T>` when each entry is a reversible change and undo or redo
  must return every change in one user transaction.
- Use a domain-specific manager when grouping depends on richer semantics than
  time or explicit boundaries. The Input component, for example, has a private
  transaction manager that understands typing, deletion, selection, and IME
  composition.

Within gpui-component, `NavStack` uses `History<NavEntry>` for page navigation,
while Dock's tiles canvas uses `UndoHistory<TileChange>` to reverse grouped
move and resize changes. Input deliberately keeps its specialized private undo
manager.

## `History`: a navigation trail

Push every location the user visits. The current entry is the last value in the trail. For example, after visiting `A -> B -> C`, `C` is current and going back returns the new current entry, `B`:

```rust
let mut history = History::new();
history.push("A");
history.push("B");
history.push("C");

assert_eq!(history.back(), Some("B"));
assert_eq!(history.current(), Some(&"B"));
```

`back()` never moves past the root entry; it returns `None` there. `forward()` restores the nearest entry that was left behind. Pushing a new entry after going back drops that forward branch, just as a browser does after opening a new page. `max_entries` bounds the root-to-current entries: lowering it removes the oldest active entries immediately, and moving forward at the limit removes the oldest active entry before restoring the next one.

`entries()` iterates from the root to the current entry. With the full `A -> B -> C` trail, it yields `A`, `B`, then `C`; `entries().rev()` yields `C`, `B`, then `A`. `forward_entries()` iterates from the nearest forward entry to the furthest. Use `retain` to remove invalid locations, `replace_current` to update the current location in place, and `remove_current` to remove it without discarding the forward branch.

| Method                                       | Does                                                                            |
| -------------------------------------------- | ------------------------------------------------------------------------------- |
| `new()`                                      | Creates an empty trail. `max_entries` defaults to 1000.                         |
| `max_entries(n)`                             | Caps root-to-current entries and immediately removes the oldest excess entries. |
| `push(entry)`                                | Makes `entry` current and drops the forward branch.                             |
| `back()`, `forward()`                        | Move through the trail and return the resulting current entry.                  |
| `current()`                                  | Returns the current entry.                                                      |
| `can_back()`, `can_forward()`                | Report whether movement in that direction is available.                         |
| `entries()`, `forward_entries()`             | Iterate the current trail and forward branch in navigation order.               |
| `replace_current(entry)`, `remove_current()` | Update or remove the current entry.                                             |
| `retain(keep)`, `clear()`                    | Remove rejected entries from both sides, or empty the trail.                    |

## `UndoHistory`: grouped undo and redo

Push a value for each change your application must reverse. To make a drag one undoable action, explicitly group all of its updates. `undo()` returns the group's changes newest first so the most recent change is reverted first; `redo()` returns the same group oldest first so it is applied in its original order:

```rust
let mut history = UndoHistory::new();
history.start_grouping();
history.push("move from x=0 to x=10");
history.push("move from x=10 to x=20");
history.end_grouping();

assert_eq!(
    history.undo(),
    Some(vec!["move from x=10 to x=20", "move from x=0 to x=10"]),
);
assert_eq!(
    history.redo(),
    Some(vec!["move from x=0 to x=10", "move from x=10 to x=20"]),
);
```

For changes whose boundary is not explicit, `group_interval` combines consecutive pushes close enough in time. A successful undo or redo ends that timed grouping window, so the next push starts a new transaction. Explicit grouping is separate: while it is active, a push still appends to the current transaction, including after an undo. A new push clears redo transactions. While replaying changes, use `set_ignoring(true)` to prevent the replay itself from being recorded.

| Method                                | Does                                                                                                                                                                                                |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `new()`                               | Creates an empty undo history. `max_undos` defaults to 1000.                                                                                                                                        |
| `max_undos(n)`                        | Caps undo transactions and immediately removes the oldest excess transactions. Redo preserves this cap.                                                                                             |
| `group_interval(duration)`            | Groups consecutive nearby pushes into one transaction.                                                                                                                                              |
| `start_grouping()`, `end_grouping()`  | Make subsequent pushes append to the current transaction; ending grouping stops that explicit append behavior. On an empty history, as in the example above, the first push starts the transaction. |
| `push(change)`                        | Records a change in the current or a new transaction and clears redo.                                                                                                                               |
| `undo()`, `redo()`                    | Return the latest transaction newest-first for undo, oldest-first for redo.                                                                                                                         |
| `can_undo()`, `can_redo()`            | Report whether a transaction is available.                                                                                                                                                          |
| `set_ignoring(bool)`, `is_ignoring()` | Control whether pushes are recorded.                                                                                                                                                                |
| `clear()`                             | Empties undo and redo transactions.                                                                                                                                                                 |
