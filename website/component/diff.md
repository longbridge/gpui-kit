---
title: Diff
description: Readonly unified and Git diff display for code review, merge conflicts and source previews.
---

# Diff

`Diff` displays an application-supplied unified or Git diff in a readonly
surface. Use it for code review and change previews; use [Editor](./editor.md)
when the user needs to edit source code.

The application obtains the patch from Git, an AI response, or another producer.
`DiffFile::parse` prepares one `DiffFile` per changed file without comparing
old and new versions. One `DiffState` shows all of those files in a single
virtualized list, each introduced by its own header. The same list can show a
whole source file for reference, or a working file with merge conflicts.

## Import

```rust
use gpui_kit::*;
use gpui_kit::component::diff::{
    Diff, DiffAnnotation, DiffChangeIndicator, DiffConflictResolution, DiffFile, DiffHoverHighlight,
    DiffHunkSeparator, DiffInlineUnit, DiffLinePosition, DiffLineRange, DiffMode,
    DiffSide, DiffState,
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
patches, parse on a background executor, then install the files only if the
result still belongs to the current patch revision.

## Git history example

For a complete application, see the [tig example](https://github.com/longbridge/gpui-kit/tree/main/examples/tig).
It reads local Git history and patches on a background executor, uses virtualized
commit/file navigation in a resizable sidebar, and displays the selected file
with one retained `DiffState`. Stale requests cannot overwrite a newer commit.
The example includes keyboard navigation, source copying, runtime display
options, loading/error/empty states and a collapsible commit message.

```sh
cargo run -p example-tig -- /path/to/repository
```

## Files

Accept unified diffs (`---`, `+++`, `@@`) and Git diff text, including
`git format-patch` output and patches written with `diff.noprefix` or
`diff.mnemonicPrefix`. Each parsed file produces one `DiffFile`. `path()` is the
path to show and identifies the file; `original_path()` and `modified_path()`
return `None` for the missing side of an added or deleted file (`/dev/null`).
`status()` reports `Added`, `Deleted`, `Modified`, `Renamed` or `Copied` in Git's
terms, and `additions()` and `deletions()` report changed lines in the patch.

A patch normally contains only changed lines and nearby context. Its line numbers
are source coordinates, not proof that the complete source file is available.
Diff neither computes differences nor loads repository files or applies
changes. Source outside the patch remains unavailable.

`extended_headers()` exposes Git's extended header lines, such as modes,
similarity, renames and the binary marker; `is_binary()` identifies binary
changes. A file without source rows, such as a binary file, a mode change or a
pure rename, shows a summary below its header. Malformed hunks and unsupported
combined diffs return a `DiffParseError`; its `line()` and `message()` identify
the patch location and reason.

Two constructors cover files that are not part of a patch. Both show one column
in every display mode:

```rust
// A whole file, for reference, with no changes and no folding.
let source = DiffFile::unchanged("src/retry.rs", &text);
// A working file with Git conflict markers, including diff3 bases.
let merge = DiffFile::parse_conflicts("src/retry.rs", &working_text)?;
```

## Merge conflicts

A conflicted file keeps the line numbers of the working file. Each conflict shows
its current, base and incoming parts under headings; the current heading offers
**Accept Current**, **Accept Incoming** and **Accept Both**, and a resolved
conflict shows only the kept lines with **Undo**. Diff records the choice; the
application writes the result:

```rust
let state = self.diff.read(cx);
if let Some(text) = state.resolved_text("src/retry.rs") {
    // Every conflict has a resolution; save `text`.
}
```

`resolved_text` returns `None` while any conflict is unresolved.
`conflict_resolution(path, ix)` reads one conflict's resolution, and
`resolve_conflict(path, ix, resolution, cx)` sets or clears it without emitting
an event. A user's choice emits `DiffEvent::ConflictResolved(path, ix)`.

`DiffFile::conflicts()` returns readonly `DiffConflict` descriptors in working-file
order. Their slice indices are the `ix` used by resolution methods and events;
`conflicts().len()` gives the count. `lines()` includes the marker lines;
`current_lines()`, `base_lines()` and `incoming_lines()` locate each source part.
These ranges use one-based working-file coordinates with an **exclusive end**,
so an empty part is represented by `start == end`. `base_lines()` and
`base_label()` return `None` for a two-way conflict. The labels are available
through `current_label()`, `base_label()` and `incoming_label()`.

For example, an application command can accept every incoming change without
parsing the working file again:

```rust
self.diff.update(cx, |state, cx| {
    let path = "src/retry.rs";
    let count = state.files().iter().find(|file| file.path().as_str() == path)
        .map_or(0, |file| file.conflicts().len());
    for ix in 0..count {
        state.resolve_conflict(path, ix, Some(DiffConflictResolution::Incoming), cx);
    }
});
```

`conflict_resolution` returns `None` for an unresolved conflict or an unknown
file/index; enumerate `conflicts()` to distinguish valid unresolved conflicts.

## Display mode and context

The default display mode is `DiffMode::Unified`, with up to three available
unchanged lines around each change. Set the initial mode when creating the
state, and change it later from application commands:

```rust
let diff = cx.new(|cx| {
    DiffState::new(files, cx)
        .with_mode(DiffMode::Split)
        .with_context_lines(Some(5))
});

