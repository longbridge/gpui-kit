/// DisplayMap: Public facade for Editor/Input display mapping.
///
/// This combines WrapMap, FoldMap and SpacerMap to provide a unified API:
/// - BufferPoint ↔ DisplayPoint conversion
/// - Fold management (candidates, toggle, query)
/// - Automatic projection updates on text/layout changes
use std::ops::Range;

use gpui::{App, Font, Pixels};
use ropey::Rope;

use super::fold_map::FoldMap;
use super::folding::FoldRange;
use super::spacer_map::{DisplayRow, SpacerMap};
pub use super::text_wrapper::WrappingIndent;
use super::text_wrapper::{LineItem, WrapDisplayPoint};
use super::wrap_map::WrapMap;
use super::{BufferPoint, DisplayPoint};
use crate::input::Point as TreeSitterPoint;
use crate::input::Spacer;
use crate::input::display_map::WrapPoint;
use crate::input::rope_ext::RopeExt as _;

/// DisplayMap is the main interface for Editor/Input coordinate mapping.
///
/// It manages the three-layer projection:
/// 1. Buffer → Wrap (soft-wrapping)
/// 2. Wrap → Fold (folding)
/// 3. Fold → Display (spacers)
///
/// [`crate::input::InputBaseState::display_map`] exposes the editor-owned map
/// through a shared reference for application coordinate queries. Text edits,
/// wrapping, and folding remain owned by the editor state.
///
/// Coordinates describe the prepared text projection, including offscreen
/// lines. They exclude inline completion ghost lines and other painted geometry.
pub struct DisplayMap {
    wrap_map: WrapMap,
    fold_map: FoldMap,
    spacer_map: SpacerMap,
    /// The spacers as they were last set, before projection.
    spacers: Vec<Spacer>,
    /// The rows version the spacers were last projected at, `None` when they
    /// must be projected again.
    spacers_projected: Option<usize>,
    /// Counts the changes of the spacers, so that [`Self::version`] moves with
    /// them.
    spacers_version: usize,
}

impl DisplayMap {
    pub fn new(font: Font, font_size: Pixels, wrap_width: Option<Pixels>) -> Self {
        Self {
            wrap_map: WrapMap::new(font, font_size, wrap_width),
            fold_map: FoldMap::new(),
            spacer_map: SpacerMap::default(),
            spacers: Vec::new(),
            spacers_projected: None,
            spacers_version: 0,
        }
    }

    // ==================== Core Coordinate Mapping ====================

    /// Convert buffer position to display position
    pub fn buffer_pos_to_display_pos(&self, pos: BufferPoint) -> DisplayPoint {
        // Buffer → Wrap
        let wrap_pos = self.wrap_map.buffer_pos_to_wrap_pos(pos);

        // Wrap → Fold → Display
        if let Some(fold_row) = self.fold_map.wrap_row_to_display_row(wrap_pos.row) {
            DisplayPoint::new(
                self.spacer_map.fold_row_to_display_row(fold_row),
                wrap_pos.col,
            )
        } else {
            // Cursor is in a folded region, find nearest visible row
            let display_row = self.nearest_visible_display_row(wrap_pos.row);
            DisplayPoint::new(display_row, 0) // Column 0 at fold boundary
        }
    }

    /// Convert display position to buffer position
    ///
    /// A spacer row is read as the nearest row of the line it stands beside.
    pub fn display_pos_to_buffer_pos(&self, pos: DisplayPoint) -> BufferPoint {
        // Display → Fold → Wrap
        let wrap_row = self
            .fold_map
            .display_row_to_wrap_row(self.text_fold_row(pos.row))
            .unwrap_or(0);

        // Wrap → Buffer
        let wrap_pos = WrapPoint::new(wrap_row, pos.col);
        self.wrap_map.wrap_pos_to_buffer_pos(wrap_pos)
    }

    /// A number that changes whenever a buffer line may have taken different
    /// display rows: an edit, a rewrap, a change of folds or of spacers.
    ///
    /// Compare versions for equality on the same map to invalidate cached
    /// queries. The counter can wrap; it is not a document revision or a count
    /// of edits. Scrolling and inline completion ghost lines do not change it,
    /// nor does a [`crate::input::SpacerProvider`] answering with the spacers
    /// already shown.
    #[inline]
    pub fn version(&self) -> usize {
        self.rows_version().wrapping_add(self.spacers_version)
    }

