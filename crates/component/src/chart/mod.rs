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

use std::{hash::Hash, panic::Location};

use gpui::{App, ElementId, Hsla, Pixels, SharedString, TextAlign, px};
use gpui_base::Spring;

use crate::{
    ActiveTheme,
    plot::{
        AxisText,
        scale::{Scale, ScaleBand, ScalePoint},
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

/// The spring a chart's pointer — the crosshair, highlight band or hover dot —
/// follows the hovered datum with.
///
/// A pointer chases the cursor across neighbouring data, so it has to arrive
/// well within the time the cursor takes to reach the next datum: ECharts moves
/// its axis pointer over 200 ms on an exponential ease-out, which is most of
/// the way there in the first third. The fast tier as a critically damped
/// response lands in the same place, and the tolerance is sub-pixel so the
/// spring rests once nothing visible moves.
pub(crate) fn pointer_spring(cx: &App) -> Spring {
    Spring::new(cx.theme().motion_tokens().duration_fast).with_epsilon(0.1)
}

/// The size of the dot marking the hovered data point.
pub(crate) const HOVER_DOT_SIZE: Pixels = px(8.);

/// The ring behind the hovered dot at full focus.
const HOVER_HALO_SIZE: f32 = 20.;

/// The ring behind a hovered dot, growing out of the dot as the hover fades in.
pub(crate) fn hover_halo_size(focus: f32) -> Pixels {
    px(HOVER_HALO_SIZE * focus)
}

/// Build x-axis labels for point-based scales (`LineChart`, `AreaChart`).
///
/// Point scales place items at evenly spaced positions. The first label is
/// left-aligned, the last is right-aligned, and the rest are centered.
pub(crate) fn build_point_x_labels<T, X>(
    data: &[T],
    x_fn: &dyn Fn(&T) -> X,
    x_scale: &ScalePoint<X>,
    tick_margin: usize,
    color: Hsla,
) -> Vec<AxisText>
where
    X: PartialEq + Into<SharedString>,
{
    let data_len = data.len();
    data.iter()
        .enumerate()
        .filter_map(|(i, d)| {
            if (i + 1) % tick_margin != 0 {
                return None;
            }
            x_scale.tick(&x_fn(d)).map(|x_tick| {
                let align = match i {
                    0 if data_len == 1 => TextAlign::Center,
                    0 => TextAlign::Left,
                    i if i == data_len - 1 => TextAlign::Right,
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
    tick_margin: usize,
    color: Hsla,
) -> Vec<AxisText>
where
    X: Eq + Hash + Into<SharedString>,
{
    data.iter()
        .enumerate()
        .filter_map(|(i, d)| {
            if (i + 1) % tick_margin != 0 {
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

    #[test]
    fn a_named_id_replaces_the_default() {
        assert_eq!(
            Plot::id(&chart().id("pie")),
            Some(gpui::ElementId::Name("pie".into()))
        );
    }
}
