use crate::TestSupportExt as _;
use std::rc::Rc;

use chrono::{NaiveTime, Timelike as _};
use gpui::{
    AnyElement, App, Context, ElementId, Empty, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyBinding, KeyDownEvent, MouseButton, MouseDownEvent,
    ParentElement, Render, RenderOnce, Role, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

use crate::{
    Decrement, Increment, StyledExt as _,
    actions::{SelectLeft, SelectNextColumn, SelectPrevColumn, SelectRight},
    h_flex,
    input::Delete,
};

const CONTEXT: &str = "TimeField";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", Increment, Some(CONTEXT)),
        KeyBinding::new("down", Decrement, Some(CONTEXT)),
        KeyBinding::new("left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(CONTEXT)),
        KeyBinding::new("tab", SelectNextColumn, Some(CONTEXT)),
        KeyBinding::new("shift-tab", SelectPrevColumn, Some(CONTEXT)),
        KeyBinding::new("backspace", Delete, Some(CONTEXT)),
        KeyBinding::new("delete", Delete, Some(CONTEXT)),
    ]);
}

/// The smallest unit a [`TimeField`] edits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimePrecision {
    /// Hours and minutes, e.g. `09:30`.
    #[default]
    Minute,
    /// Hours, minutes and seconds, e.g. `09:30:15`.
    Second,
}

impl TimePrecision {
    /// The segments shown for this precision, in reading order.
    pub fn segments(self) -> &'static [TimeSegment] {
        match self {
            Self::Minute => &[TimeSegment::Hour, TimeSegment::Minute],
            Self::Second => &[TimeSegment::Hour, TimeSegment::Minute, TimeSegment::Second],
        }
    }

    /// The `chrono` format that displays a time at this precision.
    pub fn format(self) -> &'static str {
        match self {
            Self::Minute => "%H:%M",
            Self::Second => "%H:%M:%S",
        }
    }

    /// Drop the components finer than this precision.
    pub fn truncate(self, time: NaiveTime) -> NaiveTime {
        let second = match self {
            Self::Minute => 0,
            Self::Second => time.second(),
        };
        NaiveTime::from_hms_opt(time.hour(), time.minute(), second).unwrap_or(time)
    }
}

/// One editable component of a [`TimeField`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimeSegment {
    Hour,
    Minute,
    Second,
}

impl TimeSegment {
    fn max(self) -> u32 {
        match self {
            Self::Hour => 23,
            Self::Minute | Self::Second => 59,
        }
    }

    fn value(self, time: NaiveTime) -> u32 {
        match self {
            Self::Hour => time.hour(),
            Self::Minute => time.minute(),
            Self::Second => time.second(),
        }
    }

    fn with_value(self, time: NaiveTime, value: u32) -> NaiveTime {
        match self {
            Self::Hour => time.with_hour(value),
            Self::Minute => time.with_minute(value),
            Self::Second => time.with_second(value),
        }
        .unwrap_or(time)
    }

    fn name(self) -> &'static str {
        match self {
            Self::Hour => "hour",
            Self::Minute => "minute",
            Self::Second => "second",
        }
    }
}

/// A semantic notification from a [`TimeFieldState`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeFieldEvent {
    /// Keyboard editing changed the time.
    Change(NaiveTime),
}

/// The pure editing rules of a [`TimeFieldState`], kept apart from focus so
/// they can be tested without a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SegmentEditor {
    time: NaiveTime,
    precision: TimePrecision,
    segment: TimeSegment,
    /// The first digit typed into the selected segment, awaiting a second.
    pending_digit: Option<u32>,
}

impl SegmentEditor {
    fn new(precision: TimePrecision) -> Self {
        Self {
            time: NaiveTime::MIN,
            precision,
            segment: TimeSegment::Hour,
            pending_digit: None,
        }
    }

    fn set_precision(&mut self, precision: TimePrecision) {
        self.precision = precision;
        self.time = precision.truncate(self.time);
        if !precision.segments().contains(&self.segment) {
            self.segment = TimeSegment::Hour;
        }
        self.pending_digit = None;
    }

    fn set_time(&mut self, time: NaiveTime) -> bool {
        let time = self.precision.truncate(time);
        if self.time == time {
            // Keep a half-typed segment when the owner echoes the same value back.
            return false;
        }
        self.time = time;
        self.pending_digit = None;
        true
    }

