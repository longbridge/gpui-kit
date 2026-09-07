use std::{
    collections::HashMap,
    ops::Range,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use gpui::{
    App, AppContext as _, Bounds, Context, Element, ElementId, Entity, EntityId, EventEmitter,
    Global, GlobalElementId, Half, Hitbox, InputEvent as _, InspectorElementId, IntoElement,
    LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    ScrollDelta, ScrollWheelEvent, SharedString, Style, Subscription, TextLayout, WeakEntity,
    Window, point, px,
};

use crate::text_boundary::{line_range_at, word_range_at};
use crate::{AutoScroll, GlobalState};

/// An opaque selection layer identifier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextSelectionScopeId(u64);

impl TextSelectionScopeId {
    /// Allocates a process-unique scope identifier.
    ///
    /// Keep the returned identifier for the semantic lifetime of the scope;
    /// do not allocate a new identifier on every frame.
    pub fn new() -> Self {
        static NEXT_SCOPE_ID: AtomicU64 = AtomicU64::new(1);
        let value = NEXT_SCOPE_ID
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .expect("text selection scope identifiers exhausted");
        Self(value)
    }

    #[cfg(test)]
    const fn from_raw(value: u64) -> Self {
        Self(value)
    }
}

/// Stable participant-defined identity for virtualized participant content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextSelectionContentKey(u64);

impl TextSelectionContentKey {
    /// Creates a key from a participant-defined stable content identity.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the participant-defined value.
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// A selection endpoint anchored to a participant's content coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectionEndpoint {
    entity_id: Option<EntityId>,
    point: Point<Pixels>,
    content_key: Option<TextSelectionContentKey>,
}

impl TextSelectionEndpoint {
    /// Creates an endpoint at a participant-relative content point.
    pub(crate) const fn new(entity_id: Option<EntityId>, point: Point<Pixels>) -> Self {
        Self {
            entity_id,
            point,
            content_key: None,
        }
    }

    /// Sets participant-defined endpoint metadata.
    pub(crate) const fn with_content_key(mut self, content_key: TextSelectionContentKey) -> Self {
        self.content_key = Some(content_key);
        self
    }

    /// Returns the participant which owns this endpoint, when it hit one.
    pub const fn entity_id(&self) -> Option<EntityId> {
        self.entity_id
    }

    /// Returns the participant-relative content point.
    pub const fn content_point(&self) -> Point<Pixels> {
        self.point
    }

    /// Returns participant-defined endpoint metadata captured when it hit a participant.
    pub const fn content_key(&self) -> Option<TextSelectionContentKey> {
        self.content_key
    }
}

/// Window-coordinate anchor and cursor points for painting a selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectionWindowPoints {
    anchor: Point<Pixels>,
    cursor: Point<Pixels>,
}

impl TextSelectionWindowPoints {
    /// Returns the stable anchor in window coordinates.
    pub const fn anchor(&self) -> Point<Pixels> {
        self.anchor
    }

    /// Returns the moving cursor in window coordinates.
    pub const fn cursor(&self) -> Point<Pixels> {
        self.cursor
    }
}

/// Participant-relative selection endpoints with an optional rendering projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectionSnapshot {
    anchor: TextSelectionEndpoint,
    cursor: TextSelectionEndpoint,
    is_selecting: bool,
    window_points: Option<TextSelectionWindowPoints>,
    coverage: TextSelectionCoverage,
}

/// How much of one participant participates in a window selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextSelectionCoverage {
    /// Only the interval between this participant's two endpoints is selected.
    #[default]
    Bounded,
    /// The participant is selected from its beginning through its endpoint.
    FromStart,
    /// The participant is selected from its endpoint through its end.
    ToEnd,
    /// The entire participant lies between endpoints in other participants.
    Full,
}

impl TextSelectionSnapshot {
    /// Creates a snapshot from stable participant-relative endpoints.
    pub(crate) const fn new(anchor: TextSelectionEndpoint, cursor: TextSelectionEndpoint) -> Self {
        Self {
            anchor,
            cursor,
            is_selecting: false,
            window_points: None,
            coverage: TextSelectionCoverage::Bounded,
        }
    }

    /// Sets whether the pointer gesture is still active.
    pub(crate) const fn with_selecting(mut self, is_selecting: bool) -> Self {
        self.is_selecting = is_selecting;
        self
    }

    /// Sets the current window-coordinate rendering projection.
    pub(crate) const fn with_window_points(
        mut self,
        window_points: Option<TextSelectionWindowPoints>,
    ) -> Self {
        self.window_points = window_points;
        self
    }

    /// Sets the portion of the receiving participant covered by this selection.
    #[cfg(test)]
    pub(crate) const fn with_coverage(mut self, coverage: TextSelectionCoverage) -> Self {
        self.coverage = coverage;
        self
    }

    /// Returns the stable anchor endpoint.
    pub const fn anchor(&self) -> TextSelectionEndpoint {
        self.anchor
    }

    /// Returns the moving cursor endpoint.
    pub const fn cursor(&self) -> TextSelectionEndpoint {
        self.cursor
    }

    /// Returns whether the pointer gesture is still active.
    pub const fn is_selecting(&self) -> bool {
        self.is_selecting
    }

    /// Returns the window-coordinate endpoints for participants that need them.
    pub const fn window_points(&self) -> Option<TextSelectionWindowPoints> {
        self.window_points
    }

    /// Returns the portion of the receiving participant covered by this selection.
    pub const fn coverage(&self) -> TextSelectionCoverage {
        self.coverage
    }
}

/// Per-frame geometry reported by a [`TextSelectionHandle`] participant.
pub struct TextSelectionRegistration {
    hitbox: Hitbox,
    bounds: Bounds<Pixels>,
    scroll_offset: Point<Pixels>,
    scope: TextSelectionScopeId,
    document_order: u64,
    text_bounds: Vec<Bounds<Pixels>>,
}

impl TextSelectionRegistration {
    /// Creates a registration with default scope, order, and scroll offset.
    pub fn new(hitbox: Hitbox, bounds: Bounds<Pixels>) -> Self {
        Self {
            hitbox,
            bounds,
            scroll_offset: Point::default(),
            scope: TextSelectionScopeId::default(),
            document_order: 0,
            text_bounds: Vec::new(),
        }
    }

    /// Sets the participant's content scroll offset.
    pub fn with_scroll_offset(mut self, scroll_offset: Point<Pixels>) -> Self {
        self.scroll_offset = scroll_offset;
        self
    }

    /// Sets the opaque selection scope.
    pub fn with_scope(mut self, scope: TextSelectionScopeId) -> Self {
        self.scope = scope;
        self
    }

    /// Sets the stable logical document order.
    pub fn with_document_order(mut self, document_order: u64) -> Self {
        self.document_order = document_order;
        self
    }

    /// Sets the glyph-bearing bounds used to reject blank-only gestures.
    pub fn with_text_bounds(mut self, text_bounds: Vec<Bounds<Pixels>>) -> Self {
        self.text_bounds = text_bounds;
        self
    }

    /// Returns the participant hitbox.
    pub fn hitbox(&self) -> &Hitbox {
        &self.hitbox
    }

    /// Returns the participant's window-coordinate bounds.
    pub const fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    /// Returns the participant's content scroll offset.
    pub const fn scroll_offset(&self) -> Point<Pixels> {
        self.scroll_offset
    }

    /// Returns the opaque selection scope.
    pub const fn scope(&self) -> TextSelectionScopeId {
        self.scope
    }

    /// Returns the stable logical document order.
    pub const fn document_order(&self) -> u64 {
        self.document_order
    }

    /// Returns the glyph-bearing bounds used to reject blank-only gestures.
    pub fn text_bounds(&self) -> &[Bounds<Pixels>] {
        &self.text_bounds
    }
}

/// Laid-out text reported by a plain selection participant during paint.
#[derive(Clone)]
pub struct TextSelectionRun {
    /// Logical order within the containing participant.
    document_order: u64,
    /// The exact text used to produce `layout`.
    text: SharedString,
    /// Laid-out glyph geometry in window coordinates.
    layout: TextLayout,
    /// The run's window-coordinate paint bounds.
    bounds: Bounds<Pixels>,
}

impl TextSelectionRun {
    /// Creates a laid-out text run.
    pub fn new(text: impl Into<SharedString>, layout: TextLayout, bounds: Bounds<Pixels>) -> Self {
        Self {
            document_order: 0,
            text: text.into(),
            layout,
            bounds,
        }
    }

    /// Sets the run's logical order within the participant.
    pub const fn with_document_order(mut self, document_order: u64) -> Self {
        self.document_order = document_order;
        self
    }

    /// Returns the run's logical order within its participant.
    pub const fn document_order(&self) -> u64 {
        self.document_order
    }

    /// Returns the exact text used to produce the layout.
    pub fn text(&self) -> &SharedString {
        &self.text
    }

    /// Returns the laid-out glyph geometry.
    pub fn layout(&self) -> &TextLayout {
        &self.layout
    }

    /// Returns the run's window-coordinate paint bounds.
    pub const fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }
}

/// Selection projected onto a participant's laid-out text runs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextSelectionProjection {
    /// Selected UTF-8 byte ranges paired with the input runs.
    ranges: Vec<Option<Range<usize>>>,
    /// Whether the participant participates in the current selection.
    is_active: bool,
}

impl TextSelectionProjection {
    /// Returns selected UTF-8 byte ranges paired with the input runs.
    pub fn ranges(&self) -> &[Option<Range<usize>>] {
        &self.ranges
    }

    /// Returns whether the participant participates in the selection.
    pub const fn is_active(&self) -> bool {
        self.is_active
    }
}

/// Projects a participant selection snapshot onto laid-out plain-text runs.
///
/// The returned states retain the input order so callers can pair every state
/// with its run. The ranges are always character boundaries; `order` is used
/// only when a participant caches selected text for copying.
fn project_ranges(
    snapshot: Option<TextSelectionSnapshot>,
    runs: &[TextSelectionRun],
) -> TextSelectionProjection {
    let Some(snapshot) = snapshot else {
        return TextSelectionProjection {
            ranges: vec![None; runs.len()],
            is_active: false,
        };
    };
    let Some(window_points) = snapshot.window_points() else {
        return TextSelectionProjection {
            ranges: vec![None; runs.len()],
            is_active: true,
        };
    };

    TextSelectionProjection {
        ranges: runs
            .iter()
            .map(|run| selection_range_for_run(run, window_points.anchor, window_points.cursor))
            .collect(),
        is_active: true,
    }
}

fn selection_range_for_run(
    run: &TextSelectionRun,
    selection_start: Point<Pixels>,
    selection_end: Point<Pixels>,
) -> Option<Range<usize>> {
    if run.text.len() != run.layout.len() {
        return None;
    }

    let line_height = run.layout.line_height();
    let mut range = None;
    for (offset, character) in run.text.char_indices() {
        let next_offset = offset + character.len_utf8();
        let Some(position) = run.layout.position_for_index(offset) else {
            continue;
        };

        let char_width = run
            .layout
            .position_for_index(next_offset)
            .filter(|next| next.y == position.y)
            .map_or_else(|| line_height.half(), |next| next.x - position.x);

        if point_in_selection_band(
            position,
            char_width,
            selection_start,
            selection_end,
            line_height,
        ) {
            range.get_or_insert(offset..offset).end = next_offset;
        }
    }
    range
}

fn points_for_multi_click(
    runs: &[TextSelectionRun],
    position: Point<Pixels>,
    click_count: usize,
) -> Option<(Point<Pixels>, Point<Pixels>)> {
    let run = runs.iter().find(|run| run.bounds.contains(&position))?;
    if run.text.len() != run.layout.len() {
        return None;
    }
    let offset = run.layout.index_for_position(position).ok()?;
    let range = match click_count {
        2 => word_range_at(&run.text, offset)?,
        3.. => line_range_at(&run.text, offset),
        _ => return None,
    };
    if range.is_empty() {
        return None;
    }
    Some((
        run.layout.position_for_index(range.start)?,
        run.layout.position_for_index(range.end)?,
    ))
}

fn point_in_selection_band(
    position: Point<Pixels>,
    char_width: Pixels,
    selection_start: Point<Pixels>,
    selection_end: Point<Pixels>,
    line_height: Pixels,
) -> bool {
    let point_in_line =
        |point: Point<Pixels>| point.y >= position.y && point.y < position.y + line_height;
    let top = selection_start.y.min(selection_end.y);
    let bottom = selection_start.y.max(selection_end.y);
    let x = position.x + char_width.half();

    if position.y + line_height <= top || position.y > bottom {
        return false;
    }

    if point_in_line(selection_start) && point_in_line(selection_end) {
        let left = selection_start.x.min(selection_end.x);
        let right = selection_start.x.max(selection_end.x);
        return x >= left && x <= right;
    }

    let (top_point, bottom_point) = if selection_start.y < selection_end.y {
        (selection_start, selection_end)
    } else {
        (selection_end, selection_start)
    };
    if point_in_line(top_point) {
        x >= top_point.x
    } else if point_in_line(bottom_point) {
        x <= bottom_point.x
    } else {
        true
    }
}

type FocusCallback = Rc<dyn Fn(&mut Window, &mut App)>;
type ClearHandler = Rc<dyn Fn(&mut App)>;
type CopyCallback = Rc<dyn Fn(&mut App) -> String>;
type ContentKeyResolver = Rc<dyn Fn(Point<Pixels>, &App) -> Option<TextSelectionContentKey>>;

