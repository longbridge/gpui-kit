use std::collections::HashMap;

use super::{document::EditorDocument, id::BlockId};

/// Caret leaves in document order, plus their rank for O(1) comparison
/// (design §4.5). Rebuilt only after structural steps.
#[derive(Default, Debug)]
pub(crate) struct OrderIndex {
    leaves: Vec<BlockId>,
    rank: HashMap<BlockId, u32>,
}

#[cfg(test)]
thread_local! {
    pub(crate) static REBUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl OrderIndex {
    pub(crate) fn build(doc: &EditorDocument) -> Self {
        #[cfg(test)]
        REBUILDS.with(|count| count.set(count.get() + 1));
        let leaves = doc.walk_leaves();
        let rank = leaves
            .iter()
            .enumerate()
            .map(|(ix, id)| (*id, ix as u32))
            .collect();
        Self { leaves, rank }
    }

    pub(crate) fn leaves(&self) -> &[BlockId] {
        &self.leaves
    }

    pub(crate) fn rank(&self, id: BlockId) -> Option<u32> {
        self.rank.get(&id).copied()
    }
}
