---
title: Alert Dialog
description: A modal confirmation surface for actions that need an explicit decision.
order: 2
---

# Alert Dialog

A modal confirmation surface for actions that need an explicit decision.

Like every `gpui-base` primitive, Alert Dialog supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- alert-dialog
```

## Import

```rust
use gpui_kit::base::{AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogDescription, AlertDialogPopup, AlertDialogTitle, AlertDialogTrigger};
```

## Anatomy and API

The example composes `AlertDialog`, `AlertDialogAction`, `AlertDialogCancel`, `AlertDialogDescription`, `AlertDialogPopup`, `AlertDialogTitle`, `AlertDialogTrigger`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/alert_dialog.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/alert_dialog.rs). Native and browser previews compile this same file.

## State and events

Opening and dismissal are managed by `AlertDialog`; application action buttons decide when destructive work is committed.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/alert_dialog.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Provide title and description, trap focus, offer cancel, and restore focus to the opener.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
