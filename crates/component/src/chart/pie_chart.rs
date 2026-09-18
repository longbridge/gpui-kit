use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, ElementId, Hsla, IntoElement, Pixels, Point, SharedString, TextAlign,
    Window, point, prelude::FluentBuilder, px,
};
use gpui_base::motion::spring;
use gpui_component_macros::IntoPlot;
use num_traits::Zero;

use crate::{
    ActiveTheme,
    plot::{
        PathCaches, Plot,
        label::{PlotLabel, TEXT_HEIGHT, TEXT_SIZE, Text},
        polygon,
        shape::{Arc, ArcData, Pie},
        tooltip::{PlotHover, Tooltip, TooltipState},
    },
};

/// The default extra gap (in pixels) between `outer_radius` and the label radius.
const DEFAULT_LABEL_GAP: f32 = 15.;

/// How far the hovered slice moves out past its outer radius, in pixels.
const HOVER_LIFT: f32 = 6.;

/// How much the slices other than the hovered one fade, as a share of their opacity.
const HOVER_DIM: f32 = 0.35;

/// The hover a pie chart paints, sampled once per frame in [`Plot::hover`].
struct PieHover {
    /// How far each datum's slice has lifted, `0..=1`, springing up on the
    /// hovered slice and back down on the one the cursor left.
    lift: Vec<f32>,
    /// How far the hover has faded in.
    focus: f32,
}

#[derive(IntoPlot)]
pub struct PieChart<T: 'static> {
    data: Vec<T>,
    inner_radius: f32,
    inner_radius_fn: Option<Rc<dyn Fn(&ArcData<T>) -> f32 + 'static>>,
    outer_radius: f32,
    outer_radius_fn: Option<Rc<dyn Fn(&ArcData<T>) -> f32 + 'static>>,
    pad_angle: f32,
    value: Option<Rc<dyn Fn(&T) -> f32>>,
    color: Option<Rc<dyn Fn(&T) -> Hsla>>,
    label: Option<Rc<dyn Fn(&T) -> SharedString + 'static>>,
    label_line_color: Option<Rc<dyn Fn(&T) -> Hsla + 'static>>,
    label_color: Option<Hsla>,
    label_gap: f32,
    id: Option<ElementId>,
    name: Option<SharedString>,
    hover: Option<PieHover>,
}

