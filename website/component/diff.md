---
title: Diff
description: Readonly unified and Git diff display for code review and change previews.
---

# Diff

`Diff` displays an application-supplied unified or Git diff in a readonly
surface. Use it for code review and change previews; use [Editor](./editor.md)
when the user needs to edit source code.

The application obtains the patch from Git, an AI response, or another producer.
`DiffDocument::parse` prepares display data without comparing old and new files.
The owner retains an `Entity<DiffState>` for each file it chooses to display.

## Import

```rust
use gpui_kit::*;
use gpui_kit::component::diff::{
    Diff, DiffDocument, DiffLineAnnotation, DiffLinePosition, DiffLineRange,
    DiffMode, DiffSide, DiffState,
};
```

Initialize components with `gpui_kit::init(cx)` before opening the window through
`gpui_kit::open_window`. See [Getting Started](../docs/getting-started.md).

## Basic usage

Parse the patch and create its state once in the owning view. A patch can
contain multiple files; the application chooses how to present them:

```rust
let patch = "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,3 +1,3 @@\n fn main() {\n-    run();\n+    run_app();\n }\n";
let documents = DiffDocument::parse(patch)?;
let diff = documents.into_iter().next().map(|document| {
    cx.new(|cx| DiffState::new(document, cx))
});
```

`parse` returns `Result<Vec<DiffDocument>, DiffParseError>`. Handle errors in the
application and show an appropriate empty state when the patch contains no files.
The remaining examples assume the owner stores one selected file's state.

Store the selected state as an `Entity<DiffState>` on the owner. Construct only the
presentation during rendering, and give it a bounded viewport:

```rust
Diff::new(&self.diff).h(rems(24.)).w_full()
```

Do not parse the patch or recreate its state on every render. For large patches,
prepare documents on a background executor, then install them only if the result
still belongs to the current patch revision.

## Patch input

Accept unified diffs (`---`, `+++`, `@@`) and Git diff text. Each parsed file
produces one document. `original()` and `modified()` expose readonly `DiffFile`
metadata for the available sides; `/dev/null` represents a missing side.
`additions()` and `deletions()` report changed lines represented by the patch.

A patch normally contains only changed lines and nearby context. Its line numbers
are source coordinates, not proof that the complete source file is available.
Diff neither computes differences nor loads repository files, applies changes,
or resolves merge conflicts. Source outside the patch remains unavailable.

`line_count(side)` counts supplied lines, not the complete file length.
`metadata()` exposes retained Git headers and binary metadata; `is_binary()`
identifies binary changes. Metadata-only changes can have no source rows.
Malformed hunks and unsupported combined diffs return a `DiffParseError`; its
`line()` and `message()` identify the patch location and reason.

## Layout and context

The default layout is `DiffMode::Unified`, with up to three available unchanged lines around
each change. Choose the layout explicitly for the available space:

```rust
self.diff.update(cx, |state, cx| {
    state.set_mode(DiffMode::Split, cx);
    state.set_context_lines(Some(5), cx);
});
```

Split shows the original and modified source side by side, with empty cells
where a line has no counterpart. Unified shows deleted lines before added
lines while retaining original source line numbers.

Hidden unchanged context is represented by a compact disclosure showing the
unchanged-line count. Activating it expands that entire hidden range. To expose all context supplied
by the patch without changing the configured context count, call `expand_all`; call
`collapse_all` to restore that count. Setting the context count to `None`
disables folding of available context:

```rust
self.diff.update(cx, |state, cx| {
    state.expand_all(cx);
    state.collapse_all(cx);
    state.set_context_lines(None, cx);
});
```

## Source coordinates and navigation

`DiffSide::Original` and `DiffSide::Modified` identify the file version.
`DiffLinePosition` and `DiffLineRange` use **one-based source line numbers**,
not visible row indices. A line range includes both endpoints and stays on
one side. Layout changes and context folding do not renumber source lines.

```rust
self.diff.update(cx, |state, cx| {
    state.scroll_to_line(DiffLinePosition::new(DiffSide::Modified, 12), cx);
    state.next_change(cx);
    state.previous_change(cx);
});
```

`scroll_to_line` reveals supplied context when needed; it cannot reveal lines
absent from the patch. Change navigation moves
between changed groups and wraps at the ends. Put layout and navigation
commands in the application's toolbar or menu.

## Selection and copying

Select source lines programmatically using a side and an inclusive range:

```rust
self.diff.update(cx, |state, cx| {
    state.set_selected_lines(Some(DiffLineRange::new(DiffSide::Modified, 3, 8)), cx);
});

let source = self.diff.read(cx).selected_text(cx);

self.diff.update(cx, |state, cx| {
    state.set_selected_lines(None, cx);
});
```

`selected_lines()` reports the current line range. Line selection is limited to
supplied source lines; selecting a missing side clears the selection.
`selected_text(cx)` returns source text without line numbers, change markers,
alignment cells, or annotation content. `DiffDocument::text_for_range(range)`
extracts supplied lines in an inclusive source range without changing the viewer
selection. Copy excludes diff prefixes and unavailable gaps; it cannot reconstruct
the whole file or infer source line endings that the patch does not preserve.

## Replacing the document

```rust
self.diff.update(cx, |state, cx| {
    state.set_document(updated_document, window, cx);
});
```

