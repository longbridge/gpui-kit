use std::{cell::RefCell, collections::HashSet, ops::Range, rc::Rc};

use gpui::{
    App, AppContext as _, Context, ElementId, EventEmitter, FocusHandle, Focusable, ListAlignment,
    ListOffset, ListState, Pixels, ScrollHandle, SharedString, Size, Subscription, Task, Window,
    WindowId, point, px,
};
use gpui_base::{TextSelection, TextSelectionContentKey, TextSelectionEvent, TextSelectionHandle};

use super::{
    DiffDocument, DiffLineAnnotation, DiffLinePosition, DiffLineRange, DiffMode, DiffSide,
    document::SyntaxHighlighters, selection::SelectionGeometry,
};

/// One virtualized row. Every row belongs to exactly one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DisplayRow {
    /// The file header.
    File(usize),
    /// Binary, metadata-only or empty file summary, for a file without source rows.
    Notice(usize),
    Hunk {
        file: usize,
        hunk: usize,
    },
    Fold {
        file: usize,
        pairs: Range<usize>,
    },
    Code {
        file: usize,
        original: Option<usize>,
        modified: Option<usize>,
        changed: bool,
    },
}

impl DisplayRow {
    pub(crate) fn file(&self) -> usize {
        match self {
            Self::File(file) | Self::Notice(file) => *file,
            Self::Hunk { file, .. } | Self::Fold { file, .. } | Self::Code { file, .. } => *file,
        }
    }
}

/// Notifications from the readonly comparison surface.
#[derive(Clone, Debug)]
pub enum DiffEvent {
    /// A gutter or keyboard operation changed the selected source lines.
    SelectionChanged(Option<DiffLineRange>),
}

/// Inputs that determine row heights and content width.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LayoutKey {
    rem: Pixels,
    font_size: Pixels,
    font_family: SharedString,
    mode: DiffMode,
    line_numbers: bool,
}

impl LayoutKey {
    pub(crate) fn new(
        rem: Pixels,
        font_size: Pixels,
        font_family: SharedString,
        mode: DiffMode,
        line_numbers: bool,
    ) -> Self {
        Self {
            rem,
            font_size,
            font_family,
            mode,
            line_numbers,
        }
    }
}

/// The row showing each source line, or the disclosure hiding it.
#[derive(Default)]
struct FileRows {
    original: Vec<usize>,
    modified: Vec<usize>,
}

/// Retained focus, viewport, context expansion and selection for a
/// [`super::Diff`] showing one or more files of a patch.
///
/// Create once in the owning view. Mutations notify observers; the application
/// owns the patch and chooses when to install new documents. Syntax emphasis is
/// prepared on a background thread and appears once ready.
pub struct DiffState {
    documents: Vec<DiffDocument>,
    mode: DiffMode,
    context_lines: Option<usize>,
    expanded: Vec<Vec<Range<usize>>>,
    file_rows: Vec<FileRows>,
    rows: Rc<Vec<DisplayRow>>,
    change_rows: Vec<usize>,
    list: ListState,
    horizontal_scroll: ScrollHandle,
    focus: FocusHandle,
    selection: TextSelectionHandle,
    geometry: Rc<RefCell<SelectionGeometry>>,
    selected_lines: Option<DiffLineRange>,
    selection_anchor: Option<DiffLinePosition>,
    selection_cursor: Option<DiffLinePosition>,
    viewport: Size<Pixels>,
    layout: Option<LayoutKey>,
    content_width: Pixels,
    row_height: Option<Pixels>,
    annotations: HashSet<(DiffLinePosition, ElementId)>,
    window: Option<WindowId>,
    _syntax: Task<()>,
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
    pub fn new(documents: impl IntoIterator<Item = DiffDocument>, cx: &mut Context<Self>) -> Self {
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
                        .map(|key| decode_key(key.value()).1);
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
        let documents = documents.into_iter().collect::<Vec<_>>();
        let mut state = Self {
            expanded: vec![Vec::new(); documents.len()],
            documents,
            mode: DiffMode::Unified,
            context_lines: Some(3),
            file_rows: Vec::new(),
            rows: Rc::default(),
            change_rows: Vec::new(),
            list: ListState::new(0, ListAlignment::Top, px(0.)),
            horizontal_scroll: ScrollHandle::new(),
            focus,
            selection,
            geometry,
            selected_lines: None,
            selection_anchor: None,
            selection_cursor: None,
            viewport: Size::default(),
            layout: None,
            content_width: px(0.),
            row_height: None,
            annotations: HashSet::new(),
            window: None,
            _syntax: Task::ready(()),
            _selection_subscription,
        };
        state.rebuild(true);
        state.prepare_syntax(cx);
        state
    }

