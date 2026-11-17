use crate::TestSupportExt as _;
use std::ops::Range;

use gpui::{
    AccessibleAction, Along, AnyElement, App, AppContext as _, Axis, Bounds, Context, Div,
    DragMoveEvent, Empty, Entity, EntityId, EventEmitter, FocusHandle, HitboxBehavior,
    InteractiveElement, IntoElement, KeyDownEvent, Keystroke, MouseButton, MouseDownEvent,
    Orientation, ParentElement, Pixels, Point, Render, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, TouchDragEvent, TouchPhase, Window,
    accesskit::ActionData, div, prelude::FluentBuilder as _, px,
};

use crate::{
    LayoutDirection, a11y_disabled::A11yDisabledExt as _, element_ext::ElementExt,
    geometry::AxisExt,
};

/// Events emitted by the [`SliderState`].
pub enum SliderEvent {
    /// Emitted continuously while the slider value is being changed by the user.
    Change(SliderValue),
    /// Emitted once when the user releases the slider after a drag or click, and after
    /// each step from the keyboard or an assistive technology.
    Release(SliderValue),
}

/// The value of the slider, can be a single value or a range of values.
///
/// - Can from a f32 value, which will be treated as a single value.
/// - Or from a (f32, f32) tuple, which will be treated as a range of values.
///
/// The default value is `SliderValue::Single(0.0)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SliderValue {
    Single(f32),
    Range(f32, f32),
}

impl std::fmt::Display for SliderValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SliderValue::Single(value) => write!(f, "{}", value),
            SliderValue::Range(start, end) => write!(f, "{}..{}", start, end),
        }
    }
}

impl From<f32> for SliderValue {
    fn from(value: f32) -> Self {
        SliderValue::Single(value)
    }
}

impl From<(f32, f32)> for SliderValue {
    fn from(value: (f32, f32)) -> Self {
        SliderValue::Range(value.0, value.1)
    }
}

impl From<Range<f32>> for SliderValue {
    fn from(value: Range<f32>) -> Self {
        SliderValue::Range(value.start, value.end)
    }
}

impl Default for SliderValue {
    fn default() -> Self {
        SliderValue::Single(0.)
    }
}

impl SliderValue {
    /// Clamp the value to the given range.
    pub fn clamp(self, min: f32, max: f32) -> Self {
        match self {
            SliderValue::Single(value) => SliderValue::Single(value.clamp(min, max)),
            SliderValue::Range(start, end) => {
                SliderValue::Range(start.clamp(min, max), end.clamp(min, max))
            }
        }
    }

    /// Check if the value is a single value.
    #[inline]
    pub fn is_single(&self) -> bool {
        matches!(self, SliderValue::Single(_))
    }

    /// Check if the value is a range of values.
    #[inline]
    pub fn is_range(&self) -> bool {
        matches!(self, SliderValue::Range(_, _))
    }

    /// Get the start value.
    pub fn start(&self) -> f32 {
        match self {
            SliderValue::Single(value) => *value,
            SliderValue::Range(start, _) => *start,
        }
    }

    /// Get the end value.
    pub fn end(&self) -> f32 {
        match self {
            SliderValue::Single(value) => *value,
            SliderValue::Range(_, end) => *end,
        }
    }

    fn set_start(&mut self, value: f32) {
        if let SliderValue::Range(_, end) = self {
            *self = SliderValue::Range(value.min(*end), *end);
        } else {
            *self = SliderValue::Single(value);
        }
    }

    fn set_end(&mut self, value: f32) {
        if let SliderValue::Range(start, _) = self {
            *self = SliderValue::Range(*start, value.max(*start));
        } else {
            *self = SliderValue::Single(value);
        }
    }
}

/// The scale mode of the slider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SliderScale {
    /// Linear scale where values change uniformly across the slider range.
    /// This is the default mode.
    #[default]
    Linear,
    /// Logarithmic scale where the distance between values increases exponentially.
    ///
    /// This is useful for parameters that have a large range of values where smaller
    /// changes are more significant at lower values. Common examples include:
    ///
    /// - Volume controls (human hearing perception is logarithmic)
    /// - Frequency controls (musical notes follow a logarithmic scale)
    /// - Zoom levels
    /// - Any parameter where you want finer control at lower values
    ///
    /// # For example
    ///
    /// ```
    /// use gpui_base::slider::{SliderScale, SliderState};
    ///
    /// let slider = SliderState::new()
    ///     .min(1.0)    // Must be > 0 for logarithmic scale
    ///     .max(1000.0)
    ///     .scale(SliderScale::Logarithmic);
    /// ```
    ///
    /// - Moving the slider 1/3 of the way will yield ~10
    /// - Moving it 2/3 of the way will yield ~100
    /// - The full range covers 3 orders of magnitude evenly
    Logarithmic,
}

impl SliderScale {
    #[inline]
    pub fn is_linear(&self) -> bool {
        matches!(self, SliderScale::Linear)
    }

    #[inline]
    pub fn is_logarithmic(&self) -> bool {
        matches!(self, SliderScale::Logarithmic)
    }
}

/// State of the [`Slider`].
pub struct SliderState {
    min: f32,
    max: f32,
    step: f32,
    value: SliderValue,
    /// When is single value mode, only `end` is used, the start is always 0.0.
    percentage: Range<f32>,
    /// The bounds of the slider after rendered.
    bounds: Bounds<Pixels>,
    scale: SliderScale,
    /// Tracks whether the user is currently interacting with the slider so we
    /// only emit [`SliderEvent::Release`] after a real press/drag.
    dragging: bool,
    /// The touch drag this slider claimed, while one is live.
    touch_drag: Option<TouchDrag>,
}

/// A touch drag claimed by a slider.
#[derive(Clone, Copy)]
struct TouchDrag {
    /// Identifies the gesture: every event of one drag carries the same start.
    start_position: Point<Pixels>,
    /// Whether it moves the range start thumb.
    is_start: bool,
}

