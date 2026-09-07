---
title: Input
description: An unstyled single-line text input with masking, validation, and number stepping.
order: 14
---

# Input

`Input` is the single-line text control in `gpui-base`. It owns editing behavior,
focus, selection, keyboard input, IME, masking, validation, and events while the
application supplies presentation.

Use [Textarea](./textarea.md) for ordinary multi-line text and
[Editor](./editor.md) for source code.

## Import

```rust
use gpui_kit::base::input::{Input, InputEvent, InputState};
```

## Basic usage

Create the persistent state once, then render `Input` with that entity:

```rust
let input = cx.new(|cx| {
    InputState::new(window, cx)
        .placeholder("Account name")
        .default_value("Ada")
});

Input::new(&input)
```

Read and update the value through the state:

```rust
let value = input.read(cx).value();

input.update(cx, |state, cx| {
    state.set_value("Grace", window, cx);
});
```

## Masking and validation

```rust
let password = cx.new(|cx| {
    InputState::new(window, cx)
        .placeholder("Password")
        .masked(true)
        .validate(|value, _| value.chars().count() >= 8)
});
```

For formatted values, combine `mask_pattern`, `pattern`, `min`, `max`, `step`,
or `step_by` as appropriate. `unmask_value()` returns the underlying value of a
masked input.

## Events

`InputState` emits `InputEvent::Change`, `PressEnter`, `Focus`, and `Blur`.

```rust
cx.subscribe(&input, |this, state, event: &InputEvent, cx| {
    if matches!(event, InputEvent::Change) {
        this.value = state.read(cx).value();
        cx.notify();
    }
});
```

## Presentation

`gpui-base` does not install product styling. Supply `InputEditorStyle` to the
state and compose the control inside your own frame. If you want the ready-made
theme, sizing, borders, prefix/suffix slots, and clear button, use the styled
[`gpui-component` Input](../../docs/components/input.md).

## Runnable example

```bash
cargo run -p gpui-base-examples -- input
```

The implementation is in
[`crates/base/examples/showcase/components/input.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/input.rs).
