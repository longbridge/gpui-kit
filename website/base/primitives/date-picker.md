---
title: Date Picker
description: A focus-aware date input that composes calendar behavior with a popup.
order: 10
---

# Date Picker

A focus-aware date input that composes calendar behavior with a popup.

Like every `gpui-base` primitive, Date Picker supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- date-picker
```

## Import

```rust
use gpui_kit::base::{DatePicker};
```

## Anatomy and API

The example composes `DatePicker`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/date_picker.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/date_picker.rs). Native and browser previews compile this same file.

## State and events

The picker combines focus/input state with calendar selection. Retain its entities on the parent view.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/date_picker.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Label the input, announce locale-appropriate dates, and make the calendar keyboard operable.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
