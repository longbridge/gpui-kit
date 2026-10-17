//! Per-frame geometry, collected through the renderer seam
//! (`text::editor_seam`, design D3).
//!
//! During the editor's prepaint every keyed text leaf reports its
//! `TextLayout` into a [`FrameSink`]; the sink also hands each leaf its
//! selection backgrounds, which `Inline` paints through its existing
//! `range_backgrounds` path. After prepaint the sink becomes the editor's
//! [`FrameLayout`], which handlers hit-test against until the next frame.

use std::{cell::RefCell, collections::HashMap, ops::Range, rc::Rc};

use gpui::{Bounds, Hsla, Pixels, Point, TextLayout, point, px, size};

use super::{id::BlockId, position::BlockPosition, projection::LeafInfo};
use crate::text::editor_seam::EditorSink;

/// Frame-local sink installed while the editor's children prepaint.
pub(crate) struct FrameSink {
    layouts: RefCell<HashMap<u64, Vec<Piece>>>,
    backgrounds: HashMap<u64, Vec<(Range<usize>, Hsla)>>,
}

impl FrameSink {
    pub(crate) fn new(backgrounds: HashMap<u64, Vec<(Range<usize>, Hsla)>>) -> Self {
        Self {
            layouts: RefCell::default(),
            backgrounds,
        }
    }

    pub(crate) fn into_frame(
        self,
        leaves: Rc<Vec<LeafInfo>>,
        tables: &[(BlockId, Vec<BlockId>)],
    ) -> FrameLayout {
        let mut layouts = self.layouts.into_inner();
        layouts.retain(|_, pieces| !pieces.is_empty());
        let mut frame = FrameLayout::default();
        for (table, cells) in tables {
            let bounds = cells
                .iter()
                .filter_map(|cell| layouts.get(&cell.as_u64()).map(|p| pieces_bounds(p)))
                .reduce(|a, b| a.union(&b));
            if let Some(bounds) = bounds {
                frame.tables.push((*table, bounds));
            }
        }
        for info in leaves.iter() {
            if let Some(pieces) = layouts.remove(&info.block.as_u64()) {
                frame.index.insert(info.block, frame.leaves.len());
                frame.leaves.push(LeafGeom {
                    info: *info,
                    pieces,
                });
            }
        }
        frame
    }
}

impl EditorSink for FrameSink {
    fn report(&self, key: u64, layout: &TextLayout) {
        self.layouts
            .borrow_mut()
            .insert(key, vec![Piece::Whole(layout.clone())]);
    }

    fn begin_pieces(&self, key: u64) {
        self.layouts.borrow_mut().insert(key, Vec::new());
    }

    fn report_text_piece(&self, key: u64, start: usize, layout: &TextLayout) {
        self.layouts
            .borrow_mut()
            .entry(key)
            .or_default()
            .push(Piece::Text {
                start,
                layout: layout.clone(),
            });
    }

    fn report_box_piece(&self, key: u64, range: Range<usize>, bounds: Bounds<Pixels>) {
        self.layouts
            .borrow_mut()
            .entry(key)
            .or_default()
            .push(Piece::Atom { range, bounds });
    }

    fn backgrounds(&self, key: u64) -> Vec<(Range<usize>, Hsla)> {
        self.backgrounds.get(&key).cloned().unwrap_or_default()
    }
}

/// How a leaf was laid out: one `TextLayout` for the whole leaf, or (for
/// a mixed paragraph rendered by `InlineFlow`) single-line text fragments
/// and atom boxes in leaf byte order.
#[derive(Clone)]
pub(crate) enum Piece {
    Whole(TextLayout),
    Text {
        start: usize,
        layout: TextLayout,
    },
    Atom {
        range: Range<usize>,
        bounds: Bounds<Pixels>,
    },
}

impl Piece {
    fn bounds(&self) -> Bounds<Pixels> {
        match self {
            Piece::Whole(layout) | Piece::Text { layout, .. } => layout.bounds(),
            Piece::Atom { bounds, .. } => *bounds,
        }
    }

