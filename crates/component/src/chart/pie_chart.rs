use std::rc::Rc;

use gpui::{App, Bounds, Hsla, Pixels, SharedString, TextAlign, Window, point};
use gpui_component_macros::IntoPlot;
use num_traits::Zero;

use crate::{
    ActiveTheme,
    plot::{
        Plot,
        label::{PlotLabel, TEXT_HEIGHT, TEXT_SIZE, Text},
        polygon,
        shape::{Arc, ArcData, Pie},
    },
};

/// The default extra gap (in pixels) between `outer_radius` and the label radius.
const DEFAULT_LABEL_GAP: f32 = 15.;

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
        }
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
}

impl<T> Plot for PieChart<T> {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let Some(value_fn) = self.value.as_ref() else {
            return;
        };

        let outer_radius = if self.outer_radius.is_zero() {
            bounds.size.height.as_f32() * 0.4
        } else {
            self.outer_radius
        };

        let arc = Arc::new()
            .inner_radius(self.inner_radius)
            .outer_radius(outer_radius);
        let value_fn = value_fn.clone();
        let mut pie = Pie::<T>::new().value(move |d| Some(value_fn(d)));
        pie = pie.pad_angle(self.pad_angle);
        let arcs = pie.arcs(&self.data);

        for a in &arcs {
            let inner_radius = self.get_inner_radius(a);
            let outer_radius = self.get_outer_radius(a);
            arc.paint(
                a,
                if let Some(color_fn) = self.color.as_ref() {
                    color_fn(a.data)
                } else {
                    cx.theme().chart_2
                },
                Some(inner_radius),
                Some(outer_radius),
                &bounds,
                window,
            );
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
        let edge_arc = Arc::new()
            .inner_radius(outer_radius)
            .outer_radius(outer_radius);

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
            let edge = edge_arc.centroid(a);
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
