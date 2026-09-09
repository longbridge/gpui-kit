use std::{hash::Hash, ops::RangeInclusive, rc::Rc};

use gpui::{
    AnyElement, App, Background, Bounds, Corners, ElementId, Hsla, IntoElement, LinearColorStop,
    Pixels, Point, SharedString, Size, TextAlign, Window, linear_gradient, point, px,
};
use gpui_component_macros::IntoPlot;
use num_traits::{Num, ToPrimitive};

use crate::{
    ActiveTheme,
    plot::{
        AXIS_GAP, AxisLabelSide, AxisText, Grid, Plot, PlotAxis, PlotLabel,
        label::{TEXT_GAP, TEXT_SIZE, Text, measure_text_width},
        scale::{Scale, ScaleBand, ScaleLinear, Sealed},
        shape::{Bar, BarAlignment},
        tooltip::{CrossLine, Tooltip, TooltipState},
    },
};

use super::build_band_labels;

/// Space reserved along the band axis for the value-axis tick labels, in pixels.
///
/// Like [`AXIS_GAP`] this is a fixed budget rather than a measured one: the band
/// scale is also rebuilt during hit-testing, where no [`Window`] is available to
/// shape text. Values wider than this (very large numbers) will overflow it.
const VALUE_AXIS_GAP: f32 = 32.;

#[derive(IntoPlot)]
pub struct BarChart<T, B, V>
where
    T: 'static,
    B: Eq + Hash + Into<SharedString> + 'static,
    V: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    data: Vec<T>,
    band: Option<Rc<dyn Fn(&T) -> B>>,
    value: Option<Rc<dyn Fn(&T) -> V>>,
    fill: Option<Rc<dyn Fn(&T, Bounds<f32>, Bounds<f32>, BarAlignment) -> Background>>,
    #[allow(clippy::type_complexity)]
    fill_gradient:
        Option<Rc<dyn Fn(&T, RangeInclusive<f32>, &dyn Fn(f32) -> f32) -> [LinearColorStop; 2]>>,
    tick_margin: usize,
    label: Option<Rc<dyn Fn(&T) -> SharedString>>,
    label_axis: bool,
    value_axis: bool,
    value_tick_count: usize,
    grid: bool,
    alignment: BarAlignment,
    corner_radii: Corners<Pixels>,
    id: Option<ElementId>,
    name: Option<SharedString>,
}

