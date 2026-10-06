use std::{cell::RefCell, ops::Range, rc::Rc};

use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, ListAlignment, ListOffset, ListState,
    Pixels, ScrollHandle, Subscription, Window, point, px,
};
use gpui_base::{TextSelection, TextSelectionContentKey, TextSelectionEvent, TextSelectionHandle};

use super::{
    DiffDocument, DiffLinePosition, DiffLineRange, DiffMode, DiffSide, selection::SelectionGeometry,
};

#[derive(Clone, Debug)]
pub(crate) enum DisplayRow {
    Code {
        original: Option<usize>,
        modified: Option<usize>,
        changed: bool,
    },
    Fold(Range<usize>),
    Hunk(gpui::SharedString),
}

/// Notifications from the readonly comparison surface.
#[derive(Clone, Debug)]
pub enum DiffEvent {
    /// A gutter or keyboard operation changed the selected source lines.
    SelectionChanged(Option<DiffLineRange>),
}

/// Retained focus, viewport, context expansion and selection for a [`super::Diff`].
///
/// Create once in the owning view. Mutations notify observers; the application
/// owns the source revisions and chooses when to install a new document.
pub struct DiffState {
    pub(crate) document: DiffDocument,
    pub(crate) mode: DiffMode,
    context_lines: Option<usize>,
    expanded: Vec<Range<usize>>,
    original_rows: Vec<usize>,
    modified_rows: Vec<usize>,
    pub(crate) rows: Rc<Vec<DisplayRow>>,
    change_rows: Vec<usize>,
    pub(crate) list: ListState,
    pub(crate) horizontal_scroll: ScrollHandle,
    pub(crate) focus: FocusHandle,
    pub(crate) selection: TextSelectionHandle,
    pub(crate) geometry: Rc<RefCell<SelectionGeometry>>,
    pub(crate) selected_lines: Option<DiffLineRange>,
    selection_anchor: Option<DiffLinePosition>,
    selection_cursor: Option<DiffLinePosition>,
    pub(crate) viewport_width: Pixels,
    pub(crate) viewport_height: Pixels,
    pub(crate) measurement: Option<(Pixels, Pixels, gpui::SharedString, DiffMode, u64, bool)>,
    pub(crate) width_measurement: Option<(Pixels, Pixels, gpui::SharedString, DiffMode, bool)>,
    pub(crate) measured_width: Pixels,
    _selection_subscription: Subscription,
}

impl EventEmitter<DiffEvent> for DiffState {}
impl Focusable for DiffState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl DiffState {
    /// Creates a unified viewer with three unchanged lines around each change.
    pub fn new(document: DiffDocument, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle().tab_stop(true);
        let selection = TextSelectionHandle::new("", cx);
        let geometry = Rc::new(RefCell::new(SelectionGeometry::default()));
        let weak = cx.entity().downgrade();
        selection.resolve_content_key_with(
            {
                let weak = weak.clone();
                let geometry = geometry.clone();
                move |point, cx| {
                    let state = weak.upgrade()?;
                    let snapshot = state.read(cx).selection.snapshot(cx);
                    let entity = Some(state.read(cx).selection.entity_id());
                    let snapshot =
                        snapshot.filter(|snapshot| snapshot.anchor().entity_id() == entity);
                    let side = snapshot
                        .and_then(|snapshot| snapshot.anchor().content_key())
                        .map(|key| decode_key(key.value()).0);
                    geometry
                        .borrow()
                        .key_at(point, side)
                        .map(TextSelectionContentKey::new)
                        .or_else(|| {
                            snapshot
                                .filter(|snapshot| snapshot.cursor().entity_id() == entity)
                                .and_then(|snapshot| snapshot.cursor().content_key())
                        })
                }
            },
            cx,
        );
        selection.copy_with(
            {
                let weak = weak.clone();
                move |cx| {
                    weak.upgrade()
                        .map_or_else(String::new, |state| state.read(cx).selected_text(cx))
                }
            },
            cx,
        );
        selection.focus_with(
            {
                let focus = focus.clone();
                move |window, cx| window.focus(&focus, cx)
            },
            cx,
        );
        let _selection_subscription = selection.subscribe(
            move |event, cx| {
                let _ = weak.update(cx, |state, cx| {
                    if !state.selection.has_local_selection(cx) {
                        match event {
                            TextSelectionEvent::SelectionChanged(Some(_)) => {
                                let changed = state.selected_lines.take().is_some();
                                // A new text gesture ends the previous gutter anchor.
                                state.selection_anchor = None;
                                state.selection_cursor = None;
                                if changed {
                                    cx.emit(DiffEvent::SelectionChanged(None));
                                }
                            }
                            TextSelectionEvent::Cleared => {
                                // Mouse-down cleanup precedes gutter activation. Retain
                                // its anchor for Shift-click, without an intermediate event.
                                state.selected_lines = None;
                            }
                            _ => {}
                        }
                    }
                    cx.notify();
                });
            },
            cx,
        );
        let rows = Rc::new(project_rows(&document, DiffMode::Unified, Some(3), &[]));
        let (original_rows, modified_rows) = source_rows(&document, &rows);
        Self {
            document,
            mode: DiffMode::Unified,
            context_lines: Some(3),
            expanded: Vec::new(),
            original_rows,
            modified_rows,
            change_rows: changed_groups(&rows),
            list: ListState::new(rows.len(), ListAlignment::Top, px(0.)),
            rows,
            horizontal_scroll: ScrollHandle::new(),
            focus,
            selection,
            geometry,
            selected_lines: None,
            selection_anchor: None,
            selection_cursor: None,
            viewport_width: px(0.),
            viewport_height: px(0.),
            measurement: None,
            measured_width: px(0.),
            width_measurement: None,
            _selection_subscription,
        }
    }

