---
title: Switch
description: A controlled on/off control with separately styleable track and thumb.
order: 28
---

# Switch

A controlled on/off control with separately styleable track and thumb.

Like every `gpui-base` primitive, Switch supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- switch
```

## Import

```rust
use gpui_kit::base::{Switch, SwitchThumb, SwitchTrack};
```

## Anatomy and API

The example composes `Switch`, `SwitchThumb`, `SwitchTrack`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/switch.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/switch.rs). Native and browser previews compile this same file.

## State and events

Pass the controlled boolean to `checked`; `on_change` emits the requested next value.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/switch.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Label the setting, expose checked state, and keep the accessible name stable.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