impl<T, B, V> BarChart<T, B, V>
where
    B: Eq + Hash + Into<SharedString> + 'static,
    V: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    pub fn new<I>(data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            data: data.into_iter().collect(),
            band: None,
            value: None,
            fill: None,
            fill_gradient: None,
            tick_margin: 1,
            label: None,
            label_axis: true,
            value_axis: false,
            value_tick_count: 4,
            grid: true,
            alignment: BarAlignment::default(),
            corner_radii: Corners::all(px(0.)),
            id: None,
            name: None,
        }
    }

    /// Enable an interactive hover tooltip (crosshair + category/value) for this chart.
    ///
    /// The `id` must be unique among sibling elements. Without it, the chart stays a
    /// non-interactive plot. Works for every [`BarAlignment`] (vertical bars get a
    /// vertical crosshair, horizontal bars a horizontal one).
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the series name shown in the hover tooltip row (e.g. "Desktop").
    pub fn name(mut self, name: impl Into<SharedString>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Map each datum to its band-axis value (the categorical/ordinal axis).
    pub fn band(mut self, band: impl Fn(&T) -> B + 'static) -> Self {
        self.band = Some(Rc::new(band));
        self
    }

    /// Map each datum to its numeric value along the value axis.
    pub fn value(mut self, value: impl Fn(&T) -> V + 'static) -> Self {
        self.value = Some(Rc::new(value));
        self
    }

    /// Set a per-datum verbatim fill.
    ///
    /// The closure receives:
    ///
    /// 1. the datum,
    /// 2. the **bar's bounds** in pixel space, expressed relative to the
    ///    chart's origin (i.e. the bar's painted rectangle within the chart),
    /// 3. the **chart's bounds** in pixel space with origin `(0, 0)` and size
    ///    equal to the full chart extent, and
    /// 4. the bar's [`BarAlignment`] (so callers can branch on orientation,
    ///    e.g. flip a gradient angle).
    ///
    /// Both rectangles share the same coordinate system, so callers can
    /// implement arbitrary chart-aware backgrounds — bar-local gradients,
    /// chart-wide gradients, patterns, sampled colormaps, etc. — without any
    /// help from the library.
    ///
    /// Accepts any type convertible to [`Background`]. Setting this clears any
    /// previously set [`BarChart::fill_gradient`].
    pub fn fill<Bg>(
        mut self,
        fill: impl Fn(&T, Bounds<f32>, Bounds<f32>, BarAlignment) -> Bg + 'static,
    ) -> Self
    where
        Bg: Into<Background> + 'static,
    {
        self.fill = Some(Rc::new(move |t, bar_bounds, chart_bounds, alignment| {
            fill(t, bar_bounds, chart_bounds, alignment).into()
        }));
        self.fill_gradient = None;
        self
    }

    /// Set a per-datum auto-oriented linear gradient fill.
    ///
    /// The closure receives the datum, the chart's full data range
    /// (`chart_range`, derived from all data values), and a `chart_to_bar`
    /// remap helper that maps a chart-value coordinate to a bar-local
    /// gradient position (where `0.0` is the bar's base and `1.0` is its tip).
    ///
    /// Use bar-local positions directly for per-bar gradients (every bar
    /// looks the same regardless of its value):
    ///
    /// ```ignore
    /// .fill_gradient(|_, _, _| [
    ///     linear_color_stop(c.opacity(0.3), 0.0),
    ///     linear_color_stop(c, 1.0),
    /// ])
    /// ```
    ///
    /// Or use `chart_to_bar` to position stops at chart-relative values, so
    /// each bar shows the slice of a chart-wide gradient corresponding to
    /// its own `[base, value]` span:
    ///
    /// ```ignore
    /// .fill_gradient(|_, chart_range, chart_to_bar| [
    ///     linear_color_stop(c.opacity(0.3), chart_to_bar(*chart_range.start())),
    ///     linear_color_stop(c,              chart_to_bar(*chart_range.end())),
    /// ])
    /// ```
    ///
    /// Stop positions returned outside `[0, 1]` are clipped to the bar; the
    /// library interpolates colors at the clip points so the on-bar gradient
    /// still matches the chart-wide one.
    ///
    /// The gradient angle is derived from [`BarAlignment`] so stop-0 is at the
    /// base and stop-1 at the tip. Setting this clears any previously set
    /// [`BarChart::fill`].
    pub fn fill_gradient(
        mut self,
        fill: impl Fn(&T, RangeInclusive<f32>, &dyn Fn(f32) -> f32) -> [LinearColorStop; 2] + 'static,
    ) -> Self {
        self.fill_gradient = Some(Rc::new(fill));
        self.fill = None;
        self
    }

    pub fn tick_margin(mut self, tick_margin: usize) -> Self {
        self.tick_margin = tick_margin;
        self
    }

    pub fn label<S>(mut self, label: impl Fn(&T) -> S + 'static) -> Self
    where
        S: Into<SharedString> + 'static,
    {
        self.label = Some(Rc::new(move |t| label(t).into()));
        self
    }

    /// Show or hide the band-axis line and labels.
    ///
    /// Default is true.
    pub fn label_axis(mut self, label_axis: bool) -> Self {
        self.label_axis = label_axis;
        self
    }

    /// Show or hide the value-axis tick labels.
    ///
    /// Enabling this reserves [`VALUE_AXIS_GAP`] along the band axis (left of
    /// vertical bars, below horizontal ones) for the labels.
    ///
    /// Default is false.
    pub fn value_axis(mut self, value_axis: bool) -> Self {
        self.value_axis = value_axis;
        self
    }

    /// Set how many even intervals the value axis is divided into, which drives
    /// both the grid line spacing and the value-axis tick labels.
    ///
    /// This is a count, unlike [`Self::tick_margin`], which is a stride over the
    /// band axis categories.
    ///
    /// Default is 4.
    pub fn value_tick_count(mut self, value_tick_count: usize) -> Self {
        self.value_tick_count = value_tick_count.max(1);
        self
    }

    pub fn grid(mut self, grid: bool) -> Self {
        self.grid = grid;
        self
    }

    /// Set the bar alignment.
    ///
    /// Default is [`BarAlignment::Bottom`].
    pub fn alignment(mut self, alignment: BarAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set the corner radii applied to every bar rectangle.
    ///
    /// Use [`Corners::all`] for uniform rounding, or construct [`Corners`] manually
    /// to round only specific corners (e.g. just the tip end of each bar).
    pub fn corner_radii(mut self, corner_radii: impl Into<Corners<Pixels>>) -> Self {
        self.corner_radii = corner_radii.into();
        self
    }

    /// The band scale (matching `paint`): spans the height for horizontal bars, the width
    /// otherwise. Shared by `tooltip_state` and `tooltip`.
    fn band_scale(&self, bounds: Bounds<Pixels>) -> Option<ScaleBand<B>> {
        let band_fn = self.band.as_ref()?;
        let band_extent = if self.alignment.is_horizontal() {
            bounds.size.height.as_f32()
        } else {
            bounds.size.width.as_f32()
        };
        // Value-axis labels eat into the band extent at one end; `band_offset`
        // shifts the bands away from that end when it is the leading one.
        let gap = if self.value_axis { VALUE_AXIS_GAP } else { 0. };
        Some(
            ScaleBand::new(
                self.data.iter().map(|v| band_fn(v)).collect(),
                vec![0., (band_extent - gap).max(0.)],
            )
            .padding_inner(0.4)
            .padding_outer(0.2),
        )
    }

    /// Offset added to every band-scale tick.
    ///
    /// [`ScaleBand`] ignores the start of its range, so vertical bars are shifted
    /// by hand to clear the value-axis labels on their left. Horizontal bars put
    /// those labels below the plot, past the end of the band axis, so they need no
    /// shift.
    fn band_offset(&self) -> f32 {
        if self.value_axis && !self.alignment.is_horizontal() {
            VALUE_AXIS_GAP
        } else {
            0.
        }
    }

    /// Label gaps `(band_side, value_end_side)` reserved along the value axis for
    /// horizontal bars, measured from the actual label text. Shared by `paint` and the
    /// tooltip so the crosshair lines up with the bar region.
    fn horizontal_gaps(&self, window: &mut Window) -> (f32, f32) {
        let Some(band_fn) = self.band.as_ref() else {
            return (0., 0.);
        };
        let font_size = px(TEXT_SIZE);
        let band_gap = if self.label_axis {
            self.data
                .iter()
                .map(|v| {
                    let s: SharedString = band_fn(v).into();
                    measure_text_width(&s, font_size, window)
                })
                .fold(0f32, f32::max)
                + TEXT_GAP * 2.
        } else {
            0.
        };
        let value_end_gap = if let Some(label_fn) = self.label.as_ref() {
            self.data
                .iter()
                .map(|v| measure_text_width(&label_fn(v), font_size, window))
                .fold(0f32, f32::max)
                + TEXT_GAP * 2.
        } else {
            TEXT_GAP * 4.
        };
        (band_gap, value_end_gap)
    }
}

impl<T, B, V> Plot for BarChart<T, B, V>
where
    B: Eq + Hash + Into<SharedString> + 'static,
    V: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let (Some(band_fn), Some(value_fn)) = (self.band.as_ref(), self.value.as_ref()) else {
            return;
        };

        let total_width = bounds.size.width.as_f32();
        let total_height = bounds.size.height.as_f32();
        let axis_gap = if self.label_axis { AXIS_GAP } else { 0. };
        let alignment = self.alignment;
        let is_horizontal = alignment.is_horizontal();

        // Band scale spans the full extent perpendicular to the value axis. Shared with the
        // tooltip via `band_scale()` so the bars and the hover crosshair stay aligned.
        let Some(band_scale) = self.band_scale(bounds) else {
            return;
        };
        let band_width = band_scale.band_width();

        let value_dim = if is_horizontal {
            total_width
        } else {
            total_height
        };
        // For horizontal charts the band labels (category names) are rendered
        // along the value axis and can be arbitrarily wide, so we measure the
        // actual maximum label width instead of using a fixed constant.
        // Similarly, value labels (numbers) at the bar ends are measured so the
        // scale range is always shrunk by exactly the right amount.
        let (band_gap, value_end_gap) = if is_horizontal {
            self.horizontal_gaps(window)
        } else {
            (axis_gap, 10.)
        };
        let (range, baseline) = match alignment {
            BarAlignment::Bottom => {
                let baseline = value_dim - axis_gap;
                (vec![baseline, 10.], baseline)
            }
            BarAlignment::Top => {
                let baseline = axis_gap;
                (vec![baseline, value_dim - 10.], baseline)
            }
            BarAlignment::Left => {
                let baseline = band_gap;
                (vec![baseline, value_dim - value_end_gap], baseline)
            }
            BarAlignment::Right => {
                let baseline = value_dim - band_gap;
                (vec![baseline, value_end_gap], baseline)
            }
        };
        let value_scale = ScaleLinear::new(
            self.data
                .iter()
                .map(|v| value_fn(v))
                .chain(Some(V::zero()))
                .collect(),
            range,
        );

        // Where zero sits along the value axis. Bars grow from here rather than from
        // the geometric baseline, so negative values extend to the opposite side. With
        // no negative data zero is the domain minimum and this is the baseline.
        let zero_pixel = value_scale.tick(&V::zero()).unwrap_or(baseline);
        let band_offset = self.band_offset();

        // Grid lines and the zero line span their bounds edge to edge, so they are
        // painted into bounds inset by the value-axis gap. Without this they run
        // straight through the value-axis labels.
        let value_axis_gap = if self.value_axis { VALUE_AXIS_GAP } else { 0. };
        let plot_bounds = if is_horizontal {
            Bounds {
                origin: bounds.origin,
                size: Size::new(bounds.size.width, bounds.size.height - px(value_axis_gap)),
            }
        } else {
            Bounds {
                origin: bounds.origin + point(px(value_axis_gap), px(0.)),
                size: Size::new(bounds.size.width - px(value_axis_gap), bounds.size.height),
            }
        };

        // Value domain, matching `value_scale`'s (which is the data plus zero).
        // `far` maps to the maximum and `baseline` to the minimum.
        let (domain_lo, domain_hi) = self.data.iter().fold((0.0_f32, 0.0_f32), |(lo, hi), v| {
            let f = value_fn(v).to_f32().unwrap_or(0.);
            (lo.min(f), hi.max(f))
        });

        // Draw band axis (with categorical labels).
        let mut axis = PlotAxis::new().stroke(cx.theme().border);
        if self.label_axis {
            match alignment {
                BarAlignment::Bottom | BarAlignment::Top => {
                    axis = axis.x(zero_pixel);

                    // Labels are placed one at a time rather than through
                    // `x_label`, because a chart with negative values needs them
                    // on either side of the zero line: each label goes on the side
                    // its own bar leaves empty.
                    let labels = self
                        .data
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| (i + 1) % self.tick_margin == 0)
                        .filter_map(|(_, d)| {
                            let band_x = band_scale.tick(&band_fn(d))?;
                            let value = value_fn(d).to_f32().unwrap_or(0.);
                            let label_y = if label_below_zero_line(value, alignment) {
                                zero_pixel + TEXT_GAP
                            } else {
                                zero_pixel - TEXT_GAP - TEXT_SIZE
                            };

                            Some(
                                Text::new(
                                    band_fn(d).into(),
                                    point(px(band_x + band_offset + band_width / 2.), px(label_y)),
                                    cx.theme().muted_foreground,
                                )
                                .align(TextAlign::Center),
                            )
                        })
                        .collect();
                    PlotLabel::new(labels).paint(&bounds, window, cx);
                }
                BarAlignment::Left | BarAlignment::Right => {
                    let labels = build_band_labels(
                        &self.data,
                        band_fn.as_ref(),
                        &band_scale,
                        band_width,
                        self.tick_margin,
                        cx.theme().muted_foreground,
                    );
                    let (side, align) = if matches!(alignment, BarAlignment::Left) {
                        (AxisLabelSide::Start, TextAlign::Right)
                    } else {
                        (AxisLabelSide::End, TextAlign::Left)
                    };
                    axis = axis
                        .y(zero_pixel)
                        .y_label_side(side)
                        .y_label(labels.into_iter().map(|t| t.align(align)));
                }
            }
        }
        axis.paint(&plot_bounds, window, cx);

        // Far edge of the value axis in pixel space (opposite the baseline).
        let far = match alignment {
            BarAlignment::Bottom => 10.,
            BarAlignment::Top => value_dim - 10.,
            BarAlignment::Left => value_dim - value_end_gap,
            BarAlignment::Right => value_end_gap,
        };

        let steps = self.value_tick_count;
        let value_ticks = value_tick_positions(far, baseline, steps);

        // Draw grid, excluding the line at the baseline.
        if self.grid {
            let grid = Grid::new()
                .stroke(cx.theme().border)
                .dash_array(&[px(4.), px(2.)]);
            let lines = value_ticks[..steps].to_vec();
            let grid = if is_horizontal {
                grid.x(lines)
            } else {
                grid.y(lines)
            };
            grid.paint(&plot_bounds, window);
        }

        if self.value_axis {
            // Ticks run from `far` (the domain maximum) to `baseline` (the minimum),
            // so the labels walk the domain in the same direction.
            let labels = value_ticks.iter().enumerate().map(|(i, &tick)| {
                let value = domain_hi - (domain_hi - domain_lo) * i as f32 / steps as f32;
                AxisText::new(format_tick(value), px(tick), cx.theme().muted_foreground)
            });

            // The labels go in the gap `band_scale` kept clear for them, right-aligned
            // against the plot area for vertical bars and centred under it otherwise.
            let value_axis = if is_horizontal {
                PlotAxis::new()
                    .x_axis(false)
                    .x(px(total_height - VALUE_AXIS_GAP))
                    .x_label(labels.map(|t| t.align(TextAlign::Center)))
            } else {
                PlotAxis::new()
                    .y_axis(false)
                    .y(px(VALUE_AXIS_GAP - TEXT_GAP * 2.))
                    .y_label(labels.map(|t| t.align(TextAlign::Right)))
            };
            value_axis.paint(&bounds, window, cx);
        }

        // Draw bars.
        let band_fn_cloned = band_fn.clone();
        let value_fn_cloned = value_fn.clone();
        let default_fill: Background = cx.theme().chart_2.into();
        let fill = self.fill.clone();
        let fill_gradient = self.fill_gradient.clone();
        let label_color = cx.theme().foreground;

        // Chart bounds in pixel space, with origin (0, 0) and size equal to
        // the full chart extent. Passed to user `fill` closures so they can
        // position chart-wide backgrounds (gradients, patterns, etc.).
        let chart_bounds: Bounds<f32> = Bounds {
            origin: Point::new(0., 0.),
            size: Size::new(total_width, total_height),
        };

        // Chart data range in f32 — passed to `fill_gradient` callers and used
        // by the `chart_to_bar` remap helper.
        let chart_range = {
            let mut lo = 0.0_f32;
            let mut hi = 0.0_f32;
            for v in &self.data {
                if let Some(f) = value_fn(v).to_f32() {
                    lo = lo.min(f);
                    hi = hi.max(f);
                }
            }
            lo..=hi
        };

        let mut bar = Bar::new()
            .data(&self.data)
            .alignment(alignment)
            .band_width(band_width)
            .cross(move |d| band_scale.tick(&band_fn_cloned(d)).map(|t| t + band_offset))
            .base(move |_| zero_pixel)
            .value(move |d| value_scale.tick(&value_fn_cloned(d)))
            .corner_radii(self.corner_radii);

        bar = match (fill, fill_gradient) {
            (_, Some(fg)) => {
                let value_fn_for_grad = value_fn.clone();
                bar.fill(move |d, _frame, alignment| {
                    let v = value_fn_for_grad(d).to_f32().unwrap_or(0.);
                    let base_v = 0.0_f32;
                    let bar_lo = base_v.min(v);
                    let bar_hi = base_v.max(v);
                    let bar_span = (bar_hi - bar_lo).max(f32::EPSILON);
                    let chart_to_bar = |chart_value: f32| (chart_value - bar_lo) / bar_span;
                    let stops = fg(d, chart_range.clone(), &chart_to_bar);
                    let [s0, s1] = clip_stops_to_bar(stops);
                    let bg: Background = linear_gradient(alignment.gradient_angle(), s0, s1);
                    bg
                })
            }
            (Some(f), _) => {
                bar.fill(move |d, frame, alignment| f(d, frame, chart_bounds, alignment))
            }
            _ => bar.fill(move |_, _, _| default_fill),
        };

        if let Some(label) = self.label.as_ref() {
            let label = label.clone();
            let text_align = match alignment {
                BarAlignment::Bottom | BarAlignment::Top => TextAlign::Center,
                BarAlignment::Left => TextAlign::Left,
                BarAlignment::Right => TextAlign::Right,
            };
            bar =
                bar.label(move |d, p| vec![Text::new(label(d), p, label_color).align(text_align)]);
        }

        bar.paint(&bounds, window, cx);
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
        let band_fn = self.band.as_ref()?;
        self.value.as_ref()?;

        // Only the band scale is needed to hit-test which bar is hovered, so no text
        // measurement (and thus no `window`) is required.
        let is_horizontal = self.alignment.is_horizontal();
        let band_scale = self.band_scale(bounds)?;
        let band_width = band_scale.band_width();

        let band_offset = self.band_offset();
        let cursor_band = if is_horizontal {
            position.y
        } else {
            position.x
        };
        let index = band_scale.least_index(cursor_band.as_f32() - band_offset);
        let d = self.data.get(index)?;
        let center = band_scale.tick(&band_fn(d))? + band_offset + band_width / 2.;

        // Vertical bars: vertical crosshair at the bar's x. Horizontal bars: horizontal
        // crosshair at the bar's y. The box tracks the cursor either way.
        let cross_line = if is_horizontal {
            point(position.x, px(center))
        } else {
            point(px(center), position.y)
        };

        Some(TooltipState::new(index, cross_line, vec![]))
    }

    fn tooltip(
        &self,
        state: &TooltipState,
        cursor: Point<Pixels>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let (band_fn, value_fn) = (self.band.as_ref()?, self.value.as_ref()?);
        let d = self.data.get(state.index)?;
        let title: SharedString = band_fn(d).into();
        let value = value_fn(d).to_f64()?;
        let name = self.name.clone().unwrap_or_default();

        // Highlight the hovered bar with a translucent band the width of the bar, instead
        // of a hairline. Confined to the plot area so it doesn't cover the axis labels.
        let band_width = self.band_scale(bounds)?.band_width();
        let cross_line = if self.alignment.is_horizontal() {
            let (band_gap, value_end_gap) = self.horizontal_gaps(window);
            let length = (bounds.size.width.as_f32() - band_gap - value_end_gap).max(0.);
            let start = if matches!(self.alignment, BarAlignment::Left) {
                band_gap
            } else {
                value_end_gap
            };
            // Skip the tooltip when the cursor is over the axis labels, not a bar.
            let value_labels_top = bounds.size.height.as_f32() - VALUE_AXIS_GAP;
            if cursor.x.as_f32() < start
                || cursor.x.as_f32() > start + length
                || (self.value_axis && cursor.y.as_f32() > value_labels_top)
            {
                return None;
            }
            CrossLine::new(state.cross_line)
                .horizontal()
                .h_span(start, length)
                .band(px(band_width))
        } else {
            let axis_gap = if self.label_axis { AXIS_GAP } else { 0. };
            let length = bounds.size.height.as_f32() - axis_gap;
            let start = if matches!(self.alignment, BarAlignment::Top) {
                axis_gap
            } else {
                0.
            };
            // Skip the tooltip when the cursor is over the axis labels, not a bar.
            if cursor.y.as_f32() < start
                || cursor.y.as_f32() > start + length
                || cursor.x.as_f32() < self.band_offset()
            {
                return None;
            }
            CrossLine::new(state.cross_line)
                .span(start, length)
                .band(px(band_width))
        };

        Some(
            // Follow the cursor; the highlight band stays snapped to the bar.
            Tooltip::new(cursor, bounds.size)
                .gap(px(8.))
                .cross_line(cross_line)
                .title(title)
                .row(cx.theme().chart_2, name, format!("{}", value))
                .into_any_element(),
        )
    }
}

