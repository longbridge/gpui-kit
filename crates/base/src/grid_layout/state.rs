use gpui::{
    App, Axis, Bounds, Context, EntityId, EventEmitter, FocusHandle, Pixels, Point, SharedString,
    Size, Window, px,
};

use super::layout::{self, GridArea, LayoutRules};

/// Emitted by [`GridLayoutState`] when the layout changes.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum GridLayoutEvent {
    /// A drag, a resize, a keyboard command, or [`GridLayoutState::move_item`]
    /// or [`GridLayoutState::resize_item`] produced a new layout, carried here
    /// so the application can persist it. [`GridLayoutState::set_areas`] does
    /// not emit this: the application already holds the layout it set.
    Change(Vec<GridArea>),
}

/// What a pointer gesture on an item is doing to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GridGesture {
    Move,
    Resize,
}

/// The payload GPUI carries while an item is dragged.
#[derive(Clone)]
pub(crate) struct DragGridItem {
    pub(crate) grid: EntityId,
    pub(crate) item: SharedString,
    pub(crate) gesture: GridGesture,
}

/// Pixel geometry of a grid at one container width.
///
/// Columns share the width left over by the column gaps between them, so an
/// item spanning `n` columns is `n` column widths plus the `n - 1` gaps it
/// covers; rows work the same way with the row gap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GridMetrics {
    pub(crate) width: Pixels,
    pub(crate) column_width: Pixels,
    pub(crate) row_height: Pixels,
    /// The column gap across, the row gap down.
    pub(crate) gap: Size<Pixels>,
}

impl GridMetrics {
    pub(crate) fn new(
        width: Pixels,
        columns: usize,
        row_height: Pixels,
        gap: Size<Pixels>,
    ) -> Self {
        let columns = columns.max(1) as f32;
        Self {
            width,
            column_width: ((width - gap.width * (columns - 1.)) / columns).max(px(0.)),
            row_height,
            gap,
        }
    }

    pub(crate) fn left(&self, col: usize) -> Pixels {
        (self.column_width + self.gap.width) * col as f32
    }

    pub(crate) fn top(&self, row: usize) -> Pixels {
        (self.row_height + self.gap.height) * row as f32
    }

    pub(crate) fn span_width(&self, col_span: usize) -> Pixels {
        (self.column_width * col_span as f32 + self.gap.width * col_span.saturating_sub(1) as f32)
            .max(px(0.))
    }

    pub(crate) fn span_height(&self, row_span: usize) -> Pixels {
        (self.row_height * row_span as f32 + self.gap.height * row_span.saturating_sub(1) as f32)
            .max(px(0.))
    }

    /// The column whose leading edge is nearest `left`.
    fn column_at(&self, left: Pixels) -> usize {
        snap(left, self.column_width + self.gap.width)
    }

    /// The row whose top edge is nearest `top`.
    fn row_at(&self, top: Pixels) -> usize {
        snap(top, self.row_height + self.gap.height)
    }

    /// The number of columns whose span is nearest `width`.
    fn columns_spanning(&self, width: Pixels) -> usize {
        snap(width + self.gap.width, self.column_width + self.gap.width).max(1)
    }

    /// The number of rows whose span is nearest `height`.
    fn rows_spanning(&self, height: Pixels) -> usize {
        snap(height + self.gap.height, self.row_height + self.gap.height).max(1)
    }
}

fn snap(length: Pixels, step: Pixels) -> usize {
    if step <= px(0.) {
        return 0;
    }
    (length / step).round().max(0.) as usize
}

/// An item being dragged or resized by the pointer.
pub(crate) struct GridDrag {
    pub(crate) item: SharedString,
    pub(crate) gesture: GridGesture,
    /// The pointer where the gesture began, in window coordinates.
    origin: Point<Pixels>,
    /// The layout when the gesture began; every step is computed from it.
    start: Vec<GridArea>,
    /// Where the items would settle if the pointer were released now.
    pub(crate) preview: Vec<GridArea>,
    /// The pointer's travel since `origin`.
    pub(crate) delta: Point<Pixels>,
    /// The grid's geometry, known once the pointer first moves.
    pub(crate) metrics: Option<GridMetrics>,
}

impl GridDrag {
    /// The dragged item as it was when the gesture began.
    pub(crate) fn start_area(&self) -> Option<&GridArea> {
        self.start.iter().find(|area| area.id == self.item)
    }
}

