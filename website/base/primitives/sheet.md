---
title: Sheet
description: A modal surface that enters from an edge while managing dismissal and focus.
order: 26
---

# Sheet

A modal surface that enters from an edge while managing dismissal and focus.

Like every `gpui-base` primitive, Sheet supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- sheet
```

## Import

```rust
use gpui_kit::base::{Sheet};
```

## Anatomy and API

The example composes `Sheet`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/sheet.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/sheet.rs). Native and browser previews compile this same file.

## State and events

Open and dismissal mirror a dialog while placement chooses the entering edge.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/sheet.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Apply dialog semantics: title it, trap and restore focus, and provide close.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