self.diff.update(cx, |state, cx| state.set_mode(DiffMode::Unified, cx));
```

Split shows the original and modified source side by side, with empty cells
where a line has no counterpart. Unified shows deleted lines before added lines
while retaining original source line numbers.

Hidden unchanged context is represented by a compact disclosure showing the
unchanged-line count. Its buttons reveal lines just below the source above,
lines just above the source below, or the whole range. `with_expansion_lines(n)`
sets how many lines each partial step reveals (20 by default), and
`with_min_collapsed_lines(n)` keeps unchanged ranges shorter than `n` visible
(2 by default). `expand_unchanged` reveals all supplied context without changing
the configured context count, `collapse_unchanged` restores it, and setting the
context count to `None` disables folding:

```rust
self.diff.update(cx, |state, cx| {
    state.expand_unchanged(cx);
    state.collapse_unchanged(cx);
    state.set_context_lines(None, cx);
});
```

## Source coordinates and navigation

`DiffSide::Original` and `DiffSide::Modified` identify the file version.
`DiffLinePosition` and `DiffLineRange` identify a file by its `path()`, which
stays stable when a newer revision of the patch reorders its files, and use
**one-based source line numbers**, not visible row indices. Display mode changes
and context folding do not renumber source lines.

```rust
self.diff.update(cx, |state, cx| {
    state.scroll_to_line(DiffLinePosition::new("src/main.rs", DiffSide::Modified, 12), cx);
    state.scroll_to_file("src/main.rs", cx);
    state.next_change(cx);
    state.previous_change(cx);
});
```

`scroll_to_line` reveals supplied context and expands a collapsed file when
needed; it cannot reveal lines absent from the patch. `scroll_to_file` shows a
file's header, including a file without source rows, for example from a file
list. Change navigation moves between changed groups across all files and wraps
at the ends. Put display mode and navigation commands in the application's
toolbar or menu.

Each file header has a collapse control. `set_file_collapsed(path, collapsed, cx)`
and `is_file_collapsed(path)` drive it from the application, for example to
collapse files the reviewer has marked as viewed. A user's toggle emits
`DiffEvent::FileCollapsed(path)` or `DiffEvent::FileExpanded(path)`. Collapsed
files stay collapsed when `set_files` installs a newer revision with the same
path.

## Selection and copying

Select source lines programmatically using a path, a side and an inclusive
range:

```rust
self.diff.update(cx, |state, cx| {
    state.set_selected_lines(Some(DiffLineRange::new("src/main.rs", DiffSide::Modified, 3, 8)), cx);
});

let source = self.diff.read(cx).selected_text(cx);
```

A range normally stays on one side. `with_end_side` ends it on the other side,
covering the patch body between the two lines in patch order, as Unified mode
shows it:

```rust
let range = DiffLineRange::new("src/main.rs", DiffSide::Original, 10, 12)
    .with_end_side(DiffSide::Modified);
