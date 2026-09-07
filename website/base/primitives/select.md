---
title: Select
description: A button-like selection control backed by an anchored, keyboard-navigable popup.
order: 25
---

# Select

A button-like selection control backed by an anchored, keyboard-navigable popup.

Like every `gpui-base` primitive, Select supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- select
```

## Import

```rust
use gpui_kit::base::{Select};
```

## Anatomy and API

The example composes `Select`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/select.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/select.rs). Native and browser previews compile this same file.

## State and events

The delegate/state owns items and selection; activation opens the list and selection closes it.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/select.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Set `.accessibility_label(...)` on the controlled root and
`.accessibility_value(...)` to its committed selection, not a temporary search
cursor. The root exposes its expanded state and accessible activation. Activation
requests an open-state change and moves focus between the trigger and content.
Disabled controls do not expose activation. The styled `Select` supplies its
committed value automatically, falling back to its placeholder when unselected.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
