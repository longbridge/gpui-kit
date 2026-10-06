---
title: Diff
description: Readonly unified and Git diff display for code review and change previews.
---

# Diff

`Diff` displays an application-supplied unified or Git diff in a readonly
surface. Use it for code review and change previews; use [Editor](./editor.md)
when the user needs to edit source code.

The application obtains the patch from Git, an AI response, or another producer.
`DiffFile::parse` prepares one `DiffFile` per changed file without comparing
old and new versions. One `DiffState` shows all of those files in a single
virtualized list, each introduced by its own header.

## Import

```rust
use gpui_kit::*;
use gpui_kit::component::diff::{
    Diff, DiffFile, DiffLineAnnotation, DiffLinePosition, DiffLineRange,
    DiffMode, DiffSide, DiffState,
};
```

Initialize components with `gpui_kit::init(cx)` before opening the window through
`gpui_kit::open_window`. See [Getting Started](../docs/getting-started.md).

## Basic usage

Parse the patch and create its state once in the owning view:

```rust
let patch = "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,3 +1,3 @@\n fn main() {\n-    run();\n+    run_app();\n }\n";
let files = DiffFile::parse(patch)?;
let diff = cx.new(|cx| DiffState::new(files, cx));
```

`parse` returns `Result<Vec<DiffFile>, DiffParseError>`. Handle errors in the
application. The state also accepts a subset of files, or a single file as
`[file]`, when the application presents files separately.

Store the state as an `Entity<DiffState>` on the owner. Construct only the
presentation during rendering, and give it a bounded viewport:

```rust
Diff::new(&self.diff).h(rems(24.)).w_full()
```

Do not parse the patch or recreate its state on every render. Parsing is fast
(about 15 ms for a 6.5 MB, 200-file patch in a release build); for very large
patches, parse on a background executor, then install the files only if
the result still belongs to the current patch revision.

## Patch input

Accept unified diffs (`---`, `+++`, `@@`) and Git diff text, including
`git format-patch` output and patches written with `diff.noprefix` or
`diff.mnemonicPrefix`. Each parsed file produces one `DiffFile`. `path()` is the
path to show and identifies the file; `original_path()` and `modified_path()` return `None` for the
missing side of an added or deleted file (`/dev/null`). `additions()` and
`deletions()` report changed lines represented by the patch.

A patch normally contains only changed lines and nearby context. Its line numbers
are source coordinates, not proof that the complete source file is available.
Diff neither computes differences nor loads repository files, applies changes,
or resolves merge conflicts. Source outside the patch remains unavailable.

`extended_headers()` exposes Git's extended header lines, such as modes,
similarity, renames and the binary marker; `is_binary()`
identifies binary changes. A file without source rows, such as a binary file, a
mode change or a pure rename, shows a summary below its header. Malformed hunks
and unsupported combined diffs return a `DiffParseError`; its `line()` and
`message()` identify the patch location and reason.

## Display mode and context

The default display mode is `DiffMode::Unified`, with up to three available
unchanged lines around each change. Set the initial mode when creating the state,
and change it later from application commands:

```rust
let diff = cx.new(|cx| {
    DiffState::new(files, cx)
        .with_mode(DiffMode::Split)
        .with_context_lines(Some(5))
});

self.diff.update(cx, |state, cx| state.set_mode(DiffMode::Unified, cx));
```

Split shows the original and modified source side by side, with empty cells
where a line has no counterpart. Unified shows deleted lines before added
lines while retaining original source line numbers.

Hidden unchanged context is represented by a compact disclosure showing the
unchanged-line count. Activating it expands that entire hidden range. To expose
all context supplied by the patch without changing the configured context count,
call `expand_all`; call `collapse_all` to restore that count. Setting the context
count to `None` disables folding of available context:

```rust
self.diff.update(cx, |state, cx| {
    state.expand_all(cx);
    state.collapse_all(cx);
    state.set_context_lines(None, cx);
});
```

## Source coordinates and navigation

`DiffSide::Original` and `DiffSide::Modified` identify the file version.
`DiffLinePosition` and `DiffLineRange` identify a file by its `path()`, which
stays stable when a newer revision of the patch reorders its files, and use
**one-based source line numbers**, not visible row indices. A line range includes both endpoints and stays on one side of one file.
Display mode changes and context folding do not renumber source lines.

```rust
self.diff.update(cx, |state, cx| {
    state.scroll_to_line(DiffLinePosition::new("src/main.rs", DiffSide::Modified, 12), cx);
    state.scroll_to_file("src/main.rs", cx);
    state.next_change(cx);
    state.previous_change(cx);
});
```

`scroll_to_line` reveals supplied context when needed; it cannot reveal lines
absent from the patch. `scroll_to_file` shows a file's header, including a file
without source rows, for example from a file list. Change navigation moves
between changed groups across all files and wraps at the ends. Put display mode
and navigation commands in the application's toolbar or menu.

## Selection and copying

Select source lines programmatically using a file, a side and an inclusive range:

```rust
self.diff.update(cx, |state, cx| {
    state.set_selected_lines(Some(DiffLineRange::new("src/main.rs", DiffSide::Modified, 3, 8)), cx);
});

let source = self.diff.read(cx).selected_text(cx);

self.diff.update(cx, |state, cx| {
    state.set_selected_lines(None, cx);
});
```

