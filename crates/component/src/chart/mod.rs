mod area_chart;
mod bar_chart;
mod candlestick_chart;
mod line_chart;
mod pie_chart;
mod radar_chart;
mod sankey_chart;

pub use area_chart::AreaChart;
pub use bar_chart::BarChart;
pub use candlestick_chart::CandlestickChart;
pub use line_chart::LineChart;
pub use pie_chart::PieChart;
pub use radar_chart::{RadarChart, RadarLabel};
pub use sankey_chart::{SankeyChart, SankeyLabel};

use std::{hash::Hash, panic::Location, rc::Rc};

use gpui::{
    App, Bounds, ContentMask, ElementId, Hsla, Pixels, SharedString, Size, TextAlign, Window,
    point, px,
};
use num_traits::{Num, ToPrimitive};

use crate::{
    ActiveTheme,
    plot::{
        AxisLabelPlacement, AxisText, Grid, PlotLabel,
        label::{TEXT_GAP, TEXT_HEIGHT, TEXT_SIZE, Text, measure_text_width},
        scale::{Scale, ScaleBand, ScaleLinear, ScalePoint, Sealed},
    },
};

/// The [`ElementId`] a chart carries when the caller names none: the source
/// location it was constructed at.
///
/// The crosshair, the hover lift, the tooltip and the path cache all need an id
/// unique among siblings, and a chart that has to be handed one per call site is
/// a chart every caller leaves static. One construction site written out once,
/// which is nearly every chart, is unique by construction.
///
/// The exception is one site rendering several charts as siblings, where every
/// copy shares this location and therefore one hover state and one path cache.
/// A `GlobalElementId` is the whole id stack, so rows that carry their own id —
/// which `List` and `uniform_list` give them — already separate the copies
/// underneath them; only id-less siblings collide, and those name an id with
/// `id`. GPUI takes this same trade-off for [`gpui::Window::use_state`].
#[track_caller]
pub(crate) fn caller_id() -> ElementId {
    ElementId::CodeLocation(*Location::caller())
}

/// The size of the dot marking the hovered data point.
pub(crate) const HOVER_DOT_SIZE: Pixels = px(8.);

/// The ring behind the hovered dot at full focus; a [`Tooltip`] grows it out
/// of the dot as the hover fades in.
///
/// [`Tooltip`]: crate::plot::tooltip::Tooltip
pub(crate) const HOVER_HALO_SIZE: Pixels = px(20.);

/// How many points the x axis of a point chart (`LineChart`, `AreaChart`) is
/// laid out for: `point_count`, or the data's own length when that is unset or
/// smaller.
pub(crate) fn axis_point_count(point_count: Option<usize>, data_len: usize) -> usize {
    point_count.unwrap_or(data_len).max(data_len)
}

/// The x range a point scale spreads `data_len` points over, when the axis is
/// laid out for `point_count` of them across `width` pixels from `start`.
///
/// The data takes the leading points, so each keeps its place as the data grows.
pub(crate) fn point_range(start: f32, width: f32, data_len: usize, point_count: usize) -> Vec<f32> {
    let end = if point_count > 1 {
        width * data_len.saturating_sub(1) as f32 / (point_count - 1) as f32
    } else {
        width
    };
    vec![start, start + end]
}

/// The value range a y scale spans and the pixel range it maps onto, kept in
/// `f64` so tick labels can read the value at any height of the plot.
#[derive(Clone, Copy)]
pub(crate) struct ValueExtent {
    lo: f64,
    hi: f64,
    bottom: f32,
    top: f32,
}

impl ValueExtent {
    /// The value the scale puts at pixel `y`.
    pub(crate) fn value_at(&self, y: f32) -> f64 {
        if self.bottom == self.top {
            return self.lo;
        }
        self.lo + (self.hi - self.lo) * ((self.bottom - y) / (self.bottom - self.top)) as f64
    }

    /// The pixel the scale puts `value` at, or `None` for a scale with no extent.
    pub(crate) fn position_of(&self, value: f64) -> Option<f32> {
        if self.hi == self.lo {
            return None;
        }
        Some(
            self.bottom
                - ((value - self.lo) / (self.hi - self.lo)) as f32 * (self.bottom - self.top),
        )
    }
}

