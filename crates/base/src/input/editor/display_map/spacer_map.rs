/// SpacerMap: Spacer projection layer (Fold rows → Display rows).
///
/// This module adds blank rows above or below buffer lines, the inverse of
/// what [`FoldMap`](super::fold_map::FoldMap) does:
/// - Placing each spacer between the fold rows of its line and its neighbor
/// - Maintaining bidirectional mapping: fold row ↔ display row
/// - Telling a spacer row from a row that holds text
use std::ops::Range;

use gpui::Hsla;

use crate::input::Spacer;

/// Blank rows standing together between two fold rows.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SpacerRows {
    /// The fold row the rows stand above, or the fold row count when they
    /// stand below the last one.
    pub(super) before: usize,
    /// The buffer line the rows belong to.
    pub(super) line: usize,
    /// Whether the rows stand above `line` rather than below it.
    pub(super) above: bool,
    pub(super) rows: usize,
    pub(super) background: Option<Hsla>,
    /// How many spacer rows the runs before this one hold.
    rows_before: usize,
}

impl SpacerRows {
    /// The first display row of the run.
    fn display_start(&self) -> usize {
        self.before + self.rows_before
    }

    /// The display row after the run.
    fn display_end(&self) -> usize {
        self.display_start() + self.rows
    }
}

/// What a display row shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DisplayRow {
    /// A row of text: this fold row.
    Text(usize),
    /// A row of the spacer run at this index.
    Spacer(usize),
}

/// SpacerMap projects fold rows to display rows by inserting spacer rows.
#[derive(Default)]
pub(super) struct SpacerMap {
    /// The spacer runs, in display order: sorted by `before`, and at one place
    /// the rows below a line before the rows above the next one.
    runs: Vec<SpacerRows>,
    /// How many rows the runs hold in total.
    total: usize,
}

impl SpacerMap {
    /// Place `spacers` between fold rows; `fold_rows` gives the fold rows of a
    /// buffer line, or `None` for a line that is hidden or does not exist.
    pub(super) fn rebuild(
        &mut self,
        spacers: &[Spacer],
        fold_rows: impl Fn(usize) -> Option<Range<usize>>,
    ) {
        let mut runs: Vec<SpacerRows> = spacers
            .iter()
            .filter(|spacer| spacer.rows() > 0)
            .filter_map(|spacer| {
                let rows = fold_rows(spacer.line())?;
                Some(SpacerRows {
                    before: if spacer.is_above() {
                        rows.start
                    } else {
                        rows.end
                    },
                    line: spacer.line(),
                    above: spacer.is_above(),
                    rows: spacer.rows(),
                    background: spacer.background(),
                    rows_before: 0,
                })
            })
            .collect();
        runs.sort_by_key(|run| (run.before, run.above));

        let mut total = 0;
        for run in &mut runs {
            run.rows_before = total;
            total += run.rows;
        }
        self.runs = runs;
        self.total = total;
    }

    /// How many rows the spacers hold in total.
    pub(super) fn row_count(&self) -> usize {
        self.total
    }

    /// How many spacer rows the runs before `index` hold.
    fn rows_before(&self, index: usize) -> usize {
        self.runs
            .get(index)
            .map_or(self.total, |run| run.rows_before)
    }

    /// The display row of a fold row: pushed down by every spacer row above it.
    pub(super) fn fold_row_to_display_row(&self, fold_row: usize) -> usize {
        let index = self.runs.partition_point(|run| run.before <= fold_row);
        fold_row + self.rows_before(index)
    }

    /// What a display row shows: a fold row or a spacer row.
    pub(super) fn display_row(&self, display_row: usize) -> DisplayRow {
        let index = self
            .runs
            .partition_point(|run| run.display_end() <= display_row);
        match self.runs.get(index) {
            Some(run) if run.display_start() <= display_row => DisplayRow::Spacer(index),
            _ => DisplayRow::Text(display_row - self.rows_before(index)),
        }
    }

    /// The spacer run at `index`.
    pub(super) fn run(&self, index: usize) -> &SpacerRows {
        &self.runs[index]
    }

