# Diff public API

This internal inventory records the public API implemented in
`crates/component/src/diff/`. Import these items through
`gpui_kit::component::diff`. See [Diff component design](DIFF-COMPONENT-DESIGN.md)
for the reference research, intended interaction contract and scope.
The declaration excerpts omit private fields and method bodies.

## Reference API mapping

[Pierre Diffs](https://diffs.com/docs) is the reference for the division between
file data, retained comparison state and presentation. The following mappings
describe corresponding responsibilities; they do not promise source compatibility
or feature parity.

| Pierre Diffs concept | GPUI Kit API | Responsibility |
| --- | --- | --- |
| Complete file versions / `FileContents` | `DiffFile` | Retain a file name, exact source and optional syntax language. |
| `parseDiffFromFile` / comparison metadata | `DiffDocument` | Compute and retain one immutable comparison for reuse. |
| Split/unified `diffStyle` | `DiffMode`, `DiffState::set_mode` | Choose presentation without changing source coordinates. |
| Context visibility and expansion | `DiffState::set_context_lines`, `expand_all`, `collapse_all` | Retain disclosures independently of the immutable comparison. |
| Selected source lines | `DiffSide`, `DiffLinePosition`, `DiffLineRange`, `DiffEvent` | Locate selection by source side and one-based source lines. |
| Imperative source-line navigation | `DiffState::scroll_to_line`, `next_change`, `previous_change` | Reveal context and navigate within the retained viewport. |
| `FileDiff` presentation | `Diff`, `Entity<DiffState>` | Build a themed component while retaining interaction state in the owner. |
| File-header render slots | `Diff::render_header`, `render_header_prefix`, `render_header_filename_suffix`, `render_header_metadata` | Compose application content in the header. |
| Source-line annotations | `DiffLineAnnotation`, `Diff::with_annotations`, `render_annotation` | Attach application-owned content to a source position with a stable ID. |

Complete file versions are the supported input. A missing side is represented by
`DiffDocument::added` or `deleted`; an existing empty `DiffFile` remains an
existing side. Patch parsing is not part of this API.

## Source data

The public structs below expose private fields through constructors and readers.
Documents are immutable and cheaply cloned; prepare a new document when the
application's source revision changes, then install it with `set_document`.

```rust
pub struct DiffFile;
impl DiffFile {
    pub fn new(name: impl Into<SharedString>, text: impl Into<SharedString>) -> Self;
    pub fn with_language(self, language: impl Into<SharedString>) -> Self;
    pub fn name(&self) -> &SharedString;
    pub fn text(&self) -> &SharedString;
    pub fn language(&self) -> Option<&SharedString>;
}
```

`DiffFile` retains exact source, including whitespace and line endings.
`with_language` overrides language detection from the file name.

```rust
pub enum DiffSide { Original, Modified }

pub struct DiffLinePosition;
impl DiffLinePosition {
    pub fn new(side: DiffSide, line: usize) -> Self;
    pub fn side(&self) -> DiffSide;
    pub fn line(&self) -> usize;
}

pub struct DiffLineRange;
impl DiffLineRange {
    pub fn new(side: DiffSide, start: usize, end: usize) -> Self;
    pub fn side(&self) -> DiffSide;
    pub fn start(&self) -> usize;
    pub fn end(&self) -> usize;
}
```

Positions use one-based source lines. Ranges are inclusive, normalize reversed
endpoints and clamp line numbers to at least one. They do not refer to display
rows, so changing layout does not change their meaning.

```rust
pub struct DiffDocument;
impl DiffDocument {
    pub fn new(original: DiffFile, modified: DiffFile) -> Self;
    pub fn added(modified: DiffFile) -> Self;
    pub fn deleted(original: DiffFile) -> Self;
    pub fn original(&self) -> Option<&DiffFile>;
    pub fn modified(&self) -> Option<&DiffFile>;
    pub fn additions(&self) -> usize;
    pub fn deletions(&self) -> usize;
    pub fn has_changes(&self) -> bool;
    pub fn line_count(&self, side: DiffSide) -> usize;
    pub fn text_for_range(&self, range: DiffLineRange) -> String;
}
```

The constructors compare complete source versions synchronously. Applications
should compute large comparisons on a background executor and check that the
source revision is still current before installing the result.
`text_for_range` copies exact source lines, clips to available lines and returns
an empty string for a range after the file. The addition/deletion counts describe
changed source lines; an added or deleted empty file still has changes.

## Retained state and events

```rust
pub enum DiffEvent {
    SelectionChanged(Option<DiffLineRange>),
}

pub struct DiffState;
impl EventEmitter<DiffEvent> for DiffState {}
impl Focusable for DiffState {}

impl DiffState {
    pub fn new(document: DiffDocument, cx: &mut Context<Self>) -> Self;
    pub fn document(&self) -> &DiffDocument;
    pub fn mode(&self) -> DiffMode;
    pub fn context_lines(&self) -> Option<usize>;
    pub fn selected_lines(&self) -> Option<DiffLineRange>;
    pub fn set_document(&mut self, document: DiffDocument, window: &mut Window, cx: &mut Context<Self>);
    pub fn set_mode(&mut self, mode: DiffMode, cx: &mut Context<Self>);
    pub fn set_context_lines(&mut self, lines: Option<usize>, cx: &mut Context<Self>);
    pub fn expand_all(&mut self, cx: &mut Context<Self>);
    pub fn collapse_all(&mut self, cx: &mut Context<Self>);
    pub fn set_selected_lines(&mut self, range: Option<DiffLineRange>, cx: &mut Context<Self>);
    pub fn selected_text(&self, cx: &App) -> String;
    pub fn scroll_to_line(&mut self, position: DiffLinePosition, cx: &mut Context<Self>);
    pub fn next_change(&mut self, cx: &mut Context<Self>);
    pub fn previous_change(&mut self, cx: &mut Context<Self>);
}
```

Create one `Entity<DiffState>` in the owning view and reuse it across renders.
The default is unified mode with three unchanged lines around changes.
`set_context_lines(None)` shows all source; expanding all temporarily reveals
context without changing that configured count. Collapsing restores the count.

Changing mode retains line-range selection and a nearby source row. Replacing
the document resets selection, context expansion and viewport. Programmatic
line selection clamps to existing source lines; selecting a missing or empty
side clears it. `selected_text` omits gutters, diff signs, alignment blanks and
annotation content, and preserves source whitespace and line endings.

`scroll_to_line` expands hidden context when necessary and ignores positions
past the end of the requested side. Change navigation moves between changed
groups and wraps at the first/last group. `DiffEvent::SelectionChanged` reports
source-line selection changes; arbitrary text selection does not become a
source-line range.

## Presentation and content slots

```rust
pub enum DiffMode { Split, Unified }

pub struct DiffLineAnnotation;
impl DiffLineAnnotation {
    pub fn new(id: impl Into<ElementId>, position: DiffLinePosition) -> Self;
    pub fn id(&self) -> &ElementId;
    pub fn position(&self) -> DiffLinePosition;
}

pub struct Diff;
impl Diff {
    pub fn new(state: &Entity<DiffState>) -> Self;
    pub fn line_numbers(self, value: bool) -> Self;
    pub fn inline_highlight(self, value: bool) -> Self;
    pub fn syntax_highlight(self, value: bool) -> Self;
    pub fn header(self, value: bool) -> Self;
    pub fn with_annotations(self, annotations: impl IntoIterator<Item = DiffLineAnnotation>) -> Self;
    pub fn with_layout_revision(self, revision: u64) -> Self;
    pub fn render_header<F, E>(self, render: F) -> Self
    where F: Fn(&DiffDocument, &mut Window, &mut App) -> E + 'static, E: IntoElement;
    pub fn render_header_prefix<F, E>(self, render: F) -> Self
    where F: Fn(&DiffDocument, &mut Window, &mut App) -> E + 'static, E: IntoElement;
    pub fn render_header_filename_suffix<F, E>(self, render: F) -> Self
    where F: Fn(&DiffDocument, &mut Window, &mut App) -> E + 'static, E: IntoElement;
    pub fn render_header_metadata<F, E>(self, render: F) -> Self
    where F: Fn(&DiffDocument, &mut Window, &mut App) -> E + 'static, E: IntoElement;
    pub fn render_annotation<F, E>(self, render: F) -> Self
    where F: Fn(&DiffLineAnnotation, &mut Window, &mut App) -> E + 'static, E: IntoElement;
}
impl Styled for Diff {}
impl RenderOnce for Diff {}
impl IntoElement for Diff {}
```

`DiffMode` defaults to `Unified`. The component's four boolean presentation
options default to `true`. `Diff` owns vertical and horizontal scrolling and
needs bounded height. Long lines scroll horizontally; wrapping is not exposed.

`render_header` replaces header content while retaining the header shell and
separator. The three smaller slots augment the default header: prefix before
the file name, suffix after it and metadata after change statistics. They are
used when no full-header renderer is supplied. `header(false)` hides the header
and its slots.

Annotations have application-owned stable identities and render beneath their
source line. An annotation for a hidden line becomes visible when that context
is expanded. The application owns content, draft state and review commands;
`render_annotation` supplies its presentation. Update `with_layout_revision`
when application-owned content changes height so virtual row measurements are
invalidated. This revision is supplied during rendering and does not replace
the document or state.

## Keyboard actions

The module exposes these action types for application menus and keybindings:

```rust
pub use crate::input::{Copy, SelectAll};
// Action types generated in the diff module:
ScrollUp, ScrollDown, ScrollLeft, ScrollRight,
PageUp, PageDown, First, Last,
NextChange, PreviousChange, ExtendUp, ExtendDown
```

Component initialization registers bindings in the `Diff` key context:
arrow keys scroll; Page Up/Down scroll by a page; Home/End reach the first/last
row; F7/Shift-F7 navigate changes; Shift-Up/Down extend source-line selection;
the platform's standard Copy/Select All shortcuts use the shared input actions.
`DiffState` supplies the focus handle through `Focusable`.

## Story Gallery (`gpui-component-story`)

```rust
pub struct DiffStory;
impl DiffStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self>;
}
impl Story for DiffStory {}
impl Render for DiffStory {}
```

`stories::DiffStory` provides the registered, restorable comparison showcase.
`view` constructs its retained state and application-owned review annotation.
