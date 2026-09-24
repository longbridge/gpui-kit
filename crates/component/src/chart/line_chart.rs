use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, ElementId, Hsla, IntoElement, Pixels, Point, SharedString, Size,
    Window, point, px,
};
use gpui_component_macros::IntoPlot;
use num_traits::{Num, ToPrimitive};

use crate::{
    ActiveTheme,
    plot::{
        AXIS_GAP, AxisLabelPlacement, PathCaches, Plot, PlotAxis, StrokeStyle,
        scale::{Scale, ScaleLinear, ScalePoint, Sealed},
        shape::Line,
        tooltip::{CrossLine, Dot, Tooltip, TooltipState},
    },
};

use super::{
    HOVER_DOT_SIZE, HOVER_HALO_SIZE, PointAxes, ValueExtent, axis_point_count,
    build_point_x_labels, caller_id, labeled_items, pinned_plot_mask, point_range,
    point_value_scale,
};

#[derive(IntoPlot)]
pub struct LineChart<T, X, Y>
where
    T: 'static,
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    data: Vec<T>,
    x: Option<Rc<dyn Fn(&T) -> X>>,
    y: Option<Rc<dyn Fn(&T) -> Y>>,
    stroke: Option<Hsla>,
    stroke_style: StrokeStyle,
    dot: bool,
    tick_margin: usize,
    x_axis: bool,
    grid: bool,
    y_domain: Option<(Y, Y)>,
    point_count: Option<usize>,
    axes: PointAxes,
    id: ElementId,
    interactive: bool,
    name: Option<SharedString>,
}