    fn range(&self) -> Range<usize> {
        match self {
            Piece::Whole(layout) => 0..layout.len(),
            Piece::Text { start, layout } => *start..start + layout.len(),
            Piece::Atom { range, .. } => range.clone(),
        }
    }
}

fn pieces_bounds(pieces: &[Piece]) -> Bounds<Pixels> {
    pieces
        .iter()
        .map(Piece::bounds)
        .reduce(|a, b| a.union(&b))
        .unwrap_or_default()
}

pub(crate) struct LeafGeom {
    pub(crate) info: LeafInfo,
    pub(crate) pieces: Vec<Piece>,
}

impl LeafGeom {
    fn whole(&self) -> Option<&TextLayout> {
        match self.pieces.as_slice() {
            [Piece::Whole(layout)] => Some(layout),
            _ => None,
        }
    }

    pub(crate) fn bounds(&self) -> Bounds<Pixels> {
        pieces_bounds(&self.pieces)
    }

    pub(crate) fn line_height(&self) -> Pixels {
        self.pieces
            .iter()
            .find_map(|piece| match piece {
                Piece::Whole(layout) | Piece::Text { layout, .. } => Some(layout.line_height()),
                Piece::Atom { .. } => None,
            })
            .unwrap_or_else(|| self.bounds().size.height)
    }

    /// Height of the caret: the font's ascent + descent (like a text
    /// input's caret), not the whole line box, which includes leading.
    pub(crate) fn caret_height(&self) -> Pixels {
        let line_height = self.line_height();
        self.pieces
            .iter()
            .find_map(|piece| match piece {
                Piece::Whole(layout) | Piece::Text { layout, .. } => {
                    let line = layout.line_layout_for_index(0)?;
                    let h = line.unwrapped_layout.ascent + line.unwrapped_layout.descent;
                    (h > Pixels::ZERO).then_some(h)
                }
                Piece::Atom { .. } => None,
            })
            .unwrap_or(line_height * 0.85)
            .min(line_height)
    }

    /// Rendered index for a model offset (whole-leaf layouts).
    fn index(&self, layout: &TextLayout, offset: usize) -> usize {
        if self.info.atomic {
            if offset == 0 { 0 } else { layout.len() }
        } else {
            offset.min(self.info.len).min(layout.len())
        }
    }

    /// Model offset for a rendered index (whole-leaf layouts).
    fn offset(&self, layout: &TextLayout, index: usize) -> usize {
        if self.info.atomic {
            usize::from(index * 2 > layout.len().max(1))
        } else {
            index.min(self.info.len)
        }
    }

    /// Top of the caret at `offset`, window coordinates.
    pub(crate) fn caret_point(&self, offset: usize) -> Point<Pixels> {
        if let Some(layout) = self.whole() {
            return layout
                .position_for_index(self.index(layout, offset))
                .unwrap_or_else(|| self.bounds().origin);
        }
        // The piece the offset starts, else the one it ends (a caret at a
        // fragment boundary sits at the start of the next fragment).
        let piece = self
            .pieces
            .iter()
            .find(|p| p.range().contains(&offset))
            .or_else(|| self.pieces.iter().rev().find(|p| p.range().end == offset))
            .or(self.pieces.last());
        let Some(piece) = piece else {
            return self.bounds().origin;
        };
        match piece {
            Piece::Whole(layout) | Piece::Text { layout, .. } => {
                let index = offset.saturating_sub(piece.range().start).min(layout.len());
                layout
                    .position_for_index(index)
                    .unwrap_or_else(|| layout.bounds().origin)
            }
            Piece::Atom { range, bounds } => {
                let x = if offset >= range.end {
                    bounds.right()
                } else {
                    bounds.left()
                };
                // Align with the text line: an atom taller than the line
                // keeps the caret at its bottom line.
                let y = (bounds.bottom() - self.line_height()).max(bounds.top());
                point(x, y)
            }
        }
    }

