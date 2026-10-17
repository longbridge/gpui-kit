//! `BlockEditorState`: the entity around [`BlockEditorModel`] (design §6).

use super::input_rule::RuleKind;
use std::sync::Arc;
use std::{cell::RefCell, ops::Range, rc::Rc};

use gpui::{
    App, AppContext, Bounds, ClipboardItem, Context, Entity, EntityInputHandler, EventEmitter,
    FocusHandle, Focusable, KeyBinding, Pixels, Point, SharedString, Subscription, UTF16Selection,
    Window, actions,
};

use super::{
    command::{CommandError, EditorCommand},
    editor::BlockEditorModel,
    frame::FrameLayout,
    id::BlockId,
    kind::BlockKind,
    marks::MarkKind,
    position::BlockPosition,
    projection::{ProjectionCache, Snapshot},
    selection::BlockSelection,
    style::BlockEditorStyle,
    transaction::EditOrigin,
};
use crate::{input::blink_cursor::BlinkCursor, text::MarkdownExtensions};

pub(crate) const CONTEXT: &str = "BlockEditor";

actions!(
    block_editor,
    [
        Backspace,
        Delete,
        Enter,
        StructuralEnter,
        MoveLeft,
        MoveRight,
        MoveUp,
        MoveDown,
        MoveHome,
        MoveEnd,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectHome,
        SelectEnd,
        SelectAll,
        MoveWordLeft,
        MoveWordRight,
        SelectWordLeft,
        SelectWordRight,
        Undo,
        Redo,
        ToggleBold,
        ToggleItalic,
        EditLink,
        SoftBreak,
        ExitBlock,
        ToggleCode,
        Indent,
        Outdent,
        Copy,
        Cut,
        Paste,
        PastePlain,
    ]
);

pub(crate) fn init(cx: &mut App) {
    let ctx = Some(CONTEXT);
    #[cfg(target_os = "macos")]
    let m = "cmd";
    #[cfg(not(target_os = "macos"))]
    let m = "ctrl";
    // Word motion: Option on macOS, Ctrl elsewhere.
    #[cfg(target_os = "macos")]
    let w = "alt";
    #[cfg(not(target_os = "macos"))]
    let w = "ctrl";
    cx.bind_keys(vec![
        KeyBinding::new("backspace", Backspace, ctx),
        KeyBinding::new("delete", Delete, ctx),
        KeyBinding::new("enter", Enter, ctx),
        KeyBinding::new("left", MoveLeft, ctx),
        KeyBinding::new("right", MoveRight, ctx),
        KeyBinding::new("up", MoveUp, ctx),
        KeyBinding::new("down", MoveDown, ctx),
        KeyBinding::new("home", MoveHome, ctx),
        KeyBinding::new("end", MoveEnd, ctx),
        KeyBinding::new(&format!("{w}-left"), MoveWordLeft, ctx),
        KeyBinding::new(&format!("{w}-right"), MoveWordRight, ctx),
        KeyBinding::new(&format!("{w}-shift-left"), SelectWordLeft, ctx),
        KeyBinding::new(&format!("{w}-shift-right"), SelectWordRight, ctx),
        KeyBinding::new("shift-left", SelectLeft, ctx),
        KeyBinding::new("shift-right", SelectRight, ctx),
        KeyBinding::new("shift-up", SelectUp, ctx),
        KeyBinding::new("shift-down", SelectDown, ctx),
        KeyBinding::new("shift-home", SelectHome, ctx),
        KeyBinding::new("shift-end", SelectEnd, ctx),
        KeyBinding::new("tab", Indent, ctx),
        KeyBinding::new("shift-tab", Outdent, ctx),
        KeyBinding::new(&format!("{m}-a"), SelectAll, ctx),
        KeyBinding::new(&format!("{m}-z"), Undo, ctx),
        KeyBinding::new(&format!("{m}-shift-z"), Redo, ctx),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-y", Redo, ctx),
        KeyBinding::new(&format!("{m}-b"), ToggleBold, ctx),
        KeyBinding::new(&format!("{m}-i"), ToggleItalic, ctx),
        KeyBinding::new(&format!("{m}-e"), ToggleCode, ctx),
        KeyBinding::new(&format!("{m}-k"), EditLink, ctx),
        KeyBinding::new("shift-enter", SoftBreak, ctx),
        KeyBinding::new(&format!("{m}-enter"), ExitBlock, ctx),
        KeyBinding::new(&format!("{m}-c"), Copy, ctx),
        KeyBinding::new(&format!("{m}-x"), Cut, ctx),
        KeyBinding::new(&format!("{m}-v"), Paste, ctx),
        KeyBinding::new(&format!("{m}-shift-v"), PastePlain, ctx),
        // Composer overrides only; standalone Newline/ModifiedEnter keep
        // Shift+Enter soft breaks and the platform secondary-Enter command.
        KeyBinding::new(
            "shift-enter",
            StructuralEnter,
            Some("BlockEditor && submit == enter"),
        ),
        KeyBinding::new(
            "ctrl-enter",
            SoftBreak,
            Some("BlockEditor && submit == enter"),
        ),
    ]);
}

/// Clipboard metadata that marks a copy as the editor's own inline Markdown
/// (D17). Paste parses tagged text as inline Markdown; untagged text keeps
/// the conservative import.
pub(crate) const COPY_TAG: &str = "gpui-kit/block-editor/inline-markdown/v1";

/// Events emitted by [`BlockEditorState`].
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum BlockEditorEvent {
    /// Once per committed transaction, including undo and redo.
    Changed {
        revision: u64,
        origin: EditOrigin,
    },
    SelectionChanged,
    /// Cmd/Ctrl+K: the host should show its link popover (D14) for the
    /// selection or the link at the caret, then call
    /// [`BlockEditorState::set_link`]. `url` is the current link, if any.
    EditLinkRequested {
        url: Option<SharedString>,
    },
    /// Cmd/Ctrl+click on a link. The editor never opens a browser.
    LinkClicked {
        url: SharedString,
    },
    /// The link under the mouse changed (`None` when it left a link).
    LinkHovered {
        url: Option<SharedString>,
    },
    Focus,
    Blur,
    /// Enter requested submission; emitted only outside IME composition. The
    /// owner applies completion/picker and domain-send policy, in that order.
    SubmitRequested,
    /// Semantic paste interception, shared by keyboard and native actions.
    /// The owner must resolve this request with `apply_paste`; stale requests
    /// are rejected instead of pasting into a later selection/document.
    PasteRequested {
        request: PasteRequest,
        item: ClipboardItem,
    },
}

/// Which Enter action requests submission. Newline is the document-editor default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubmitPolicy {
    #[default]
    Newline,
    /// Enter submits; Shift+Enter edits structurally and Ctrl+Enter inserts
    /// a cell soft break. On macOS Cmd+Enter also submits.
    Enter,
    ModifiedEnter,
}

/// Whether paste uses editor defaults or is first offered to the host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClipboardPolicy {
    #[default]
    Editor,
    Host,
}

/// An intercepted semantic paste tied to the editor's current document and
/// selection. Resolve once; no synthetic key events or OS clipboard access is
/// needed when delivering an explicit `ClipboardItem`.
#[derive(Clone, Debug)]
pub struct PasteRequest {
    range: super::CapturedRange,
    plain: bool,
    /// The item was the editor's own tagged inline-Markdown copy (D17).
    markdown: bool,
    sequence: u64,
}

/// IME composition in progress: the marked byte range in one leaf.
#[derive(Clone, Debug)]
struct Marked {
    block: BlockId,
    range: Range<usize>,
}

pub struct BlockEditorState {
    pub(crate) model: BlockEditorModel,
    pub(crate) focus_handle: FocusHandle,
    pub(crate) blink_cursor: Entity<BlinkCursor>,
    pub(crate) style: BlockEditorStyle,
    pub(crate) projection: ProjectionCache,
    /// Last complete frame, written by the element after prepaint.
    pub(crate) frame: Rc<RefCell<FrameLayout>>,
    pub(crate) cull: Rc<RefCell<super::cull::CullState>>,
    /// The accessible value (document text), cached per revision (D8).
    pub(crate) a11y_value: RefCell<Option<(u64, SharedString)>>,
    /// Scroll container to keep the caret in view (see `set_scroll_handle`).
    pub(crate) scroll_handle: Option<gpui::ScrollHandle>,
    /// The caret moved and should be scrolled into view after layout.
    pub(crate) reveal: std::cell::Cell<bool>,
    marked: Option<Marked>,
    /// An IME composition opened an undo group that is not committed yet.
    composition_open: bool,
    /// Remembered x for repeated up/down.
    goal_x: Option<Pixels>,
    pub(crate) dragging: bool,
    readonly: bool,
    image_policy: super::ImagePolicy,
    submit_policy: SubmitPolicy,
    clipboard_policy: ClipboardPolicy,
    paste_sequence: u64,
    pending_paste: Option<u64>,
    input_rules: Vec<super::input_rule::InputRule>,
    input_rule_filter: Option<Arc<dyn Fn(super::InputRule, &str) -> bool + Send + Sync>>,
    hovered_link: Option<SharedString>,
    /// The mouse is over a task checkbox, or a link with Cmd/Ctrl held.
    pub(crate) pointer_hand: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<BlockEditorEvent> for BlockEditorState {}

impl Focusable for BlockEditorState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BlockEditorState {
    /// Create an editor holding one empty paragraph.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let blink_cursor = cx.new(|_| BlinkCursor::new());
        // Focus and blink wiring as in `InputBaseState::new_in_mode` (§7.1).
        let subscriptions = vec![
            cx.observe(&blink_cursor, |_, _, cx| cx.notify()),
            cx.on_focus(&focus_handle, window, |this, _, cx| {
                this.blink_cursor.update(cx, |b, cx| b.start(cx));
                cx.emit(BlockEditorEvent::Focus);
            }),
            cx.on_blur(&focus_handle, window, |this, _, cx| {
                this.blink_cursor.update(cx, |b, cx| b.stop(cx));
                this.model.break_coalescing();
                this.end_composition();
                cx.emit(BlockEditorEvent::Blur);
            }),
            cx.observe_window_activation(window, |this, window, cx| {
                if window.is_window_active() && this.focus_handle.is_focused(window) {
                    this.blink_cursor.update(cx, |b, cx| b.start(cx));
                }
            }),
        ];
        let model = BlockEditorModel::from_markdown("", MarkdownExtensions::default())
            .unwrap_or_else(|_| unreachable!("empty source always parses"));
        Self {
            model,
            focus_handle,
            blink_cursor,
            style: BlockEditorStyle::default(),
            projection: ProjectionCache::default(),
            frame: Rc::default(),
            cull: Rc::default(),
            a11y_value: RefCell::default(),
            scroll_handle: Some(gpui::ScrollHandle::new()),
            reveal: Default::default(),
            marked: None,
            composition_open: false,
            goal_x: None,
            dragging: false,
            readonly: false,
            image_policy: super::ImagePolicy::default(),
            submit_policy: SubmitPolicy::default(),
            clipboard_policy: ClipboardPolicy::default(),
            paste_sequence: 0,
            pending_paste: None,
            input_rules: super::input_rule::InputRule::markdown_defaults(),
            input_rule_filter: None,
            hovered_link: None,
            pointer_hand: false,
            _subscriptions: subscriptions,
        }
    }