impl SliderState {
    /// Create a new [`SliderState`].
    pub fn new() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            value: SliderValue::default(),
            percentage: (0.0..0.0),
            bounds: Bounds::default(),
            scale: SliderScale::default(),
            dragging: false,
            touch_drag: None,
        }
    }

    /// Set the minimum value of the slider, default: 0.0
    pub fn min(mut self, min: f32) -> Self {
        if self.scale.is_logarithmic() {
            assert!(
                min > 0.0,
                "`min` must be greater than 0 for SliderScale::Logarithmic"
            );
            assert!(
                min < self.max,
                "`min` must be less than `max` for Logarithmic scale"
            );
        }
        self.min = min;
        self.update_thumb_pos();
        self
    }

    /// Set the maximum value of the slider, default: 100.0
    pub fn max(mut self, max: f32) -> Self {
        if self.scale.is_logarithmic() {
            assert!(
                max > self.min,
                "`max` must be greater than `min` for Logarithmic scale"
            );
        }
        self.max = max;
        self.update_thumb_pos();
        self
    }

    /// Set the step value of the slider, default: 1.0
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Set the scale of the slider, default: [`SliderScale::Linear`].
    pub fn scale(mut self, scale: SliderScale) -> Self {
        if scale.is_logarithmic() {
            assert!(
                self.min > 0.0,
                "`min` must be greater than 0 for Logarithmic scale"
            );
            assert!(
                self.max > self.min,
                "`max` must be greater than `min` for Logarithmic scale"
            );
        }
        self.scale = scale;
        self.update_thumb_pos();
        self
    }

    /// Set the default value of the slider, default: 0.0
    pub fn default_value(mut self, value: impl Into<SliderValue>) -> Self {
        self.value = value.into();
        self.update_thumb_pos();
        self
    }

    /// Set the value of the slider.
    pub fn set_value(
        &mut self,
        value: impl Into<SliderValue>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.value = value.into();
        self.update_thumb_pos();
        cx.notify();
    }

    /// Get the value of the slider.
    pub fn value(&self) -> SliderValue {
        self.value
    }

    /// Get the minimum value.
    pub fn min_value(&self) -> f32 {
        self.min
    }

    /// Get the maximum value.
    pub fn max_value(&self) -> f32 {
        self.max
    }

    /// Get the step value.
    pub fn step_value(&self) -> f32 {
        self.step
    }

    /// The amount an assistive technology's Increment and Decrement move a thumb, and the
    /// step reported to it: the step, or a hundredth of the range for a step that is not a
    /// positive finite number (a continuous slider).
    pub fn accessibility_step(&self) -> f32 {
        if is_step(self.step) {
            self.step
        } else {
            (self.max - self.min) / 100.
        }
    }

    /// Converts a value between 0.0 and 1.0 to a value between the minimum and maximum value,
    /// depending on the chosen scale.
    fn percentage_to_value(&self, percentage: f32) -> f32 {
        match self.scale {
            SliderScale::Linear => self.min + (self.max - self.min) * percentage,
            SliderScale::Logarithmic => {
                // when percentage is 0, this simplifies to (max/min)^0 * min = 1 * min = min
                // when percentage is 1, this simplifies to (max/min)^1 * min = (max*min)/min = max
                // we clamp just to make sure we don't have issue with floating point precision
                let base = self.max / self.min;
                (base.powf(percentage) * self.min).clamp(self.min, self.max)
            }
        }
    }

    /// Converts a value between the minimum and maximum value to a value between 0.0 and 1.0,
    /// depending on the chosen scale.
    fn value_to_percentage(&self, value: f32) -> f32 {
        match self.scale {
            SliderScale::Linear => {
                let range = self.max - self.min;
                if range <= 0.0 {
                    0.0
                } else {
                    (value - self.min) / range
                }
            }
            SliderScale::Logarithmic => {
                let base = self.max / self.min;
                (value / self.min).log(base).clamp(0.0, 1.0)
            }
        }
    }

    fn update_thumb_pos(&mut self) {
        match self.value {
            SliderValue::Single(value) => {
                let percentage = self.value_to_percentage(value.min(self.max).max(self.min));
                self.percentage = 0.0..percentage;
            }
            SliderValue::Range(start, end) => {
                // Not `clamp`: while a builder is between its `min` and `max`, `min`
                // can stand above `max`.
                let clamped_start = start.min(self.max).max(self.min);
                let clamped_end = end.min(self.max).max(self.min);
                self.percentage =
                    self.value_to_percentage(clamped_start)..self.value_to_percentage(clamped_end);
            }
        }
    }

    /// Update value by mouse position
    #[doc(hidden)]
    pub fn update_value_by_position(
        &mut self,
        axis: Axis,
        position: Point<Pixels>,
        is_start: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dragging = true;
        let bounds = self.bounds;
        let step = self.step;

        let inner_pos = self.offset_along(axis, position, cx);
        let total_size = bounds.size.along(axis);
        let percentage = inner_pos.clamp(px(0.), total_size) / total_size;

        let percentage = if is_start {
            percentage.clamp(0.0, self.percentage.end)
        } else {
            percentage.clamp(self.percentage.start, 1.0)
        };

        let value = self.percentage_to_value(percentage);
        let value = snap_to_step(value, self.min, self.max, step);

        if is_start {
            self.value.set_start(value);
        } else {
            self.value.set_end(value);
        }
        // The thumb stands at the value, on its step, not at the pointer.
        self.update_thumb_pos();
        cx.emit(SliderEvent::Change(self.value));
        cx.notify();
    }

    /// Moves the end thumb or, with `start`, a range's start thumb by `delta`, as a key and
    /// an assistive technology's Increment and Decrement do, and emits
    /// [`SliderEvent::Change`] then [`SliderEvent::Release`] if it moved. The thumb lands
    /// on the step grid counted from `min` and stops at the other thumb and at the
    /// slider's ends.
    #[doc(hidden)]
    pub fn step_thumb(
        &mut self,
        start: bool,
        delta: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // A NaN never equals itself, so it would read as a change below.
        if !steppable(self.value, self.min, self.max) {
            return;
        }
        let value = step_thumb(self.value, start, delta, self.min, self.max, self.step);
        self.commit_thumb_value(value, window, cx);
    }

    fn handle_accessibility_set_value(
        &mut self,
        data: Option<&ActionData>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ActionData::NumericValue(target)) = data else {
            return;
        };
        if !target.is_finite() || !steppable(self.value, self.min, self.max) {
            return;
        }
        let value = thumb_value_at(
            self.value,
            false,
            *target as f32,
            self.min,
            self.max,
            self.step,
        );
        self.commit_thumb_value(value, window, cx);
    }

    fn commit_thumb_value(
        &mut self,
        value: SliderValue,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if value != self.value {
            self.set_value(value, window, cx);
            cx.emit(SliderEvent::Change(value));
            // A keyboard or accessibility adjustment is a complete gesture.
            cx.emit(SliderEvent::Release(value));
        }
    }

    /// Whether `position` is closer to the range start thumb than to the end thumb.
    fn is_nearer_start(&self, axis: Axis, position: Point<Pixels>, cx: &App) -> bool {
        let size = self.bounds.size.along(axis);
        let along = self.offset_along(axis, position, cx);
        let center =
            ((self.percentage.end - self.percentage.start) / 2. + self.percentage.start) * size;
        along < center
    }

    /// How far `position` is from the minimum end: the start edge of a
    /// horizontal slider (the right in RTL) or the bottom of a vertical one.
    fn offset_along(&self, axis: Axis, position: Point<Pixels>, cx: &App) -> Pixels {
        if axis.is_vertical() {
            self.bounds.bottom() - position.y
        } else if LayoutDirection::global(cx).is_rtl() {
            self.bounds.right() - position.x
        } else {
            position.x - self.bounds.left()
        }
    }

    /// Emit [`SliderEvent::Release`] if the user was actively interacting
    /// with the slider. Called on mouse-up both inside and outside the slider.
    #[doc(hidden)]
    pub fn handle_release(&mut self, cx: &mut Context<Self>) {
        if !self.dragging {
            return;
        }
        self.dragging = false;
        cx.emit(SliderEvent::Release(self.value));
    }
}