/// The y scale of a point chart, from `height` less the bottom `padding` up to
/// the top `padding`.
///
/// A pinned `domain` maps its ends onto that range; otherwise the scale fits
/// `values` from zero.
pub(crate) fn point_value_scale<Y>(
    values: impl IntoIterator<Item = Y>,
    domain: Option<(Y, Y)>,
    height: f32,
    (top, bottom): (f32, f32),
) -> (ScaleLinear<Y>, ValueExtent)
where
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed,
{
    let domain: Vec<Y> = match domain {
        Some((min, max)) => vec![min, max],
        None => values.into_iter().chain(Some(Y::zero())).collect(),
    };
    let (lo, hi) = domain
        .iter()
        .filter_map(|v| v.to_f64())
        .fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)));
    let extent = ValueExtent {
        lo,
        hi,
        bottom: height - bottom,
        top,
    };
    (ScaleLinear::new(domain, vec![height - bottom, top]), extent)
}

/// The least space kept beside the plot for value-axis tick labels drawn
/// outside it, in pixels; wider labels widen it (see [`value_axis_gap`]).
pub(crate) const VALUE_AXIS_GAP: f32 = 32.;

/// The gutter value-axis tick `labels` drawn outside the plot need: the widest
/// label and the gap before the plot, and never less than [`VALUE_AXIS_GAP`].
///
/// Measured in `prepaint`, which runs before `tooltip_state` and `paint`, so
/// hit-testing and painting share one gutter.
pub(crate) fn value_axis_gap(
    labels: impl IntoIterator<Item = SharedString>,
    window: &mut Window,
) -> f32 {
    labels
        .into_iter()
        .map(|label| measure_text_width(&label, px(TEXT_SIZE), window) + TEXT_GAP * 2.)
        .fold(VALUE_AXIS_GAP, f32::max)
}

/// A caller's tick label text for a value.
pub(crate) type TickFormat = Rc<dyn Fn(f64) -> SharedString>;

/// The default tick label: whole numbers bare, the rest to one decimal.
pub(crate) fn format_tick(value: f64) -> SharedString {
    if (value - value.round()).abs() < 0.001 {
        format!("{:.0}", value).into()
    } else {
        format!("{:.1}", value).into()
    }
}

/// Which of `len` items carry a category label: `label_count` of them evenly
/// spread from the first to the last when set, otherwise every `tick_margin`-th.
pub(crate) fn labeled_items(
    len: usize,
    label_count: Option<usize>,
    tick_margin: usize,
) -> Vec<bool> {
    match label_count {
        Some(count) => {
            let mut labeled = vec![false; len];
            match count {
                0 => {}
                1 => labeled.iter_mut().take(1).for_each(|l| *l = true),
                count if count >= len => labeled.iter_mut().for_each(|l| *l = true),
                count => {
                    for k in 0..count {
                        let ix =
                            (k as f32 * (len - 1) as f32 / (count - 1) as f32).round() as usize;
                        labeled[ix] = true;
                    }
                }
            }
            labeled
        }
        None => (0..len).map(|i| (i + 1) % tick_margin == 0).collect(),
    }
}

/// The grid, value-axis labels and reference lines a point chart (`LineChart`,
/// `AreaChart`) draws, and the builders both charts forward to it.
pub(crate) struct PointAxes {
    pub(crate) y_axis: bool,
    pub(crate) y_axis_label_placement: AxisLabelPlacement,
    pub(crate) y_tick_count: usize,
    pub(crate) y_tick_format: Option<TickFormat>,
    pub(crate) x_tick_count: Option<usize>,
    pub(crate) grid_columns: usize,
    pub(crate) grid_dashed: bool,
    pub(crate) y_padding: (f32, f32),
    pub(crate) reference_lines: Vec<f64>,
    /// The value-axis gutter measured in `prepaint`; see [`value_axis_gap`].
    y_label_gap: f32,
}

impl Default for PointAxes {
    fn default() -> Self {
        Self {
            y_axis: false,
            y_axis_label_placement: AxisLabelPlacement::default(),
            y_tick_count: 5,
            y_tick_format: None,
            x_tick_count: None,
            grid_columns: 0,
            grid_dashed: true,
            y_padding: (10., 0.),
            reference_lines: vec![],
            y_label_gap: VALUE_AXIS_GAP,
        }
    }
}

