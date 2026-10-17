//! Off-screen culling of top-level blocks (light virtualization).
//!
//! Each frame, rendered top-level blocks are wrapped in a pass-through
//! [`Measured`] element that records their window-space top and bottom.
//! The next render lays out only the blocks within one viewport of the
//! visible part of the editor (the content mask at prepaint) plus the blocks
//! holding the selection ends. Runs of the others become one spacer sized
//! from the measured heights (or an estimate). Navigation into a culled
//! block falls back to the model (`BlockEditorState::vertical`).

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, Font, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, TextOverflow, TextStyle, WhiteSpace, Window, px,
};

use super::id::BlockId;

/// Estimated height of a never-measured top-level block.
pub(crate) const ESTIMATE: Pixels = px(28.);

/// Prefix sums of the cull units' heights (a Fenwick tree), so a frame
/// finds the visible units and sizes the spacers in `O(log n)` instead of
/// summing every unit (Phase 5, step 1). Rebuilt when the unit list changes
/// (a new document revision); a measurement updates one entry.
#[derive(Default)]
pub(crate) struct HeightIndex {
    /// Document revision the units were built for.
    revision: Option<u64>,
    /// Unit index of every block a unit covers (its id, or every item of a
    /// list chunk), for pinning and measurement updates.
    pub(crate) unit_of: HashMap<BlockId, usize>,
    /// Each unit's own id.
    ids: Vec<BlockId>,
    heights: Vec<Pixels>,
    tree: Vec<f32>,
    /// Rebuild count (tests: a frame without an edit must not rebuild).
    pub(crate) rebuilds: usize,
}

impl HeightIndex {
    pub(crate) fn len(&self) -> usize {
        self.heights.len()
    }

    pub(crate) fn is_current(&self, revision: u64) -> bool {
        self.revision == Some(revision)
    }

    /// Rebuild for `units` (each unit's id and the blocks it covers).
    pub(crate) fn rebuild<'a>(
        &mut self,
        revision: u64,
        units: impl Iterator<Item = (BlockId, &'a [BlockId])>,
        heights: &HashMap<BlockId, Pixels>,
    ) {
        self.revision = Some(revision);
        self.rebuilds += 1;
        self.unit_of.clear();
        self.ids.clear();
        self.heights.clear();
        for (ix, (id, covered)) in units.enumerate() {
            self.ids.push(id);
            self.heights
                .push(heights.get(&id).copied().unwrap_or(ESTIMATE));
            self.unit_of.insert(id, ix);
            for block in covered {
                self.unit_of.insert(*block, ix);
            }
        }
        // O(n) Fenwick construction.
        let n = self.heights.len();
        self.tree = vec![0.; n + 1];
        for i in 1..=n {
            self.tree[i] += f32::from(self.heights[i - 1]);
            let parent = i + (i.isolate_lowest_one());
            if parent <= n {
                let value = self.tree[i];
                self.tree[parent] += value;
            }
        }
    }

    pub(crate) fn height(&self, ix: usize) -> Pixels {
        self.heights[ix]
    }

    /// Set the height of the unit whose id is `id`.
    pub(crate) fn set(&mut self, id: BlockId, height: Pixels) {
        let Some(&ix) = self.unit_of.get(&id) else {
            return;
        };
        if self.ids[ix] != id {
            return;
        }
        let delta = f32::from(height) - f32::from(self.heights[ix]);
        self.heights[ix] = height;
        let mut i = ix + 1;
        while i < self.tree.len() {
            self.tree[i] += delta;
            i += i.isolate_lowest_one();
        }
    }

    /// Sum of the heights of units `0..ix`.
    pub(crate) fn prefix(&self, ix: usize) -> Pixels {
        let mut i = ix.min(self.len());
        let mut sum = 0.;
        while i > 0 {
            sum += self.tree[i];
            i -= i.isolate_lowest_one();
        }
        px(sum)
    }

    /// The unit containing editor-local `y`: the first `ix` whose bottom
    /// (`prefix(ix + 1)`) is past `y`; `len()` when `y` is past the end.
    pub(crate) fn find(&self, y: Pixels) -> usize {
        let n = self.len();
        let mut pos = 0;
        let mut rest = f32::from(y);
        let mut step = n.checked_next_power_of_two().unwrap_or(0);
        if step > n {
            step /= 2;
        }
        while step > 0 {
            let next = pos + step;
            if next <= n && self.tree[next] <= rest {
                pos = next;
                rest -= self.tree[next];
            }
            step /= 2;
        }
        pos
    }
}

/// One cull unit, cached per document revision (Phase 5, step 2) so a
/// frame does not walk every top-level block: a top-level node, or a chunk
/// of a long top-level list.
pub(crate) struct UnitDesc {
    pub(crate) id: BlockId,
    /// `snapshot.blocks[block].nodes[node]`.
    pub(crate) block: usize,
    pub(crate) node: usize,
    /// Index of the node among all top-level nodes.
    pub(crate) node_ix: usize,
    /// For a list chunk: its item range and the list's item ids.
    pub(crate) items: Option<(std::ops::Range<usize>, Rc<[BlockId]>)>,
    pub(crate) is_first: bool,
    pub(crate) prev: crate::text::document::PrevBlock,
}