/// Steps Page Up and Page Down move the thumb.
const PAGE_STEPS: f32 = 10.;

/// How far a key moves the thumb: an arrow `step`, Page Up and Page Down ten of them, Home
/// and End the whole `range`. `None` for any other key and for a key held with a modifier.
fn key_delta(keystroke: &Keystroke, axis: Axis, rtl: bool, step: f32, range: f32) -> Option<f32> {
    if keystroke.modifiers.modified() {
        return None;
    }
    // A horizontal slider grows to the left in a right-to-left layout.
    let toward_end = if axis.is_horizontal() && rtl {
        -step
    } else {
        step
    };
    match keystroke.key.as_str() {
        "right" => Some(toward_end),
        "left" => Some(-toward_end),
        "up" => Some(step),
        "down" => Some(-step),
        "pageup" => Some(step * PAGE_STEPS),
        "pagedown" => Some(-step * PAGE_STEPS),
        "home" => Some(-range),
        "end" => Some(range),
        _ => None,
    }
}

/// Whether `step` divides the range into steps: a positive finite number.
fn is_step(step: f32) -> bool {
    step > 0. && step.is_finite()
}

/// `value` on the nearest step counted from `min`, kept inside the range. A step that is
/// not a positive finite number leaves the value continuous.
fn snap_to_step(value: f32, min: f32, max: f32, step: f32) -> f32 {
    if !is_step(step) {
        return value;
    }
    (min + ((value - min) / step).round() * step)
        .min(max)
        .max(min)
}

/// Whether `value` can be stepped inside `min..=max`: not for a NaN value or bound, a
/// `min` above `max`, or a range whose start is past its end.
fn steppable(value: SliderValue, min: f32, max: f32) -> bool {
    let (start, end) = (value.start(), value.end());
    min <= max && !start.is_nan() && !end.is_nan() && (!value.is_range() || start <= end)
}

/// `value` with its end, or with `start` a range's start, moved by `delta` and snapped to
/// `step` counted from `min`, inside `min..=max` and without passing the other thumb. A
/// value that is not [`steppable`] comes back unchanged.
fn step_thumb(
    value: SliderValue,
    start: bool,
    delta: f32,
    min: f32,
    max: f32,
    step: f32,
) -> SliderValue {
    if !steppable(value, min, max) {
        return value;
    }
    let current = if start && value.is_range() {
        value.start()
    } else {
        value.end()
    };
    thumb_value_at(value, start, current + delta, min, max, step)
}

fn thumb_value_at(
    mut value: SliderValue,
    start: bool,
    target: f32,
    min: f32,
    max: f32,
    step: f32,
) -> SliderValue {
    let next = snap_to_step(target, min, max, step).min(max).max(min);
    if start && value.is_range() {
        value.set_start(next);
    } else {
        value.set_end(next);
    }
    value
}

#[derive(Clone)]
struct DragThumb((EntityId, bool));

impl Render for DragThumb {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone)]
struct DragSlider(EntityId);

impl Render for DragSlider {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// An unstyled slider behavior root.
///
/// Applications provide the track, range, and thumb presentation as children.
#[derive(IntoElement)]
pub struct Slider {
    state: Entity<SliderState>,
    axis: Axis,
    disabled: bool,
    accessibility_label: Option<SharedString>,
    focus: Option<FocusHandle>,
    base: Div,
    children: Vec<AnyElement>,
}

impl Slider {
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            state: state.clone(),
            axis: Axis::Horizontal,
            disabled: false,
            accessibility_label: None,
            focus: None,
            base: div(),
            children: Vec::new(),
        }
    }

    pub fn horizontal(mut self) -> Self {
        self.axis = Axis::Horizontal;
        self
    }

    pub fn vertical(mut self) -> Self {
        self.axis = Axis::Vertical;
        self
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the label exposed to accessibility clients.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Makes the slider's accessibility node itself focusable with `focus`,
    /// so keyboard focus and assistive technology land on the slider. While
    /// it has focus, the arrow keys move the end thumb a step, Page Up and
    /// Page Down ten steps, and Home and End to the slider's ends.
    pub fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.focus = Some(focus.clone());
        self
    }
}