    pub fn document(&self) -> &DiffDocument {
        &self.document
    }
    pub fn mode(&self) -> DiffMode {
        self.mode
    }
    pub fn context_lines(&self) -> Option<usize> {
        self.context_lines
    }
    pub fn selected_lines(&self) -> Option<DiffLineRange> {
        self.selected_lines
    }

    /// Replaces source versions and resets expansion, selection and viewport.
    pub fn set_document(
        &mut self,
        document: DiffDocument,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        TextSelection::clear(window, cx);
        self.document = document;
        self.width_measurement = None;
        self.expanded.clear();
        self.selection.set_local_selection(false, cx);
        self.selection.set_fallback_copy_text("", cx);
        self.selected_lines = None;
        self.selection_anchor = None;
        self.selection_cursor = None;
        self.geometry.borrow_mut().clear();
        self.rebuild();
        self.list.scroll_to(ListOffset::default());
        self.horizontal_scroll.set_offset(point(px(0.), px(0.)));
        cx.notify();
    }

    /// Changes layout while preserving source-line selection and a nearby source row.
    pub fn set_mode(&mut self, mode: DiffMode, cx: &mut Context<Self>) {
        if self.mode == mode {
            return;
        }
        let anchor = self.row_position(self.list.logical_scroll_top().item_ix);
        self.mode = mode;
        self.rebuild();
        if let Some(anchor) = anchor {
            self.reveal_line(anchor);
        }
        self.horizontal_scroll.set_offset(point(px(0.), px(0.)));
        cx.notify();
    }

    /// Sets surrounding context. `None` shows every supplied patch line.
    pub fn set_context_lines(&mut self, lines: Option<usize>, cx: &mut Context<Self>) {
        if self.context_lines == lines {
            return;
        }
        let anchor = self.row_position(self.list.logical_scroll_top().item_ix);
        self.context_lines = lines;
        self.expanded.clear();
        self.rebuild();
        if let Some(anchor) = anchor {
            self.reveal_line(anchor);
        }
        cx.notify();
    }

    /// Reveals all unchanged source while preserving the configured context count.
    pub fn expand_all(&mut self, cx: &mut Context<Self>) {
        let anchor = self.row_position(self.list.logical_scroll_top().item_ix);
        self.expanded = vec![0..self.document.0.pairs.len()];
        self.rebuild();
        if let Some(anchor) = anchor {
            self.reveal_line(anchor);
        }
        cx.notify();
    }

    /// Restores the configured context disclosures.
    pub fn collapse_all(&mut self, cx: &mut Context<Self>) {
        let anchor = self.row_position(self.list.logical_scroll_top().item_ix);
        self.expanded.clear();
        self.rebuild();
        if let Some(anchor) = anchor {
            self.reveal_line(anchor);
        }
        cx.notify();
    }