/// The state of a [`GridLayout`](super::GridLayout): the items' areas, the
/// rules the layout settles under, and any gesture in progress.
///
/// The application owns the layout. Create the state with the grid's rules,
/// then its areas, and subscribe to [`GridLayoutEvent::Change`] to persist
/// what the user arranges.
///
/// ```ignore
/// let state = cx.new(|cx| {
///     GridLayoutState::new(cx).cols(12).default_areas([
///         GridArea::new("revenue").col_span(8).row_span(4),
///         GridArea::new("orders").col(8).col_span(4).row_span(4),
///     ])
/// });
/// ```
pub struct GridLayoutState {
    areas: Vec<GridArea>,
    columns: usize,
    compaction: Option<Axis>,
    prevent_collision: bool,
    draggable: bool,
    resizable: bool,
    focus_handle: FocusHandle,
    selected: Option<SharedString>,
    /// Whether the last interaction came from the keyboard, which is when the
    /// selected item shows its selected style.
    keyboard_active: bool,
    /// Where the pointer last pressed an item, so a drag starts from the press
    /// rather than from wherever GPUI's drag threshold was crossed.
    press_position: Option<Point<Pixels>>,
    pub(crate) drag: Option<GridDrag>,
}

impl EventEmitter<GridLayoutEvent> for GridLayoutState {}

impl GridLayoutState {
    /// An empty, 12-column grid that compacts vertically.
    pub fn new(cx: &mut App) -> Self {
        Self {
            areas: Vec::new(),
            columns: 12,
            compaction: Some(Axis::Vertical),
            prevent_collision: false,
            draggable: true,
            resizable: true,
            focus_handle: cx.focus_handle().tab_stop(true),
            selected: None,
            keyboard_active: false,
            press_position: None,
            drag: None,
        }
    }

    /// The number of equal columns the width is divided into. Defaults to 12.
    pub fn cols(mut self, cols: usize) -> Self {
        self.columns = cols.max(1);
        self.normalize();
        self
    }

    /// The direction items float toward when space opens up, or `None` to
    /// leave them wherever they are put. Defaults to `Some(Axis::Vertical)`.
    pub fn compaction(mut self, compaction: Option<Axis>) -> Self {
        self.compaction = compaction;
        self.normalize();
        self
    }

    /// Refuse a move or resize that would overlap another item, instead of
    /// pushing that item out of the way. Defaults to `false`.
    pub fn prevent_collision(mut self, prevent_collision: bool) -> Self {
        self.prevent_collision = prevent_collision;
        self
    }

    /// Whether items can be moved. Defaults to `true`.
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    /// Whether items can be resized. Defaults to `true`.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// The areas the grid starts with, settled under the rules set so far:
    /// overlaps are pushed apart and, with compaction on, gaps close. Set the
    /// rules first.
    pub fn default_areas(mut self, areas: impl IntoIterator<Item = GridArea>) -> Self {
        self.areas = areas.into_iter().collect();
        self.normalize();
        self
    }

    /// The committed layout, in the order the areas were given.
    pub fn areas(&self) -> &[GridArea] {
        &self.areas
    }

    /// The area of the item `id`.
    pub fn area(&self, id: &str) -> Option<&GridArea> {
        self.areas.iter().find(|area| area.id.as_ref() == id)
    }

    pub fn column_count(&self) -> usize {
        self.columns
    }

    pub fn compaction_axis(&self) -> Option<Axis> {
        self.compaction
    }

    pub fn is_collision_prevented(&self) -> bool {
        self.prevent_collision
    }

    pub fn is_draggable(&self) -> bool {
        self.draggable
    }

    pub fn is_resizable(&self) -> bool {
        self.resizable
    }

    /// The item keyboard commands act on.
    pub fn selected_item(&self) -> Option<&SharedString> {
        self.selected.as_ref()
    }

    /// The item being dragged or resized, if a gesture is in progress.
    pub fn active_item(&self) -> Option<&SharedString> {
        self.drag.as_ref().map(|drag| &drag.item)
    }

    pub(crate) fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    pub(crate) fn is_keyboard_active(&self) -> bool {
        self.keyboard_active
    }

    /// Replace the layout, settling it under the grid's rules. Cancels any
    /// gesture in progress, and does not emit [`GridLayoutEvent::Change`].
    pub fn set_areas(&mut self, areas: impl IntoIterator<Item = GridArea>, cx: &mut Context<Self>) {
        self.areas = areas.into_iter().collect();
        self.drag = None;
        self.normalize();
        if let Some(selected) = &self.selected
            && self.area(selected).is_none()
        {
            self.selected = None;
        }
        cx.notify();
    }