impl ParentElement for Slider {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Slider {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let axis = self.axis;
        let entity_id = self.state.entity_id();
        let state = self.state.read(cx);
        let slider_state = self.state.clone();

        self.base
            .id(("slider", entity_id))
            .test_support()
            .when_some(self.focus, |this, focus| this.track_focus(&focus))
            .role(Role::Slider)
            .when_some(self.accessibility_label, |this, label| {
                this.aria_label(label)
            })
            .aria_numeric_value(state.value().end() as f64)
            .aria_min_numeric_value(state.min_value() as f64)
            .aria_max_numeric_value(state.max_value() as f64)
            .aria_numeric_value_step(state.accessibility_step() as f64)
            .aria_orientation(if axis.is_vertical() {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            })
            // A disabled slider takes no value, from the pointer or from
            // assistive technology.
            .when(!self.disabled, |this| {
                this.on_a11y_action(AccessibleAction::Increment, {
                    let state = slider_state.clone();
                    move |_, window, cx| {
                        state.update(cx, |state, cx| {
                            state.step_thumb(false, state.accessibility_step(), window, cx);
                        });
                    }
                })
                .on_a11y_action(AccessibleAction::Decrement, {
                    let state = slider_state.clone();
                    move |_, window, cx| {
                        state.update(cx, |state, cx| {
                            state.step_thumb(false, -state.accessibility_step(), window, cx);
                        });
                    }
                })
                .on_a11y_action(AccessibleAction::SetValue, {
                    let state = slider_state.clone();
                    move |data, window, cx| {
                        state.update(cx, |state, cx| {
                            state.handle_accessibility_set_value(data, window, cx);
                        });
                    }
                })
                // Keys reach the slider only while its `track_focus` handle has focus.
                .on_key_down({
                    let state = slider_state.clone();
                    move |event: &KeyDownEvent, window, cx| {
                        let rtl = LayoutDirection::global(cx).is_rtl();
                        state.update(cx, |state, cx| {
                            let step = state.accessibility_step();
                            let range = state.max - state.min;
                            let Some(delta) = key_delta(&event.keystroke, axis, rtl, step, range)
                            else {
                                return;
                            };
                            state.step_thumb(false, delta, window, cx);
                            cx.stop_propagation();
                        });
                    }
                })
                .on_mouse_up(
                    MouseButton::Left,
                    window.listener_for(&self.state, |state, _, _, cx| state.handle_release(cx)),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    window.listener_for(&self.state, |state, _, _, cx| state.handle_release(cx)),
                )
            })
            .children(self.children)
            .a11y_disabled(self.disabled)
    }
}

/// An unstyled track that records the geometry used to map pointer positions.
#[derive(IntoElement)]
pub struct SliderTrack {
    state: Entity<SliderState>,
    axis: Axis,
    disabled: bool,
    base: Div,
    children: Vec<AnyElement>,
}

impl SliderTrack {
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            state: state.clone(),
            axis: Axis::Horizontal,
            disabled: false,
            base: div(),
            children: Vec::new(),
        }
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ParentElement for SliderTrack {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for SliderTrack {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for SliderTrack {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for SliderTrack {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let axis = self.axis;
        let entity_id = self.state.entity_id();
        let state = self.state.read(cx);
        let is_range = state.value().is_range();
        // Touch: GPUI routes a finger drag to `TouchDragEvent` (claimed with
        // `prevent_default`) or to scrolling; `on_drag` below is mouse-only. Claim
        // drags that start on the track, as the scrollbar thumb does.
        let touch_layer = (!self.disabled).then(|| {
            let slider_state = self.state.clone();
            gpui::canvas(
                |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                move |_, hitbox, window, _| {
                    let slider_state = slider_state.clone();
                    window.on_mouse_event(move |event: &TouchDragEvent, phase, window, cx| {
                        if !phase.bubble() {
                            return;
                        }
                        slider_state.update(cx, |state, cx| {
                            if event.phase == TouchPhase::Started {
                                // A drag whose end never arrived (the slider was
                                // disabled or not painted) ends when another begins.
                                if state.touch_drag.take().is_some() {
                                    state.handle_release(cx);
                                }
                                // The hitbox, not the bounds, so a layer covering
                                // the track keeps its own drags.
                                if window.default_prevented() || !hitbox.is_hovered(window) {
                                    return;
                                }
                                window.prevent_default();
                                let is_start = is_range
                                    && state.is_nearer_start(axis, event.start_position, cx);
                                state.touch_drag = Some(TouchDrag {
                                    start_position: event.start_position,
                                    is_start,
                                });
                            }
                            let Some(drag) = state.touch_drag else {
                                return;
                            };
                            if drag.start_position != event.start_position {
                                return;
                            }
                            cx.stop_propagation();
                            match event.phase {
                                TouchPhase::Started | TouchPhase::Moved => {
                                    state.update_value_by_position(
                                        axis,
                                        event.position,
                                        drag.is_start,
                                        window,
                                        cx,
                                    );
                                }
                                TouchPhase::Ended | TouchPhase::Cancelled => {
                                    state.touch_drag = None;
                                    state.handle_release(cx);
                                }
                            }
                        });
                    });
                },
            )
            .absolute()
            .size_full()
        });
        self.base
            .id("slider-bar-container")
            .test_support()
            .children(self.children)
            .children(touch_layer)
            .when(!self.disabled, |this| {
                this.on_mouse_down(
                    MouseButton::Left,
                    window.listener_for(
                        &self.state,
                        move |state, event: &MouseDownEvent, window, cx| {
                            let is_start =
                                is_range && state.is_nearer_start(axis, event.position, cx);
                            state.update_value_by_position(
                                axis,
                                event.position,
                                is_start,
                                window,
                                cx,
                            );
                        },
                    ),
                )
                .when(!is_range, |this| {
                    this.on_drag(DragSlider(entity_id), |drag, _, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| drag.clone())
                    })
                    .on_drag_move(window.listener_for(
                        &self.state,
                        move |state, event: &DragMoveEvent<DragSlider>, window, cx| {
                            let DragSlider(id) = event.drag(cx);
                            if *id == entity_id {
                                state.update_value_by_position(
                                    axis,
                                    event.event.position,
                                    false,
                                    window,
                                    cx,
                                );
                            }
                        },
                    ))
                })
            })
    }
}

