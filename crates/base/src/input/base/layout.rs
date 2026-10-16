use std::{ops::Range, rc::Rc};

use gpui::{Bounds, Half, Pixels, ShapedLine, TextAlign, px};

use crate::Side;

use super::{WrappingIndent, display_map::LineLayout};

#[derive(Clone, Default)]
pub(crate) struct WhitespaceIndicators {
    pub(crate) space: ShapedLine,
    pub(crate) tab: ShapedLine,
}

/// A column of the editor's gutter, ordered from the text outward with
/// `gutter_order`.
///
/// The order is the same on either [`Side`], so a gutter on the right is the
/// mirror of one on the left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GutterColumn {
    /// The fold icons.
    FoldIcons,
    /// The line numbers.
    LineNumbers,
    /// The gutter markers of the line decorations.
    Markers,
}

impl GutterColumn {
    /// The columns in their default order, from the text outward.
    pub(super) const DEFAULT_ORDER: [Self; 3] = [Self::FoldIcons, Self::LineNumbers, Self::Markers];

    /// Return every column once, those listed first and the rest in their
    /// default order.
    pub(super) fn order(columns: impl IntoIterator<Item = Self>) -> [Self; 3] {
        let mut order = Self::DEFAULT_ORDER;
        let mut len = 0;
        for column in columns.into_iter().chain(Self::DEFAULT_ORDER) {
            if len == order.len() {
                break;
            }
            if !order[..len].contains(&column) {
                order[len] = column;
                len += 1;
            }
        }
        order
    }
}

/// What a point of an input is over, as it was last painted.
///
/// Returned by [`hit_test`](super::InputBaseState::hit_test). The fields are private
/// and reached through the methods below, so that more can be told about a
/// point without breaking its readers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputHit {
    area: InputHitArea,
    row: Option<usize>,
    spacer: bool,
}

/// The part of an input a point is over.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputHitArea {
    /// The text, at the byte offset a click there places the caret at.
    #[non_exhaustive]
    Text { offset: usize },
    /// The gutter, in one of its columns, or `None` in its margin or padding.
    #[non_exhaustive]
    Gutter { column: Option<GutterColumn> },
}

impl InputHit {
    pub(crate) fn new(area: InputHitArea, row: Option<usize>, spacer: bool) -> Self {
        Self { area, row, spacer }
    }

    pub fn area(&self) -> InputHitArea {
        self.area
    }

    /// The zero-based buffer row the point is beside.
    ///
    /// In the text it is the row of [`Self::offset`]. In the gutter it is
    /// `None` above the first laid-out row and below the last.
    pub fn row(&self) -> Option<usize> {
        self.row
    }

    /// The byte offset in the text, when the point is over the text.
    pub fn offset(&self) -> Option<usize> {
        match self.area {
            InputHitArea::Text { offset } => Some(offset),
            InputHitArea::Gutter { .. } => None,
        }
    }

    pub fn is_gutter(&self) -> bool {
        matches!(self.area, InputHitArea::Gutter { .. })
    }

    /// The gutter column the point is in, when it is in one.
    pub fn gutter_column(&self) -> Option<GutterColumn> {
        match self.area {
            InputHitArea::Gutter { column } => column,
            InputHitArea::Text { .. } => None,
        }
    }

    /// Returns true when the point is on a spacer row beside [`Self::row`]
    /// rather than on the row itself.
    pub fn is_spacer(&self) -> bool {
        self.spacer
    }
}