impl PointAxes {
    /// Where the plot starts along x: past the value-axis gutter when the labels
    /// sit outside it.
    pub(crate) fn plot_left(&self) -> f32 {
        if self.y_axis && self.y_axis_label_placement == AxisLabelPlacement::Outside {
            self.y_label_gap
        } else {
            0.
        }
    }

    /// Measure the gutter the y labels need outside the plot, before the x
    /// scale is laid out past it.
    pub(crate) fn measure_y_labels(
        &mut self,
        extent: ValueExtent,
        height: f32,
        window: &mut Window,
    ) {
        if self.y_axis && self.y_axis_label_placement == AxisLabelPlacement::Outside {
            let labels = self
                .y_tick_labels(extent, height)
                .into_iter()
                .map(|(_, text)| text);
            self.y_label_gap = value_axis_gap(labels, window);
        }
    }

    /// Each y tick's position and the label text for the value there.
    fn y_tick_labels(&self, extent: ValueExtent, height: f32) -> Vec<(f32, SharedString)> {
        self.tick_positions(height)
            .into_iter()
            .map(|y| {
                let value = extent.value_at(y);
                let text = match self.y_tick_format.as_ref() {
                    Some(format) => format(value),
                    None => format_tick(value),
                };
                (y, text)
            })
            .collect()
    }

    /// The plot area within `bounds`: past the value-axis gutter and above the
    /// x axis at `height`.
    pub(crate) fn plot_bounds(&self, bounds: Bounds<Pixels>, height: f32) -> Bounds<Pixels> {
        let left = self.plot_left();
        Bounds {
            origin: bounds.origin + point(px(left), px(0.)),
            size: Size::new(bounds.size.width - px(left), px(height)),
        }
    }

    /// The y ticks, evenly spaced in pixels from the top edge (0) to the
    /// baseline (`height`), both included.
    fn tick_positions(&self, height: f32) -> Vec<f32> {
        let count = self.y_tick_count.max(2);
        (0..count)
            .map(|i| height * i as f32 / (count - 1) as f32)
            .collect()
    }

    /// Paint the grid: a line at every y tick but the baseline, which the x axis
    /// draws, and `grid_columns` evenly spaced vertical lines from the left edge.
    pub(crate) fn paint_grid(
        &self,
        bounds: Bounds<Pixels>,
        height: f32,
        window: &mut Window,
        cx: &mut App,
    ) {
        let plot = self.plot_bounds(bounds, height);
        let mut rows = self.tick_positions(height);
        rows.pop();
        let width = plot.size.width.as_f32();
        let columns: Vec<f32> = (0..self.grid_columns)
            .map(|i| width * i as f32 / self.grid_columns as f32)
            .collect();
        let grid = Grid::new().y(rows).x(columns).stroke(cx.theme().border);
        let grid = if self.grid_dashed {
            grid.dash_array(&[px(4.), px(2.)])
        } else {
            grid
        };
        grid.paint(&plot, window);
    }

    /// Paint a dashed line across the plot at each reference value, darker than
    /// the grid so it reads apart from a dashed grid line.
    pub(crate) fn paint_reference_lines(
        &self,
        extent: ValueExtent,
        bounds: Bounds<Pixels>,
        height: f32,
        window: &mut Window,
        cx: &mut App,
    ) {
        let rows: Vec<f32> = self
            .reference_lines
            .iter()
            .filter_map(|v| extent.position_of(*v))
            .filter(|y| (0. ..=height).contains(y))
            .collect();
        if rows.is_empty() {
            return;
        }
        Grid::new()
            .y(rows)
            .stroke(cx.theme().muted_foreground)
            .dash_array(&[px(4.), px(2.)])
            .paint(&self.plot_bounds(bounds, height), window);
    }