/// An unstyled slider indicator that records the value-mapping bounds.
#[derive(IntoElement)]
pub struct SliderIndicator {
    state: Entity<SliderState>,
    base: Div,
    children: Vec<AnyElement>,
}

impl SliderIndicator {
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            state: state.clone(),
            base: div(),
            children: Vec::new(),
        }
    }
}

impl ParentElement for SliderIndicator {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for SliderIndicator {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for SliderIndicator {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for SliderIndicator {}

impl RenderOnce for SliderIndicator {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .id("slider-bar")
            .children(self.children)
            .on_prepaint({
                let state = self.state;
                // Only when they moved: with retained views, an update while
                // the window draws renders every view that reads the state
                // again.
                move |bounds, _, cx| {
                    if state.read(cx).bounds != bounds {
                        state.update(cx, |state, _| state.set_bounds(bounds));
                    }
                }
            })
    }
}

/// An unstyled draggable slider thumb.
#[derive(IntoElement)]
pub struct SliderThumb {
    state: Entity<SliderState>,
    axis: Axis,
    start: bool,
    disabled: bool,
    base: Div,
    children: Vec<AnyElement>,
}

impl SliderThumb {
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            state: state.clone(),
            axis: Axis::Horizontal,
            start: false,
            disabled: false,
            base: div(),
            children: Vec::new(),
        }
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    pub fn start(mut self, start: bool) -> Self {
        self.start = start;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ParentElement for SliderThumb {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for SliderThumb {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for SliderThumb {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for SliderThumb {}

impl RenderOnce for SliderThumb {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        let entity_id = self.state.entity_id();
        let axis = self.axis;
        let start = self.start;
        self.base
            .id(("slider-thumb", start as u32))
            .test_support()
            .children(self.children)
            .when(!self.disabled, |this| {
                this.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_drag(DragThumb((entity_id, start)), |drag, _, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| drag.clone())
                    })
                    .on_drag_move(window.listener_for(
                        &self.state,
                        move |state, event: &DragMoveEvent<DragThumb>, window, cx| {
                            let DragThumb((id, start)) = event.drag(cx);
                            if *id == entity_id {
                                state.update_value_by_position(
                                    axis,
                                    event.event.position,
                                    *start,
                                    window,
                                    cx,
                                );
                            }
                        },
                    ))
            })
            // A caller can give a thumb a role of its own, as for a range's thumbs.
            .a11y_disabled(self.disabled)
    }
}

impl EventEmitter<SliderEvent> for SliderState {}

impl SliderState {
    #[doc(hidden)]
    pub fn percentage(&self) -> Range<f32> {
        self.percentage.clone()
    }

    #[doc(hidden)]
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    #[doc(hidden)]
    pub fn set_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.bounds = bounds;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_value_conversions_and_clamping_are_preserved() {
        assert_eq!(SliderValue::from(5.), SliderValue::Single(5.));
        assert_eq!(SliderValue::from((2., 8.)), SliderValue::Range(2., 8.));
        assert_eq!(SliderValue::from(2.0..8.0), SliderValue::Range(2., 8.));
        assert_eq!(
            SliderValue::Range(-1., 12.).clamp(0., 10.),
            SliderValue::Range(0., 10.)
        );
    }

    #[test]
    fn legacy_linear_state_keeps_percentage_and_range_ordering() {
        let state = SliderState::new()
            .min(0.)
            .max(200.)
            .default_value((50., 150.));
        assert_eq!(state.value(), SliderValue::Range(50., 150.));
        assert_eq!(state.percentage(), 0.25..0.75);
    }

    #[test]
    fn legacy_logarithmic_state_keeps_mapping() {
        let state = SliderState::new()
            .min(1.)
            .max(1000.)
            .scale(SliderScale::Logarithmic)
            .default_value(10.);
        let percentage = state.percentage().end;
        assert!((percentage - (1. / 3.)).abs() < 0.0001);
    }

    #[gpui::test]
    fn a_disabled_thumb_marks_its_node_disabled(cx: &mut gpui::TestAppContext) {
        struct Thumb {
            state: Entity<SliderState>,
            disabled: bool,
        }

        impl Render for Thumb {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                SliderThumb::new(&self.state)
                    .role(Role::Slider)
                    .disabled(self.disabled)
            }
        }

        for disabled in [false, true] {
            let (_, cx) = cx.add_window_view(|_, cx| Thumb {
                state: cx.new(|_| SliderState::new()),
                disabled,
            });
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                let thumb = crate::test_support::snapshots(window)
                    .into_iter()
                    .find(|node| node.role() == Some(Role::Slider))
                    .expect("a thumb node");
                assert_eq!(thumb.disabled(), disabled.then_some(true));
            });
        }
    }

    #[gpui::test]
    fn a_slider_takes_focus_on_its_own_node(cx: &mut gpui::TestAppContext) {
        struct Focusable {
            state: Entity<SliderState>,
            focus: FocusHandle,
        }

        impl Render for Focusable {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                Slider::new(&self.state).track_focus(&self.focus)
            }
        }

