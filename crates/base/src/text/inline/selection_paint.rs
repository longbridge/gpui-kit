//! Selection backgrounds in shaped, visual-row coordinates.
use super::{GlyphBox, aligned_row_left, glyph_boxes};
use gpui::{Bounds, Pixels, TextAlign, TextLayout, WrappedLineLayout, point};
use std::{ops::Range, sync::Arc};

/// Glyphs in visual order. Logical-order rows allow binary search during a drag.
#[derive(Debug, Default)]
struct Row {
    glyphs: Vec<GlyphBox>,
    ordered: bool,
}

/// Geometry retained while a partial selection moves through the same layout.
#[derive(Debug)]
pub(in crate::text) struct Rows {
    layouts: Vec<Arc<WrappedLineLayout>>,
    width: Pixels,
    align: TextAlign,
    rows: Vec<Row>,
}

impl Rows {
    /// Group glyphs by row and restore visual order for bidirectional text.
    fn build(layout: &TextLayout, layouts: &[Arc<WrappedLineLayout>], align: TextAlign) -> Self {
        let width = layout.bounds().size.width;
        let mut rows = Vec::new();
        for glyph in glyph_boxes(layout, align, width) {
            if rows.len() <= glyph.row {
                rows.resize_with(glyph.row + 1, Row::default);
            }
            rows[glyph.row].glyphs.push(glyph);
        }
        for row in &mut rows {
            row.ordered = row.glyphs.windows(2).all(|w| w[0].left <= w[1].left);
            if !row.ordered {
                row.glyphs.sort_by(|a, b| {
                    a.left
                        .partial_cmp(&b.left)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }
        Self {
            layouts: layouts.to_vec(),
            width,
            align,
            rows,
        }
    }

    /// Scrolling and moving the selection reuse the same shaped lines.
    fn matches(&self, layouts: &[Arc<WrappedLineLayout>], width: Pixels, align: TextAlign) -> bool {
        self.align == align
            && self.width == width
            && self.layouts.len() == layouts.len()
            && self
                .layouts
                .iter()
                .zip(layouts)
                .all(|(a, b)| Arc::ptr_eq(a, b))
    }
}
/// Emit the visible selected spans. Whole inlines only need their wrap boundaries.
pub(super) fn visit(
    layout: &TextLayout,
    align: TextAlign,
    selection: Range<usize>,
    clip: Bounds<Pixels>,
    cache: &mut Option<Box<Rows>>,
    mut paint: impl FnMut(Bounds<Pixels>),
) {
    let bounds = layout.bounds();
    let h = layout.line_height();
    if selection.is_empty() || h <= Pixels::ZERO || !bounds.intersects(&clip) {
        return;
    }
    let first = ((clip.top() - bounds.top()) / h).floor().max(0.) as usize;
    let end = ((clip.bottom() - bounds.top()) / h).ceil().max(0.) as usize;
    let mut emit = |row: usize, left: Pixels, right: Pixels| {
        let y = bounds.top() + h * row as f32;
        let rect = Bounds::from_corners(
            point(bounds.left() + left, y),
            point(bounds.left() + right, y + h),
        )
        .intersect(&clip);
        if rect.size.width > Pixels::ZERO && rect.size.height > Pixels::ZERO {
            paint(rect);
        }
    };
    let lines = layout.line_layouts();
    if selection.start == 0 && selection.end == layout.len() {
        let mut base = 0;
        for line in &lines {
            if base >= end {
                break;
            }
            let count = line.wrap_boundaries.len() + 1;
            let boundary_x = |i: usize| {
                let b = line.wrap_boundaries[i];
                line.unwrapped_layout.runs[b.run_ix].glyphs[b.glyph_ix]
                    .position
                    .x
            };
            for row in first.saturating_sub(base)..end.saturating_sub(base).min(count) {
                let start_x = if row == 0 {
                    Pixels::ZERO
                } else {
                    boundary_x(row - 1)
                };
                let end_x = if row + 1 == count {
                    line.unwrapped_layout.width
                } else {
                    boundary_x(row)
                };
                let width = end_x - start_x;
                let left = aligned_row_left(align, bounds.size.width, width);
                emit(base + row, left, left + width);
            }
            base += count;
        }
        return;
    }
    if cache
        .as_ref()
        .is_none_or(|rows| !rows.matches(&lines, bounds.size.width, align))
    {
        *cache = Some(Box::new(Rows::build(layout, &lines, align)));
    }
    let rows = &cache.as_ref().unwrap().rows;
    for (row, data) in rows.iter().enumerate().take(end).skip(first) {
        if data.ordered {
            let first = data
                .glyphs
                .partition_point(|g| g.text.end <= selection.start);
            let end = data
                .glyphs
                .partition_point(|g| g.text.start < selection.end);
            if first < end {
                emit(row, data.glyphs[first].left, data.glyphs[end - 1].right);
            }
        } else {
            // A contiguous logical selection can cover disjoint visual spans.
            let mut span: Option<(Pixels, Pixels)> = None;
            for glyph in &data.glyphs {
                if glyph.text.start >= selection.end || glyph.text.end <= selection.start {
                    continue;
                }
                match &mut span {
                    Some((_, right)) if glyph.left <= *right => *right = (*right).max(glyph.right),
                    _ => {
                        if let Some((l, r)) = span {
                            emit(row, l, r);
                        }
                        span = Some((glyph.left, glyph.right));
                    }
                }
            }
            if let Some((l, r)) = span {
                emit(row, l, r);
            }
        }
    }
}