/// Notifications emitted by a text-selection participant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextSelectionEvent {
    /// The participant's window-selection projection changed.
    SelectionChanged(Option<TextSelectionSnapshot>),
    /// The active drag requests vertical auto-scroll, or `None` to stop.
    AutoScroll(Option<Pixels>),
    /// Window selection cleared the participant's participant-local state.
    Cleared,
}

struct CopyItem {
    document_order: u64,
    callback: Option<CopyCallback>,
    fallback: String,
}

fn resolve_copy_items(mut items: Vec<CopyItem>, cx: &mut App) -> String {
    items.sort_by_key(|item| item.document_order);
    items
        .into_iter()
        .map(|item| {
            item.callback
                .map(|callback| callback(cx))
                .unwrap_or(item.fallback)
        })
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn dispatch_clear_handlers(handlers: Vec<ClearHandler>, cx: &mut App) {
    for handler in handlers {
        handler(cx);
    }
}

struct SelectableTextState {
    fallback_copy_text: String,
    projected_copy_text: Option<String>,
    runs: Vec<TextSelectionRun>,
    local_selection: bool,
    snapshot: Option<TextSelectionSnapshot>,
    on_focus: Option<FocusCallback>,
    clear: Option<ClearHandler>,
    copy: Option<CopyCallback>,
    content_key_resolver: Option<ContentKeyResolver>,
}

impl EventEmitter<TextSelectionEvent> for SelectableTextState {}

impl SelectableTextState {
    fn new(fallback_copy_text: impl Into<String>) -> Self {
        Self {
            fallback_copy_text: fallback_copy_text.into(),
            projected_copy_text: None,
            runs: Vec::new(),
            local_selection: false,
            snapshot: None,
            on_focus: None,
            clear: None,
            copy: None,
            content_key_resolver: None,
        }
    }

    /// The current geometry selection snapshot for this participant.
    fn snapshot(&self) -> Option<TextSelectionSnapshot> {
        self.snapshot
    }

    /// Sets the text copied by this participant when it participates in selection.
    fn set_fallback_copy_text(&mut self, text: impl Into<String>) {
        self.fallback_copy_text = text.into();
        self.projected_copy_text = None;
    }

    /// Marks participant-local selection (for example select-all) as active.
    fn set_local_selection(&mut self, active: bool) {
        self.local_selection = active;
    }

    /// Projects this participant's current snapshot onto plain-text runs and caches
    /// their selected substrings for the window selection query.
    ///
    /// Call this once per painted run. A snapshot change or
    /// Clearing window selection invalidates the cache immediately, so copy
    /// never returns text from a previous projection while waiting to repaint.
    fn update_runs(&mut self, runs: &[TextSelectionRun]) -> TextSelectionProjection {
        self.runs = runs.to_vec();
        let states = project_ranges(self.snapshot, runs);
        let mut selected_runs = runs
            .iter()
            .zip(states.ranges())
            .enumerate()
            .filter_map(|(index, (run, state))| {
                state.as_ref().map(|range| {
                    debug_assert!(run.text.is_char_boundary(range.start));
                    debug_assert!(run.text.is_char_boundary(range.end));
                    (
                        run.document_order,
                        index,
                        run.text[range.clone()].to_string(),
                    )
                })
            })
            .collect::<Vec<_>>();
        selected_runs.sort_by_key(|(order, index, _)| (*order, *index));
        self.projected_copy_text =
            Some(selected_runs.into_iter().map(|(_, _, text)| text).collect());
        states
    }

    /// Installs the callback which focuses the participant when a drag begins in it.
    fn set_focus_handler(&mut self, callback: impl Fn(&mut Window, &mut App) + 'static) {
        self.on_focus = Some(Rc::new(callback));
    }

    fn clear_with(&mut self, callback: impl Fn(&mut App) + 'static) {
        self.clear = Some(Rc::new(callback));
    }

    /// Installs a participant-specific copy projection.
    fn copy_with(&mut self, callback: impl Fn(&mut App) -> String + 'static) {
        self.copy = Some(Rc::new(callback));
    }

    /// Installs a participant-specific lookup for stable virtualized content keys.
    fn resolve_content_key_with(
        &mut self,
        callback: impl Fn(Point<Pixels>, &App) -> Option<TextSelectionContentKey> + 'static,
    ) {
        self.content_key_resolver = Some(Rc::new(callback));
    }

    fn set_snapshot(&mut self, snapshot: Option<TextSelectionSnapshot>, cx: &mut Context<Self>) {
        if self.snapshot == snapshot {
            return;
        }
        self.snapshot = snapshot;
        self.projected_copy_text = None;
        cx.emit(TextSelectionEvent::SelectionChanged(snapshot));
    }

    fn clear_state(&mut self, cx: &mut Context<Self>) -> Option<ClearHandler> {
        self.snapshot = None;
        self.projected_copy_text = None;
        self.local_selection = false;
        cx.emit(TextSelectionEvent::Cleared);
        cx.emit(TextSelectionEvent::SelectionChanged(None));
        self.clear.clone()
    }

    fn set_auto_scroll(&self, delta: Option<Pixels>, cx: &mut Context<Self>) {
        cx.emit(TextSelectionEvent::AutoScroll(delta));
    }

    fn focus(&self, window: &mut Window, cx: &mut App) {
        if let Some(callback) = self.on_focus.clone() {
            window.defer(cx, move |window, cx| callback(window, cx));
        }
    }

    fn copy_item(&self, document_order: u64) -> Option<CopyItem> {
        (self.snapshot.is_some() || self.local_selection).then(|| CopyItem {
            document_order,
            callback: self.copy.clone(),
            fallback: self
                .projected_copy_text
                .clone()
                .unwrap_or_else(|| self.fallback_copy_text.clone()),
        })
    }
}

/// A stable, participant-neutral handle for text that participates in window selection.
#[derive(Clone)]
pub struct TextSelectionHandle(Entity<SelectableTextState>);

impl TextSelectionHandle {
    /// Creates a selection participant handle with fallback text for copying.
    pub fn new(fallback_copy_text: impl Into<String>, cx: &mut App) -> Self {
        Self(cx.new(|_| SelectableTextState::new(fallback_copy_text)))
    }

    /// Returns this participant's stable identity.
    pub fn entity_id(&self) -> EntityId {
        self.0.entity_id()
    }

    /// Returns the current geometry selection snapshot for this participant.
    pub fn snapshot(&self, cx: &App) -> Option<TextSelectionSnapshot> {
        self.0.read(cx).snapshot()
    }

    /// Sets the fallback text copied while this participant participates.
    pub fn set_fallback_copy_text(&self, text: impl Into<String>, cx: &mut App) {
        self.0
            .update(cx, |state, _| state.set_fallback_copy_text(text));
    }

    /// Marks participant-local selection, such as select-all, as active.
    pub fn set_local_selection(&self, active: bool, cx: &mut App) {
        self.0
            .update(cx, |state, _| state.set_local_selection(active));
    }

    /// Returns whether participant-local selection is active.
    pub fn has_local_selection(&self, cx: &App) -> bool {
        self.0.read(cx).local_selection
    }

    /// Registers this participant and its geometry for the current frame.
    pub fn register(
        &self,
        mut registration: TextSelectionRegistration,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(scope) = current_text_selection_scope(window.window_handle().window_id(), cx) {
            registration.scope = scope;
        }
        let Some(state) = WindowSelectionState::existing(window, cx) else {
            return;
        };
        state.update(cx, |state, cx| {
            state.register_participant(self.clone(), registration, cx)
        });
    }

    /// Projects the current snapshot onto plain-text runs and caches their copy text.
    pub fn update_runs(&self, runs: &[TextSelectionRun], cx: &mut App) -> TextSelectionProjection {
        self.0.update(cx, |state, _| state.update_runs(runs))
    }

    /// Subscribes to participant selection notifications.
    pub fn subscribe(
        &self,
        mut callback: impl FnMut(&TextSelectionEvent, &mut App) + 'static,
        cx: &mut App,
    ) -> Subscription {
        cx.subscribe(&self.0, move |_, event, cx| callback(event, cx))
    }

    /// Subscribes `window` to refresh whenever this participant's selection changes.
    #[must_use = "retain the subscription or explicitly detach it"]
    pub fn refresh_window_on_change(&self, window: &Window, cx: &mut App) -> Subscription {
        let window = window.window_handle();
        self.subscribe(
            move |event, cx| {
                if matches!(event, TextSelectionEvent::SelectionChanged(_)) {
                    _ = window.update(cx, |_, window, _| window.refresh());
                }
            },
            cx,
        )
    }

    /// Sets the callback which focuses the participant when a drag begins in it.
    pub fn focus_with(&self, callback: impl Fn(&mut Window, &mut App) + 'static, cx: &mut App) {
        self.0
            .update(cx, |state, _| state.set_focus_handler(callback));
    }

    /// Sets the synchronous participant cleanup command used by window clear.
    pub fn clear_with(&self, callback: impl Fn(&mut App) + 'static, cx: &mut App) {
        self.0.update(cx, |state, _| state.clear_with(callback));
    }

    /// Sets a participant-specific copy projection.
    pub fn copy_with(&self, callback: impl Fn(&mut App) -> String + 'static, cx: &mut App) {
        self.0.update(cx, |state, _| state.copy_with(callback));
    }

    /// Sets a participant-specific lookup for stable virtualized content keys.
    pub fn resolve_content_key_with(
        &self,
        callback: impl Fn(Point<Pixels>, &App) -> Option<TextSelectionContentKey> + 'static,
        cx: &mut App,
    ) {
        self.0
            .update(cx, |state, _| state.resolve_content_key_with(callback));
    }

    fn downgrade(&self) -> WeakEntity<SelectableTextState> {
        self.0.downgrade()
    }
}

#[derive(Clone)]
struct ParticipantRegistration {
    participant: WeakEntity<SelectableTextState>,
    registration: Rc<TextSelectionRegistration>,
    generation: u64,
}

#[derive(Clone)]
struct SelectionEndpoint {
    participant: Option<WeakEntity<SelectableTextState>>,
    point: Point<Pixels>,
    inside: bool,
    inside_text: bool,
    content_key: Option<TextSelectionContentKey>,
    content_key_resolver: Option<(ContentKeyResolver, Point<Pixels>)>,
}

impl SelectionEndpoint {
    fn snapshot(&self) -> TextSelectionEndpoint {
        let snapshot = TextSelectionEndpoint::new(self.entity_id(), self.point);
        if let Some(content_key) = self.content_key {
            snapshot.with_content_key(content_key)
        } else {
            snapshot
        }
    }

    fn resolve(
        &self,
        participants: &HashMap<EntityId, ParticipantRegistration>,
    ) -> Option<Point<Pixels>> {
        let participant = self.participant.as_ref()?;
        let registration = participants.get(&participant.entity_id())?;
        participant.upgrade()?;
        Some(
            self.point
                + registration.registration.scroll_offset
                + registration.registration.bounds.origin,
        )
    }

    fn entity_id(&self) -> Option<EntityId> {
        self.participant
            .as_ref()
            .map(|participant| participant.entity_id())
    }
}

/// Window-local generic text-selection state.
#[derive(Default)]
struct WindowSelectionState {
    participants: HashMap<EntityId, ParticipantRegistration>,
    active_scope: TextSelectionScopeId,
    anchor: Option<SelectionEndpoint>,
    cursor: Option<SelectionEndpoint>,
    pending_extension_anchor: Option<SelectionEndpoint>,
    is_selecting: bool,
    did_hit_text: bool,
    frame_generation: u64,
    finish_frame_scheduled: bool,
    mouse_down_prepared: bool,
    auto_scroll: AutoScroll,
}

impl WindowSelectionState {
    fn resolve_content_keys(state: &Entity<Self>, cx: &mut App) {
        let pending = state.update(cx, |state, _| {
            [
                state
                    .anchor
                    .as_ref()
                    .and_then(|endpoint| endpoint.content_key_resolver.clone()),
                state
                    .cursor
                    .as_ref()
                    .and_then(|endpoint| endpoint.content_key_resolver.clone()),
            ]
        });
        let resolved =
            pending.map(|pending| pending.and_then(|(callback, point)| callback(point, cx)));
        state.update(cx, |state, cx| {
            if let (Some(endpoint), Some(key)) = (state.anchor.as_mut(), resolved[0]) {
                endpoint.content_key = Some(key);
                endpoint.content_key_resolver = None;
            }
            if let (Some(endpoint), Some(key)) = (state.cursor.as_mut(), resolved[1]) {
                endpoint.content_key = Some(key);
                endpoint.content_key_resolver = None;
            }
            state.publish_snapshots(cx);
        });
    }
    fn acquire(window_id: gpui::WindowId, cx: &mut App) -> Entity<Self> {
        if !cx.has_global::<SelectionStateRegistry>() {
            cx.set_global(SelectionStateRegistry::default());
        }
        if let Some(state) = cx
            .global::<SelectionStateRegistry>()
            .0
            .get(&window_id)
            .and_then(WeakEntity::upgrade)
        {
            return state;
        }

        let active_scope = if cx.has_global::<PendingTextSelectionScopes>() {
            cx.global_mut::<PendingTextSelectionScopes>()
                .0
                .remove(&window_id)
                .unwrap_or_default()
        } else {
            TextSelectionScopeId::default()
        };

        let state = cx.new(move |cx| {
            let entity_id = cx.entity_id();
            cx.on_release(move |state: &mut WindowSelectionState, cx| {
                let handlers = state.clear_state(cx);
                if cx.has_global::<SelectionStateRegistry>() {
                    let registry = &mut cx.global_mut::<SelectionStateRegistry>().0;
                    if registry
                        .get(&window_id)
                        .is_some_and(|state| state.entity_id() == entity_id)
                    {
                        registry.remove(&window_id);
                    }
                }
                if !handlers.is_empty() {
                    cx.defer(move |cx| dispatch_clear_handlers(handlers, cx));
                }
            })
            .detach();
            Self {
                active_scope,
                ..Self::default()
            }
        });
        cx.global_mut::<SelectionStateRegistry>()
            .0
            .insert(window_id, state.downgrade());
        state
    }

    #[cfg(test)]
    fn ensure(window: &Window, cx: &mut App) -> Entity<Self> {
        Self::acquire(window.window_handle().window_id(), cx)
    }

    fn existing(window: &Window, cx: &App) -> Option<Entity<Self>> {
        if !cx.has_global::<SelectionStateRegistry>() {
            return None;
        }
        cx.global::<SelectionStateRegistry>()
            .0
            .get(&window.window_handle().window_id())
            .and_then(WeakEntity::upgrade)
    }

    /// Updates the active scope. Participants from other scopes cannot participate.
    #[cfg(test)]
    fn set_active_scope(&mut self, scope: TextSelectionScopeId, cx: &mut App) {
        let handlers = self.set_active_scope_state(scope, cx);
        dispatch_clear_handlers(handlers, cx);
    }

    fn set_active_scope_state(
        &mut self,
        scope: TextSelectionScopeId,
        cx: &mut App,
    ) -> Vec<ClearHandler> {
        if self.active_scope == scope {
            return Vec::new();
        }
        let handlers = self.clear_state(cx);
        self.active_scope = scope;
        self.publish_snapshots(cx);
        handlers
    }

    /// Sweeps participants after a rendered frame has completed.
    ///
    /// Registrations are stamped with the current generation while any sibling
    /// is painting. Sweeping only after paint makes registration independent of
    /// whether a participant or the lifecycle element paints first.
    pub fn finish_frame(&mut self, cx: &mut App) -> Vec<ClearHandler> {
        self.finish_frame_scheduled = false;
        let stale = self
            .participants
            .iter()
            .filter_map(|(id, registration)| {
                (registration.generation != self.frame_generation)
                    .then(|| (*id, registration.participant.clone()))
            })
            .collect::<Vec<_>>();
        let mut handlers = Vec::new();
        for (id, participant) in stale {
            self.participants.remove(&id);
            if let Some(participant) = participant.upgrade() {
                if let Some(handler) = participant.update(cx, |state, cx| state.clear_state(cx)) {
                    handlers.push(handler);
                }
            }
        }
        self.publish_snapshots(cx);
        self.frame_generation = self.frame_generation.wrapping_add(1);
        handlers
    }

    fn schedule_finish_frame(&mut self) -> bool {
        if self.finish_frame_scheduled {
            return false;
        }
        self.finish_frame_scheduled = true;
        true
    }

    /// Registers this frame's geometry for a participant.
    pub fn register_participant(
        &mut self,
        selection: TextSelectionHandle,
        registration: TextSelectionRegistration,
        cx: &mut App,
    ) {
        self.prune_dead_participants();
        self.participants.insert(
            selection.entity_id(),
            ParticipantRegistration {
                participant: selection.downgrade(),
                registration: Rc::new(registration),
                generation: self.frame_generation,
            },
        );
        self.publish_snapshots(cx);
    }

    /// Starts a selection gesture using bounds hit testing (useful to adapters/tests).
    #[cfg(test)]
    fn begin(&mut self, position: Point<Pixels>, extend: bool, cx: &mut App) {
        self.begin_impl(position, extend, false, None, cx);
    }

    /// Updates the current gesture using bounds hit testing.
    #[cfg(test)]
    fn update(&mut self, position: Point<Pixels>, cx: &mut App) {
        self.update_impl(position, None, cx);
    }

    /// Ends the current gesture and keeps its selection visible.
    pub fn end(&mut self, cx: &mut App) {
        self.pending_extension_anchor = None;
        if !self.is_selecting {
            return;
        }
        self.is_selecting = false;
        if !self.did_hit_text {
            self.anchor = None;
            self.cursor = None;
        }
        self.stop_anchor_auto_scroll(cx);
        self.publish_snapshots(cx);
    }

    /// Clears both window selection and every participant's local selection.
    pub fn clear(&mut self, cx: &mut App) {
        let handlers = self.clear_state(cx);
        dispatch_clear_handlers(handlers, cx);
    }

    fn clear_state(&mut self, cx: &mut App) -> Vec<ClearHandler> {
        self.stop_anchor_auto_scroll(cx);
        self.anchor = None;
        self.cursor = None;
        self.pending_extension_anchor = None;
        self.is_selecting = false;
        self.did_hit_text = false;
        self.prune_dead_participants();
        self.participants
            .values()
            .filter_map(|registration| registration.participant.upgrade())
            .filter_map(|participant| participant.update(cx, |state, cx| state.clear_state(cx)))
            .collect()
    }

    fn copy_items(&self, cx: &App) -> Vec<CopyItem> {
        self.participants
            .values()
            .filter_map(|registration| {
                let participant = registration.participant.upgrade()?;
                participant
                    .read(cx)
                    .copy_item(registration.registration.document_order)
            })
            .collect()
    }

    #[cfg(test)]
    fn selected_text(&self, cx: &mut App) -> String {
        resolve_copy_items(self.copy_items(cx), cx)
    }

    /// Returns whether a drag or a participant-local selection is active.
    pub fn has_selection(&self, cx: &App) -> bool {
        self.snapshot().is_some()
            || self.participants.values().any(|registration| {
                registration
                    .participant
                    .upgrade()
                    .is_some_and(|participant| participant.read(cx).local_selection)
            })
    }

    /// Returns the current resolved selection endpoints.
    pub fn snapshot(&self) -> Option<TextSelectionSnapshot> {
        if !self.did_hit_text {
            return None;
        }
        let anchor_endpoint = self.anchor.as_ref()?;
        let cursor_endpoint = self.cursor.as_ref()?;
        let anchor = anchor_endpoint.resolve(&self.participants)?;
        let cursor = cursor_endpoint.resolve(&self.participants)?;
        (anchor != cursor).then(|| {
            TextSelectionSnapshot::new(anchor_endpoint.snapshot(), cursor_endpoint.snapshot())
                .with_selecting(self.is_selecting)
                .with_window_points(Some(TextSelectionWindowPoints { anchor, cursor }))
        })
    }

    /// Returns whether a drag is currently in progress.
    #[cfg(test)]
    fn is_selecting(&self) -> bool {
        self.is_selecting
    }

    fn prepare_for_mouse_down(&mut self, extend: bool, cx: &mut App) -> Vec<ClearHandler> {
        let pending_extension_anchor = extend.then(|| self.anchor.clone()).flatten();
        self.stop_anchor_auto_scroll(cx);
        self.anchor = None;
        self.cursor = None;
        self.pending_extension_anchor = None;
        self.is_selecting = false;
        self.did_hit_text = false;
        self.prune_dead_participants();
        let handlers = self
            .participants
            .values()
            .filter_map(|registration| registration.participant.upgrade())
            .filter_map(|participant| participant.update(cx, |state, cx| state.clear_state(cx)))
            .collect();
        self.pending_extension_anchor = pending_extension_anchor;
        handlers
    }

    fn begin_in_window(
        &mut self,
        position: Point<Pixels>,
        extend: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.begin_impl(position, extend, true, Some(window), cx);
    }

    fn update_in_window(
        &mut self,
        position: Point<Pixels>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if !cx.has_active_drag() {
            self.update_impl(position, Some(window), cx);
            self.update_auto_scroll(position, Some(window), cx);
        }
    }

    fn select_at(
        &mut self,
        position: Point<Pixels>,
        click_count: usize,
        window: &mut Window,
        cx: &mut App,
    ) {
        GlobalState::init(cx);
        if GlobalState::is_text_selection_suppressed(cx) {
            return;
        }
        let hit = self.endpoint(position, Some(window), cx);
        if !hit.inside_text {
            return;
        }
        let Some(participant) = hit
            .participant
            .and_then(|participant| participant.upgrade())
        else {
            return;
        };
        let points = points_for_multi_click(&participant.read(cx).runs, position, click_count);
        let Some((anchor, cursor)) = points else {
            return;
        };
        let Some(registration) = self.participants.get(&participant.entity_id()) else {
            return;
        };
        let content_key_resolver = participant.read(cx).content_key_resolver.clone();
        let to_endpoint = |point: Point<Pixels>| {
            let content_point = point
                - registration.registration.bounds.origin
                - registration.registration.scroll_offset;
            SelectionEndpoint {
                participant: Some(participant.downgrade()),
                point: content_point,
                inside: true,
                inside_text: true,
                content_key: None,
                content_key_resolver: content_key_resolver
                    .clone()
                    .map(|resolver| (resolver, content_point)),
            }
        };
        self.anchor = Some(to_endpoint(anchor));
        self.cursor = Some(to_endpoint(cursor));
        self.did_hit_text = true;
        self.is_selecting = false;
        participant.update(cx, |state, cx| state.focus(window, cx));
        self.publish_snapshots(cx);
    }

    #[cfg(test)]
    fn update_in_window_with_active_drag(
        &mut self,
        position: Point<Pixels>,
        active_drag: bool,
        window: &Window,
        cx: &mut App,
    ) {
        if !active_drag {
            self.update_impl(position, Some(window), cx);
        }
    }

    fn begin_impl(
        &mut self,
        position: Point<Pixels>,
        extend: bool,
        already_prepared: bool,
        window: Option<&mut Window>,
        cx: &mut App,
    ) {
        GlobalState::init(cx);
        if GlobalState::is_text_selection_suppressed(cx) {
            self.pending_extension_anchor = None;
            return;
        }
        let previous_anchor = extend
            .then(|| {
                self.pending_extension_anchor
                    .take()
                    .or_else(|| self.anchor.clone())
            })
            .flatten()
            .filter(|anchor| anchor.resolve(&self.participants).is_some());
        if !extend && !already_prepared {
            self.clear(cx);
        }
        let endpoint = self.endpoint(position, window.as_deref(), cx);
        let focus_participant = endpoint
            .inside
            .then(|| endpoint.participant.clone())
            .flatten();
        let anchor = previous_anchor.unwrap_or_else(|| endpoint.clone());
        self.anchor = Some(anchor.clone());
        self.cursor = Some(endpoint.clone());
        self.did_hit_text = anchor.inside_text || endpoint.inside_text;
        self.is_selecting = true;
        if let Some(participant) = focus_participant.and_then(|participant| participant.upgrade()) {
            if let Some(window) = window {
                participant.update(cx, |state, cx| state.focus(window, cx));
            }
        }
        self.publish_snapshots(cx);
    }

    fn update_impl(&mut self, position: Point<Pixels>, window: Option<&Window>, cx: &mut App) {
        if !self.is_selecting {
            return;
        }
        let endpoint = self.endpoint(position, window, cx);
        self.did_hit_text |= endpoint.inside_text;
        self.cursor = Some(endpoint);
        if window.is_none() {
            self.update_participant_auto_scroll(position, cx);
        }
        self.publish_snapshots(cx);
    }

    fn endpoint(
        &mut self,
        position: Point<Pixels>,
        window: Option<&Window>,
        cx: &App,
    ) -> SelectionEndpoint {
        self.prune_dead_participants();
        let mut hit: Option<(
            WeakEntity<SelectableTextState>,
            Rc<TextSelectionRegistration>,
            f32,
        )> = None;
        let mut predecessor: Option<(
            WeakEntity<SelectableTextState>,
            Rc<TextSelectionRegistration>,
        )> = None;
        let mut first: Option<(
            WeakEntity<SelectableTextState>,
            Rc<TextSelectionRegistration>,
        )> = None;

        for registration in self.participants.values() {
            if registration.registration.scope != self.active_scope
                || registration.participant.upgrade().is_none()
            {
                continue;
            }
            let participant_geometry = &registration.registration;
            let hovered = window.map_or_else(
                || participant_geometry.bounds.contains(&position),
                |window| participant_geometry.hitbox.is_hovered(window),
            );
            if hovered {
                let area = f32::from(participant_geometry.bounds.size.width)
                    * f32::from(participant_geometry.bounds.size.height);
                if hit.as_ref().is_none_or(|(_, best, best_area)| {
                    area < *best_area
                        || (area == *best_area
                            && participant_geometry.document_order < best.document_order)
                }) {
                    hit = Some((
                        registration.participant.clone(),
                        participant_geometry.clone(),
                        area,
                    ));
                }
            }
            if participant_geometry.bounds.top() <= position.y
                && predecessor.as_ref().is_none_or(|(_, best)| {
                    participant_geometry.bounds.top() > best.bounds.top()
                        || (participant_geometry.bounds.top() == best.bounds.top()
                            && participant_geometry.document_order < best.document_order)
                })
            {
                predecessor = Some((
                    registration.participant.clone(),
                    participant_geometry.clone(),
                ));
            }
            if first.as_ref().is_none_or(|(_, best)| {
                participant_geometry.bounds.top() < best.bounds.top()
                    || (participant_geometry.bounds.top() == best.bounds.top()
                        && participant_geometry.document_order < best.document_order)
            }) {
                first = Some((
                    registration.participant.clone(),
                    participant_geometry.clone(),
                ));
            }
        }

        let selection = hit
            .map(|(participant, registration, _)| (participant, registration, true))
            .or_else(|| {
                predecessor
                    .or(first)
                    .map(|(participant, registration)| (participant, registration, false))
            });
        match selection {
            Some((participant, registration, inside)) => {
                let point = position - registration.bounds.origin - registration.scroll_offset;
                let content_key_resolver = participant.upgrade().and_then(|participant| {
                    participant
                        .read(cx)
                        .content_key_resolver
                        .clone()
                        .map(|callback| (callback, point))
                });
                SelectionEndpoint {
                    point,
                    participant: Some(participant),
                    inside,
                    inside_text: inside
                        && registration
                            .text_bounds
                            .iter()
                            .any(|bounds| bounds.contains(&position)),
                    content_key: None,
                    content_key_resolver,
                }
            }
            None => SelectionEndpoint {
                participant: None,
                point: position,
                inside: false,
                inside_text: false,
                content_key: None,
                content_key_resolver: None,
            },
        }
    }

    fn publish_snapshots(&mut self, cx: &mut App) {
        self.prune_dead_participants();
        let snapshot = self.snapshot();
        let single_participant = self.single_participant();
        for (id, registration) in &self.participants {
            let Some(participant) = registration.participant.upgrade() else {
                continue;
            };
            let participant_snapshot = (registration.registration.scope == self.active_scope
                && self.participates(*id, registration)
                && single_participant.is_none_or(|single| single == *id))
            .then_some(snapshot)
            .flatten()
            .map(|mut snapshot| {
                snapshot.coverage = self.coverage_for(*id);
                snapshot
            });
            participant.update(cx, |state, cx| state.set_snapshot(participant_snapshot, cx));
        }
    }

    fn coverage_for(&self, id: EntityId) -> TextSelectionCoverage {
        let Some(anchor) = self.anchor.as_ref().and_then(SelectionEndpoint::entity_id) else {
            return TextSelectionCoverage::Bounded;
        };
        let Some(cursor) = self.cursor.as_ref().and_then(SelectionEndpoint::entity_id) else {
            return TextSelectionCoverage::Bounded;
        };
        if anchor == cursor {
            return TextSelectionCoverage::Bounded;
        }
        let anchor_order = self.participants[&anchor].registration.document_order;
        let cursor_order = self.participants[&cursor].registration.document_order;
        if id != anchor && id != cursor {
            TextSelectionCoverage::Full
        } else if (id == anchor) == (anchor_order < cursor_order) {
            TextSelectionCoverage::ToEnd
        } else {
            TextSelectionCoverage::FromStart
        }
    }

    fn single_participant(&self) -> Option<EntityId> {
        let anchor = self.anchor.as_ref()?.entity_id()?;
        let cursor = self.cursor.as_ref()?.entity_id()?;
        (anchor == cursor).then_some(anchor)
    }

    fn participates(&self, id: EntityId, registration: &ParticipantRegistration) -> bool {
        let Some(anchor) = self.anchor.as_ref().and_then(SelectionEndpoint::entity_id) else {
            return false;
        };
        let Some(cursor) = self.cursor.as_ref().and_then(SelectionEndpoint::entity_id) else {
            return false;
        };
        let Some(anchor_registration) = self.participants.get(&anchor) else {
            return false;
        };
        let Some(cursor_registration) = self.participants.get(&cursor) else {
            return false;
        };
        let start = anchor_registration
            .registration
            .document_order
            .min(cursor_registration.registration.document_order);
        let end = anchor_registration
            .registration
            .document_order
            .max(cursor_registration.registration.document_order);
        (start..=end).contains(&registration.registration.document_order)
            || id == anchor
            || id == cursor
    }

    fn update_auto_scroll(
        &mut self,
        position: Point<Pixels>,
        window: Option<&Window>,
        cx: &mut Context<Self>,
    ) {
        // A finished gesture keeps its anchor for shift-click extension; only
        // a live drag may scroll.
        if !self.is_selecting {
            return;
        }
        let Some(anchor) = self.anchor.as_ref().filter(|anchor| anchor.inside) else {
            return;
        };
        let Some(participant) = anchor.participant.as_ref().and_then(WeakEntity::upgrade) else {
            return;
        };
        let Some(registration) = self.participants.get(&participant.entity_id()) else {
            return;
        };
        // The content mask is the nearest clipping viewport established by a
        // scrollable ancestor. It remains stable as the participant itself
        // moves, so selection keeps scrolling the same related region even
        // after the anchor text has moved out of view.
        let visible_bounds = registration.registration.hitbox.content_mask.bounds;
        // Keeps the synthesized wheel event hit-testing inside the mask.
        const HIT_TEST_INSET: Pixels = px(1.);
        // A collapsed mask leaves an empty clamp range below — stop.
        if visible_bounds.size.width < HIT_TEST_INSET * 2.
            || visible_bounds.size.height < HIT_TEST_INSET * 2.
        {
            self.stop_anchor_auto_scroll(cx);
            return;
        }
        let delta = AutoScroll::compute_delta(position.y, visible_bounds);
        let Some(window) = window else {
            participant.update(cx, |state, cx| state.set_auto_scroll(delta, cx));
            return;
        };

        let event_position = point(
            position.x.clamp(
                visible_bounds.left() + HIT_TEST_INSET,
                visible_bounds.right() - HIT_TEST_INSET,
            ),
            position.y.clamp(
                visible_bounds.top() + HIT_TEST_INSET,
                visible_bounds.bottom() - HIT_TEST_INSET,
            ),
        );
        self.auto_scroll.last_drag_position = Some(event_position);
        let window = window.window_handle();
        self.auto_scroll.set(delta, cx, move |delta, state, cx| {
            let Some(position) = state.auto_scroll.last_drag_position else {
                return;
            };
            let window = window;
            cx.defer(move |cx| {
                _ = window.update(cx, |_, window, cx| {
                    window.dispatch_event(
                        ScrollWheelEvent {
                            position,
                            delta: ScrollDelta::Pixels(point(px(0.), -delta)),
                            ..Default::default()
                        }
                        .to_platform_input(),
                        cx,
                    );
                });
            });
        });
    }

    fn update_participant_auto_scroll(&self, position: Point<Pixels>, cx: &mut App) {
        let Some(anchor) = self.anchor.as_ref().filter(|anchor| anchor.inside) else {
            return;
        };
        let Some(participant) = anchor.participant.as_ref().and_then(WeakEntity::upgrade) else {
            return;
        };
        let Some(registration) = self.participants.get(&participant.entity_id()) else {
            return;
        };
        let delta = AutoScroll::compute_delta(position.y, registration.registration.bounds);
        participant.update(cx, |state, cx| state.set_auto_scroll(delta, cx));
    }

    fn stop_anchor_auto_scroll(&mut self, cx: &mut App) {
        self.auto_scroll.stop();
        let Some(participant) = self
            .anchor
            .as_ref()
            .filter(|anchor| anchor.inside)
            .and_then(|anchor| anchor.participant.as_ref())
            .and_then(WeakEntity::upgrade)
        else {
            return;
        };
        participant.update(cx, |state, cx| state.set_auto_scroll(None, cx));
    }

    fn prune_dead_participants(&mut self) {
        self.participants
            .retain(|_, registration| registration.participant.upgrade().is_some());
    }
}

#[derive(Default)]
/// Non-owning window locator; retained [`TextSelection`] element state owns
/// each live selection entity.
struct SelectionStateRegistry(HashMap<gpui::WindowId, WeakEntity<WindowSelectionState>>);

impl Global for SelectionStateRegistry {}

#[derive(Default)]
struct PendingTextSelectionScopes(HashMap<gpui::WindowId, TextSelectionScopeId>);

impl Global for PendingTextSelectionScopes {}

#[derive(Default)]
struct TextSelectionScopeStacks(HashMap<gpui::WindowId, Vec<TextSelectionScopeId>>);

impl Global for TextSelectionScopeStacks {}

fn push_text_selection_scope(window_id: gpui::WindowId, scope: TextSelectionScopeId, cx: &mut App) {
    if !cx.has_global::<TextSelectionScopeStacks>() {
        cx.set_global(TextSelectionScopeStacks::default());
    }
    cx.global_mut::<TextSelectionScopeStacks>()
        .0
        .entry(window_id)
        .or_default()
        .push(scope);
}

fn pop_text_selection_scope(window_id: gpui::WindowId, cx: &mut App) {
    let stacks = &mut cx.global_mut::<TextSelectionScopeStacks>().0;
    let remove_stack = stacks.get_mut(&window_id).is_some_and(|stack| {
        stack.pop();
        stack.is_empty()
    });
    if remove_stack {
        stacks.remove(&window_id);
    }
}

fn current_text_selection_scope(
    window_id: gpui::WindowId,
    cx: &App,
) -> Option<TextSelectionScopeId> {
    cx.has_global::<TextSelectionScopeStacks>()
        .then(|| {
            cx.global::<TextSelectionScopeStacks>()
                .0
                .get(&window_id)
                .and_then(|stack| stack.last().copied())
        })
        .flatten()
}

fn with_text_selection_scope<T>(
    window_id: gpui::WindowId,
    scope: TextSelectionScopeId,
    cx: &mut App,
    callback: impl FnOnce(&mut App) -> T,
) -> T {
    push_text_selection_scope(window_id, scope, cx);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(cx)));
    pop_text_selection_scope(window_id, cx);
    match result {
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Window-level operations for text selection.
pub struct TextSelection;

impl TextSelection {
    /// Returns the currently selected text in logical document order.
    pub fn selected_text(window: &mut Window, cx: &mut App) -> String {
        let Some(state) = live_text_selection_state(window, cx) else {
            return String::new();
        };
        let items = state.read(cx).copy_items(cx);
        resolve_copy_items(items, cx)
    }

    /// Returns whether the window has a geometry selection or any participant
    /// has an active participant-local selection such as select-all.
    pub fn has_selection(window: &mut Window, cx: &mut App) -> bool {
        live_text_selection_state(window, cx).is_some_and(|state| state.read(cx).has_selection(cx))
    }

    /// Clears window selection and every participant's local selection.
    pub fn clear(window: &mut Window, cx: &mut App) {
        if let Some(state) = live_text_selection_state(window, cx) {
            let handlers = state.update(cx, |state, cx| state.clear_state(cx));
            dispatch_clear_handlers(handlers, cx);
        }
    }

    /// Clears selection for a known window identifier.
    ///
    /// Prefer [`Self::clear`] when a window reference is available. This
    /// narrow entry point supports hosts retiring deprecated window wrappers.
    pub fn clear_for_window(window_id: gpui::WindowId, cx: &mut App) {
        clear_window_text_selection(window_id, cx);
    }

    /// Ends the current drag while leaving its selection visible.
    pub fn end(window: &mut Window, cx: &mut App) {
        if let Some(state) = live_text_selection_state(window, cx) {
            state.update(cx, |state, cx| state.end(cx));
        }
    }

    /// Activates the opaque selection scope for this window.
    pub fn activate_scope(scope: TextSelectionScopeId, window: &mut Window, cx: &mut App) {
        let Some(state) = WindowSelectionState::existing(window, cx) else {
            if !cx.has_global::<PendingTextSelectionScopes>() {
                cx.set_global(PendingTextSelectionScopes::default());
            }
            cx.global_mut::<PendingTextSelectionScopes>()
                .0
                .insert(window.window_handle().window_id(), scope);
            return;
        };
        let handlers = state.update(cx, |state, cx| state.set_active_scope_state(scope, cx));
        dispatch_clear_handlers(handlers, cx);
    }
}

/// A zero-sized root layer which enables text selection for a window.
///
/// Mount one as the root's first child. Its stable `"window-text-selection"`
/// element identity retains the window-local selection entity across frames.
pub struct TextSelectionLayer;

pub(crate) fn text_selection_scope(
    scope: TextSelectionScopeId,
    element: impl IntoElement,
) -> impl IntoElement {
    TextSelectionScopeMarker {
        scope,
        element: element.into_element(),
    }
}

struct TextSelectionScopeMarker<E> {
    scope: TextSelectionScopeId,
    element: E,
}

impl<E: Element> IntoElement for TextSelectionScopeMarker<E> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<E: Element> Element for TextSelectionScopeMarker<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.element.id()
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.element.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let window_id = window.window_handle().window_id();
        with_text_selection_scope(window_id, self.scope, cx, |cx| {
            self.element.request_layout(id, inspector_id, window, cx)
        })
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let window_id = window.window_handle().window_id();
        with_text_selection_scope(window_id, self.scope, cx, |cx| {
            self.element
                .prepaint(id, inspector_id, bounds, request_layout, window, cx)
        })
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let window_id = window.window_handle().window_id();
        with_text_selection_scope(window_id, self.scope, cx, |cx| {
            self.element.paint(
                id,
                inspector_id,
                bounds,
                request_layout,
                prepaint,
                window,
                cx,
            );
        });
    }
}

