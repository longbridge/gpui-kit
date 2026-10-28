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

Highlighting comes from one of two engines, chosen with a Cargo feature of
`gpui-kit`. Neither is enabled by default.

- `tree-sitter` parses a syntax tree, which also drives folding and
  syntax-aware editing. Each language is a grammar feature as above; all of
  them together add tens of megabytes to an application.
- `syntect` highlights about 200 languages with regular expressions for a few
  megabytes. It builds no syntax tree, so folding follows braces and editing
  has no syntax context. Use it when an application needs highlighted code
  rather than a structural editor:

```toml
gpui-kit = { version = "{{gpui_kit_version}}", features = ["syntect"] }
```

With both enabled, `tree-sitter` is used.

## Editor options

```rust
let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language("json")
        .line_number(true)
        .folding(true)
        .show_whitespaces(true)
        .scroll_bounce(true)
        .default_value(source)
});
```

`scroll_bounce(true)` lets code longer than the editor stretch past its top or
bottom and spring back with precise trackpad or Magic Mouse scrolling, as in the
[Textarea](./textarea.md#scroll-bounce). The gutter moves with the code; the
frame and scrollbar stay put. Ordinary mouse-wheel line scrolling does not bounce.

## Manual completions

`ShowCompletions` requests suggestions without editing text. It has **no default
shortcut**, so applications can choose bindings that fit the platform and avoid
input-method shortcuts such as Ctrl+Space. Editor bindings can use the context
expression `Input && mode == editor`; existing `Input` bindings continue to work.

```rust
use gpui_kit::component::input::ShowCompletions;

// Dispatch to the focused editor from an application command.
window.dispatch_action(Box::new(ShowCompletions), cx);

// Or address a specific editor; this also focuses it.
editor.update(cx, |state, cx| state.show_completions(window, cx));
```

The menu refines against the identifier prefix before the caret, using the same
letter, digit, and underscore boundaries as the editor's word helpers. An empty
prefix still requests suggestions. Manual requests use the LSP `INVOKED` trigger
kind without a trigger character. Disabled/readonly editors, editors without a
provider, and IME preedit do not issue a request. After committing IME text, the
command is available again.

If the completion menu is already open, the command preserves its selection.
Otherwise it cancels the previous overlay request and inline suggestion and
closes code actions before requesting completions. Late results cannot reopen
suggestions after editing, moving the caret, dismissing, or losing focus.

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

### Custom search UI

The search engine is usable without the panel, so an application can draw
its own search bar on top of the editor's matching, highlighting, scrolling
and replacing. `set_search_query` starts a search; the editor highlights the
matches until `close_search`. An editor that is not `searchable` never opens
the built-in panel and leaves `Ctrl-F` / `Cmd-F` to its ancestors, so the
application can bind the shortcut to its own search field.

```rust
let editor = cx.new(|cx| EditorState::new(window, cx).searchable(false));

// Search from the application's own field
editor.update(cx, |state, cx| {
    state.set_search_query("needle", true, cx);
});

// Navigate; each call scrolls the match into view
editor.update(cx, |state, cx| {
    state.next_search_match(cx);
    state.previous_search_match(cx);
});

// Describe the matches: "2/5"
let matcher = &editor.read(cx).search_session().matcher;
let label = matcher.label();
let count = matcher.len();
let current = matcher.current(); // None without matches

// Replace, when the editor is editable
editor.update(cx, |state, cx| {
    state.replace_current_search_match("replacement", window, cx);
    state.replace_all_search_matches("replacement", window, cx);
});

// End the search and its highlights
editor.update(cx, |state, cx| {
    state.close_search(cx);
});
```

Take the shortcut on the view that owns the search field:

```rust
use gpui_kit::component::input::Search;

div()
    .on_action(cx.listener(|this: &mut Self, _: &Search, window, cx| {
        this.search.update(cx, |search, cx| search.focus(window, cx));
    }))
    .child(Editor::new(&this.editor))
```

## Decorations

```rust
let decorations = editor.update(cx, |state, cx| {
    state.create_decorations_collection(initial_decorations, cx)
});
```

Keep the returned `TextDecorationCollection` to update or clear that owner's
text styles. Ranges follow edits; dropping the handle does not remove decorations.

### Geometric range decorations

Use a separate `RangeDecorationCollection` for continuous fills or one-logical-pixel
frames. These are paint-only annotations: they do not reserve inline space, add
widgets, intercept pointer events, or change keyboard focus.

```rust
use gpui_kit::component::input::{RangeDecoration, RangeDecorationStyle};

let review_ranges = editor.update(cx, |state, cx| {
    state.create_range_decorations_collection(
        vec![
            RangeDecoration::new(0..8).with_style(RangeDecorationStyle::Fill),
            RangeDecoration::new(12..24), // Frame is the default.
        ],
        cx,
    )
});

review_ranges.set(vec![RangeDecoration::new(4..16)], cx);
review_ranges.append(vec![RangeDecoration::new(20..28)], cx);
let tracked_ranges = review_ranges.get_ranges(cx);
review_ranges.clear(cx);   // Keep the collection available for reuse.
review_ranges.dispose(cx); // Invalidate this handle and all its clones.
```

Each collection owns only its own entries, so separate extensions cannot overwrite
one another. Dropping a handle leaves its collection in the editor; `dispose`
releases it permanently. Calls on disposed collections or a dropped editor are
no-ops. Individual decorations do not require an ID.

Ranges are half-open UTF-8 **byte offsets**, not character indices or line numbers.
Start/end offsets are clipped outward to valid character boundaries; empty,
reversed, and entirely out-of-document ranges are discarded. Both text and
geometric decorations share these tracking rules:

- Inserting at either edge does not grow the range; inserting inside it does.
- Replacements clip overlapping anchors to the replaced span; deleting an entire
  range removes the decoration.
- Undo/redo, `set_value`, and `replace_all` (including formatting) apply the same
  edit transforms. Annotations are **not** snapshots in undo history: undoing a
  deletion does not resurrect a removed decoration, and undoing a replacement
  does not recover its former interior anchors. Reset the collection from your
  semantic source when that distinction matters.
- Folding changes only visual projection. Hidden-only ranges are not painted;
  visible portions remain clipped to the viewport and follow soft-wrapped glyphs.

Fills paint behind frames, and both sit below the selection and glyphs. Within
one style, later collections/items paint over earlier ones. Without `with_color`,
frames use the editor foreground and fills use that color at 12% opacity, so the
fallback follows theme changes. An explicit color is owned by the application.

Visible-range queries use an interval index and skip folded buffer spans; they
do not scan every decoration per frame. Setting/appending entries rebuilds that
collection's index; edits update affected collections linearly without re-sorting.
The Editor showcase's **Decorations** tab demonstrates both collection types.

### Gutter and overview ruler decorations

Use these collections to mark whole lines outside the text, such as lines that a
version-control diff added, modified, or deleted. A `GutterDecorationCollection`
draws in a strip before the line numbers. An `OverviewRulerDecorationCollection`
draws on the vertical scrollbar track. Monaco calls this track the overview ruler.
Create one collection for each surface you want to use.

```rust
use gpui_kit::component::input::{
    GutterDecoration, OverviewRulerDecoration, OverviewRulerLane,
};

let style = &cx.theme().highlight_theme.style;
let (added, deleted) = (style.status.created(cx), style.status.deleted(cx));

let gutter = editor.update(cx, |state, cx| {
    state.create_gutter_decorations_collection(
        vec![
            GutterDecoration::new(10..42).with_color(added), // Marks every line touched by the range.
            GutterDecoration::new(60..60).with_color(deleted), // Marks the boundary above the line that starts at 60.
        ],
        cx,
    )
});
let ruler = editor.update(cx, |state, cx| {
    state.create_overview_ruler_decorations_collection(
        vec![
            OverviewRulerDecoration::new(10..42)
                .with_lane(OverviewRulerLane::Left)
                .with_color(added.opacity(0.6)),
        ],
        cx,
    )
});
```

- Use a non-empty range to mark every line it touches, like Monaco's
  `isWholeLine`. A range that ends at a line start does not mark that line. Use an
  **empty** range to mark a line boundary, such as where lines were deleted. When
  the range is at the start of a line, the mark appears above that line. Otherwise
  it appears below the line. In a document without a final newline, `text.len()`
  marks below the last line. The gutter draws a bar for a non-empty range and a
  pill for a boundary.
- These collections keep empty ranges; text and geometric collections discard them.
  Text inserted at an empty range goes after the range. If an edit empties a range,
  the collection keeps it as a boundary. Use the same handles and
  `set`/`append`/`clear`/`dispose` methods as the other decoration collections.
  Recompute the decorations and call `set` on the whole collection. Edit tracking
  keeps the marks on their lines between updates.
- The gutter reserves its strip while any gutter collection exists, including an
  empty collection. This keeps the text from shifting as marks change. The strip is
  3 px wide by default. Set its width with `gutter_decorations_width` or
  `set_gutter_decorations_width`. If a range starts inside a fold, the gutter draws
  a bar on the fold's row and marks the rest of the range after the fold as usual.
  `with_folded_color` sets the color of the bar on the fold's row.
- The overview ruler has four lanes. `Left`, `Center` (the default), and `Right`
  each use one third of the track. `Full` uses the whole track. By convention, use
  `Left` for version-control changes, `Center` for search results, and `Right` for
  diagnostics. A mark's position on the track is proportional to its lines' position
  in the document, and each mark is at least 6 px tall. Touching marks with the same color in the same lane merge.
- The marks draw under the scrollbar, whether it is Base's scrollbar or one supplied
  with `set_scrollbar_renderer`. They remain visible while the thumb fades.
- Without `with_color`, both collections use the editor foreground color. For
  version-control colors, read the highlight theme's Zed status keys `created`,
  `modified`, and `deleted` with `status.created(cx)`, `status.modified(cx)`, and
  `status.deleted(cx)`. If a key is missing, these methods use `success`,
  `warning`, and `error`.

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

## Sizing to content

`display_map().wrap_row_count()` is the number of rows the text takes once soft
wrapped, so a read-only editor can be sized to its content. Layout counts them
on the first frame. To know the count before that, call `prewrap` with the font,
font size and text-area width after setting the value:

```rust
use gpui_kit::component::ActiveTheme as _;

let editor = cx.new(|cx| {
    let mut state = EditorState::new(window, cx)
        .line_number(false)
        .default_value(snippet);
    let mut font = window.text_style().font();
    font.family = cx.theme().mono_font_family.clone();
    state.prewrap(font, cx.theme().mono_font_size, width, cx);
    state
});
let rows = editor.read(cx).display_map().wrap_row_count();
```

`width` is the editor's width less its padding, and less the line-number gutter
when line numbers are shown. Folded rows are still counted.
The font and font size must match the editor's actual text style. This example
uses its theme defaults; if you customize the editor's font or font size, pass
the same values to `prewrap`.

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

## Sizes

When an Editor shares a form with Input and Textarea, apply the same `Sizable`
size tier to coordinate their padding and density. This keeps their presentation
consistent while the editor retains its independently configured code font.

```rust
Editor::new(&editor).large()
Editor::new(&editor) // medium (default)
Editor::new(&editor).xsmall()
```

Unlike `Textarea`, the size changes only the padding around the code, not the
font size (see [Font](#font)). `xsmall` leaves no padding above and below the
code.

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