```

`selected_lines()` reports the current range, clipped to supplied source lines;
a range containing none clears the selection. `selected_text(cx)` returns source
text without line numbers, change markers, alignment cells or annotation
content. Copy excludes diff prefixes and unavailable gaps; it cannot reconstruct
the whole file or infer source line endings that the patch does not preserve.

## Replacing the files

```rust
self.diff.update(cx, |state, cx| {
    state.set_files(updated_files, cx);
});
```

Replacing the files resets expansion, selection, conflict resolutions and
scrolling because their coordinates belong to the previous patch. State
mutations notify observers; callers do not need an additional `cx.notify()` for
the Diff state.

## Appearance

Line numbers, syntax highlighting and file headers are enabled by default:

```rust
Diff::new(&self.diff)
    .line_number(true)
    .syntax_highlight(true)
    .header_visible(true)
    .hunk_separator(DiffHunkSeparator::Metadata)
    .change_indicator(DiffChangeIndicator::Signs)
    .change_background(true)
    .hover_highlight(DiffHoverHighlight::None)
    .soft_wrap(false)
    .h(rems(24.))
```

- `hunk_separator` marks each hunk with its `@@` header (`Metadata`), the count
  of lines the patch does not supply (`LineInfo`), or a thin divider (`Simple`).
- `change_indicator` marks changed lines with `+` and `−` signs (`Signs`), a
  colored bar (`Bars`) or nothing (`None`); `change_background(false)` removes
  the line tint while keeping inline emphasis.
- `hover_highlight` emphasizes the line, its number, or both under the pointer.
- `soft_wrap(true)` wraps long lines at the column width, and rows grow instead
  of scrolling horizontally.

Syntax language is detected from each file's path; `DiffFile::with_language`
overrides it with a highlighter language name such as `rust`. Enable the
matching Cargo grammar feature, such as `tree-sitter-rust`, or use
`tree-sitter-languages` for all built-in grammars. Without a grammar, supplied
code still renders with line change colors.

Changed lines paired across the two sides also emphasize the words that differ.
`with_inline_unit(Some(DiffInlineUnit::Character))` compares characters instead,
and `None` emphasizes whole lines only. Paired lines with little in common get
no inline emphasis, because marking nearly every word adds noise.

The state prepares syntax and inline emphasis on a background thread, file by
file, after it is created or receives new files; code first appears uncolored and
gains color as each file is ready. Each hunk is parsed on its own, so a comment
or string left open at the end of one hunk does not color the next. Injected
languages, such as code blocks inside Markdown, are not highlighted. Lines longer
than 1,000 bytes are shown without syntax color or inline emphasis; change the
limits with `with_syntax_max_line_length` and `with_inline_max_line_length`.

The code uses the theme's monospace font and syntax theme. Supplied source
whitespace is retained for copying; a patch `\ No newline at end of file` marker
is shown explicitly. Rows are virtualized across all files, including annotation
height. Diff owns its scrolling; avoid nesting it in another scrolling region.

The `with_*` state builders configure initial values. To change them on the
retained entity, use `set_expansion_lines`, `set_min_collapsed_lines`,
`set_inline_unit`, `set_inline_max_line_length` and `set_syntax_max_line_length`.
These methods notify observers and preserve selection and the nearby source
position; emphasis changes discard stale preparation and prepare it again.

```rust
self.diff.update(cx, |state, cx| {
    state.set_inline_unit(Some(DiffInlineUnit::Character), cx);
});
```

## File headers

The default header shows the collapse control, the path, a rename as
`old → new`, the addition and deletion counts, and the file status. Three slots
add application content to it, and `render_header` replaces its content while keeping
the shell, separator and collapse control:

| Builder | Placement |
| --- | --- |
| `render_header_prefix` | Before the path |
| `render_header_title_suffix` | Immediately after the path |
| `render_header_suffix` | After the counts and status |
| `render_header` | Replaces the content |

```rust
Diff::new(&self.diff)
    .render_header_suffix(|file, _, _| div().child(format!("{} lines", file.additions())))
    .h(rems(24.))
