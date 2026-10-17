# Block-based WYSIWYG editor for gpui-kit: design proposal

**Status:** Draft for review. A model-only spike of §4–§5 exists as a draft PR from `wabzqem:block-editor-model-spike` (feature `block-editor`); see Appendix B for what it changed in this design.
**Context:** Taking over [longbridge/gpui-kit#3049](https://github.com/longbridge/gpui-kit/pull/3049), which EnoroF opened and then handed over on 2026-10-08.
**Author:** Richard Nelson (@wabzqem)
**Reviewers:** @huacnlee and the gpui-kit maintainers. Also cc @EnoroF.
**Checked against:** `main` @ `369f7a8f` and PR head @ `47141037` (merge-base `3f1dda6e`). Appendix A lists what was verified and how.

---

## 0. Sources

- The public PR page and @huacnlee's review on #3049 ("Design-level issues" 1–8 and "Suggested path"). Labels **R1**–**R8** below refer to those eight review items.
- The PR head `47141037` (`crates/base/src/text/editor/{mod,model,input,tests}.rs`, plus its changes to `inline.rs`, `inline_flow.rs`, `node.rs`, `text_view.rs`, `mod.rs`).
- `main` @ `369f7a8f`, in particular `crates/base/src/text/*`, `crates/base/src/input/base/*`, `crates/base/src/dock/layout/*`, `crates/base/src/undo_history.rs`, `crates/base/src/text_selection.rs`, `crates/component/src/input/input.rs`, and `docs/ARCHITECTURE.md`.
- Issue #3110 (atomic inline tokens), whose design landed as `gpui_base::input::{InlineToken, InlineTokenSpan, InputContent}`.

Every type and API sketch here is a **proposal**. Statements about existing code were checked against the SHAs above. The few that are still open are marked *(unverified: reason)*.

---

## 1. Goal

Build a block-based WYSIWYG Markdown editor that others can build on. Its foundation is a document model, not a list of commands:

1. **Stable, validated positions** that can address *every* block, including code blocks, tables and cells, images, custom plugin nodes, and content nested in lists or blockquotes.
2. **One edit path.** Every change is an invertible step, and steps are grouped into transactions. Undo, redo, event payloads, and position mapping all come from that path.
3. **Serialization that always round-trips through gpui-kit's own parser**, guaranteed by keeping the model normalized.
4. **A strict render/paint/event contract.** Paint reads a snapshot and records geometry. It never changes the editor model.
5. **A queryable public API.** Applications can read the caret, selection, active marks, and block kind (enough for toolbar state), and events carry payloads.
6. **Reuse of the Markdown pipeline.** Parsing, `BlockNode` rendering, and `MarkdownExtensions` plugins keep working through the `InlineInteraction` seam that the review asked us to keep.

## 2. What we keep from #3049

| Keep | Status on `main` @ `369f7a8f` |
|---|---|
| `InlineInteraction` + `NodeContext::paragraph_interaction` | Still the right seam, but the patch **does not apply** to main. `inline.rs`, `inline_flow.rs`, `node.rs`, and `text_view.rs` all conflict. `text/` has changed by about 11.9k lines across 46 commits since the merge-base: `Inline` now has a retained layout (`RetainedLayout`, `handed_over`), `range_backgrounds`, and `reveal`; `Paragraph` has a `render_cache`; `NodeContext.style` is `Arc<TextViewStyle>`. Phase 0a is therefore a re-implementation, not a cherry-pick (§7.2). |
| List-item continuation-line indentation fix in `BlockNode::to_markdown` | Still needed. Main still emits `format!("{}{}", prefix, child.to_markdown())` for list items. |
| `examples/fixtures/markdown_plugins.rs` extraction | Needs redoing. `examples/markdown/src/main.rs` on main is 1866 lines and has gained `mention.rs`. |
| `editor/input.rs` (`EntityInputHandler`: UTF-16↔byte mapping, marked text, `bounds_for_range`, `character_index_for_point`) | Carries over once rebased onto the new position types. |
| Key bindings, `BlinkCursor` reuse | Carry over. `crate::text::selection::word_range_at`, which the PR uses, still exists on main; `crate::text_boundary::word_range_at` is also there and is what the input engine uses. |
| The editor example | Carries over. It loads `examples/fixtures/test.md` (320 lines). §8 adds a generated fixture of 1000+ blocks for performance work. |

What we replace: `model.rs` (rewritten), the whole-document snapshot undo, the `UndoHistory` use (§5.2), the paint-time `owner.update(..)` in `InlineEditing::paint`, and the `MarkdownEditorState` name.

**What I confirmed about the current PR** (by probing the PR head): typing `c` then `d` in bold after `**ab**` gives `source() == "**ab****c****d**"` with 3 inline nodes (R3). For `# T\n\n- a\n\n```rs\ncode\n```` the model sees 2 editable paragraphs out of 3 top-level blocks, so the code block has no position (R2). The PR builds cleanly and its 24 editor tests pass, alongside 2 probe tests of mine.

## 3. Non-goals for the first landing

- Collaborative editing and CRDTs. The step/mapping design shouldn't rule them out.
- Byte-exact source preservation. Serialization is *canonical*: it round-trips through our parser but doesn't keep stylistic choices like `*` vs `_` or `-` vs `*`.
- In-place editing inside code blocks and table cells in Phase 3. Both are addressable from day one, so they can be selected, deleted, moved, and copied. In-place editing arrives in Phase 4.
- Raw HTML. On main, HTML goes through `format::html::parse` into ordinary nodes, and `BlockNode` has no HTML variant, so the editor sees whatever nodes that produces. Footnote definitions (`BlockNode::Definition`) and frontmatter are kept as opaque atoms.
- Drag handles, slash menus, and floating toolbars. Applications can build these on the public API.
- Virtualized rendering (planned, §8).

## 4. Proposed model

### 4.1 Separate the editing model from the render AST

Today the PR's `Document` is a `Vec<BlockNode>` and edits the render AST in place. On main, `BlockNode` and `Paragraph` carry render state (`state: Arc<Mutex<InlineState>>`, `render_cache`), and the PR identifies a paragraph by `Arc::as_ptr(&self.state)` (R1). I propose an editing model that owns identity and invariants, with a cached projection into `BlockNode` for rendering:

```text
 Markdown ──markdown::parse──▶ BlockNode tree ──import──▶ EditorDocument (ids, invariants)
                                                              │
                                           transactions ──────┤
                                                              ▼
                                     projection (dirty blocks only) ──▶ BlockNode for rendering
                                                              │
                                      serialize (MarkdownSource-based) ──▶ Markdown
```

The dock already works this way. `PaneTree` (`crates/base/src/dock/layout/tree.rs`) is the single source of truth, has stable `NodeId`s, runs `normalize` inside every edit method, and hands out a borrowed `PaneRef` projection.

### 4.2 Block tree and identity (proposal)

```rust
/// Opaque and process-unique, allocated from a global AtomicU64 like
/// dock::NodeId (`next_node_id`) and TextSelectionScopeId::new(). Never reused,
/// so an id from a discarded document or a pre-`set_source` state can only
/// ever fail lookup — it can never alias a live block.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct BlockId(NonZeroU64);

pub(crate) struct EditorDocument {
    revision: u64,                         // bumped per committed transaction
    root: BlockId,
    blocks: HashMap<BlockId, BlockEntry>,  // see D4
    order: OrderIndex,                     // §4.5
    extensions: Arc<MarkdownExtensions>,
    link_refs: HashMap<SharedString, LinkMark>,   // as NodeContext::link_refs
}

struct BlockEntry {
    parent: Option<BlockId>,
    kind: BlockKindData,     // private; carries e.g. ColumnumnAlign (sic, main's type name) for tables
    content: BlockContent,
}

/// Public, payload-light projection of a block's kind, for matching in toolbar code.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlockKind {
    // containers
    Root, Blockquote, List { ordered: bool }, ListItem { checked: Option<bool> },
    Table, TableRow, TableCell,
    // text-bearing leaves
    Paragraph, Heading { level: u8 }, CodeBlock,
    // atomic leaves
    Image, HorizontalRule, Break, Custom, Definition, Unknown,
}

enum BlockContent {
    Children(Vec<BlockId>),
    Inline(InlineText),     // Paragraph, Heading, TableCell
    Plain(String),          // CodeBlock (language kept in BlockKindData)
    Atom(AtomData),         // Custom (MarkdownNode), Definition, Break, HorizontalRule, Unknown
}
```

The variants mirror main's `BlockNode` (`Root, Paragraph, Heading, Blockquote, List, ListItem, CodeBlock, Custom { .. }, Table, Break, HorizontalRule, Definition, Unknown`). The one addition is that table rows and cells become nodes of their own.

- **Every** node has a `BlockId`, so code blocks, tables, cells, images, and custom nodes all get positions and node selection (R2).
- Custom nodes keep their `MarkdownNode`. Main's `MarkdownNode::to_markdown()` is what the serializer uses for them, so plugin content round-trips without the editor understanding it.

### 4.3 Position (proposal) (R1)

The name is `BlockPosition`. `gpui_base::input` already exports `Position` (a re-export of `lsp_types::Position`) and `Selection` (`input/base/cursor.rs`). Reusing those names would make glob imports ambiguous.

```rust
/// A caret location. Plain value type (its fields are the definition),
/// in the ARCHITECTURE.md sense of `Point` / `Selection`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BlockPosition {
    pub block: BlockId,
    /// UTF-8 byte offset into the block's flat text, on a grapheme boundary.
    /// For atomic blocks: 0 = before, 1 = after.
    pub offset: usize,
    /// Caret side at a soft wrap or mark boundary. Not part of ordering.
    pub affinity: Affinity,
}

pub enum Affinity { Upstream, Downstream }

#[non_exhaustive]
pub enum PositionError {
    UnknownBlock(BlockId),           // removed, from another document, or from before set_source
    NotACaretBlock(BlockId),         // e.g. List, TableRow
    OffsetOutOfBounds { len: usize },
    NotOnGraphemeBoundary,
}

impl EditorDocument {
    pub fn validate(&self, p: BlockPosition) -> Result<BlockPosition, PositionError>;
    pub fn clamp(&self, p: BlockPosition) -> BlockPosition;
    pub fn cmp(&self, a: BlockPosition, b: BlockPosition) -> Result<Ordering, PositionError>; // O(1), §4.5
    pub fn source_offset(&self, p: BlockPosition) -> Option<usize>;            // for persistence
    pub fn position_at_source_offset(&self, offset: usize) -> Option<BlockPosition>;
}
```

- **Stable:** ids come from a counter, not an address, and survive edits, undo, and redo. Undoing a removal re-inserts the recorded entries with their original ids.
- **Validated:** every public entry point validates its input and returns `Result`. In the PR, `Document::replace` does `ps.iter().find(..).unwrap()` at `model.rs:296` (verified at `47141037`), so a stale position panics. Here it becomes an `Err`.
- **Serializable:** it is plain data, but only meaningful while its blocks exist. Anything that has to outlive `set_source` or a session goes through `source_offset` (D1).

### 4.4 Selection (proposal)

```rust
#[derive(Clone, PartialEq, Debug)]
pub enum BlockSelection {
    /// Text range, possibly spanning blocks; anchor == head is a caret.
    Text { anchor: BlockPosition, head: BlockPosition },
    /// Whole blocks as objects (image, code block, table, run of siblings).
    Blocks { anchor: BlockId, head: BlockId },
    /// Rectangular cell range in one table (Phase 4c).
    Cells { table: BlockId, anchor: (u32, u32), head: (u32, u32) },
}
```

`BlockEditorState::set_selection` enforces these rules:
- `Text` endpoints must be valid caret positions. Atoms between the endpoints count as fully covered, so select-all followed by typing removes the code block too, which fixes the R2 repro. *Spike:* a selection covering the whole document is replaced by one empty paragraph, so typing after select-all gives a plain paragraph (not the first block's kind, e.g. a heading).
- `Blocks` endpoints are normalized to siblings under their lowest common ancestor.

### 4.5 Document order index (R4)

```rust
struct OrderIndex {
    structure_revision: u64,
    leaves: Vec<BlockId>,             // caret-bearing leaves in document order
    rank: HashMap<BlockId, u32>,
}
```

The index is rebuilt in O(N) only on *structural* steps. Text steps leave it alone. Position comparison and per-block selection-range queries become O(1). The PR, by contrast, calls `selection_in` for every painted paragraph, and each call runs `paragraphs()` and `ordered()`, which is O(P²) per frame.

### 4.6 Inline content, marks, normalization, serialization (R3)

```rust
pub(crate) struct InlineText {
    text: String,                      // flat UTF-8; inline atoms occupy one U+FFFC
    runs: Vec<MarkRun>,                // contiguous, non-overlapping, cover 0..text.len()
    atoms: Vec<(usize, InlineAtom)>,   // offset of each U+FFFC
}
struct MarkRun { len: usize, marks: TextMark }      // TextMark: main's crate-internal storage type
enum InlineAtom { Image(ImageNode), Custom(MarkdownNode) }
```

These invariants hold after every step, and a `debug_assert!` checks them after each one:
- **N1:** no empty runs, and no two adjacent runs with equal `TextMark`.
- **N2:** every atom has a run of its own.
- **N3:** run lengths sum to the text length.

**Serializer: extend main's `MarkdownSource` rather than write a new one.** Main already has `MarkdownSource` in `node.rs`. It is a layered serializer that picks the mark spanning the most neighbouring pieces, puts it outermost, and recurses. TextView uses it for selection-as-source and for paragraphs that contain custom nodes. I probed it on main:

| Input runs | `MarkdownSource` output | Reparsed by main's `markdown::parse` |
|---|---|---|
| `b`{bold} `c`{bold,italic} `d`{bold} | `**b*c*d**` | ✅ same marks |
| `a`{bold} `b`{bold} (unmerged equal runs) | `**ab**` | ✅ |
| `ab`{underline} | `<u>ab</u>` | ✅ underline |
| `ab`{highlight #336699} | `==ab==` | ❌ literal `==ab==`, no highlight, colour lost |
| `a `{bold} `b` (trailing space in run) | `**a **b` | ❌ literal text, no emphasis |
| overlapping ranges 0..3 bold + 1..2 italic | `**bcd***c*d` | ❌ duplicated text (input breaks N1–N3) |

With normalized runs, `MarkdownSource` is already correct for nesting and merging. The editor serializer reuses it and adds three things:
1. **Whitespace hoisting:** leading and trailing whitespace moves outside the delimiters.
2. **Literal escaping:** `push_str` currently emits text verbatim.
3. **Highlight as `<mark>`:** inline `<mark>` *is* parsed by main (`inline_html_mark`, test `inline_html_mark_applies_highlight_colors`), including `color` / `style` attributes (D5).

*Spike finding:* those three were not enough for round-trip. The spike's property tests also needed (a) **flanking-safe delimiters**: `*`, `**` or `~~` between a word character outside and punctuation inside (``a*`x`*``) cannot open or close in CommonMark, so the serializer falls back to `<em>` / `<strong>` / `<s>`, which main's parser reads back; and (b) **code innermost**: nothing inside a code span is parsed, so other marks always wrap the code span rather than the reverse. Both sit behind `hoist_whitespace`, so TextView's default output is unchanged. Reference links are resolved against the document's definitions on import and written back as inline links (canonical form).

**`Paragraph::to_markdown` on main is lossy.** For paragraphs without custom nodes it doesn't use `MarkdownSource`. It rewraps the node text once per mark and overwrites the result each time. Probed on main:
- `a ***x*** b` → `a *x* b` (bold lost).
- `a **b*c*d** e` → `a **bcd** e` (italic lost).
- `a <u>x</u> b` → `a x b` (underline dropped).

`ParsedDocument::to_markdown` is `#[allow(dead_code)]` and documented as "used to generate markdown for test", yet the PR uses it as `source()`. Phase 0d routes `Paragraph::to_markdown` through `MarkdownSource` regardless of the editor work.

On R3's specific string: main's parser reads `**ab****c****d**` back as bold `abcd`, plus a redundant nested bold mark on `c`. So it isn't lossy *in our parser*, but it isn't canonical, and normalization (N1) prevents it from being produced at all. *(Unverified: how other CommonMark implementations read that string. I only tested gpui-kit's parser.)*

**Marks in the public API:** `TextMark` and `LinkMark` aren't re-exported from `gpui_base` on main (`text::node` is private; only `CodeBlock` and `TableData` are re-exported), and both have `pub` fields. So the public mark API is keyed on `TextMark`'s *attributes*, and `TextMark` stays internal storage (D12):

```rust
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MarkKind { Bold, Italic, Strikethrough, Underline, Code, Highlight, Link }

/// Read-only view of the marks at a position. Private fields, readers only.
pub struct MarkSet { /* wraps TextMark */ }
impl MarkSet {
    pub fn has(&self, kind: MarkKind) -> bool;
    pub fn highlight(&self) -> Option<Hsla>;
    pub fn link_url(&self) -> Option<&SharedString>;
}

/// A change to apply over a range. Builder with private fields.
pub struct MarkChange { /* .set(MarkKind) .clear(MarkKind) .with_link(url, title) .with_highlight(Hsla) */ }

pub enum MarkState { Off, On, Mixed }
```

Stored marks at a collapsed caret work like the PR's `stored`: they are cleared on caret movement, and inheritance at boundaries follows D6.

## 5. Editing primitives, transactions, history (R6, R7)

### 5.1 Steps

```rust
pub(crate) enum Step {
    ReplaceText  { block: BlockId, range: Range<usize>, insert: InlineText },
    ChangeMarks  { block: BlockId, range: Range<usize>, change: MarkChange, prev: Vec<MarkRun> },
    InsertBlocks { parent: BlockId, index: usize, blocks: Vec<DetachedBlock> },
    RemoveBlocks { parent: BlockId, range: Range<usize>, removed: Vec<DetachedBlock> },
    MoveBlocks   { from: (BlockId, Range<usize>), to: (BlockId, usize) },
    SetKind      { block: BlockId, kind: BlockKindData, prev: BlockKindData },
    SplitBlock   { block: BlockId, offset: usize, new_ids: SmallVec<[BlockId; 2]>, depth: u8 },
    JoinBlocks   { first: BlockId, second: BlockId, at: usize },
}
impl Step {
    fn apply(&self, doc: &mut EditorDocument) -> Result<StepMap, StepError>;
    fn invert(&self, before: &EditorDocument) -> Step;
}
```

- *Spike finding:* only four primitive steps were needed: `ReplaceText`, `InsertBlocks`, `RemoveBlocks`, `SetKind`. Mark changes are a `ReplaceText` of the same text with new runs; split, join and move are composites (move = remove + insert, ids kept). `apply` returns the inverse step directly instead of a separate `invert(&before)`. This kept inversion trivial; a dedicated `ChangeMarks` can come back if undo memory for large mark changes matters.
- `DetachedBlock` keeps its ids. New ids are allocated when a transaction is *built* and recorded in its steps, so redo reproduces the same ids.
- A `normalize` pass, the same idea as `PaneTree::normalize`, ends every transaction. It appends its own steps so that undo stays exact. It enforces the structural invariants: a `List` contains only `ListItem`s; a non-empty `ListItem` starts with a leaf; the only empty container allowed is `Root`; and there is always at least one caret leaf. *Spike finding:* one more rule was needed: adjacent sibling lists of the same type are merged, because Markdown cannot write them apart (main's parser reads them back as one list, even with a `*` vs `-` marker switch).

### 5.2 Transactions and history

```rust
pub struct Transaction {           // private fields
    steps: Vec<Step>,
    maps: Vec<StepMap>,
    selection_before: BlockSelection,
    selection_after: BlockSelection,
    origin: EditOrigin,
}
#[non_exhaustive]
pub enum EditOrigin { Typing, Backspace, DeleteForward, Ime, Paste, Cut, Command(&'static str), InputRule(&'static str), Api, Undo, Redo }
```

**History follows the input engine's `UndoManager` semantics, not `UndoHistory`** (D11). Here is what main actually has:
- `crate::undo_history::UndoHistory<T>` groups only by elapsed time (`group_interval`) or by an explicit `start_grouping` / `end_grouping` flag. `undo` and `redo` need `T: Clone`. The only in-tree user is the PR.
- `input/base/undo_manager.rs::UndoManager` (`pub(crate)`) coalesces by **intent** (`EditIntent::{Typing, Backspace, DeleteForward, Atomic}`). It supports nested `begin_transaction` / `begin_transaction_with` / `commit_transaction` brackets for IME and batches. It breaks coalescing explicitly on caret movement, touch selection, and blur (`break_transaction_coalescing`). That is the behaviour this editor needs: a word is one undo step, and a caret move ends the step.

`UndoManager` is tied to the input engine's rope `Change` type. Phase 2 therefore implements a `BlockUndoManager` over `Transaction` with the same coalescing rules. Merging the two into a generic manager is left as a follow-up rather than a prerequisite. A no-op is detected as an empty step list, not by comparing whole trees as the PR's `edit()` does.

### 5.3 Commands on resolved, nested positions (R7)

```rust
pub type CommandResult = Result<Option<Transaction>, CommandError>; // Ok(None) = not applicable here

pub(crate) struct ResolvedPosition<'a> {
    pos: BlockPosition,
    path: SmallVec<[BlockRef<'a>; 8]>,   // root → leaf, e.g. [Root, Blockquote, List, ListItem, Paragraph]
}
```

Commands find their target by walking `path` (for example, "the nearest `ListItem`"). In the PR, `make_list` and `move_block` search `self.blocks` directly, so they only see the top level. Initial command set: `insert_text`, `delete_backward/forward` (grapheme / word / line, reusing `crate::text_boundary`), `split_block`, `join_backward/forward`, `toggle_mark`, `apply_mark`, `set_block_kind`, `toggle_list`, `toggle_task`, `set_task_checked`, `indent`, `outdent`, `wrap_in_blockquote`, `lift`, `move_blocks`, `delete_selection`, `insert_block`, `select_all`, `select_block`.

The Markdown shortcuts (`# `, `- `, `1. `, `[ ] `) leave `insert()` and become an `InputRule` registry (`InputRule::markdown_defaults()`). Each rule runs as its own transaction after the typing transaction, so a single undo reverts the conversion and keeps the typed text.

## 6. Public API and events (R6)

These follow ARCHITECTURE.md's seam rules: private fields, builders, and readers on every type that crosses the seam.

```rust
impl BlockEditorState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self;
    pub fn with_extensions(self, extensions: MarkdownExtensions) -> Self;
    pub fn with_input_rules(self, rules: Vec<InputRule>) -> Self;
    pub fn set_source(&mut self, source: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), SharedString>; // markdown::parse's error type; no Changed event; clears history
    pub fn source(&self) -> String;
    pub fn revision(&self) -> u64;
    pub fn set_editor_style(&mut self, style: BlockEditorStyle);   // mirrors InputBaseState::set_editor_style

    pub fn document(&self) -> DocumentRef<'_>;                  // borrowed projection, like PaneRef
    pub fn block(&self, id: BlockId) -> Option<BlockRef<'_>>;
    pub fn selection(&self) -> &BlockSelection;
    pub fn caret(&self) -> Option<BlockPosition>;
    pub fn selected_text(&self) -> String;
    pub fn selected_source(&self) -> String;
    pub fn active_marks(&self) -> MarkSet;
    pub fn mark_state(&self, kind: MarkKind) -> MarkState;
    pub fn block_kind_at_caret(&self) -> Option<BlockKind>;
    pub fn can_undo(&self) -> bool;
    pub fn can_redo(&self) -> bool;
    pub fn is_readonly(&self) -> bool;

    pub fn set_selection(&mut self, selection: BlockSelection, cx: &mut Context<Self>) -> Result<(), SelectionError>;
    pub fn run(&mut self, command: EditorCommand, window: &mut Window, cx: &mut Context<Self>) -> Result<bool, CommandError>;
    pub fn transact(&mut self, origin: EditOrigin,
                    build: impl FnOnce(&mut TransactionBuilder<'_>) -> Result<(), CommandError>,
                    cx: &mut Context<Self>) -> Result<(), CommandError>;
    pub fn undo(&mut self, cx: &mut Context<Self>);
    pub fn redo(&mut self, cx: &mut Context<Self>);
    pub fn set_readonly(&mut self, readonly: bool, cx: &mut Context<Self>);
    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>);
}

#[non_exhaustive]
pub enum EditorCommand { ToggleMark(MarkKind), ApplyMark(MarkChange), SetBlockKind(BlockKind), ToggleList { ordered: bool }, ToggleTask, Indent, Outdent, MoveBlocks { down: bool }, InsertBlock(BlockKind) }
```

Events extend the pattern of main's `InputEvent { Change, PressEnter { .. }, Focus, Blur }` by adding payloads:

```rust
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum BlockEditorEvent {
    Changed(ChangeSummary),            // once per committed transaction, incl. undo/redo
    SelectionChanged(SelectionSummary),
    Focus,
    Blur,
}
pub struct ChangeSummary { revision: u64, origin: EditOrigin, changed_blocks: Vec<BlockId>, structural: bool }   // readers
pub struct SelectionSummary { selection: BlockSelection, active_marks: MarkSet, block_kind: Option<BlockKind> } // readers
```

`set_source` never emits `Changed`. This matches the existing convention for setting a value programmatically.

## 7. Rendering and layout contract (R5)

### 7.1 Phases

| Phase | May read | May write | Must not |
|---|---|---|---|
| Event / action handlers | model, last complete `FrameLayout` | model (through transactions), selection | — |
| `render` | model, focus, `BlinkCursor::visible()` | projection cache; builds the immutable `Arc<EditorFrame>` | walk the full tree when the revision hasn't changed |
| prepaint / paint | `Arc<EditorFrame>` | the frame-local `FrameLayoutBuilder` sink only | `owner.update(..)`, editor model, focus or blink state, `cx.notify()` |

A frame-local sink behind a mutex is in line with existing practice. On main, `Inline::paint` already writes `InlineState.selection` under its mutex during paint. The rule is that paint never reaches back into the editor entity.

**Precedent on main.** `NodeContext` already carries per-frame immutable snapshots that are resolved once and looked up per leaf: `range_highlights: Option<Arc<RangeHighlightFrame>>` and `stream_fade: Option<Arc<StreamFadeFrame>>`. `RangeHighlightFrame::backgrounds(TextLeafKey)` binary-searches a vector sorted by key, and `Inline` paints the result through `range_backgrounds`. The editor follows the same pattern:

- **Selection backgrounds** are resolved in `render` into per-leaf ranges (O(1) per leaf via `OrderIndex`). They are painted through `Inline`'s existing `range_backgrounds` path, so selection drawing needs no new paint code.
- **`InlineInteraction`** shrinks to caret, composition underline, and reporting layout into the sink.

**Blink and focus** copy `InputBaseState::new_in_mode` on main exactly:
- `cx.on_focus` → `blink_cursor.start` and emit `Focus`.
- `cx.on_blur` → `blink_cursor.stop`, break undo coalescing, and emit `Blur`.
- `cx.observe_window_activation` restarts the blink when the window is active and the editor is focused.
- `cx.intercept_keystrokes` pauses the blink on every keystroke while focused.
- `cx.observe(&blink_cursor, ..)` calls `notify`.

None of this lives in paint. The PR does it all from `InlineEditing::paint`.

`FrameLayout` (per-leaf `TextLayout` and bounds, plus block bounds) is double-buffered. It is swapped in at the start of the next `render`, so handlers always hit-test against a complete frame. Positions taken from an older frame go through `clamp`.

### 7.2 Paragraph identity in the seam (D3)

Main identifies text leaves by `TextLeafKey { block_start, ordinal }`, which is derived from the parse-time source `span` (`stream_fade.rs`). That works for read-only views. It isn't stable across edits, and the editor's projected nodes have no meaningful source spans. Phase 0a therefore re-implements the PR's seam on main's current `Inline` / `InlineFlow` / `Paragraph::render` (including the `render_cache` plain path) and adds a crate-internal key:

```rust
impl Paragraph { pub(crate) fn with_editor_key(self, key: u64) -> Self; pub(crate) fn editor_key(&self) -> Option<u64>; }
// same on CodeBlock, Table cells, and the image/custom wrappers
```

`TextView` never sets the key, so read-only rendering is unchanged. Non-text blocks render through their existing renderers inside an editor-owned **block frame**. The frame records bounds, paints the block-selection treatment from `BlockEditorStyle`, and turns clicks into `BlockSelection::Blocks`.

### 7.3 Projection cache

The model is projected directly into `BlockNode`s (`block_editor/direct.rs`); there is no Markdown round trip. `NodeCache` holds `HashMap<BlockId, (block_revision, BlockNode)>` for every block and the same for table rows. `Step::apply` bumps the revision of the touched block and all its ancestors, so an edit rebuilds only the path from the edited leaf to its top-level block; every other subtree is reused as is, keeping its `InlineState` and retained layout (the reused node is cloned, which shares those `Arc`s). Entries of blocks that left the document are dropped once the cache exceeds about twice the document size. Equivalence with `parse(serialize(doc))` is tested on fixtures and 300 seeded documents (`tests/projection.rs`).

## 8. Performance constraints (R4)

| Operation | Bound |
|---|---|
| Steady-state frame (blink, hover) | O(visible blocks). No tree walk, no allocation proportional to N. |
| Per-leaf selection lookup | O(1) via `OrderIndex` (or a binary search, as in `RangeHighlightFrame`) |
| Keystroke in one paragraph | O(paragraph + log N) plus one re-projection. No `OrderIndex` rebuild. |
| Structural edit | O(N) `OrderIndex` rebuild plus re-projection of the affected blocks |
| Undo / redo | O(size of the transaction) |
| Accessibility value | Built only when `window.is_a11y_active()` (or `test-support`), cached per revision (D8) |
| `source()` | O(N), on request only |

**Fixture.** I couldn't find a 1000-line Markdown fixture in either tree. The PR example loads `examples/fixtures/test.md` (320 lines). Phase 3 adds a generated fixture of 1000+ blocks for the counters and the manual timing. *(Unverified: which fixture R4 meant by "the 1000-line fixture".)* Virtualization (`VirtualList`) is Phase 5.

## 9. Naming and layering (R8)

**Naming (D10):** `BlockEditorState` / `BlockEditor` in `gpui_base::text::block_editor`. `EditorState` on main is `pub type EditorState = InputBaseState<EditorMode>`, one of the three source-text facades over the shared engine. This editor is a separate entity, not another `InputBaseState` mode.

**Style seam.** `InputEditorStyle` on main has `pub` fields (`foreground`, `selection`, `caret`, …), `resolved(&self, tokens: &SemanticThemeTokens) -> Self` (fills in unset, i.e. transparent, colours), and is injected with `InputBaseState::set_editor_style`. `BlockEditorStyle` mirrors the injection and resolution, but its fields are **private**, with `with_*` setters and readers, per ARCHITECTURE.md invariant 12:

```rust
pub struct BlockEditorStyle { /* text: Option<TextViewStyle>, caret, caret_width, selection,
                                 block_selection, composition_underline, placeholder, placeholder_color */ }
impl BlockEditorStyle { pub fn resolved(&self, tokens: &SemanticThemeTokens) -> Self; }
```

- **Selection colour:** defaults to `TextViewStyle::selection()`, the same source `Inline::paint` uses.
- **`TextViewDefaults` fields stay private.** On main they are private and only read inside `text_view.rs` (`cx.try_global::<TextViewDefaults>()`). The editor gets `pub(crate)` readers (`style()`, `code_block_highlighter()`) instead of the PR's `pub(super)` fields.
- **The `\u{200b}` empty-paragraph measurement** stays internal to `Inline` and only activates when an interaction is installed. Placeholder text comes from the style.

**Layering (D2):** the code lives in `gpui-base` behind an off-by-default `block-editor` feature, with no `gpui-component` re-export until Phase 3 is accepted.

**Input engine (D7):** prose and code blocks use the block editor's own text path. #3113's inline tokens are the reference for atomic inline semantics (Phase 4d), not a shared implementation.

## 10. Phased plan

**Phase 0: small, independent PRs against current main**
- **0a** Re-implement `InlineInteraction` + `paragraph_interaction` on main's `Inline` / `InlineFlow` / `Paragraph::render`, plus `editor_key` (D3). Tests: TextView output is unchanged with no interaction installed. With one installed, the callback gets the right source ranges, including for empty paragraphs.
- **0b** List-item continuation-line indentation in `BlockNode::to_markdown`, plus a regression test.
- **0c** Redo the `examples/fixtures/markdown_plugins.rs` extraction against main's 1866-line `examples/markdown`.
- **0d** Route `Paragraph::to_markdown` through `MarkdownSource` so multi-mark and underline paragraphs serialize correctly, with regression tests for the cases in §4.6.

**Phase 1: model, no UI.** `EditorDocument`, `BlockId`, `BlockPosition`, `BlockSelection`, `OrderIndex`, `InlineText` with N1–N3, import from `BlockNode`, and the `MarkdownSource`-based serializer (whitespace hoisting, escaping, `<mark>`). Round-trip property tests.

**Phase 2: steps, transactions, `BlockUndoManager`, commands, input rules (headless).**

**Phase 3: view.** Projection cache, `EditorFrame` / `FrameLayout`, selection through `range_backgrounds`, IME port, key bindings, focus/blink wiring as in §7.1, `BlockEditorStyle`, events, the component wrapper, the example, and the generated fixture.

**Phase 4: non-text blocks.** 4a node selection, delete, move, and copy for atoms, code blocks, and tables. 4b code-block text editing (*landed on the UI spike*). 4c table cells and `BlockSelection::Cells` (*landed on the UI spike*). 4d inline atoms with #3113 semantics (*landed on the UI spike; see Appendix B*).

**Phase 5: scale.** Virtualization and per-block accessibility. *(Partial on the UI spike, then stopped by decision: prefix-sum height index, cull units cached per revision, O(selected) selection backgrounds, layout reuse for scrolled-back blocks, root `MultilineTextInput` plus per-top-level-block roles. Not done: chunking long tables and nested long lists, accessibility nodes for nested list items and table rows/cells.)*

**Status on `block-editor-ui-spike` (14 Oct 2026).** Phase 0 seam work exists only inside the spike (feature-gated), not as separate PRs against main. Phases 1–2: done (model spike, #3463). Phase 3: done. Phase 4a–4d: done. Phase 5: partial, stopped. Scope is frozen pending review.

#3049 stays open as the tracking PR. Its description links to each phase, and it is closed when Phase 3 lands.

## 11. Decisions for review

Each item is a proposed decision you can accept or push back on.

**D1. Block ids are process-unique, come from a global counter, and are never reused. `set_source` re-imports with fresh ids, and old positions fail with `PositionError::UnknownBlock`. Anything that has to persist goes through `source_offset` / `position_at_source_offset`.**
*Rationale:* This matches `dock::NodeId` (`next_node_id`) and `TextSelectionScopeId::new()` on main. A stale id can never alias a live block, so no separate epoch counter is needed. Matching ids across a re-parse can be added later without breaking anything.
*Alternative considered:* Document-scoped ids plus an epoch counter (the previous draft), or carrying ids across `set_source` by structural diff.

**D2. The editor lands in `gpui-base` behind an off-by-default `block-editor` cargo feature and isn't re-exported from `gpui-component` until Phase 3 is accepted.**
*Rationale:* The model needs `pub(crate)` text internals (`BlockNode`, `Paragraph`, `InlineNode`, `TextMark` storage, `MarkdownSource`). A separate crate would force those to be widened.
*Status (decided):* `gpui-kit` forwarding a `block-editor` feature to `gpui-base/block-editor` (so the story can reach the editor) is acceptable under D2; there is still no `gpui-component` re-export.
*Alternative considered:* A separate `gpui-block-editor` crate over a widened text API.

**D3. Phase 0a re-implements the seam on current main and adds a crate-internal `editor_key: Option<u64>` to `Paragraph`, `CodeBlock`, table cells, and the image/custom wrappers. `TextView` never sets it.**
*Rationale:* The PR's patch doesn't apply to main. Main's own leaf identity (`TextLeafKey`) comes from parse-time spans and isn't stable under edits. The PR's `Arc::as_ptr` identity is what R1 rejected.
*Status (UI spike):* projection is now direct model→`BlockNode` (§7.3, `block_editor/direct.rs`), no Markdown round trip. Seam implemented in a narrower form: `editor_key` lives on `InlineState`, and the seam is a crate-internal `EditorSink` that collects `TextLayout`s and supplies selection backgrounds (Appendix B).
*Alternative considered:* Reusing `TextLeafKey` with synthetic spans assigned by the projection, or a side table keyed by `InlineState` pointer.

**D4. Storage is a private `HashMap<BlockId, BlockEntry>` with parent links plus an explicit `OrderIndex`. No slotmap dependency.**
*Rationale:* `PaneTree` on main is an owned recursive tree (`PaneNode` holding `Vec<PaneNode>`) with DFS lookup (`find_node`). That suits a dock of tens of nodes, but not a document of thousands of blocks, where every command needs O(1) lookup by id and an O(1) parent walk. Ids are never reused (D1), so generational keys would protect nothing. The storage sits behind `DocumentRef` / `BlockRef` and can be swapped without any API change.
*Alternative considered:* An owned recursive tree like `PaneTree`, or a slotmap/arena with generational keys.

**D5. The editor serializer builds on `MarkdownSource` and emits highlight as `<mark>` (with a `color` attribute when it isn't the default `<mark>` colour) and underline as `<u>`. TextView's copy-as-source output stays as it is unless maintainers want `<mark>` there too.**
*Rationale:* Probed on main: `<u>` and `<mark color=…>` both parse back into `TextMark`, while `==x==` stays literal text and loses the colour. Round-trip is a core guarantee. Changing the shared `wrap_with_mark` would change existing TextView behaviour and its tests (`marks_round_trip_through_reconstruction` and the `html.rs` tests expect `==`).
*Alternative considered:* Teaching `markdown::parse` to read `==…==` (it would still lose the colour), or disabling highlight in the editor (the previous draft's fallback).

**D6. Bold, italic, strikethrough, underline, and highlight extend when you type at their trailing boundary. Links and inline code don't, though typing inside them inherits the mark. Stored (toolbar) marks always win.**
*Rationale:* This matches common rich-text editors and avoids the "can't escape the link or code span" problem that the PR's uniform `mark_at` inheritance creates.
*Alternative considered:* Uniform inheritance for all marks (the PR's current behaviour).

**D7. Code-block text editing (Phase 4b) uses the block editor's own text path with `BlockContent::Plain`. It doesn't embed `TextareaState` / `EditorState`.**
*Rationale:* One model means one undo history and selections that can span a code block. An embedded `InputBaseState` would bring its own `UndoManager`, focus handle, and selection.
*Status (Phase 4b, done):* as decided; code text is `BlockContent::Plain` edited through `Step::ReplacePlain` (Appendix B).
*Alternative considered:* Embedding the input engine in the focused code block and syncing through `ReplaceText` steps.

**D8. The editor root uses `Role::MultilineTextInput` with an `aria_value` that is built only when `window.is_a11y_active()` (or with `test-support`) and is cached per revision. Per-block roles are deferred to Phase 5.**
*Rationale:* This is exactly what `gpui-component`'s `Input` does on main for multi-line inputs (`accessibility_role` → `MultilineTextInput`; `aria_value` gated on `is_a11y_active`). It removes the PR's per-frame full-document `aria_value` (R4). *(Unverified: how screen readers on each platform present a long rich document through a single text-input node. That needs runtime testing, not source reading.)*
*Status:* root implemented as described; Phase 5 added roles for laid-out top-level blocks. Nested list items and table rows/cells are not exposed (stopped with Phase 5).
*Alternative considered:* A per-block accessibility tree from Phase 3.

**D9. CI gets deterministic `cfg(test)` counters (zero full-tree walks per steady-state frame, one re-projection per keystroke) and no timing benchmark. Timing numbers on the generated fixture go in PR descriptions.**
*Rationale:* Counters catch complexity regressions reliably, while wall-clock benchmarks are noisy on shared runners. `gpui-base` already has `criterion = "0.8"` as a dev-dependency, so a bench can be added later if wanted.
*Alternative considered:* A criterion benchmark in CI.

**D10. The names are `BlockEditorState` / `BlockEditor`, in `gpui_base::text::block_editor`, with `BlockPosition` / `BlockSelection` as the position types.**
*Rationale:* This avoids confusion with `EditorState` (`InputBaseState<EditorMode>`) and glob-import collisions with `gpui_base::input::{Position, Selection}`.
*Alternative considered:* `RichTextEditorState` with bare `Position` / `Selection`.

**D11. History uses a new `BlockUndoManager` that copies `input::UndoManager`'s semantics: intent-based coalescing (Typing / Backspace / DeleteForward / Atomic), nested begin/commit brackets for IME and batches, and explicit breaks on caret movement and blur. It is not built on `UndoHistory`.**
*Rationale:* `UndoHistory` on main only groups by time or by an explicit flag, so "one keystroke = one undo step" can only be avoided with a timer. `UndoManager` is the behaviour users already get in `Textarea` / `Editor`, but it is tied to the input engine's rope `Change` type.
*Alternative considered:* `UndoHistory<Transaction>` with `group_interval` (the PR's approach and the previous draft's), or making `UndoManager` generic first.

**D12. The public mark API uses `MarkKind`, `MarkSet` (read-only, private fields), and `MarkChange` (builder). `TextMark` stays crate-internal storage.**
*Rationale:* `TextMark` / `LinkMark` aren't exported from `gpui_base` on main and have `pub` fields. Exporting them as they are would break ARCHITECTURE.md invariant 12 and freeze their shape. The API is still keyed on exactly `TextMark`'s attributes, as R6 asks.
*Alternative considered:* Exporting `TextMark` as-is and returning it from `active_marks()` (the previous draft).


**D13. Markdown input rules are a registry: `InputRule` values, with `InputRule::markdown_defaults()` as the default set and `BlockEditorState::with_input_rules` / `set_input_rules` to change it. Each firing is its own undo step, so one undo brings back the literal Markdown.**
The default rules:
- *Heading:* `# `…`###### `.
- *Lists:* `- ` / `* ` / `+ ` and `1. `.
- *Task item:* `[ ] ` / `[x] `, also inside a list item, so `- [ ] ` works.
- *Blockquote:* `> `.
- *Code fence:* ```` ```lang ```` then space or Enter.
- *Horizontal rule:* `---` then Enter.
- *Inline marks:* `` `x` ``, `**x**`, `*x*`, `~~x~~` on the closing delimiter.
- *Link:* `[text](url)` on the closing `)`.
- *Table:* a paragraph line that starts and ends with `|` and has at least one cell (for example `| Name | Age |`) converts on Enter at its end, not on the closing `|`.
  - The cells (marks kept) become the header row, and an empty body row is added with the caret in its first cell.
  - No alignment is set (`---`).
  - One undo step brings back the literal line.

*Keys in tables (D13):*
- *Enter:*
  - In the last row, it appends an empty row with the caret in its first cell.
  - In a body row whose cells are all empty, it deletes the row and leaves the table to a new paragraph after it.
  - In any other row, it moves to the same column of the next row (a caret move, not an edit).
- *Shift+Enter:* a soft line break in the cell, saved as `<br>`. `<br>` in a cell loads back as a line break, so it round-trips.
- *Tab / Shift+Tab:* unchanged.
- *Down past the last row, or Cmd/Ctrl+Enter:* moves to the text block after the table, and creates an empty paragraph when there is none.
- Each edit is one undo step.

Block rules fire only at the start of a paragraph. An empty list turns them all off.
*Rationale:* The rules in TipTap/ProseMirror are composable, but the spike's rules run against our own model and caret. For now the API is a closed set of built-ins, a newtype over a private enum, so custom closure rules can be added later without breaking anything.
*Alternative considered:* Closure-based rules (`Fn(&str) -> Option<Edit>`) now. That is deferred until there is a consumer.
*Status:* The rules (table rule and table keys included) are implemented and tested: typing through `replace_text_in_range` (the platform input path), one undo step each, and disabling.

**D14. Links and tasks UI: Cmd/Ctrl+K opens a popover to add, edit or remove a link. Hovering a link shows its URL. Cmd/Ctrl+click on a link emits a `BlockEditorEvent` (the editor never opens a browser). Task checkboxes are clickable through `BlockEditorState::set_task_checked` (one undo step), and the toolbar has a Task toggle (`EditorCommand::ToggleTask`).**
*Rationale:* The host owns navigation and policy, and the popover is the one in-editor affordance needed for marks that carry data.
*Status:* Implemented and tested.
- *Task checkboxes:* clicking the checkbox, in the strip left of the item's first line, toggles it through `set_task_checked`, as one undo step. `EditorCommand::SetTaskChecked` is public too.
- *Links:* Cmd/Ctrl+K (`EditLink`) emits `BlockEditorEvent::EditLinkRequested { url }`, where `url` is the link at the caret if there is one. The host shows its popover and calls `set_link(Some(url) | None)`. That call adds or changes the link on the selection, or on the whole link at a collapsed caret, or removes it, as one undo step.
- *Hover:* `BlockEditorEvent::LinkHovered` plus `hovered_link()`. The editor draws no tooltip; the host shows the URL (the story shows it in a line above the editor).
- *Cmd/Ctrl+click:* emits `BlockEditorEvent::LinkClicked { url }` and does not move the caret.
- *Story:* has Task and Link toolbar buttons and an inline link bar (input plus Apply / Remove / Cancel).
- *Not done:* the popover is a bar, not anchored at the link, and there is no in-editor tooltip.

**D16. Leaving containers works like leaving an empty list item. Each step is one undo step.**
- *Blockquote, Enter on an empty paragraph:*
  - When it is the last child, it moves after the quote, so Enter twice leaves a quote.
  - In the middle, the quote splits in two around it.
  - A list inside a quote is left first, then the quote on the next Enter.
- *Blockquote, Backspace* at the start of the quote's first paragraph moves that paragraph out before the quote. The quote is removed if it is left empty.
- *List item with an empty last paragraph, after its first child:* this is where the caret lands after leaving a code block in an item. Enter, Shift+Tab, or Backspace at its start turns it into an empty item of its own after that item. From there the usual empty-item rules apply: up one level, then out of the list. The original item becomes tight again when only one child remains.
- *Task checkboxes:* a mouse-down on the checkbox toggles it and stops propagation, so the caret, the selection and drag-select are untouched. `set_task_checked` keeps the selection.
- *Cursor:* `CursorStyle::PointingHand` over a checkbox, and over a link while Cmd/Ctrl is held. The cursor updates on mouse move, so pressing Cmd/Ctrl without moving the mouse does not change it.
*Heading keyboard shortcuts are deferred.*

**D15. Markdown the model does not represent stays literal text and is escaped on save, so a load/save cycle is lossless in meaning. For example, `> [x]` stays a quote with literal text, and HTML and footnote text stay as text.**
*Rationale:* Silent rewrites are worse than visible literals. Escaping ensures that typed `*` or `[` don't become syntax on reload.
*Code blocks:* trailing blank lines are not written on save. Parsers drop them, so writing them would make save drift on every round trip; the model keeps them until reload.
*Future work:* A raw-source fallback block that keeps unmodelled constructs byte-for-byte.
**D17. Copy writes inline marks as Markdown; the editor's own copies are tagged so paste parses them, and external text keeps the conservative import.**
- *Copy:* a text selection serializes each selected leaf as the Markdown of its sliced inline (`serialize::inline_markdown`). That is the same writer as save, so the delimiters are balanced at both cut edges (`` Hello `testing` world `` selected from `ting` to `wo` copies as `` `ting` wo ``), whitespace at an edge stays outside the markers, and nested marks and links stay nested. Leaves are joined with a single newline. Block-node selections and table-cell selections are unchanged (Markdown of the blocks; a table with its columns).
- *Tag:* when every selected leaf is inline text, the clipboard item is written with `ClipboardItem::new_string_with_metadata(markdown, COPY_TAG)`. gpui keeps this metadata (macOS: a pasteboard type; Linux: in the process). Plain text is still the Markdown, so other apps receive Markdown.
- *Paste, tagged:* each line is parsed as inline Markdown (a line that is not exactly one paragraph stays literal), leading and trailing whitespace is kept literal, and lines become blocks as typed. One undo step. In a code block the text is inserted raw. A tagged paste of several lines into a table cell falls back to plain text.
- *Paste, untagged (external or metadata lost):* unchanged. Multi-line text is parsed as block Markdown, and a single line is parsed only if it holds `![` or a backtick. So `2*3*4` from another app stays literal and is not turned into italic.
- *Cmd/Ctrl+Shift+V* still inserts the clipboard text raw, tagged or not.
- *Host paste policy:* `PasteRequest` records whether the item was tagged, so `apply_paste` behaves the same after the host resolves it.
- *Tradeoff:* a copy that loses its metadata (for example through a clipboard manager on Linux) pastes with literal markers instead of marks. Other apps see the Markdown source, so escapes such as `snake\_case` appear in plain text consumers. Parsing every external single line as Markdown was rejected because it can mangle ordinary text.

**Caret (after main #3515):** the default `caret_width` is the input's shared `CURSOR_WIDTH` (1px on every platform), and the editor paints its caret snapped to whole device pixels with the input's `snap_caret_to_device_pixels`. Callers can still override it with `with_caret_width`.

**Decisions taken during the UI spike (answered by Richard, 14 Oct 2026):**
- **Scroll handle:** the owner passes its scroll container's `ScrollHandle` (`set_scroll_handle`); the editor does not own a scroll container.
- **Culling / Phase 5:** light culling first (top-level units, chunked long top-level lists, overscan, pinned selection/IME units), then Phase 5 taken partway (prefix-sum index, cached units, layout reuse, top-level a11y roles) and stopped; remaining Phase 5 items stay open.
- **Paste:** Cmd/Ctrl+V pastes multi-block Markdown as blocks in one undo step; single-line text is plain unless it holds an image (`![`) or an inline code span (`` ` ``). Cmd/Ctrl+Shift+V always pastes plain text.
- **Arc sharing of cached nodes:** deferred; reused subtrees are cloned with `Arc`-shared `InlineState`. Sharing whole nodes needs `Arc` children in the shared `BlockNode` and touches `TextView`.
- **Feature forwarding:** `gpui-kit/block-editor` → `gpui-base/block-editor` is OK under D2.
- **Projection:** direct model→`BlockNode` with a per-node revision cache (§7.3), equivalence-tested against `parse(serialize(doc))` on `BlockNode` trees.
- **List exit:** Enter in an empty item exits the list (top-level item → paragraph after the list, splitting it mid-list; nested item outdents), one undo step.
- **Code blocks (4b):** D7 as written; fence input rules, Backspace at the start of a non-empty code block moves the caret to the previous block without merging.
- **Table cells (4c):** cells are caret leaves, `BlockSelection::Cells` by ids; no block handle, so whole-table selection is via Backspace/Delete beside the table or a selection across its edge.
- **Unspecified alignment:** `---` columns stay `---` (`ColumnumnAlign::None`, rendered left), so tables round-trip losslessly.
- **Inline atoms (4d):** #3113 semantics (one unit, copy as Markdown), rendered through `InlineFlow` with piece reporting.
- **Inline mark input rules:** `` `x` ``, `**x**`, `*x*`, `~~x~~` convert on the closing delimiter as their own undo step; other marks never apply inside inline code.
- **Toolchain:** no MSRV pin; `usize::isolate_lowest_one` is used and needs a current stable rustc.

---

## Appendix A: Verification notes

**Trees checked** (cloned locally, read-only; nothing pushed or posted):
- `main` @ `369f7a8f5a6602da980f008cf726467588ad517f` (2026-10-12 21:27 AEDT, "button: Keep the hover style registered…").
- PR head `refs/pull/3049/head` @ `47141037d3b680af6f940aef2e1d9359d00ab0a4`. Merge-base `3f1dda6ee9256ba317b19552999d130e3d078159`. Main is 278 commits ahead; the PR is 2 ahead.

**Build:**
- `cargo check -p gpui-base --lib` passes on both trees. This needed rustup stable, because the box's rustc 1.85.1 is too old for the dependencies, and Linux dev packages for fontconfig, freetype, xkbcommon, wayland, and xcb.
- `cargo test -p gpui-base --lib text::editor` on the PR: 26 passed (24 existing tests plus 2 temporary probes).
- On main I ran two temporary probe tests (parser round-trip, `MarkdownSource`) in a scratch worktree that has since been deleted. Nothing was committed.

**Checked, and what changed in the design:**

| Item | Finding | Design change |
|---|---|---|
| `PaneTree` storage | Owned recursive `PaneNode` tree; `NodeId(u64)` from a global `AtomicU64` (`next_node_id`); DFS `find_node` (`dock/layout/{tree,node}.rs`) | D4 rationale corrected; D1 switched to process-unique ids and dropped the epoch counter |
| `<u>` / `==` / `<mark>` parsing | `<u>` and `<mark>` parse (`inline_html_mark` in `format/markdown.rs`); `==x==` stays literal | D5 flipped: highlight is serialized as `<mark>`, not `==`; no "disable if unsupported" fallback |
| `Paragraph::to_markdown` on main | Plain path overwrites per mark: `***x***`→`*x*`, `**b*c*d**`→`**bcd**`, `<u>x</u>`→`x` | Phase 0d confirmed |
| `MarkdownSource` | Already nests and merges correctly for non-overlapping runs and emits `<u>`; lacks whitespace hoisting and escaping; emits `==` | Serializer now builds on `MarkdownSource` instead of being new (§4.6) |
| Blink and focus | `InputBaseState::new_in_mode`: `on_focus`/`on_blur` start/stop, `observe_window_activation`, `intercept_keystrokes` pause, `observe(blink)` | §7.1 copies it exactly; hedge removed |
| Accessibility | `gpui-component` `Input`: `MultilineTextInput` for multi-line; `aria_value` only if `window.is_a11y_active()`; base `InputBase` defaults to `TextInput` | D8 made concrete. Implemented: the root is `Role::MultilineTextInput` with `aria_value` = the document's plain text (leaves one per line, atoms dropped), built only while accessibility is active (and in tests) and cached per revision. Phase 5: every laid-out top-level block gets its own node (`Heading` + level, `Paragraph`, `List`, `Code`, `Table`, `Blockquote`, `Splitter` for rules); nested list items, table rows/cells and inline roles are not exposed yet, and culled blocks have no node. Screen-reader behaviour is unverified at runtime |
| `UndoHistory` | Time or explicit grouping only; `T: Clone`; no in-tree users besides the PR. The input engine uses `UndoManager` with `EditIntent` coalescing | New D11; §5.2 rewritten |
| `InputEditorStyle` / `EditorState` | `pub` fields, `resolved(&SemanticThemeTokens)`, injected by `set_editor_style`; `EditorState = InputBaseState<EditorMode>` | Style sketch renamed to `resolved` / `set_editor_style`, keeping private fields; naming rationale updated |
| Seam drift | PR seam patch conflicts in `inline.rs`, `inline_flow.rs`, `node.rs`, `text_view.rs`; `text/` changed by about 11.9k lines (46 commits). Main has `RangeHighlightFrame` / `StreamFadeFrame`, `Inline::range_backgrounds`, `TextLeafKey`, `Paragraph::render_cache` | Phase 0a is a re-implementation; selection painted through `range_backgrounds`; D3 rationale updated |
| Name collisions | `gpui_base::input::{Position, Selection}` already exported; `TextMark` / `LinkMark` not exported and have `pub` fields; table alignment type is `ColumnumnAlign` | Renamed to `BlockPosition` / `BlockSelection` (D10); new D12; `BlockKind` made payload-light |
| `BlockNode` variants | `Custom { .. }` struct variant; no HTML variant; `Break` and `Definition` exist | `BlockKind` / `BlockContent` aligned; HTML non-goal reworded |
| `TextViewDefaults` | Fields private on main; read only in `text_view.rs`; public API has setters and two readers | `pub(crate)` readers retained as the plan |
| `word_range_at` | Still in `crate::text::selection` on main; a second copy is in `crate::text_boundary` | §2 corrected (an earlier draft wrongly said it had moved) |
| Examples | `examples/markdown/src/main.rs` is 1866 lines plus `mention.rs`; `fixtures/test.md` is 320 lines | 0c must be redone; generated fixture added |
| R1 / R2 / R3 repro on the PR | `.unwrap()` at `model.rs:296`; 2 of 3 blocks addressable; `**ab****c****d**` with 3 nodes (the review said four), which main's parser reads back as bold `abcd` | §2, §4.3, §4.6 reworded to match |

**Verified by the model spike** (branch `block-editor-model-spike` on `wabzqem/gpui-component`, based on `369f7a8f`):
- `cargo test -p gpui-base --lib` passes with and without `--features block-editor` (1335 and 1387 tests); `cargo clippy -p gpui-base --lib --features block-editor -- -D warnings` and `cargo fmt --check` are clean; the build without the feature is unchanged.
- The R1/R2/R3/R6/R7 repros and main's mark-loss cases pass as named tests; every `BlockNode` variant round-trips or is a documented atom (Appendix B).
- One keystroke in a 1000-block document records exactly one `ReplaceText` step, and the `OrderIndex` is rebuilt only on structural steps (`cfg(test)` counter).
- `examples/fixtures/test.md` reaches a fixpoint after one save.

**Still unverified:**
- How non-gpui-kit CommonMark parsers read `**ab****c****d**`. Only our parser was tested.
- Which "1000-line fixture" R4 refers to. None was found in either tree.
- Platform screen-reader behaviour for a single `MultilineTextInput` holding a long document. This needs runtime testing.
- Runtime UI behaviour in general (IME, frame timing). Everything above comes from source reading and headless tests.


---

## Appendix B: What the model spike changed

The spike (`crates/base/src/text/block_editor/`, feature `block-editor`, draft PR from `wabzqem:block-editor-model-spike`) implements §4–§5 headless. All the outline's named tests pass, plus seeded property tests (checked locally with 3000 seeds; 150 in CI). Differences from the text above:

| Area | Design | Spike | Status |
|---|---|---|---|
| Steps (§5.1) | Eight step kinds with `invert(&before)` | Four primitives; others are composites; `apply` returns the inverse | Simplification, design text updated |
| Structural normalize (§5.1) | Four rules | Plus "merge adjacent same-type lists" | Required by the parser |
| Serializer (§4.6, D5) | Hoisting, escaping, `<mark>` | Plus flanking-safe HTML fallback and code-innermost | Required for round-trip; still only `MarkdownSource` options, defaults unchanged |
| Reference links | Not covered | Resolved to inline links on import; unresolved references and footnote links lose their link mark | Canonicalization, documented |
| Select-all + type (§4.4) | Removes everything covered | Whole-document selection becomes one empty paragraph | Matches the R2 expectation (`x`, one Paragraph) |
| Inline mark input rules | — | Typing a closing `` ` ``, `**`, `*` or `~~` after an opening one in the same leaf makes the text between code, bold, italic or strikethrough and drops the delimiters, as its own undo group (undo restores the literal delimiters); typing on is outside the span. No rule inside inline code, around space-padded content, or across an atom. Other marks never apply inside inline code (only a link may wrap it). Ctrl/Cmd-E and story toolbar buttons (B, I, S, `code`) toggle marks; a single-line paste holding `` `x` `` pastes as code | Done |
| Code blocks / atoms (§4.3) | Offsets into code text | Phase 4b landed: offsets are UTF-8 byte offsets into `BlockContent::Plain` (char boundaries), edited via `Step::ReplacePlain`. Fence input rule (```` ```lang ```` + Enter/space), Enter = newline, Tab/Shift+Tab = 4-space indent, exit by Enter on trailing empty line or Down past the end, Backspace in empty block → paragraph, Backspace at the start of a non-empty block (Delete at its end) moves the caret to the neighbouring block without merging, marks no-op. Trailing empty lines of code have no Markdown form and are dropped on save (documented lossiness) | Done |
| `source_offset` / `position_at_source_offset` (D1) | Proposed | Not implemented | Deferred |
| `BlockSelection::Cells`, `InputRule`s | Proposed / stretch | `BlockSelection::Cells { anchor, head }` holds two cell `BlockId`s (not `(row, col)` pairs: ids survive row/column edits); the rectangle is derived from the grid. Input rules are hard-coded in the view (headings, lists, code fence), not a pluggable `InputRule` registry | Cells done; registry deferred |
| `BlockKind::Image` | Block kind | Images are inline atoms on main; the kind is reserved and unused | Keep or drop in review |
| `ResolvedPosition` | `SmallVec<[BlockRef; 8]>` path | `Vec<BlockId>` path | Equivalent |

**Documented lossiness found by the property tests** (canonical output, not data corruption): empty paragraphs have no Markdown form and disappear on save; an empty paragraph between two same-type lists therefore merges them on reload; edge whitespace of a paragraph and whitespace-only mark runs are normalized; list tightness can change once on the first save, after which output is a fixpoint; `Break` (non-HTML) and `Unknown` blocks serialize to nothing.

### What the UI spike broke

The UI spike (branch `block-editor-ui-spike` on `wabzqem/gpui-component`, on top of the model spike; `BlockEditorState`, `BlockEditor`, story `BlockEditor` behind the `block-editor` feature) implements a minimal Phase 3: paragraphs, headings, bullet and ordered lists, bold/italic/code shortcuts, the input rules `# `, `- ` and `1. `, caret navigation across blocks, cross-block selection, IME, and undo/redo through the model. After review it was reworked to render through gpui-kit's own text renderer (D3). Where it still departs from §6–§8:

| Area | Design | UI spike | Status |
|---|---|---|---|
| Rendering path (§7.2, D3) | Project into `BlockNode` and render through `Inline` / `InlineFlow` with a seam and `editor_key` | **Follows D3.** Phase 0a seam (own commit; compiled only with the `block-editor` feature, and it reads the thread-local sink before taking the `InlineState` lock, so plain `TextView`s pay nothing): `text/editor_seam.rs` defines a crate-internal `EditorSink` (`report(key, TextLayout)`, `backgrounds(key)`) installed thread-locally by the editor's root element during prepaint. `InlineState` gained `editor_key: Option<u64>`; `Inline::prepaint` reports its layout when a key and a sink are present. `TextView` never sets a key, so read-only rendering is unchanged. Text leaves, including code blocks and table cells, are keyed by `BlockId`. Blocks render via the normal `render_block` with `TextViewStyle` from the theme (no hard-coded sizes) | Seam is narrower than #3049's `InlineInteraction`: it only collects layouts and supplies backgrounds; hit-testing, carets and vertical motion live in `block_editor/frame.rs` over the reported `TextLayout`s |
| Projection (§7.3) | Model → `BlockNode` directly, re-project changed blocks | **Follows §7.3.** Direct projection with a per-node cache (blocks and table rows) keyed by revisions that bubble to ancestors. Counter-checked: a top-level edit rebuilds 1 node, an edit inside a 1000-item list 3 (paragraph, item, list), a cell edit 2 (table, row) | Reused subtrees are cloned with `Arc`-shared state. Sharing nodes outright would need `Arc` children in the shared `BlockNode`, which also changes `TextView`; recorded as a follow-up. Table rows have no layout key (they have no text) |
| Selection painting (§7.1) | Through `Inline::range_backgrounds` | Through `Inline::range_backgrounds`, fed by the sink | Matches |
| Frame sink (§7.1) | Frame-local builder, swapped at the next `render` | `FrameSink` collected in prepaint, turned into `FrameLayout` (leaf geometry, `hit_test`, `caret_bounds`, `vertical`, `line_edge`) | Equivalent; paint never updates the entity |
| Events (§6) | `Changed(ChangeSummary)`, `SelectionChanged(SelectionSummary)` | `Changed { revision, origin }`, payload-free `SelectionChanged`; no-op commands emit nothing | Summaries deferred |
| Home / End | Visual line start/end | Visual line start/end via `FrameLayout::line_edge`; End at a soft wrap stays before the wrap point (the shared index would draw on the next line) | `Affinity` still unused (not needed for this) |
| IME (§2) | Port #3049's handler over `BlockPosition` | The IME document is the caret leaf. An undo group opens only when marked text is actually inserted (fixes the leaked group from empty or cancelled marks); a cancel removes the marked text; composing over a cross-block selection deletes the selection first; Cut ends a composition first | Accessibility text ranges need the full document (D8) |
| Tables | Block frame, node selection; cell editing Phase 4 | Phase 4c: cells are caret leaves. Click places the caret in a cell; typing, marks and IME work; Enter in a cell does nothing. Tab/Shift+Tab move to the next/previous cell selecting its text, Tab in the last cell adds a row; Left/Right cross cells in document order, Up/Down go to the cell above/below (frame hit-testing and vertical motion pick the horizontally nearest leaf among side-by-side cells). Dragging or Shift+arrows from one cell into another selects `BlockSelection::Cells` (painted per cell; copy writes a Markdown table of the rectangle, first selected row as header; Backspace/Delete/typing clear the cells). Whole-table node selection: Backspace/Delete next to a table, or a text selection across its edge; there is no block handle yet, so a click never selects the table. Commands: `InsertTable { rows, cols }`, `EditTable(TableEdit::{AddRowAbove, AddRowBelow, RemoveRow, AddColumnLeft, AddColumnRight, RemoveColumn})` (removing the last row/column removes the table; removing the header row promotes the next); story toolbar buttons. Alignment round-trips, including unspecified (`---` stays `---`; `ColumnumnAlign::None`) | Done |
| Atoms / custom nodes | Block frame with own renderer | Superseded by Phase 4d (inline atoms row below): images and custom inline nodes get caret geometry and click-to-select | Done |
| Lists | — | Enter in an empty list item exits the list: a top-level item becomes a paragraph after the list (splitting it when mid-list, `exit_list`), a nested item outdents one level. One transaction, one undo step (`tests/commands.rs`) | Done |
| Feature plumbing (D2) | No `gpui-component` re-export until Phase 3 | `gpui-kit` gained a `block-editor` feature forwarding to `gpui-base/block-editor` so the story can reach it | Decision for review |
| Inline atoms (Phase 4d, #3113) | Atomic caret, selection, deletion; clipboard holds the token's text | Model: an atom is one U+FFFC (3 bytes) with its own run, so Left/Right, Shift+arrows and Backspace treat it as one unit and offsets inside it are invalid; typing beside it never merges into it; undo restores it. Copy writes an atom as its Markdown (`![alt](url)`, marks dropped), and a single-line paste holding `![` re-parses, so cut + paste round-trips. Rendering: the seam now covers `InlineFlow` too. A flow rendering an editor leaf reports *pieces* (`EditorSink::begin_pieces` / `report_text_piece` / `report_box_piece`): each single-line text fragment's `TextLayout` with its leaf byte start, and each image / custom node's box over its U+FFFC; the fragments take their slice of the selection backgrounds. `frame.rs` hit-tests pieces (line band, then nearest piece; an atom's left/right half gives its start/end), so mixed paragraphs, image-only paragraphs (forced onto the flow in the editor) and inline-code paragraphs get carets, clicks and up/down, and a click on an image selects it whole. Inline code keeps its pill padding (visual parity with `TextView`) with caret geometry from the padded fragment | Done (no `ActivateToken` / on-click hook for atoms) |
| Virtualization | Phase 5 | Phase 5 started. Cull units (top-level blocks, 32-item chunks of long top-level lists) are cached per document revision, and their heights live in a Fenwick prefix-sum index (`cull.rs` `HeightIndex`): a frame binary-searches the visible range, lays out only it plus the pinned units (selection ends, IME), and sizes the two or more spacers from prefix sums, so a steady frame no longer walks every block (counter-tested: no rebuild on selection changes, one per edit). Measurements update one index entry. Selection backgrounds now cost O(selected leaves): leaf lookups go through the order index's ranks and atom checks through the document, with no scans of the leaf list. Element state for scrolled-back blocks: culled blocks keep their cached node (with its `InlineState`) and their shaped text in the retained-layout table, so scrolling back reshapes and rebuilds nothing (counter-tested for plain paragraphs; `InlineFlow` paragraphs keep their fragment states in GPUI element state, which is dropped while culled, so they reshape). Remaining: chunking long tables (a chunk would need the full table's column widths and must not restyle its first row as a header) and long lists nested in other blocks; accessibility nodes for nested list items and table rows/cells (they need roles inside the shared `TextView` renderers) | Partial |

**Story.** Toolbar for B / I / S / `code`, Text / H1–H3 / Bullets / Numbered, Code, Table / ±Row / ±Col, a sample document and a 1200-block generator. The Markdown preview is cached and refreshed only on `Changed` or load (it was re-serialized every frame).

**Perf note (UI spike).** Headless test platform. "Frame" is one forced full redraw. "Keystroke" is one typed character plus redraws; the harness draws three frames per keystroke.

| | Pre-D3 own shaping | D3 + direct projection, no culling | With culling |
|---|---|---|---|
| 1200 blocks, frame, release | — | 16–25 ms | ~1–1.8 ms |
| 1200 blocks, keystroke, release | ~16 ms | 70–120 ms | ~4 ms |
| 1200 blocks, keystroke, debug | ~34 ms | 190–245 ms | ~13–18 ms |
| 1000-item list, frame, debug | — | 45–59 ms | ~4–5 ms |
| 1000-item list, keystroke, release | — | ~77 ms | ~9–11 ms |
| 1000-item list, keystroke, debug | — | 180–220 ms | ~22–31 ms |

*Profile before culling (release, 1200 blocks, per frame):* element creation 4–10 ms, `request_layout` 4–6 ms, prepaint 4–6 ms, paint 3–4 ms; the model edit took 0.8 ms and the snapshot 0.4 ms. Shaping was already reused (GPUI's line-layout cache) and projection is O(changed path), so the cost was per-element work for off-screen blocks.

*Culling (`block_editor/cull.rs`).* This started as light virtualization before Phase 5 (Phase 5 additions are in the Virtualization row above). The cull units are the top-level blocks, and long top-level lists are split into 32-item chunks. Each chunk renders as its own list that continues the numbering, with no gap between chunks. Units outside one viewport of the visible area (the content mask at prepaint) are replaced by spacers. The spacers are sized from measured top-to-next-top distances, or a 28 px estimate before a unit has been measured. Units holding the selection ends or the IME composition are always laid out. When the visible area leaves the laid-out span, another frame is requested.
- Up/down into a culled leaf falls back to the model: the adjacent caret leaf at offset 0 or at its end.
- With `BlockEditorState::set_scroll_handle`, the caret is scrolled into view once its block is laid out.
- Overscan is two viewports each way (at least 600 px), so ordinary fast scrolling stays inside laid-out blocks while the next frame catches up. A fling of more than two viewports in one frame can still show a blank spacer for one frame.
- Spacers are exact: block gaps are padding inside each block's element, so the top-to-next-top distance is the whole slot. A test checks that a block far down sits at the same y with and without culling.
- *Scroll handle (decision):* the owner passes its scroll container's `ScrollHandle` (`set_scroll_handle`); the editor does not own a scroll container. Without a handle, caret-into-view is a no-op (tested).
- Nested long lists are not chunked.
- *Virtualization (Phase 5):* cull units are cached per document revision and the visible range is found through a prefix-sum height index (O(log n) per frame); nested lists and long tables are not chunked yet.

*Paste (decision; the editor's own tagged copies are covered by D17).* Cmd/Ctrl+V: single-line text, or text that parses to a single plain paragraph, is inserted as plain text inline (a single line holding `![` or a `` `code` `` span is parsed, so images and inline code paste back). Multi-block Markdown becomes blocks in one undo step: a leading paragraph merges into the text before the caret, a trailing one into the text after it, and the blocks between go between the two halves. Cmd/Ctrl+Shift+V: plain text (lines become paragraphs).
