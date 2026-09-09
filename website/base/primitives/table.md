---
title: Table
description: Semantic table primitives for composing headers, bodies, rows, and cells.
order: 29
---

# Table

Semantic table primitives for composing headers, bodies, rows, and cells.

Like every `gpui-base` primitive, Table supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- table
```

## Import

```rust
use gpui_kit::base::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
```

## Anatomy and API

The example composes `Table`, `TableBody`, `TableCell`, `TableHead`, `TableHeader`, `TableRow`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/table.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/table.rs). Native and browser previews compile this same file.

## State and events

Rows and cells are stateless composition; sorting, selection, and mutations remain in the parent.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/table.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Use headers, preserve reading order, and separately expose sort and selection controls.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