    /// Closest model offset for a window point (glyph halves round, unlike
    /// `TextLayout::index_for_position`).
    pub(crate) fn offset_at(&self, at: Point<Pixels>) -> usize {
        if let Some(layout) = self.whole() {
            return self.offset(layout, whole_index_at(layout, at));
        }
        // The visual line under the point (pieces whose vertical extent
        // holds it, else the nearest), then the horizontally nearest piece.
        let dy = |p: &Piece| {
            let b = p.bounds();
            if at.y < b.top() {
                b.top() - at.y
            } else if at.y > b.bottom() {
                at.y - b.bottom()
            } else {
                px(0.)
            }
        };
        let dx = |p: &Piece| {
            let b = p.bounds();
            if at.x < b.left() {
                b.left() - at.x
            } else if at.x > b.right() {
                at.x - b.right()
            } else {
                px(0.)
            }
        };
        let Some(piece) = self.pieces.iter().min_by(|a, b| {
            (dy(a), dx(a))
                .partial_cmp(&(dy(b), dx(b)))
                .unwrap_or(std::cmp::Ordering::Equal)
        }) else {
            return 0;
        };
        let offset = match piece {
            Piece::Whole(layout) => whole_index_at(layout, at),
            // Fragments are one line: take the caret position nearest to the
            // point (`closest_index_for_position` rounds up at an exact
            // boundary inside scaled code text).
            Piece::Text { start, layout } => {
                let nearest = (0..=layout.len())
                    .filter_map(|i| {
                        layout
                            .position_for_index(i)
                            .map(|p| (i, (p.x - at.x).abs()))
                    })
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map_or(0, |(i, _)| i);
                start + nearest
            }
            Piece::Atom { range, bounds } => {
                if at.x < bounds.center().x {
                    range.start
                } else {
                    range.end
                }
            }
        };
        offset.min(self.info.len)
    }

    /// The inline atom drawn under `at`, as its leaf byte range.
    pub(crate) fn atom_at(&self, at: Point<Pixels>) -> Option<Range<usize>> {
        self.pieces.iter().find_map(|piece| match piece {
            Piece::Atom { range, bounds } if bounds.contains(&at) => Some(range.clone()),
            _ => None,
        })
    }
}

/// Closest index in a `TextLayout` for a window point.
fn whole_index_at(layout: &TextLayout, at: Point<Pixels>) -> usize {
    let bounds = layout.bounds();
    let line_height = layout.line_height();
    let mut top = bounds.top();
    let mut start = 0;
    let lines = layout.line_layouts();
    for (ix, line) in lines.iter().enumerate() {
        let height = line.size(line_height).height.max(line_height);
        let is_last = ix + 1 == lines.len();
        if at.y < top + height || is_last {
            let local = point(
                (at.x - bounds.left()).max(px(0.)),
                (at.y - top).clamp(px(0.), (height - px(1.)).max(px(0.))),
            );
            let index = match line.closest_index_for_position(local, line_height) {
                Ok(ix) | Err(ix) => ix,
            };
            return start + index;
        }
        top += height;
        start += line.len() + 1;
    }
    0
}

/// The last complete frame's leaf geometry, in document order.
#[derive(Default)]
pub(crate) struct FrameLayout {
    pub(crate) leaves: Vec<LeafGeom>,
    pub(crate) index: HashMap<BlockId, usize>,
    /// Table bounds (union of its cells), for node selection by click.
    pub(crate) tables: Vec<(BlockId, Bounds<Pixels>)>,
}

impl FrameLayout {
    pub(crate) fn leaf(&self, block: BlockId) -> Option<&LeafGeom> {
        self.index.get(&block).map(|ix| &self.leaves[*ix])
    }

    pub(crate) fn caret_bounds(&self, pos: BlockPosition, width: Pixels) -> Option<Bounds<Pixels>> {
        let leaf = self.leaf(pos.block)?;
        let (line, h) = (leaf.line_height(), leaf.caret_height());
        let mut origin = leaf.caret_point(pos.offset);
        // GPUI centres the glyphs in the line box: so does the caret.
        origin.y += (line - h) / 2.;
        Some(Bounds::new(origin, size(width, h)))
    }