    fn select_segment(&mut self, segment: TimeSegment) -> bool {
        if !self.precision.segments().contains(&segment) {
            return false;
        }
        self.segment = segment;
        self.pending_digit = None;
        true
    }

    /// Move the selected segment by `offset`; returns `false` at either end.
    fn move_segment(&mut self, offset: isize) -> bool {
        let segments = self.precision.segments();
        let ix = segments
            .iter()
            .position(|segment| *segment == self.segment)
            .unwrap_or(0);
        let Some(next) = ix
            .checked_add_signed(offset)
            .and_then(|ix| segments.get(ix))
        else {
            return false;
        };
        self.select_segment(*next)
    }

    /// Step the selected segment by `delta`, wrapping within the segment.
    fn step(&mut self, delta: i32) -> bool {
        self.pending_digit = None;
        let modulus = self.segment.max() as i32 + 1;
        let value = (self.segment.value(self.time) as i32 + delta).rem_euclid(modulus) as u32;
        self.replace_segment(value)
    }

    fn input_digit(&mut self, digit: u32) -> bool {
        let max = self.segment.max();
        let (value, complete) = match self.pending_digit.take() {
            Some(first) if first * 10 + digit <= max => (first * 10 + digit, true),
            // The two digits cannot form a valid value, so the new digit starts over.
            _ => (digit, digit * 10 > max),
        };
        if !complete {
            self.pending_digit = Some(digit);
        }
        let changed = self.replace_segment(value);
        if complete {
            self.move_segment(1);
        }
        changed
    }

    fn clear_segment(&mut self) -> bool {
        self.pending_digit = None;
        self.replace_segment(0)
    }

    fn replace_segment(&mut self, value: u32) -> bool {
        let time = self.segment.with_value(self.time, value);
        let changed = time != self.time;
        self.time = time;
        changed
    }
}

/// Retained editing behavior for a segmented time field.
///
/// The field is one Tab stop. Inside it, one segment is selected at a time:
/// Up/Down step the selected segment and wrap within it (no carry into the
/// next unit), Left/Right and Tab/Shift-Tab move between segments, digits are
/// typed with a two-digit buffer that advances to the next segment once no
/// further digit could fit, and Backspace/Delete reset the segment to zero.
///
/// The owner supplies the value with [`TimeFieldState::set_time`], which does
/// not emit; user edits emit [`TimeFieldEvent::Change`].
pub struct TimeFieldState {
    focus_handle: FocusHandle,
    editor: SegmentEditor,
}

impl TimeFieldState {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle().tab_stop(true),
            editor: SegmentEditor::new(TimePrecision::default()),
        }
    }

    /// Set the precision, default: [`TimePrecision::Minute`].
    pub fn precision(mut self, precision: TimePrecision) -> Self {
        self.editor.set_precision(precision);
        self
    }

    pub fn set_precision(
        &mut self,
        precision: TimePrecision,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editor.precision != precision {
            self.editor.set_precision(precision);
            cx.notify();
        }
    }

    pub fn time(&self) -> NaiveTime {
        self.editor.time
    }

    /// Replace the time without emitting [`TimeFieldEvent::Change`].
    pub fn set_time(&mut self, time: NaiveTime, _: &mut Window, cx: &mut Context<Self>) {
        if self.editor.set_time(time) {
            cx.notify();
        }
    }

    /// The segment that keyboard editing applies to.
    pub fn segment(&self) -> TimeSegment {
        self.editor.segment
    }

    fn edit(&mut self, cx: &mut Context<Self>, edit: impl FnOnce(&mut SegmentEditor) -> bool) {
        if edit(&mut self.editor) {
            cx.emit(TimeFieldEvent::Change(self.editor.time));
        }
        cx.notify();
    }

    fn on_increment(&mut self, _: &Increment, _: &mut Window, cx: &mut Context<Self>) {
        self.edit(cx, |editor| editor.step(1));
    }

    fn on_decrement(&mut self, _: &Decrement, _: &mut Window, cx: &mut Context<Self>) {
        self.edit(cx, |editor| editor.step(-1));
    }

    fn on_delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        self.edit(cx, SegmentEditor::clear_segment);
    }

    fn on_select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.editor.move_segment(-1);
        cx.notify();
    }

    fn on_select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.editor.move_segment(1);
        cx.notify();
    }

    // Tab walks the segments first and leaves the field only from the last one.
    fn on_next_column(&mut self, _: &SelectNextColumn, _: &mut Window, cx: &mut Context<Self>) {
        if self.editor.move_segment(1) {
            cx.notify();
        } else {
            cx.propagate();
        }
    }

    fn on_prev_column(&mut self, _: &SelectPrevColumn, _: &mut Window, cx: &mut Context<Self>) {
        if self.editor.move_segment(-1) {
            cx.notify();
        } else {
            cx.propagate();
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.modified() || keystroke.key.chars().count() != 1 {
            return;
        }
        let Some(digit) = keystroke.key.chars().next().and_then(|ch| ch.to_digit(10)) else {
            return;
        };
        window.prevent_default();
        cx.stop_propagation();
        self.edit(cx, |editor| editor.input_digit(digit));
    }

    fn select_segment(&mut self, segment: TimeSegment, cx: &mut Context<Self>) {
        self.editor.select_segment(segment);
        cx.notify();
    }
}