    /// Synchronizes source-line selection without emitting a user event.
    /// Clips to supplied patch lines; a range containing none clears selection.
    pub fn set_selected_lines(&mut self, range: Option<DiffLineRange>, cx: &mut Context<Self>) {
        let range = range.and_then(|range| {
            let lines = self.document.lines(range.side());
            let start = lines.partition_point(|line| line.line_number < range.start());
            let end = lines.partition_point(|line| line.line_number <= range.end());
            (start < end).then(|| {
                DiffLineRange::new(
                    range.side(),
                    lines[start].line_number,
                    lines[end - 1].line_number,
                )
            })
        });
        if self.selected_lines == range {
            return;
        }
        self.selected_lines = range;
        self.selection_anchor =
            range.map(|range| DiffLinePosition::new(range.side(), range.start()));
        self.selection_cursor = range.map(|range| DiffLinePosition::new(range.side(), range.end()));
        self.selection.set_local_selection(range.is_some(), cx);
        cx.notify();
    }

    /// Returns selected source without gutters, diff markers or annotation content.
    pub fn selected_text(&self, cx: &App) -> String {
        if self.selection.has_local_selection(cx)
            && let Some(range) = self.selected_lines
        {
            return self.document.text_for_range(range);
        }
        let Some(snapshot) = self.selection.snapshot(cx) else {
            return String::new();
        };
        let entity_id = Some(self.selection.entity_id());
        if snapshot.anchor().entity_id() != entity_id || snapshot.cursor().entity_id() != entity_id
        {
            return String::new();
        }
        let Some(anchor) = snapshot.anchor().content_key() else {
            return String::new();
        };
        let Some(cursor) = snapshot.cursor().content_key() else {
            return String::new();
        };
        let (side, anchor) = decode_key(anchor.value());
        let (cursor_side, cursor) = decode_key(cursor.value());
        if side != cursor_side {
            return String::new();
        }
        let source = self.document.source(side);
        source
            .get(anchor.min(cursor)..anchor.max(cursor))
            .unwrap_or("")
            .to_owned()
    }

    /// Reveals and scrolls to a source line, expanding its hidden context if needed.
    pub fn scroll_to_line(&mut self, position: DiffLinePosition, cx: &mut Context<Self>) {
        if self.document.line_index(position).is_none() {
            return;
        }
        if self.expand_line(position, cx).is_some() {
            self.reveal_line(position);
            cx.notify();
        }
    }

    fn expand_line(&mut self, position: DiffLinePosition, cx: &mut Context<Self>) -> Option<usize> {
        let row = self.source_row(position)?;
        if let Some(DisplayRow::Fold(range)) = self.rows.get(row) {
            let range = range.clone();
            self.expand(range, cx);
        }
        self.source_row(position)
    }

    /// Moves to the next changed group after the current viewport.
    pub fn next_change(&mut self, cx: &mut Context<Self>) {
        self.move_change(true, cx);
    }
    /// Moves to the previous changed group before the current viewport.
    pub fn previous_change(&mut self, cx: &mut Context<Self>) {
        self.move_change(false, cx);
    }

    pub(crate) fn expand(&mut self, range: Range<usize>, cx: &mut Context<Self>) {
        let top = self.row_position(self.list.logical_scroll_top().item_ix);
        self.expanded.push(range);
        self.rebuild();
        if let Some(top) = top {
            self.reveal_line(top);
        }
        cx.notify();
    }

    pub(crate) fn click_line(
        &mut self,
        position: DiffLinePosition,
        extend: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.document.line_index(position).is_none() {
            return;
        }
        let anchor = self
            .selection_anchor
            .filter(|a| extend && a.side() == position.side())
            .unwrap_or(position);
        TextSelection::clear(window, cx);
        self.select_user_range(anchor, position, window, cx);
    }

    pub(crate) fn keyboard_select(
        &mut self,
        direction: isize,
        extend: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let side = self
            .selected_lines
            .map(|range| range.side())
            .unwrap_or_else(|| {
                if self.document.line_count(DiffSide::Modified) > 0 {
                    DiffSide::Modified
                } else {
                    DiffSide::Original
                }
            });
        let count = self.document.line_count(side);
        if count == 0 {
            return;
        }
        let ix = self
            .selection_cursor
            .filter(|position| position.side() == side)
            .and_then(|position| self.document.line_index(position))
            .map(|ix| ix.saturating_add_signed(direction).min(count - 1))
            .unwrap_or(0);
        let position = self.document.position(side, ix);
        self.click_line(position, extend, window, cx);
        if let Some(row) = self.expand_line(position, cx) {
            self.list.scroll_to_reveal_item(row);
        }
    }

