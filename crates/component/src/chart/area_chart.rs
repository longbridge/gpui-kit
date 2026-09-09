use std::rc::Rc;

use gpui::{
    AnyElement, App, Background, Bounds, ElementId, Hsla, IntoElement, Pixels, Point, SharedString,
    Window, point, px,
};
use gpui_component_macros::IntoPlot;
use num_traits::{Num, ToPrimitive};

use crate::{
    ActiveTheme,
    plot::{
        AXIS_GAP, Grid, Plot, PlotAxis, StrokeStyle,
        scale::{Scale, ScaleLinear, ScalePoint, Sealed},
        shape::Area,
        tooltip::{CrossLine, Dot, Tooltip, TooltipState},
    },
};

use super::build_point_x_labels;

#[derive(IntoPlot)]
pub struct AreaChart<T, X, Y>
where
    T: 'static,
    X: Clone + PartialEq + Into<SharedString> + 'static,
    Y: Clone + Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    data: Vec<T>,
    x: Option<Rc<dyn Fn(&T) -> X>>,
    y: Vec<Rc<dyn Fn(&T) -> Y>>,
    strokes: Vec<Hsla>,
    stroke_styles: Vec<StrokeStyle>,
    fills: Vec<Background>,
    names: Vec<SharedString>,
    tick_margin: usize,
    x_axis: bool,
    grid: bool,
    id: Option<ElementId>,
}

impl<T, X, Y> AreaChart<T, X, Y>
where
    X: Clone + PartialEq + Into<SharedString> + 'static,
    Y: Clone + Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    pub fn new<I>(data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            data: data.into_iter().collect(),
            stroke_styles: vec![],
            strokes: vec![],
            fills: vec![],
            names: vec![],
            tick_margin: 1,
            x: None,
            y: vec![],
            x_axis: true,
            grid: true,
            id: None,
        }
    }

    /// Enable an interactive hover tooltip (crosshair + a dot and row per series).
    ///
    /// The `id` must be unique among sibling elements. Without it, the chart stays a
    /// non-interactive plot.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the name of the most recently added series, shown in its tooltip row.
    ///
    /// Call after the matching [`AreaChart::y`] (e.g. `.y(..).stroke(..).name("Desktop")`).
    pub fn name(mut self, name: impl Into<SharedString>) -> Self {
        self.names.push(name.into());
        self
    }

    pub fn x(mut self, x: impl Fn(&T) -> X + 'static) -> Self {
        self.x = Some(Rc::new(x));
        self
    }

    pub fn y(mut self, y: impl Fn(&T) -> Y + 'static) -> Self {
        self.y.push(Rc::new(y));
        self
    }

    pub fn stroke(mut self, stroke: impl Into<Hsla>) -> Self {
        self.strokes.push(stroke.into());
        self
    }

    pub fn fill(mut self, fill: impl Into<Background>) -> Self {
        self.fills.push(fill.into());
        self
    }

    pub fn natural(mut self) -> Self {
        self.stroke_styles.push(StrokeStyle::Natural);
        self
    }

    pub fn linear(mut self) -> Self {
        self.stroke_styles.push(StrokeStyle::Linear);
        self
    }

    pub fn step_after(mut self) -> Self {
        self.stroke_styles.push(StrokeStyle::StepAfter);
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
    /// Shared by `paint` and `tooltip_state` so the two stay in sync. Returns `None` when there
    /// is no x accessor or no series.
    fn scales(&self, bounds: Bounds<Pixels>) -> Option<(ScalePoint<X>, ScaleLinear<Y>)> {
        let x_fn = self.x.as_ref()?;
        if self.y.is_empty() {
            return None;
        }

        let width = bounds.size.width.as_f32();
        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        let height = bounds.size.height.as_f32() - axis_gap;

        let x = ScalePoint::new(self.data.iter().map(|v| x_fn(v)).collect(), vec![0., width]);
        let domain = self
            .data
            .iter()
            .flat_map(|v| self.y.iter().map(|y_fn| y_fn(v)))
            .chain(Some(Y::zero()))
            .collect::<Vec<_>>();
        let y = ScaleLinear::new(domain, vec![height, 10.]);

        Some((x, y))
    }
}

impl<T, X, Y> Plot for AreaChart<T, X, Y>
where
    X: Clone + PartialEq + Into<SharedString> + 'static,
    Y: Clone + Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let Some(x_fn) = self.x.as_ref() else {
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

        // Draw area
        for (i, y_fn) in self.y.iter().enumerate() {
            let x = x.clone();
            let y = y.clone();
            let x_fn = x_fn.clone();
            let y_fn = y_fn.clone();

            let fill = *self
                .fills
                .get(i)
                .unwrap_or(&cx.theme().chart_2.opacity(0.4).into());

            let stroke = *self.strokes.get(i).unwrap_or(&cx.theme().chart_2);

            let stroke_style = *self
                .stroke_styles
                .get(i)
                .unwrap_or(self.stroke_styles.first().unwrap_or(&Default::default()));

            Area::new()
                .data(&self.data)
                .x(move |d| x.tick(&x_fn(d)))
                .y0(height)
                .y1(move |d| y.tick(&y_fn(d)))
                .stroke(stroke)
                .stroke_style(stroke_style)
                .fill(fill)
                .paint(&bounds, window);
        }
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
        let x_fn = self.x.as_ref()?;
        let (x, y) = self.scales(bounds)?;

        // Ignore the x-axis label gutter so hovering the labels doesn't show a tooltip.
        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        if position.y.as_f32() > bounds.size.height.as_f32() - axis_gap {
            return None;
        }

        let index = x.least_index(position.x.as_f32());
        let d = self.data.get(index)?;
        let x_tick = x.tick(&x_fn(d))?;

        // One dot per series at the hovered x.
        let dots = self
            .y
            .iter()
            .filter_map(|y_fn| Some(point(px(x_tick), px(y.tick(&y_fn(d))?))))
            .collect();

        Some(TooltipState::new(
            index,
            point(px(x_tick), position.y),
            dots,
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
        let x_fn = self.x.as_ref()?;
        let d = self.data.get(state.index)?;
        let title: SharedString = x_fn(d).into();

        let default_color = cx.theme().chart_2;
        let dot_stroke = cx.theme().background;
        let color = |i: usize| *self.strokes.get(i).unwrap_or(&default_color);

        // Follow the cursor; the crosshair and dots stay snapped to the data point.
        let mut tooltip = Tooltip::new(cursor, bounds.size)
            .gap(px(8.))
            // Confine the crosshair to the plot area so it doesn't cross the x-axis.
            .cross_line(
                CrossLine::new(state.cross_line)
                    .height(bounds.size.height.as_f32() - if self.x_axis { AXIS_GAP } else { 0. }),
            )
            .dots(
                state
                    .dots
                    .iter()
                    .enumerate()
                    .map(|(i, p)| Dot::new(*p).stroke(dot_stroke).fill(color(i))),
            )
            .title(title);

        // One row per series: swatch + label + value.
        for (i, y_fn) in self.y.iter().enumerate() {
            let name = self.names.get(i).cloned().unwrap_or_default();
            let value = y_fn(d).to_f64()?;
            tooltip = tooltip.row(color(i), name, format!("{}", value));
        }

        Some(tooltip.into_any_element())
    }
}