`selected_lines()` reports the current line range. Line selection is limited to
supplied source lines; selecting a missing side clears the selection.
`selected_text(cx)` returns source text without line numbers, change markers,
alignment cells, or annotation content. Copy excludes diff prefixes and
unavailable gaps; it cannot reconstruct the whole file or infer source line
endings that the patch does not preserve.

## Replacing the files

```rust
self.diff.update(cx, |state, cx| {
    state.set_files(updated_files, cx);
});
```

Replacing the files resets expansion, selection, and scrolling because
their coordinates belong to the previous patch. State mutations notify
observers; callers do not need an additional `cx.notify()` for the Diff state.

## Appearance and scrolling

Line numbers, syntax highlighting, and file headers are enabled by default:

```rust
Diff::new(&self.diff)
    .line_number(true)
    .syntax_highlight(true)
    .header_visible(true)
    .h(rems(24.))
```

Syntax language is detected from each file's path. Enable the matching Cargo
grammar feature, such as `tree-sitter-rust`, or use `tree-sitter-languages` for
all built-in grammars. Without a grammar, supplied code still renders with line
change colors.

The state prepares syntax on a background thread, file by file, after it is
created or receives new files; code first appears uncolored and gains color
as each file is ready. Each hunk is parsed on its own, so a comment or string
left open at the end of one hunk does not color the next. Injected languages,
such as code blocks inside Markdown, are not highlighted. Lines longer than
1,000 bytes are shown without syntax color. Patch fragments may still lack
syntax context; colors do not establish parse completeness. There is no source
comparison or word-level diff computation in this component.

The code uses the theme's monospace font and syntax theme. Supplied source
whitespace is retained for copying; a patch `\ No newline at end of file` marker
is shown explicitly. Rows are virtualized across all files, including annotation
height. Diff owns both scrolling axes; avoid nesting it in another scrolling
region. Long lines scroll horizontally. Soft wrapping is not supported.

## File headers

The default header shows the path, a rename as `old → new`, the
addition/deletion counts, and whether the file was added or deleted. Use
`header` to replace its content while retaining the header's shell and
separator, and `header_visible(false)` to hide headers:

```rust
Diff::new(&self.diff)
    .header(|file, _, _| {
        div()
            .flex()
            .gap_2()
            .child(file.path().clone())
            .child(format!("+{}", file.additions()))
    })
    .h(rems(24.))
```

The callback receives `&DiffFile`, `&mut Window`, and `&mut App`, and
returns an `IntoElement`. Callbacks implement `Fn` and retain their captures for
`'static`; they receive an `App`, rather than the owning view's `Context`.
Construct the content in the callback and update state from its event handlers.
Do not synchronously read or update the same `DiffState` entity inside a render
callback.

## Source-line annotations

An annotation has a stable application ID and a one-based source position.
Keep comment content and draft state in the application, and render that
content beneath the associated source line:

```rust
let annotation = DiffLineAnnotation::new(
    "review-comment-42",
    DiffLinePosition::new("src/main.rs", DiffSide::Modified, 3),
);

Diff::new(&self.diff)
    .annotations([annotation])
    .annotation_content(|_, _, _| div().child("Check the fallback behavior."))
    .h(rems(24.))
```

Supply both `annotations` and `annotation_content`. The callback receives
`&DiffLineAnnotation`, `&mut Window`, and `&mut App`; use `id()` and
`position()` to find application content. Annotation IDs must be stable and
unique within the Diff. An annotation on hidden context appears when that
source line is revealed. Invalid or missing-side positions are not rendered.
Unified context rows can display annotations from either source side, even
though the code is shown once. Annotation callbacks have the same `Fn` and
`'static` requirements as header callbacks.

Annotation content may change height freely. Visible rows are measured on every
frame, and rows are measured again when annotations are added, moved, removed or
replaced. In Split mode, the paired row stretches to the taller side.
Annotation text is excluded from source copying; buttons and other controls in
annotation content own their actions.

## Pointer and keyboard interaction

Drag over code to select supplied text on one side of one file, including across
virtualized or folded available ranges. Unavailable gaps are not selectable.
Click a line number to select that source line; Shift-click another number on
the same side extends the inclusive range. Text selection and source-line
selection are separate modes. The viewer is focusable through Tab. Focusing the
viewer retains its ordinary border.

These defaults apply while the code body is focused:

| Command | Shortcut |
| --- | --- |
| Scroll vertically / horizontally | Up / Down, Left / Right |
| Scroll one page | PageUp / PageDown |
| Scroll to the start / end | Home / End |
| Extend source-line selection | Shift+Up / Shift+Down |
| Copy source selection | Cmd+C on macOS; Ctrl+C on Windows/Linux |
| Select all supplied source on the active side | Cmd+A on macOS; Ctrl+A on Windows/Linux |

Diff binds no shortcut for change navigation; the application decides whether
`next_change` and `previous_change` get one. Select all uses the selected file and
side; without a selection it uses the file at the top of the viewport and
chooses the modified side when it contains source, otherwise the original side.
Shift+Up and Shift+Down start at the first supplied source line when no
source-line selection exists. Context disclosures are ordinary buttons and can
be reached with Tab. When a control inside a header or annotation has focus,
its own keyboard commands take priority; Diff's source-copy and scrolling
bindings apply only to the code body's focus.

`DiffEvent::SelectionChanged(Option<DiffLineRange>)` reports user source-line
selection changes. It does not report every text-selection gesture or
programmatic state update. Applications can subscribe to the state entity for
review actions that depend on a selected line range.