impl<T, X, Y> LineChart<T, X, Y>
where
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    #[track_caller]
    pub fn new<I>(data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            data: data.into_iter().collect(),
            stroke: None,
            stroke_style: Default::default(),
            dot: false,
            x: None,
            y: None,
            tick_margin: 1,
            x_axis: true,
            grid: true,
            y_domain: None,
            point_count: None,
            axes: PointAxes::default(),
            id: caller_id(),
            interactive: true,
            name: None,
        }
    }

    /// Name this chart's [`ElementId`], replacing the default taken from the
    /// construction site.
    ///
    /// Pass one where a single construction site renders several of these
    /// charts as siblings: they share the default id, and with it one hover
    /// state and one path cache. The id must be unique among those siblings.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Turn this chart's interactive layer on or off. On by default.
    ///
    /// The layer is the hitbox under the cursor and what it drives: a crosshair
    /// and a dot mark the hovered point, and a tooltip shows its value. Turn it
    /// off for a chart that only decorates, or one an element above it wants the
    /// cursor for: without a hitbox it neither answers the mouse nor takes the
    /// hover from what sits over it. A chart that is off also drops its path
    /// cache, which is keyed on the same id.
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Set the series name shown in the hover tooltip row (e.g. "Desktop").
    pub fn name(mut self, name: impl Into<SharedString>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn x(mut self, x: impl Fn(&T) -> X + 'static) -> Self {
        self.x = Some(Rc::new(x));
        self
    }

    pub fn y(mut self, y: impl Fn(&T) -> Y + 'static) -> Self {
        self.y = Some(Rc::new(y));
        self
    }

    pub fn stroke(mut self, stroke: impl Into<Hsla>) -> Self {
        self.stroke = Some(stroke.into());
        self
    }

    pub fn natural(mut self) -> Self {
        self.stroke_style = StrokeStyle::Natural;
        self
    }

    pub fn linear(mut self) -> Self {
        self.stroke_style = StrokeStyle::Linear;
        self
    }

    pub fn step_after(mut self) -> Self {
        self.stroke_style = StrokeStyle::StepAfter;
        self
    }

    pub fn dot(mut self) -> Self {
        self.dot = true;
        self
    }

    pub fn tick_margin(mut self, tick_margin: usize) -> Self {
        self.tick_margin = tick_margin;
        self
    }

    /// Show or hide the x-axis line and labels.
    ///
    /// Default is true.
    pub fn x_axis(mut self, x_axis: bool) -> Self {
        self.x_axis = x_axis;
        self
    }

    pub fn grid(mut self, grid: bool) -> Self {
        self.grid = grid;
        self
    }

    /// Pin the y axis to `min..=max` instead of fitting the line from zero.
    ///
    /// Pin it where zero is not a meaningful baseline, such as a price line
    /// that would otherwise be pressed flat against the top. The range keeps
    /// the `y_padding` headroom above `max`, 10px by default,
    /// and the line is clipped to the plot, so a value outside the range stops
    /// at its edge. Nothing is drawn when `min` equals `max`.
    pub fn y_domain(mut self, min: Y, max: Y) -> Self {
        self.y_domain = Some((min, max));
        self
    }

    /// Lay the x axis out for `count` evenly spaced points instead of the
    /// data's own length.
    ///
    /// The data takes the leading points in order, the i-th item on the i-th
    /// point, and the rest stay empty, as an intraday chart does before the
    /// close. The data has to be contiguous from the first point: a missing
    /// item shifts every later one a point to the left. A `count` below the
    /// data's length has no effect.
    pub fn point_count(mut self, count: usize) -> Self {
        self.point_count = Some(count);
        self
    }

    /// Show the y axis's tick labels, one at each of the `y_tick_count` ticks.
    ///
    /// Default is false.
    pub fn y_axis(mut self, y_axis: bool) -> Self {
        self.axes.y_axis = y_axis;
        self
    }

    /// Set where the y-axis tick labels sit: in a gutter left of the plot, or
    /// inside it beside their grid lines.
    ///
    /// Default is [`AxisLabelPlacement::Outside`].
    pub fn y_axis_label_placement(mut self, placement: AxisLabelPlacement) -> Self {
        self.axes.y_axis_label_placement = placement;
        self
    }

    /// Set how many ticks the y axis carries, evenly spaced from the baseline
    /// to the top edge with both ends included.
    ///
    /// The ticks place the horizontal grid lines and the tick labels, and each
    /// label reads the value the scale puts at its height. Values below 2 are
    /// raised to 2.
    ///
    /// Default is 5.
    pub fn y_tick_count(mut self, count: usize) -> Self {
        self.axes.y_tick_count = count.max(2);
        self
    }

    /// Set the text of each y-axis tick label from the value at its tick.
    pub fn y_tick_format<S>(mut self, format: impl Fn(f64) -> S + 'static) -> Self
    where
        S: Into<SharedString> + 'static,
    {
        self.axes.y_tick_format = Some(Rc::new(move |value| format(value).into()));
        self
    }

    /// Label `count` of the x values, spread evenly from the first to the
    /// last, instead of every `tick_margin`-th.
    ///
    /// With [`Self::point_count`] set, the labels spread over all the points the
    /// axis is laid out for, so they keep their places as the data grows; one
    /// that falls past the data is not drawn yet.
    pub fn x_tick_count(mut self, count: usize) -> Self {
        self.axes.x_tick_count = Some(count);
        self
    }

    /// Divide the plot into `count` columns with vertical grid lines, the first
    /// on its left edge.
    ///
    /// Default is 0, no vertical lines.
    pub fn grid_columns(mut self, count: usize) -> Self {
        self.axes.grid_columns = count;
        self
    }

    /// Draw the grid dashed or solid.
    ///
    /// Default is true.
    pub fn grid_dashed(mut self, dashed: bool) -> Self {
        self.axes.grid_dashed = dashed;
        self
    }

    /// Draw a dashed line across the plot at `value`, such as a previous close.
    ///
    /// Call again for more lines. A value outside the y axis is not drawn.
    pub fn reference_line(mut self, value: Y) -> Self {
        if let Some(value) = value.to_f64() {
            self.axes.reference_lines.push(value);
        }
        self
    }

    /// Set the space kept clear above the highest value and below the lowest,
    /// in pixels.
    ///
    /// Default is 10px above and none below.
    pub fn y_padding(mut self, top: f32, bottom: f32) -> Self {
        self.axes.y_padding = (top, bottom);
        self
    }

    /// Build the x (point) and y (linear) scales for the given bounds.
    ///
    /// Shared by `paint` and `tooltip_state` so the two stay in sync. Returns `None` when the
    /// x/y accessors have not been set.
    fn scales(
        &self,
        bounds: Bounds<Pixels>,
    ) -> Option<(ScalePoint<X>, ScaleLinear<Y>, ValueExtent)> {
        let (x_fn, y_fn) = (self.x.as_ref()?, self.y.as_ref()?);

        let width = bounds.size.width.as_f32();
        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        let height = bounds.size.height.as_f32() - axis_gap;

        let len = self.data.len();
        let x = ScalePoint::new(
            self.data.iter().map(|v| x_fn(v)).collect(),
            point_range(
                self.axes.plot_left(),
                width - self.axes.plot_left(),
                len,
                axis_point_count(self.point_count, len),
            ),
        );
        let (y, extent) = point_value_scale(
            self.data.iter().map(|v| y_fn(v)),
            self.y_domain,
            height,
            self.axes.y_padding,
        );

        Some((x, y, extent))
    }
}

impl<T, X, Y> Plot for LineChart<T, X, Y>
where
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    fn prepaint(
        &mut self,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut App,
    ) -> Vec<AnyElement> {
        // The y labels' gutter is measured before the x scale is laid out past it.
        if let Some((_, _, extent)) = self.scales(bounds) {
            let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
            let height = bounds.size.height.as_f32() - axis_gap;
            self.axes.measure_y_labels(extent, height, window);
        }
        vec![]
    }

    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let (Some(x_fn), Some(y_fn)) = (self.x.as_ref(), self.y.as_ref()) else {
            return;
        };
        let Some((x, y, extent)) = self.scales(bounds) else {
            return;
        };

        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        let height = bounds.size.height.as_f32() - axis_gap;

        // Draw X axis
        // The axis runs under the plot only, clear of a value-axis gutter, so
        // its labels shift back by the gutter the x scale already includes.
        let left = self.axes.plot_left();
        let axis_bounds = Bounds {
            origin: bounds.origin + point(px(left), px(0.)),
            size: Size::new(bounds.size.width - px(left), bounds.size.height),
        };
        let mut axis = PlotAxis::new().stroke(cx.theme().border);
        if self.x_axis {
            let labeled = labeled_items(
                axis_point_count(self.point_count, self.data.len()),
                self.axes.x_tick_count,
                self.tick_margin,
            );
            let labels = build_point_x_labels(
                &self.data,
                x_fn.as_ref(),
                &x,
                axis_point_count(self.point_count, self.data.len()),
                &labeled,
                cx.theme().muted_foreground,
            )
            .into_iter()
            .map(|mut label| {
                label.tick -= px(left);
                label
            });
            axis = axis.x(height).x_label(labels);
        }
        axis.paint(&axis_bounds, window, cx);

        if self.grid {
            self.axes.paint_grid(bounds, height, window, cx);
        }

        // Draw line
        let stroke = self.stroke.unwrap_or(cx.theme().chart_2);
        let x_fn = x_fn.clone();
        let y_fn = y_fn.clone();
        let mut line = Line::new()
            .data(&self.data)
            .x(move |d| x.tick(&x_fn(d)))
            .y(move |d| y.tick(&y_fn(d)))
            .stroke(stroke)
            .stroke_style(self.stroke_style)
            .stroke_width(2.);

        if self.dot {
            line = line.dot().dot_size(8.).dot_fill_color(stroke);
        }

        let mask = self
            .y_domain
            .is_some()
            .then(|| pinned_plot_mask(bounds, height));
        window.with_content_mask(mask, |window| {
            // Caching hangs off the chart's own id, which only an interactive chart
            // puts on the stack; without one, siblings would share a slot and thrash
            // it, so a chart that is off tessellates afresh each paint.
            if self.interactive {
                let caches = PathCaches::for_paint("line", window, cx);
                caches.update(cx, |caches, _| {
                    line.paint_cached(&bounds, caches.slot(0), window);
                });
            } else {
                line.paint(&bounds, window);
            }
        });

        self.axes
            .paint_reference_lines(extent, bounds, height, window, cx);
        self.axes.paint_y_labels(extent, bounds, height, window, cx);
    }

    fn id(&self) -> Option<ElementId> {
        self.interactive.then(|| self.id.clone())
    }

    fn tooltip_state(
        &self,
        position: Point<Pixels>,
        bounds: Bounds<Pixels>,
        _cx: &App,
    ) -> Option<TooltipState> {
        let (x_fn, y_fn) = (self.x.as_ref()?, self.y.as_ref()?);
        let (x, y, _) = self.scales(bounds)?;

        // Ignore the x-axis label gutter so hovering the labels doesn't show a tooltip.
        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        if position.y.as_f32() > bounds.size.height.as_f32() - axis_gap
            || position.x.as_f32() < self.axes.plot_left()
        {
            return None;
        }

        let index = x.least_index(position.x.as_f32());
        let d = self.data.get(index)?;
        let x_tick = x.tick(&x_fn(d))?;
        let y_tick = y.tick(&y_fn(d))?;

        Some(TooltipState::new(
            index,
            point(px(x_tick), position.y),
            vec![point(px(x_tick), px(y_tick))],
        ))
    }

    fn tooltip(
        &self,
        state: &TooltipState,
        cursor: Point<Pixels>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let (x_fn, y_fn) = (self.x.as_ref()?, self.y.as_ref()?);
        let d = self.data.get(state.index)?;
        let title: SharedString = x_fn(d).into();
        let value = y_fn(d).to_f64()?;
        let stroke = self.stroke.unwrap_or(cx.theme().chart_2);
        let name = self.name.clone().unwrap_or_default();
        let dot = *state.dots.first()?;

        Some(
            // Follow the cursor; the crosshair and dot glide to the data point.
            Tooltip::new(cursor, bounds.size)
                .gap(px(8.))
                // Confine the crosshair to the plot area so it doesn't cross the x-axis.
                .cross_line(
                    CrossLine::new(state.cross_line).height(
                        bounds.size.height.as_f32() - if self.x_axis { AXIS_GAP } else { 0. },
                    ),
                )
                .dots(Some(
                    Dot::new(dot)
                        .size(HOVER_DOT_SIZE)
                        .halo(HOVER_HALO_SIZE)
                        .stroke(cx.theme().background)
                        .fill(stroke),
                ))
                .title(title)
                .row(stroke, name, format!("{}", value))
                .into_any_element(),
        )
    }
}