    /// Sets the initial layout. Default is [`DiffMode::Unified`].
    pub fn with_mode(mut self, mode: DiffMode) -> Self {
        self.mode = mode;
        self.rebuild(true);
        self
    }

    /// Sets the initial unchanged context around each change. Default is
    /// `Some(3)`; `None` shows every supplied patch line.
    pub fn with_context_lines(mut self, lines: Option<usize>) -> Self {
        self.context_lines = lines;
        self.rebuild(true);
        self
    }

    /// The files in patch order. Positions and ranges address a file by its
    /// index in this slice.
    pub fn documents(&self) -> &[DiffDocument] {
        &self.documents
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

    pub(crate) fn rows(&self) -> &Rc<Vec<DisplayRow>> {
        &self.rows
    }
    pub(crate) fn list(&self) -> &ListState {
        &self.list
    }
    pub(crate) fn horizontal_scroll(&self) -> &ScrollHandle {
        &self.horizontal_scroll
    }
    pub(crate) fn selection(&self) -> &TextSelectionHandle {
        &self.selection
    }
    pub(crate) fn geometry(&self) -> &Rc<RefCell<SelectionGeometry>> {
        &self.geometry
    }
    /// The body's size as of the last paint.
    pub(crate) fn viewport(&self) -> Size<Pixels> {
        self.viewport
    }
    /// The width rows need to show their longest line.
    pub(crate) fn content_width(&self) -> Pixels {
        self.content_width
    }
    pub(crate) fn set_content_width(&mut self, width: Pixels) {
        self.content_width = width;
    }
    /// Records the painted body size and widest painted row. Returns whether
    /// either changed, so the frame must be laid out again.
    pub(crate) fn record_paint(&mut self, viewport: Size<Pixels>, painted_width: Pixels) -> bool {
        let wider = painted_width > self.content_width;
        if wider {
            self.content_width = painted_width;
        }
        let resized = self.viewport != viewport;
        self.viewport = viewport;
        wider || resized
    }

    /// Replaces the files and resets expansion, selection and viewport.
    pub fn set_documents(
        &mut self,
        documents: impl IntoIterator<Item = DiffDocument>,
        cx: &mut Context<Self>,
    ) {
        if let Some(window) = self.window
            && self.selection_is_local(cx)
        {
            TextSelection::clear_for_window(window, cx);
        }
        self.documents = documents.into_iter().collect();
        self.expanded = vec![Vec::new(); self.documents.len()];
        self.layout = None;
        self.annotations.clear();
        self.selection.set_local_selection(false, cx);
        self.selection.set_fallback_copy_text("", cx);
        self.selected_lines = None;
        self.selection_anchor = None;
        self.selection_cursor = None;
        self.geometry.borrow_mut().clear();
        self.rebuild(true);
        self.horizontal_scroll.set_offset(point(px(0.), px(0.)));
        self.prepare_syntax(cx);
        cx.notify();
    }

    /// Changes layout while preserving source-line selection and a nearby source row.
    pub fn set_mode(&mut self, mode: DiffMode, cx: &mut Context<Self>) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.rebuild_at_anchor();
        self.horizontal_scroll.set_offset(point(px(0.), px(0.)));
        cx.notify();
    }

    /// Sets surrounding context. `None` shows every supplied patch line.
    pub fn set_context_lines(&mut self, lines: Option<usize>, cx: &mut Context<Self>) {
        if self.context_lines == lines {
            return;
        }
        self.context_lines = lines;
        self.expanded.iter_mut().for_each(Vec::clear);
        self.rebuild_at_anchor();
        cx.notify();
    }

    /// Reveals all unchanged source while preserving the configured context count.
    pub fn expand_all(&mut self, cx: &mut Context<Self>) {
        for (expanded, document) in self.expanded.iter_mut().zip(&self.documents) {
            *expanded = vec![0..document.pairs().len()];
        }
        self.rebuild_at_anchor();
        cx.notify();
    }

