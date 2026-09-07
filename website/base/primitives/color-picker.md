---
title: Color Picker
description: State and interaction foundations for selecting colors in a custom picker UI.
order: 8
---

# Color Picker

State and interaction foundations for selecting colors in a custom picker UI.

Like every `gpui-base` primitive, Color Picker supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- color-picker
```

## Import

```rust
use gpui_kit::base::{ColorPicker, ColorPickerEvent, ColorPickerState, ColorSwatch};
```

## Anatomy and API

The example composes `ColorPicker`, `ColorSwatch`, and `ColorPickerState`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

`ColorPicker` is the controlled root: it carries the trigger's accessibility semantics and focus, opens on Confirm, and dismisses on Cancel. `ColorSwatch` is one selectable color in a palette, carrying radio semantics, an accessible hex name, and the hover and activation callbacks a picker previews and commits with.

The authoritative module is [`components/color_picker.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/color_picker.rs). Native and browser previews compile this same file.

## State and events

`ColorPickerState` owns the committed color, the transient preview shown while the user hovers or edits, the controlled open state, and the active panel. It also owns a hex `InputState` and four component `SliderState`s and keeps all of them in sync, so an application renders those with its own input and slider presentation rather than reconciling them itself. Committing a color emits `ColorPickerEvent::Change`.

A color supplied to `default_value` cannot reach the hex field and sliders without a window, so call `sync_pending_value` from render; it is a no-op once nothing is pending.

Retain the state's entity on the parent view.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/color_picker.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Provide a textual color value and keyboard controls; never communicate selection by color alone. The root exposes the trigger's expanded state, and each swatch exposes its hex value as its accessible name plus its selected state, so a palette never depends on color alone.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
