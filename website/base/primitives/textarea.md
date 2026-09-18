---
title: Textarea
description: An unstyled multi-line text field with fixed rows or auto-grow behavior.
order: 15
---

# Textarea

`Textarea` is for ordinary multi-line text. Its interface stays focused on text
entry: rows, wrapping, auto-grow, value updates, insertion, replacement, and
cursor position. Code-editor concepts are intentionally kept on
[`Editor`](./editor.md).

## Import

```rust
use gpui_kit::base::input::{InputEvent, Textarea, TextareaState};
```

## Fixed rows

```rust
let notes = cx.new(|cx| {
    TextareaState::new(window, cx)
        .rows(5)
        .placeholder("Notes")
        .default_value("First line\nSecond line")
});

Textarea::new(&notes)
```

## Auto-grow

The textarea grows between the supplied minimum and maximum row counts. Once it
reaches the maximum, its content scrolls.

```rust
let message = cx.new(|cx| {
    TextareaState::new(window, cx)
        .auto_grow(2, 8)
        .placeholder("Write a message")
});

Textarea::new(&message)
```

## Editing the value

```rust
notes.update(cx, |state, cx| {
    state.insert("Appended text", window, cx);
});

let cursor = notes.read(cx).cursor_position(cx);
let value = notes.read(cx).value();
```

Use `soft_wrap(false)` when visual wrapping is undesirable. Set
`submit_on_enter(true)` only when Enter should submit instead of inserting a
line break. `TextareaState` emits the same `InputEvent` variants as `InputState`.

## Presentation

The control is unstyled. Your design system supplies the frame, height, colors,
padding, and `InputEditorStyle`. For a styled control, see the
[`gpui-component` Textarea](../../component/textarea.md).

## Runnable example

```bash
cargo run -p gpui-base-examples -- textarea
```

## Atomic inline tokens

Use tokens for mentions or references that users select and delete as a whole.
Insert one through the TextareaState you already use for this control:

```rust
use gpui_kit::base::input::InlineToken;

notes.update(cx, |state, cx| {
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
`gpui_kit::base::input`. In JavaScript, use `TextareaState.new()` from `gpui-base`;
it provides the same token methods.