#[derive(Default)]
pub(crate) struct CullState {
    pub(crate) index: HeightIndex,
    /// The cull units of `index`'s revision.
    pub(crate) units: Vec<UnitDesc>,
    /// Top-level node count of that revision.
    pub(crate) node_count: usize,
    /// Top-to-next-top distance of each measured top-level block.
    pub(crate) heights: HashMap<BlockId, Pixels>,
    /// Visible span of the editor in editor-local y, from the last prepaint.
    pub(crate) visible: Option<(Pixels, Pixels)>,
    /// Blocks measured in the current frame: (id, index, top, bottom).
    pub(crate) measured: Vec<(BlockId, usize, Pixels, Pixels)>,
    /// Editor-local y span that the current frame actually laid out.
    pub(crate) rendered_span: Option<(Pixels, Pixels)>,
    /// Number of top-level blocks laid out in the last render (perf tests).
    pub(crate) rendered: usize,
    /// Layout metrics only: paint changes must not discard heights or scroll.
    geometry: Option<Geometry>,
}

#[derive(PartialEq)]
struct Geometry {
    width: Pixels,
    rem_size: Pixels,
    font: Font,
    font_size: Pixels,
    line_height: Pixels,
    white_space: WhiteSpace,
    text_overflow: Option<TextOverflow>,
    line_clamp: Option<usize>,
}

impl CullState {
    /// Drop layout-dependent measurements when wrapping or zoom changes.
    /// The current frame's spacers are stale; the next render measures all
    /// units before culling resumes.
    pub(crate) fn set_geometry(
        &mut self,
        width: Pixels,
        rem_size: Pixels,
        typography: &TextStyle,
    ) -> bool {
        let geometry = Geometry {
            width,
            rem_size,
            font: typography.font(),
            font_size: typography.font_size.to_pixels(rem_size),
            line_height: typography.line_height_in_pixels(rem_size),
            white_space: typography.white_space,
            text_overflow: typography.text_overflow.clone(),
            line_clamp: typography.line_clamp,
        };
        let changed = self
            .geometry
            .as_ref()
            .is_some_and(|previous| previous != &geometry);
        self.geometry = Some(geometry);
        if changed {
            self.heights.clear();
            self.index.revision = None;
            self.visible = None;
            self.measured.clear();
            return true;
        }
        false
    }

    /// After prepaint: fold this frame's measurements into the heights and
    /// record the visible span. Returns whether the visible span left the
    /// laid-out span (the caller then requests another frame).
    pub(crate) fn finish_frame(&mut self, editor: Bounds<Pixels>, mask: Bounds<Pixels>) -> bool {
        let measured = std::mem::take(&mut self.measured);
        for (ix, (id, index, top, bottom)) in measured.iter().enumerate() {
            let height = match measured.get(ix + 1) {
                Some((_, next_index, next_top, _)) if *next_index == index + 1 => *next_top - *top,
                _ => self
                    .heights
                    .get(id)
                    .copied()
                    .unwrap_or(*bottom - *top)
                    .max(*bottom - *top),
            };
            self.heights.insert(*id, height);
            self.index.set(*id, height);
        }
        let top = (mask.top() - editor.top()).max(px(0.));
        let bottom = (mask.bottom() - editor.top()).min(editor.size.height);
        self.visible = Some((top, bottom.max(top)));
        match (self.rendered_span, measured.first(), measured.last()) {
            (_, Some(first), Some(last)) => {
                let span = (first.2 - editor.top(), last.3 - editor.top());
                top < span.0 - px(1.) || bottom > span.1 + px(1.)
            }
            _ => false,
        }
    }
}

/// Pass-through wrapper that reports its child's bounds.
pub(crate) struct Measured {
    pub(crate) id: BlockId,
    pub(crate) index: usize,
    pub(crate) state: Rc<RefCell<CullState>>,
    pub(crate) child: Option<AnyElement>,
}

impl IntoElement for Measured {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for Measured {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, AnyElement) {
        let mut child = self
            .child
            .take()
            .unwrap_or_else(|| gpui::Empty.into_any_element());
        (child.request_layout(window, cx), child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.state
            .borrow_mut()
            .measured
            .push((self.id, self.index, bounds.top(), bounds.bottom()));
        child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        child: &mut AnyElement,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_index_prefix_find_and_update() {
        let ids: Vec<BlockId> = (0..100).map(|_| BlockId::next()).collect();
        let mut heights = HashMap::new();
        for (i, id) in ids.iter().enumerate() {
            heights.insert(*id, px(10. + (i % 3) as f32));
        }
        let mut index = HeightIndex::default();
        index.rebuild(1, ids.iter().map(|id| (*id, &[][..])), &heights);
        let naive = |hs: &[f32], ix: usize| hs[..ix].iter().sum::<f32>();
        let mut hs: Vec<f32> = (0..100).map(|i| 10. + (i % 3) as f32).collect();
        for ix in [0, 1, 7, 50, 99, 100] {
            assert!((f32::from(index.prefix(ix)) - naive(&hs, ix)).abs() < 0.01);
        }
        index.set(ids[42], px(500.));
        hs[42] = 500.;
        for ix in [42, 43, 100] {
            assert!((f32::from(index.prefix(ix)) - naive(&hs, ix)).abs() < 0.01);
        }
        for y in [0., 5., 10.5, 400., 1000., 1050., 1500., 99999.] {
            let found = index.find(px(y));
            let want = (0..100).find(|ix| naive(&hs, ix + 1) > y).unwrap_or(100);
            assert_eq!(found, want, "y = {y}");
        }
        assert!(index.is_current(1) && !index.is_current(2));
    }
}