    pub fn with_submit_policy(mut self, policy: SubmitPolicy) -> Self {
        self.submit_policy = policy;
        self
    }
    /// Replace the policy; notify the editor after changing it so the
    /// retained view's policy-scoped key context is refreshed.
    pub fn set_submit_policy(&mut self, policy: SubmitPolicy) {
        self.submit_policy = policy;
    }
    pub(crate) fn key_context(&self) -> &'static str {
        if self.submit_policy == SubmitPolicy::Enter {
            "BlockEditor submit=enter"
        } else {
            CONTEXT
        }
    }
    pub fn with_clipboard_policy(mut self, policy: ClipboardPolicy) -> Self {
        self.clipboard_policy = policy;
        self
    }
    pub fn set_clipboard_policy(&mut self, policy: ClipboardPolicy) {
        self.clipboard_policy = policy;
    }
    /// The retained scroll owner; content-sized wrappers share this handle.
    pub fn scroll_handle(&self) -> Option<gpui::ScrollHandle> {
        self.scroll_handle.clone()
    }

    /// Builder: the Markdown input rules to run while typing (D13). The
    /// default is [`InputRule::markdown_defaults`](super::InputRule); pass an
    /// empty list to type Markdown literally.
    pub fn with_input_rules(
        mut self,
        rules: impl IntoIterator<Item = super::input_rule::InputRule>,
    ) -> Self {
        self.input_rules = rules.into_iter().collect();
        self
    }

    /// Pure policy invoked before an enabled input rule rewrites typed content.
    /// Return false when a host workflow (e.g. completion) owns the same sigil.
    /// Receives leaf-native text before the caret, never serialized offsets;
    /// must not perform I/O or re-enter the editor. Defaults to allowing rules.
    pub fn with_input_rule_filter(
        mut self,
        filter: impl Fn(super::InputRule, &str) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.input_rule_filter = Some(Arc::new(filter));
        self
    }

    /// Replace the input rules (see [`Self::with_input_rules`]).
    pub fn set_input_rules(
        &mut self,
        rules: impl IntoIterator<Item = super::input_rule::InputRule>,
    ) {
        self.input_rules = rules.into_iter().collect();
    }

    /// Check or uncheck the task item holding `block` (a list item or its
    /// text leaf); one undo step (D14). The selection is kept.
    pub fn set_task_checked(&mut self, block: BlockId, checked: bool, cx: &mut Context<Self>) {
        if self.readonly {
            return;
        }
        let leaf = if self.model.doc.inline(block).is_some() {
            block
        } else {
            match self.model.doc.children(block).first() {
                Some(child) => *child,
                None => return,
            }
        };
        self.model.break_coalescing();
        self.edit(
            EditOrigin::Command("set_task_checked"),
            |m| m.set_item_checked(leaf, checked).map(|_| ()),
            cx,
        );
        self.model.break_coalescing();
    }

    fn rule_on(&self, kind: super::input_rule::RuleKind) -> bool {
        self.input_rules.iter().any(|rule| rule.0 == kind)
            && self.input_rule_filter.as_ref().is_none_or(|filter| {
                filter(
                    super::InputRule(kind),
                    self.model.caret_prefix().unwrap_or_default(),
                )
            })
    }

    /// Builder: initial Markdown.
    pub fn with_source(mut self, source: &str) -> Self {
        let _ = self.model.set_source(source);
        self.projection.invalidate();
        self
    }

    /// Replace the content. No `Changed` event; clears history.
    pub fn set_source(&mut self, source: &str, cx: &mut Context<Self>) -> Result<(), SharedString> {
        self.model.set_source(source)?;
        self.marked = None;
        self.composition_open = false;
        self.pending_paste = None;
        self.projection.invalidate();
        cx.notify();
        Ok(())
    }

    /// The document as canonical Markdown.
    pub fn source(&self) -> String {
        self.model.source()
    }

    /// Shared canonical source, materialized at most once per revision.
    pub fn source_snapshot(&self) -> super::SourceSnapshot {
        self.model.source_snapshot()
    }

    pub fn capture_caret(&self) -> Result<super::CapturedRange, super::CapturedRangeError> {
        self.model.capture_caret()
    }
    pub fn capture_selection(&self) -> Result<super::CapturedRange, super::CapturedRangeError> {
        self.model.capture_selection()
    }
    pub fn capture_suffix(
        &self,
        len: usize,
    ) -> Result<super::CapturedRange, super::CapturedRangeError> {
        self.model.capture_suffix(len)
    }
    pub fn capture_leaf_range(
        &self,
        range: std::ops::Range<usize>,
    ) -> Result<super::CapturedRange, super::CapturedRangeError> {
        self.model.capture_leaf_range(range)
    }
    /// Configure presentation/resource admission without mutating the document.
    pub fn with_image_policy(mut self, policy: super::ImagePolicy) -> Self {
        self.image_policy = policy;
        self
    }
    pub fn set_image_policy(&mut self, policy: super::ImagePolicy, cx: &mut Context<Self>) {
        self.image_policy = policy;
        self.projection.invalidate();
        cx.notify();
    }
    pub fn image_policy(&self) -> super::ImagePolicy {
        self.image_policy
    }

    pub fn caret_prefix_snapshot(&self) -> Option<SharedString> {
        self.model.caret_prefix_snapshot()
    }
    pub fn caret_prefix(&self) -> Option<&str> {
        self.model.caret_prefix()
    }
    pub fn is_capture_current(&self, range: &super::CapturedRange) -> bool {
        self.model.is_capture_current(range)
    }
    pub fn replace_captured(
        &mut self,
        range: &mut super::CapturedRange,
        text: &str,
        cx: &mut Context<Self>,
    ) -> Result<(), super::CapturedRangeError> {
        if self.readonly || self.marked.is_some() {
            return Err(super::CapturedRangeError::Stale);
        }
        self.model.replace_captured(range, text)?;
        self.after_edit(EditOrigin::Api, cx);
        Ok(())
    }
    pub fn select_end(&mut self, cx: &mut Context<Self>) -> Result<(), super::SelectionError> {
        self.model.select_end()?;
        self.selection_changed(cx);
        Ok(())
    }
    /// Reveal the current caret after viewport/layout changes without moving it.
    pub fn reveal_caret(&mut self, cx: &mut Context<Self>) {
        self.reveal.set(true);
        cx.notify();
    }
    /// Fence host callbacks on external owner changes without reparsing content.
    pub fn invalidate_captures(&mut self) {
        self.model.invalidate_captures();
        self.pending_paste = None;
    }
    pub fn is_composing(&self) -> bool {
        self.marked.is_some()
    }

    /// The headless model (document, selection, history).
    pub fn model(&self) -> &BlockEditorModel {
        &self.model
    }

    /// Document revision; bumped by every committed transaction.
    pub fn revision(&self) -> u64 {
        self.model.revision()
    }

    /// The current selection.
    pub fn selection(&self) -> &BlockSelection {
        self.model.selection()
    }

    /// Kind of the text block holding the caret, if any.
    pub fn block_kind_at_caret(&self) -> Option<BlockKind> {
        self.model.block_kind_at_caret()
    }

    /// Override caret, selection and composition colours (`BlockEditorStyle`).
    pub fn set_editor_style(&mut self, style: BlockEditorStyle, cx: &mut Context<Self>) {
        self.style = style;
        cx.notify();
    }

    /// The editor style overrides in effect.
    pub fn editor_style(&self) -> &BlockEditorStyle {
        &self.style
    }

    /// The scroll container the editor sits in (its `track_scroll` handle).
    /// With one set, the caret is scrolled into view after it moves; blocks
    /// outside the container's visible area are culled either way.
    pub fn set_scroll_handle(&mut self, handle: gpui::ScrollHandle) {
        self.scroll_handle = Some(handle);
    }

    /// Whether edits are blocked (navigation and selection still work).
    pub fn is_readonly(&self) -> bool {
        self.readonly
    }

    /// Block or allow edits.
    pub fn set_readonly(&mut self, readonly: bool, cx: &mut Context<Self>) {
        self.readonly = readonly;
        cx.notify();
    }

    /// Focus the editor.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus_handle, cx);
    }

    /// Set the selection; emits `SelectionChanged`.
    pub fn set_selection(
        &mut self,
        selection: BlockSelection,
        cx: &mut Context<Self>,
    ) -> Result<(), super::SelectionError> {
        self.model.set_selection(selection)?;
        self.selection_changed(cx);
        Ok(())
    }

    /// Run a model command as one transaction.
    pub fn run(
        &mut self,
        command: EditorCommand,
        cx: &mut Context<Self>,
    ) -> Result<bool, CommandError> {
        if self.readonly {
            return Ok(false);
        }
        let name = command_name(&command);
        self.end_composition();
        let revision = self.model.revision();
        let changed = self.model.run(command)?;
        if self.model.revision() != revision {
            self.after_edit(EditOrigin::Command(name), cx);
        } else {
            // Stored-mark toggles change no text but show in the toolbar.
            cx.notify();
        }
        Ok(changed)
    }

    /// Undo the last transaction (or IME composition) group.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if self.readonly {
            return;
        }
        if self.model.undo() {
            self.after_edit(EditOrigin::Undo, cx);
        }
    }

    /// Redo the last undone group.
    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if self.readonly {
            return;
        }
        if self.model.redo() {
            self.after_edit(EditOrigin::Redo, cx);
        }
    }

    /// The document as plain text for assistive technology (D8): leaves in
    /// order, one per line, inline atoms dropped. Built at most once per
    /// revision, and only asked for while accessibility is active.
    pub(crate) fn accessible_value(&self) -> SharedString {
        let revision = self.model.revision();
        if let Some((cached, value)) = &*self.a11y_value.borrow()
            && *cached == revision
        {
            return value.clone();
        }
        let doc = &self.model.doc;
        let value: SharedString = doc
            .leaves()
            .iter()
            .filter_map(|id| doc.leaf_text(*id))
            .map(|text| text.replace(super::inline::ATOM_CHAR, ""))
            .collect::<Vec<_>>()
            .join("\n")
            .into();
        *self.a11y_value.borrow_mut() = Some((revision, value.clone()));
        value
    }

    pub(crate) fn snapshot(&mut self) -> Rc<Snapshot> {
        self.projection.snapshot(&self.model.doc)
    }

    /// Blocks that are always laid out: the selection ends, the IME
    /// composition, and their ancestors.
    pub(crate) fn pinned_blocks(&self) -> Vec<BlockId> {
        let doc = &self.model.doc;
        let mut ids = match self.model.selection() {
            BlockSelection::Text { anchor, head } => vec![anchor.block, head.block],
            BlockSelection::Blocks { anchor, head } | BlockSelection::Cells { anchor, head } => {
                vec![*anchor, *head]
            }
        };
        if let Some((block, _)) = self.marked_range() {
            ids.push(block);
        }
        // Every ancestor, so a chunk of a long list holding the caret's item
        // is pinned too.
        ids.into_iter().flat_map(|id| doc.path(id)).collect()
    }

    /// Per-leaf selected byte ranges, for painting.
    pub(crate) fn selection_ranges(&self) -> Vec<(BlockId, Range<usize>)> {
        if let Some(cells) = self.model.selected_cells() {
            return cells
                .into_iter()
                .flatten()
                .map(|c| (c, 0..self.model.doc.leaf_len(c).unwrap_or(0)))
                .collect();
        }
        let Ok(range) = self.model.doc.resolve_selection(self.model.selection()) else {
            return Vec::new();
        };
        if range.is_collapsed() {
            return Vec::new();
        }
        self.model
            .text_ranges(range)
            .into_iter()
            .map(|(id, lo, hi)| (id, lo..hi))
            .collect()
    }

    pub(crate) fn marked_range(&self) -> Option<(BlockId, Range<usize>)> {
        self.marked.as_ref().map(|m| (m.block, m.range.clone()))
    }

    pub(crate) fn show_caret(&self, window: &Window, cx: &App) -> bool {
        self.focus_handle.is_focused(window) && self.blink_cursor.read(cx).visible()
    }

    // ---- internal edit plumbing -------------------------------------------

    fn after_edit(&mut self, origin: EditOrigin, cx: &mut Context<Self>) {
        self.goal_x = None;
        self.reveal.set(true);
        self.pause_blink(cx);
        cx.emit(BlockEditorEvent::Changed {
            revision: self.model.revision(),
            origin,
        });
        cx.emit(BlockEditorEvent::SelectionChanged);
        cx.notify();
    }

    fn selection_changed(&mut self, cx: &mut Context<Self>) {
        self.reveal.set(true);
        self.pause_blink(cx);
        cx.emit(BlockEditorEvent::SelectionChanged);
        cx.notify();
    }

    fn pause_blink(&mut self, cx: &mut Context<Self>) {
        self.blink_cursor.update(cx, |b, cx| b.pause(cx));
    }

    fn edit(
        &mut self,
        origin: EditOrigin,
        f: impl FnOnce(&mut BlockEditorModel) -> Result<(), CommandError>,
        cx: &mut Context<Self>,
    ) {
        if self.readonly {
            return;
        }
        let revision = self.model.revision();
        // Errors (e.g. typing into a code block, Phase 4) leave the document
        // untouched; the spike ignores them.
        let _ = f(&mut self.model);
        if self.model.revision() != revision {
            self.after_edit(origin, cx);
        } else {
            cx.notify();
        }
    }

    fn end_composition(&mut self) {
        self.marked = None;
        if std::mem::take(&mut self.composition_open) {
            self.model.history.commit_transaction();
        }
    }

    /// Whether an IME undo group is open (tests: it must never leak).
    #[cfg(test)]
    pub(crate) fn composition_open(&self) -> bool {
        self.composition_open
    }

    /// A whole-block selection collapses to its start or end.
    fn collapse_blocks(&mut self, forward: bool, cx: &mut Context<Self>) -> bool {
        if !matches!(
            self.model.selection(),
            BlockSelection::Blocks { .. } | BlockSelection::Cells { .. }
        ) {
            return false;
        }
        if let Ok(range) = self.model.doc.resolve_selection(self.model.selection()) {
            let target = if forward { range.end } else { range.start };
            self.move_to(target, false, cx);
        }
        true
    }

    fn head_anchor(&self) -> Option<(BlockPosition, BlockPosition)> {
        match self.model.selection() {
            BlockSelection::Text { anchor, head } => Some((*anchor, *head)),
            BlockSelection::Blocks { .. } => None,
            BlockSelection::Cells { anchor, head } => {
                Some((BlockPosition::new(*anchor, 0), BlockPosition::new(*head, 0)))
            }
        }
    }

    fn move_to(&mut self, head: BlockPosition, extend: bool, cx: &mut Context<Self>) {
        let anchor = match (extend, self.head_anchor()) {
            (true, Some((anchor, _))) => anchor,
            _ => head,
        };
        self.end_composition();
        // A drag or Shift+arrow from one cell into another of the same table
        // selects cells, not text.
        let doc = &self.model.doc;
        let selection = match (
            super::table::table_of(doc, anchor.block),
            super::table::table_of(doc, head.block),
        ) {
            (Some(a), Some(b)) if a == b && anchor.block != head.block => BlockSelection::Cells {
                anchor: anchor.block,
                head: head.block,
            },
            _ => BlockSelection::Text { anchor, head },
        };
        let _ = self.model.set_selection(selection);
        self.selection_changed(cx);
    }

    /// One grapheme left/right, crossing leaf boundaries in document order.
    pub(crate) fn step_horizontal(&self, pos: BlockPosition, forward: bool) -> BlockPosition {
        let doc = &self.model.doc;
        let len = doc.leaf_len(pos.block).unwrap_or(0);
        if let Some(code) = doc.plain(pos.block) {
            let next = if forward {
                code[pos.offset..]
                    .chars()
                    .next()
                    .map(|c| pos.offset + c.len_utf8())
            } else {
                code[..pos.offset]
                    .chars()
                    .next_back()
                    .map(|c| pos.offset - c.len_utf8())
            };
            if let Some(offset) = next {
                return BlockPosition::new(pos.block, offset);
            }
        } else if let Some(inline) = doc.inline(pos.block) {
            let next = if forward {
                inline.next_grapheme(pos.offset)
            } else {
                inline.prev_grapheme(pos.offset)
            };
            if let Some(offset) = next.filter(|o| *o != pos.offset) {
                return BlockPosition::new(pos.block, offset);
            }
        } else if forward && pos.offset == 0 {
            return BlockPosition::new(pos.block, 1);
        } else if !forward && pos.offset == 1 {
            return BlockPosition::new(pos.block, 0);
        }
        let leaves = doc.leaves();
        let Some(ix) = leaves.iter().position(|id| *id == pos.block) else {
            return pos;
        };
        let neighbour = if forward {
            leaves.get(ix + 1)
        } else {
            ix.checked_sub(1).and_then(|i| leaves.get(i))
        };
        match neighbour {
            Some(id) if forward => BlockPosition::new(*id, 0),
            Some(id) => BlockPosition::new(*id, doc.leaf_len(*id).unwrap_or(0)),
            None => BlockPosition::new(pos.block, if forward { len } else { 0 }),
        }
    }

    /// The next word boundary from `pos` (skip spaces, then a word); at a
    /// leaf edge, one step into the neighbouring leaf.
    fn step_word(&self, pos: BlockPosition, forward: bool) -> BlockPosition {
        let Some(text) = self.model.doc.leaf_text(pos.block) else {
            return self.step_horizontal(pos, forward);
        };
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        let mut offset = pos.offset;
        if forward {
            let rest = &text[offset..];
            let mut it = rest.char_indices().peekable();
            while let Some((_, c)) = it.peek().copied()
                && !is_word(c)
            {
                it.next();
            }
            while let Some((_, c)) = it.peek().copied()
                && is_word(c)
            {
                it.next();
            }
            offset += it.peek().map(|(i, _)| *i).unwrap_or(rest.len());
        } else {
            let before = &text[..offset];
            let mut it = before.char_indices().rev().peekable();
            while let Some((_, c)) = it.peek().copied()
                && !is_word(c)
            {
                it.next();
            }
            let mut start = it.peek().map(|(i, c)| i + c.len_utf8()).unwrap_or(0);
            while let Some((i, c)) = it.peek().copied()
                && is_word(c)
            {
                start = i;
                it.next();
            }
            offset = if start < offset { start } else { 0 };
        }
        if offset == pos.offset {
            return self.step_horizontal(pos, forward);
        }
        self.model.doc.clamp(BlockPosition::new(pos.block, offset))
    }

    fn word(&mut self, forward: bool, extend: bool, cx: &mut Context<Self>) {
        if self.collapse_blocks(forward, cx) {
            return;
        }
        let Some((_, head)) = self.head_anchor() else {
            return;
        };
        self.goal_x = None;
        let next = self.step_word(head, forward);
        self.move_to(next, extend, cx);
    }

    pub(crate) fn on_move_word_left(
        &mut self,
        _: &MoveWordLeft,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.word(false, false, cx);
    }
    pub(crate) fn on_move_word_right(
        &mut self,
        _: &MoveWordRight,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.word(true, false, cx);
    }
    pub(crate) fn on_select_word_left(
        &mut self,
        _: &SelectWordLeft,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.word(false, true, cx);
    }
    pub(crate) fn on_select_word_right(
        &mut self,
        _: &SelectWordRight,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.word(true, true, cx);
    }

    fn horizontal(&mut self, forward: bool, extend: bool, cx: &mut Context<Self>) {
        if self.collapse_blocks(forward, cx) {
            return;
        }
        let Some((anchor, head)) = self.head_anchor() else {
            return;
        };
        self.goal_x = None;
        // Collapsing a range moves to its edge, like every text field.
        if !extend && anchor != head {
            let ord = self.model.doc.cmp_positions(anchor, head).ok();
            let pick_head = matches!(
                (forward, ord),
                (true, Some(std::cmp::Ordering::Less)) | (false, Some(std::cmp::Ordering::Greater))
            );
            let target = if pick_head { head } else { anchor };
            return self.move_to(target, false, cx);
        }
        let next = self.step_horizontal(head, forward);
        self.move_to(next, extend, cx);
    }

    fn vertical(&mut self, down: bool, extend: bool, cx: &mut Context<Self>) {
        if self.collapse_blocks(down, cx) {
            return;
        }
        let Some((_, head)) = self.head_anchor() else {
            return;
        };
        let leaves = self.snapshot().leaves.clone();
        let Some(ix) = leaves.iter().position(|l| l.block == head.block) else {
            return;
        };
        // The model-adjacent caret leaf, which may be culled (no geometry).
        let adjacent = if down {
            leaves.get(ix + 1)
        } else {
            ix.checked_sub(1).and_then(|i| leaves.get(i))
        };
        let frame = self.frame.clone();
        let frame = frame.borrow();
        let laid_out = frame.caret_bounds(head, self.style.caret_width());
        let x = laid_out.map(|caret| *self.goal_x.get_or_insert(caret.origin.x));
        let visual = x.and_then(|x| frame.vertical(head, x, down));
        let target = match (visual, adjacent) {
            // Within the block, or into the laid-out adjacent leaf.
            (Some(t), _) if t.block == head.block => t,
            // Down past a table's last row leaves the table (D13).
            _ if down && !extend && self.model.in_last_table_row() => {
                drop(frame);
                self.edit(
                    EditOrigin::Command("table_exit"),
                    |m| m.exit_table().map(|_| ()),
                    cx,
                );
                return;
            }
            (Some(t), Some(next)) if t.block == next.block => t,
            // Up/down inside a table goes to the cell above/below (cells of
            // one table are laid out together).
            (Some(t), _)
                if super::table::table_of(&self.model.doc, head.block).is_some_and(|table| {
                    super::table::table_of(&self.model.doc, t.block) == Some(table)
                }) =>
            {
                t
            }
            // Into a culled (or not yet laid-out) leaf: model fallback.
            (_, Some(next)) => {
                let offset = if down { 0 } else { next.len };
                BlockPosition::new(next.block, offset)
            }
            (_, None) if down && self.model.doc.plain(head.block).is_some() && !extend => {
                drop(frame);
                self.edit(
                    EditOrigin::Command("exit_code_block"),
                    |m| m.exit_code_block().map(|_| ()),
                    cx,
                );
                return;
            }
            (_, None) => {
                let doc = &self.model.doc;
                if down {
                    doc.last_caret()
                } else {
                    doc.first_caret()
                }
            }
        };
        drop(frame);
        let goal = self.goal_x;
        self.move_to(self.model.doc.clamp(target), extend, cx);
        self.goal_x = goal;
    }

    /// Home/End: the visual (soft-wrapped) line from the last frame, else
    /// the block edge.
    fn line_edge(&mut self, end: bool, extend: bool, cx: &mut Context<Self>) {
        if self.collapse_blocks(end, cx) {
            return;
        }
        let Some((_, head)) = self.head_anchor() else {
            return;
        };
        let visual = self.frame.borrow().line_edge(head, end);
        // End at a soft wrap: the line's end index is also the next line's
        // start, where the caret would draw. Stay before the wrap point.
        let visual = visual.map(|pos| {
            if !end || pos.block != head.block {
                return pos;
            }
            let frame = self.frame.borrow();
            let (Some(leaf), Some(inline)) =
                (frame.leaf(pos.block), self.model.doc.inline(pos.block))
            else {
                return pos;
            };
            let line_y = leaf.caret_point(head.offset).y;
            if pos.offset < inline.len() && leaf.caret_point(pos.offset).y > line_y {
                let prev = inline.prev_grapheme(pos.offset).unwrap_or(pos.offset);
                BlockPosition::new(pos.block, prev)
            } else {
                pos
            }
        });
        let target = visual.unwrap_or_else(|| {
            let offset = if end {
                self.model.doc.leaf_len(head.block).unwrap_or(0)
            } else {
                0
            };
            BlockPosition::new(head.block, offset)
        });
        self.goal_x = None;
        self.move_to(self.model.doc.clamp(target), extend, cx);
    }

    /// The link drawn under the window point `at`: its leaf, byte range
    /// and url.
    pub fn link_at(&self, at: Point<Pixels>) -> Option<(BlockId, Range<usize>, SharedString)> {
        let pos = {
            let frame = self.frame.borrow();
            let pos = frame.hit_test(at)?;
            if !frame.leaf(pos.block)?.bounds().contains(&at) {
                return None;
            }
            pos
        };
        let inline = self.model.doc.inline(pos.block)?;
        let url = inline.mark_after(pos.offset)?.link.as_ref()?.url.clone();
        let range = link_extent(inline, pos.offset, &url)?;
        Some((pos.block, range, url))
    }

    /// The url of the link under the mouse, if any (D14 hover).
    pub fn hovered_link(&self) -> Option<&SharedString> {
        self.hovered_link.as_ref()
    }

    pub(crate) fn hover_at(&mut self, at: Point<Pixels>, secondary: bool, cx: &mut Context<Self>) {
        let url = self.link_at(at).map(|(_, _, url)| url);
        let hand = self.task_checkbox_at(at).is_some() || (secondary && url.is_some());
        if hand != self.pointer_hand {
            self.pointer_hand = hand;
            cx.notify();
        }
        if url != self.hovered_link {
            self.hovered_link = url.clone();
            cx.emit(BlockEditorEvent::LinkHovered { url });
            cx.notify();
        }
    }

    /// Cmd/Ctrl+click: emit [`BlockEditorEvent::LinkClicked`] for the link
    /// under `at`. Returns whether there was one.
    pub(crate) fn follow_link_at(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) -> bool {
        match self.link_at(at) {
            Some((_, _, url)) => {
                cx.emit(BlockEditorEvent::LinkClicked { url });
                true
            }
            None => false,
        }
    }

    /// The url of the link at the caret (or the selection start).
    pub fn link_at_caret(&self) -> Option<SharedString> {
        self.link_range_at_caret().map(|(_, _, url)| url)
    }

    fn link_range_at_caret(&self) -> Option<(BlockId, Range<usize>, SharedString)> {
        let head = match &self.model.selection {
            BlockSelection::Text { anchor, head } => {
                if anchor.block == head.block && anchor.offset < head.offset {
                    *anchor
                } else {
                    *head
                }
            }
            _ => return None,
        };
        let inline = self.model.doc.inline(head.block)?;
        let mark = inline
            .mark_after(head.offset)
            .filter(|m| m.link.is_some())
            .or_else(|| inline.mark_before(head.offset))?;
        let url = mark.link.as_ref()?.url.clone();
        let probe = if inline
            .mark_after(head.offset)
            .is_some_and(|m| m.link.is_some())
        {
            head.offset
        } else {
            head.offset.checked_sub(1)?
        };
        let range = link_extent(inline, probe, &url)?;
        Some((head.block, range, url))
    }

    /// Cmd/Ctrl+K: ask the host for its link popover.
    pub fn request_link_edit(&mut self, cx: &mut Context<Self>) {
        if self.readonly {
            return;
        }
        let url = self.link_at_caret();
        cx.emit(BlockEditorEvent::EditLinkRequested { url });
    }

    pub(crate) fn on_edit_link(&mut self, _: &EditLink, _: &mut Window, cx: &mut Context<Self>) {
        self.request_link_edit(cx);
    }

    /// Add, change (`Some(url)`) or remove (`None`) the link on the
    /// selection, or on the whole link at a collapsed caret. One undo step.
    pub fn set_link(&mut self, url: Option<&str>, cx: &mut Context<Self>) {
        if self.readonly {
            return;
        }
        let collapsed = matches!(
            &self.model.selection,
            BlockSelection::Text { anchor, head } if anchor == head
        );
        if collapsed && let Some((block, range, _)) = self.link_range_at_caret() {
            let _ = self.model.set_selection(BlockSelection::Text {
                anchor: BlockPosition::new(block, range.start),
                head: BlockPosition::new(block, range.end),
            });
        } else if collapsed {
            return;
        }
        let change = match url.map(str::trim).filter(|u| !u.is_empty()) {
            Some(url) => super::marks::MarkChange::new().with_link(url.to_string(), None),
            None => super::marks::MarkChange::new().clear(MarkKind::Link),
        };
        self.model.break_coalescing();
        let _ = self.run(EditorCommand::ApplyMark(change), cx);
        self.model.break_coalescing();
    }

    /// The task checkbox drawn left of a task item's first line under
    /// `at`: the item's text leaf and whether it is checked.
    /// Mouse-down on a task checkbox: toggle it, leaving the caret and
    /// selection alone. Returns whether it was one.
    pub(crate) fn click_checkbox(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) -> bool {
        match self.task_checkbox_at(at) {
            Some((leaf, checked)) if !self.readonly => {
                self.end_composition();
                self.dragging = false;
                self.set_task_checked(leaf, !checked, cx);
                true
            }
            _ => false,
        }
    }

    pub(crate) fn task_checkbox_at(&self, at: Point<Pixels>) -> Option<(BlockId, bool)> {
        use super::kind::BlockKindData;
        use gpui::px;
        let frame = self.frame.borrow();
        frame.leaves.iter().find_map(|leaf| {
            let b = leaf.bounds();
            let line = px(28.).min(b.size.height);
            if !(at.y >= b.top()
                && at.y < b.top() + line
                && at.x < b.left()
                && at.x >= b.left() - px(28.))
            {
                return None;
            }
            let block = leaf.info.block;
            let (item, index) = self.model.doc.index_in_parent(block)?;
            match self.model.doc.kind(item) {
                Some(BlockKindData::ListItem {
                    checked: Some(c), ..
                }) if index == 0 => Some((block, *c)),
                _ => None,
            }
        })
    }

    /// Mouse: place or extend the caret at a window point.
    pub(crate) fn mouse_at(&mut self, at: Point<Pixels>, extend: bool, cx: &mut Context<Self>) {
        // Block handle: a click in the left edge beside a table or code
        // block (left of its text) selects the block as a node.
        if !extend && let Some(block) = self.block_handle_at(at) {
            self.goal_x = None;
            self.end_composition();
            let _ = self.model.set_selection(BlockSelection::Blocks {
                anchor: block,
                head: block,
            });
            self.dragging = false;
            self.selection_changed(cx);
            return;
        }
        let hit = self.frame.borrow().hit_test(at);
        // A click on an inline atom selects the whole atom (#3113).
        if !extend && let Some(pos) = hit.and_then(|pos| self.atom_under(pos, at)) {
            self.goal_x = None;
            self.end_composition();
            let _ = self.model.set_selection(BlockSelection::Text {
                anchor: pos,
                head: BlockPosition::new(
                    pos.block,
                    pos.offset + super::inline::ATOM_CHAR.len_utf8(),
                ),
            });
            self.selection_changed(cx);
            return;
        }
        if let Some(pos) = hit {
            self.goal_x = None;
            let pos = self.model.doc.clamp(pos);
            self.move_to(pos, extend, cx);
        }
    }

    /// Double-click: the word at the caret; triple-click: the whole block.
    pub(crate) fn select_unit(&mut self, block: bool, cx: &mut Context<Self>) {
        let Some(caret) = self.model.caret() else {
            return;
        };
        let Some(text) = self.model.doc.leaf_text(caret.block) else {
            return;
        };
        let range = if block {
            0..text.len()
        } else {
            match crate::text::selection::word_range_at(text, caret.offset) {
                Some(r) => r,
                None => return,
            }
        };
        let _ = self.model.set_selection(BlockSelection::Text {
            anchor: BlockPosition::new(caret.block, range.start),
            head: BlockPosition::new(caret.block, range.end),
        });
        self.dragging = false;
        self.selection_changed(cx);
    }

    /// The table or code block whose left edge (the strip left of its
    /// text, inside the block's padding) is under `at`.
    pub(crate) fn block_handle_at(&self, at: Point<Pixels>) -> Option<BlockId> {
        let frame = self.frame.borrow();
        use gpui::px;
        let in_edge = |b: &Bounds<Pixels>| {
            at.y >= b.top() && at.y < b.bottom() && at.x < b.left() && at.x >= b.left() - px(24.)
        };
        if let Some((table, _)) = frame.tables.iter().find(|(_, b)| in_edge(b)) {
            return Some(*table);
        }
        frame
            .leaves
            .iter()
            .find(|leaf| self.model.doc.plain(leaf.info.block).is_some() && in_edge(&leaf.bounds()))
            .map(|leaf| leaf.info.block)
    }

    /// The start of the inline atom drawn under the window point `at` in
    /// the leaf a hit test found.
    fn atom_under(&self, pos: BlockPosition, at: Point<Pixels>) -> Option<BlockPosition> {
        let frame = self.frame.borrow();
        let range = frame.leaf(pos.block)?.atom_at(at)?;
        Some(BlockPosition::new(pos.block, range.start))
    }

    fn toggle_mark(&mut self, kind: MarkKind, cx: &mut Context<Self>) {
        let _ = self.run(EditorCommand::ToggleMark(kind), cx);
    }

    /// The selection as clipboard text, and whether it is the editor's own
    /// inline Markdown (D17): then the copy is tagged with
    /// [`COPY_TAG`] so paste parses it back.
    fn selected_text(&self) -> (String, bool) {
        if let Some(table) = self.model.cells_markdown() {
            return (table, false);
        }
        if let Some((parent, range)) = self.model.selected_blocks() {
            let ids = &self.model.doc.children(parent)[range];
            return (super::serialize::join_blocks(&self.model.doc, ids), false);
        }
        let Ok(range) = self.model.doc.resolve_selection(self.model.selection()) else {
            return (String::new(), false);
        };
        let mut inline_only = true;
        let text = self
            .model
            .text_ranges(range)
            .into_iter()
            .filter_map(|(id, lo, hi)| {
                // Marks and inline atoms copy as Markdown (an image as
                // `![alt](url)`, never the U+FFFC placeholder, #3113). A
                // partial selection serializes as its own inline, so
                // delimiters are always balanced at the cut edges.
                if let Some(inline) = self.model.doc.inline(id) {
                    return Some(super::serialize::inline_markdown(&inline.slice(lo..hi)));
                }
                inline_only = false;
                let text = self.model.doc.leaf_text(id)?;
                text.get(lo..hi).map(str::to_string)
            })
            .collect::<Vec<_>>()
            .join("\n");
        (text, inline_only)
    }

    // ---- action handlers --------------------------------------------------

    pub(crate) fn on_backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        self.end_composition();
        self.edit(EditOrigin::Backspace, |m| m.delete(false), cx);
    }
    pub(crate) fn on_delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        self.end_composition();
        self.edit(EditOrigin::DeleteForward, |m| m.delete(true), cx);
    }
    pub(crate) fn on_enter(&mut self, _: &Enter, _: &mut Window, cx: &mut Context<Self>) {
        if self.readonly || self.is_composing() {
            return;
        }
        if self.submit_policy == SubmitPolicy::Enter {
            cx.emit(BlockEditorEvent::SubmitRequested);
            return;
        }
        self.newline(cx);
    }
    pub(crate) fn on_structural_enter(
        &mut self,
        _: &StructuralEnter,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.readonly || self.is_composing() {
            return;
        }
        self.newline(cx);
    }
    fn newline(&mut self, cx: &mut Context<Self>) {
        self.end_composition();
        // "```lang" then Enter: a code block.
        if !self.readonly
            && self.rule_on(RuleKind::CodeFence)
            && let Some(caret) = self.model.caret()
            && self.model.doc.kind(caret.block) == Some(&super::kind::BlockKindData::Paragraph)
            && caret.offset == self.model.doc.leaf_len(caret.block).unwrap_or(0)
            && let Some(lang) = self
                .model
                .doc
                .inline(caret.block)
                .and_then(|i| code_fence(i.text()))
        {
            self.fence_to_code_block(caret.block, lang, cx);
            return;
        }
        // Enter in a table cell (D13).
        if !self.readonly && self.model.current_cell().is_some() {
            self.model.break_coalescing();
            self.edit(
                EditOrigin::Command("table_enter"),
                |m| m.table_enter().map(|_| ()),
                cx,
            );
            self.model.break_coalescing();
            return;
        }
        // "| a | b |" then Enter at the end of a paragraph: a table.
        if !self.readonly
            && self.rule_on(RuleKind::Table)
            && let Some(caret) = self.model.caret()
            && caret.offset == self.model.doc.leaf_len(caret.block).unwrap_or(0)
            && self
                .model
                .doc
                .inline(caret.block)
                .is_some_and(|i| super::table::table_row_cells(i.text()).is_some())
        {
            let block = caret.block;
            self.model.break_coalescing();
            self.edit(
                EditOrigin::InputRule("table"),
                |m| m.paragraph_to_table(block).map(|_| ()),
                cx,
            );
            self.model.break_coalescing();
            return;
        }
        // "---" then Enter in an otherwise empty paragraph: a rule.
        if !self.readonly
            && self.rule_on(RuleKind::HorizontalRule)
            && let Some(caret) = self.model.caret()
            && self.model.doc.kind(caret.block) == Some(&super::kind::BlockKindData::Paragraph)
            && self.model.doc.inline(caret.block).map(|i| i.text()) == Some("---")
        {
            let block = caret.block;
            self.model.break_coalescing();
            self.model.history.begin_transaction();
            let _ = self.model.set_selection(BlockSelection::Text {
                anchor: BlockPosition::new(block, 0),
                head: BlockPosition::new(block, 3),
            });
            let _ = self.model.delete(false);
            let _ = self.model.paragraph_to_rule();
            self.model.history.commit_transaction();
            self.model.break_coalescing();
            self.after_edit(EditOrigin::InputRule("horizontal_rule"), cx);
            return;
        }
        self.edit(EditOrigin::Command("split_block"), |m| m.split_block(), cx);
    }
    /// Shift+Enter (Ctrl+Enter in composer mode): a cell soft break;
    /// structural Enter elsewhere, without submission.
    pub(crate) fn on_soft_break(&mut self, _: &SoftBreak, _: &mut Window, cx: &mut Context<Self>) {
        if self.readonly || self.is_composing() {
            return;
        }
        self.end_composition();
        if self.model.current_cell().is_some() {
            self.model.break_coalescing();
            self.edit(
                EditOrigin::Command("cell_soft_break"),
                |m| m.cell_soft_break().map(|_| ()),
                cx,
            );
            self.model.break_coalescing();
        } else {
            self.newline(cx);
        }
    }

    /// Cmd/Ctrl+Enter: leave a table to the block after it.
    pub(crate) fn on_exit_block(&mut self, _: &ExitBlock, _: &mut Window, cx: &mut Context<Self>) {
        if self.readonly || self.is_composing() {
            return;
        }
        if matches!(
            self.submit_policy,
            SubmitPolicy::Enter | SubmitPolicy::ModifiedEnter
        ) {
            cx.emit(BlockEditorEvent::SubmitRequested);
            return;
        }
        self.end_composition();
        if self.readonly || self.model.current_cell().is_none() {
            cx.propagate();
            return;
        }
        self.model.break_coalescing();
        self.edit(
            EditOrigin::Command("table_exit"),
            |m| m.exit_table().map(|_| ()),
            cx,
        );
        self.model.break_coalescing();
    }

    pub(crate) fn on_move_left(&mut self, _: &MoveLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(false, false, cx);
    }
    pub(crate) fn on_move_right(&mut self, _: &MoveRight, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(true, false, cx);
    }
    pub(crate) fn on_move_up(&mut self, _: &MoveUp, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(false, false, cx);
    }
    pub(crate) fn on_move_down(&mut self, _: &MoveDown, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(true, false, cx);
    }
    pub(crate) fn on_move_home(&mut self, _: &MoveHome, _: &mut Window, cx: &mut Context<Self>) {
        self.line_edge(false, false, cx);
    }
    pub(crate) fn on_move_end(&mut self, _: &MoveEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.line_edge(true, false, cx);
    }
    pub(crate) fn on_select_left(
        &mut self,
        _: &SelectLeft,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.horizontal(false, true, cx);
    }
    pub(crate) fn on_select_right(
        &mut self,
        _: &SelectRight,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.horizontal(true, true, cx);
    }
    pub(crate) fn on_select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(false, true, cx);
    }
    pub(crate) fn on_select_down(
        &mut self,
        _: &SelectDown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.vertical(true, true, cx);
    }
    pub(crate) fn on_select_home(
        &mut self,
        _: &SelectHome,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.line_edge(false, true, cx);
    }
    pub(crate) fn on_select_end(&mut self, _: &SelectEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.line_edge(true, true, cx);
    }
    pub(crate) fn on_select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.end_composition();
        let _ = self.model.run(EditorCommand::SelectAll);
        self.selection_changed(cx);
    }
    pub(crate) fn on_undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        self.end_composition();
        self.undo(cx);
    }
    pub(crate) fn on_redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        self.end_composition();
        self.redo(cx);
    }
    pub(crate) fn on_toggle_bold(
        &mut self,
        _: &ToggleBold,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_mark(MarkKind::Bold, cx);
    }
    pub(crate) fn on_toggle_italic(
        &mut self,
        _: &ToggleItalic,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_mark(MarkKind::Italic, cx);
    }
    pub(crate) fn on_toggle_code(
        &mut self,
        _: &ToggleCode,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_mark(MarkKind::Code, cx);
    }
    pub(crate) fn on_indent(&mut self, _: &Indent, _: &mut Window, cx: &mut Context<Self>) {
        let _ = self.run(EditorCommand::Indent, cx);
    }
    pub(crate) fn on_outdent(&mut self, _: &Outdent, _: &mut Window, cx: &mut Context<Self>) {
        let _ = self.run(EditorCommand::Outdent, cx);
    }
    pub(crate) fn on_copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        let (text, own) = self.selected_text();
        if text.is_empty() {
            return;
        }
        cx.write_to_clipboard(if own {
            ClipboardItem::new_string_with_metadata(text, COPY_TAG.to_string())
        } else {
            ClipboardItem::new_string(text)
        });
    }
    pub(crate) fn on_cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        self.end_composition();
        self.on_copy(&Copy, window, cx);
        self.edit(
            EditOrigin::Cut,
            |m| m.run(EditorCommand::DeleteSelection).map(|_| ()),
            cx,
        );
    }
    pub(crate) fn on_paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if self.readonly {
            return;
        }
        if let Some(item) = cx.read_from_clipboard() {
            self.paste(item, false, cx);
        }
    }
    pub(crate) fn on_native_paste(
        &mut self,
        _: &crate::input::Paste,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.on_paste(&Paste, window, cx);
    }
    pub(crate) fn on_paste_plain(
        &mut self,
        _: &PastePlain,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.readonly {
            return;
        }
        if let Some(item) = cx.read_from_clipboard() {
            self.paste(item, true, cx);
        }
    }

    /// Deliver semantic clipboard content. `plain` bypasses Markdown import.
    /// With Host policy this emits one request; there is no fallback insertion
    /// until the owner resolves it, so mixed image/text entries cannot double
    /// insert. This method itself never reads the system clipboard.
    pub fn paste(&mut self, item: ClipboardItem, plain: bool, cx: &mut Context<Self>) {
        if self.readonly {
            return;
        }
        self.end_composition();
        if self.clipboard_policy == ClipboardPolicy::Host {
            self.paste_sequence += 1;
            self.pending_paste = Some(self.paste_sequence);
            let request = PasteRequest {
                range: self.model.capture(self.model.selection.clone()),
                plain,
                markdown: item.metadata().is_some_and(|m| m == COPY_TAG),
                sequence: self.paste_sequence,
            };
            cx.emit(BlockEditorEvent::PasteRequested { request, item });
        } else if let Some(text) = item.text() {
            let markdown = item.metadata().is_some_and(|m| m == COPY_TAG);
            self.paste_text(&text, plain, markdown, cx);
        }
    }

    /// Resolve an intercepted paste with text not consumed by the host's other
    /// admissions (e.g. image attachments). `None` means wholly handled or
    /// rejected. Returns false when navigation/edits made the request stale.
    pub fn is_paste_current(&self, request: &PasteRequest) -> bool {
        !self.readonly
            && self.pending_paste == Some(request.sequence)
            && self.model.is_capture_current(&request.range)
    }
    pub fn apply_paste(
        &mut self,
        request: &PasteRequest,
        text: Option<&str>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.is_paste_current(request) {
            return false;
        }
        self.pending_paste = None;
        if let Some(text) = text {
            self.paste_text(text, request.plain, request.markdown, cx);
        }
        true
    }
    fn paste_text(&mut self, text: &str, plain: bool, markdown: bool, cx: &mut Context<Self>) {
        self.edit(
            EditOrigin::Paste,
            |model| {
                if !plain && markdown && model.paste_inline_markdown(text)? {
                    return Ok(());
                }
                if !plain && model.paste_blocks(text)? {
                    return Ok(());
                }
                model.insert_text(text, EditOrigin::Paste)
            },
            cx,
        );
    }

    /// Markdown input rules (design §5.3): `# `..`###### `, `- `/`* `/`+ `
    /// and `1. ` at the start of a plain paragraph. The conversion is its own
    /// undo group after the typing, so one undo reverts it and keeps the
    /// typed text.
    fn apply_input_rule(&mut self, cx: &mut Context<Self>) {
        use super::kind::BlockKindData;
        let Some(caret) = self.model.caret() else {
            return;
        };
        let doc = &self.model.doc;
        if doc.kind(caret.block) != Some(&BlockKindData::Paragraph) {
            return;
        }
        // "```lang " typed into an otherwise empty paragraph.
        if self.rule_on(RuleKind::CodeFence)
            && let Some(lang) = doc
                .inline(caret.block)
                .and_then(|i| code_fence(i.text().strip_suffix(' ')?))
                .filter(|_| caret.offset == doc.leaf_len(caret.block).unwrap_or(0))
        {
            self.fence_to_code_block(caret.block, lang, cx);
            return;
        }
        let in_list = doc
            .parent(caret.block)
            .and_then(|parent| doc.kind(parent))
            .is_some_and(|kind| matches!(kind, BlockKindData::ListItem { .. }));
        let Some(text) = doc.inline(caret.block).map(|i| i.text().to_string()) else {
            return;
        };
        let prefix = &text[..caret.offset.min(text.len())];
        let Some(marker) = prefix.strip_suffix(' ') else {
            return;
        };
        let task = match marker {
            "[ ]" => Some(false),
            "[x]" | "[X]" => Some(true),
            _ => None,
        };
        let (commands, rule) = if self.rule_on(RuleKind::Heading)
            && !marker.is_empty()
            && marker.len() <= 6
            && marker.bytes().all(|b| b == b'#')
        {
            let level = marker.len() as u8;
            (
                vec![EditorCommand::SetBlockKind(BlockKind::Heading { level })],
                "heading",
            )
        } else if self.rule_on(RuleKind::BulletList)
            && !in_list
            && matches!(marker, "-" | "*" | "+")
        {
            (
                vec![EditorCommand::ToggleList { ordered: false }],
                "bullet_list",
            )
        } else if self.rule_on(RuleKind::OrderedList)
            && !in_list
            && marker.len() >= 2
            && marker.ends_with('.')
            && marker[..marker.len() - 1]
                .bytes()
                .all(|b| b.is_ascii_digit())
        {
            (
                vec![EditorCommand::ToggleList { ordered: true }],
                "ordered_list",
            )
        } else if self.rule_on(RuleKind::TaskItem)
            && let Some(checked) = task
        {
            let mut commands = Vec::new();
            if !in_list {
                commands.push(EditorCommand::ToggleList { ordered: false });
            }
            commands.push(EditorCommand::SetTaskChecked(checked));
            (commands, "task_item")
        } else if self.rule_on(RuleKind::Blockquote) && marker == ">" {
            (Vec::new(), "blockquote")
        } else {
            return;
        };
        let block = caret.block;
        let len = prefix.len();
        self.model.break_coalescing();
        self.model.history.begin_transaction();
        let _ = self.model.set_selection(BlockSelection::Text {
            anchor: BlockPosition::new(block, 0),
            head: BlockPosition::new(block, len),
        });
        let _ = self.model.delete(false);
        if commands.is_empty() {
            let _ = self.model.wrap_in_blockquote();
        }
        for command in commands {
            let _ = self.model.run(command);
        }
        self.model.history.commit_transaction();
        self.model.break_coalescing();
        self.after_edit(EditOrigin::InputRule(rule), cx);
    }

    /// Link input rule: `[text](url)` typed, on the closing `)`, becomes
    /// `text` with a link mark. One undo group: undo restores the source.
    fn apply_link_rule(&mut self, cx: &mut Context<Self>) {
        let Some(caret) = self.model.caret() else {
            return;
        };
        let Some(inline) = self.model.doc.inline(caret.block) else {
            return;
        };
        let Some((start, text, url)) = link_rule(inline.text(), caret.offset) else {
            return;
        };
        let block = caret.block;
        let pos = |offset| BlockPosition::new(block, offset);
        let text_end = start + 1 + text.len();
        self.model.break_coalescing();
        self.model.history.begin_transaction();
        // Drop "](url)" then "[".
        let _ = self.model.set_selection(BlockSelection::Text {
            anchor: pos(text_end),
            head: pos(caret.offset),
        });
        let _ = self.model.delete(false);
        let _ = self.model.set_selection(BlockSelection::Text {
            anchor: pos(start),
            head: pos(start + 1),
        });
        let _ = self.model.delete(false);
        let end = start + text.len();
        let _ = self.model.set_selection(BlockSelection::Text {
            anchor: pos(start),
            head: pos(end),
        });
        let _ = self.model.run(EditorCommand::ApplyMark(
            super::marks::MarkChange::new().with_link(url, None),
        ));
        let _ = self.model.set_selection(BlockSelection::caret(pos(end)));
        let after = super::marks::MarkChange::new()
            .clear(super::marks::MarkKind::Link)
            .apply(&self.model.active_marks().0);
        self.model.history.commit_transaction();
        self.model.break_coalescing();
        self.model.stored_marks = Some(after);
        self.after_edit(EditOrigin::InputRule("link"), cx);
    }

    /// Inline-mark input rules: a closing `` ` ``, `*`, `**` or `~~` typed
    /// after an opening one in the same leaf turns the text between into
    /// code, italic, bold or strikethrough and drops the delimiters. One undo
    /// group of its own: undo restores the literal delimiters.
    fn apply_mark_rule(&mut self, cx: &mut Context<Self>) {
        let Some(caret) = self.model.caret() else {
            return;
        };
        let Some(inline) = self.model.doc.inline(caret.block) else {
            return;
        };
        let Some((open, close, kind)) = mark_rule(inline, caret.offset) else {
            return;
        };
        let block = caret.block;
        let d = close.len();
        let pos = |offset| BlockPosition::new(block, offset);
        self.model.break_coalescing();
        self.model.history.begin_transaction();
        let _ = self.model.set_selection(BlockSelection::Text {
            anchor: pos(close.start),
            head: pos(close.end),
        });
        let _ = self.model.delete(false);
        let _ = self.model.set_selection(BlockSelection::Text {
            anchor: pos(open.start),
            head: pos(open.end),
        });
        let _ = self.model.delete(false);
        let end = close.start - d;
        let _ = self.model.set_selection(BlockSelection::Text {
            anchor: pos(open.start),
            head: pos(end),
        });
        let _ = self.model.run(EditorCommand::ApplyMark(
            super::marks::MarkChange::new().set(kind),
        ));
        let _ = self.model.set_selection(BlockSelection::caret(pos(end)));
        // Text typed next is outside the new span.
        let after = super::marks::MarkChange::new()
            .clear(kind)
            .apply(&self.model.active_marks().0);
        self.model.history.commit_transaction();
        self.model.break_coalescing();
        self.model.stored_marks = Some(after);
        self.after_edit(EditOrigin::InputRule("inline_mark"), cx);
    }

    /// Replace a typed code fence with an empty code block, caret inside,
    /// as its own undo group (one undo restores the typed fence).
    fn fence_to_code_block(&mut self, block: BlockId, lang: String, cx: &mut Context<Self>) {
        let len = self.model.doc.leaf_len(block).unwrap_or(0);
        self.model.break_coalescing();
        self.model.history.begin_transaction();
        let _ = self.model.set_selection(BlockSelection::Text {
            anchor: BlockPosition::new(block, 0),
            head: BlockPosition::new(block, len),
        });
        let _ = self.model.delete(false);
        let (lang, meta) = lang
            .split_once(char::is_whitespace)
            .map_or((lang.as_str(), None), |(lang, meta)| {
                (lang, Some(meta.trim_start().to_owned()))
            });
        let _ = self.model.set_code_block_info(Some(lang.to_owned()), meta);
        self.model.history.commit_transaction();
        self.model.break_coalescing();
        self.after_edit(EditOrigin::InputRule("code_block"), cx);
    }

    // ---- IME ---------------------------------------------------------------

    /// The IME sees the caret leaf as its document.
    fn ime_block(&self) -> Option<BlockId> {
        if let Some(m) = &self.marked {
            return Some(m.block);
        }
        let (_, head) = self.head_anchor()?;
        Some(head.block)
    }

    fn ime_text(&self) -> &str {
        self.ime_block()
            .and_then(|id| self.model.doc.leaf_text(id))
            .unwrap_or("")
    }

    fn select_utf16(&mut self, block: BlockId, range16: Range<usize>) {
        let text = self.ime_text();
        let range = utf16_to_byte(text, range16.start)..utf16_to_byte(text, range16.end);
        let _ = self.model.set_selection(BlockSelection::Text {
            anchor: BlockPosition::new(block, range.start),
            head: BlockPosition::new(block, range.end),
        });
    }

    fn select_marked_or(&mut self, range16: Option<Range<usize>>) {
        let Some(block) = self.ime_block() else {
            return;
        };
        if let Some(range16) = range16 {
            self.select_utf16(block, range16);
        } else if let Some(m) = self.marked.clone() {
            let _ = self.model.set_selection(BlockSelection::Text {
                anchor: BlockPosition::new(m.block, m.range.start),
                head: BlockPosition::new(m.block, m.range.end),
            });
        }
    }
}