#[doc(hidden)]
pub struct TextSelectionLayerPrepaintState(Entity<WindowSelectionState>);

impl IntoElement for TextSelectionLayer {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextSelectionLayer {
    type RequestLayoutState = ();
    type PrepaintState = TextSelectionLayerPrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some("window-text-selection".into())
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
    ) -> (LayoutId, Self::RequestLayoutState) {
        (window.request_layout(Style::default(), [], cx), ())
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        // Automatic participant order is paint order within this frame. Keep
        // this lifecycle in base so base-only applications do not need a
        // separate root component to reset it. Otherwise, registering the
        // first of two selected TextViews temporarily reverses their order
        // against the previous frame and alternates coverage forever.
        GlobalState::init(cx);
        GlobalState::global_mut(cx).begin_selection_frame();
        TextSelectionLayerPrepaintState(retain_text_selection_state(global_id, window, cx))
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        state: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        paint_text_selection(&state.0, window, cx);
    }
}

fn retain_text_selection_state(
    global_id: Option<&GlobalElementId>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<WindowSelectionState> {
    let window_id = window.window_handle().window_id();
    let state = window.with_element_state::<Entity<WindowSelectionState>, _>(
        global_id.expect("TextSelection has a stable element id"),
        |retained, _| {
            let state = retained.unwrap_or_else(|| WindowSelectionState::acquire(window_id, cx));
            (state.clone(), state)
        },
    );
    if !cx.has_global::<SelectionStateRegistry>() {
        cx.set_global(SelectionStateRegistry::default());
    }
    cx.global_mut::<SelectionStateRegistry>()
        .0
        .insert(window_id, state.downgrade());
    state
}

fn paint_text_selection(state: &Entity<WindowSelectionState>, window: &mut Window, cx: &mut App) {
    if state.update(cx, |state, _| state.schedule_finish_frame()) {
        let state = state.downgrade();
        window.defer(cx, move |_, cx| {
            let Some(state) = state.upgrade() else {
                return;
            };
            let handlers = state.update(cx, |state, cx| state.finish_frame(cx));
            dispatch_clear_handlers(handlers, cx);
        });
    }

    let mouse_down_state = state.downgrade();
    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
        if event.button != MouseButton::Left {
            return;
        }
        let Some(state) = mouse_down_state.upgrade() else {
            return;
        };
        if phase.capture() {
            GlobalState::init(cx);
            GlobalState::reset_text_selection_suppression(cx);
            let handlers = state.update(cx, |state, cx| {
                if state.mouse_down_prepared {
                    return Vec::new();
                }
                state.mouse_down_prepared = true;
                state.prepare_for_mouse_down(event.click_count == 1 && event.modifiers.shift, cx)
            });
            dispatch_clear_handlers(handlers, cx);
        } else if event.click_count == 1 {
            if GlobalState::is_text_selection_suppressed(cx) {
                state.update(cx, |state, _| state.pending_extension_anchor = None);
                return;
            }
            state.update(cx, |state, cx| {
                if !state.is_selecting {
                    state.begin_in_window(event.position, event.modifiers.shift, window, cx)
                }
            });
            WindowSelectionState::resolve_content_keys(&state, cx);
        } else if event.click_count >= 2 {
            if GlobalState::is_text_selection_suppressed(cx) {
                return;
            }
            state.update(cx, |state, cx| {
                state.select_at(event.position, event.click_count, window, cx)
            });
            WindowSelectionState::resolve_content_keys(&state, cx);
        }
    });

    let mouse_move_state = state.downgrade();
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
        if phase.bubble()
            && let Some(state) = mouse_move_state.upgrade()
        {
            state.update(cx, |state, cx| {
                state.update_in_window(event.position, window, cx)
            });
            WindowSelectionState::resolve_content_keys(&state, cx);
        }
    });

    let mouse_up_state = state.downgrade();
    window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
        if phase.bubble()
            && let Some(state) = mouse_up_state.upgrade()
        {
            state.update(cx, |state, cx| {
                state.mouse_down_prepared = false;
                state.end(cx)
            });
        }
    });

    let scroll_state = state.downgrade();
    window.on_mouse_event(move |_: &ScrollWheelEvent, phase, window, cx| {
        if phase.bubble()
            && let Some(state) = scroll_state.upgrade()
        {
            let position = window.mouse_position();
            state.update(cx, |state, cx| state.update_in_window(position, window, cx));
            WindowSelectionState::resolve_content_keys(&state, cx);
        }
    });
}