    /// Paint a tick label at every y tick, reading the value the scale puts there.
    pub(crate) fn paint_y_labels(
        &self,
        extent: ValueExtent,
        bounds: Bounds<Pixels>,
        height: f32,
        window: &mut Window,
        cx: &mut App,
    ) {
        if !self.y_axis {
            return;
        }
        let color = cx.theme().muted_foreground;
        let labels = self
            .y_tick_labels(extent, height)
            .into_iter()
            .map(|(y, text)| {
                match self.y_axis_label_placement {
                    // Beside its grid line, above it but for the top one, which
                    // would leave the plot.
                    AxisLabelPlacement::Inside => {
                        let top = if y < TEXT_HEIGHT {
                            y + TEXT_GAP
                        } else {
                            y - TEXT_HEIGHT
                        };
                        Text::new(text, point(TEXT_GAP, top), color)
                    }
                    AxisLabelPlacement::Outside => {
                        let top = (y - TEXT_SIZE / 2.).clamp(0., (height - TEXT_SIZE).max(0.));
                        Text::new(text, point(self.y_label_gap - TEXT_GAP * 2., top), color)
                            .align(TextAlign::Right)
                    }
                }
            })
            .collect();
        PlotLabel::new(labels).paint(&bounds, window, cx);
    }
}

/// The mask a point chart paints its series under once its y axis is pinned,
/// so a value outside the pinned domain stops at the plot area instead of
/// running over the x-axis labels. It bleeds by half a hover dot, keeping
/// strokes and dots on the plot's edges whole.
pub(crate) fn pinned_plot_mask(bounds: Bounds<Pixels>, height: f32) -> ContentMask<Pixels> {
    let bleed = HOVER_DOT_SIZE / 2.;
    ContentMask {
        bounds: Bounds::from_corners(
            bounds.origin - gpui::point(bleed, bleed),
            gpui::point(bounds.right() + bleed, bounds.top() + px(height) + bleed),
        ),
    }
}

/// Build x-axis labels for point-based scales (`LineChart`, `AreaChart`).
///
/// Point scales place items at evenly spaced positions, on an axis laid out for
/// `point_count` of them. A label on the first point is left-aligned, one on
/// the last is right-aligned, and the rest are centered.
pub(crate) fn build_point_x_labels<T, X>(
    data: &[T],
    x_fn: &dyn Fn(&T) -> X,
    x_scale: &ScalePoint<X>,
    point_count: usize,
    labeled: &[bool],
    color: Hsla,
) -> Vec<AxisText>
where
    X: PartialEq + Into<SharedString>,
{
    data.iter()
        .enumerate()
        .filter_map(|(i, d)| {
            if !labeled.get(i).copied().unwrap_or(false) {
                return None;
            }
            x_scale.tick(&x_fn(d)).map(|x_tick| {
                let align = match i {
                    0 if point_count == 1 => TextAlign::Center,
                    0 => TextAlign::Left,
                    i if i == point_count - 1 => TextAlign::Right,
                    _ => TextAlign::Center,
                };
                // Call x_fn again to get an owned value for the label text.
                AxisText::new(x_fn(d).into(), x_tick, color).align(align)
            })
        })
        .collect()
}

