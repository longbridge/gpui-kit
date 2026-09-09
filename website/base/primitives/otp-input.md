---
title: OTP Input
description: A multi-cell one-time-code input driven by a shared text state.
order: 17
---

# OTP Input

A multi-cell one-time-code input driven by a shared text state.

Like every `gpui-base` primitive, OTP Input supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- otp-input
```

## Import

```rust
use gpui_kit::base::{OtpInput, OtpState};
```

## Anatomy and API

The example composes `OtpInput`, `OtpState`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/otp_input.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/otp_input.rs). Native and browser previews compile this same file.

## State and events

`OtpState` owns the complete code and active cell; visual cells share that state.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/otp_input.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Label the whole code, announce length/errors, and support paste.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