    /// Restores the configured context disclosures.
    pub fn collapse_all(&mut self, cx: &mut Context<Self>) {
        self.expanded.iter_mut().for_each(Vec::clear);
        self.rebuild_at_anchor();
        cx.notify();
    }

    /// Synchronizes source-line selection without emitting a user event.
    /// Clips to supplied patch lines; a range containing none clears selection.
    pub fn set_selected_lines(&mut self, range: Option<DiffLineRange>, cx: &mut Context<Self>) {
        let range = range.and_then(|range| {
            let lines = self.documents.get(range.file())?.lines(range.side());
            let start = lines.partition_point(|line| line.line_number() < range.start());
            let end = lines.partition_point(|line| line.line_number() <= range.end());
            (start < end).then(|| {
                DiffLineRange::new(
                    range.file(),
                    range.side(),
                    lines[start].line_number(),
                    lines[end - 1].line_number(),
                )
            })
        });
        if self.selected_lines == range {
            return;
        }
        self.selected_lines = range;
        self.selection_anchor =
            range.map(|range| DiffLinePosition::new(range.file(), range.side(), range.start()));
        self.selection_cursor =
            range.map(|range| DiffLinePosition::new(range.file(), range.side(), range.end()));
        self.selection.set_local_selection(range.is_some(), cx);
        cx.notify();
    }

    /// Returns selected source without gutters, diff markers or annotation content.
    pub fn selected_text(&self, cx: &App) -> String {
        if self.selection.has_local_selection(cx)
            && let Some(range) = self.selected_lines
        {
            return self
                .documents
                .get(range.file())
                .map_or_else(String::new, |document| {
                    document.text_for_lines(range.side(), range.start(), range.end())
                });
        }
        let Some((file, side, range)) =
            super::selection::selected_source_range(&self.selection, cx)
        else {
            return String::new();
        };
        self.documents
            .get(file)
            .and_then(|document| document.source(side).get(range))
            .unwrap_or("")
            .to_owned()
    }

    /// Reveals and scrolls to a source line, expanding its hidden context if needed.
    pub fn scroll_to_line(&mut self, position: DiffLinePosition, cx: &mut Context<Self>) {
        if self.line_index(position).is_none() {
            return;
        }
        if self.expand_line(position).is_some() {
            self.reveal_line(position);
            cx.notify();
        }
    }

    /// Moves to the next changed group after the current viewport, across files.
    pub fn next_change(&mut self, cx: &mut Context<Self>) {
        self.move_change(true, cx);
    }
    /// Moves to the previous changed group before the current viewport, across files.
    pub fn previous_change(&mut self, cx: &mut Context<Self>) {
        self.move_change(false, cx);
    }

    pub(crate) fn expand(&mut self, file: usize, pairs: Range<usize>, cx: &mut Context<Self>) {
        if let Some(expanded) = self.expanded.get_mut(file) {
            expanded.push(pairs);
            self.rebuild_at_anchor();
            cx.notify();
        }
    }

    pub(crate) fn click_line(
        &mut self,
        position: DiffLinePosition,
        extend: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.line_index(position).is_none() {
            return;
        }
        let anchor = self
            .selection_anchor
            .filter(|a| extend && a.file() == position.file() && a.side() == position.side())
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
        let Some((file, side)) = self.active_side(cx) else {
            return;
        };
        let document = &self.documents[file];
        let count = document.lines_count(side);
        let ix = self
            .selection_cursor
            .filter(|position| position.file() == file && position.side() == side)
            .and_then(|position| self.line_index(position))
            .map(|ix| ix.saturating_add_signed(direction).min(count - 1))
            .unwrap_or(0);
        let position = self.position(file, side, ix);
        self.click_line(position, extend, window, cx);
        if let Some(row) = self.expand_line(position) {
            self.list.scroll_to_reveal_item(row);
        }
    }

