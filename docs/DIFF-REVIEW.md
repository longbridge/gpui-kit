# Diff design and performance review

Reviewed on 2026-10-06 against the repository Design Guides and Coding Guides.
Scope: readonly comparison, retained state, selection, virtualization, public
API, Story composition and bilingual documentation. No Base code was changed.

## Design decisions

- Unified is the default. Split is an explicit application choice.
- The comparison has one frame using the semantic border token. The Story
  toolbar groups layout and change navigation; secondary choices use Options.
- Context disclosures are compact and left aligned. Review comments have top
  and bottom separators, author/status hierarchy, and a compact trailing command.
- Header and source-line annotation slots keep review operations in the owner.
  Application annotation inputs retain their own focus and actions.
- The body remains keyboard focusable without a container focus ring, as
  requested. Native buttons retain their normal focus treatment. This reduces
  the visibility of body focus and should be considered by consuming apps.
- Source coordinates, stable annotation IDs and exact copy bytes are independent
  of display mode, folding and virtualized rows. Signs supplement change colors.

## Findings resolved

| Area | Finding and correction |
| --- | --- |
| Source model | Bare CR boundaries disagreed with the comparison tokenizer and could index past the model. Compare exact model line slices. Preserve LF/CRLF and final-newline differences. |
| Focus | Body mouse handling could take focus from annotation inputs. Source selection focuses the body; child controls retain focus, and body actions propagate when a child owns focus. |
| Selection | Captured clear events could discard the Shift-gutter anchor or overwrite newer controlled selection. Retain the anchor during capture and reject stale clear events when local selection has been restored. |
| Unified selection | Original-side unchanged selection could lose visible emphasis after switching mode. Shared rows resolve selection against both source positions. |
| Long lines | Whole-line native selection projection repeatedly scanned long tokens. Retain bounded 512-byte display chunks and their text, preserving graphemes where possible and UTF-8 boundaries for exceptionally large clusters. Preserve full-source keys and word selection across chunks. |
| Folding | Repeated context filling could become quadratic. Build merged visibility boundaries and source-to-row maps in linear passes. |
| Navigation | Each change command scanned and allocated the groups again. Cache group rows on projection rebuild and use binary search. |
| Highlighting | Overlapping highlight composition rescanned intervals. Merge intervals with a sweep and ordered active set. |
| Memory | Source/display mappings used per-byte offsets. Retain sparse tab expansions and use identity offsets for ordinary text. |
| Measurements | Annotation height changes invalidated full-file width scans. Separate width caching from height revisions; retain observed text-width corrections. |
| Annotations | Visible rows searched every annotation. Index descriptors by source position. Constructing the descriptor index still costs O(A) for A supplied annotations. |
| Accessibility | Source text lacked an accessible label. Expose exact source with side, line and change description; name icon commands and context disclosures. |

## Measured document construction

Standalone driver linked against the native **Debug** Component library. Each
fixture ran in an isolated process, with one warm-up and five measured document
constructions. Inputs used plain-text syntax. Timing includes `DiffDocument`
construction and its Rope preparation, and excludes fixture/SharedString creation.
RSS is the live memory of the entire process, not document allocation size.

| Fixture | Bytes across both inputs | Median | Process RSS |
| --- | ---: | ---: | ---: |
| 5,000 lines, sparse changes | 480,000 | 7.162 ms | 11,592 KiB |
| 50,000 lines, sparse changes | 4,800,000 | 69.043 ms | 46,756 KiB |
| 50,000 lines, complete replacement | 4,800,000 | 1,143.281 ms | 50,408 KiB |
| 5,000 Unicode/CRLF lines | 447,780 | 6.605 ms | 11,364 KiB |
| One 100 KiB token per side | 204,800 | 15.644 ms | 8,488 KiB |
| 100 KiB line with one tab per side | 204,810 | 17.301 ms | 8,744 KiB |

These measurements do not establish release performance, rendering latency,
scrolling FPS, selection latency, or syntax-query/injected-language costs.
Document creation is synchronous: prepare large comparisons on the background
executor and verify the source revision before installing the result.

Line comparison has a one-second algorithm deadline, which may coarsen grouping
without discarding source. Optional inline comparisons have a ten-millisecond
per-line budget and a 100-millisecond aggregate cutoff. These are algorithm
budgets, not a hard deadline for all document construction. The highlighter's
100-millisecond host-parser budget does not cap query compilation or injection
parsing. Vertical rendering is virtualized; a visible long line still lays out
its chunks horizontally and is not horizontally virtualized.

## Verification evidence and limits

Passed:

- `cargo fmt --check`
- `cargo clippy -p gpui-component -p gpui-component-story -- --deny warnings`
- `cargo check -p gpui-component --tests -p gpui-kit --features test-support --test diff`
- `cargo check -p gpui-component --features gpui-fast`
- `cargo build -p gpui-component-story`
- `git diff --check`

Model and state regression tests, the combined builder test, and seven production
UI integration tests were added. They cover source bytes, newline changes,
selection, mode transitions, gutters, child input focus, context keyboard access,
and accessibility source labels. **Tests were compiled but not executed**, in
accordance with the user's repository configuration.

Local Linux screenshots and the user's screenshots informed the unified/split
visual review. macOS/Windows behavior, screen-reader operation, a complete theme
and sizing matrix, and native frame-time profiling remain unverified. Full-file
input, unwrapped lines and whole-region context expansion are the current
contracts; patch parsing, editing and incremental context expansion are absent.