Replacing the document resets expansion, selection, and scrolling because
their coordinates belong to the previous patch. State mutations
notify observers; callers do not need an additional `cx.notify()` for the
Diff state.

## Appearance and scrolling

Line numbers, syntax highlighting, and the file header are enabled by default:

```rust
Diff::new(&self.diff)
    .line_numbers(true)
    .syntax_highlight(true)
    .header(true)
    .h(rems(24.))
```

Syntax language is detected from the filename. Enable the matching Cargo grammar
feature, such as `tree-sitter-rust`, or use `tree-sitter-languages` for all built-in
grammars. Without a grammar, supplied code still renders with line change colors.
Syntax highlighting is skipped for lines longer than 1,000 bytes. Patch fragments
may have incomplete syntax context; colors do not establish parse completeness.
Syntax preparation budgets host-language parsing; syntax queries and injected
language parsing can take additional time. There is no source comparison or
word-level diff computation in this component.

The code uses the theme's monospace font and syntax theme. Supplied source whitespace is retained for copying; a patch
`\ No newline at end of file` marker is shown explicitly. Rows are virtualized, including annotation height. Diff owns both
scrolling axes; avoid nesting it in another scrolling region. Long lines
scroll horizontally. Soft wrapping is not supported.

## File header slots

The default header shows the filename and addition/deletion counts. Four
render callbacks support application content:

| Builder | Placement |
| --- | --- |
| `render_header` | Replaces header content, retaining its shell and separator |
| `render_header_prefix` | Before the filename in the default header |
| `render_header_filename_suffix` | Immediately after the filename in the default header |
| `render_header_metadata` | After the statistics in the default header |

Each callback receives `&DiffDocument`, `&mut Window`, and `&mut App`, and
returns an `IntoElement`. A custom `render_header` replaces the default content
and its three insertion slots. `header(false)` hides the entire header.
Callbacks implement `Fn` and retain their captures for `'static`; they receive
an `App`, rather than the owning view's `Context`. Construct the content in the
callback and update state from its event handlers. Do not synchronously read or
update the same `DiffState` entity inside a render callback.

```rust
Diff::new(&self.diff)
    .render_header_metadata(|_, _, _| div().child("Working tree"))
    .h(rems(24.))
```

## Source-line annotations

An annotation has a stable application ID and a one-based source position.
Keep comment content and draft state in the application, and render that
content beneath the associated source line:

```rust
let annotation = DiffLineAnnotation::new(
    "review-comment-42",
    DiffLinePosition::new(DiffSide::Modified, 3),
);

Diff::new(&self.diff)
    .with_annotations([annotation])
    .render_annotation(|_, _, _| div().child("Check the fallback behavior."))
    .with_layout_revision(self.annotation_revision)
    .h(rems(24.))
```

Supply both `with_annotations` and `render_annotation`. The callback receives
`&DiffLineAnnotation`, `&mut Window`, and `&mut App`; use `id()` and
`position()` to find application content. Annotation IDs must be stable and
unique within the Diff. An annotation on hidden context appears when that
source line is revealed. Invalid or missing-side positions are not rendered.
Unified context rows can display annotations from either source side, even
though the code is shown once. Annotation callbacks have the same `Fn` and
`'static` requirements as header callbacks.

Keep `annotation_revision` as a `u64` counter on the owning view. Increment the
value passed to `with_layout_revision` when annotation content
changes height or when adding, moving, or removing annotations affects layout.
The application owns this revision; changes to captured content do not
automatically invalidate cached virtual row measurements. The viewer handles
invalidation for font, zoom, layout mode, line-number visibility,
and document changes. If an annotation's own layout depends on other
application settings, update the revision when those settings affect its
height. In split layout, the paired
row stretches to the taller side. Annotation text is excluded from source
copying; buttons and other controls in annotation content own their actions.

## Pointer and keyboard interaction

Drag over code to select supplied text on one file side, including across
virtualized or folded available ranges. Unavailable gaps are not selectable. Click a line number to select that source line;
Shift-click another number on the same side extends the inclusive range.
Text selection and source-line selection are separate modes. The viewer is
focusable through Tab. Focusing the viewer retains its ordinary border.

These defaults apply while the code body is focused:

| Command | Shortcut |
| --- | --- |
| Scroll vertically / horizontally | Up / Down, Left / Right |
| Scroll one page | PageUp / PageDown |
| Scroll to the start / end | Home / End |
| Next / previous changed group | F7 / Shift+F7 |
| Extend source-line selection | Shift+Up / Shift+Down |
| Copy source selection | Cmd+C on macOS; Ctrl+C on Windows/Linux |
| Select all supplied source on the active side | Cmd+A on macOS; Ctrl+A on Windows/Linux |

Select all uses the selected file side; without a selection it chooses the
modified side when it contains source, otherwise the original side. Shift+Up
and Shift+Down start at the first supplied source line when no source-line selection exists. Context
disclosures are ordinary buttons and can be reached with Tab.
When a control inside a header or annotation has focus, its own keyboard
commands take priority; Diff's source-copy and scrolling bindings apply only
to the code body's focus.

`DiffEvent::SelectionChanged(Option<DiffLineRange>)` reports user source-line
selection changes. It does not report every text-selection gesture or
programmatic state update. Applications can subscribe to the state entity for
review actions that depend on a selected line range.
