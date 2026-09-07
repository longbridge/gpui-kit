---
title: Radio Group
description: Groups radio items and provides keyboard navigation for a single selection.
order: 22
---

# Radio Group

Groups radio items and provides keyboard navigation for a single selection.

Like every `gpui-base` primitive, Radio Group supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- radio-group
```

## Import

```rust
use gpui_kit::base::{Radio, RadioGroup};
```

## Anatomy and API

The example composes `Radio`, `RadioGroup`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/radio_group.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/radio_group.rs). Native and browser previews compile this same file.

## State and events

The group coordinates one selected value and keyboard movement; `Radio` renders each option.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/radio_group.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Label the group, expose one checked item, and support arrow keys among enabled choices.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
