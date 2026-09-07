---
title: Avatar
description: An image with composable fallback content for a person or entity.
order: 3
---

# Avatar

An image with composable fallback content for a person or entity.

Like every `gpui-base` primitive, Avatar supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- avatar
```

## Import

```rust
use gpui_kit::base::{Avatar, AvatarFallback, AvatarImage};
```

## Anatomy and API

The example composes `Avatar`, `AvatarFallback`, `AvatarImage`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/avatar.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/avatar.rs). Native and browser previews compile this same file.

## State and events

`Avatar` is presentational. Supply fallback content for the image-loading and image-error paths.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/avatar.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Fallback text should identify the entity; decorative avatars should not duplicate nearby labels.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
