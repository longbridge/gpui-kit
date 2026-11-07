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

## Sizing to content

For host-controlled sizing, read `notes.read(cx).display_map().wrap_row_count()`.
The same [prewrap pattern as Editor](./editor.md#sizing-to-content) prepares the
count before the first layout; use the textarea's actual font, font size and
text-area width rather than the editor's monospace defaults.

## Scrolling

A scroll over a textarea whose text can scroll stays in the textarea, even at
the end of the text: it does not scroll the page around it. A textarea whose
text fits leaves the scroll to the page.

## Scroll bounce

```rust
let notes = cx.new(|cx| {
    TextareaState::new(window, cx)
        .rows(10)
        .scroll_bounce(true)
});
```

With `scroll_bounce(true)`, text taller than the textarea stretches past its
top or bottom when precise scrolling from a trackpad or Magic Mouse, or its
momentum, runs into the end, then springs back. Ordinary mouse wheels scroll
by lines and do not bounce. The frame stays put. It is on by default on iOS, Android and macOS, whose own scroll views bounce, and off elsewhere. When the system asks for reduced motion, the text
stops at its ends without stretching.

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

## Sizes

```rust
Textarea::new(&notes).large()
Textarea::new(&notes) // medium (default)
Textarea::new(&notes).small()
```

The size changes the text size and the padding around the text together. It does
not set the height: use `h` for a fixed height, and let `rows` or `auto_grow`
decide the height of a growing textarea.

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

## Atomic inline tokens

Use tokens to include mentions, file references or commands in a multi-line
message. Insert a token with `TextareaState::replace_with_token`, or restore a
saved draft with `set_value`. The default label is ready to use; add a renderer
when you want an icon or other custom content:

```rust
Textarea::new(&state)
    .token(|token, _, _| InputToken::new(token).icon(IconName::File))
```

Import `InputToken` from `gpui_kit::component::input` and `IconName` from
`gpui_kit::component`. A token wraps onto the next line as a whole, and auto-grow adjusts the textarea
height to fit. Put line breaks in the text between tokens. Use `on_token_hover`
to show a tooltip or preview without selecting or editing; disabled tokens never
report hover entry; disabling a hovered token sends its exit. See
[Input: atomic inline tokens](./input.md#atomic-inline-tokens) for editing,
activation, draft persistence, mode restrictions and JavaScript APIs.