impl<T> PieChart<T> {
    pub fn new<I>(data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            data: data.into_iter().collect(),
            inner_radius: 0.,
            inner_radius_fn: None,
            outer_radius: 0.,
            outer_radius_fn: None,
            pad_angle: 0.,
            value: None,
            color: None,
            label: None,
            label_line_color: None,
            label_color: None,
            label_gap: DEFAULT_LABEL_GAP,
            id: None,
            name: None,
            hover: None,
        }
    }

    /// Enable an interactive hover tooltip for this chart: the hovered slice
    /// lifts out of the ring and the tooltip shows its value and share.
    ///
    /// The `id` must be unique among sibling elements. Without it, the chart
    /// stays a non-interactive plot.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the series name shown in the hover tooltip row (e.g. "Desktop").
    pub fn name(mut self, name: impl Into<SharedString>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the inner radius of the pie chart.
    pub fn inner_radius(mut self, inner_radius: f32) -> Self {
        self.inner_radius = inner_radius;
        self
    }

    /// Set the inner radius of the pie chart based on the arc data.
    pub fn inner_radius_fn(
        mut self,
        inner_radius_fn: impl Fn(&ArcData<T>) -> f32 + 'static,
    ) -> Self {
        self.inner_radius_fn = Some(Rc::new(inner_radius_fn));
        self
    }

    fn get_inner_radius(&self, arc: &ArcData<T>) -> f32 {
        if let Some(inner_radius_fn) = self.inner_radius_fn.as_ref() {
            inner_radius_fn(arc)
        } else {
            self.inner_radius
        }
    }

    /// Set the outer radius of the pie chart.
    pub fn outer_radius(mut self, outer_radius: f32) -> Self {
        self.outer_radius = outer_radius;
        self
    }

    /// Set the outer radius of the pie chart based on the arc data.
    pub fn outer_radius_fn(
        mut self,
        outer_radius_fn: impl Fn(&ArcData<T>) -> f32 + 'static,
    ) -> Self {
        self.outer_radius_fn = Some(Rc::new(outer_radius_fn));
        self
    }

    fn get_outer_radius(&self, arc: &ArcData<T>) -> f32 {
        if let Some(outer_radius_fn) = self.outer_radius_fn.as_ref() {
            outer_radius_fn(arc)
        } else {
            self.outer_radius
        }
    }

    /// Set the pad angle of the pie chart.
    pub fn pad_angle(mut self, pad_angle: f32) -> Self {
        self.pad_angle = pad_angle;
        self
    }

    pub fn value(mut self, value: impl Fn(&T) -> f32 + 'static) -> Self {
        self.value = Some(Rc::new(value));
        self
    }

    /// Set the color of the pie chart.
    pub fn color<H>(mut self, color: impl Fn(&T) -> H + 'static) -> Self
    where
        H: Into<Hsla> + 'static,
    {
        self.color = Some(Rc::new(move |t| color(t).into()));
        self
    }

    /// Set the label text for each slice.
    ///
    /// Once set, a "leader line + text" is drawn outside the ring for every
    /// slice.
    pub fn label(mut self, label: impl Fn(&T) -> SharedString + 'static) -> Self {
        self.label = Some(Rc::new(label));
        self
    }

    /// Set the leader line color per slice (defaults to `cx.theme().border`).
    pub fn label_line_color(mut self, color: impl Fn(&T) -> Hsla + 'static) -> Self {
        self.label_line_color = Some(Rc::new(color));
        self
    }

    /// Set the label text color (defaults to `cx.theme().foreground`).
    pub fn label_color(mut self, color: Hsla) -> Self {
        self.label_color = Some(color);
        self
    }

    /// Set the extra gap between `outer_radius` and the label radius
    /// (defaults to 15px).
    pub fn label_gap(mut self, gap: f32) -> Self {
        self.label_gap = gap;
        self
    }

    /// The outer radius the ring is laid out with: the set one, or 40% of the
    /// bounds height.
    fn resolve_outer_radius(&self, bounds: &Bounds<Pixels>) -> f32 {
        if self.outer_radius.is_zero() {
            bounds.size.height.as_f32() * 0.4
        } else {
            self.outer_radius
        }
    }

    /// The slices, in ring order. Shared by `paint` and `tooltip_state` so the
    /// two stay in sync; empty without a value accessor.
    fn arcs(&self) -> Vec<ArcData<'_, T>> {
        let Some(value_fn) = self.value.clone() else {
            return vec![];
        };
        Pie::<T>::new()
            .value(move |d| Some(value_fn(d)))
            .pad_angle(self.pad_angle)
            .arcs(&self.data)
    }

    /// The fill of a slice: the per-datum color, or the theme's.
    fn slice_color(&self, datum: &T, cx: &App) -> Hsla {
        match self.color.as_ref() {
            Some(color_fn) => color_fn(datum),
            None => cx.theme().chart_2,
        }
    }

    /// How far the slice of datum `index` has lifted and how much it has faded
    /// behind the hovered one this frame, as `(lift, opacity)`.
    fn slice_emphasis(&self, index: usize) -> (f32, f32) {
        let Some(hover) = self.hover.as_ref() else {
            return (0., 1.);
        };
        let lift = hover.lift.get(index).copied().unwrap_or(0.) * hover.focus;
        (lift, 1. - HOVER_DIM * hover.focus * (1. - lift))
    }
}

impl<T> Plot for PieChart<T> {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        if self.value.is_none() {
            return;
        }

        let outer_radius = self.resolve_outer_radius(&bounds);

        let arc = Arc::new()
            .inner_radius(self.inner_radius)
            .outer_radius(outer_radius);
        let arcs = self.arcs();