    /// Like [`Self::version`], without the spacers: what the spacers are
    /// placed by and asked for again at, so that an answer does not ask anew.
    #[inline]
    pub(crate) fn rows_version(&self) -> usize {
        self.wrap_map
            .wrapper()
            .version()
            .wrapping_add(self.fold_map.version())
    }

    /// Get total number of visible display rows, spacer rows included
    #[inline]
    pub fn display_row_count(&self) -> usize {
        self.fold_map.display_row_count() + self.spacer_map.row_count()
    }

    /// Get the buffer line for a given display row
    ///
    /// A spacer row belongs to the line it stands beside.
    pub fn display_row_to_buffer_line(&self, display_row: usize) -> usize {
        let fold_row = match self.spacer_map.display_row(display_row) {
            DisplayRow::Text(fold_row) => fold_row,
            DisplayRow::Spacer(index) => return self.spacer_map.run(index).line,
        };

        // Fold → Wrap
        let wrap_row = self.fold_map.display_row_to_wrap_row(fold_row).unwrap_or(0);

        // Wrap → Buffer line
        self.wrap_map.wrap_row_to_buffer_line(wrap_row)
    }

    /// Get the display row range for a buffer line: [start, end)
    /// Returns None if the buffer line is completely hidden
    ///
    /// The range holds the rows of the line's text, not its spacers.
    pub fn buffer_line_to_display_row_range(&self, line: usize) -> Option<Range<usize>> {
        let rows = self.fold_row_range(line)?;
        Some(
            self.spacer_map.fold_row_to_display_row(rows.start)
                ..self.spacer_map.fold_row_to_display_row(rows.end - 1) + 1,
        )
    }

    /// The fold rows of a buffer line: [start, end)
    /// Returns None if the buffer line is completely hidden
    fn fold_row_range(&self, line: usize) -> Option<Range<usize>> {
        // Buffer line → Wrap row range
        let wrap_row_range = self.wrap_map.buffer_line_to_wrap_row_range(line);

        // Find first and last visible display rows in this range
        let mut first_display_row = None;
        let mut last_display_row = None;

        for wrap_row in wrap_row_range {
            if let Some(display_row) = self.fold_map.wrap_row_to_display_row(wrap_row) {
                if first_display_row.is_none() {
                    first_display_row = Some(display_row);
                }
                last_display_row = Some(display_row);
            }
        }

        if let (Some(start), Some(end)) = (first_display_row, last_display_row) {
            Some(start..end + 1)
        } else {
            None // Completely folded
        }
    }

    /// Check if a buffer line is completely hidden
    #[inline]
    pub fn is_buffer_line_hidden(&self, line: usize) -> bool {
        self.buffer_line_to_display_row_range(line).is_none()
    }

    /// First display row of a buffer line. If the line is fully folded, returns the
    /// nearest visible display row.
    pub fn buffer_line_to_display_row(&self, line: usize) -> usize {
        match self.buffer_line_to_display_row_range(line) {
            Some(range) => range.start,
            None => {
                let wrap_row = self.wrap_map.buffer_line_to_first_wrap_row(line);
                self.nearest_visible_display_row(wrap_row)
            }
        }
    }

    // ==================== Spacers ====================

    /// Replace the spacers. They are placed again whenever the rows move.
    pub(crate) fn set_spacers(&mut self, spacers: Vec<Spacer>) {
        if spacers == self.spacers {
            return;
        }
        self.spacers = spacers;
        self.spacers_version = self.spacers_version.wrapping_add(1);
        self.spacers_projected = None;
        self.project_spacers();
    }

    /// How many spacer rows there are, beside lines not hidden by a fold.
    #[inline]
    pub(crate) fn spacer_row_count(&self) -> usize {
        self.spacer_map.row_count()
    }

    /// Whether a display row is a spacer row: a blank row that holds no text,
    /// standing above or below a line. See [`Spacer`].
    pub fn is_spacer_row(&self, display_row: usize) -> bool {
        matches!(
            self.spacer_map.display_row(display_row),
            DisplayRow::Spacer(_)
        )
    }

