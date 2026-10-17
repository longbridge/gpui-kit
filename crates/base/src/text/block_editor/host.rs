//! Host edits use editor-native positions, never canonical-Markdown offsets.
use gpui::SharedString;

use super::{BlockEditorModel, BlockPosition, BlockSelection, CommandError, EditOrigin};

/// Immutable canonical source derived once per document revision. Cloning this
/// value shares the text; it is not another writable document owner.
#[derive(Clone, Debug)]
pub struct SourceSnapshot {
    revision: u64,
    text: SharedString,
}
impl SourceSnapshot {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn text(&self) -> &SharedString {
        &self.text
    }
}

/// A read cache, never an editable source owner.
pub(super) struct PrefixSnapshot {
    pub(super) revision: u64,
    pub(super) selection_epoch: u64,
    pub(super) text: Option<SharedString>,
}

/// An opaque range tied to one editor, revision and navigation epoch. Successful
/// replacement advances the capture to cover just the inserted text. Any other
/// edit, load, undo/redo or navigation invalidates it, even if text is unchanged.
#[derive(Clone, Debug)]
pub struct CapturedRange {
    owner: u64,
    revision: u64,
    selection_epoch: u64,
    range: BlockSelection,
    expected_selection: BlockSelection,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum CapturedRangeError {
    Stale,
    NotText,
    InvalidRange,
    Edit(CommandError),
}

impl BlockEditorModel {
    pub fn source_snapshot(&self) -> SourceSnapshot {
        let revision = self.revision();
        if let Some(snapshot) = &*self.source_cache.borrow()
            && snapshot.revision == revision
        {
            return snapshot.clone();
        }
        let snapshot = SourceSnapshot {
            revision,
            text: super::serialize::serialize(&self.doc).to_owned().into(),
        };
        *self.source_cache.borrow_mut() = Some(snapshot.clone());
        snapshot
    }

    pub(super) fn capture(&self, range: BlockSelection) -> CapturedRange {
        CapturedRange {
            owner: self.owner,
            revision: self.revision(),
            selection_epoch: self.selection_epoch,
            range,
            expected_selection: self.selection.clone(),
        }
    }

    /// Capture the head/caret, inserting there even when text is selected.
    pub fn capture_caret(&self) -> Result<CapturedRange, CapturedRangeError> {
        let caret = self.caret().ok_or(CapturedRangeError::NotText)?;
        Ok(self.capture(BlockSelection::caret(caret)))
    }

    /// Capture the current text selection for one atomic replacement.
    pub fn capture_selection(&self) -> Result<CapturedRange, CapturedRangeError> {
        if !matches!(self.selection, BlockSelection::Text { .. }) {
            return Err(CapturedRangeError::NotText);
        }
        Ok(self.capture(self.selection.clone()))
    }

    /// Text before the caret in its editable leaf. Completion engines inspect
    /// this plain text, not escaped or reformatted canonical source.
    /// Revision/selection-keyed shared native prefix. Repeated query/render
    /// reads do not clone a large caret leaf's bytes.
    pub fn caret_prefix_snapshot(&self) -> Option<SharedString> {
        let mut cache = self.prefix_cache.borrow_mut();
        if cache.as_ref().is_none_or(|cached| {
            cached.revision != self.revision() || cached.selection_epoch != self.selection_epoch
        }) {
            *cache = Some(PrefixSnapshot {
                revision: self.revision(),
                selection_epoch: self.selection_epoch,
                text: self
                    .caret_prefix()
                    .map(|text| SharedString::from(text.to_owned())),
            });
        }
        cache.as_ref().and_then(|cached| cached.text.clone())
    }

    pub fn caret_prefix(&self) -> Option<&str> {
        let caret = self.caret()?;
        self.doc.leaf_text(caret.block)?.get(..caret.offset)
    }

    /// Capture a UTF-8 suffix of the caret's leaf for completion acceptance.
    pub fn capture_suffix(&self, len: usize) -> Result<CapturedRange, CapturedRangeError> {
        let head = self.caret().ok_or(CapturedRangeError::NotText)?;
        let offset = head
            .offset
            .checked_sub(len)
            .ok_or(CapturedRangeError::InvalidRange)?;
        let anchor = BlockPosition::new(head.block, offset);
        self.doc
            .validate(anchor)
            .map_err(|_| CapturedRangeError::InvalidRange)?;
        Ok(self.capture(BlockSelection::Text { anchor, head }))
    }

    /// Capture an exact UTF-8 range in the caret's editing leaf. This range is
    /// measured in plain leaf text, never canonical Markdown. Useful when a
    /// completion token ends before trailing text/newlines in that same leaf.
    pub fn capture_leaf_range(
        &self,
        range: std::ops::Range<usize>,
    ) -> Result<CapturedRange, CapturedRangeError> {
        let caret = self.caret().ok_or(CapturedRangeError::NotText)?;
        if range.start > range.end {
            return Err(CapturedRangeError::InvalidRange);
        }
        let anchor = BlockPosition::new(caret.block, range.start);
        let head = BlockPosition::new(caret.block, range.end);
        self.doc
            .validate(anchor)
            .map_err(|_| CapturedRangeError::InvalidRange)?;
        self.doc
            .validate(head)
            .map_err(|_| CapturedRangeError::InvalidRange)?;
        Ok(self.capture(BlockSelection::Text { anchor, head }))
    }

    pub fn is_capture_current(&self, capture: &CapturedRange) -> bool {
        capture.owner == self.owner
            && capture.revision == self.revision()
            && capture.selection_epoch == self.selection_epoch
            && capture.expected_selection == self.selection
    }

    /// Replace only a captured range with plain text. One undoable transaction
    /// per replacement; does not reparse, apply input rules or submit. Errors
    /// leave both the document and capture intact. The caller retains domain
    /// ownership (session/window/request) and rejects its own stale callbacks.
    pub fn replace_captured(
        &mut self,
        capture: &mut CapturedRange,
        text: &str,
    ) -> Result<(), CapturedRangeError> {
        if !self.is_capture_current(capture) {
            return Err(CapturedRangeError::Stale);
        }
        let range = self
            .doc
            .resolve_selection(&capture.range)
            .map_err(|_| CapturedRangeError::InvalidRange)?;
        let before = self.selection.clone();
        self.selection = capture.range.clone();
        if let Err(error) = self.insert_text(text, EditOrigin::Api) {
            self.selection = before;
            return Err(CapturedRangeError::Edit(error));
        }
        let head = self.caret().ok_or(CapturedRangeError::NotText)?;
        *capture = self.capture(BlockSelection::Text {
            anchor: range.start,
            head,
        });
        Ok(())
    }

    /// Invalidate outstanding host captures without changing document/source.
    /// Hosts call this when the external owner (session/window/request) changes.
    pub fn invalidate_captures(&mut self) {
        self.selection_epoch += 1;
    }

    pub fn select_end(&mut self) -> Result<(), super::SelectionError> {
        let block = *self.doc.leaves().last().unwrap();
        let offset = self.doc.leaf_len(block).unwrap_or(0);
        self.set_selection(BlockSelection::caret(BlockPosition::new(block, offset)))
    }
}