    pub(crate) fn select_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let side = self
            .selected_lines
            .map(|range| range.side())
            .or_else(|| {
                self.selection.snapshot(cx).and_then(|snapshot| {
                    (snapshot.cursor().entity_id() == Some(self.selection.entity_id()))
                        .then(|| {
                            snapshot
                                .cursor()
                                .content_key()
                                .map(|key| decode_key(key.value()).0)
                        })
                        .flatten()
                })
            })
            .unwrap_or_else(|| {
                if self.document.line_count(DiffSide::Modified) > 0 {
                    DiffSide::Modified
                } else {
                    DiffSide::Original
                }
            });
        let count = self.document.line_count(side);
        if count == 0 {
            return;
        }
        TextSelection::clear(window, cx);
        self.select_user_range(
            self.document.position(side, 0),
            self.document.position(side, count - 1),
            window,
            cx,
        );
    }

    fn select_user_range(
        &mut self,
        anchor: DiffLinePosition,
        cursor: DiffLinePosition,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = Some(DiffLineRange::new(
            cursor.side(),
            anchor.line(),
            cursor.line(),
        ));
        let changed = self.selected_lines != range;
        self.selected_lines = range;
        self.selection_anchor = Some(anchor);
        self.selection_cursor = Some(cursor);
        self.selection.set_local_selection(true, cx);
        window.focus(&self.focus, cx);
        if changed {
            cx.emit(DiffEvent::SelectionChanged(range));
        }
        cx.notify();
    }

    fn move_change(&mut self, forward: bool, cx: &mut Context<Self>) {
        let top = self.list.logical_scroll_top().item_ix;
        let groups = &self.change_rows;
        let target = if forward {
            groups
                .get(groups.partition_point(|ix| *ix <= top))
                .copied()
                .or(groups.first().copied())
        } else {
            groups
                .partition_point(|ix| *ix < top)
                .checked_sub(1)
                .and_then(|ix| groups.get(ix))
                .copied()
                .or(groups.last().copied())
        };
        if let Some(ix) = target {
            self.list.scroll_to(ListOffset {
                item_ix: ix,
                offset_in_item: px(0.),
            });
            cx.notify();
        }
    }

    fn row_position(&self, ix: usize) -> Option<DiffLinePosition> {
        match self.rows.get(ix)? {
            DisplayRow::Code {
                original, modified, ..
            } => modified
                .map(|ix| self.document.position(DiffSide::Modified, ix))
                .or_else(|| original.map(|ix| self.document.position(DiffSide::Original, ix))),
            DisplayRow::Hunk(_) => self.row_position(ix + 1),
            DisplayRow::Fold(range) => self.document.0.pairs.get(range.start).and_then(|pair| {
                pair.modified
                    .map(|ix| self.document.position(DiffSide::Modified, ix))
                    .or_else(|| {
                        pair.original
                            .map(|ix| self.document.position(DiffSide::Original, ix))
                    })
            }),
        }
    }
    fn source_row(&self, position: DiffLinePosition) -> Option<usize> {
        let rows = match position.side() {
            DiffSide::Original => &self.original_rows,
            DiffSide::Modified => &self.modified_rows,
        };
        rows.get(self.document.line_index(position)?).copied()
    }

    fn reveal_line(&self, position: DiffLinePosition) {
        if let Some(item_ix) = self.source_row(position) {
            self.list.scroll_to(ListOffset {
                item_ix,
                offset_in_item: px(0.),
            });
        }
    }
    fn rebuild(&mut self) {
        self.rows = Rc::new(project_rows(
            &self.document,
            self.mode,
            self.context_lines,
            &self.expanded,
        ));
        (self.original_rows, self.modified_rows) = source_rows(&self.document, &self.rows);
        self.change_rows = changed_groups(&self.rows);
        self.list.reset(self.rows.len());
        self.measurement = None;
    }
}

/// Every source line maps to its code row or the disclosure hiding it.
/// Disjoint folds visit each pair once, so building both maps is linear.
fn source_rows(document: &DiffDocument, rows: &[DisplayRow]) -> (Vec<usize>, Vec<usize>) {
    let mut original_rows = vec![0; document.line_count(DiffSide::Original)];
    let mut modified_rows = vec![0; document.line_count(DiffSide::Modified)];
    for (row_ix, row) in rows.iter().enumerate() {
        match row {
            DisplayRow::Code {
                original, modified, ..
            } => {
                if let Some(line) = original {
                    original_rows[*line] = row_ix;
                }
                if let Some(line) = modified {
                    modified_rows[*line] = row_ix;
                }
            }
            DisplayRow::Hunk(_) => {}
            DisplayRow::Fold(range) => {
                for pair in &document.0.pairs[range.clone()] {
                    if let Some(line) = pair.original {
                        original_rows[line] = row_ix;
                    }
                    if let Some(line) = pair.modified {
                        modified_rows[line] = row_ix;
                    }
                }
            }
        }
    }
    (original_rows, modified_rows)
}