#[derive(Clone)]
pub(super) struct LastLayout {
    pub(super) visible_range: Range<usize>,
    pub(super) visible_buffer_lines: Vec<usize>,
    pub(super) visible_line_byte_offsets: Vec<usize>,
    pub(super) visible_top: Pixels,
    pub(super) visible_range_offset: Range<usize>,
    pub(super) lines: Rc<Vec<LineLayout>>,
    pub(super) line_height: Pixels,
    pub(super) wrap_width: Option<Pixels>,
    pub(super) wrapping_indent: WrappingIndent,
    pub(super) line_number_width: Pixels,
    /// Width reserved between the line numbers and the outer edge of the gutter
    /// for gutter markers, zero when none can be painted. Part of
    /// `line_number_width`.
    pub(super) gutter_marker_width: Pixels,
    /// The buffer row that inline completion ghost lines follow, and their height.
    pub(super) ghost_lines: Option<(usize, Pixels)>,
    /// How many spacer rows stand above and below each laid-out row, in
    /// `visible_buffer_lines` order; empty when the editor has no spacers.
    pub(super) spacer_rows: Vec<(usize, usize)>,
    pub(super) gutter_side: Side,
    pub(super) gutter_order: [GutterColumn; 3],
    /// The x of the text, relative to the input bounds.
    pub(super) text_origin_x: Pixels,
    /// The x of the gutter, relative to the input bounds.
    pub(super) gutter_origin_x: Pixels,
    /// The width of the text, without the gutter and the margin kept from a
    /// scrollbar on the left.
    pub(super) text_width: Pixels,
    /// Width of one space in the editor font.
    ///
    /// Past the end of a line there are no glyphs to hit-test against, so this is the
    /// step used to measure how far past the end a pointer sits.
    pub(super) space_width: Pixels,
    pub(super) cursor_bounds: Option<Bounds<Pixels>>,
    pub(super) text_align: TextAlign,
    pub(super) content_width: Pixels,
}

impl LastLayout {
    pub(crate) fn line(&self, row: usize) -> Option<&LineLayout> {
        let pos = self.visible_buffer_lines.binary_search(&row).ok()?;
        self.lines.get(pos)
    }

    /// The heights of the spacer rows above and below the `ix`th laid-out row.
    pub(super) fn spacers(&self, ix: usize) -> (Pixels, Pixels) {
        self.spacer_rows
            .get(ix)
            .map_or((px(0.), px(0.)), |&(above, below)| {
                (self.line_height * above, self.line_height * below)
            })
    }

    /// The top and height of each laid-out row, in `visible_buffer_lines` order.
    ///
    /// Tops are relative to the content origin and, as painted, rows after the
    /// ghost lines are shifted down by their height. A row's spacers are not
    /// part of its height; ghost lines stand between its text and the spacers
    /// below it.
    pub(super) fn row_extents(&self) -> impl Iterator<Item = (Pixels, Pixels)> + '_ {
        let mut top = self.visible_top;
        self.lines
            .iter()
            .zip(&self.visible_buffer_lines)
            .enumerate()
            .map(move |(ix, (line, &row))| {
                let (above, below) = self.spacers(ix);
                top += above;
                let height = line.size(self.line_height).height;
                let extent = (top, height);
                top += height;
                if let Some((ghost_row, ghost_height)) = self.ghost_lines
                    && ghost_row == row
                {
                    top += ghost_height;
                }
                top += below;
                extent
            })
    }

    /// The buffer row at `y`, relative to the content origin, and whether `y`
    /// is on one of its spacer rows. `None` above the first row, below the
    /// last and on ghost lines.
    pub(super) fn row_at(&self, y: Pixels) -> Option<(usize, bool)> {
        self.row_extents()
            .zip(&self.visible_buffer_lines)
            .enumerate()
            .find_map(|(ix, ((top, height), &row))| {
                let (above, below) = self.spacers(ix);
                let bottom = top + height;
                let below_top = match self.ghost_lines {
                    Some((ghost_row, ghost_height)) if ghost_row == row => bottom + ghost_height,
                    _ => bottom,
                };
                let spacer =
                    (y >= top - above && y < top) || (y >= below_top && y < below_top + below);
                (spacer || (y >= top && y < bottom)).then_some((row, spacer))
            })
    }

    pub(super) fn alignment_offset(&self, line_width: Pixels) -> Pixels {
        match self.text_align {
            TextAlign::Left => px(0.),
            TextAlign::Center => (self.content_width - line_width).half().max(px(0.)),
            TextAlign::Right => (self.content_width - line_width).max(px(0.)),
        }
    }
}