        // An identified chart keeps its slices tessellated across frames; without
        // an id, sibling charts would share one cache and thrash it.
        let caches = self
            .id
            .is_some()
            .then(|| PathCaches::for_paint("slices", window, cx));
        for (ix, a) in arcs.iter().enumerate() {
            let inner_radius = self.get_inner_radius(a);
            // The hovered slice lifts out of the ring while the others fade behind it.
            let (lift, opacity) = self.slice_emphasis(a.index);
            let outer_radius = self.get_outer_radius(a) + HOVER_LIFT * lift;
            let color = self.slice_color(a.data, cx).opacity(opacity);
            match caches.as_ref() {
                Some(caches) => caches.update(cx, |caches, _| {
                    arc.paint_cached(
                        a,
                        color,
                        Some(inner_radius),
                        Some(outer_radius),
                        &bounds,
                        caches.slot(ix),
                        window,
                    );
                }),
                None => arc.paint(
                    a,
                    color,
                    Some(inner_radius),
                    Some(outer_radius),
                    &bounds,
                    window,
                ),
            }
        }

        // Draw leader-line labels outside the ring (only when `label` is set).
        let Some(label_fn) = self.label.as_ref() else {
            return;
        };

        let label_radius = outer_radius + self.label_gap;
        let center_x = bounds.size.width.as_f32() / 2.;
        let center_y = bounds.size.height.as_f32() / 2.;
        let label_arc = Arc::new()
            .inner_radius(label_radius)
            .outer_radius(label_radius);

        let label_color = self.label_color.unwrap_or(cx.theme().foreground);
        let default_line_color = cx.theme().border;

        // First pass: collect a layout candidate per visible slice, split by
        // side. `y` is the target vertical position relative to the center and
        // gets adjusted later to remove overlaps.
        let mut right: Vec<LabelLayout> = vec![];
        let mut left: Vec<LabelLayout> = vec![];
        for a in &arcs {
            // Skip tiny slices (< 0.5°) that are too thin to label.
            if a.end_angle - a.start_angle < std::f32::consts::PI / 360. {
                continue;
            }

            let centroid = label_arc.centroid(a);
            // Anchor the line on the edge the slice reaches this frame, so a
            // lifted slice never paints over its own leader line. The label
            // anchor stays put, so the line may not start past it.
            let (lift, _) = self.slice_emphasis(a.index);
            let edge_radius = (outer_radius + HOVER_LIFT * lift).min(label_radius);
            let edge = Arc::new()
                .inner_radius(edge_radius)
                .outer_radius(edge_radius)
                .centroid(a);
            let is_right = centroid.x > 0.;
            let line_color = self
                .label_line_color
                .as_ref()
                .map(|f| f(a.data))
                .unwrap_or(default_line_color);

            let layout = LabelLayout {
                arc_x: edge.x,
                arc_y: edge.y,
                label_x: centroid.x,
                y: centroid.y,
                text: label_fn(a.data),
                line_color,
            };
            if is_right { &mut right } else { &mut left }.push(layout);
        }

        // Second pass: spread labels on each side so neighbors keep at least one
        // text height apart, clamped within the vertical bounds.
        let top = -center_y + TEXT_HEIGHT / 2.;
        let bottom = center_y - TEXT_HEIGHT / 2.;
        spread_labels(&mut right, top, bottom);
        spread_labels(&mut left, top, bottom);

        // Third pass: paint leader lines first, then the text on top.
        let mut labels = vec![];
        for (side, items) in [(1., &right), (-1., &left)] {
            for item in items {
                // Leader line: ring edge -> label anchor -> horizontal pull to
                // ±label_radius.
                let pts = [
                    point(item.arc_x + center_x, item.arc_y + center_y),
                    point(item.label_x + center_x, item.y + center_y),
                    point(side * label_radius + center_x, item.y + center_y),
                ];
                if let Some(p) = polygon(&pts, &bounds) {
                    window.paint_path(p, item.line_color);
                }

                // Text sits 4px further out, aligned by side.
                let origin = point(
                    side * (label_radius + 4.) + center_x,
                    item.y - TEXT_SIZE / 2. + center_y,
                );
                let align = if side > 0. {
                    TextAlign::Left
                } else {
                    TextAlign::Right
                };
                labels.push(Text::new(item.text.clone(), origin, label_color).align(align));
            }
        }

