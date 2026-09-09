---
title: Dialog
description: A composable modal surface with focus management, backdrop, title, and close parts.
order: 11
---

# Dialog

A composable modal surface with focus management, backdrop, title, and close parts.

Like every `gpui-base` primitive, Dialog supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- dialog
```

## Import

```rust
use gpui_kit::base::{Dialog, DialogBackdrop, DialogClose, DialogDescription, DialogPopup, DialogTitle, DialogTrigger};
```

## Anatomy and API

The example composes `Dialog`, `DialogBackdrop`, `DialogClose`, `DialogDescription`, `DialogPopup`, `DialogTitle`, `DialogTrigger`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/dialog.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/dialog.rs). Native and browser previews compile this same file.

## State and events

`Dialog` manages modal presentation and dismissal; application callbacks own submitted work.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/dialog.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Provide title, initial and return focus, a focus trap, Escape policy, and explicit close action.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
