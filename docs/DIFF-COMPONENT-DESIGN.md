# Diff component design

Status: implemented and reviewed. The evidence and remaining platform checks
are recorded in [Diff review](DIFF-REVIEW.md); compilation and local screenshots
do not certify every native interaction or supported platform.

The requested component is a readonly surface for code review and change
previews. The primary reference is [Diffs, from Pierre](https://diffs.com/),
selected by the user. This document records the reference concepts and the GPUI Kit implementation
boundary. The mapping is a design reference, not a claim of API parity.

## Reference implementation

Read the [Pierre documentation](https://diffs.com/docs) together with its
[plain-text reference](https://diffs.com/llms-full.txt). The relevant source is
in [pierrecomputer/pierre](https://github.com/pierrecomputer/pierre/tree/main/packages/diffs).
Source links below are pinned to `89f5913e579a7a686623c653302585ae1c987c53`,
the revision inspected on 2026-10-06.

| Reference | What to inspect | Implication for GPUI Kit |
| --- | --- | --- |
| [Core types](https://github.com/pierrecomputer/pierre/blob/89f5913e579a7a686623c653302585ae1c987c53/packages/diffs/src/types.ts) | File versions, hunks, original line coordinates, split and unified positions | Keep source coordinates separate from display rows; carry the old/new side in line positions. |
| [File comparison](https://github.com/pierrecomputer/pierre/blob/89f5913e579a7a686623c653302585ae1c987c53/packages/diffs/src/utils/parseDiffFromFile.ts) | Converts two versions into reusable diff metadata | Compute the diff outside rendering; let the application retain the result. |
| [Row traversal](https://github.com/pierrecomputer/pierre/blob/89f5913e579a7a686623c653302585ae1c987c53/packages/diffs/src/utils/iterateOverDiff.ts) | Corresponding old/new lines, missing-side rows, visible windows | One alignment model must drive both rendering and navigation. |
| [Context layout](https://github.com/pierrecomputer/pierre/blob/89f5913e579a7a686623c653302585ae1c987c53/packages/diffs/src/utils/virtualDiffLayout.ts) | Leading and trailing hidden context, expansion from either boundary | Folding belongs in retained state and must preserve source line numbers. |
| [FileDiff](https://github.com/pierrecomputer/pierre/blob/89f5913e579a7a686623c653302585ae1c987c53/packages/diffs/src/components/FileDiff.ts) | Lifecycle of a reusable file comparison | Keep parsing, retained interaction state, and themed rendering distinct. |

Pierre accepts complete file versions and parsed patches. A deliberately
missing file side differs from an existing empty file. Its API provides
split/unified layout, inline change highlighting, context expansion, header
slots, and annotations addressed by file side and source line. These are useful
component contracts to study; the Rust implementation adapts them to GPUI.

Two supporting references answer native interaction questions:

- [Zed's split-diff implementation explanation](https://zed.dev/blog/split-diffs)
  describes padding the shorter side and keeping corresponding lines aligned
  while both sides scroll together. It is directly relevant to GPUI layout.
- [Monaco Diff Editor options](https://microsoft.github.io/monaco-editor/typedoc/interfaces/editor_editor_api.editor.IDiffEditorOptions.html)
  provide a checklist for layout constraints, computation limits and accessible
  viewing. Its editable-document lifecycle is outside this component's
  confirmed readonly use case.

## Task and interaction promise

The reader needs to identify what changed, inspect the surrounding code, and
copy the relevant source. The surface should work in an application workspace
and in a bounded change-preview panel.

Use a quiet file header, stable line-number columns, a code body and explicit
context disclosures. Changes have both a `+`/`−` indicator and a themed line
background. Changed words receive a stronger local highlight. A split view
keeps unchanged lines aligned and reserves blank cells for additions or
deletions that have no counterpart.

The file body owns its scrolling. The application owns the toolbar and any
review, accept/reject, stage/unstage or navigation commands. Header and
annotation slots can host application components without making these domain
operations intrinsic to Diff.

## Component boundary and retained lifecycle

The component lives in `crates/component/src/diff/` and is exported through
`gpui_kit::component::diff`. It uses the existing public Base selection and
scroll seams without introducing a Base-layer change. The public inventory is
in [Diff public API](DIFF-PUBLIC-API.md); the published usage guide is
[Diff](../website/component/diff.md).

| Unit | Implemented responsibility |
| --- | --- |
| `DiffFile` | Existing complete file version: name, exact `SharedString` source and optional syntax language override. |
| `DiffDocument` | Immutable, `Arc`-backed comparison prepared before rendering, with retained source versions, aligned source lines and optional inline word ranges. |
| `DiffState` | Retained focus, vertical and horizontal viewport, expanded context, source-line selection and the Base text-selection participant. |
| `Diff` | `RenderOnce` presentation of `Entity<DiffState>`, using Component themes and application content slots. |
| `DiffMode` | Explicit `Split` or `Unified` presentation, chosen through retained state. |
| `DiffSide`, `DiffLinePosition`, `DiffLineRange` | Original/modified side and one-based source lines; an inclusive line range belongs to one side. |
| `DiffEvent` | Notifications about source-line selection; character-level text selection has a separate contract. |
| `DiffLineAnnotation` | Stable application `ElementId` and a source-line position; application content stays in the renderer callback. |

Public source data has private fields and explicit readers. `SourceLine`,
alignment pairs, display rows and byte mappings are internal details, not
application inputs. Source text is retained; visible rows do not reconstruct
the whole file each frame.

Prepare `DiffDocument` when source versions change, retain one
`Entity<DiffState>` in the owner, and construct `Diff` during render.
Document creation is synchronous. Applications should prepare large documents
on the background executor and check source identity/revision before installing
a result. The line comparison has a computation deadline; exceeding it can
coarsen grouping without dropping source. Inline emphasis is optional and
bounded separately, so large replacements can fall back to line highlighting.
Syntax highlighting uses Component's highlighter and theme.

`DiffDocument::new` compares two existing files. `added` and `deleted` express
a deliberately missing side; an existing empty `DiffFile` remains an existing
file. The input contract is complete source versions. This component does not
parse Git patches, edit source, apply changes or resolve merge conflicts.
Multiple file comparisons are composed by the application.

Source and display coordinates remain distinct. Both layouts use the same
alignment pairs. Mode changes preserve source-line selection and a nearby
source row; replacing the document resets selection, context expansion and
scroll positions because they belong to the previous source versions.

## Presentation and interaction contract

The default is unified mode with three unchanged lines around each change.
The application explicitly chooses split for side-by-side inspection; panel
width does not silently override that choice. Split reserves empty cells for
unpaired lines. Unified emits deleted lines before added lines within each
change group while retaining source line numbers.

Changed lines have signs and semantic themed backgrounds; bounded inline word
ranges receive stronger emphasis. Source comparison preserves whitespace,
blank lines, LF/CRLF and missing final newlines. Tabs are expanded only for
display, with a mapping back to exact source byte offsets for selection and
copying. Line-ending explanations must make visually identical changed lines
understandable.

Context is retained separately from the document. Each disclosure expands its
whole hidden unchanged region. `expand_all` exposes all context without
changing the configured context count; `collapse_all` restores disclosures.
`set_context_lines(None)` disables folding. Directional or incremental context
expansion from either edge, as in Pierre, is not currently part of the API.

The code body virtualizes vertical rows and owns its scrolling. Long lines are
unwrapped and scroll horizontally. Split sides share vertical movement and
alignment. Wrapping is not supported; it would require a distinct paired-height
contract and measurement invalidation. Annotation content participates in paired row measurement. The application
changes `with_layout_revision` when its annotation content changes height; the
renderer also tracks typography, rem and mode. Width-dependent annotation
geometry needs application-specific runtime verification.

The body remains a Tab stop and owns keyboard focus without adding a focus
ring around the Diff container. Keyboard actions provide scrolling and change
navigation; disclosures must be reachable through a
keyboard path. `scroll_to_line` reveals hidden context when needed, and change
navigation wraps between change groups. Review, stage/unstage, accept/reject,
repository loading and toolbar commands remain application responsibilities.

Text selection and line-range selection have separate meanings. Code gestures
use Base's window selection participant with stable side/source-offset keys.
Gutter and keyboard line selection uses `DiffLineRange`; programmatic selection
uses `set_selected_lines` and does not emit a user selection event. User line
selection emits `DiffEvent::SelectionChanged`. `selected_lines()` does not
convert arbitrary character selection into a line range.

Copying returns original source, preserving tabs and line endings and excluding
gutters, signs, blank alignment cells, and annotation content. Split text
selection stays on one source side. Keys are independent of the currently
painted rows, so virtualized rows do not redefine selected source. Missing or
empty sides cannot receive source selection.

Identical source uses a plain empty-state explanation, with source context
available through the disclosure policy. An added or deleted empty file remains
a file change even with no changed lines. Loading and errors belong to the
operation that loads or computes the source versions.

## Reference API adaptation and extension boundary

| Pierre concept | GPUI Kit adaptation |
| --- | --- |
| `FileContents` and `parseDiffFromFile` | `DiffFile` and a retained `DiffDocument`. |
| `diffStyle` split/unified | `DiffMode` and `DiffState::set_mode`, independent of source identity. |
| Source-line annotations and selections | Side plus one-based `DiffLinePosition`/`DiffLineRange`, separate from display rows. |
| Context expansion | Retained whole-region disclosures and explicit state methods. |
| Header/annotation rendering | Named application composition seams; see the final public inventory for exact signatures. |
| Virtual diff layout | GPUI row virtualization and measured content, coordinated by `DiffState`. |

The implemented header seam has four callbacks: `render_header` replaces the
header content inside its existing shell; `render_header_prefix`,
`render_header_filename_suffix` and `render_header_metadata` extend the default
header. Each receives `&DiffDocument`, `&mut Window` and `&mut App` and returns
an element. `header(false)` hides the header. A full replacement takes precedence
over the three default-header slots.

`DiffLineAnnotation::new(id, position)` identifies application-owned content.
`with_annotations` supplies the descriptors and `render_annotation` receives
each descriptor with window/application access. Annotations render beneath a
matching visible source line. A hidden context line must be revealed before its
annotation appears; annotations do not implicitly change folding. These slots
retain Diff's source selection and scroll ownership without adding review
commands to the core comparison. Integration tests cover their interaction with
focus, source selection and copy; these tests were compiled, not executed during
this delivery. API inventory and English/Chinese docs agree with these signatures.

Pierre additionally supports parsed patches and browser-specific machinery.
Patch input would require a separate fallible parser and explicit unavailable
context metadata. A patch must not be treated as a complete source file.
Shadow DOM, CSS Grid, Shiki themes and web workers are library implementation
choices, not Rust API names to reproduce.

## Verification and delivery

Completion requires the module export, a Story Gallery entry with restoration
and runnable demonstration scenarios, and matching English/Chinese component
docs reflecting the actual supported API. The gallery provides layout controls,
change navigation and an options menu around the comparison surface. The reference and scope above are not proof of completion.

The source model needs focused coverage for insertion, deletion, replacement,
multiple change groups, exact source line numbers, Unicode byte boundaries,
newline differences and whole-region context expansion. Behavior coverage must
exercise selection/copy, navigation, state replacement and meaningful event
semantics through their public contracts. Include the combined component builder
test required by the repository; avoid tests solely for presentation sizes.
The user's repository configuration does not require running tests.

Local Linux screenshots were reviewed for the unified/split surface, context
disclosures, paired annotation placement and comment hierarchy. A complete
platform matrix remains unverified: light/dark themes, narrow panels, enlarged
rem size, long lines, large input and keyboard-only navigation should be checked
in consumer applications on each supported platform. Compiler, test and runtime
evidence are reported separately in the review record.
