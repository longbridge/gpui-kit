# History and UndoHistory Split

## Context

The public `History<I>` was originally an undo/redo store. It requires every
item to carry a version and exposes grouping, ignore, undo-stack, and redo-stack
concepts. Input no longer uses it: Input has its own transaction-aware
`UndoManager`. Dock still uses `History<TileChange>` as an undo store, while
`NavStack` uses it as a navigation trail and translates `undo` to `pop` and
`redo` to `forward`.

Those uses have different contracts. Undo returns the changes crossed so the
caller can reverse or replay them. Navigation returns the location reached and
must preserve its root. Renaming the existing methods would hide that mismatch.

## Decision

Expose two independent public data structures from `gpui-base` and through the
legacy `gpui-component::history` module:

- `History<T>` is a browser-style linear trail with a current entry.
- `UndoHistory<T>` is a grouped change log with undo and redo transactions.

This is a breaking API redesign. The old `HistoryItem` trait and old
`History` API are removed without deprecated compatibility aliases.

## History

`History<T>` stores a back stack containing the root through the current entry
and a forward stack containing entries left by `back`. It requires `T: Clone`
only for operations that return an owned entry.

Its public API is:

```rust
History::new()
History::max_entries(usize)

push(T)
current() -> Option<&T>
replace_current(T)
remove_current() -> Option<T>

can_back() -> bool
can_forward() -> bool
back() -> Option<T>
forward() -> Option<T>

entries() -> DoubleEndedIterator<Item = &T>
forward_entries() -> DoubleEndedIterator<Item = &T>
retain(impl FnMut(&T) -> bool)
clear()
```

`entries()` runs from the root to the current entry, so `.rev()` runs from the
current entry to the root. `forward_entries()` runs from the nearest forward
entry to the furthest.

`back()` keeps the root and returns the new current entry. `forward()` returns
the entry it restores. Pushing clears the forward branch. `replace_current`
pushes when empty. `remove_current` removes only the current entry, reveals the
previous entry when present, and leaves the forward branch intact. A zero entry
limit accepts pushes as no-ops rather than panicking.

`History` does not deduplicate equal entries. A navigation trail must preserve
`A -> B -> A`; callers may suppress an unchanged current entry themselves.

## UndoHistory

`UndoHistory<T>` owns undo and redo transactions. Version/group metadata is an
implementation detail, so `T` does not implement a companion trait or carry a
version field.

Its public API is:

```rust
UndoHistory::new()
UndoHistory::max_undos(usize)
UndoHistory::group_interval(Duration)

push(T)
undo() -> Option<Vec<T>>
redo() -> Option<Vec<T>>
can_undo() -> bool
can_redo() -> bool

start_grouping()
end_grouping()
is_ignoring() -> bool
set_ignoring(bool)
clear()
```

Each ungrouped push is one transaction. Timed or explicit grouping appends to
the current transaction. Undo returns a transaction newest-first so callers can
reverse its effects; redo returns it oldest-first so callers can replay it.
Pushing after undo clears the redo branch. While ignoring, `push` is a no-op.
A zero undo limit retains no transactions.

The old `unique` option is removed. Reordering equal entries is MRU-list
behavior, not navigation or undo behavior, and has no production consumer.

## Consumers and Exports

- Dock changes from `History<TileChange>` to `UndoHistory<TileChange>` and
  retains its public `undo` and `redo` actions.
- `NavStack` uses `History<NavEntry>`. Its `pop` calls `back`; its `forward`
  calls `forward`; `views` and `forward_views` use the iterator APIs.
- Input's stale `HistoryItem for Change` implementation and version field are
  removed. Its private transaction-aware `UndoManager` remains unchanged.
- `gpui_base::{History, UndoHistory}` and
  `gpui_component::history::{History, UndoHistory}` are both available.
- The English and Chinese History documentation explain the two types and stop
  presenting MRU behavior as part of `History`.

Longbridge migration is deliberately downstream work. Its `NavJournal` can
wrap `History<NavLocation>` while retaining domain logic such as
`RecordOutcome`, refinement, and restoration of unreachable targets.

## Verification

Unit tests cover navigation roots, repeated entries, branch truncation,
capacity, replacement, removal, retention, iterator order, and forward order.
Undo tests cover single transactions, timed and explicit grouping, undo/redo
order, redo truncation, ignore mode, and capacity.

Existing Dock and NavStack tests must pass after migration. Compatibility tests
verify both types are re-exported through `gpui-component`. Formatting, the
targeted `gpui-base` and compatibility tests, and a workspace check are run
before completion.