    /// How many spacer rows stand above and below a buffer line.
    pub(crate) fn spacer_rows_beside(&self, line: usize) -> (usize, usize) {
        self.spacer_map.rows_beside(line)
    }

    /// The spacer rows that have a background, beside the buffer lines within
    /// `lines`: the display rows they take, and the color.
    pub(crate) fn spacer_backgrounds(
        &self,
        lines: Range<usize>,
    ) -> impl Iterator<Item = (Range<usize>, gpui::Hsla)> + '_ {
        self.spacer_map
            .runs_beside(lines)
            .filter_map(|(start, run)| Some((start..start + run.rows, run.background?)))
    }

    /// The display row of text nearest a display row: the row itself when it
    /// holds text, else the first one past its spacers in the direction of
    /// `forward`, or the other way when there is none that way.
    pub(crate) fn text_display_row(&self, display_row: usize, forward: bool) -> usize {
        let row = display_row.min(self.display_row_count().saturating_sub(1));
        let DisplayRow::Spacer(index) = self.spacer_map.display_row(row) else {
            return row;
        };
        let before = self.spacer_map.run(index).before;
        let fold_row = if forward && before < self.fold_map.display_row_count() {
            before
        } else {
            before.saturating_sub(1)
        };
        self.spacer_map.fold_row_to_display_row(fold_row)
    }

    /// The fold row a display row stands for: its own, or for a spacer row the
    /// nearest row of the line it stands beside.
    fn text_fold_row(&self, display_row: usize) -> usize {
        match self.spacer_map.display_row(display_row) {
            DisplayRow::Text(fold_row) => fold_row,
            DisplayRow::Spacer(index) => {
                let run = self.spacer_map.run(index);
                if run.above {
                    run.before
                } else {
                    run.before.saturating_sub(1)
                }
            }
        }
    }

    /// Place the spacers again if the rows moved since they were last placed.
    fn project_spacers(&mut self) {
        if self.spacers.is_empty() && self.spacer_map.row_count() == 0 {
            return;
        }
        let version = self.rows_version();
        if self.spacers_projected == Some(version) {
            return;
        }
        self.spacers_projected = Some(version);
        let line_count = self.buffer_line_count();
        let mut map = std::mem::take(&mut self.spacer_map);
        map.rebuild(&self.spacers, |line| {
            (line < line_count)
                .then(|| self.fold_row_range(line))
                .flatten()
        });
        self.spacer_map = map;
    }

    /// Set fold candidates (from tree-sitter/LSP)
    pub fn set_fold_candidates(&mut self, candidates: Vec<FoldRange>) {
        self.fold_map.set_candidates(candidates);
        self.rebuild_fold_projection();
    }

    /// Set a fold at the given start_line (must be in candidates)
    pub fn set_folded(&mut self, start_line: usize, folded: bool) {
        self.fold_map.set_folded(start_line, folded);
        self.rebuild_fold_projection();
    }

    /// Toggle fold at the given start_line
    pub fn toggle_fold(&mut self, start_line: usize) {
        self.fold_map.toggle_fold(start_line);
        self.rebuild_fold_projection();
    }

    /// Check if a line is currently folded
    #[inline]
    pub fn is_folded_at(&self, start_line: usize) -> bool {
        self.fold_map.is_folded_at(start_line)
    }

    /// Check if a line is a fold candidate
    #[inline]
    pub fn is_fold_candidate(&self, start_line: usize) -> bool {
        self.fold_map.is_fold_candidate(start_line)
    }

    /// Get all currently folded ranges
    #[inline]
    pub fn folded_ranges(&self) -> &[FoldRange] {
        self.fold_map.folded_ranges()
    }

    /// Clear all folds
    pub fn clear_folds(&mut self) {
        self.fold_map.clear_folds();
        self.rebuild_fold_projection();
    }

    // ==================== Text and Layout Updates ====================

    /// Adjust folds and candidates for a text edit before updating the wrap map.
    ///
    /// Must be called with the OLD text (before replacement) and the edit range/new_text
    /// so we can compute which old lines were affected.
    pub fn adjust_folds_for_edit(&mut self, old_text: &Rope, range: &Range<usize>, new_text: &str) {
        if self.fold_map.folded_ranges().is_empty() && self.fold_map.fold_candidates().is_empty() {
            return;
        }

        let edit_start_line = old_text.offset_to_point(range.start).row;
        let edit_end_line = old_text.offset_to_point(range.end.min(old_text.len())).row;

        let old_lines_in_range = edit_end_line.saturating_sub(edit_start_line);
        let new_lines_in_range = new_text.chars().filter(|c| *c == '\n').count();
        let line_delta = new_lines_in_range as isize - old_lines_in_range as isize;

        self.fold_map
            .adjust_folds_for_edit(edit_start_line, edit_end_line, line_delta);
    }

    /// Incrementally update fold candidates after a text edit.
    ///
    /// Extracts new fold candidates only within the edited byte range
    /// and merges them with existing (already adjusted) candidates.
    pub fn update_fold_candidates_for_edit(
        &mut self,
        extract_fold_ranges: impl FnOnce(Range<usize>, &Rope) -> Vec<FoldRange>,
        edit_byte_range: Range<usize>,
        new_text: &Rope,
    ) {
        let new_start_line = new_text.offset_to_point(edit_byte_range.start).row;
        let new_end_line = new_text
            .offset_to_point(edit_byte_range.end.min(new_text.len()))
            .row;

        let new_candidates = extract_fold_ranges(edit_byte_range, new_text);
        self.fold_map
            .merge_candidates_for_edit(new_start_line, new_end_line, new_candidates);
    }

    /// Update text (incremental or full)
    pub fn on_text_changed(
        &mut self,
        changed_text: &Rope,
        range: &Range<usize>,
        new_text: &Rope,
        cx: &mut App,
    ) {
        self.wrap_map
            .on_text_changed(changed_text, range, new_text, cx);
        self.rebuild_fold_projection();
    }

    /// Update layout parameters (wrap width or font)
    pub(crate) fn set_inline_metrics(
        &mut self,
        metrics: std::rc::Rc<[(std::ops::Range<usize>, Pixels)]>,
        cx: &mut App,
    ) {
        self.wrap_map.set_inline_metrics(metrics, cx);
        self.rebuild_fold_projection();
    }

    pub fn on_layout_changed(&mut self, wrap_width: Option<Pixels>, cx: &mut App) {
        self.wrap_map.on_layout_changed(wrap_width, cx);
        self.rebuild_fold_projection();
    }

    /// Set the wrapping indent for continuation lines.
    pub fn set_wrapping_indent(&mut self, wrapping_indent: WrappingIndent, cx: &mut App) {
        self.wrap_map.set_wrapping_indent(wrapping_indent, cx);
        self.rebuild_fold_projection();
    }

    /// Set the number of spaces per tab stop.
    pub(crate) fn set_tab_size(&mut self, tab_size: usize, cx: &mut App) {
        if self.wrap_map.set_tab_size(tab_size, cx) {
            self.rebuild_fold_projection();
        }
    }

    /// Set font parameters
    pub fn set_font(&mut self, font: Font, font_size: Pixels, cx: &mut App) {
        self.wrap_map.set_font(font, font_size, cx);
        self.rebuild_fold_projection();
    }

    /// Ensure text is prepared (initializes wrapper if needed)
    pub fn ensure_text_prepared(&mut self, text: &Rope, cx: &mut App) {
        let did_initialize = self.wrap_map.ensure_text_prepared(text, cx);
        if did_initialize {
            self.rebuild_fold_projection();
        }
    }

    /// Initialize with text
    pub fn set_text(&mut self, text: &Rope, cx: &mut App) {
        self.wrap_map.set_text(text, cx);
        self.rebuild_fold_projection();
    }

    // ==================== Internal Helpers ====================

    /// Rebuild fold projection after wrap_map or fold state changes
    /// Only rebuilds if there are actually folded ranges
    fn rebuild_fold_projection(&mut self) {
        if !self.fold_map.folded_ranges().is_empty() {
            self.fold_map.rebuild(&self.wrap_map);
        } else {
            // No active folds: identity mapping (wrap_row == display_row).
            // Just update cached count so query methods work without Vec allocation.
            self.fold_map
                .mark_dirty_with_wrap_count(self.wrap_map.wrap_row_count());
        }
        self.project_spacers();
    }

    // ==================== Wrap Display Point Operations ====================

    /// Convert byte offset to wrap display point (with soft wrap info).
    #[inline]
    pub(crate) fn offset_to_wrap_display_point(&self, offset: usize) -> WrapDisplayPoint {
        self.wrap_map.wrapper().offset_to_display_point(offset)
    }

    /// Like [`Self::offset_to_wrap_display_point`], but honours the caret's line-end affinity so
    /// an offset on a soft wrap boundary resolves to the row the caret is drawn on.
    #[inline]
    pub(crate) fn offset_to_wrap_display_point_with_affinity(
        &self,
        offset: usize,
        line_end_affinity: bool,
    ) -> WrapDisplayPoint {
        self.wrap_map
            .wrapper()
            .offset_to_display_point_with_affinity(offset, line_end_affinity)
    }

    /// Resolve the caret's visual row through both wrapping and folded regions.
    pub(crate) fn offset_to_display_row(&self, offset: usize, line_end_affinity: bool) -> usize {
        let point = self.offset_to_wrap_display_point_with_affinity(offset, line_end_affinity);
        self.wrap_row_to_display_row(point.row)
            .unwrap_or_else(|| self.nearest_visible_display_row(point.row))
    }

    /// Convert wrap display point to byte offset.
    #[inline]
    pub(crate) fn wrap_display_point_to_offset(&self, point: WrapDisplayPoint) -> usize {
        self.wrap_map.wrapper().display_point_to_offset(point)
    }

    /// Convert wrap display point to TreeSitterPoint (buffer line/col).
    #[inline]
    pub(crate) fn wrap_display_point_to_point(&self, point: WrapDisplayPoint) -> TreeSitterPoint {
        self.wrap_map.wrapper().display_point_to_point(point)
    }

    /// Convert a wrap row to a display row (skipping folded rows).
    /// Returns None if the wrap row is folded.
    #[inline]
    pub fn wrap_row_to_display_row(&self, wrap_row: usize) -> Option<usize> {
        self.fold_map
            .wrap_row_to_display_row(wrap_row)
            .map(|fold_row| self.spacer_map.fold_row_to_display_row(fold_row))
    }

    /// Find the nearest visible display row for a given wrap row.
    #[inline]
    pub fn nearest_visible_display_row(&self, wrap_row: usize) -> usize {
        self.spacer_map
            .fold_row_to_display_row(self.fold_map.nearest_visible_display_row(wrap_row))
    }

    /// Convert a display row to a wrap row.
    /// Returns None for a spacer row.
    #[inline]
    pub fn display_row_to_wrap_row(&self, display_row: usize) -> Option<usize> {
        match self.spacer_map.display_row(display_row) {
            DisplayRow::Text(fold_row) => self.fold_map.display_row_to_wrap_row(fold_row),
            DisplayRow::Spacer(_) => None,
        }
    }

    pub(crate) fn display_row_column_to_offset(&self, display_row: usize, column: usize) -> usize {
        let wrap_row = self
            .fold_map
            .display_row_to_wrap_row(self.text_fold_row(display_row))
            .unwrap_or(0);
        self.wrap_map
            .wrapper()
            .display_point_to_offset(WrapDisplayPoint::new(wrap_row, 0, column))
    }

    /// Get the longest row index (by byte length).
    #[inline]
    pub(crate) fn longest_row(&self) -> usize {
        self.wrap_map.wrapper().longest_row()
    }

    // ==================== Access Methods ====================

    /// Get the line item by buffer row index.
    #[inline]
    pub(crate) fn line(&self, row: usize) -> Option<&LineItem> {
        self.wrap_map.line(row)
    }

    /// Get the rope text
    #[inline]
    pub fn text(&self) -> &Rope {
        self.wrap_map.text()
    }

    /// Calculate how many wrap rows of a buffer line are visible (not folded)
    #[inline]
    pub fn visible_wrap_row_count_for_buffer_line(&self, line: usize) -> usize {
        self.wrap_map
            .visible_wrap_row_count_for_line(line, &self.fold_map)
    }

    /// Get the wrap row count (before folding)
    #[inline]
    pub fn wrap_row_count(&self) -> usize {
        self.wrap_map.wrap_row_count()
    }

    /// Get the buffer line count (logical lines)
    #[inline]
    pub fn buffer_line_count(&self) -> usize {
        self.wrap_map.buffer_line_count()
    }
}