```

Each callback receives `&DiffFile`, `&mut Window` and `&mut App`, and returns an
`IntoElement`. `header_visible(false)` hides headers. Callbacks implement `Fn`
and retain their captures for `'static`; they receive an `App`, rather than the
owning view's `Context`. Construct the content in the callback and update state
from its event handlers. Do not synchronously read or update the same
`DiffState` entity inside a render callback.

The earlier names `header`, `header_prefix`, `header_title_suffix`,
`header_suffix` and `annotation_content` remain available as compatibility
aliases for the corresponding `render_*` builders.

## Annotations

An annotation has a stable application ID and a target: a source line, or a
whole file. Keep comment content and draft state in the application, and render
that content beneath the line or below the file header:

```rust
let annotations = [
    DiffAnnotation::line(
        "review-comment-42",
        DiffLinePosition::new("src/main.rs", DiffSide::Modified, 3),
    ),
    DiffAnnotation::file("review-summary", "src/main.rs"),
];

Diff::new(&self.diff)
    .annotations(annotations)
    .render_annotation(|annotation, _, _| div().child(annotation.id().to_string()))
    .h(rems(24.))
```

Supply both `annotations` and `render_annotation`. The callback receives
`&DiffAnnotation`; use `id()`, `path()` and `position()`, which is `None` for a
file annotation, to find application content. Annotation IDs must be stable and
unique within the Diff. An annotation on hidden context appears when that source
line is revealed. Invalid or missing-side positions are not rendered. Unified
context rows can display annotations from either source side, even though the
code is shown once.

Annotation content may change height freely. Visible rows are measured on every
frame, and rows are measured again when annotations are added, moved, removed or
replaced. In Split mode, the paired row stretches to the taller side. Annotation
text is excluded from source copying; controls in annotation content own their
actions.

To let reviewers start a comment, handle `on_add_annotation`. An add button then
appears beside the hovered line and at the end of the selected lines; the handler
receives the selected range when the line is part of it, or that line alone:

```rust
Diff::new(&self.diff).on_add_annotation(cx.listener(|this, range: &DiffLineRange, _, cx| {
    this.start_comment(range.clone(), cx);
}))
```

## Pointer and keyboard interaction

Wheel axes remain independent: vertical input scrolls the source rows;
horizontal input (including a platform's Shift+wheel gesture) scrolls long
lines without moving the vertical position. Soft wrapping removes horizontal
overflow.

Drag over code to select supplied text on one side of one file, including across
virtualized or folded available ranges. Unavailable gaps are not selectable.
Click a line number to select that source line, drag across line numbers to
select a range, or Shift-click another number to extend the inclusive range. In
Unified mode a range may cross from deleted to added lines; in Split mode
changing side starts a new selection. Text selection and source-line selection
are separate modes. The viewer is focusable through Tab. Focusing the viewer
retains its ordinary border.

`on_line_click` reports a click on a line's code that is not a text-selection
drag, with the line's position and the click event. `on_line_hover` reports the
line under the pointer, or `None` when it leaves.

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
`next_change` and `previous_change` get one. Select all uses the selected file
and side; without a selection it uses the file at the top of the viewport and
chooses the modified side when it contains source, otherwise the original side.
Shift+Up and Shift+Down start at the first supplied source line when no
source-line selection exists. Context disclosures are ordinary buttons and can
be reached with Tab. When a control inside a header or annotation has focus, its
own keyboard commands take priority; Diff's source-copy and scrolling bindings
apply only to the code body's focus.

## Events

Subscribe to the state entity for review actions:

| Event | When |
| --- | --- |
| `SelectionStarted(range)` | A line-number press starts a selection |
| `SelectionChanged(range)` | A user gesture changes the selected lines, or clears them with `None` |
| `SelectionEnded(range)` | A selection gesture finishes: a pointer release, a keyboard step or Select All |
| `FileCollapsed(path)` / `FileExpanded(path)` | The user toggles a file from its header |
| `ConflictResolved(path, ix)` | The user resolves a conflict or undoes it |

Programmatic changes such as `set_selected_lines`, `set_file_collapsed` and
`resolve_conflict` do not emit events. `DiffEvent` is non-exhaustive, so match
it with a wildcard arm.
