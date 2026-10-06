# Diff component design

Scope: readonly display of application-supplied unified and Git diffs. The
application obtains or computes the patch; the UI framework parses its display
structure and renders it. Complete old/new file comparison is outside this API.

## Reference implementation

The user selected [Pierre Diffs](https://diffs.com/) as the reference. Its
[documentation](https://diffs.com/docs) exposes distinct input paths:

| Pierre API | Input and responsibility | GPUI Kit decision |
| --- | --- | --- |
| `MultiFileDiff` | Original and modified file contents; computes comparison metadata | Do not adopt this comparison responsibility. |
| `PatchDiff` | Patch string; parses files and hunks for display | Accept external unified/Git diff text through `DiffDocument::parse`. |
| `FileDiff` | Prepared file-diff metadata | Retain one immutable document per file; one interaction state shows any number of them. |

The useful shared contracts are side/source-line coordinates, explicit
Unified/Split presentation, file-header slots, line annotations and coordinated
virtualized scrolling. This is a design reference, not API or feature parity.
Browser-specific machinery is not part of the Rust contract.

## Task and ownership

The reader identifies changes, inspects supplied context, and copies relevant
code. AI-produced patches and Git diff output may arrive without either complete
file version. The viewer must preserve that limitation throughout its API and
interaction: unavailable source cannot be inferred, selected or expanded.

| Unit | Responsibility |
| --- | --- |
| Application | Obtain/compute patches, select files, load errors, review commands and comment state. |
| `DiffDocument::parse` | Fallibly parse external patch structure into one document per file; never compute differences. |
| `DiffDocument` | Immutable shared file-display model retaining paths, available source, original coordinates, hunks and lazily prepared syntax. |
| `DiffState` | Files in patch order, focus, one shared viewport, folding of supplied context, selection and background syntax preparation. |
| `Diff` | Themed `RenderOnce` presentation, header slots and application-owned annotations. |

The module lives in `crates/component/src/diff/` and uses existing Base selection
and scrolling seams. No Base changes are required. Bilingual usage is published
in [Diff](../website/component/diff.md).

Parse when the patch revision changes, retain state in the owner, and construct
only the presentation during render. Parsing is synchronous and builds each
side's source in one pass; large patches may be parsed on the background
executor, and the owner checks revision identity before installing them.

Syntax is not part of parsing. `DiffState` prepares it on the background
executor, one file at a time, reusing a highlighter per language because
building one compiles its queries. Each hunk is parsed independently so state
left open at a hunk's end cannot leak across unavailable source. Injection
layers are skipped: building them compiles a query on every parse, which made a
200-file patch take 100 s instead of 1.8 s. The result is theme-independent
capture names per line; rendering resolves them against the current theme
without running tree-sitter queries. Neither rendering nor parsing computes
line-level or word-level comparisons.

## Presentation and interaction

Unified is the default; Split is an explicit application choice. Both preserve
one-based old/new source coordinates from hunk headers. Deleted lines precede
added lines in Unified; Split reserves blank alignment cells for unpaired lines.
Signs supplement themed change colors. The viewer has one semantic border;
context disclosure and annotations do not add nested frames.

The default context policy shows up to three supplied unchanged lines around a
change. Disclosures can reveal only source already present in the patch.
`expand_all` shows all supplied context, `collapse_all` restores the configured
policy, and `set_context_lines(None)` disables folding of supplied lines. Gaps
between hunks remain unavailable and must not offer an expansion command.

All files share one virtualized list. A file header row starts each file, and a
summary row stands in for a file without source rows; row identities are scoped
by file. Positions and ranges carry the file index. Multi-file review therefore
needs no nested scrolling regions and stays virtualized across the whole patch.

The body owns both scrolling axes; unwrapped long lines scroll horizontally.
Split sides share vertical alignment, including annotation height. Disclosure
changes splice only the changed rows so measured heights survive; unmeasured
rows carry a code-row height hint so the scrollbar spans the whole patch.
Visible rows are re-measured every frame, and rows whose annotation set changes
are invalidated, so applications never signal layout changes. Typography, rem,
layout and document changes invalidate the corresponding measurements. Content
width comes from shaping the widest candidates by display cells and grows if a
painted line is wider.

The body is a Tab stop without a container focus ring, as requested. Buttons
retain their native focus styling. Code focus owns scrolling, change navigation
and source copying; header and annotation controls retain their own focus and
actions. Programmatic line selection is silent; user line selection emits
`DiffEvent::SelectionChanged`. Character and gutter selection remain separate.

Selection uses source-side keys rather than current display rows. Copy excludes
line numbers, diff prefixes, missing alignment cells, annotation content and
unavailable gaps. It returns only supplied source. A patch cannot guarantee
reconstruction of a complete file or its original newline encoding. An explicit
`\ No newline at end of file` marker is preserved in presentation.

File-header slots and stable source-line annotations are application composition
seams. Review, resolve/reopen, accept/reject, stage/unstage and repository commands
remain outside the core component. Hidden supplied context must be revealed
before its annotation appears; unavailable positions never acquire fabricated
content.

## Verification

Source-model coverage must exercise hunk line counts, one-based sparse source
coordinates, multiple files, added/deleted sides, malformed patches, Unicode,
newline markers and unavailable gaps. State and UI coverage must exercise
selection/copy, navigation, document replacement, annotation focus and genuine
selection events. The combined builder test covers the supported presentation
options. Do not retain old/new comparison fixtures as evidence for this contract.

Parser coverage also includes `git format-patch` signatures, `diff.noprefix` and
`diff.mnemonicPrefix` headers, whitespace-stripped context lines and plain
unified paths whose directories resemble Git prefixes. Syntax coverage verifies
that hunks are parsed independently.