fn command_name(command: &EditorCommand) -> &'static str {
    match command {
        EditorCommand::ToggleMark(_) => "toggle_mark",
        EditorCommand::ApplyMark(_) => "apply_mark",
        EditorCommand::SetBlockKind(_) => "set_block_kind",
        EditorCommand::ToggleList { .. } => "toggle_list",
        EditorCommand::ToggleTask => "toggle_task",
        EditorCommand::SetTaskChecked(_) => "set_task_checked",
        EditorCommand::Indent => "indent",
        EditorCommand::Outdent => "outdent",
        EditorCommand::MoveBlocks { .. } => "move_blocks",
        EditorCommand::InsertBlock(_) => "insert_block",
        EditorCommand::SelectAll => "select_all",
        EditorCommand::DeleteSelection => "delete_selection",
        EditorCommand::SetCodeBlock { .. } => "code_block",
        EditorCommand::InsertTable { .. } => "insert_table",
        EditorCommand::EditTable(_) => "edit_table",
    }
}

pub(crate) fn utf16_to_byte(text: &str, utf16: usize) -> usize {
    let mut count = 0;
    for (ix, ch) in text.char_indices() {
        if count >= utf16 {
            return ix;
        }
        count += ch.len_utf16();
    }
    text.len()
}

pub(crate) fn byte_to_utf16(text: &str, byte: usize) -> usize {
    text.get(..byte.min(text.len()))
        .map(|s| s.encode_utf16().count())
        .unwrap_or(0)
}

