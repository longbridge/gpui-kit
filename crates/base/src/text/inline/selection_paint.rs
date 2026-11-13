//! Paint a selection using the same glyph extents as text range highlights.
use super::{GlyphBox, aligned_row_left, glyph_boxes, range_boxes};
use gpui::{Bounds, Pixels, TextAlign, TextLayout, WrappedLineLayout, point, size};
use std::{ops::Range, sync::Arc};

#[derive(Debug)]
pub(in crate::text) struct Rows {
    lines: Vec<Arc<WrappedLineLayout>>,
    width: Pixels,
    align: TextAlign,
    rows: Vec<Row>,
}

#[derive(Debug, Default)]
struct Row {
    glyphs: Vec<GlyphBox>,
    contiguous: bool,
}

pub(super) fn visit(
    layout: &TextLayout,
    align: TextAlign,
    selection: Range<usize>,
    clip: Bounds<Pixels>,
    cache: &mut Option<Box<Rows>>,
    mut paint: impl FnMut(Bounds<Pixels>),
) {
    let bounds = layout.bounds();
    let height = layout.line_height();
    if selection.is_empty() || height <= Pixels::ZERO || !bounds.intersects(&clip) {
        return;
    }
    let mut emit = |row: usize, left: Pixels, right: Pixels| {
        let origin = bounds.origin + point(left, height * row as f32);
        let rect = Bounds::new(origin, size(right - left, height)).intersect(&clip);
        if rect.size.width > Pixels::ZERO && rect.size.height > Pixels::ZERO {
            paint(rect);
        }
    };
    let lines = layout.line_layouts();
    let first = ((clip.top() - bounds.top()) / height).floor().max(0.) as usize;
    let limit = ((clip.bottom() - bounds.top()) / height).ceil().max(0.) as usize;
    if selection != (0..layout.len()) {
        let width = bounds.size.width;
        let stale = cache.as_ref().is_none_or(|c| {
            c.width != width
                || c.align != align
                || c.lines.len() != lines.len()
                || !c.lines.iter().zip(&lines).all(|(a, b)| Arc::ptr_eq(a, b))
        });
        if stale {
            let mut rows = Vec::new();
            for glyph in glyph_boxes(layout, align, width) {
                if glyph.row >= rows.len() {
                    rows.resize_with(glyph.row + 1, Row::default);
                }
                let row = &mut rows[glyph.row];
                row.contiguous = row
                    .glyphs
                    .last()
                    .is_none_or(|prev| row.contiguous && prev.right == glyph.left);
                row.glyphs.push(glyph);
            }
            *cache = Some(Box::new(Rows {
                lines: lines.to_vec(),
                width,
                align,
                rows,
            }));
        }
        let rows = &cache.as_ref().unwrap().rows;
        for (row, data) in rows.iter().enumerate().take(limit).skip(first) {
            let glyphs = &data.glyphs;
            let (Some(head), Some(tail)) = (glyphs.first(), glyphs.last()) else {
                continue;
            };
            if selection.end <= head.text.start || selection.start >= tail.text.end {
                continue;
            }
            if data.contiguous {
                // Most visible rows lie entirely between the moving selection endpoints.
                let left = if selection.start <= head.text.start {
                    head.left
                } else {
                    glyphs[glyphs.partition_point(|g| g.text.end <= selection.start)].left
                };
                let right = if selection.end >= tail.text.end {
                    tail.right
                } else {
                    glyphs[glyphs.partition_point(|g| g.text.start < selection.end) - 1].right
                };
                emit(row, left, right);
            } else {
                // A logical range can cover disjoint spans on a bidirectional row.
                for (_, left, right) in range_boxes(glyphs, selection.clone()) {
                    emit(row, left, right);
                }
            }
        }
        return;
    }

    // Whole paragraphs need only the existing wrap boundaries, even for bidi text.
    let mut base = 0;
    for line in lines {
        if base >= limit {
            break;
        }
        let shaped = &line.unwrapped_layout;
        let breaks = &line.wrap_boundaries;
        let x = |b: &gpui::WrapBoundary| shaped.runs[b.run_ix].glyphs[b.glyph_ix].position.x;
        for row in first.saturating_sub(base)..limit.saturating_sub(base).min(breaks.len() + 1) {
            let start = row
                .checked_sub(1)
                .map_or(Pixels::ZERO, |prev| x(&breaks[prev]));
            let width = breaks.get(row).map_or(shaped.width, x) - start;
            let left = aligned_row_left(align, bounds.size.width, width);
            emit(base + row, left, left + width);
        }
        base += breaks.len() + 1;
    }
}
