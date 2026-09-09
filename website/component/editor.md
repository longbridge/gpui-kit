---
title: Editor
description: Source-code editor with syntax highlighting, gutter, folding, and decorations.
---

# Editor

`Editor` is the styled source-code control. Use [`Input`](./input.md) for
single-line values and [`Textarea`](./textarea.md) for ordinary multi-line text.

## Import

```rust
use gpui_kit::component::input::{Editor, EditorState, TabSize};
```

## Language editing rules

`LanguageConfig` describes a language; `.auto_close(bool)` and `.smart_indent(bool)`
are independent editor preferences. Changing languages or replacing rules does
not reset either preference. Automatic closing, skip-over, and paired Backspace
use `auto_closing_pairs`. Enter uses `brackets` and `indentation_rules`, so it
can still split an existing pair when automatic closing is disabled.

```rust
use gpui_kit::component::input::{
    AutoClosingPair, BracketPair, language_config::LanguageConfig, SyntaxContext, set_language_config,
};

let rules = LanguageConfig::default()
    .brackets([BracketPair::new("{", "}"), BracketPair::new("(", ")")])
    .auto_closing_pairs([
        AutoClosingPair::new("{", "}")
            .not_in([SyntaxContext::String, SyntaxContext::Comment]),
        AutoClosingPair::new("(", ")")
            .not_in([SyntaxContext::String, SyntaxContext::Comment]),
    ])
    .auto_close_before(";:.,=}])>");

set_language_config("rust", rules, cx);

let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language("rust")
        .auto_close(true)
        .smart_indent(true)
});
```

`set_language_config` replaces the configuration for a language in the current
application. Existing editors use the replacement on their next edit, including
within the same event handler. Aliases share configurations: `python`, `py`, and
`pyi` refer to the same language even without its grammar feature. Custom
configurations survive component initialization. Exact custom grammar registrations
take precedence over built-in aliases and retain their original case. Unknown languages use
`LanguageConfig::default()`.

Component installs a `LanguageProvider` for language names, editing defaults,
and editor-owned syntax providers. Syntax selection follows the language on the
first edit and after language changes, independently of rendering. Base clients
can install their own service with `set_language_provider`; ordinary Component
clients only need `set_language_config`. Grammar resources are available as
`highlighter::GrammarConfig`; its existing `highlighter::LanguageConfig` name
remains compatible.

Pairs use strings, including multi-character delimiters. `auto_closing_pairs`
is optional: `None` uses the structural `brackets`, while `Some(vec![])` disables
all automatic pairs. Its builder sets `Some`. Whitespace and end-of-document
always allow automatic insertion; `auto_close_before` lists other allowed
following characters. `not_in` requires a syntax-context provider; without one,
the Base editor reports `Code`. The styled editor supplies a provider when the
language's Tree-sitter grammar is enabled.

`IndentationRules::new(increase, decrease)` accepts two compiled `regex::Regex`
patterns. On Enter, the increase pattern tests text before the cursor and the
decrease pattern tests text after it. Without an increase pattern, structural
opening brackets provide the default indentation. These rules do not reformat
existing lines or pasted text. Python's language defaults additionally recognize
a trailing colon; unknown languages use structural brackets only.

This is the supported subset of Monaco-style language configuration, not a
loader for Monaco JSON or Tree-sitter `.scm` files. Selection-surrounding and
custom `onEnterRules` are not part of this interface yet.


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

