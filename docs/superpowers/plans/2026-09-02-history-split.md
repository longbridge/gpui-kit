# History and UndoHistory Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the overloaded public history type with a browser-style `History<T>` and a grouped `UndoHistory<T>`, then migrate every in-repository consumer.

**Architecture:** Add `UndoHistory<T>` first without disturbing the existing type. Then atomically replace `History<T>` and migrate NavStack, Dock, Input, and compatibility exports in one compile-safe task. `History` owns a root-to-current stack and a nearest-last forward stack; `UndoHistory` owns vectors of transactions and all grouping metadata.

**Tech Stack:** Rust 2024, `instant`, GPUI unit and component tests, VitePress Markdown documentation.

**Spec:** `docs/superpowers/specs/2026-09-02-history-split-design.md`

## Global Constraints

- Both `History<T>` and `UndoHistory<T>` are public from `gpui-base` and the legacy `gpui-component::history` path.
- Remove `HistoryItem` and the old overloaded API without deprecated aliases.
- `History::back` and `History::forward` return the destination entry.
- `History::back` never removes the root.
- `UndoHistory::undo` returns newest-first; `redo` returns oldest-first.
- Do not add a dependency for either data structure.
- Do not modify the sibling `longbridge-gpui` repository in this PR.

---

### Task 1: Add Grouped UndoHistory

**Files:**
- Create: `crates/base/src/undo_history.rs`
- Modify: `crates/base/src/lib.rs`

**Interfaces:**
- Consumes: `instant::{Duration, Instant}`.
- Produces: public `UndoHistory<T>` with `new`, `max_undos`, `group_interval`, `push`, `undo`, `redo`, `can_undo`, `can_redo`, `start_grouping`, `end_grouping`, `is_ignoring`, `set_ignoring`, and `clear`.

- [ ] **Step 1: Write failing transaction and ordering tests**

Create `undo_history.rs` with the test module and a deliberately incomplete
type declaration so the tests compile far enough to prove the missing methods.
Assert the literal ordering contract:

```rust
let mut history = UndoHistory::new();
history.start_grouping();
history.push(1);
history.push(2);
history.push(3);
history.end_grouping();

assert_eq!(history.undo(), Some(vec![3, 2, 1]));
assert_eq!(history.redo(), Some(vec![1, 2, 3]));
```

Add independent tests proving ungrouped pushes form separate transactions,
`group_interval(Duration::from_secs(60))` combines immediate pushes, a new
push clears redo, ignore mode drops pushes, `clear` clears both directions,
`max_undos(2)` evicts the oldest transaction, and `max_undos(0)` retains none.

- [ ] **Step 2: Run the UndoHistory tests and verify RED**

Run:

```bash
cargo test -p gpui-base undo_history::tests --lib
```

Expected: compilation fails on the first deliberately missing `UndoHistory`
method, demonstrating that the new API is not implemented.

- [ ] **Step 3: Implement transaction-owned grouping**

Use transaction storage rather than putting metadata in `T`:

```rust
#[derive(Debug)]
pub struct UndoHistory<T> {
    undos: Vec<Vec<T>>,
    redos: Vec<Vec<T>>,
    last_changed_at: Instant,
    max_undos: usize,
    group_interval: Option<Duration>,
    grouping: bool,
    ignoring: bool,
}
```

`push` appends to the last transaction only while explicit grouping is active
or the configured interval has not elapsed. Otherwise it creates a transaction.
It clears redo only for a recorded push. `undo` moves the stored transaction to
redo and returns a reversed clone; `redo` moves it back and returns an
oldest-first clone. Implement `Default` as `new`. A zero limit records nothing.

- [ ] **Step 4: Export and verify UndoHistory GREEN**

Declare the module in `crates/base/src/lib.rs`, export `UndoHistory`, then run:

```bash
cargo test -p gpui-base undo_history::tests --lib
```

Expected: every transaction, ordering, grouping, ignore, and capacity test
passes with no warnings.

- [ ] **Step 5: Commit UndoHistory**

```bash
git add crates/base/src/undo_history.rs crates/base/src/lib.rs
git commit -m "base: add grouped UndoHistory"
```

### Task 2: Replace History and Atomically Migrate Consumers

**Files:**
- Replace: `crates/base/src/history.rs`
- Modify: `crates/base/src/nav_stack.rs`
- Modify: `crates/base/src/dock/tiles_state.rs`
- Modify: `crates/base/src/dock/tiles_geometry.rs`
- Modify: `crates/base/src/input/base/change.rs`
- Modify: `crates/base/src/lib.rs`
- Modify: `crates/ui/src/history.rs`
- Modify: `crates/ui/tests/base_compat.rs`

**Interfaces:**
- Consumes: `UndoHistory<T>` from Task 1.
- Produces: navigation `History<T>` with `new`, `max_entries`, `push`, `current`, `replace_current`, `remove_current`, `can_back`, `can_forward`, `back`, `forward`, `entries`, `forward_entries`, `retain`, and `clear`; migrated NavStack and Dock; no `HistoryItem` anywhere.

- [ ] **Step 1: Change History and compatibility tests to the new contract**

Replace the old `HistoryItem` fixtures with integer entries. Cover this trail:

```rust
let mut history = History::new().max_entries(3);
history.push(1);
history.push(2);
history.push(3);

assert_eq!(history.current(), Some(&3));
assert_eq!(history.back(), Some(2));
assert_eq!(history.back(), Some(1));
assert_eq!(history.back(), None);
assert_eq!(history.forward(), Some(2));
assert_eq!(history.entries().copied().collect::<Vec<_>>(), [1, 2]);
assert_eq!(history.entries().rev().copied().collect::<Vec<_>>(), [2, 1]);
assert_eq!(history.forward_entries().copied().collect::<Vec<_>>(), [3]);
```