/// Clip a two-stop gradient to bar-local `[0, 1]`, interpolating colors at the
/// clip points so the on-bar gradient matches the (possibly broader) gradient
/// the caller defined.
///
/// When a stop position falls outside `[0, 1]` (e.g. because `chart_to_bar`
/// returned a value past the bar's edge for a chart-relative gradient),
/// gpui's renderer would clamp the position and lose the gradient effect.
/// This function instead replaces such a stop with the color sampled along
/// the line through both stops at position `0.0` or `1.0`, preserving the
/// visual slice.
fn clip_stops_to_bar(stops: [LinearColorStop; 2]) -> [LinearColorStop; 2] {
    let [a, b] = stops;
    let p0 = a.percentage;
    let p1 = b.percentage;
    let lerp = |t: f32| -> Hsla {
        Hsla {
            h: a.color.h + (b.color.h - a.color.h) * t,
            s: a.color.s + (b.color.s - a.color.s) * t,
            l: a.color.l + (b.color.l - a.color.l) * t,
            a: a.color.a + (b.color.a - a.color.a) * t,
        }
    };
    let span = p1 - p0;
    let sample = |target: f32| -> Hsla {
        if span.abs() < f32::EPSILON {
            a.color
        } else {
            lerp((target - p0) / span)
        }
    };
    let new_a = if (0. ..=1.).contains(&p0) {
        a
    } else {
        LinearColorStop {
            color: sample(p0.clamp(0., 1.)),
            percentage: p0.clamp(0., 1.),
        }
    };
    let new_b = if (0. ..=1.).contains(&p1) {
        b
    } else {
        LinearColorStop {
            color: sample(p1.clamp(0., 1.)),
            percentage: p1.clamp(0., 1.),
        }
    };
    [new_a, new_b]
}