/// Build axis labels for band-based scales (`BarChart`, `CandlestickChart`).
///
/// Band scales place items in evenly sized bands. The returned `tick`
/// coordinate is the centre of each band along the band axis; the caller
/// decides whether to feed the result to `PlotAxis::x_label` (vertical
/// charts) or `PlotAxis::y_label` (horizontal charts).
pub(crate) fn build_band_labels<T, X>(
    data: &[T],
    x_fn: &dyn Fn(&T) -> X,
    x_scale: &ScaleBand<X>,
    band_width: f32,
    labeled: &[bool],
    color: Hsla,
) -> Vec<AxisText>
where
    X: Eq + Hash + Into<SharedString>,
{
    data.iter()
        .enumerate()
        .filter_map(|(i, d)| {
            if !labeled.get(i).copied().unwrap_or(false) {
                return None;
            }
            x_scale.tick(&x_fn(d)).map(|x_tick| {
                // Call x_fn again to get an owned value for the label text.
                AxisText::new(x_fn(d).into(), x_tick + band_width / 2., color)
                    .align(TextAlign::Center)
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{chart::PieChart, plot::Plot};

    fn chart() -> PieChart<f32> {
        PieChart::new([1., 2.])
    }

    /// The whole point of the default: a chart nobody gave an id to is still
    /// interactive, because every caller forgot to ask for it.
    #[test]
    fn a_chart_is_interactive_without_being_given_an_id() {
        assert!(Plot::id(&chart()).is_some());
    }

    /// Two construction sites must not share hover state or a path cache.
    #[test]
    fn charts_built_at_different_sites_get_different_ids() {
        assert_ne!(
            Plot::id(&PieChart::new([1.])),
            Plot::id(&PieChart::new([1.]))
        );
    }

    /// One site reached twice is one id — the caveat `id` exists for.
    #[test]
    fn charts_built_at_one_site_share_an_id() {
        assert_eq!(Plot::id(&chart()), Plot::id(&chart()));
    }

    /// The escape hatch: no id means no hitbox, so nothing above the chart has
    /// to fight it for the cursor.
    #[test]
    fn a_chart_turned_off_has_no_id_to_key_anything_on() {
        assert!(Plot::id(&chart().interactive(false)).is_none());
        assert!(Plot::id(&chart().id("pie").interactive(false)).is_none());
    }

    /// Only a label on the axis's last point hugs the right edge; the last item
    /// of data laid out for more points sits mid-axis and stays centered.
    #[test]
    fn only_the_last_point_right_aligns_its_label() {
        use gpui::{Hsla, TextAlign};

        use super::{build_point_x_labels, point_range};
        use crate::plot::scale::ScalePoint;

        let data = ["a", "b", "c"];
        let align = |point_count| {
            let x = ScalePoint::new(
                data.to_vec(),
                point_range(0., 100., data.len(), point_count),
            );
            build_point_x_labels(
                &data,
                &|d: &&'static str| *d,
                &x,
                point_count,
                &[true; 3],
                Hsla::default(),
            )
            .into_iter()
            .map(|label| label.align)
            .collect::<Vec<_>>()
        };

        assert_eq!(
            align(3),
            [TextAlign::Left, TextAlign::Center, TextAlign::Right]
        );
        assert_eq!(
            align(5),
            [TextAlign::Left, TextAlign::Center, TextAlign::Center]
        );
    }

    #[test]
    fn a_named_id_replaces_the_default() {
        assert_eq!(
            Plot::id(&chart().id("pie")),
            Some(gpui::ElementId::Name("pie".into()))
        );
    }

    /// The default five ticks put the grid where it always was: four lines
    /// splitting the plot, the baseline left to the x axis.
    #[test]
    fn the_default_ticks_keep_the_grid_in_place() {
        let axes = super::PointAxes::default();
        let mut rows = axes.tick_positions(100.);
        rows.pop();
        assert_eq!(rows, vec![0., 25., 50., 75.]);
    }

    #[test]
    fn a_label_count_spreads_labels_from_the_first_item_to_the_last() {
        use super::labeled_items;

        let shown = |len, count| {
            labeled_items(len, Some(count), 1)
                .iter()
                .enumerate()
                .filter_map(|(i, &on)| on.then_some(i))
                .collect::<Vec<_>>()
        };
        assert_eq!(shown(11, 3), vec![0, 5, 10]);
        assert_eq!(shown(10, 2), vec![0, 9]);
        assert_eq!(shown(3, 5), vec![0, 1, 2]);
        assert_eq!(shown(4, 1), vec![0]);
        assert!(shown(4, 0).is_empty());

        // Without a count the stride still decides.
        assert_eq!(labeled_items(4, None, 2), vec![false, true, false, true]);
    }

    /// A tick label reads the value its height stands for, so the top one reads
    /// past the highest value by the padding above it.
    #[test]
    fn a_tick_reads_the_value_at_its_height() {
        use super::point_value_scale;

        let (_, extent) = point_value_scale([10., 20.], None, 110., (10., 0.));
        assert_eq!(extent.value_at(110.), 0.);
        assert_eq!(extent.value_at(10.), 20.);
        assert!((extent.value_at(0.) - 22.).abs() < 1e-4);
        assert_eq!(extent.position_of(20.), Some(10.));

        let (_, extent) = point_value_scale([0.], Some((100., 200.)), 100., (0., 0.));
        assert_eq!(extent.value_at(0.), 200.);
        assert_eq!(extent.position_of(150.), Some(50.));
    }
}