    pub fn set_cols(&mut self, cols: usize, cx: &mut Context<Self>) {
        self.columns = cols.max(1);
        self.drag = None;
        self.normalize();
        cx.notify();
    }

    pub fn set_compaction(&mut self, compaction: Option<Axis>, cx: &mut Context<Self>) {
        self.compaction = compaction;
        self.drag = None;
        self.normalize();
        cx.notify();
    }

    pub fn set_prevent_collision(&mut self, prevent_collision: bool, cx: &mut Context<Self>) {
        self.prevent_collision = prevent_collision;
        cx.notify();
    }

    pub fn set_draggable(&mut self, draggable: bool, cx: &mut Context<Self>) {
        self.draggable = draggable;
        cx.notify();
    }

    pub fn set_resizable(&mut self, resizable: bool, cx: &mut Context<Self>) {
        self.resizable = resizable;
        cx.notify();
    }

    /// Select the item keyboard commands act on.
    pub fn set_selected_item(&mut self, id: Option<SharedString>, cx: &mut Context<Self>) {
        let id = id.filter(|id| self.area(id).is_some());
        if self.selected != id {
            self.selected = id;
            cx.notify();
        }
    }

    /// Move the item `id` so its leading column is `col` and its top row
    /// `row`, pushing whatever it lands on out of the way. Returns whether the
    /// layout changed; a refused or no-op move returns `false`.
    pub fn move_item(&mut self, id: &str, col: usize, row: usize, cx: &mut Context<Self>) -> bool {
        let next = layout::move_item(&self.areas, id, col, row, self.rules());
        self.commit(next, cx)
    }

    /// Resize the item `id` to cover `col_span` columns and `row_span` rows,
    /// clamped to its limits. Returns whether the layout changed.
    pub fn resize_item(
        &mut self,
        id: &str,
        col_span: usize,
        row_span: usize,
        cx: &mut Context<Self>,
    ) -> bool {
        let next = layout::resize_item(&self.areas, id, col_span, row_span, self.rules());
        self.commit(next, cx)
    }

    fn rules(&self) -> LayoutRules {
        LayoutRules {
            columns: self.columns,
            compaction: self.compaction,
            prevent_collision: self.prevent_collision,
        }
    }

    fn normalize(&mut self) {
        let rules = self.rules();
        layout::normalize(&mut self.areas, rules);
    }

    fn commit(&mut self, next: Option<Vec<GridArea>>, cx: &mut Context<Self>) -> bool {
        match next {
            Some(next) if next != self.areas => {
                self.areas = next;
                cx.emit(GridLayoutEvent::Change(self.areas.clone()));
                cx.notify();
                true
            }
            _ => false,
        }
    }

    /// The layout to draw: a gesture's preview while one is in progress.
    pub(crate) fn displayed_areas(&self, cx: &App) -> &[GridArea] {
        match &self.drag {
            Some(drag) if cx.has_active_drag() => &drag.preview,
            _ => &self.areas,
        }
    }

    /// The active gesture, unless GPUI has already abandoned the drag.
    pub(crate) fn active_drag(&self, cx: &App) -> Option<&GridDrag> {
        self.drag.as_ref().filter(|_| cx.has_active_drag())
    }

    pub(crate) fn is_gesture_allowed(&self, id: &str, gesture: GridGesture) -> bool {
        let allowed = match gesture {
            GridGesture::Move => self.draggable,
            GridGesture::Resize => self.resizable,
        };
        allowed && self.area(id).is_some_and(|area| !area.locked)
    }