Add separate tests for forward-branch truncation, repeated `1 -> 2 -> 1`
entries, limit eviction, zero capacity, replace, remove, retain on both stacks,
and clear. In `base_compat.rs`, compile both legacy re-exports:

```rust
let _: gpui_component::history::History<u8> = gpui_base::History::new();
let _: gpui_component::history::UndoHistory<u8> = gpui_base::UndoHistory::new();
```

- [ ] **Step 2: Run tests and verify RED before changing production code**

Run:

```bash
cargo test -p gpui-base history::tests --lib
```

Expected: compilation fails because navigation methods such as `max_entries`,
`back`, `forward`, and `entries` do not exist.

- [ ] **Step 3: Implement navigation History**

Use:

```rust
#[derive(Debug)]
pub struct History<T> {
    entries: Vec<T>,
    forward_entries: Vec<T>, // nearest entry is last
    max_entries: usize,
}

pub fn back(&mut self) -> Option<T>
where
    T: Clone,
{
    if self.entries.len() <= 1 {
        return None;
    }
    self.forward_entries.push(self.entries.pop().unwrap());
    self.current().cloned()
}

pub fn forward(&mut self) -> Option<T>
where
    T: Clone,
{
    let entry = self.forward_entries.pop()?;
    self.entries.push(entry);
    self.current().cloned()
}
```

Return `impl DoubleEndedIterator<Item = &T> + ExactSizeIterator` from both
iterator methods. Implement `Default`. On `push`, clear forward entries, skip
storage at zero capacity, and evict the oldest entry at the limit. `retain`
filters both stacks without reordering them. `remove_current` removes only the
current entry and preserves forward entries.

- [ ] **Step 4: Migrate NavStack to navigation semantics**

Remove `HistoryItem for NavEntry` and its version. Use `entries().len()`,
`entries()`, and `forward_entries()` for inspection. Pop clones the outgoing
top before calling `history.back()` and returns that outgoing view. Forward
uses the destination returned by `history.forward()`. Update `pop_to_root` so
each iteration records the outgoing view while `back()` selects the destination.

- [ ] **Step 5: Migrate Dock and Input to the split**

Change the tile field to `UndoHistory<TileChange>`, preserving its 100 ms group
interval and its public undo/redo behavior. Remove `TileChange.version` and its
trait implementation. Remove `Change.version`, its stale `HistoryItem`
implementation, and the unused import from Input.

- [ ] **Step 6: Finish exports and verify GREEN**

Export `History` and `UndoHistory` from `gpui-base`, re-export both from
`gpui-component::history`, and run:

```bash
cargo test -p gpui-base history::tests --lib
cargo test -p gpui-base nav_stack --lib
cargo test -p gpui-base dock --lib
cargo test -p gpui-component --test base_compat legacy_history_path_reexports_the_base_type
rg -n "HistoryItem|\.undos\(|\.redos\(" crates --glob '*.rs'
```

Expected: all tests pass; the search returns no results.

- [ ] **Step 7: Commit the atomic migration**

```bash
git add crates/base/src/history.rs crates/base/src/nav_stack.rs crates/base/src/dock/tiles_state.rs crates/base/src/dock/tiles_geometry.rs crates/base/src/input/base/change.rs crates/base/src/lib.rs crates/ui/src/history.rs crates/ui/tests/base_compat.rs
git commit -m "base: split navigation and undo history"
```

### Task 3: Rewrite Documentation and Verify the Branch

**Files:**
- Modify: `crates/base/README.md`
- Modify: `website/base/history.md`
- Modify: `website/zh-CN/base/history.md`

**Interfaces:**
- Consumes: final APIs from Tasks 1 and 2.
- Produces: matching English and Chinese public documentation for both types and final PR evidence.

- [ ] **Step 1: Rewrite documentation around the split**

Document `History` first as a browser-style trail, with an `A -> B -> C`
example showing that `back()` returns `B`. Document `entries()` and
`entries().rev()` order. Document `UndoHistory` separately with a grouped drag
example and newest-first undo/oldest-first redo. Remove all references to
`HistoryItem`, `unique`, and MRU behavior. Update the README catalog row to list
both types and their distinct purposes.

- [ ] **Step 2: Run fresh formatting and targeted verification**

Run:

```bash
cargo fmt --all -- --check
cargo test -p gpui-base history::tests --lib
cargo test -p gpui-base undo_history::tests --lib
cargo test -p gpui-base nav_stack --lib
cargo test -p gpui-base dock --lib
cargo test -p gpui-component --test base_compat
```

Expected: all commands exit zero with no failed tests.

- [ ] **Step 3: Run the broad compilation gate**

Run:

```bash
cargo check --workspace --all-targets
git diff --check origin/main...HEAD
```

Expected: both commands exit zero.

- [ ] **Step 4: Commit documentation**

```bash
git add crates/base/README.md website/base/history.md website/zh-CN/base/history.md
git commit -m "docs: distinguish navigation and undo history"
```

- [ ] **Step 5: Prepare the independent pull request**

Push `history-split`, open a PR targeting `longbridge/gpui-kit:main`, and
summarize the breaking API split, consumer migrations, and verification. Confirm
the PR begins at squash merge `0c746dff` from #2922 and contains no commits from
the former `nav-stack` branch.
