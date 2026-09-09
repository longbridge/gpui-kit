use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, ElementId, Hsla, IntoElement, Pixels, Point, SharedString, Window,
    point, px,
};
use gpui_component_macros::IntoPlot;
use num_traits::{Num, ToPrimitive};

use crate::{
    ActiveTheme,
    plot::{
        AXIS_GAP, Grid, Plot, PlotAxis, StrokeStyle,
        scale::{Scale, ScaleLinear, ScalePoint, Sealed},
        shape::Line,
        tooltip::{CrossLine, Dot, Tooltip, TooltipState},
    },
};

use super::build_point_x_labels;

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
    id: Option<ElementId>,
    name: Option<SharedString>,
}

impl<T, X, Y> LineChart<T, X, Y>
where
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
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
            id: None,
            name: None,
        }
    }

    /// Enable an interactive hover tooltip (with crosshair and a data dot) for this chart.
    ///
    /// The `id` must be unique among sibling elements. Without it, the chart stays a
    /// non-interactive plot.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
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

    /// Build the x (point) and y (linear) scales for the given bounds.
    ///
    /// Shared by `paint` and `tooltip_state` so the two stay in sync. Returns `None` when the
    /// x/y accessors have not been set.
    fn scales(&self, bounds: Bounds<Pixels>) -> Option<(ScalePoint<X>, ScaleLinear<Y>)> {
        let (x_fn, y_fn) = (self.x.as_ref()?, self.y.as_ref()?);

        let width = bounds.size.width.as_f32();
        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        let height = bounds.size.height.as_f32() - axis_gap;

        let x = ScalePoint::new(self.data.iter().map(|v| x_fn(v)).collect(), vec![0., width]);
        // Y scale, ensure start from 0.
        let y = ScaleLinear::new(
            self.data
                .iter()
                .map(|v| y_fn(v))
                .chain(Some(Y::zero()))
                .collect(),
            vec![height, 10.],
        );

        Some((x, y))
    }
}

impl<T, X, Y> Plot for LineChart<T, X, Y>
where
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let (Some(x_fn), Some(y_fn)) = (self.x.as_ref(), self.y.as_ref()) else {
            return;
        };
        let Some((x, y)) = self.scales(bounds) else {
            return;
        };

        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        let height = bounds.size.height.as_f32() - axis_gap;

        // Draw X axis
        let mut axis = PlotAxis::new().stroke(cx.theme().border);
        if self.x_axis {
            let labels = build_point_x_labels(
                &self.data,
                x_fn.as_ref(),
                &x,
                self.tick_margin,
                cx.theme().muted_foreground,
            );
            axis = axis.x(height).x_label(labels);
        }
        axis.paint(&bounds, window, cx);

        // Draw grid
        if self.grid {
            Grid::new()
                .y((0..=3).map(|i| height * i as f32 / 4.0).collect())
                .stroke(cx.theme().border)
                .dash_array(&[px(4.), px(2.)])
                .paint(&bounds, window);
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

        line.paint(&bounds, window);
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
        let (x_fn, y_fn) = (self.x.as_ref()?, self.y.as_ref()?);
        let (x, y) = self.scales(bounds)?;

        // Ignore the x-axis label gutter so hovering the labels doesn't show a tooltip.
        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        if position.y.as_f32() > bounds.size.height.as_f32() - axis_gap {
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

        Some(
            // Follow the cursor; the crosshair and dot stay snapped to the data point.
            Tooltip::new(cursor, bounds.size)
                .gap(px(8.))
                // Confine the crosshair to the plot area so it doesn't cross the x-axis.
                .cross_line(
                    CrossLine::new(state.cross_line).height(
                        bounds.size.height.as_f32() - if self.x_axis { AXIS_GAP } else { 0. },
                    ),
                )
                .dots(
                    state
                        .dots
                        .iter()
                        .map(|p| Dot::new(*p).stroke(cx.theme().background).fill(stroke)),
                )
                .title(title)
                .row(stroke, name, format!("{}", value))
                .into_any_element(),
        )
    }
}
