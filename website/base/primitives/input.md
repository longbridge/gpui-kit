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
[`gpui-component` Input](../../component/input.md).

## Runnable example

```bash
cargo run -p gpui-base-examples -- input
```

The implementation is in
[`crates/base/examples/showcase/components/input.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/input.rs).

## Atomic inline tokens

Use tokens for mentions or references that users select and delete as a whole.
Insert one through the InputState you already use for this control:

```rust
use gpui_kit::base::input::InlineToken;

input.update(cx, |state, cx| {
    state.replace_with_token(
        InlineToken::new("person-1", "@alice").with_label("Alice"),
        window,
        cx,
    ).expect("valid reference");
});
```

Tokens display as unstyled labels. Use the `token` slot to supply your own single-row
element and `on_token_click` to open a reference. Copy and `value()` return the
real text, such as `@alice`. Save drafts with `content()` and restore them with
`set_value(content)` to keep their references.

See [Input's token examples](../../component/input.md#atomic-inline-tokens) for
custom rendering, draft restoration and range units. Import the data types from
`gpui_kit::base::input`. In JavaScript, use `InputState.new()` from `gpui-base`;
it provides the same token methods.
