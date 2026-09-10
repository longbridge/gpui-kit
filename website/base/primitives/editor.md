---
title: Editor
description: An unstyled source-code editor with language, gutter, folding, and decoration support.
order: 16
---

# Editor

`Editor` is the source-code editing control. It builds on the shared text engine
and adds a language, line-number gutter, folding, whitespace display, text
decorations, highlighting, search infrastructure, diagnostics, and LSP hooks.

Use [Input](./input.md) for single-line values and
[Textarea](./textarea.md) for ordinary multi-line text.

## Language editing rules

The Base editor accepts `LanguageConfig` and independent `auto_close` / `smart_indent`
preferences. It loads registered language configurations without a parser;
Component installs a `LanguageProvider` for built-in names, defaults, and syntax
providers during initialization. Base clients can install their own service
with `set_language_provider`; configurations are set with `set_language_config`.
See [Language editing rules](../../component/editor.md#language-editing-rules)
for the configuration fields and language registration; import the same types
from `gpui_kit::base::input` when using Base directly.


## Keyboard shortcuts

The base and styled editors share keyboard and mouse behavior. See
[Keyboard shortcuts and column selection](../../component/editor.md#keyboard-shortcuts-and-column-selection)
for the macOS, Linux, and Windows bindings, multi-cursor editing, and column-selection details.

## Search

The editor has a built-in search panel. Press `Ctrl-F` (Windows/Linux) or
`Cmd-F` (macOS) while the editor is focused to open it. See
[Search](../../component/editor.md#search) for the programmatic API
(`open_search`, `close_search`, `set_searchable`) and read-only behavior.

## Import

```rust
use gpui_kit::base::input::{Editor, EditorState, TabSize};
```

## Basic usage

```rust
let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language("rust")
        .line_number(true)
        .folding(true)
        .tab_size(TabSize {
            tab_size: 4,
            hard_tabs: false,
        })
        .default_value("fn main() {\n    println!(\"Hello\");\n}")
});

Editor::new(&editor)
```

## Whitespace and decorations

```rust
let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language("rust")
        .show_whitespaces(true)
        .default_value(source)
});

let decorations = editor.update(cx, |state, cx| {
    state.create_decorations_collection(initial_decorations, cx)
});
```

Decoration collections track ranges as the text changes. Keep the returned
collection alive for as long as its decorations should remain active.

## Highlighting and language features

`InputHighlighterFactory`, `InputHighlighter`, diagnostic types, and the LSP
provider traits are low-level extension seams for design-system authors. They
operate on the shared `InputBaseState`; applications using the styled component
normally configure these through their editor integration rather than through
ordinary text fields.

The runnable showcase demonstrates this seam with `syntect`. It selects the
WASM-compatible `fancy-regex` backend rather than the native Oniguruma backend,
so the same Rust highlighting adapter runs in the desktop example and the Base
WASM example. Syntect only identifies syntax scopes: the adapter maps those to
semantic names and resolves their styles through `HighlightStyleResolver`, so
the application theme remains the source of colors and font styles. The adapter
is intentionally simple and reparses the short sample after each edit;
production integrations can keep incremental parser state in their
`InputHighlighter` implementation.

## Font

The editor has no font setting of its own: it paints with the ambient text
style, so the family, size, weight, and line height come from the element the
application wraps it in.

```rust
div()
    .font_family("JetBrains Mono")
    .text_size(px(13.))
    .child(Editor::new(&editor))
```

A relative `line_height` keeps the rows in step with the glyphs at any size; an
absolute one stays put. For a ready-made monospace treatment, see the
[`gpui-component` Editor](../../component/editor.md).

## Presentation

The application owns editor colors, gutter appearance, fold icons, and overlay
content. Use `InputEditorStyle`, `FoldIconRenderer`, and the provider traits to
connect those adapters. For the repository's ready-made visual treatment, see
the [`gpui-component` Editor](../../component/editor.md).

## Runnable example

```bash
cargo run -p gpui-base-examples -- editor
```