Editor::new(&editor).h(px(320.))
```

The language set via `.language()` selects syntax highlighting. Enable the
matching Cargo feature, such as `tree-sitter-rust` or `tree-sitter-markdown`;
use `tree-sitter-languages` to bundle all built-in grammars.

## Editor options

```rust
let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language("json")
        .line_number(true)
        .folding(true)
        .show_whitespaces(true)
        .default_value(source)
});
```

## Keyboard shortcuts and column selection

These defaults apply while the editor is focused. On macOS, Option is the Alt
modifier. Linux uses no Super/Win bindings for these operations.

| Operation | macOS | Linux | Windows |
| --- | --- | --- | --- |
| Add a cursor above / below | Cmd+Option+Up / Down | Alt+Shift+Up / Down | Ctrl+Alt+Up / Down |
| Extend every selection by one character | Shift+Left / Right | Shift+Left / Right | Shift+Left / Right |
| Extend every selection by one word | Option+Shift+Left / Right | Ctrl+Shift+Left / Right | Ctrl+Shift+Left / Right |
| Add a cursor with the mouse | Option+left click | Alt+left click | Alt+left click |
| Select a rectangular block | Option+Shift+left drag | Alt+Shift+left drag | Alt+Shift+left drag |
| Keep only the active cursor | Escape | Escape | Escape |

Linux also accepts Ctrl+Alt+left drag for rectangular selection, matching
Ghostty, and Alt+Shift+Left / Right for word selection. Windows additionally
accepts Alt+Shift+Left / Right for character selection. Alt/Option+left drag
works as a column-selection shortcut on all three platforms: a click adds a
cursor, while dragging builds a new block from the mouse-down position.

Holding Alt/Option over the editor shows a `+` crosshair. Selection gestures
that include Alt take priority over Ctrl/Cmd-click go-to-definition. A block
creates one selection per display row, clipped to the available text on short
rows. Typing or deleting edits all selections. Releasing the mouse ends the
drag; Escape keeps the active cursor (an open context menu handles Escape
first).

Adding cursors with Up / Down is additive: reversing direction does not shrink
the block's height. This is multi-cursor editing with mouse column selection,
not a persistent Vim Visual Block mode. During keyboard input, carets remain
visible; blinking resumes after 300 ms without input.

Linux desktop shortcuts can intercept key combinations before the editor sees
them. In particular, Ctrl+Alt+Up / Down is not bound by default on Linux because
some desktops use it to switch workspaces. The shortcuts above refer to logical
modifiers after any keyboard remapping.

## Search

The editor has a built-in search panel. Press `Ctrl-F` (Windows/Linux) or
`Cmd-F` (macOS) while the editor is focused to open it. `Enter` jumps to the
next match, `Shift+Enter` to the previous one, `Escape` closes the panel.

```rust
// Open the find panel programmatically
editor.update(cx, |state, cx| {
    state.open_search(false, cx);
});

// Close it
editor.update(cx, |state, cx| {
    state.close_search(cx);
});
```

Search is enabled by default for `Editor`. To disable it:

```rust
editor.update(cx, |state, cx| {
    state.set_searchable(false, cx);
});
```

A read-only editor can still be searched — the replace UI is hidden
automatically.

## Decorations

```rust
let decorations = editor.update(cx, |state, cx| {
    state.create_decorations_collection(initial_decorations, cx)
});
```

Keep the returned `TextDecorationCollection` alive while the decorations are
needed. Its ranges follow subsequent text edits.

## Value and events

```rust
let source = editor.read(cx).value();

editor.update(cx, |state, cx| {
    state.set_value(new_source, window, cx);
});

cx.subscribe(&editor, |this, state, event: &InputEvent, cx| {
    if matches!(event, InputEvent::Change) {
        this.source = state.read(cx).value();
        cx.notify();
    }
});
```

## Font

The editor paints its code in the theme's monospace font — `mono_font_family` at
`mono_font_size` — with rows 1.5 times the font size. That is only the default:
a text style set on the editor refines over it, and the gutter and row height
follow the size. The theme's platform default (`Menlo`, `Consolas`, `DejaVu Sans
Mono`) is checked against the installed fonts when the theme loads and swapped
for an installed monospace font, or `.SystemUIFont`, when it is missing; a
family you set yourself is used as-is.

```rust
Editor::new(&editor).text_sm()

Editor::new(&editor)
    .font_family("JetBrains Mono")
    .text_size(px(15.))
```

These are the ordinary [`Styled`](https://docs.rs/gpui/latest/gpui/trait.Styled.html)
methods every element has, so `font_weight` and `line_height` work the same way.

## Appearance

```rust
Editor::new(&editor)
    .h(px(480.))
    .bordered(true)
    .disabled(false)
    .readonly(false)
    .aria_label("Rust source")
```

Use `readonly` to preview a file without allowing changes. Unlike `disabled`,
a read-only editor keeps the normal appearance and still can be focused,
selected, copied and searched, it only rejects the changes made by the user.
The programmatic APIs such as `set_value` keep working.

```rust
Editor::new(&editor).readonly(true)
```

Editor focus does not add the single-line Input focus-border treatment. The
gutter, current-line background, and scrollbars are painted as one aligned
editor surface.

Input-only adornments such as `prefix`, `suffix`, mask toggle, and clear button
are intentionally absent. Compose toolbars and actions around `Editor`.