    pub(crate) fn select_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((file, side)) = self.active_side(cx) else {
            return;
        };
        let count = self.documents[file].lines_count(side);
        TextSelection::clear(window, cx);
        self.select_user_range(
            self.position(file, side, 0),
            self.position(file, side, count - 1),
            window,
            cx,
        );
    }

    /// Records the window that renders this state, for clearing its text selection.
    pub(crate) fn set_window(&mut self, window: WindowId) {
        self.window = Some(window);
    }

    /// Updates width and row-height measurement when fonts or layout change.
    /// Returns whether content width must be measured again.
    pub(crate) fn update_layout(&mut self, key: LayoutKey, row_height: Pixels) -> bool {
        if self.layout.as_ref() == Some(&key) {
            return false;
        }
        if self.layout.is_some() {
            self.list.remeasure();
        }
        self.layout = Some(key);
        self.row_height = Some(row_height);
        self.apply_row_height_hint();
        true
    }

    /// Re-measures rows whose annotations were added, removed or replaced.
    /// Visible rows are re-measured every frame, so content changes inside an
    /// annotation need no notification.
    pub(crate) fn sync_annotations(&mut self, annotations: &[DiffLineAnnotation]) {
        let current = annotations
            .iter()
            .map(|annotation| (annotation.position(), annotation.id().clone()))
            .collect::<HashSet<_>>();
        if current == self.annotations {
            return;
        }
        let rows = current
            .symmetric_difference(&self.annotations)
            .filter_map(|(position, _)| self.source_row(*position))
            .collect::<Vec<_>>();
        for row in rows {
            self.list.remeasure_items(row..row + 1);
        }
        self.annotations = current;
    }

    pub(crate) fn position(&self, file: usize, side: DiffSide, ix: usize) -> DiffLinePosition {
        DiffLinePosition::new(file, side, self.documents[file].line_number(side, ix))
    }

    fn line_index(&self, position: DiffLinePosition) -> Option<usize> {
        self.documents
            .get(position.file())?
            .line_index(position.side(), position.line())
    }

    fn selection_is_local(&self, cx: &App) -> bool {
        let entity = Some(self.selection.entity_id());
        self.selection
            .snapshot(cx)
            .is_some_and(|snapshot| snapshot.anchor().entity_id() == entity)
    }

    /// The file and side that keyboard selection and Select All act on.
    fn active_side(&self, cx: &App) -> Option<(usize, DiffSide)> {
        let (file, side) = self
            .selected_lines
            .map(|range| (range.file(), Some(range.side())))
            .or_else(|| {
                super::selection::selected_source_range(&self.selection, cx)
                    .map(|(file, side, _)| (file, Some(side)))
            })
            .or_else(|| {
                let top = self.list.logical_scroll_top().item_ix;
                self.rows.get(top).map(|row| (row.file(), None))
            })?;
        let document = self.documents.get(file)?;
        let side = side.unwrap_or(if document.lines_count(DiffSide::Modified) > 0 {
            DiffSide::Modified
        } else {
            DiffSide::Original
        });
        (document.lines_count(side) > 0).then_some((file, side))
    }

    fn expand_line(&mut self, position: DiffLinePosition) -> Option<usize> {
        let row = self.source_row(position)?;
        if let Some(DisplayRow::Fold { file, pairs }) = self.rows.get(row) {
            let (file, pairs) = (*file, pairs.clone());
            self.expanded[file].push(pairs);
            self.rebuild_at_anchor();
        }
        self.source_row(position)
    }

    fn select_user_range(
        &mut self,
        anchor: DiffLinePosition,
        cursor: DiffLinePosition,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = Some(DiffLineRange::new(
            cursor.file(),
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

    /// The source line shown by a row, looking past headers within its file.
    fn row_position(&self, ix: usize) -> Option<DiffLinePosition> {
        let row = self.rows.get(ix)?;
        let file = row.file();
        let pair_position = |original: Option<usize>, modified: Option<usize>| {
            modified
                .map(|ix| self.position(file, DiffSide::Modified, ix))
                .or_else(|| original.map(|ix| self.position(file, DiffSide::Original, ix)))
        };
        match row {
            DisplayRow::Code {
                original, modified, ..
            } => pair_position(*original, *modified),
            DisplayRow::Fold { pairs, .. } => self.documents[file]
                .pairs()
                .get(pairs.start)
                .and_then(|pair| pair_position(pair.original(), pair.modified())),
            DisplayRow::File(_) | DisplayRow::Notice(_) | DisplayRow::Hunk { .. } => self
                .rows
                .get(ix + 1)
                .filter(|next| next.file() == file)
                .and_then(|_| self.row_position(ix + 1)),
        }
    }

    pub(crate) fn source_row(&self, position: DiffLinePosition) -> Option<usize> {
        let rows = self.file_rows.get(position.file())?;
        let rows = match position.side() {
            DiffSide::Original => &rows.original,
            DiffSide::Modified => &rows.modified,
        };
        rows.get(self.line_index(position)?).copied()
    }

    fn reveal_line(&self, position: DiffLinePosition) {
        if let Some(item_ix) = self.source_row(position) {
            self.list.scroll_to(ListOffset {
                item_ix,
                offset_in_item: px(0.),
            });
        }
    }

    /// Rebuilds rows and restores the source line at the top of the viewport.
    fn rebuild_at_anchor(&mut self) {
        let anchor = self.row_position(self.list.logical_scroll_top().item_ix);
        self.rebuild(false);
        if let Some(anchor) = anchor {
            self.reveal_line(anchor);
        }
    }

    /// Projects rows. Without `reset`, only the changed middle is spliced so
    /// measured heights of unchanged rows survive disclosure changes.
    fn rebuild(&mut self, reset: bool) {
        let rows = project_rows(
            &self.documents,
            self.mode,
            self.context_lines,
            &self.expanded,
        );
        self.file_rows = source_rows(&self.documents, &rows);
        self.change_rows = changed_groups(&rows);
        if reset {
            self.list.reset(rows.len());
        } else {
            let old = &self.rows;
            let prefix = old.iter().zip(&rows).take_while(|(a, b)| a == b).count();
            let suffix = old
                .iter()
                .rev()
                .zip(rows.iter().rev())
                .take(old.len().min(rows.len()) - prefix)
                .take_while(|(a, b)| a == b)
                .count();
            self.list
                .splice(prefix..old.len() - suffix, rows.len() - prefix - suffix);
        }
        self.rows = Rc::new(rows);
        self.apply_row_height_hint();
    }

    /// Gives unmeasured rows a code-row height so the scrollbar is sized for
    /// the whole patch before every row has been rendered.
    fn apply_row_height_hint(&self) {
        if let Some(height) = self.row_height {
            let _ = self.list.clone().with_uniform_item_height(height);
        }
    }

    fn prepare_syntax(&mut self, cx: &mut Context<Self>) {
        let pending = self
            .documents
            .iter()
            .filter(|document| document.syntax().is_none())
            .cloned()
            .collect::<Vec<_>>();
        self._syntax = if pending.is_empty() {
            Task::ready(())
        } else {
            cx.spawn(async move |this, cx| {
                let mut highlighters = Some(SyntaxHighlighters::default());
                for document in pending {
                    let mut moved = highlighters.take();
                    highlighters = cx
                        .background_spawn(async move {
                            document.prepare_syntax(moved.get_or_insert_default());
                            moved
                        })
                        .await;
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        return;
                    }
                }
            })
        };
    }
}

/// Every source line maps to its code row or the disclosure hiding it.
/// Disjoint folds visit each pair once, so building the maps is linear.
fn source_rows(documents: &[DiffDocument], rows: &[DisplayRow]) -> Vec<FileRows> {
    let mut file_rows = documents
        .iter()
        .map(|document| FileRows {
            original: vec![0; document.lines_count(DiffSide::Original)],
            modified: vec![0; document.lines_count(DiffSide::Modified)],
        })
        .collect::<Vec<_>>();
    for (row_ix, row) in rows.iter().enumerate() {
        let mut map = |file: usize, original: Option<usize>, modified: Option<usize>| {
            if let Some(line) = original {
                file_rows[file].original[line] = row_ix;
            }
            if let Some(line) = modified {
                file_rows[file].modified[line] = row_ix;
            }
        };
        match row {
            DisplayRow::Code {
                file,
                original,
                modified,
                ..
            } => map(*file, *original, *modified),
            DisplayRow::Fold { file, pairs } => {
                for pair in &documents[*file].pairs()[pairs.clone()] {
                    map(*file, pair.original(), pair.modified());
                }
            }
            DisplayRow::File(_) | DisplayRow::Notice(_) | DisplayRow::Hunk { .. } => {}
        }
    }
    file_rows
}

fn changed_groups(rows: &[DisplayRow]) -> Vec<usize> {
    let changed = |row: &DisplayRow| matches!(row, DisplayRow::Code { changed: true, .. });
    rows.iter()
        .enumerate()
        .filter_map(|(ix, row)| {
            (changed(row)
                && (ix == 0 || !changed(&rows[ix - 1]) || rows[ix - 1].file() != row.file()))
            .then_some(ix)
        })
        .collect()
}

const SIDE_BIT: u64 = 1 << 40;
const OFFSET_MASK: u64 = SIDE_BIT - 1;

/// Packs a source offset into a selection key, ordered by file, side and offset.
pub(crate) fn encode_key(file: usize, side: DiffSide, offset: usize) -> u64 {
    ((file as u64) << 41)
        | if side == DiffSide::Modified {
            SIDE_BIT
        } else {
            0
        }
        | (offset as u64 & OFFSET_MASK)
}
pub(crate) fn decode_key(key: u64) -> (usize, DiffSide, usize) {
    (
        (key >> 41) as usize,
        if key & SIDE_BIT == 0 {
            DiffSide::Original
        } else {
            DiffSide::Modified
        },
        (key & OFFSET_MASK) as usize,
    )
}

fn project_rows(
    documents: &[DiffDocument],
    mode: DiffMode,
    context: Option<usize>,
    expanded: &[Vec<Range<usize>>],
) -> Vec<DisplayRow> {
    let mut rows = Vec::new();
    for (file, document) in documents.iter().enumerate() {
        rows.push(DisplayRow::File(file));
        if document.pairs().is_empty() {
            rows.push(DisplayRow::Notice(file));
        } else {
            project_file(&mut rows, file, document, mode, context, &expanded[file]);
        }
    }
    rows
}

fn project_file(
    rows: &mut Vec<DisplayRow>,
    file: usize,
    document: &DiffDocument,
    mode: DiffMode,
    context: Option<usize>,
    expanded: &[Range<usize>],
) {
    let pairs = document.pairs();
    let hunks = document.hunks();
    let mut hunk_end = vec![pairs.len(); pairs.len()];
    let mut hunk_start = vec![0; pairs.len()];
    for hunk in hunks {
        hunk_start[hunk.pairs()].fill(hunk.pairs().start);
        hunk_end[hunk.pairs()].fill(hunk.pairs().end);
    }
    // Interval boundaries avoid repeatedly filling overlapping context windows.
    // Projection remains linear in source lines plus the number of disclosures.
    let mut boundaries = vec![0isize; pairs.len() + 1];
    if let Some(context) = context {
        let mut ix = 0;
        while ix < pairs.len() {
            if !pairs[ix].is_changed() {
                ix += 1;
                continue;
            }
            let start = ix;
            while ix < pairs.len() && ix < hunk_end[start] && pairs[ix].is_changed() {
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
    let mut hunk_ix = 0;
    let mut ix = 0;
    while ix < pairs.len() {
        while let Some(hunk) = hunks.get(hunk_ix) {
            if hunk.pairs().start != ix {
                break;
            }
            rows.push(DisplayRow::Hunk {
                file,
                hunk: hunk_ix,
            });
            hunk_ix += 1;
        }
        if !visible[ix] {
            let start = ix;
            while ix < pairs.len() && ix < hunk_end[start] && !visible[ix] {
                ix += 1;
            }
            rows.push(DisplayRow::Fold {
                file,
                pairs: start..ix,
            });
        } else if mode == DiffMode::Unified && pairs[ix].is_changed() {
            let start = ix;
            while ix < pairs.len() && ix < hunk_end[start] && pairs[ix].is_changed() {
                ix += 1;
            }
            for pair in &pairs[start..ix] {
                if let Some(original) = pair.original() {
                    rows.push(DisplayRow::Code {
                        file,
                        original: Some(original),
                        modified: None,
                        changed: true,
                    });
                }
            }
            for pair in &pairs[start..ix] {
                if let Some(modified) = pair.modified() {
                    rows.push(DisplayRow::Code {
                        file,
                        original: None,
                        modified: Some(modified),
                        changed: true,
                    });
                }
            }
        } else {
            let pair = pairs[ix];
            rows.push(DisplayRow::Code {
                file,
                original: pair.original(),
                modified: pair.modified(),
                changed: pair.is_changed(),
            });
            ix += 1;
        }
    }
}