/// Format a tick value for display on the value axis.
fn format_tick(v: f32) -> String {
    if (v - v.round()).abs() < 0.001 {
        format!("{:.0}", v)
    } else {
        format!("{:.1}", v)
    }
}

/// Whether a vertical bar's category label belongs below the zero line.
///
/// A bar grows away from the zero line, so its label goes on the side the bar
/// leaves empty. Which side that is flips with both the sign of the value and the
/// alignment. A zero-length bar counts as positive, which puts its label in the
/// axis gap rather than inside the plot.
fn label_below_zero_line(value: f32, alignment: BarAlignment) -> bool {
    (value < 0.) == (alignment == BarAlignment::Top)
}

/// Tick positions along the value axis, dividing it into `steps` even intervals.
///
/// Runs from `far` (the value domain's maximum) through `baseline` (its minimum)
/// inclusive, so the result holds `steps + 1` positions and the last one is the
/// baseline.
fn value_tick_positions(far: f32, baseline: f32, steps: usize) -> Vec<f32> {
    (0..=steps)
        .map(|i| far + (baseline - far) * i as f32 / steps as f32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_below_zero_line() {
        // Bottom-aligned: positive bars grow up, leaving the space below free.
        assert!(label_below_zero_line(5., BarAlignment::Bottom));
        assert!(label_below_zero_line(0., BarAlignment::Bottom));
        assert!(!label_below_zero_line(-5., BarAlignment::Bottom));

        // Top-aligned bars grow the other way, so the sides swap.
        assert!(!label_below_zero_line(5., BarAlignment::Top));
        assert!(!label_below_zero_line(0., BarAlignment::Top));
        assert!(label_below_zero_line(-5., BarAlignment::Top));
    }

    #[test]
    fn test_value_tick_positions() {
        // Both ends are included, so 4 intervals means 5 positions.
        assert_eq!(
            value_tick_positions(10., 110., 4),
            vec![10., 35., 60., 85., 110.]
        );

        // Top-aligned charts have the baseline before the far edge.
        assert_eq!(value_tick_positions(110., 10., 2), vec![110., 60., 10.]);

        assert_eq!(value_tick_positions(0., 50., 1), vec![0., 50.]);
    }
}
