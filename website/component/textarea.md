---
title: Textarea
description: Multi-line text input with fixed rows, soft wrapping, and auto-grow.
---

# Textarea

`Textarea` is the styled control for ordinary multi-line text. Use
[`Input`](./input.md) for a single line and [`Editor`](./editor.md) for source
code.

## Import

```rust
use gpui_kit::component::input::{Textarea, TextareaState};
```

## Basic usage

```rust
let notes = cx.new(|cx| {
    TextareaState::new(window, cx)
        .rows(5)
        .placeholder("Notes")
});

Textarea::new(&notes)
```

## Auto-grow

```rust
let message = cx.new(|cx| {
    TextareaState::new(window, cx)
        .auto_grow(2, 8)
        .placeholder("Write a message")
});

Textarea::new(&message)
```

The control grows until `max_rows`; overflowing content then scrolls.

## Value and events

```rust
let value = notes.read(cx).value();

notes.update(cx, |state, cx| {
    state.set_value("Updated notes", window, cx);
});

cx.subscribe(&notes, |this, state, event: &InputEvent, cx| {
    if matches!(event, InputEvent::Change) {
        this.notes = state.read(cx).value();
        cx.notify();
    }
});
```

`insert`, `replace`, `cursor_position`, `soft_wrap`, `searchable`, and
`submit_on_enter` are available on `TextareaState`.

## Appearance

```rust
Textarea::new(&notes)
    .h(px(160.))
    .bordered(true)
    .disabled(false)
    .readonly(false)
    .aria_label("Notes")
```

Unlike `disabled`, a read-only textarea keeps the normal appearance and still
can be focused, selected and copied, it only rejects the changes made by the
user.

`Textarea` deliberately does not expose Input-only adornments such as `prefix`,
`suffix`, mask toggle, or the clear button. Compose related actions beside the
textarea.