        let (view, cx) = cx.add_window_view(|_, cx| Focusable {
            state: cx.new(|_| SliderState::new()),
            focus: cx.focus_handle(),
        });
        cx.update(|window, cx| {
            let focus = view.read(cx).focus.clone();
            focus.focus(window, cx);
            window.draw(cx).clear(cx);
            let slider = crate::test_support::snapshots(window)
                .into_iter()
                .find(|node| node.role() == Some(Role::Slider))
                .expect("a slider node");
            assert_eq!(slider.focused(), Some(true));
        });
    }

    #[test]
    fn keys_map_to_steps() {
        let key = |key: &'static str, axis, rtl| {
            key_delta(&Keystroke::parse(key).unwrap(), axis, rtl, 5., 100.)
        };
        for axis in [Axis::Horizontal, Axis::Vertical] {
            assert_eq!(key("right", axis, false), Some(5.));
            assert_eq!(key("left", axis, false), Some(-5.));
            assert_eq!(key("up", axis, false), Some(5.));
            assert_eq!(key("down", axis, false), Some(-5.));
            assert_eq!(key("pageup", axis, false), Some(50.));
            assert_eq!(key("pagedown", axis, false), Some(-50.));
            assert_eq!(key("home", axis, false), Some(-100.));
            assert_eq!(key("end", axis, false), Some(100.));
            assert_eq!(key("a", axis, false), None);
            assert_eq!(key("ctrl-right", axis, false), None);
            assert_eq!(key("shift-home", axis, false), None);
        }
        // Right to left turns only a horizontal slider's left and right.
        assert_eq!(key("right", Axis::Horizontal, true), Some(-5.));
        assert_eq!(key("left", Axis::Horizontal, true), Some(5.));
        assert_eq!(key("up", Axis::Horizontal, true), Some(5.));
        assert_eq!(key("right", Axis::Vertical, true), Some(5.));
    }

    #[gpui::test]
    fn a_focused_slider_steps_on_keys_and_reports_each_step(cx: &mut gpui::TestAppContext) {
        struct Focusable {
            state: Entity<SliderState>,
            focus: FocusHandle,
            disabled: bool,
        }

        impl Render for Focusable {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                Slider::new(&self.state)
                    .track_focus(&self.focus)
                    .disabled(self.disabled)
            }
        }

        let (view, cx) = cx.add_window_view(|_, cx| Focusable {
            state: cx.new(|_| SliderState::new().step(5.).default_value(50.)),
            focus: cx.focus_handle(),
            disabled: false,
        });
        let state = view.read_with(cx, |view, _| view.state.clone());
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        cx.update(|window, cx| {
            let events = events.clone();
            cx.subscribe(&state, move |_, event: &SliderEvent, _| {
                events.borrow_mut().push(match event {
                    SliderEvent::Change(value) => ("change", value.end()),
                    SliderEvent::Release(value) => ("release", value.end()),
                });
            })
            .detach();
            view.read(cx).focus.clone().focus(window, cx);
            window.draw(cx).clear(cx);
        });

        for (key, value) in [
            ("right", 55.),
            ("left", 50.),
            ("up", 55.),
            ("down", 50.),
            ("pageup", 100.),
            ("pagedown", 50.),
            ("home", 0.),
            ("end", 100.),
        ] {
            cx.simulate_keystrokes(key);
            assert_eq!(
                std::mem::take(&mut *events.borrow_mut()),
                [("change", value), ("release", value)],
                "{key}"
            );
        }
        // At its end, with a modifier, or disabled, a key reports nothing.
        cx.simulate_keystrokes("end ctrl-left");
        view.update(cx, |view, cx| {
            view.disabled = true;
            cx.notify();
        });
        cx.simulate_keystrokes("home");
        assert!(events.borrow().is_empty());
        assert_eq!(state.read_with(cx, |state, _| state.value().end()), 100.);
    }

    #[test]
    fn min_can_be_set_above_the_default_max_before_max() {
        let state = SliderState::new().min(150.).max(200.).default_value(175.);
        assert_eq!(state.value(), SliderValue::Single(175.));
        assert_eq!(state.percentage(), 0.0..0.5);

        let state = SliderState::new()
            .default_value((160., 180.))
            .min(150.)
            .max(200.);
        assert_eq!(state.percentage(), 0.2..0.6);
    }

    #[test]
    fn dragging_snaps_from_min() {
        // Steps count from `min`: 3 + 5k, not multiples of 5.
        assert_eq!(snap_to_step(9.4, 3., 100., 5.), 8.);
        assert_eq!(snap_to_step(11., 3., 100., 5.), 13.);
        // A step past the end stays inside the range.
        assert_eq!(snap_to_step(99., 3., 100., 5.), 98.);
        assert_eq!(snap_to_step(100., 0., 99., 10.), 99.);
        // A step that is not a positive finite number leaves the value continuous.
        assert_eq!(snap_to_step(42.5, 0., 100., 0.), 42.5);
        assert_eq!(snap_to_step(42.5, 0., 100., -5.), 42.5);
        assert_eq!(snap_to_step(42.5, 0., 100., f32::NAN), 42.5);
        assert_eq!(snap_to_step(42.5, 0., 100., f32::INFINITY), 42.5);
    }

    #[test]
    fn accessibility_steps_move_only_the_stepped_thumb() {
        assert_eq!(
            step_thumb(SliderValue::Range(20., 60.), false, 1., 0., 100., 1.),
            SliderValue::Range(20., 61.)
        );
        assert_eq!(
            step_thumb(SliderValue::Range(20., 20.), false, -1., 0., 100., 1.),
            SliderValue::Range(20., 20.),
            "the end never passes the start"
        );
        assert_eq!(
            step_thumb(SliderValue::Single(99.5), false, 1., 0., 100., 1.),
            SliderValue::Single(100.)
        );
        assert_eq!(
            step_thumb(SliderValue::Range(20., 60.), true, -1., 0., 100., 1.),
            SliderValue::Range(19., 60.)
        );
        assert_eq!(
            step_thumb(SliderValue::Range(60., 60.), true, 1., 0., 100., 1.),
            SliderValue::Range(60., 60.),
            "the start never passes the end"
        );
    }

    #[test]
    fn accessibility_steps_land_on_the_step_grid() {
        assert_eq!(
            step_thumb(SliderValue::Single(33.), false, 10., 0., 100., 10.),
            SliderValue::Single(40.)
        );
        assert_eq!(
            step_thumb(SliderValue::Single(33.), false, -10., 0., 100., 10.),
            SliderValue::Single(20.)
        );
        assert_eq!(
            step_thumb(SliderValue::Single(33.), false, 0.5, 0., 100., 0.),
            SliderValue::Single(33.5),
            "a continuous slider does not snap"
        );
    }

    #[test]
    fn a_continuous_slider_steps_by_a_hundredth_of_its_range() {
        for step in [0., -1., f32::NAN] {
            let state = SliderState::new().min(0.).max(200.).step(step);
            assert_eq!(state.accessibility_step(), 2., "step {step}");
        }
        assert_eq!(SliderState::new().step(5.).accessibility_step(), 5.);
    }

    #[test]
    fn stepping_leaves_an_invalid_value_or_range_alone() {
        let nan = step_thumb(SliderValue::Single(f32::NAN), false, 1., 0., 100., 1.);
        assert!(nan.end().is_nan(), "a NaN value does not jump to max");
        assert_eq!(
            step_thumb(SliderValue::Single(50.), false, 1., 100., 0., 1.),
            SliderValue::Single(50.),
            "min above max"
        );
        assert_eq!(
            step_thumb(SliderValue::Range(60., 20.), true, 1., 0., 100., 1.),
            SliderValue::Range(60., 20.),
            "a reversed range"
        );
    }

    struct Holder(Entity<SliderState>);

    impl Render for Holder {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    /// A slider of `state`, 100px wide, in a window.
    fn sized(
        state: SliderState,
        cx: &mut gpui::TestAppContext,
    ) -> (Entity<SliderState>, &mut gpui::VisualTestContext) {
        let (holder, cx) = cx.add_window_view(|_, cx| Holder(cx.new(|_| state)));
        let state = holder.read_with(cx, |holder, _| holder.0.clone());
        state.update(cx, |state, _| {
            state.set_bounds(Bounds::new(
                gpui::point(px(0.), px(0.)),
                gpui::size(px(100.), px(10.)),
            ));
        });
        (state, cx)
    }

    fn drag(state: &Entity<SliderState>, x: f32, start: bool, cx: &mut gpui::VisualTestContext) {
        state.update_in(cx, |state, window, cx| {
            let position = gpui::point(px(x), px(0.));
            state.update_value_by_position(Axis::Horizontal, position, start, window, cx);
        })
    }

    #[gpui::test]
    fn a_drag_snaps_to_steps_counted_from_min(cx: &mut gpui::TestAppContext) {
        let (state, cx) = sized(SliderState::new().min(3.).max(103.).step(5.), cx);
        // 9 is between the steps 8 and 13, not next to 10.
        drag(&state, 6., false, cx);
        assert_eq!(
            state.read_with(cx, |state, _| state.value()),
            SliderValue::Single(8.)
        );
    }

    #[gpui::test]
    fn a_right_to_left_slider_grows_from_the_right(cx: &mut gpui::TestAppContext) {
        let (state, cx) = sized(SliderState::new().step(25.), cx);
        cx.update(|_, cx| crate::LayoutDirection::Rtl.set_global(cx));
        // 30px from the left is 70px from the start edge on the right.
        drag(&state, 30., false, cx);
        assert_eq!(
            state.read_with(cx, |state, _| state.value()),
            SliderValue::Single(75.)
        );

        state.update_in(cx, |state, window, cx| {
            state.set_value((25., 75.), window, cx);
        });
        // The start thumb now sits on the right: a pointer near it moves it.
        let nearer_start = state.read_with(cx, |state, cx| {
            state.is_nearer_start(Axis::Horizontal, gpui::point(px(80.), px(0.)), cx)
        });
        assert!(nearer_start);
    }

    #[gpui::test]
    fn a_dropped_thumb_stands_at_its_value(cx: &mut gpui::TestAppContext) {
        let (state, cx) = sized(SliderState::new().step(25.), cx);
        drag(&state, 30., false, cx);
        state.read_with(cx, |state, _| {
            assert_eq!(state.value(), SliderValue::Single(25.));
            assert_eq!(state.percentage(), 0.0..0.25);
        });

        state.update_in(cx, |state, window, cx| {
            state.set_value((25., 75.), window, cx);
        });
        drag(&state, 40., true, cx);
        drag(&state, 60., false, cx);
        state.read_with(cx, |state, _| {
            assert_eq!(state.value(), SliderValue::Range(50., 50.));
            assert_eq!(state.percentage(), 0.5..0.5);
        });
    }

    #[gpui::test]
    fn a_zero_step_drags_continuously(cx: &mut gpui::TestAppContext) {
        for step in [0., -5., f32::NAN, f32::INFINITY] {
            let (state, cx) = sized(SliderState::new().step(step), cx);
            drag(&state, 30., false, cx);
            state.read_with(cx, |state, _| {
                let value = state.value().end();
                assert!((value - 30.).abs() < 1e-3, "step {step}: {value}");
                assert!((state.percentage().end - 0.3).abs() < 1e-5, "step {step}");
            });
        }
    }

    #[gpui::test]
    fn an_accessibility_step_reports_a_change_then_a_release(cx: &mut gpui::TestAppContext) {
        let (state, cx) = sized(SliderState::new().default_value(99.), cx);
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        cx.update(|_, cx| {
            let events = events.clone();
            cx.subscribe(&state, move |_, event: &SliderEvent, _| {
                events.borrow_mut().push(match event {
                    SliderEvent::Change(value) => ("change", value.end()),
                    SliderEvent::Release(value) => ("release", value.end()),
                });
            })
            .detach();
        });
        let step = |delta: f32, cx: &mut gpui::VisualTestContext| {
            state.update_in(cx, |state, window, cx| {
                state.step_thumb(false, delta, window, cx)
            })
        };
        step(1., cx);
        assert_eq!(&*events.borrow(), &[("change", 100.), ("release", 100.)]);
        assert_eq!(state.read_with(cx, |state, _| state.value().end()), 100.);
        // At its end the step changes nothing and reports nothing.
        step(1., cx);
        assert_eq!(events.borrow().len(), 2);
        step(-1., cx);
        assert_eq!(events.borrow()[2..], [("change", 99.), ("release", 99.)]);
    }

    #[gpui::test]
    fn accessibility_set_value_moves_the_reported_thumb(cx: &mut gpui::TestAppContext) {
        let (state, cx) = sized(SliderState::new().step(5.).default_value((20., 60.)), cx);
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        cx.update(|_, cx| {
            let events = events.clone();
            cx.subscribe(&state, move |_, event: &SliderEvent, _| {
                events.borrow_mut().push(match event {
                    SliderEvent::Change(value) => ("change", *value),
                    SliderEvent::Release(value) => ("release", *value),
                });
            })
            .detach();
        });
        let set = |target, cx: &mut gpui::VisualTestContext| {
            state.update_in(cx, |state, window, cx| {
                state.handle_accessibility_set_value(
                    Some(&ActionData::NumericValue(target)),
                    window,
                    cx,
                );
            });
        };
        set(72., cx);
        set(72., cx);
        set(f64::NAN, cx);
        set(10., cx);
        assert_eq!(
            &*events.borrow(),
            &[
                ("change", SliderValue::Range(20., 70.)),
                ("release", SliderValue::Range(20., 70.)),
                ("change", SliderValue::Range(20., 20.)),
                ("release", SliderValue::Range(20., 20.)),
            ]
        );
    }

    #[gpui::test]
    fn an_accessibility_step_leaves_an_invalid_value_alone(cx: &mut gpui::TestAppContext) {
        let cases = [
            SliderState::new().default_value(f32::NAN),
            SliderState::new().default_value((60., 20.)),
        ];
        for state in cases {
            let (state, cx) = sized(state, cx);
            let before = state.read_with(cx, |state, _| state.value());
            let changes = std::rc::Rc::new(std::cell::Cell::new(0));
            cx.update(|_, cx| {
                let changes = changes.clone();
                cx.subscribe(&state, move |_, _: &SliderEvent, _| {
                    changes.set(changes.get() + 1)
                })
                .detach();
            });
            for start in [false, true] {
                state.update_in(cx, |state, window, cx| {
                    state.step_thumb(start, 1., window, cx)
                });
            }
            let after = state.read_with(cx, |state, _| state.value());
            assert_eq!(
                (after.start().to_bits(), after.end().to_bits()),
                (before.start().to_bits(), before.end().to_bits()),
                "{before:?} became {after:?}"
            );
            assert_eq!(changes.get(), 0, "{before:?} reported a change");
        }
    }

    #[gpui::test]
    fn an_accessibility_step_keeps_a_range(cx: &mut gpui::TestAppContext) {
        let (state, cx) = sized(SliderState::new().default_value((20., 60.)), cx);
        let step = |start: bool, delta: f32, cx: &mut gpui::VisualTestContext| {
            state.update_in(cx, |state, window, cx| {
                state.step_thumb(start, delta, window, cx)
            });
            state.read_with(cx, |state, _| state.value())
        };
        assert_eq!(step(false, 1., cx), SliderValue::Range(20., 61.));
        assert_eq!(step(true, 5., cx), SliderValue::Range(25., 61.));
        assert_eq!(
            step(true, 100., cx),
            SliderValue::Range(61., 61.),
            "a thumb stops at the other"
        );
    }

    #[gpui::test]
    fn a_continuous_slider_reports_a_usable_step(cx: &mut gpui::TestAppContext) {
        use gpui::{Element as _, accesskit, canvas};
        use std::{cell::Cell, rc::Rc};

        struct Probe(Rc<Cell<Option<f64>>>);

        impl Render for Probe {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let reported = self.0.clone();
                canvas(
                    move |_, window, cx| {
                        let state = cx.new(|_| SliderState::new().step(0.));
                        let element = Slider::new(&state).render(window, cx).into_element();
                        let mut node = accesskit::Node::new(Role::Slider);
                        element.write_a11y_info(&mut node);
                        reported.set(node.numeric_value_step());
                    },
                    |_, _, _, _| {},
                )
            }
        }

        let reported = Rc::new(Cell::new(None));
        let (_, cx) = cx.add_window_view({
            let reported = reported.clone();
            move |_, _| Probe(reported)
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(reported.get(), Some(1.), "a hundredth of 0..100");
    }

    #[test]
    #[should_panic(expected = "`min` must be greater than 0")]
    fn legacy_logarithmic_validation_is_preserved() {
        let _ = SliderState::new().scale(SliderScale::Logarithmic);
    }

    /// A slider's bar beside a view that reads its state, under retained views.
    #[cfg(feature = "gpui-fast")]
    mod retained {
        use std::{cell::Cell, rc::Rc};

        use gpui::{
            AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
            TestAppContext, Window, div, px,
        };

        use super::super::{SliderIndicator, SliderState};

        /// A view that shows a slider's value, counting its renders.
        struct Readout {
            state: Entity<SliderState>,
            renders: Rc<Cell<usize>>,
        }

        impl Render for Readout {
            fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                self.renders.set(self.renders.get() + 1);
                div().child(self.state.read(cx).value().to_string())
            }
        }

        /// A slider's bar in a view of its own.
        struct Bar {
            state: Entity<SliderState>,
        }

        impl Render for Bar {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
                    .w(px(120.))
                    .h(px(24.))
                    .child(SliderIndicator::new(&self.state).size_full())
            }
        }

        /// The bar and the readout side by side.
        struct Pair {
            bar: Entity<Bar>,
            readout: Entity<Readout>,
        }

        impl Render for Pair {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
                    .flex()
                    .child(self.bar.clone())
                    .child(self.readout.clone())
            }
        }

        /// Drawing a slider's bar again where it was changes nothing in the
        /// slider's state: a view beside it that reads the state is not built
        /// again for it.
        #[gpui::test]
        fn drawing_a_slider_again_leaves_its_readers_retained(cx: &mut TestAppContext) {
            let renders = Rc::new(Cell::new(0));
            let (view, cx) = cx.add_window_view({
                let renders = renders.clone();
                move |_, cx| {
                    let state = cx.new(|_| SliderState::new());
                    Pair {
                        bar: cx.new(|_| Bar {
                            state: state.clone(),
                        }),
                        readout: cx.new(|_| Readout { state, renders }),
                    }
                }
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let rendered = renders.get();
            for _ in 0..3 {
                view.update(cx, |pair, cx| pair.bar.update(cx, |_, cx| cx.notify()));
                cx.update(|window, cx| window.draw(cx).clear(cx));
            }
            cx.update(|window, cx| window.draw(cx).clear(cx));
            assert_eq!(renders.get(), rendered, "the readout was built again");
        }
    }
}