    /// The table under a window point.
    /// Kept for the block handle (not built yet); clicks in a table place the caret.
    #[allow(dead_code)]
    pub(crate) fn table_at(&self, at: Point<Pixels>) -> Option<BlockId> {
        self.tables
            .iter()
            .find(|(_, bounds)| bounds.contains(&at))
            .map(|(id, _)| *id)
    }

    /// Nearest caret position to a window point.
    pub(crate) fn hit_test(&self, at: Point<Pixels>) -> Option<BlockPosition> {
        let last = self.leaves.len().checked_sub(1)?;
        let ix = self
            .leaves
            .partition_point(|leaf| leaf.bounds().bottom() <= at.y)
            .min(last);
        let ix = self.nearest_in_band(ix, at.x);
        let leaf = &self.leaves[ix];
        Some(BlockPosition::new(leaf.info.block, leaf.offset_at(at)))
    }

    /// Leaves side by side with `ix` (table cells in one row share a
    /// vertical band): the one horizontally nearest to `x`.
    fn nearest_in_band(&self, ix: usize, x: Pixels) -> usize {
        let band = self.leaves[ix].bounds();
        let overlaps = |i: usize| {
            let b = self.leaves[i].bounds();
            b.top() < band.bottom() && b.bottom() > band.top()
        };
        let mut lo = ix;
        while lo > 0 && overlaps(lo - 1) {
            lo -= 1;
        }
        let mut hi = ix;
        while hi + 1 < self.leaves.len() && overlaps(hi + 1) {
            hi += 1;
        }
        let distance = |i: usize| {
            let b = self.leaves[i].bounds();
            if x < b.left() {
                b.left() - x
            } else if x > b.right() {
                x - b.right()
            } else {
                px(0.)
            }
        };
        (lo..=hi)
            .min_by(|a, b| {
                distance(*a)
                    .partial_cmp(&distance(*b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(ix)
    }

    /// One visual line above/below `pos`, keeping `x`.
    pub(crate) fn vertical(
        &self,
        pos: BlockPosition,
        x: Pixels,
        down: bool,
    ) -> Option<BlockPosition> {
        let ix = *self.index.get(&pos.block)?;
        let leaf = &self.leaves[ix];
        let caret = leaf.caret_point(pos.offset);
        let lh = leaf.line_height();
        let y = if down {
            caret.y + lh * 1.5
        } else {
            caret.y - lh * 0.5
        };
        let bounds = leaf.bounds();
        if y >= bounds.top() && y < bounds.bottom() {
            return Some(BlockPosition::new(
                leaf.info.block,
                leaf.offset_at(point(x, y)),
            ));
        }
        // The next leaf above/below that is not side by side with this one
        // (skips the rest of a table row), then the nearest in its band.
        let side_by_side = |i: usize| {
            let b = self.leaves[i].bounds();
            b.top() < bounds.bottom() && b.bottom() > bounds.top()
        };
        let mut j = ix;
        loop {
            j = if down {
                Some(j + 1).filter(|j| *j < self.leaves.len())?
            } else {
                j.checked_sub(1)?
            };
            if !side_by_side(j) {
                break;
            }
        }
        let next = &self.leaves[self.nearest_in_band(j, x)];
        let b = next.bounds();
        let y = if down {
            b.top() + next.line_height() / 2.
        } else {
            b.bottom() - next.line_height() / 2.
        };
        Some(BlockPosition::new(
            next.info.block,
            next.offset_at(point(x, y)),
        ))
    }

    /// Start or end of the visual (soft-wrapped) line holding `pos`.
    pub(crate) fn line_edge(&self, pos: BlockPosition, end: bool) -> Option<BlockPosition> {
        let leaf = self.leaf(pos.block)?;
        let caret = leaf.caret_point(pos.offset);
        let b = leaf.bounds();
        let x = if end {
            b.right() + px(10000.)
        } else {
            b.left() - px(1.)
        };
        let y = caret.y + leaf.line_height() / 2.;
        Some(BlockPosition::new(pos.block, leaf.offset_at(point(x, y))))
    }
}