    /// The pointer pressed an item: select it and take focus.
    pub(crate) fn press(
        &mut self,
        id: &SharedString,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.press_position = Some(position);
        self.keyboard_active = false;
        self.selected = Some(id.clone());
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    pub(crate) fn begin_drag(
        &mut self,
        id: SharedString,
        gesture: GridGesture,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let origin = self
            .press_position
            .take()
            .unwrap_or_else(|| window.mouse_position());
        self.selected = Some(id.clone());
        self.drag = Some(GridDrag {
            item: id,
            gesture,
            origin,
            start: self.areas.clone(),
            preview: self.areas.clone(),
            delta: Point::default(),
            metrics: None,
        });
        cx.notify();
    }

    /// Follow the pointer to `position` over a grid laid out in `bounds`, with
    /// rows `row_height` tall and `gap` between cells.
    pub(crate) fn drag_to(
        &mut self,
        position: Point<Pixels>,
        bounds: Bounds<Pixels>,
        row_height: Pixels,
        gap: Size<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let metrics = GridMetrics::new(bounds.size.width, self.columns, row_height, gap);
        let rules = self.rules();
        let Some(drag) = self.drag.as_mut() else {
            return;
        };
        let Some(start) = drag.start_area().cloned() else {
            return;
        };
        drag.delta = position - drag.origin;
        drag.metrics = Some(metrics);

        let next = match drag.gesture {
            GridGesture::Move => {
                let left = metrics.left(start.col) + drag.delta.x;
                let top = metrics.top(start.row) + drag.delta.y;
                let col = metrics
                    .column_at(left)
                    .min(self.columns.saturating_sub(start.col_span));
                layout::move_item(&drag.start, &start.id, col, metrics.row_at(top), rules)
            }
            GridGesture::Resize => {
                let col_span =
                    metrics.columns_spanning(metrics.span_width(start.col_span) + drag.delta.x);
                let row_span =
                    metrics.rows_spanning(metrics.span_height(start.row_span) + drag.delta.y);
                layout::resize_item(&drag.start, &start.id, col_span, row_span, rules)
            }
        };
        // A refused step keeps the last accepted preview.
        if let Some(next) = next {
            drag.preview = next;
        }
        cx.notify();
    }

    /// The pointer was released: commit the preview.
    pub(crate) fn end_drag(&mut self, cx: &mut Context<Self>) {
        self.press_position = None;
        let Some(drag) = self.drag.take() else {
            return;
        };
        cx.notify();
        self.commit(Some(drag.preview), cx);
    }

    /// Abandon the gesture, restoring the layout it started from.
    pub(crate) fn cancel_drag(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.drag.take().is_none() {
            return false;
        }
        cx.stop_active_drag(window);
        cx.notify();
        true
    }

    /// Move the selected item one cell along `cols`, `rows`.
    pub(crate) fn nudge_selected(
        &mut self,
        cols: isize,
        rows: isize,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(area) = self.selected_area() else {
            return false;
        };
        if !self.is_gesture_allowed(&area.id, GridGesture::Move) {
            return false;
        }
        let (Some(col), Some(row)) = (
            area.col.checked_add_signed(cols),
            area.row.checked_add_signed(rows),
        ) else {
            return false;
        };
        self.keyboard_active = true;
        let moved = self.move_item(&area.id, col, row, cx);
        cx.notify();
        moved
    }

    /// Grow or shrink the selected item by one cell along `cols`, `rows`.
    pub(crate) fn resize_selected(
        &mut self,
        cols: isize,
        rows: isize,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(area) = self.selected_area() else {
            return false;
        };
        if !self.is_gesture_allowed(&area.id, GridGesture::Resize) {
            return false;
        }
        let (Some(col_span), Some(row_span)) = (
            area.col_span.checked_add_signed(cols),
            area.row_span.checked_add_signed(rows),
        ) else {
            return false;
        };
        self.keyboard_active = true;
        let resized = self.resize_item(&area.id, col_span, row_span, cx);
        cx.notify();
        resized
    }

    /// Select the next or previous item in reading order.
    ///
    /// Returns `false` past the last or before the first item, leaving the
    /// selection as it is so Tab moves focus out of the grid.
    pub(crate) fn select_adjacent(&mut self, forward: bool, cx: &mut Context<Self>) -> bool {
        let mut order = self.areas.iter().collect::<Vec<_>>();
        order.sort_by_key(|area| (area.row, area.col));
        if order.is_empty() {
            return false;
        }
        let current = self
            .selected
            .as_ref()
            .and_then(|id| order.iter().position(|area| &area.id == id));
        let next = match (current, forward) {
            (None, true) => 0,
            (None, false) => order.len() - 1,
            (Some(ix), true) if ix + 1 < order.len() => ix + 1,
            (Some(ix), false) if ix > 0 => ix - 1,
            (Some(_), _) => return false,
        };
        self.keyboard_active = true;
        self.selected = Some(order[next].id.clone());
        cx.notify();
        true
    }

    fn selected_area(&self) -> Option<GridArea> {
        self.selected.as_ref().and_then(|id| self.area(id)).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid_layout::layout::tests::area;
    use gpui::{AppContext as _, TestAppContext, size};

    fn state(cx: &mut TestAppContext) -> gpui::Entity<GridLayoutState> {
        cx.update(|cx| {
            cx.new(|cx| {
                GridLayoutState::new(cx)
                    .default_areas([area("a", 0, 0, 4, 2), area("b", 4, 0, 4, 2)])
            })
        })
    }

    #[test]
    fn metrics_share_the_width_between_columns_and_gaps() {
        let metrics = GridMetrics::new(px(1190.), 12, px(40.), size(px(10.), px(20.)));
        assert_eq!(metrics.column_width, px(90.));
        assert_eq!(metrics.left(1), px(100.));
        assert_eq!(metrics.span_width(12), px(1190.));
        // Rows use the row gap, not the column gap.
        assert_eq!(metrics.top(1), px(60.));
        assert_eq!(metrics.span_height(2), px(100.));
        assert_eq!(metrics.column_at(px(140.)), 1);
        assert_eq!(metrics.column_at(px(160.)), 2);
        assert_eq!(metrics.column_at(px(-50.)), 0);
        assert_eq!(metrics.columns_spanning(metrics.span_width(3) + px(20.)), 3);
        assert_eq!(metrics.rows_spanning(px(0.)), 1);
    }

    #[gpui::test]
    fn test_grid_layout_state_builder(cx: &mut TestAppContext) {
        let state = cx.update(|cx| {
            cx.new(|cx| {
                GridLayoutState::new(cx)
                    .cols(6)
                    .compaction(None)
                    .prevent_collision(true)
                    .draggable(false)
                    .resizable(false)
                    .default_areas([area("a", 4, 5, 4, 1)])
            })
        });
        state.read_with(cx, |state, _| {
            assert_eq!(state.column_count(), 6);
            assert_eq!(state.compaction_axis(), None);
            assert!(state.is_collision_prevented());
            assert!(!state.is_draggable());
            assert!(!state.is_resizable());
            // Fitted to six columns, and left where it is without compaction.
            let area = state.area("a").unwrap();
            assert_eq!((area.col, area.row), (2, 5));
        });
    }

    #[gpui::test]
    fn programmatic_changes_emit_the_new_layout(cx: &mut TestAppContext) {
        let state = state(cx);
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        cx.update(|cx| {
            let events = events.clone();
            cx.subscribe(&state, move |_, event: &GridLayoutEvent, _| {
                events.borrow_mut().push(event.clone())
            })
            .detach();
        });

        state.update(cx, |state, cx| {
            assert!(state.move_item("a", 8, 0, cx));
            // Already there: nothing changes and nothing is emitted.
            assert!(!state.move_item("a", 8, 0, cx));
            assert!(state.resize_item("b", 2, 3, cx));
            state.set_areas([area("c", 0, 0, 1, 1)], cx);
        });

        let events = events.borrow();
        assert_eq!(events.len(), 2);
        let GridLayoutEvent::Change(areas) = &events[1];
        assert!(
            areas
                .iter()
                .any(|area| area.id == "b" && area.col_span == 2 && area.row_span == 3)
        );
    }

    #[gpui::test]
    fn keyboard_commands_act_on_the_selected_item(cx: &mut TestAppContext) {
        let state = state(cx);
        state.update(cx, |state, cx| {
            assert!(!state.nudge_selected(1, 0, cx));
            assert!(state.select_adjacent(true, cx));
            assert_eq!(state.selected_item().map(|id| id.as_ref()), Some("a"));
            assert!(state.select_adjacent(true, cx));
            assert_eq!(state.selected_item().map(|id| id.as_ref()), Some("b"));
            // Past the last item Tab is not handled, so focus can leave the
            // grid, and the selection stays where it was.
            assert!(!state.select_adjacent(true, cx));
            assert_eq!(state.selected_item().map(|id| id.as_ref()), Some("b"));

            assert!(state.nudge_selected(1, 0, cx));
            assert_eq!(state.area("b").unwrap().col, 5);
            // Cannot move past the leading edge or the top.
            assert!(!state.nudge_selected(0, -1, cx));

            assert!(state.resize_selected(0, 1, cx));
            assert_eq!(state.area("b").unwrap().row_span, 3);
            assert!(state.is_keyboard_active());

            state.set_draggable(false, cx);
            assert!(!state.nudge_selected(-1, 0, cx));

            // Before the first item, Shift-Tab is not handled either.
            state.set_selected_item(Some("a".into()), cx);
            assert!(!state.select_adjacent(false, cx));
        });
    }
}