fn live_text_selection_state(
    window: &Window,
    cx: &mut App,
) -> Option<Entity<WindowSelectionState>> {
    WindowSelectionState::existing(window, cx)
}

pub(crate) fn clear_window_text_selection(window_id: gpui::WindowId, cx: &mut App) {
    if !cx.has_global::<SelectionStateRegistry>() {
        return;
    }
    let Some(state) = cx
        .global::<SelectionStateRegistry>()
        .0
        .get(&window_id)
        .and_then(WeakEntity::upgrade)
    else {
        return;
    };
    let handlers = state.update(cx, |state, cx| state.clear_state(cx));
    dispatch_clear_handlers(handlers, cx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ElementExt as _;
    use gpui::{
        Bounds, ContentMask, Context, Hitbox, HitboxBehavior, HitboxId, InteractiveElement as _,
        IntoElement, ParentElement as _, Render, SharedString, Styled as _, StyledText,
        TestAppContext, TextLayout, Window, div, point, prelude::FluentBuilder as _, px, size,
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    struct FakeParticipant {
        selection: TextSelectionHandle,
    }

    struct WindowSelectionView {
        selection: TextSelectionHandle,
    }

    struct SelectionElementOnlyView;
    struct ToggleSelectionElementView {
        enabled: bool,
        selection: TextSelectionHandle,
    }

    struct DoubleSelectionElementView {
        selection: TextSelectionHandle,
    }

    struct WindowOwnedSelectionView {
        selection: TextSelectionHandle,
    }

    struct FirstFrameScopedSelectionView {
        selection: TextSelectionHandle,
    }

    struct PlainRunLayoutView {
        texts: Vec<SharedString>,
        layouts: Vec<TextLayout>,
    }

    impl Render for WindowSelectionView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    impl Render for SelectionElementOnlyView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(TextSelectionLayer)
                .child(
                    div()
                        .size_full()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| {
                            GlobalState::suppress_text_selection(cx);
                        }),
                )
        }
    }

    impl Render for ToggleSelectionElementView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let selection = self.selection.clone();
            div().when(self.enabled, |this| {
                this.child(TextSelectionLayer)
                    .child(div().size_full().on_prepaint(move |bounds, window, cx| {
                        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
                        selection.register(
                            TextSelectionRegistration::new(hitbox, bounds)
                                .with_text_bounds(vec![bounds]),
                            window,
                            cx,
                        );
                    }))
            })
        }
    }

    impl Render for DoubleSelectionElementView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let selection = self.selection.clone();
            div()
                .size_full()
                .child(TextSelectionLayer)
                .child(TextSelectionLayer)
                .on_prepaint(move |bounds, window, cx| {
                    let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
                    selection.register(
                        TextSelectionRegistration::new(hitbox, bounds)
                            .with_text_bounds(vec![bounds]),
                        window,
                        cx,
                    );
                })
        }
    }

    impl Render for WindowOwnedSelectionView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let selection = self.selection.clone();
            div()
                .size_full()
                .child(TextSelectionLayer)
                .child(div().size_full().on_prepaint(move |bounds, window, cx| {
                    let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
                    selection.register(
                        TextSelectionRegistration::new(hitbox, bounds)
                            .with_text_bounds(vec![bounds]),
                        window,
                        cx,
                    );
                }))
        }
    }

    impl Render for FirstFrameScopedSelectionView {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let scope = TextSelectionScopeId::from_raw(23);
            TextSelection::activate_scope(scope, window, cx);
            let selection = self.selection.clone();

            div().child(TextSelectionLayer).child(
                div()
                    .size_full()
                    .on_prepaint(move |bounds, window, cx| {
                        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
                        selection.register(
                            TextSelectionRegistration::new(hitbox, bounds)
                                .with_text_bounds(vec![bounds]),
                            window,
                            cx,
                        );
                    })
                    .text_selection_scope(scope),
            )
        }
    }

    impl Render for PlainRunLayoutView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.layouts.clear();
            let children = self
                .texts
                .iter()
                .enumerate()
                .map(|(index, text)| {
                    let text = StyledText::new(text.clone());
                    self.layouts.push(text.layout().clone());
                    div().absolute().top(px(index as f32 * 40.)).child(text)
                })
                .collect::<Vec<_>>();
            div().size_full().children(children)
        }
    }

    impl FakeParticipant {
        fn new(text: &str, cx: &mut gpui::App) -> Self {
            let selection = TextSelectionHandle::new(text, cx);
            Self { selection }
        }

        fn register(
            &self,
            selection_state: &mut WindowSelectionState,
            y: f32,
            scope: TextSelectionScopeId,
            document_order: u64,
            cx: &mut gpui::App,
        ) {
            let bounds = Bounds::new(point(px(0.), px(y)), size(px(100.), px(10.)));
            selection_state.register_participant(
                self.selection.clone(),
                TextSelectionRegistration::new(
                    Hitbox {
                        id: HitboxId::placeholder(),
                        bounds,
                        content_mask: ContentMask { bounds },
                        behavior: HitboxBehavior::Normal,
                    },
                    bounds,
                )
                .with_scope(scope)
                .with_document_order(document_order)
                .with_text_bounds(vec![bounds]),
                cx,
            );
        }
    }

    fn laid_out_runs(texts: &[&str], cx: &mut TestAppContext) -> Vec<(SharedString, TextLayout)> {
        let texts = texts
            .iter()
            .map(|text| SharedString::from(*text))
            .collect::<Vec<_>>();
        let view = cx.add_window({
            let texts = texts.clone();
            move |_, _| PlainRunLayoutView {
                texts,
                layouts: Vec::new(),
            }
        });
        cx.update_window(*view, |_, window, cx| {
            let _ = window.draw(cx);
        })
        .unwrap();
        let layouts = cx.update(|cx| view.read(cx).unwrap().layouts.clone());
        texts.into_iter().zip(layouts).collect()
    }

    fn plain_snapshot(anchor: Point<Pixels>, cursor: Point<Pixels>) -> TextSelectionSnapshot {
        TextSelectionSnapshot::new(
            TextSelectionEndpoint::new(None, anchor),
            TextSelectionEndpoint::new(None, cursor),
        )
        .with_window_points(Some(TextSelectionWindowPoints { anchor, cursor }))
    }

    #[gpui::test]
    fn scope_stack_is_cleaned_after_panicking_subtree(cx: &mut TestAppContext) {
        let window_id = {
            let (_, window_cx) = cx.add_window_view(|_, _| SelectionElementOnlyView);
            window_cx.update(|window, _| window.window_handle().window_id())
        };
        let scope = TextSelectionScopeId::from_raw(41);

        cx.update(|cx| {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                with_text_selection_scope(window_id, scope, cx, |_| panic!("subtree failed"));
            }));

            assert!(result.is_err());
            assert_eq!(current_text_selection_scope(window_id, cx), None);
        });
    }

    #[gpui::test]
    fn reentrant_scope_from_one_window_does_not_pollute_another(cx: &mut TestAppContext) {
        let first_window_id = {
            let (_, window_cx) = cx.add_window_view(|_, _| SelectionElementOnlyView);
            window_cx.update(|window, _| window.window_handle().window_id())
        };
        let second_window_id = {
            let (_, window_cx) = cx.add_window_view(|_, _| SelectionElementOnlyView);
            window_cx.update(|window, _| window.window_handle().window_id())
        };
        let scope = TextSelectionScopeId::from_raw(42);

        cx.update(|cx| {
            with_text_selection_scope(first_window_id, scope, cx, |cx| {
                assert_eq!(current_text_selection_scope(second_window_id, cx), None);
                assert_eq!(
                    current_text_selection_scope(first_window_id, cx),
                    Some(scope)
                );
            });
        });
    }

    #[gpui::test]
    fn selection_callback_can_reenter_its_selection_state(cx: &mut TestAppContext) {
        let called = Rc::new(Cell::new(false));
        let called_from_callback = called.clone();
        let (selection_state, participant) = cx.update(|cx| {
            let selection_state = cx.new(|_| WindowSelectionState::default());
            let selection_state_for_callback = selection_state.clone();
            let participant = FakeParticipant::new("participant", cx);
            participant
                .selection
                .subscribe(
                    move |event, cx| {
                        if matches!(event, TextSelectionEvent::SelectionChanged(Some(_))) {
                            selection_state_for_callback
                                .update(cx, |_, _| called_from_callback.set(true));
                        }
                    },
                    cx,
                )
                .detach();
            (selection_state, participant)
        });
        cx.run_until_parked();
        cx.update(|cx| {
            selection_state.update(cx, |selection_state, cx| {
                participant.register(selection_state, 0., TextSelectionScopeId::default(), 0, cx);
                selection_state.begin(point(px(1.), px(1.)), false, cx);
                selection_state.update(point(px(20.), px(1.)), cx);
            });
        });
        cx.run_until_parked();
        assert!(called.get());
    }

    #[gpui::test]
    fn selection_events_preserve_snapshot_then_clear_order(cx: &mut TestAppContext) {
        let observed = Rc::new(RefCell::new(Vec::new()));
        let observed_for_callback = observed.clone();
        let selection = cx.update(|cx| {
            let selection = TextSelectionHandle::new("selection", cx);
            selection
                .subscribe(
                    move |event, _| {
                        if let TextSelectionEvent::SelectionChanged(snapshot) = event {
                            observed_for_callback.borrow_mut().push(snapshot.is_some());
                        }
                    },
                    cx,
                )
                .detach();
            selection
        });
        cx.run_until_parked();
        cx.update(|cx| {
            selection.0.update(cx, |state, cx| {
                state.set_snapshot(
                    Some(plain_snapshot(point(px(1.), px(1.)), point(px(8.), px(1.)))),
                    cx,
                );
                state.clear_state(cx);
            });
        });
        cx.run_until_parked();
        assert_eq!(&*observed.borrow(), &[true, false]);
    }

    fn text_run(order: u64, text: SharedString, layout: TextLayout) -> TextSelectionRun {
        let bounds = layout.bounds();
        TextSelectionRun::new(text, layout, bounds).with_document_order(order)
    }

    #[gpui::test]
    fn public_selection_data_uses_builders_and_readers(cx: &mut TestAppContext) {
        let bounds = Bounds::new(point(px(1.), px(2.)), size(px(30.), px(10.)));
        let hitbox = Hitbox {
            id: HitboxId::placeholder(),
            bounds,
            content_mask: ContentMask { bounds },
            behavior: HitboxBehavior::Normal,
        };
        let scope = TextSelectionScopeId::from_raw(7);
        let endpoint = TextSelectionEndpoint::new(None, bounds.origin)
            .with_content_key(TextSelectionContentKey::new(11));
        let snapshot = TextSelectionSnapshot::new(endpoint, endpoint)
            .with_selecting(true)
            .with_window_points(Some(TextSelectionWindowPoints {
                anchor: bounds.origin,
                cursor: bounds.bottom_right(),
            }))
            .with_coverage(TextSelectionCoverage::Full);
        let registration = TextSelectionRegistration::new(hitbox, bounds)
            .with_scroll_offset(point(px(3.), px(4.)))
            .with_scope(scope)
            .with_document_order(9)
            .with_text_bounds(vec![bounds]);

        assert_eq!(endpoint.entity_id(), None);
        assert_eq!(endpoint.content_point(), bounds.origin);
        assert_eq!(
            endpoint.content_key(),
            Some(TextSelectionContentKey::new(11))
        );
        assert_eq!(snapshot.anchor(), endpoint);
        assert_eq!(snapshot.cursor(), endpoint);
        assert!(snapshot.is_selecting());
        assert_eq!(snapshot.coverage(), TextSelectionCoverage::Full);
        assert_eq!(
            snapshot.window_points(),
            Some(TextSelectionWindowPoints {
                anchor: bounds.origin,
                cursor: bounds.bottom_right(),
            })
        );
        assert_eq!(registration.bounds(), bounds);
        assert_eq!(registration.scroll_offset(), point(px(3.), px(4.)));
        assert_eq!(registration.scope(), scope);
        assert_eq!(registration.document_order(), 9);
        assert_eq!(registration.text_bounds(), &[bounds]);

        let (text, layout) = laid_out_runs(&["aé"], cx).pop().unwrap();
        let text_run = TextSelectionRun::new(text.clone(), layout.clone(), layout.bounds())
            .with_document_order(3);
        assert_eq!(text_run.document_order(), 3);
        assert_eq!(text_run.text(), &text);
        assert_eq!(text_run.layout().len(), layout.len());
        assert_eq!(text_run.bounds(), layout.bounds());

        let projection = TextSelectionProjection {
            ranges: vec![Some(1..3)],
            is_active: true,
        };
        assert_eq!(projection.ranges(), &[Some(1..3)]);
        assert!(projection.is_active());
    }

    #[gpui::test]
    fn selection_handle_is_the_public_adapter_seam(cx: &mut TestAppContext) {
        let selected = Rc::new(Cell::new(false));
        let selected_from_callback = selected.clone();
        cx.update(|cx| {
            let selection = TextSelectionHandle::new("initial", cx);
            let entity_id = selection.entity_id();
            selection.set_fallback_copy_text("updated", cx);
            selection.set_local_selection(true, cx);
            selection
                .subscribe(
                    move |event, _| {
                        if let TextSelectionEvent::SelectionChanged(snapshot) = event {
                            selected_from_callback.set(snapshot.is_some());
                        }
                    },
                    cx,
                )
                .detach();
            selection.focus_with(|_, _| {}, cx);
            selection.copy_with(|_| "copied".to_string(), cx);
            selection.resolve_content_key_with(|_, _| Some(TextSelectionContentKey::new(3)), cx);

            assert_eq!(selection.entity_id(), entity_id);
            assert_eq!(selection.snapshot(cx), None);
            assert_eq!(
                selection.update_runs(&[], cx),
                TextSelectionProjection::default()
            );
        });
        assert!(!selected.get());
    }

    #[gpui::test]
    fn selection_handle_can_subscribe_its_window_to_refresh(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, cx| WindowSelectionView {
            selection: TextSelectionHandle::new("refresh", cx),
        });
        cx.update(|window, cx| {
            let selection = TextSelectionHandle::new("refresh", cx);
            selection.refresh_window_on_change(window, cx).detach();
        });
    }

    #[gpui::test]
    fn plain_projection_preserves_forward_reversed_and_unicode_ranges(cx: &mut TestAppContext) {
        let (text, layout) = laid_out_runs(&["aé🙂z"], cx).pop().unwrap();
        let run = text_run(0, text, layout.clone());
        let start = layout.position_for_index(1).unwrap();
        let end = layout.position_for_index(7).unwrap();

        let forward = project_ranges(Some(plain_snapshot(start, end)), std::slice::from_ref(&run));
        let reversed = project_ranges(Some(plain_snapshot(end, start)), &[run]);

        assert_eq!(forward.ranges(), &[Some(1..7)]);
        assert_eq!(reversed.ranges(), &[Some(1..7)]);
        assert!(forward.is_active());
        assert!(reversed.is_active());
    }

    #[gpui::test]
    fn double_click_expands_a_plain_run_to_the_input_word_boundary(cx: &mut TestAppContext) {
        let (text, layout) = laid_out_runs(&["one café, three"], cx).pop().unwrap();
        let run = text_run(0, text, layout.clone());
        let click = layout.position_for_index(6).unwrap();

        let (anchor, cursor) =
            points_for_multi_click(std::slice::from_ref(&run), click, 2).unwrap();
        let states = project_ranges(Some(plain_snapshot(anchor, cursor)), &[run]);

        assert_eq!(states.ranges(), &[Some(4..9)]);
    }

    #[gpui::test]
    fn multi_click_uses_text_layout_window_coordinates_at_a_nonzero_origin(
        cx: &mut TestAppContext,
    ) {
        let mut runs = laid_out_runs(&["above", "alpha beta"], cx);
        let (text, layout) = runs.pop().unwrap();
        assert!(layout.bounds().origin.y > px(0.));
        let run = text_run(0, text, layout.clone());
        let click = layout.position_for_index(7).unwrap();

        let (anchor, cursor) =
            points_for_multi_click(std::slice::from_ref(&run), click, 2).unwrap();
        let projection = project_ranges(Some(plain_snapshot(anchor, cursor)), &[run]);

        assert_eq!(projection.ranges(), &[Some(6..10)]);
    }

    #[gpui::test]
    fn triple_click_expands_to_the_input_logical_line_not_the_visual_row(cx: &mut TestAppContext) {
        let (text, layout) = laid_out_runs(&["second line"], cx).pop().unwrap();
        let run = text_run(0, text, layout.clone());
        let click = layout.position_for_index(4).unwrap();

        let (anchor, cursor) =
            points_for_multi_click(std::slice::from_ref(&run), click, 4).unwrap();
        let states = project_ranges(Some(plain_snapshot(anchor, cursor)), &[run]);

        assert_eq!(states.ranges(), &[Some(0..11)]);
        assert_eq!(line_range_at("first line\nsecond line\nthird", 15), 11..22);
    }

    #[gpui::test]
    fn plain_projection_spans_multiple_runs_and_leaves_empty_gutters_unselected(
        cx: &mut TestAppContext,
    ) {
        let mut runs = laid_out_runs(&["first", "", "second"], cx);
        let (first_text, first_layout) = runs.remove(0);
        let (gutter_text, gutter_layout) = runs.remove(0);
        let (second_text, second_layout) = runs.remove(0);
        let start = first_layout.position_for_index(2).unwrap();
        let end = second_layout.position_for_index(3).unwrap();
        let states = project_ranges(
            Some(plain_snapshot(start, end)),
            &[
                text_run(2, second_text, second_layout),
                text_run(1, gutter_text, gutter_layout),
                text_run(0, first_text, first_layout),
            ],
        );

        assert_eq!(states.ranges(), &[Some(0..3), None, Some(2..5)]);
        assert!(states.is_active());
    }

    #[gpui::test]
    fn plain_projection_caches_multiple_participant_copies_in_document_order(
        cx: &mut TestAppContext,
    ) {
        let mut runs = laid_out_runs(&["one", "two"], cx);
        let (first_text, first_layout) = runs.remove(0);
        let (second_text, second_layout) = runs.remove(0);
        let snapshot = plain_snapshot(
            first_layout.position_for_index(1).unwrap(),
            second_layout.position_for_index(2).unwrap(),
        );
        cx.update(|cx| {
            let mut selection_state = WindowSelectionState::default();
            let first = FakeParticipant::new("", cx);
            let second = FakeParticipant::new("", cx);
            first.register(
                &mut selection_state,
                0.,
                TextSelectionScopeId::default(),
                1,
                cx,
            );
            second.register(
                &mut selection_state,
                20.,
                TextSelectionScopeId::default(),
                0,
                cx,
            );

            first
                .selection
                .0
                .update(cx, |state, cx| state.set_snapshot(Some(snapshot), cx));
            let projection = first
                .selection
                .update_runs(&[text_run(0, first_text, first_layout)], cx);
            assert_eq!(projection.ranges(), &[Some(1..3)]);
            assert!(projection.is_active());
            second
                .selection
                .0
                .update(cx, |state, cx| state.set_snapshot(Some(snapshot), cx));
            let projection = second
                .selection
                .update_runs(&[text_run(0, second_text, second_layout)], cx);
            assert_eq!(projection.ranges(), &[Some(0..2)]);
            assert!(projection.is_active());

            assert_eq!(selection_state.selected_text(cx), "tw\nne");
        });
    }

    #[gpui::test]
    fn plain_projection_invalidates_cached_copy_when_the_snapshot_changes(cx: &mut TestAppContext) {
        let (text, layout) = laid_out_runs(&["first"], cx).pop().unwrap();
        let first_snapshot = plain_snapshot(
            layout.position_for_index(1).unwrap(),
            layout.position_for_index(3).unwrap(),
        );
        let changed_snapshot = plain_snapshot(
            layout.position_for_index(3).unwrap(),
            layout.position_for_index(5).unwrap(),
        );
        let run = text_run(0, text, layout);
        cx.update(|cx| {
            let mut selection_state = WindowSelectionState::default();
            let participant = FakeParticipant::new("", cx);
            participant.register(
                &mut selection_state,
                0.,
                TextSelectionScopeId::default(),
                0,
                cx,
            );
            participant.selection.0.update(cx, |state, cx| {
                state.set_snapshot(Some(first_snapshot), cx);
                state.update_runs(std::slice::from_ref(&run));
            });
            assert_eq!(selection_state.selected_text(cx), "ir");

            participant.selection.0.update(cx, |state, cx| {
                state.set_snapshot(Some(changed_snapshot), cx);
            });
            assert_eq!(selection_state.selected_text(cx), "");

            participant.selection.update_runs(&[run], cx);
            assert_eq!(selection_state.selected_text(cx), "st");
            selection_state.clear(cx);
            participant.selection.set_local_selection(true, cx);
            assert_eq!(selection_state.selected_text(cx), "");
        });
    }

    #[gpui::test]
    fn plain_projection_orders_cached_runs_by_frame_order_not_input_order(cx: &mut TestAppContext) {
        let mut runs = laid_out_runs(&["one", "two"], cx);
        let (first_text, first_layout) = runs.remove(0);
        let (second_text, second_layout) = runs.remove(0);
        let snapshot = plain_snapshot(
            first_layout.position_for_index(1).unwrap(),
            second_layout.position_for_index(2).unwrap(),
        );
        cx.update(|cx| {
            let mut selection_state = WindowSelectionState::default();
            let participant = FakeParticipant::new("", cx);
            participant.register(
                &mut selection_state,
                0.,
                TextSelectionScopeId::default(),
                0,
                cx,
            );
            participant.selection.0.update(cx, |state, cx| {
                state.set_snapshot(Some(snapshot), cx);
                state.update_runs(&[
                    text_run(1, first_text, first_layout),
                    text_run(0, second_text, second_layout),
                ]);
            });

            assert_eq!(selection_state.selected_text(cx), "twne");
        });
    }

    #[gpui::test]
    fn plain_projection_safely_rejects_a_text_layout_length_mismatch(cx: &mut TestAppContext) {
        let (_, layout) = laid_out_runs(&["short"], cx).pop().unwrap();
        let start = layout.position_for_index(0).unwrap();
        let end = layout.position_for_index(5).unwrap();
        let states = project_ranges(
            Some(plain_snapshot(start, end)),
            &[text_run(0, SharedString::from("longer"), layout)],
        );

        assert_eq!(states.ranges(), &[None]);
        assert!(states.is_active());
    }

    #[gpui::test]
    fn begin_update_and_end_publish_a_cross_participant_selection(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut selection_state = WindowSelectionState::default();
            let first = FakeParticipant::new("first", cx);
            let second = FakeParticipant::new("second", cx);
            first.register(
                &mut selection_state,
                0.,
                TextSelectionScopeId::default(),
                0,
                cx,
            );
            second.register(
                &mut selection_state,
                20.,
                TextSelectionScopeId::default(),
                1,
                cx,
            );

            selection_state.begin(point(px(1.), px(1.)), false, cx);
            selection_state.update(point(px(1.), px(25.)), cx);
            assert!(selection_state.has_selection(cx));
            assert_eq!(selection_state.selected_text(cx), "first\nsecond");

            selection_state.end(cx);
            assert!(!selection_state.is_selecting());
        });
    }

    #[gpui::test]
    fn shift_extension_keeps_its_original_anchor_when_reversed(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut selection_state = WindowSelectionState::default();
            let participant = FakeParticipant::new("participant", cx);
            participant.register(
                &mut selection_state,
                0.,
                TextSelectionScopeId::default(),
                0,
                cx,
            );

            selection_state.begin(point(px(2.), px(2.)), false, cx);
            selection_state.end(cx);
            selection_state.begin(point(px(8.), px(2.)), true, cx);
            selection_state.end(cx);
            let first_anchor = selection_state.snapshot().unwrap().anchor();

            selection_state.begin(point(px(0.), px(2.)), true, cx);
            selection_state.end(cx);
            let reversed = selection_state.snapshot().unwrap();
            assert_eq!(reversed.anchor(), first_anchor);
            assert!(reversed.cursor().content_point().x < reversed.anchor().content_point().x);
        });
    }

    #[gpui::test]
    fn content_key_resolver_runs_outside_the_window_state_lease(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let state = cx.new(|_| WindowSelectionState::default());
            let participant = FakeParticipant::new("virtual", cx);
            let state_for_callback = state.clone();
            participant.selection.resolve_content_key_with(
                move |_, cx| {
                    let _ = state_for_callback.read(cx).snapshot();
                    Some(TextSelectionContentKey::new(7))
                },
                cx,
            );
            state.update(cx, |state, cx| {
                participant.register(state, 0., TextSelectionScopeId::default(), 0, cx);
                state.begin(point(px(1.), px(1.)), false, cx);
                state.update(point(px(8.), px(1.)), cx);
            });

            WindowSelectionState::resolve_content_keys(&state, cx);

            assert_eq!(
                state.read(cx).snapshot().unwrap().cursor().content_key(),
                Some(TextSelectionContentKey::new(7))
            );
        });
    }

    #[gpui::test]
    fn active_dnd_does_not_move_a_text_selection_cursor(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, cx| WindowSelectionView {
            selection: TextSelectionHandle::new("unused", cx),
        });
        window
            .update(cx, |_, window, cx| {
                let mut state = WindowSelectionState::default();
                let participant = FakeParticipant::new("participant", cx);
                participant.register(&mut state, 0., TextSelectionScopeId::default(), 0, cx);
                state.begin(point(px(1.), px(1.)), false, cx);
                let before = state.cursor.as_ref().unwrap().point;
                state.update_in_window_with_active_drag(point(px(80.), px(1.)), true, window, cx);
                assert_eq!(state.cursor.as_ref().unwrap().point, before);
            })
            .unwrap();
    }

    #[gpui::test]
    fn shift_extension_falls_back_when_the_anchor_participant_was_swept(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut selection_state = WindowSelectionState::default();
            let first = FakeParticipant::new("first", cx);
            let second = FakeParticipant::new("second", cx);
            first.register(
                &mut selection_state,
                0.,
                TextSelectionScopeId::default(),
                0,
                cx,
            );
            selection_state.begin(point(px(1.), px(1.)), false, cx);
            selection_state.update(point(px(8.), px(1.)), cx);
            selection_state.end(cx);

            selection_state.finish_frame(cx);
            selection_state.finish_frame(cx);
            second.register(
                &mut selection_state,
                20.,
                TextSelectionScopeId::default(),
                1,
                cx,
            );
            selection_state.begin(point(px(1.), px(21.)), true, cx);
            selection_state.update(point(px(8.), px(21.)), cx);
            selection_state.end(cx);

            assert_eq!(selection_state.selected_text(cx), "second");
        });
    }

    #[gpui::test]
    fn scope_and_suppression_prevent_unrelated_participants_from_participating(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            let mut selection_state = WindowSelectionState::default();
            let base = FakeParticipant::new("base", cx);
            let modal = FakeParticipant::new("modal", cx);
            base.register(
                &mut selection_state,
                0.,
                TextSelectionScopeId::default(),
                0,
                cx,
            );
            modal.register(&mut selection_state, 20., TextSelectionScopeId(1), 1, cx);

            selection_state.set_active_scope(TextSelectionScopeId(1), cx);
            selection_state.begin(point(px(1.), px(21.)), false, cx);
            selection_state.update(point(px(8.), px(21.)), cx);
            selection_state.end(cx);
            assert_eq!(selection_state.selected_text(cx), "modal");

            selection_state.clear(cx);
            GlobalState::init(cx);
            GlobalState::suppress_text_selection(cx);
            selection_state.begin(point(px(1.), px(21.)), false, cx);
            selection_state.update(point(px(8.), px(21.)), cx);
            assert!(!selection_state.has_selection(cx));
        });
    }

    #[gpui::test]
    fn dead_participants_are_pruned_and_empty_selection_falls_back_safely(cx: &mut TestAppContext) {
        let selection_state = cx.update(|cx| {
            let selection_state = cx.new(|_| WindowSelectionState::default());
            let participant = FakeParticipant::new("gone", cx);
            selection_state.update(cx, |selection_state, cx| {
                participant.register(selection_state, 0., TextSelectionScopeId::default(), 0, cx)
            });
            selection_state
        });
        cx.update(|cx| {
            selection_state.update(cx, |selection_state, cx| {
                selection_state.begin(point(px(1.), px(1.)), false, cx);
                selection_state.update(point(px(8.), px(1.)), cx);
                selection_state.end(cx);

                assert_eq!(selection_state.selected_text(cx), "");
                assert!(!selection_state.has_selection(cx));
            });
        });
    }

    #[gpui::test]
    fn text_selection_namespace_reports_copies_ends_and_clears_selection(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| WindowSelectionView {
            selection: TextSelectionHandle::new("copied", cx),
        });
        cx.update(|window, cx| {
            let selection = view.read(cx).selection.clone();
            let selection_state = WindowSelectionState::ensure(window, cx);
            selection_state.update(cx, |selection_state, cx| {
                FakeParticipant { selection }.register(
                    selection_state,
                    0.,
                    TextSelectionScopeId::default(),
                    0,
                    cx,
                );
                selection_state.begin(point(px(1.), px(1.)), false, cx);
                selection_state.update(point(px(8.), px(1.)), cx);
            });

            assert!(TextSelection::has_selection(window, cx));
            assert_eq!(TextSelection::selected_text(window, cx), "copied");
            TextSelection::end(window, cx);
            assert!(TextSelection::has_selection(window, cx));
            TextSelection::clear(window, cx);
            assert!(!TextSelection::has_selection(window, cx));
            assert_eq!(TextSelection::selected_text(window, cx), "");
        });
    }

    #[gpui::test]
    fn two_windows_isolate_selection_copy_clear_and_release_ownership(cx: &mut TestAppContext) {
        let first = cx.add_window(|_, cx| WindowOwnedSelectionView {
            selection: TextSelectionHandle::new("first", cx),
        });
        let second = cx.add_window(|_, cx| WindowOwnedSelectionView {
            selection: TextSelectionHandle::new("second", cx),
        });
        let first_selection = cx.update(|cx| first.read(cx).unwrap().selection.clone());
        let second_selection = cx.update(|cx| second.read(cx).unwrap().selection.clone());

        let first_state = cx
            .update_window(*first, |_, window, cx| {
                let _ = window.draw(cx);
                first_selection.set_local_selection(true, cx);
                assert_eq!(TextSelection::selected_text(window, cx), "first");
                WindowSelectionState::existing(window, cx)
                    .unwrap()
                    .downgrade()
            })
            .unwrap();
        cx.update_window(*second, |_, window, cx| {
            let _ = window.draw(cx);
            second_selection.set_local_selection(true, cx);
            assert_eq!(TextSelection::selected_text(window, cx), "second");
        })
        .unwrap();

        cx.update_window(*first, |_, window, cx| {
            TextSelection::clear(window, cx);
            assert_eq!(TextSelection::selected_text(window, cx), "");
        })
        .unwrap();
        cx.update_window(*second, |_, window, cx| {
            assert_eq!(TextSelection::selected_text(window, cx), "second");
        })
        .unwrap();

        cx.update_window(*first, |_, window, _| window.remove_window())
            .unwrap();
        cx.run_until_parked();

        assert!(first_state.upgrade().is_none());
        cx.update_window(*second, |_, window, cx| {
            assert_eq!(TextSelection::selected_text(window, cx), "second");
        })
        .unwrap();
        cx.update(|cx| {
            assert_eq!(cx.global::<SelectionStateRegistry>().0.len(), 1);
        });
    }

    #[gpui::test]
    fn copy_callback_can_reenter_window_and_handle_selection(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| SelectionElementOnlyView);
        cx.update(|window, cx| {
            let _ = window.draw(cx);
            let state = WindowSelectionState::existing(window, cx).unwrap();
            let selection = TextSelectionHandle::new("fallback", cx);
            let state_for_copy = state.clone();
            let selection_for_copy = selection.clone();
            selection.copy_with(
                move |cx: &mut App| {
                    state_for_copy.update(cx, |state, _| {
                        assert!(state.snapshot().is_some());
                    });
                    assert!(selection_for_copy.snapshot(cx).is_some());
                    selection_for_copy.set_fallback_copy_text("reentered", cx);
                    "reentrant copy".to_string()
                },
                cx,
            );
            state.update(cx, |state, cx| {
                FakeParticipant {
                    selection: selection.clone(),
                }
                .register(state, 0., TextSelectionScopeId::default(), 0, cx);
                state.begin(point(px(1.), px(1.)), false, cx);
                state.update(point(px(8.), px(1.)), cx);
                state.end(cx);
            });

            assert_eq!(TextSelection::selected_text(window, cx), "reentrant copy");
        });
    }

    #[gpui::test]
    fn cross_participant_selection_excludes_participants_outside_its_document_interval(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            let mut selection_state = WindowSelectionState::default();
            let first = FakeParticipant::new("first", cx);
            let second = FakeParticipant::new("second", cx);
            let third = FakeParticipant::new("third", cx);
            first.register(
                &mut selection_state,
                0.,
                TextSelectionScopeId::default(),
                0,
                cx,
            );
            second.register(
                &mut selection_state,
                20.,
                TextSelectionScopeId::default(),
                1,
                cx,
            );
            third.register(
                &mut selection_state,
                40.,
                TextSelectionScopeId::default(),
                2,
                cx,
            );

            selection_state.begin(point(px(1.), px(1.)), false, cx);
            selection_state.update(point(px(1.), px(25.)), cx);
            selection_state.end(cx);

            assert_eq!(selection_state.selected_text(cx), "first\nsecond");
            assert!(third.selection.snapshot(cx).is_none());
        });
    }

    #[gpui::test]
    fn changing_scope_clears_the_previous_scope_selection(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut selection_state = WindowSelectionState::default();
            let base = FakeParticipant::new("base", cx);
            let modal = FakeParticipant::new("modal", cx);
            base.register(
                &mut selection_state,
                0.,
                TextSelectionScopeId::default(),
                0,
                cx,
            );
            modal.register(
                &mut selection_state,
                20.,
                TextSelectionScopeId::from_raw(1),
                1,
                cx,
            );

            selection_state.begin(point(px(1.), px(1.)), false, cx);
            selection_state.update(point(px(8.), px(1.)), cx);
            selection_state.end(cx);
            selection_state.set_active_scope(TextSelectionScopeId::from_raw(1), cx);

            assert!(!selection_state.has_selection(cx));
            assert!(base.selection.snapshot(cx).is_none());
        });
    }

    #[gpui::test]
    fn blank_only_drag_never_publishes_or_copies_selection(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut selection_state = WindowSelectionState::default();
            let participant = FakeParticipant::new("participant", cx);
            participant.register(
                &mut selection_state,
                0.,
                TextSelectionScopeId::default(),
                0,
                cx,
            );

            selection_state.begin(point(px(200.), px(1.)), false, cx);
            selection_state.update(point(px(200.), px(8.)), cx);
            selection_state.end(cx);

            assert!(!selection_state.has_selection(cx));
            assert_eq!(selection_state.selected_text(cx), "");
            assert!(participant.selection.snapshot(cx).is_none());
        });
    }

    #[gpui::test]
    fn stale_live_participants_are_removed_when_the_next_frame_begins(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut selection_state = WindowSelectionState::default();
            let participant = FakeParticipant::new("stale", cx);
            participant.register(
                &mut selection_state,
                0.,
                TextSelectionScopeId::default(),
                0,
                cx,
            );
            selection_state.begin(point(px(1.), px(1.)), false, cx);
            selection_state.update(point(px(8.), px(1.)), cx);
            selection_state.end(cx);

            selection_state.finish_frame(cx);
            selection_state.finish_frame(cx);
            assert_eq!(selection_state.selected_text(cx), "");
            assert!(participant.selection.snapshot(cx).is_none());
        });
    }

    #[gpui::test]
    fn clear_stops_anchor_auto_scroll_before_discarding_the_anchor(cx: &mut TestAppContext) {
        let commands = Rc::new(RefCell::new(Vec::new()));
        let observed = commands.clone();
        let (mut selection_state, participant) = cx.update(|cx| {
            let selection_state = WindowSelectionState::default();
            let participant = FakeParticipant::new("scroll", cx);
            participant
                .selection
                .subscribe(
                    move |event, _| {
                        if let TextSelectionEvent::AutoScroll(delta) = event {
                            observed.borrow_mut().push(*delta);
                        }
                    },
                    cx,
                )
                .detach();
            (selection_state, participant)
        });
        cx.run_until_parked();
        cx.update(|cx| {
            participant.register(
                &mut selection_state,
                0.,
                TextSelectionScopeId::default(),
                0,
                cx,
            );

            selection_state.begin(point(px(1.), px(1.)), false, cx);
            selection_state.update(point(px(1.), px(25.)), cx);
            selection_state.clear(cx);
        });
        cx.run_until_parked();
        assert!(commands.borrow().iter().any(Option::is_some));
        assert_eq!(commands.borrow().last(), Some(&None));
    }

    #[gpui::test]
    fn drag_auto_scroll_stops_when_the_content_mask_collapses(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, cx| WindowSelectionView {
            selection: TextSelectionHandle::new("unused", cx),
        });
        window
            .update(cx, |_, window, cx| {
                let state = cx.new(|_| WindowSelectionState::default());
                let participant = FakeParticipant::new("participant", cx);
                state.update(cx, |state, cx| {
                    participant.register(state, 0., TextSelectionScopeId::default(), 0, cx);
                    state.begin(point(px(1.), px(1.)), false, cx);
                });
                // The scrollable ancestor got clipped away mid-drag, so the
                // refreshed registration carries a collapsed content mask.
                let collapsed = Bounds::new(point(px(0.), px(0.)), size(px(100.), px(0.)));
                state.update(cx, |state, cx| {
                    state.register_participant(
                        participant.selection.clone(),
                        TextSelectionRegistration::new(
                            Hitbox {
                                id: HitboxId::placeholder(),
                                bounds: collapsed,
                                content_mask: ContentMask { bounds: collapsed },
                                behavior: HitboxBehavior::Normal,
                            },
                            collapsed,
                        )
                        .with_text_bounds(vec![collapsed]),
                        cx,
                    );
                    state.update_in_window(point(px(1.), px(50.)), window, cx);
                    assert!(!state.auto_scroll.is_active());
                    assert!(state.auto_scroll.last_drag_position.is_none());
                });
            })
            .unwrap();
    }

    #[gpui::test]
    fn pointer_moves_after_a_click_do_not_auto_scroll(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, cx| WindowSelectionView {
            selection: TextSelectionHandle::new("unused", cx),
        });
        window
            .update(cx, |_, window, cx| {
                let state = cx.new(|_| WindowSelectionState::default());
                let participant = FakeParticipant::new("participant", cx);
                state.update(cx, |state, cx| {
                    participant.register(state, 0., TextSelectionScopeId::default(), 0, cx);
                    // A click on text keeps its anchor so shift-click can extend it.
                    state.begin(point(px(1.), px(1.)), false, cx);
                    state.end(cx);
                    assert!(state.anchor.is_some());

                    state.update_in_window(point(px(1.), px(50.)), window, cx);
                    assert!(!state.auto_scroll.is_active());
                });
            })
            .unwrap();
    }

    #[gpui::test]
    fn proxy_endpoints_break_equal_position_ties_by_document_order(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut selection_state = WindowSelectionState::default();
            let later = FakeParticipant::new("later", cx);
            let earlier = FakeParticipant::new("earlier", cx);
            later.register(
                &mut selection_state,
                0.,
                TextSelectionScopeId::default(),
                2,
                cx,
            );
            earlier.register(
                &mut selection_state,
                0.,
                TextSelectionScopeId::default(),
                1,
                cx,
            );

            selection_state.begin(point(px(1.), px(1.)), false, cx);
            selection_state.update(point(px(200.), px(25.)), cx);
            let endpoint = selection_state.snapshot().unwrap().cursor();

            assert_eq!(endpoint.entity_id(), Some(earlier.selection.entity_id()));
        });
    }

    #[gpui::test]
    fn equal_area_hovered_participants_break_ties_by_document_order(cx: &mut TestAppContext) {
        cx.update(|cx| {
            for _ in 0..64 {
                let mut selection_state = WindowSelectionState::default();
                let later = FakeParticipant::new("later", cx);
                let earliest = FakeParticipant::new("earliest", cx);
                let middle = FakeParticipant::new("middle", cx);
                later.register(
                    &mut selection_state,
                    0.,
                    TextSelectionScopeId::default(),
                    30,
                    cx,
                );
                earliest.register(
                    &mut selection_state,
                    0.,
                    TextSelectionScopeId::default(),
                    10,
                    cx,
                );
                middle.register(
                    &mut selection_state,
                    0.,
                    TextSelectionScopeId::default(),
                    20,
                    cx,
                );

                selection_state.begin(point(px(1.), px(1.)), false, cx);
                selection_state.update(point(px(8.), px(1.)), cx);

                assert_eq!(
                    selection_state.snapshot().unwrap().anchor().entity_id(),
                    Some(earliest.selection.entity_id())
                );
            }
        });
    }

    #[gpui::test]
    fn text_selection_namespace_is_a_safe_no_op_until_the_element_is_rendered(
        cx: &mut TestAppContext,
    ) {
        let (_, cx) = cx.add_window_view(|_, cx| WindowSelectionView {
            selection: TextSelectionHandle::new("not enabled", cx),
        });
        cx.update(|window, cx| {
            assert!(!TextSelection::has_selection(window, cx));
            assert_eq!(TextSelection::selected_text(window, cx), "");
            TextSelection::clear(window, cx);
            TextSelection::end(window, cx);
            assert!(!TextSelection::has_selection(window, cx));
        });
    }

    #[gpui::test]
    fn unit_selection_element_supports_scope_and_registration_on_the_first_frame(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, cx| FirstFrameScopedSelectionView {
            selection: TextSelectionHandle::new("first frame", cx),
        });
        let selection = cx.update(|_, cx| view.read(cx).selection.clone());

        cx.update(|window, cx| {
            let _ = window.draw(cx);
            let state = WindowSelectionState::existing(window, cx).unwrap();
            assert_eq!(
                state.read(cx).active_scope,
                TextSelectionScopeId::from_raw(23)
            );
            assert!(
                state
                    .read(cx)
                    .participants
                    .contains_key(&selection.entity_id())
            );
        });
    }

    #[gpui::test]
    fn lazy_registration_does_not_enable_queries_without_the_element(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, cx| WindowSelectionView {
            selection: TextSelectionHandle::new("registered", cx),
        });
        cx.update(|window, cx| {
            let selection = TextSelectionHandle::new("registered", cx);
            selection.set_local_selection(true, cx);
            let bounds = Bounds::new(point(px(0.), px(0.)), size(px(100.), px(20.)));
            let hitbox = Hitbox {
                id: HitboxId::placeholder(),
                bounds,
                content_mask: ContentMask { bounds },
                behavior: HitboxBehavior::Normal,
            };
            selection.register(
                TextSelectionRegistration::new(hitbox, bounds).with_text_bounds(vec![bounds]),
                window,
                cx,
            );
            assert_eq!(TextSelection::selected_text(window, cx), "");
            assert!(!TextSelection::has_selection(window, cx));
            TextSelection::clear(window, cx);
            assert_eq!(TextSelection::selected_text(window, cx), "");
        });
    }

    #[gpui::test]
    fn retained_selection_state_releases_and_does_not_resurrect_selection(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| ToggleSelectionElementView {
            enabled: true,
            selection: TextSelectionHandle::new("local", cx),
        });
        let selection = cx.update(|_, cx| view.read(cx).selection.clone());
        cx.update(|window, cx| {
            let _ = window.draw(cx);
            selection.set_local_selection(true, cx);
            assert!(TextSelection::has_selection(window, cx));

            window.simulate_next_frame(cx);
            assert!(TextSelection::has_selection(window, cx));
            let _ = window.draw(cx);
            assert!(TextSelection::has_selection(window, cx));
        });
        view.update(cx, |view, cx| {
            view.enabled = false;
            cx.notify();
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.update(|window, cx| {
            window.simulate_next_frame(cx);
        });
        cx.update(|window, cx| {
            window.simulate_next_frame(cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(!TextSelection::has_selection(window, cx));
            assert_eq!(TextSelection::selected_text(window, cx), "");
            assert!(!selection.has_local_selection(cx));
            TextSelection::clear(window, cx);
        });

        view.update(cx, |view, cx| {
            view.enabled = true;
            cx.notify();
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
            assert!(!TextSelection::has_selection(window, cx));
            assert_eq!(TextSelection::selected_text(window, cx), "");
        });
    }

    #[gpui::test]
    fn mounted_selection_element_does_not_keep_an_idle_frame_queue_alive(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| SelectionElementOnlyView);
        cx.update(|window, cx| {
            let _ = window.draw(cx);
            assert_eq!(window.simulate_next_frame(cx), 0);
            assert_eq!(window.simulate_next_frame(cx), 0);
            assert!(live_text_selection_state(window, cx).is_some());
        });
    }

    #[gpui::test]
    fn selection_element_initializes_suppression_and_respects_bubble_suppression(
        cx: &mut TestAppContext,
    ) {
        let (_, cx) = cx.add_window_view(|_, _| SelectionElementOnlyView);
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.simulate_mouse_down(
            point(px(1.), px(1.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(1.), px(1.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.update(|window, cx| {
            assert!(GlobalState::is_text_selection_suppressed(cx));
            assert!(!TextSelection::has_selection(window, cx));
        });
    }

    #[gpui::test]
    fn frame_sweep_keeps_a_participant_registered_before_the_selection_element_paints(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            let mut selection_state = WindowSelectionState::default();
            let participant = FakeParticipant::new("painted first", cx);
            participant.register(
                &mut selection_state,
                0.,
                TextSelectionScopeId::default(),
                0,
                cx,
            );
            selection_state.begin(point(px(1.), px(1.)), false, cx);
            selection_state.update(point(px(8.), px(1.)), cx);
            selection_state.end(cx);

            selection_state.finish_frame(cx);

            assert_eq!(selection_state.selected_text(cx), "painted first");
            assert!(participant.selection.snapshot(cx).is_some());
        });
    }

    #[gpui::test]
    fn two_selection_elements_schedule_only_one_post_frame_sweep(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| DoubleSelectionElementView {
            selection: TextSelectionHandle::new("once", cx),
        });
        cx.update(|window, cx| {
            let selection_state = WindowSelectionState::ensure(window, cx);
            let selection = view.read(cx).selection.clone();
            selection_state.update(cx, |selection_state, cx| {
                FakeParticipant { selection }.register(
                    selection_state,
                    0.,
                    TextSelectionScopeId::default(),
                    0,
                    cx,
                );
                selection_state.begin(point(px(1.), px(1.)), false, cx);
                selection_state.update(point(px(8.), px(1.)), cx);
                selection_state.end(cx);
            });

            let _ = window.draw(cx);
            window.simulate_next_frame(cx);

            let items = selection_state.read(cx).copy_items(cx);
            assert_eq!(resolve_copy_items(items, cx), "once");
        });
    }

    #[gpui::test]
    fn duplicate_selection_elements_gate_real_pointer_gestures_and_reentrant_clear(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, cx| DoubleSelectionElementView {
            selection: TextSelectionHandle::new("once", cx),
        });
        let clear_count = Rc::new(Cell::new(0));
        cx.update(|window, cx| {
            let state = WindowSelectionState::ensure(window, cx);
            let state_for_clear = state.clone();
            let count = clear_count.clone();
            let selection = view.read(cx).selection.clone();
            selection
                .subscribe(
                    move |event, cx| {
                        if matches!(event, TextSelectionEvent::Cleared) {
                            count.set(count.get() + 1);
                            let _ = state_for_clear.read(cx).snapshot();
                        }
                    },
                    cx,
                )
                .detach();
            let _ = window.draw(cx);
        });

        cx.simulate_mouse_down(
            point(px(10.), px(10.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(10.), px(10.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.simulate_mouse_down(
            point(px(70.), px(10.)),
            MouseButton::Left,
            gpui::Modifiers {
                shift: true,
                ..Default::default()
            },
        );
        cx.simulate_mouse_up(
            point(px(70.), px(10.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.update(|window, cx| assert!(TextSelection::has_selection(window, cx)));

        cx.simulate_mouse_down(
            point(px(15.), px(10.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.simulate_mouse_move(
            point(px(85.), px(10.)),
            Some(MouseButton::Left),
            gpui::Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(85.), px(10.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.update(|window, cx| assert!(TextSelection::has_selection(window, cx)));
        assert_eq!(clear_count.get(), 3);
    }

    #[gpui::test]
    fn selection_layer_handles_real_double_and_triple_click_events(cx: &mut TestAppContext) {
        let (text, layout) = laid_out_runs(&["alpha beta"], cx).pop().unwrap();
        let (view, cx) = cx.add_window_view(|_, cx| DoubleSelectionElementView {
            selection: TextSelectionHandle::new("", cx),
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
            let selection = view.read(cx).selection.clone();
            selection.resolve_content_key_with(|_, _| Some(TextSelectionContentKey::new(17)), cx);
            selection.update_runs(&[text_run(0, text.clone(), layout.clone())], cx);
        });

        let position = layout.position_for_index(7).unwrap();
        cx.simulate_event(MouseDownEvent {
            position,
            modifiers: gpui::Modifiers::default(),
            button: MouseButton::Left,
            click_count: 2,
            first_mouse: false,
        });
        cx.simulate_event(MouseUpEvent {
            position,
            modifiers: gpui::Modifiers::default(),
            button: MouseButton::Left,
            click_count: 2,
        });
        cx.update(|window, cx| {
            let selection = view.read(cx).selection.clone();
            selection.update_runs(&[text_run(0, text.clone(), layout.clone())], cx);
            assert_eq!(TextSelection::selected_text(window, cx), "beta");
            let snapshot = selection.snapshot(cx).unwrap();
            assert_eq!(
                snapshot.anchor().content_key(),
                Some(TextSelectionContentKey::new(17))
            );
            assert_eq!(
                snapshot.cursor().content_key(),
                Some(TextSelectionContentKey::new(17))
            );
        });

        cx.simulate_event(MouseDownEvent {
            position,
            modifiers: gpui::Modifiers::default(),
            button: MouseButton::Left,
            click_count: 3,
            first_mouse: false,
        });
        cx.simulate_event(MouseUpEvent {
            position,
            modifiers: gpui::Modifiers::default(),
            button: MouseButton::Left,
            click_count: 3,
        });
        cx.update(|window, cx| {
            let selection = view.read(cx).selection.clone();
            selection.update_runs(&[text_run(0, text, layout)], cx);
            assert_eq!(TextSelection::selected_text(window, cx), "alpha beta");
        });
    }
}
