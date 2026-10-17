use super::{
    selection::BlockSelection,
    step::Step,
    transaction::{EditOrigin, Transaction},
};

const MAX_UNDO_ENTRIES: usize = 1000;

/// Mirrors `input::base::undo_manager::EditIntent` (D11).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EditIntent {
    Typing,
    Backspace,
    DeleteForward,
    Atomic,
}

impl From<EditOrigin> for EditIntent {
    fn from(origin: EditOrigin) -> Self {
        match origin {
            EditOrigin::Typing => Self::Typing,
            EditOrigin::Backspace => Self::Backspace,
            EditOrigin::DeleteForward => Self::DeleteForward,
            _ => Self::Atomic,
        }
    }
}

#[derive(Debug)]
pub(crate) struct UndoEntry {
    intent: EditIntent,
    /// Forward steps, in application order (for redo).
    pub(crate) steps: Vec<Step>,
    /// Inverse steps, in the order to apply them (for undo).
    pub(crate) inverse: Vec<Step>,
    pub(crate) selection_before: BlockSelection,
    pub(crate) selection_after: BlockSelection,
}

impl UndoEntry {
    fn absorb(&mut self, tx: Transaction) {
        self.steps.extend(tx.steps);
        let mut inverse = tx.inverse;
        inverse.append(&mut self.inverse);
        self.inverse = inverse;
        self.selection_after = tx.selection_after;
    }
}

/// History with `UndoManager`'s semantics over block transactions: adjacent
/// edits with the same non-atomic intent coalesce until an explicit boundary
/// (caret move, blur); nested begin/commit brackets group edits.
#[derive(Debug, Default)]
pub(crate) struct BlockUndoManager {
    undo: Vec<UndoEntry>,
    redo: Vec<UndoEntry>,
    boundary: bool,
    depth: usize,
    pending: Option<UndoEntry>,
}

impl BlockUndoManager {
    pub(crate) fn record(&mut self, tx: Transaction) {
        if tx.is_empty() {
            self.break_coalescing();
            return;
        }
        let intent = EditIntent::from(tx.origin);
        if self.depth > 0 {
            match &mut self.pending {
                Some(pending) => pending.absorb(tx),
                None => self.pending = Some(Self::entry(tx, EditIntent::Atomic)),
            }
            return;
        }
        self.redo.clear();
        match self.undo.last_mut() {
            Some(last)
                if !self.boundary && intent != EditIntent::Atomic && last.intent == intent =>
            {
                last.absorb(tx)
            }
            _ => {
                self.undo.push(Self::entry(tx, intent));
                if self.undo.len() > MAX_UNDO_ENTRIES {
                    self.undo.remove(0);
                }
            }
        }
        self.boundary = intent == EditIntent::Atomic;
    }

    fn entry(tx: Transaction, intent: EditIntent) -> UndoEntry {
        UndoEntry {
            intent,
            steps: tx.steps,
            inverse: tx.inverse,
            selection_before: tx.selection_before,
            selection_after: tx.selection_after,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn begin_transaction(&mut self) {
        self.depth += 1;
    }

    #[allow(dead_code)]
    pub(crate) fn commit_transaction(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        if self.depth == 0
            && let Some(entry) = self.pending.take()
        {
            self.redo.clear();
            self.undo.push(entry);
            self.boundary = true;
        }
    }

    pub(crate) fn break_coalescing(&mut self) {
        self.boundary = true;
    }

    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub(crate) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub(crate) fn pop_undo(&mut self) -> Option<UndoEntry> {
        self.boundary = true;
        self.undo.pop()
    }

    pub(crate) fn push_redo(&mut self, entry: UndoEntry) {
        self.redo.push(entry);
    }

    pub(crate) fn pop_redo(&mut self) -> Option<UndoEntry> {
        self.boundary = true;
        self.redo.pop()
    }

    pub(crate) fn push_undo(&mut self, entry: UndoEntry) {
        self.undo.push(entry);
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    #[cfg(test)]
    pub(crate) fn last_undo_steps(&self) -> Option<&[Step]> {
        self.undo.last().map(|entry| entry.steps.as_slice())
    }
}