    /// The spacer runs beside the buffer lines within `lines`, in display order,
    /// each with the display row it starts at.
    pub(super) fn runs_beside(
        &self,
        lines: Range<usize>,
    ) -> impl Iterator<Item = (usize, &SpacerRows)> + '_ {
        let start = self.runs.partition_point(|run| run.line < lines.start);
        self.runs[start..]
            .iter()
            .take_while(move |run| run.line < lines.end)
            .map(|run| (run.display_start(), run))
    }

    /// How many spacer rows stand above and below a buffer line.
    pub(super) fn rows_beside(&self, line: usize) -> (usize, usize) {
        self.runs_beside(line..line + 1)
            .fold((0, 0), |(above, below), (_, run)| {
                if run.above {
                    (above + run.rows, below)
                } else {
                    (above, below + run.rows)
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One entry per display row: `Some(fold row)` for text, `None` for a
    /// spacer row — the projection built row by row, the slow obvious way.
    fn dense_projection(fold_row_count: usize, spacers: &[(usize, usize)]) -> Vec<Option<usize>> {
        let mut rows = Vec::new();
        for fold_row in 0..=fold_row_count {
            for &(before, count) in spacers {
                if before == fold_row {
                    rows.extend(std::iter::repeat_n(None, count));
                }
            }
            if fold_row < fold_row_count {
                rows.push(Some(fold_row));
            }
        }
        rows
    }

    /// Lines of one, three, one, two and one rows; line 3 hidden by a fold.
    fn lines() -> Vec<Option<Range<usize>>> {
        vec![
            Some(0..1),
            Some(1..4),
            Some(4..5),
            None,
            Some(5..7),
            Some(7..8),
        ]
    }

    fn map(spacers: &[Spacer]) -> SpacerMap {
        let lines = lines();
        let mut map = SpacerMap::default();
        map.rebuild(spacers, |line| lines.get(line).cloned().flatten());
        map
    }

    #[test]
    fn spacer_runs_match_the_dense_projection() {
        let spacers = [
            Spacer::above(0, 2),
            Spacer::below(1, 1),
            Spacer::above(2, 3),
            Spacer::below(3, 4),
            Spacer::below(5, 2),
            Spacer::below(6, 9),
            Spacer::above(4, 0),
        ];
        let map = map(&spacers);
        // Line 3 is hidden and line 6 does not exist; a spacer of no rows is none.
        let dense = dense_projection(8, &[(0, 2), (4, 1), (4, 3), (8, 2)]);
        assert_eq!(map.row_count(), 8);

        for (display_row, expected) in dense.iter().enumerate() {
            match (map.display_row(display_row), expected) {
                (DisplayRow::Text(fold_row), Some(expected)) => {
                    assert_eq!(fold_row, *expected, "display row {display_row}");
                    assert_eq!(map.fold_row_to_display_row(fold_row), display_row);
                }
                (DisplayRow::Spacer(_), None) => {}
                (shown, expected) => {
                    panic!("display row {display_row}: {shown:?}, expected {expected:?}")
                }
            }
        }
        assert_eq!(
            map.display_row(dense.len()),
            DisplayRow::Text(8),
            "past the end is past the last fold row"
        );
    }

    #[test]
    fn the_rows_below_a_line_come_before_the_rows_above_the_next() {
        let map = map(&[Spacer::above(2, 3), Spacer::below(1, 1)]);
        let runs: Vec<_> = map
            .runs_beside(0..6)
            .map(|(start, run)| (start, run.line))
            .collect();
        assert_eq!(runs, vec![(4, 1), (5, 2)]);
        assert_eq!(map.rows_beside(1), (0, 1));
        assert_eq!(map.rows_beside(2), (3, 0));
        assert_eq!(map.rows_beside(4), (0, 0));
    }

    #[test]
    fn a_line_with_spacers_on_both_sides_counts_both() {
        let background = gpui::red();
        let map = map(&[
            Spacer::below(4, 2).with_background(background),
            Spacer::above(4, 1),
        ]);
        assert_eq!(map.rows_beside(4), (1, 2));
        let runs: Vec<_> = map
            .runs_beside(4..5)
            .map(|(start, run)| (start, run.above, run.background))
            .collect();
        assert_eq!(runs, vec![(5, true, None), (8, false, Some(background))]);
        // The line's two rows stand between its spacers.
        assert_eq!(map.fold_row_to_display_row(5), 6);
        assert_eq!(map.fold_row_to_display_row(6), 7);
        assert_eq!(map.display_row(8), DisplayRow::Spacer(1));
        assert_eq!(map.display_row(10), DisplayRow::Text(7));
    }
}