fn changed_groups(rows: &[DisplayRow]) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .filter_map(|(ix, row)| {
            (matches!(row, DisplayRow::Code { changed: true, .. })
                && (ix == 0 || !matches!(rows[ix - 1], DisplayRow::Code { changed: true, .. })))
            .then_some(ix)
        })
        .collect()
}

pub(crate) fn encode_key(side: DiffSide, offset: usize) -> u64 {
    (offset as u64)
        | if side == DiffSide::Modified {
            1 << 63
        } else {
            0
        }
}
pub(crate) fn decode_key(key: u64) -> (DiffSide, usize) {
    (
        if key >> 63 == 0 {
            DiffSide::Original
        } else {
            DiffSide::Modified
        },
        (key & !(1 << 63)) as usize,
    )
}

fn project_rows(
    document: &DiffDocument,
    mode: DiffMode,
    context: Option<usize>,
    expanded: &[Range<usize>],
) -> Vec<DisplayRow> {
    let pairs = &document.0.pairs;
    let hunk_boundaries = &document.0.hunk_boundaries;
    let mut hunk_end = vec![pairs.len(); pairs.len()];
    let mut hunk_start = vec![0; pairs.len()];
    for (ix, (start, _)) in hunk_boundaries.iter().enumerate() {
        let end = hunk_boundaries
            .get(ix + 1)
            .map_or(pairs.len(), |(start, _)| *start);
        hunk_start[*start..end].fill(*start);
        hunk_end[*start..end].fill(end);
    }
    // Interval boundaries avoid repeatedly filling overlapping context windows.
    // Projection remains linear in source lines plus the number of disclosures.
    let mut boundaries = vec![0isize; pairs.len() + 1];
    if let Some(context) = context {
        let mut ix = 0;
        while ix < pairs.len() {
            if !pairs[ix].changed {
                ix += 1;
                continue;
            }
            let start = ix;
            while ix < pairs.len() && ix < hunk_end[start] && pairs[ix].changed {
                ix += 1;
            }
            boundaries[start.saturating_sub(context).max(hunk_start[start])] += 1;
            boundaries[ix.saturating_add(context).min(hunk_end[start])] -= 1;
        }
    } else {
        boundaries[0] += 1;
        boundaries[pairs.len()] -= 1;
    }
    for range in expanded {
        let start = range.start.min(pairs.len());
        let end = range.end.min(pairs.len());
        if start < end {
            boundaries[start] += 1;
            boundaries[end] -= 1;
        }
    }
    let mut active = 0;
    let visible = boundaries[..pairs.len()]
        .iter()
        .map(|boundary| {
            active += boundary;
            active > 0
        })
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    let mut hunk_ix = 0;
    let mut ix = 0;
    while ix < pairs.len() {
        while let Some((start, label)) = hunk_boundaries.get(hunk_ix) {
            if *start != ix {
                break;
            }
            rows.push(DisplayRow::Hunk(label.clone()));
            hunk_ix += 1;
        }
        if !visible[ix] {
            let start = ix;
            while ix < pairs.len() && ix < hunk_end[start] && !visible[ix] {
                ix += 1;
            }
            rows.push(DisplayRow::Fold(start..ix));
        } else if mode == DiffMode::Unified && pairs[ix].changed {
            let start = ix;
            while ix < pairs.len() && ix < hunk_end[start] && pairs[ix].changed {
                ix += 1;
            }
            for pair in &pairs[start..ix] {
                if let Some(original) = pair.original {
                    rows.push(DisplayRow::Code {
                        original: Some(original),
                        modified: None,
                        changed: true,
                    });
                }
            }
            for pair in &pairs[start..ix] {
                if let Some(modified) = pair.modified {
                    rows.push(DisplayRow::Code {
                        original: None,
                        modified: Some(modified),
                        changed: true,
                    });
                }
            }
        } else {
            let pair = pairs[ix];
            rows.push(DisplayRow::Code {
                original: pair.original,
                modified: pair.modified,
                changed: pair.changed,
            });
            ix += 1;
        }
    }
    rows
}
