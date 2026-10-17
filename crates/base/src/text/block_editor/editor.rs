use std::{
    cell::RefCell,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT_EDITOR_OWNER: AtomicU64 = AtomicU64::new(1);

use gpui::SharedString;

use super::{
    command::CommandError,
    document::{DocumentRef, EditorDocument},
    import::parse_document,
    kind::BlockKind,
    marks::{MarkKind, MarkSet, MarkState},
    position::BlockPosition,
    selection::{BlockSelection, SelectionError},
    transaction::{EditOrigin, Transaction, TransactionBuilder},
    undo::BlockUndoManager,
};
use crate::text::{markdown_ext::MarkdownExtensions, node::TextMark};

/// Headless editor facade (spike name). Phase 3 wraps this in the
/// `BlockEditorState` entity.
pub struct BlockEditorModel {
    pub(crate) doc: EditorDocument,
    pub(super) owner: u64,
    pub(super) selection_epoch: u64,
    pub(super) source_cache: RefCell<Option<super::SourceSnapshot>>,
    pub(super) prefix_cache: RefCell<Option<super::host::PrefixSnapshot>>,
    pub(crate) selection: BlockSelection,
    /// Toolbar marks at a collapsed caret; cleared when the caret moves.
    pub(crate) stored_marks: Option<TextMark>,
    pub(crate) history: BlockUndoManager,
}

impl BlockEditorModel {
    /// Parse Markdown into a model.
    pub fn from_markdown(
        source: &str,
        extensions: MarkdownExtensions,
    ) -> Result<Self, SharedString> {
        let extensions = super::import::preserve_source_extensions(extensions);
        let doc = parse_document(source, Arc::new(extensions))?;
        let selection = BlockSelection::caret(doc.first_caret());
        Ok(Self {
            doc,
            owner: NEXT_EDITOR_OWNER.fetch_add(1, Ordering::Relaxed),
            selection_epoch: 0,
            source_cache: RefCell::default(),
            prefix_cache: RefCell::default(),
            selection,
            stored_marks: None,
            history: BlockUndoManager::default(),
        })
    }

    /// Replace the content. Fresh ids (old positions fail with
    /// `UnknownBlock`), history cleared.
    pub fn set_source(&mut self, source: &str) -> Result<(), SharedString> {
        let revision = self.doc.revision;
        self.doc = parse_document(source, self.doc.extensions.clone())?;
        self.doc.revision = revision + 1;
        self.selection = BlockSelection::caret(self.doc.first_caret());
        self.stored_marks = None;
        self.history.clear();
        Ok(())
    }

    /// The document as canonical Markdown.
    pub fn source(&self) -> String {
        self.source_snapshot().text().to_string()
    }

    /// Document revision; bumped by every committed transaction.
    pub fn revision(&self) -> u64 {
        self.doc.revision
    }

    /// Read-only view of the block tree.
    pub fn document(&self) -> DocumentRef<'_> {
        DocumentRef { doc: &self.doc }
    }

    /// The current selection.
    pub fn selection(&self) -> &BlockSelection {
        &self.selection
    }

    /// Set the selection after validating it against the document.
    pub fn set_selection(&mut self, selection: BlockSelection) -> Result<(), SelectionError> {
        self.doc.validate_selection(&selection)?;
        if selection != self.selection {
            self.selection_epoch += 1;
            self.stored_marks = None;
            self.history.break_coalescing();
        }
        self.selection = selection;
        Ok(())
    }

    /// The caret (head of a text selection); `None` for a block selection.
    pub fn caret(&self) -> Option<BlockPosition> {
        match &self.selection {
            BlockSelection::Text { head, .. } => Some(*head),
            BlockSelection::Blocks { .. } | BlockSelection::Cells { .. } => None,
        }
    }

    /// Marks the next typed text gets (stored marks, else inherited at the caret).
    pub fn active_marks(&self) -> MarkSet {
        if let Some(stored) = &self.stored_marks {
            return MarkSet(stored.clone());
        }
        let Some(caret) = self.caret() else {
            return MarkSet::default();
        };
        MarkSet(self.inherited_marks(caret))
    }

    /// Whether `kind` is on, off or mixed over the selection.
    pub fn mark_state(&self, kind: MarkKind) -> MarkState {
        let Ok(range) = self.doc.resolve_selection(&self.selection) else {
            return MarkState::Off;
        };
        if range.is_collapsed() {
            return if self.active_marks().has(kind) {
                MarkState::On
            } else {
                MarkState::Off
            };
        }
        let (mut on, mut off) = (false, false);
        for (block, lo, hi) in self.text_ranges(range) {
            if let Some(inline) = self.doc.inline(block) {
                for (piece, marks, _) in inline.pieces() {
                    if piece.start < hi && piece.end > lo {
                        if kind.is_set(marks) {
                            on = true
                        } else {
                            off = true
                        }
                    }
                }
            }
        }
        match (on, off) {
            (true, false) => MarkState::On,
            (true, true) => MarkState::Mixed,
            _ => MarkState::Off,
        }
    }

    /// Kind of the text block holding the caret, if any.
    pub fn block_kind_at_caret(&self) -> Option<BlockKind> {
        let caret = self.caret()?;
        self.doc.kind(caret.block).map(|kind| kind.public())
    }

    /// End the current typing group, so the next edit is its own undo step.
    pub fn break_coalescing(&mut self) {
        self.history.break_coalescing();
    }

    /// Whether there is something to undo.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Whether there is something to redo.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Undo one group; returns whether anything changed.
    pub fn undo(&mut self) -> bool {
        let Some(entry) = self.history.pop_undo() else {
            return false;
        };
        if Transaction::replay(&entry.inverse, &mut self.doc).is_err() {
            return false;
        }
        self.selection = self.valid_or_start(entry.selection_before.clone());
        self.stored_marks = None;
        self.history.push_redo(entry);
        true
    }

    /// Redo one group; returns whether anything changed.
    pub fn redo(&mut self) -> bool {
        let Some(entry) = self.history.pop_redo() else {
            return false;
        };
        if Transaction::replay(&entry.steps, &mut self.doc).is_err() {
            return false;
        }
        self.selection = self.valid_or_start(entry.selection_after.clone());
        self.stored_marks = None;
        self.history.push_undo(entry);
        true
    }

    fn valid_or_start(&self, selection: BlockSelection) -> BlockSelection {
        match self.doc.validate_selection(&selection) {
            Ok(()) => selection,
            Err(_) => match selection {
                BlockSelection::Text { head, .. } => BlockSelection::caret(self.doc.clamp(head)),
                BlockSelection::Cells { head, .. } if self.doc.get(head).is_some() => {
                    BlockSelection::caret(BlockPosition::new(head, 0))
                }
                BlockSelection::Blocks { .. } | BlockSelection::Cells { .. } => {
                    BlockSelection::caret(self.doc.first_caret())
                }
            },
        }
    }

    /// Run `build` as one transaction: normalize, commit, record history.
    /// `build` returns the caret to leave behind. On error nothing changes.
    pub(crate) fn transact(
        &mut self,
        origin: EditOrigin,
        build: impl FnOnce(&mut TransactionBuilder<'_>) -> Result<Option<BlockSelection>, CommandError>,
    ) -> Result<bool, CommandError> {
        let before = self.selection.clone();
        let mut tx = TransactionBuilder::new(&mut self.doc);
        let after = match build(&mut tx) {
            Ok(after) => after,
            Err(err) => {
                tx.rollback();
                return Err(err);
            }
        };
        let tx = tx
            .finish(origin, before.clone(), before.clone())
            .map_err(CommandError::from)?;
        let changed = !tx.is_empty();
        // Without an explicit caret, map the old selection through the steps.
        let after = after
            .or_else(|| tx.map_selection(&before))
            .unwrap_or(before);
        let after = self.valid_or_start(after);
        let mut tx = tx;
        tx.selection_after = after.clone();
        self.selection = after;
        self.history.record(tx);
        Ok(changed)
    }

    #[cfg(test)]
    pub(crate) fn doc(&self) -> &EditorDocument {
        &self.doc
    }
}
