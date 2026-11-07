---
title: Time Field
description: A segmented time-of-day editor with a complete keyboard model and 24- or 12-hour clocks.
order: 30.5
---

# Time Field

A segmented time-of-day editor with a complete keyboard model and 24- or 12-hour clocks.

Like every `gpui-base` primitive, Time Field supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- time-field
```

## Import

```rust
use gpui_kit::base::{HourCycle, TimeField, TimeFieldEvent, TimeFieldState, TimePrecision};
```

## Anatomy and API

The example composes `TimeField` over a `TimeFieldState`. The field renders one `TimeFieldSegment` per hour, minute, optional second and optional AM/PM part, separated by `:`. Lay out and style the root with `Styled`, and decorate each segment through `TimeField::render_segment`; the slot receives a `TimeFieldSegmentState` with the segment, its value and whether it is selected.

The authoritative module is [`components/time_field.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/time_field.rs). Native and browser previews compile this same file.

## State and events

`TimeFieldState` owns the time, its precision (`TimePrecision::Minute` or `Second`) and hour cycle (`HourCycle::H23` by default, or `H12`). `set_time` replaces the value without emitting; user edits emit `TimeFieldEvent::Change`.

The field is one Tab stop. Up/Down step the selected segment and wrap within it without carrying into the next unit, Left/Right and Tab/Shift-Tab move between segments, digits type a value with a two-digit buffer and advance once no further digit fits, `a`/`p` set AM or PM, and Backspace/Delete reset the segment.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/time_field.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

The root exposes `Role::TimeInput` with the formatted time as its value. Label the field, and keep the selected segment visibly distinct from the others.

## Notes

Use tabular figures or fixed segment widths so the field does not change width while digits are typed. Verify focus, selected, disabled, and high-contrast appearances in the consuming design system.
