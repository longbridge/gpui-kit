---
title: Link
description: An accessible link-like control with application-defined styling.
order: 15
---

# Link

An accessible link-like control with application-defined styling.

Like every `gpui-base` primitive, Link supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- link
```

## Import

```rust
use gpui_kit::base::{Link};
```

## Anatomy and API

The example composes `Link`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/link.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/link.rs). Native and browser previews compile this same file.

## State and events

The link emits activation while the application defines URL or in-app navigation.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/link.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Use links for navigation, meaningful text, and a visible focus style.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