        PlotLabel::new(labels).paint(&bounds, window, cx);
    }

    fn id(&self) -> Option<ElementId> {
        self.id.clone()
    }

    fn tooltip_state(
        &self,
        position: Point<Pixels>,
        bounds: Bounds<Pixels>,
        _cx: &App,
    ) -> Option<TooltipState> {
        let outer_radius = self.resolve_outer_radius(&bounds);
        let arc = Arc::new()
            .inner_radius(self.inner_radius)
            .outer_radius(outer_radius);
        let position = point(position.x.as_f32(), position.y.as_f32());

        let index = self.arcs().into_iter().find_map(|a| {
            arc.contains(
                &a,
                position,
                Some(self.get_inner_radius(&a)),
                Some(self.get_outer_radius(&a)),
                &bounds,
            )
            .then_some(a.index)
        })?;

        Some(TooltipState::new(
            index,
            point(px(position.x), px(position.y)),
            vec![],
        ))
    }

    fn hover(&mut self, hover: Option<&PlotHover>, window: &mut Window, cx: &mut App) {
        self.hover = hover.map(|hover| {
            // Every slice springs toward lifted or resting, so the one the cursor
            // left settles back while the new one rises. On the first hovered
            // frame the target is rest, so the slice rises from the ring rather
            // than adopting the lifted position outright.
            let policy = cx.theme().motion_tokens().spring_control;
            let lift = (0..self.data.len())
                .map(|ix| {
                    let lifted =
                        hover.is_hovered() && !hover.is_entering() && ix == hover.state().index;
                    spring(
                        ElementId::named_usize("pie-slice", ix),
                        if lifted { 1. } else { 0. },
                        policy,
                        window,
                        cx,
                    )
                })
                .collect();
            PieHover {
                lift,
                focus: hover.focus(),
            }
        });
    }

    fn tooltip(
        &self,
        state: &TooltipState,
        cursor: Point<Pixels>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let value_fn = self.value.as_ref()?;
        let d = self.data.get(state.index)?;
        let value = value_fn(d);
        let total: f32 = self.data.iter().map(|d| value_fn(d).max(0.)).sum();
        let share = if total > 0. { value / total * 100. } else { 0. };
        let name = self.name.clone().unwrap_or_default();

        Some(
            // Follow the cursor; the lifted slice marks the datum.
            Tooltip::new(cursor, bounds.size)
                .gap(px(8.))
                .when_some(self.label.as_ref(), |this, label| this.title(label(d)))
                .row(
                    self.slice_color(d, cx),
                    name,
                    format!("{} ({:.1}%)", value, share),
                )
                .into_any_element(),
        )
    }
}

/// A resolved label position before overlap adjustment.
struct LabelLayout {
    /// Anchor on the ring edge (relative to center).
    arc_x: f32,
    arc_y: f32,
    /// Centroid x at the label radius (relative to center).
    label_x: f32,
    /// Target/adjusted vertical position (relative to center).
    y: f32,
    text: SharedString,
    line_color: Hsla,
}

/// Spread `items` vertically so that adjacent labels keep at least
/// [`TEXT_HEIGHT`] apart, clamped within `[top, bottom]`.
///
/// Uses a two-direction relaxation: a top-down pass pushes crowded labels down,
/// then a bottom-up pass (anchored at `bottom`) pushes them back up. This
/// resolves cascading overlaps that a single-neighbor nudge cannot.
fn spread_labels(items: &mut [LabelLayout], top: f32, bottom: f32) {
    let n = items.len();
    if n == 0 {
        return;
    }

    // Sort by target position so neighbors in the slice are neighbors in y.
    items.sort_by(|a, b| a.y.total_cmp(&b.y));

    // Top-down: enforce the minimum gap by pushing labels down.
    for i in 1..n {
        let min_y = items[i - 1].y + TEXT_HEIGHT;
        if items[i].y < min_y {
            items[i].y = min_y;
        }
    }

    // Bottom-up: clamp the bottom-most label, then pull overflowing labels up.
    if items[n - 1].y > bottom {
        items[n - 1].y = bottom;
    }
    for i in (0..n - 1).rev() {
        let max_y = items[i + 1].y - TEXT_HEIGHT;
        if items[i].y > max_y {
            items[i].y = max_y;
        }
    }

    // Keep the top-most label within bounds.
    if items[0].y < top {
        items[0].y = top;
    }
}