impl EntityInputHandler for BlockEditorState {
    fn text_for_range(
        &mut self,
        range16: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let text = self.ime_text();
        let range = utf16_to_byte(text, range16.start)..utf16_to_byte(text, range16.end);
        adjusted.replace(byte_to_utf16(text, range.start)..byte_to_utf16(text, range.end));
        text.get(range).map(str::to_string)
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let (anchor, head) = self.head_anchor()?;
        let text = self.ime_text();
        // Cross-block selections are reported as the caret leaf's part.
        let in_leaf = |p: BlockPosition| {
            if p.block == head.block {
                p.offset
            } else if self.model.doc.cmp_positions(p, head).ok() == Some(std::cmp::Ordering::Less) {
                0
            } else {
                text.len()
            }
        };
        let (a, h) = (in_leaf(anchor), head.offset);
        Some(UTF16Selection {
            range: byte_to_utf16(text, a.min(h))..byte_to_utf16(text, a.max(h)),
            reversed: h < a,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        let m = self.marked.as_ref()?;
        let text = self.model.doc.leaf_text(m.block)?;
        Some(byte_to_utf16(text, m.range.start)..byte_to_utf16(text, m.range.end))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.end_composition();
    }

    fn paste(&mut self, item: ClipboardItem, _: &mut Window, cx: &mut Context<Self>) {
        BlockEditorState::paste(self, item, false, cx);
    }

    fn replace_text_in_range(
        &mut self,
        range16: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.readonly {
            return;
        }
        let composing = self.marked.is_some();
        self.select_marked_or(range16);
        let origin = if composing {
            EditOrigin::Ime
        } else {
            EditOrigin::Typing
        };
        self.edit(origin, |m| m.insert_text(text, origin), cx);
        self.end_composition();
        if text == " " {
            self.apply_input_rule(cx);
        } else if text.ends_with(['`', '*', '~']) && self.rule_on(RuleKind::InlineMarks) {
            self.apply_mark_rule(cx);
        } else if text.ends_with(')') && self.rule_on(RuleKind::Link) {
            self.apply_link_rule(cx);
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range16: Option<Range<usize>>,
        new_text: &str,
        new_selected16: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.readonly {
            return;
        }
        if new_text.is_empty() {
            // Cancelled composition: remove the marked text, close the group.
            if let Some(marked) = self.marked.clone() {
                self.select_marked_or(None);
                if !marked.range.is_empty() {
                    self.edit(EditOrigin::Ime, |m| m.delete(false), cx);
                }
            }
            self.end_composition();
            return;
        }
        self.select_marked_or(range16);
        let start = match self.model.doc.resolve_selection(self.model.selection()) {
            Ok(range) => range.start,
            Err(_) => return,
        };
        if !self.composition_open {
            // The whole composition is one undo step. Opened only once
            // something is about to be inserted; always closed by
            // `end_composition`.
            self.model.history.begin_transaction();
            self.composition_open = true;
        }
        let revision = self.model.revision();
        self.edit(
            EditOrigin::Ime,
            |m| m.insert_text(new_text, EditOrigin::Ime),
            cx,
        );
        let inserted = self.model.revision() != revision;
        let Some((_, head)) = self.head_anchor().filter(|_| inserted) else {
            // Nothing was inserted (e.g. into a code block): close the group.
            self.end_composition();
            return;
        };
        let block = head.block;
        let begin = if block == start.block {
            start.offset
        } else {
            head.offset.saturating_sub(new_text.len())
        };
        let range = begin..begin + new_text.len();
        self.marked = Some(Marked {
            block,
            range: range.clone(),
        });
        if let Some(sel16) = new_selected16 {
            let sel = utf16_to_byte(new_text, sel16.start)..utf16_to_byte(new_text, sel16.end);
            let _ = self.model.set_selection(BlockSelection::Text {
                anchor: BlockPosition::new(block, range.start + sel.start),
                head: BlockPosition::new(block, range.start + sel.end),
            });
        }
    }

    fn bounds_for_range(
        &mut self,
        range16: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let block = self.ime_block()?;
        let text = self.ime_text();
        let start = utf16_to_byte(text, range16.start);
        let end = utf16_to_byte(text, range16.end);
        let frame = self.frame.borrow();
        let width = self.style.caret_width();
        let a = frame.caret_bounds(BlockPosition::new(block, start), width)?;
        let b = frame.caret_bounds(BlockPosition::new(block, end), width)?;
        Some(Bounds::from_corners(
            a.origin,
            gpui::point(
                b.origin.x.max(a.origin.x + width),
                a.origin.y + a.size.height,
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        at: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let pos = self.frame.borrow().hit_test(at)?;
        if Some(pos.block) != self.ime_block() {
            return None;
        }
        Some(byte_to_utf16(self.ime_text(), pos.offset))
    }
}

/// The byte range of the contiguous run linking to `url` around `offset`.
fn link_extent(
    inline: &super::inline::InlineText,
    offset: usize,
    url: &SharedString,
) -> Option<Range<usize>> {
    let pieces: Vec<_> = inline
        .pieces()
        .map(|(range, marks, _)| (range, marks.link.as_ref().map(|l| l.url.clone())))
        .collect();
    let ix = pieces
        .iter()
        .position(|(r, _)| r.start <= offset && offset < r.end)?;
    let same = |i: usize| pieces[i].1.as_ref() == Some(url);
    let (mut lo, mut hi) = (ix, ix);
    while lo > 0 && same(lo - 1) {
        lo -= 1;
    }
    while hi + 1 < pieces.len() && same(hi + 1) {
        hi += 1;
    }
    Some(pieces[lo].0.start..pieces[hi].0.end)
}

/// `[text](url)` ending at `caret`: the `[` offset, the text and the url.
fn link_rule(text: &str, caret: usize) -> Option<(usize, String, String)> {
    let before = text.get(..caret)?.strip_suffix(')')?;
    let open_paren = before.rfind("](")?;
    let url = &before[open_paren + 2..];
    if url.is_empty() || url.contains(char::is_whitespace) {
        return None;
    }
    let head = &before[..open_paren];
    let open = head.rfind('[')?;
    let label = &head[open + 1..];
    if label.is_empty() || label.contains(['[', ']']) {
        return None;
    }
    Some((open, label.to_string(), url.to_string()))
}

/// The language of a typed code fence (```` ``` ```` plus an optional word).
fn code_fence(text: &str) -> Option<String> {
    let lang = text.strip_prefix("```")?;
    lang.chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '+' | '-' | '_' | '#' | '.'))
        .then(|| lang.to_string())
}

/// The `(opening delimiter, closing delimiter, mark)` of an inline-mark input
/// rule whose closing delimiter ends at `caret`, if any. Nothing fires inside
/// inline code, across an atom, or around empty / space-padded content.
fn mark_rule(
    inline: &super::inline::InlineText,
    caret: usize,
) -> Option<(Range<usize>, Range<usize>, MarkKind)> {
    let text = inline.text();
    let prefix = text.get(..caret)?;
    let (delim, kind) = if prefix.ends_with('`') {
        ("`", MarkKind::Code)
    } else if prefix.ends_with("~~") {
        ("~~", MarkKind::Strikethrough)
    } else if prefix.ends_with("**") {
        ("**", MarkKind::Bold)
    } else if prefix.ends_with('*') {
        ("*", MarkKind::Italic)
    } else {
        return None;
    };
    let close = caret - delim.len()..caret;
    let before = &prefix[..close.start];
    let start = before.rfind(delim)?;
    // `*` must not be half of a `**`.
    if delim == "*" && (before[..start].ends_with('*') || before.ends_with('*')) {
        return None;
    }
    let open = start..start + delim.len();
    let content = &text[open.end..close.start];
    if content.is_empty()
        || content.contains(super::inline::ATOM_CHAR)
        || (kind != MarkKind::Code
            && (content.starts_with(char::is_whitespace) || content.ends_with(char::is_whitespace)))
    {
        return None;
    }
    let in_code = inline
        .pieces()
        .any(|(r, m, _)| m.code && r.start < close.end && r.end > open.start);
    (!in_code).then_some((open, close, kind))
}