impl Focusable for TimeFieldState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<TimeFieldEvent> for TimeFieldState {}
impl Render for TimeFieldState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// State exposed to a [`TimeField`] segment slot for decoration.
#[derive(Clone, Copy, Debug)]
pub struct TimeSegmentState {
    segment: TimeSegment,
    selected: bool,
    disabled: bool,
}

impl TimeSegmentState {
    pub fn segment(&self) -> TimeSegment {
        self.segment
    }

    /// Whether keyboard editing applies to this segment: it is the selected
    /// segment and the field has focus.
    pub fn is_selected(&self) -> bool {
        self.selected
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

type SegmentRenderer =
    Rc<dyn Fn(TimeFieldSegment, TimeSegmentState, &mut Window, &mut App) -> AnyElement>;

/// An unstyled, pre-wired segment passed to the [`TimeField`] segment slot.
#[derive(IntoElement)]
pub struct TimeFieldSegment {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl ParentElement for TimeFieldSegment {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}
impl Styled for TimeFieldSegment {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl InteractiveElement for TimeFieldSegment {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}
impl StatefulInteractiveElement for TimeFieldSegment {}
impl RenderOnce for TimeFieldSegment {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base.children(self.children).refine_style(&self.style)
    }
}

/// Unstyled segmented time editor.
///
/// Base owns focus, the keyboard model and pointer segment selection; the
/// presentation decorates the field with `Styled` and each segment through
/// [`TimeField::segment`].
#[derive(IntoElement)]
pub struct TimeField {
    id: ElementId,
    state: Entity<TimeFieldState>,
    disabled: bool,
    style: StyleRefinement,
    segment: SegmentRenderer,
}

impl TimeField {
    pub fn new(id: impl Into<ElementId>, state: &Entity<TimeFieldState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            disabled: false,
            style: StyleRefinement::default(),
            segment: Rc::new(|segment, _, _, _| segment.into_any_element()),
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Decorate each segment. The segment already carries its two-digit label.
    pub fn segment(
        mut self,
        render: impl Fn(TimeFieldSegment, TimeSegmentState, &mut Window, &mut App) -> AnyElement
        + 'static,
    ) -> Self {
        self.segment = Rc::new(render);
        self
    }
}

impl Styled for TimeField {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TimeField {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let disabled = self.disabled;
        let state = self.state.read(cx);
        let focus_handle = state.focus_handle.clone();
        let focused = focus_handle.is_focused(window);
        let editor = state.editor;

        let mut root = h_flex()
            .id(self.id.clone())
            .test_support()
            .role(Role::TimeInput)
            .aria_value(SharedString::from(
                editor.time.format(editor.precision.format()).to_string(),
            ))
            .track_focus(&focus_handle.clone().tab_stop(!disabled))
            .when(!disabled, |this| {
                this.key_context(CONTEXT)
                    .on_action(window.listener_for(&self.state, TimeFieldState::on_increment))
                    .on_action(window.listener_for(&self.state, TimeFieldState::on_decrement))
                    .on_action(window.listener_for(&self.state, TimeFieldState::on_delete))
                    .on_action(window.listener_for(&self.state, TimeFieldState::on_select_left))
                    .on_action(window.listener_for(&self.state, TimeFieldState::on_select_right))
                    .on_action(window.listener_for(&self.state, TimeFieldState::on_next_column))
                    .on_action(window.listener_for(&self.state, TimeFieldState::on_prev_column))
                    .on_key_down(window.listener_for(&self.state, TimeFieldState::on_key_down))
            });

        for (ix, segment) in editor.precision.segments().iter().copied().enumerate() {
            if ix > 0 {
                root = root.child(div().child(":"));
            }
            let segment_state = TimeSegmentState {
                segment,
                selected: focused && segment == editor.segment,
                disabled,
            };
            let mut item = TimeFieldSegment {
                base: div().id(segment.name()).test_support(),
                style: StyleRefinement::default(),
                children: vec![],
            }
            .child(format!("{:02}", segment.value(editor.time)));
            if !disabled {
                let state = self.state.clone();
                let focus_handle = focus_handle.clone();
                item =
                    item.on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, window, cx| {
                        focus_handle.focus(window, cx);
                        state.update(cx, |state, cx| state.select_segment(segment, cx));
                    });
            }
            root = root.child((self.segment)(item, segment_state, window, cx));
        }

        root.refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hms(h: u32, m: u32, s: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, s).unwrap()
    }

    fn type_digits(editor: &mut SegmentEditor, digits: &str) {
        for ch in digits.chars() {
            editor.input_digit(ch.to_digit(10).unwrap());
        }
    }

    #[test]
    fn typing_fills_segments_and_advances() {
        let mut editor = SegmentEditor::new(TimePrecision::Second);
        type_digits(&mut editor, "093015");
        assert_eq!(editor.time, hms(9, 30, 15));
        // The last segment stays selected once it is complete.
        assert_eq!(editor.segment, TimeSegment::Second);
    }

    #[test]
    fn a_digit_that_cannot_start_two_digits_completes_the_segment() {
        let mut editor = SegmentEditor::new(TimePrecision::Minute);
        type_digits(&mut editor, "7");
        assert_eq!(editor.time, hms(7, 0, 0));
        assert_eq!(editor.segment, TimeSegment::Minute);
        type_digits(&mut editor, "8");
        assert_eq!(editor.time, hms(7, 8, 0));
    }

    #[test]
    fn an_out_of_range_pair_restarts_from_the_second_digit() {
        let mut editor = SegmentEditor::new(TimePrecision::Minute);
        // 2 then 5 cannot form 25 hours, so 5 starts a new entry and completes it.
        type_digits(&mut editor, "25");
        assert_eq!(editor.time, hms(5, 0, 0));
        assert_eq!(editor.segment, TimeSegment::Minute);
    }

    #[test]
    fn stepping_wraps_without_carry() {
        let mut editor = SegmentEditor::new(TimePrecision::Minute);
        editor.set_time(hms(23, 59, 0));
        editor.step(1);
        assert_eq!(editor.time, hms(0, 59, 0));
        editor.select_segment(TimeSegment::Minute);
        editor.step(1);
        assert_eq!(editor.time, hms(0, 0, 0));
        editor.step(-1);
        assert_eq!(editor.time, hms(0, 59, 0));
    }

    #[test]
    fn segment_movement_stops_at_the_ends() {
        let mut editor = SegmentEditor::new(TimePrecision::Minute);
        assert!(!editor.move_segment(-1));
        assert!(editor.move_segment(1));
        assert!(!editor.move_segment(1));
        assert!(!editor.select_segment(TimeSegment::Second));
    }

    #[test]
    fn precision_truncates_seconds() {
        let mut editor = SegmentEditor::new(TimePrecision::Second);
        editor.set_time(hms(9, 30, 15));
        editor.select_segment(TimeSegment::Second);
        editor.set_precision(TimePrecision::Minute);
        assert_eq!(editor.time, hms(9, 30, 0));
        assert_eq!(editor.segment, TimeSegment::Hour);
    }

    #[test]
    fn clearing_resets_only_the_selected_segment() {
        let mut editor = SegmentEditor::new(TimePrecision::Minute);
        editor.set_time(hms(9, 30, 0));
        editor.select_segment(TimeSegment::Minute);
        assert!(editor.clear_segment());
        assert_eq!(editor.time, hms(9, 0, 0));
    }
}
