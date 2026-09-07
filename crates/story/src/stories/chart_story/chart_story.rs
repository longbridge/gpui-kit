use std::rc::Rc;

use gpui_kit::base::ElementExt as _;
use gpui_kit::component::{
    ActiveTheme, StyledExt,
    chart::{
        AreaChart, BarChart, CandlestickChart, LineChart, PieChart, RadarChart, SankeyChart,
        SankeyLabel,
    },
    dock::PanelControl,
    h_flex,
    plot::shape::{BarAlignment, SankeyAlign, SankeyLink, SankeyValueScale},
    scroll::ScrollableElement as _,
    separator::Separator,
    v_flex,
};
use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, Focusable, FontWeight, Hsla,
    IntoElement, ListAlignment, ListState, ParentElement, Pixels, Render, Rgba, SharedString,
    Styled, Window, div, linear_color_stop, linear_gradient, list, prelude::FluentBuilder, px,
};
use serde::Deserialize;

use super::StackedBarChart;
use crate::Story;

/// The height of one chart card, and the list's overdraw: the virtual list
/// keeps one row of cards live on either side of the viewport.
const CARD_HEIGHT: Pixels = px(400.);
/// The gap between cards in a row, and between rows.
const CARD_GAP: Pixels = px(16.);
/// A chart stops being readable below this width, so a narrower viewport drops
/// a column rather than squeezing one more card in.
const MIN_CARD_WIDTH: Pixels = px(280.);
/// The inset between the panel edge and the cards. The list owns the scroll
/// here, so the inset sits inside the list and leaves the scrollbar on the
/// panel edge where the other stories put it.
const CONTENT_INSET: Pixels = px(16.);

/// The number of cards a row fits when the list is `width` wide.
fn columns_for(width: Pixels) -> usize {
    let available = width - CONTENT_INSET * 2.;
    ((available + CARD_GAP) / (MIN_CARD_WIDTH + CARD_GAP))
        .floor()
        .max(1.) as usize
}

#[derive(Clone, Deserialize)]
struct MonthlyDevice {
    pub month: SharedString,
    pub desktop: f64,
    pub color_alpha: f32,
}

impl MonthlyDevice {
    pub fn color(&self, color: Hsla) -> Hsla {
        color.alpha(self.color_alpha)
    }
}

#[derive(Clone, Deserialize)]
pub struct DailyDevice {
    pub date: SharedString,
    pub desktop: f64,
    pub mobile: f64,
    pub tablet: f64,
    pub watch: f64,
}

#[derive(Clone, Deserialize)]
pub struct RadarDevice {
    pub month: SharedString,
    pub desktop: f64,
    pub mobile: f64,
}

#[derive(Clone, Deserialize)]
pub struct StockPrice {
    pub date: SharedString,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

/// TSLA income statement data, values and colors as strings like the real API.
#[derive(Clone, Deserialize)]
struct TslaStatementNode {
    key: SharedString,
    name: SharedString,
    value: SharedString,
    growth: SharedString,
    color: SharedString,
}

#[derive(Clone, Deserialize)]
struct TslaStatementLink {
    source: SharedString,
    target: SharedString,
    value: SharedString,
}

#[derive(Clone, Deserialize)]
struct TslaStatement {
    period: SharedString,
    nodes: Vec<TslaStatementNode>,
    links: Vec<TslaStatementLink>,
}

#[derive(Clone, Deserialize)]
struct TslaIncomeStatement {
    list: Vec<TslaStatement>,
}

#[derive(Clone)]
pub struct TslaNode {
    pub name: SharedString,
    /// The real dollar value, for the label; the layout gets sqrt-compressed
    /// link values to keep small flows readable.
    pub value: f64,
    /// Year-over-year growth in percent, for the label.
    pub growth: Option<f64>,
    pub color: Hsla,
}

/// The fixture data behind every card.
///
/// The virtual list builds a card only when it scrolls into view, so the row
/// renderer holds one shared handle to this instead of a copy of each series.
struct ChartData {
    daily_devices: Vec<DailyDevice>,
    monthly_devices: Vec<MonthlyDevice>,
    /// The monthly figures recentred on their mean, so the bar charts have a
    /// mix of positive and negative values to draw around the zero line.
    monthly_variations: Vec<MonthlyDevice>,
    radar_devices: Vec<RadarDevice>,
    stock_prices: Vec<StockPrice>,
    tsla_statements: Vec<(SharedString, Vec<TslaNode>, Vec<SankeyLink>)>,
}

/// One chart card in the gallery.
#[derive(Clone, Copy, PartialEq)]
enum ChartCard {
    AreaStacked,
    Pie,
    PieDonut,
    PiePadAngle,
    PieLabel,
    Radar,
    RadarMultiple,
    RadarDots,
    RadarLinesOnly,
    Bar,
    BarMixed,
    BarStacked,
    BarRounded,
    BarBottomAligned,
    BarTopAligned,
    BarLeftAligned,
    BarRightAligned,
    BarNegative,
    BarGradientBottom,
    BarGradientTop,
    BarGradientLeft,
    BarGradientRight,
    BarGradientPerBar,
    BarGradientDiagonal,
    Line,
    LineLinear,
    LineStepAfter,
    LineDots,
    Area,
    AreaLinear,
    AreaStepAfter,
    AreaGradient,
    Candlestick,
    CandlestickNarrow,
    CandlestickWide,
    CandlestickTickMargin,
    /// The income statement at this index of [`ChartData::tsla_statements`].
    Sankey(usize),
}

impl ChartCard {
    /// Whether the heading and footnotes sit over the middle of the card, as
    /// the round charts want.
    fn is_centered(self) -> bool {
        matches!(
            self,
            Self::Pie
                | Self::PieDonut
                | Self::PiePadAngle
                | Self::PieLabel
                | Self::Radar
                | Self::RadarMultiple
                | Self::RadarDots
                | Self::RadarLinesOnly
        )
    }

    fn render(self, data: &ChartData, cx: &App) -> AnyElement {
        let color = cx.theme().chart_3;
        let (title, chart): (SharedString, AnyElement) = match self {
            Self::AreaStacked => (
                "Area Chart - Stacked".into(),
                AreaChart::new(data.daily_devices.clone())
                    .x(|d| d.date.clone())
                    .y(|d| d.desktop)
                    .stroke(cx.theme().chart_1)
                    .fill(linear_gradient(
                        0.,
                        linear_color_stop(cx.theme().chart_1.opacity(0.4), 1.),
                        linear_color_stop(cx.theme().background.opacity(0.3), 0.),
                    ))
                    .name("Desktop")
                    .y(|d| d.mobile)
                    .stroke(cx.theme().chart_2)
                    .fill(linear_gradient(
                        0.,
                        linear_color_stop(cx.theme().chart_2.opacity(0.4), 1.),
                        linear_color_stop(cx.theme().background.opacity(0.3), 0.),
                    ))
                    .name("Mobile")
                    .tick_margin(8)
                    .id("area-chart-tooltip")
                    .into_any_element(),
            ),
            Self::Pie => (
                "Pie Chart".into(),
                PieChart::new(data.monthly_devices.clone())
                    .value(|d| d.desktop as f32)
                    .outer_radius(100.)
                    .color(move |d| d.color(color))
                    .into_any_element(),
            ),
            Self::PieDonut => (
                "Pie Chart - Donut".into(),
                PieChart::new(data.monthly_devices.clone())
                    .value(|d| d.desktop as f32)
                    .inner_radius(60.)
                    .outer_radius_fn(|d| 100. - d.index as f32 * 4.)
                    .color(move |d| d.color(color))
                    .into_any_element(),
            ),
            Self::PiePadAngle => (
                "Pie Chart - Pad Angle".into(),
                PieChart::new(data.monthly_devices.clone())
                    .value(|d| d.desktop as f32)
                    .inner_radius(60.)
                    .outer_radius(100.)
                    .pad_angle(4. / 100.)
                    .color(move |d| d.color(color))
                    .into_any_element(),
            ),
            Self::PieLabel => (
                "Pie Chart - Label".into(),
                PieChart::new(data.monthly_devices.clone())
                    .value(|d| d.desktop as f32)
                    .inner_radius(50.)
                    .outer_radius(80.)
                    .color(move |d| d.color(color))
                    .label(|d| d.month.clone())
                    .into_any_element(),
            ),
            Self::Radar => (
                "Radar Chart".into(),
                RadarChart::new(data.radar_devices.clone())
                    .label(|d| d.month.clone())
                    .value(|d| d.desktop)
                    .name("Desktop")
                    .id("radar-chart")
                    .into_any_element(),
            ),
            Self::RadarMultiple => (
                "Radar Chart - Multiple".into(),
                RadarChart::new(data.radar_devices.clone())
                    .label(|d| d.month.clone())
                    .value(|d| d.desktop)
                    .name("Desktop")
                    .value(|d| d.mobile)
                    .name("Mobile")
                    .id("radar-chart-multiple")
                    .into_any_element(),
            ),
            Self::RadarDots => (
                "Radar Chart - Dots".into(),
                RadarChart::new(data.radar_devices.clone())
                    // An element label: the dimension name over a grade badge.
                    .label({
                        let muted_foreground = cx.theme().muted_foreground;
                        let accent = cx.theme().chart_2;
                        let badge_radius = cx.theme().radius_full();

                        move |d: &RadarDevice| {
                            let grade = match d.desktop {
                                v if v >= 250. => "A",
                                v if v >= 200. => "B",
                                _ => "C",
                            };

                            v_flex()
                                .items_center()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(muted_foreground)
                                        .child(d.month.clone()),
                                )
                                .child(
                                    h_flex()
                                        .justify_center()
                                        .size_6()
                                        .rounded(badge_radius)
                                        .bg(accent.opacity(0.1))
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(accent)
                                        .child(grade),
                                )
                                .into_any_element()
                        }
                    })
                    .value(|d| d.desktop)
                    .name("Desktop")
                    .stroke(cx.theme().chart_2)
                    .dot()
                    // A badge label is far taller than a line of text, so
                    // pull the ring in to leave it room.
                    .outer_radius(64.)
                    .id("radar-chart-dots")
                    .into_any_element(),
            ),
            Self::RadarLinesOnly => (
                "Radar Chart - Lines Only".into(),
                RadarChart::new(data.radar_devices.clone())
                    .label(|d| d.month.clone())
                    .value(|d| d.desktop)
                    .name("Desktop")
                    .stroke(cx.theme().chart_3)
                    .fill(gpui_kit::transparent_black())
                    .max_value(400.)
                    .grid_levels(5)
                    .id("radar-chart-lines-only")
                    .into_any_element(),
            ),
            Self::Bar => (
                "Bar Chart".into(),
                BarChart::new(data.monthly_devices.clone())
                    .band(|d| d.month.clone())
                    .value(|d| d.desktop)
                    .name("Desktop")
                    .id("bar-chart-tooltip")
                    .into_any_element(),
            ),
            Self::BarMixed => (
                "Bar Chart - Mixed".into(),
                BarChart::new(data.monthly_devices.clone())
                    .id("bar-chart-mixed")
                    .name("Desktop")
                    .band(|d| d.month.clone())
                    .value(|d| d.desktop)
                    .fill(move |d, _, _, _| d.color(color))
                    .into_any_element(),
            ),
            Self::BarStacked => (
                "Bar Chart - Stacked".into(),
                StackedBarChart::new(data.daily_devices.iter().take(8).cloned().collect())
                    .into_any_element(),
            ),
            Self::BarRounded => (
                "Bar Chart - Rounded corners".into(),
                BarChart::new(data.monthly_devices.clone())
                    .id("bar-chart-rounded")
                    .name("Desktop")
                    .band(|d| d.month.clone())
                    .value(|d| d.desktop)
                    .label(|d| d.desktop.to_string())
                    .corner_radii(px(8.))
                    .into_any_element(),
            ),
            Self::BarBottomAligned => (
                "Bar Chart - Bottom aligned".into(),
                BarChart::new(data.monthly_devices.clone())
                    .id("bar-chart-bottom")
                    .name("Desktop")
                    .band(|d| d.month.clone())
                    .value(|d| d.desktop)
                    .label(|d| d.desktop.to_string())
                    .into_any_element(),
            ),
            Self::BarTopAligned => (
                "Bar Chart - Top aligned".into(),
                BarChart::new(data.monthly_devices.clone())
                    .id("bar-chart-top")
                    .name("Desktop")
                    .band(|d| d.month.clone())
                    .value(|d| d.desktop)
                    .label(|d| d.desktop.to_string())
                    .alignment(BarAlignment::Top)
                    .into_any_element(),
            ),
            Self::BarLeftAligned => (
                "Bar Chart - Left aligned".into(),
                BarChart::new(data.monthly_devices.clone())
                    .id("bar-chart-left")
                    .name("Desktop")
                    .band(|d| d.month.clone())
                    .value(|d| d.desktop)
                    .label(|d| d.desktop.to_string())
                    .alignment(BarAlignment::Left)
                    .into_any_element(),
            ),
            Self::BarRightAligned => (
                "Bar Chart - Right aligned".into(),
                BarChart::new(data.monthly_devices.clone())
                    .id("bar-chart-right")
                    .name("Desktop")
                    .band(|d| d.month.clone())
                    .value(|d| d.desktop)
                    .label(|d| d.desktop.to_string())
                    .alignment(BarAlignment::Right)
                    .into_any_element(),
            ),
            Self::BarNegative => (
                "Bar Chart - Negative values".into(),
                BarChart::new(data.monthly_variations.clone())
                    .id("bar-chart-negative")
                    .name("Variation")
                    .band(|d| d.month.clone())
                    .value(|d| d.desktop)
                    .label(|d| format!("{:.0}", d.desktop))
                    .value_axis(true)
                    .into_any_element(),
            ),
            Self::BarGradientBottom => {
                let c = cx.theme().chart_1;
                (
                    "Bar Chart - Gradient (Bottom)".into(),
                    BarChart::new(data.monthly_devices.clone())
                        .id("bar-chart-gradient-bottom")
                        .name("Desktop")
                        .band(|d| d.month.clone())
                        .value(|d| d.desktop)
                        .label(|d| d.desktop.to_string())
                        .fill_gradient(move |_, chart_range, chart_to_bar| {
                            [
                                linear_color_stop(
                                    c.opacity(0.3),
                                    chart_to_bar(*chart_range.start()),
                                ),
                                linear_color_stop(c, chart_to_bar(*chart_range.end())),
                            ]
                        })
                        .into_any_element(),
                )
            }
            Self::BarGradientTop => {
                let c = cx.theme().chart_1;
                (
                    "Bar Chart - Gradient (Top)".into(),
                    BarChart::new(data.monthly_devices.clone())
                        .id("bar-chart-gradient-top")
                        .name("Desktop")
                        .band(|d| d.month.clone())
                        .value(|d| d.desktop)
                        .label(|d| d.desktop.to_string())
                        .alignment(BarAlignment::Top)
                        .fill_gradient(move |_, chart_range, chart_to_bar| {
                            [
                                linear_color_stop(
                                    c.opacity(0.3),
                                    chart_to_bar(*chart_range.start()),
                                ),
                                linear_color_stop(c, chart_to_bar(*chart_range.end())),
                            ]
                        })
                        .into_any_element(),
                )
            }
            Self::BarGradientLeft => {
                let c = cx.theme().chart_1;
                (
                    "Bar Chart - Gradient (Left)".into(),
                    BarChart::new(data.monthly_devices.clone())
                        .id("bar-chart-gradient-left")
                        .name("Desktop")
                        .band(|d| d.month.clone())
                        .value(|d| d.desktop)
                        .label(|d| d.desktop.to_string())
                        .alignment(BarAlignment::Left)
                        .fill_gradient(move |_, chart_range, chart_to_bar| {
                            [
                                linear_color_stop(
                                    c.opacity(0.3),
                                    chart_to_bar(*chart_range.start()),
                                ),
                                linear_color_stop(c, chart_to_bar(*chart_range.end())),
                            ]
                        })
                        .into_any_element(),
                )
            }
            Self::BarGradientRight => {
                let c = cx.theme().chart_1;
                (
                    "Bar Chart - Gradient (Right)".into(),
                    BarChart::new(data.monthly_devices.clone())
                        .id("bar-chart-gradient-right")
                        .name("Desktop")
                        .band(|d| d.month.clone())
                        .value(|d| d.desktop)
                        .label(|d| d.desktop.to_string())
                        .alignment(BarAlignment::Right)
                        .fill_gradient(move |_, chart_range, chart_to_bar| {
                            [
                                linear_color_stop(
                                    c.opacity(0.3),
                                    chart_to_bar(*chart_range.start()),
                                ),
                                linear_color_stop(c, chart_to_bar(*chart_range.end())),
                            ]
                        })
                        .into_any_element(),
                )
            }
            Self::BarGradientPerBar => {
                let c = cx.theme().chart_1;
                (
                    "Bar Chart - Gradient (Per-bar)".into(),
                    BarChart::new(data.monthly_devices.clone())
                        .id("bar-chart-gradient-per-bar")
                        .name("Desktop")
                        .band(|d| d.month.clone())
                        .value(|d| d.desktop)
                        .label(|d| d.desktop.to_string())
                        .fill_gradient(move |_, _, _| {
                            [
                                linear_color_stop(c.opacity(0.3), 0.),
                                linear_color_stop(c, 1.),
                            ]
                        })
                        .into_any_element(),
                )
            }
            Self::BarGradientDiagonal => {
                let c1 = cx.theme().chart_1;
                let c2 = cx.theme().chart_5;
                (
                    "Bar Chart - Gradient (Diagonal, across bars)".into(),
                    BarChart::new(data.monthly_devices.clone())
                        .id("bar-chart-gradient-diagonal")
                        .name("Desktop")
                        .band(|d| d.month.clone())
                        .value(|d| d.desktop)
                        .label(|d| d.desktop.to_string())
                        .fill(move |_, bar, chart, _| {
                            // Project the bar's corners onto the chart's
                            // bottom-left → top-right diagonal so each bar
                            // shows the slice of a chart-wide diagonal
                            // gradient corresponding to its own footprint.
                            let w = chart.size.width.max(f32::EPSILON);
                            let h = chart.size.height.max(f32::EPSILON);
                            let denom = w * w + h * h;
                            let project = |x: f32, y: f32| -> f32 { (x * w + (h - y) * h) / denom };
                            let lo = project(bar.origin.x, bar.origin.y + bar.size.height);
                            let hi = project(bar.origin.x + bar.size.width, bar.origin.y);
                            let lerp = |t: f32| Hsla {
                                h: c1.h + (c2.h - c1.h) * t,
                                s: c1.s + (c2.s - c1.s) * t,
                                l: c1.l + (c2.l - c1.l) * t,
                                a: c1.a + (c2.a - c1.a) * t,
                            };
                            linear_gradient(
                                45.,
                                linear_color_stop(lerp(lo), 0.),
                                linear_color_stop(lerp(hi), 1.),
                            )
                        })
                        .into_any_element(),
                )
            }
            Self::Line => (
                "Line Chart - Tooltip".into(),
                LineChart::new(data.monthly_devices.clone())
                    .x(|d| d.month.clone())
                    .y(|d| d.desktop)
                    .name("Desktop")
                    .id("line-chart-tooltip")
                    .into_any_element(),
            ),
            Self::LineLinear => (
                "Line Chart - Linear".into(),
                LineChart::new(data.monthly_devices.clone())
                    .x(|d| d.month.clone())
                    .y(|d| d.desktop)
                    .linear()
                    .id("line-chart-linear")
                    .into_any_element(),
            ),
            Self::LineStepAfter => (
                "Line Chart - Step After".into(),
                LineChart::new(data.monthly_devices.clone())
                    .x(|d| d.month.clone())
                    .y(|d| d.desktop)
                    .step_after()
                    .id("line-chart-step-after")
                    .into_any_element(),
            ),
            Self::LineDots => (
                "Line Chart - Dots".into(),
                LineChart::new(data.monthly_devices.clone())
                    .x(|d| d.month.clone())
                    .y(|d| d.desktop)
                    .dot()
                    .stroke(cx.theme().chart_5)
                    .id("line-chart-dots")
                    .into_any_element(),
            ),
            Self::Area => (
                "Area Chart".into(),
                AreaChart::new(data.monthly_devices.clone())
                    .x(|d| d.month.clone())
                    .y(|d| d.desktop)
                    .id("area-chart")
                    .into_any_element(),
            ),
            Self::AreaLinear => (
                "Area Chart - Linear".into(),
                AreaChart::new(data.monthly_devices.clone())
                    .x(|d| d.month.clone())
                    .y(|d| d.desktop)
                    .linear()
                    .id("area-chart-linear")
                    .into_any_element(),
            ),
            Self::AreaStepAfter => (
                "Area Chart - Step After".into(),
                AreaChart::new(data.monthly_devices.clone())
                    .x(|d| d.month.clone())
                    .y(|d| d.desktop)
                    .step_after()
                    .id("area-chart-step-after")
                    .into_any_element(),
            ),
            Self::AreaGradient => (
                "Area Chart - Linear Gradient".into(),
                AreaChart::new(data.monthly_devices.clone())
                    .x(|d| d.month.clone())
                    .y(|d| d.desktop)
                    .fill(linear_gradient(
                        0.,
                        linear_color_stop(cx.theme().chart_1.opacity(0.4), 1.),
                        linear_color_stop(cx.theme().background.opacity(0.3), 0.),
                    ))
                    .id("area-chart-gradient")
                    .into_any_element(),
            ),
            Self::Candlestick => (
                "Candlestick Chart".into(),
                CandlestickChart::new(data.stock_prices.clone())
                    .x(|d| d.date.clone())
                    .open(|d| d.open)
                    .high(|d| d.high)
                    .low(|d| d.low)
                    .close(|d| d.close)
                    .into_any_element(),
            ),
            Self::CandlestickNarrow => (
                "Candlestick Chart - Narrow".into(),
                CandlestickChart::new(data.stock_prices.clone())
                    .x(|d| d.date.clone())
                    .open(|d| d.open)
                    .high(|d| d.high)
                    .low(|d| d.low)
                    .close(|d| d.close)
                    .body_width_ratio(0.5)
                    .into_any_element(),
            ),
            Self::CandlestickWide => (
                "Candlestick Chart - Wide".into(),
                CandlestickChart::new(data.stock_prices.clone())
                    .x(|d| d.date.clone())
                    .open(|d| d.open)
                    .high(|d| d.high)
                    .low(|d| d.low)
                    .close(|d| d.close)
                    .body_width_ratio(1.0)
                    .into_any_element(),
            ),
            Self::CandlestickTickMargin => (
                "Candlestick Chart - Tick Margin".into(),
                CandlestickChart::new(data.stock_prices.clone())
                    .x(|d| d.date.clone())
                    .open(|d| d.open)
                    .high(|d| d.high)
                    .low(|d| d.low)
                    .close(|d| d.close)
                    .tick_margin(2)
                    .into_any_element(),
            ),
            Self::Sankey(index) => {
                let Some((period, nodes, links)) = data.tsla_statements.get(index) else {
                    return div().into_any_element();
                };

                // Sqrt value scale keeps the huge revenue flow from
                // dwarfing the small profit/expense ones.
                let chart = SankeyChart::new(nodes.clone(), links.clone())
                    .node_align(SankeyAlign::Center)
                    .node_padding(40.)
                    .value_scale(SankeyValueScale::Sqrt)
                    .node_color(|d: &TslaNode| d.color);
                // The first chart shows fully custom three-line labels with
                // the year-over-year change; the other keeps the default
                // value/name lines.
                let chart = if index == 0 {
                    let up = cx.theme().success;
                    let down = cx.theme().danger;
                    let muted = cx.theme().muted_foreground;
                    chart.labels(move |d: &TslaNode, _| {
                        let mut lines = vec![SankeyLabel::new(format!(
                            "${:.2}B",
                            d.value / 1_000_000_000.
                        ))];
                        if let Some(growth) = d.growth {
                            let arrow = if growth >= 0. { "▲" } else { "▼" };
                            lines.push(
                                SankeyLabel::new(format!("{} {:+.2}%", arrow, growth))
                                    .color(if growth >= 0. { up } else { down }),
                            );
                        }
                        lines.push(SankeyLabel::new(d.name.clone()).color(muted));
                        lines
                    })
                } else {
                    chart
                        .node_label(|d| d.name.clone())
                        .value_label(|d, _| format!("${:.2}B", d.value / 1_000_000_000.).into())
                };

                (
                    format!("Sankey Chart - TSLA {}", period).into(),
                    chart.into_any_element(),
                )
            }
        };

        chart_container(title, chart, self.is_centered(), cx).into_any_element()
    }
}

/// A group of related cards, drawn as consecutive rows.
struct ChartSection {
    /// Whether a rule separates this group from the one above it.
    rule_above: bool,
    cards: Vec<ChartCard>,
}

impl ChartSection {
    fn new(cards: impl IntoIterator<Item = ChartCard>) -> Self {
        Self {
            rule_above: false,
            cards: cards.into_iter().collect(),
        }
    }

    /// The same, with a rule above the group.
    fn after_rule(cards: impl IntoIterator<Item = ChartCard>) -> Self {
        Self {
            rule_above: true,
            ..Self::new(cards)
        }
    }
}

/// One row of the virtual list.
enum ChartRow {
    /// The rule between two groups.
    Rule,
    /// A row of cards, sharing the width equally.
    Cards(Vec<ChartCard>),
}

/// Flattens `sections` into rows of at most `columns` cards.
fn rows_of(sections: &[ChartSection], columns: usize) -> Vec<ChartRow> {
    sections
        .iter()
        .flat_map(|section| {
            section
                .rule_above
                .then_some(ChartRow::Rule)
                .into_iter()
                .chain(
                    section
                        .cards
                        .chunks(columns)
                        .map(|cards| ChartRow::Cards(cards.to_vec())),
                )
        })
        .collect()
}

pub struct ChartStory {
    focus_handle: FocusHandle,
    data: Rc<ChartData>,
    sections: Vec<ChartSection>,
    /// How many cards a row currently holds, from the width measured during
    /// the last prepaint.
    columns: usize,
    list_state: ListState,
}

impl ChartStory {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let daily_devices = serde_json::from_str::<Vec<DailyDevice>>(include_str!(
            "../../fixtures/daily-devices.json"
        ))
        .unwrap();
        let monthly_devices = serde_json::from_str::<Vec<MonthlyDevice>>(include_str!(
            "../../fixtures/monthly-devices.json"
        ))
        .unwrap();
        let radar_devices = serde_json::from_str::<Vec<RadarDevice>>(include_str!(
            "../../fixtures/radar-devices.json"
        ))
        .unwrap();
        let stock_prices = serde_json::from_str::<Vec<StockPrice>>(include_str!(
            "../../fixtures/stock-prices.json"
        ))
        .unwrap();
        let tsla = serde_json::from_str::<TslaIncomeStatement>(include_str!(
            "../../fixtures/tsla-income-statement.json"
        ))
        .unwrap();
        let tsla_statements = tsla
            .list
            .iter()
            .map(|statement| {
                // Map the fixture's string keys to node indices for `SankeyLink`.
                let node_indexes: std::collections::HashMap<SharedString, usize> = statement
                    .nodes
                    .iter()
                    .enumerate()
                    .map(|(index, node)| (node.key.clone(), index))
                    .collect();
                let nodes = statement
                    .nodes
                    .iter()
                    .map(|node| TslaNode {
                        name: node.name.clone(),
                        value: node.value.parse().unwrap_or(0.),
                        growth: node.growth.parse().ok(),
                        color: Rgba::try_from(node.color.as_ref())
                            .map(Into::into)
                            .unwrap_or(gpui_kit::black()),
                    })
                    .collect();
                // Skip links with unknown node keys or unparsable values
                // instead of panicking on bad fixture data.
                let links = statement
                    .links
                    .iter()
                    .filter_map(|link| {
                        Some(SankeyLink::new(
                            *node_indexes.get(&link.source)?,
                            *node_indexes.get(&link.target)?,
                            link.value.parse().ok()?,
                        ))
                    })
                    .collect();
                (statement.period.clone(), nodes, links)
            })
            .collect::<Vec<_>>();

        let mean = monthly_devices.iter().map(|d| d.desktop).sum::<f64>()
            / monthly_devices.len().max(1) as f64;
        let monthly_variations = monthly_devices
            .iter()
            .map(|d| MonthlyDevice {
                month: d.month.clone(),
                desktop: (d.desktop - mean).round(),
                color_alpha: d.color_alpha,
            })
            .collect();

        let sections = sections(tsla_statements.len());
        // The story is docked inside a narrower panel than the window, so this
        // is only a first guess; the prepaint below corrects it.
        let columns = columns_for(window.viewport_size().width);
        let list_state = ListState::new(
            rows_of(&sections, columns).len(),
            ListAlignment::Top,
            CARD_HEIGHT,
        );

        Self {
            focus_handle: cx.focus_handle(),
            data: Rc::new(ChartData {
                daily_devices,
                monthly_devices,
                monthly_variations,
                radar_devices,
                stock_prices,
                tsla_statements,
            }),
            sections,
            columns,
            list_state,
        }
    }

    /// Records the width the gallery was laid out at, re-chunking the rows
    /// when it changes how many cards fit side by side.
    fn measure(&mut self, width: Pixels, cx: &mut Context<Self>) {
        let columns = columns_for(width);
        if columns == self.columns {
            return;
        }

        self.columns = columns;
        self.list_state
            .reset(rows_of(&self.sections, columns).len());
        cx.notify();
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

/// The gallery's groups, in the order they appear.
fn sections(sankey_count: usize) -> Vec<ChartSection> {
    use ChartCard::*;

    vec![
        ChartSection::new([AreaStacked]),
        ChartSection::new([Pie, PieDonut, PiePadAngle, PieLabel]),
        ChartSection::after_rule([Radar, RadarMultiple, RadarDots, RadarLinesOnly]),
        ChartSection::after_rule([
            Bar,
            BarMixed,
            BarStacked,
            BarRounded,
            BarBottomAligned,
            BarTopAligned,
            BarLeftAligned,
            BarRightAligned,
            BarNegative,
            BarGradientBottom,
            BarGradientTop,
            BarGradientLeft,
            BarGradientRight,
            BarGradientPerBar,
            BarGradientDiagonal,
        ]),
        ChartSection::after_rule([Line, LineLinear, LineStepAfter, LineDots]),
        ChartSection::after_rule([Area, AreaLinear, AreaStepAfter, AreaGradient]),
        ChartSection::after_rule([
            Candlestick,
            CandlestickNarrow,
            CandlestickWide,
            CandlestickTickMargin,
        ]),
        ChartSection::after_rule((0..sankey_count).map(Sankey)),
    ]
}

impl Story for ChartStory {
    fn title() -> &'static str {
        "Chart"
    }

    fn description() -> &'static str {
        "Beautiful Charts & Graphs."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    /// The virtual list scrolls the gallery itself, so it carries the inset
    /// and the container leaves the panel edge to the scrollbar.
    fn paddings() -> Pixels {
        px(0.)
    }
}

impl Focusable for ChartStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn chart_container(
    title: SharedString,
    chart: AnyElement,
    center: bool,
    cx: &App,
) -> impl IntoElement {
    v_flex()
        .flex_1()
        .min_w_0()
        .h(CARD_HEIGHT)
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius_lg)
        .p_4()
        .child(
            div()
                .when(center, |this| this.text_center())
                .font_semibold()
                .child(title),
        )
        .child(
            div()
                .when(center, |this| this.text_center())
                .text_color(cx.theme().muted_foreground)
                .text_sm()
                .child("January-June 2025"),
        )
        .child(div().flex_1().py_4().child(chart))
        .child(
            div()
                .when(center, |this| this.text_center())
                .font_semibold()
                .text_sm()
                .child("Trending up by 5.2% this month"),
        )
        .child(
            div()
                .when(center, |this| this.text_center())
                .text_color(cx.theme().muted_foreground)
                .text_sm()
                .child("Showing total visitors for the last 6 months"),
        )
}

impl Render for ChartStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = Rc::new(rows_of(&self.sections, self.columns));
        if self.list_state.item_count() != rows.len() {
            self.list_state.reset(rows.len());
        }

        let data = self.data.clone();
        let story = cx.entity();
        div()
            .size_full()
            .bg(cx.theme().background)
            .on_prepaint(move |bounds, _, cx| {
                story.update(cx, |this, cx| this.measure(bounds.size.width, cx));
            })
            .child(
                list(self.list_state.clone(), move |index, _, cx| {
                    let Some(row) = rows.get(index) else {
                        return div().into_any_element();
                    };

                    div()
                        .w_full()
                        .px(CONTENT_INSET)
                        // Spacing between rows only, like a CSS gap.
                        .when(index + 1 < rows.len(), |this| this.pb(CARD_GAP))
                        .child(match row {
                            ChartRow::Rule => Separator::horizontal().into_any_element(),
                            ChartRow::Cards(cards) => h_flex()
                                .w_full()
                                .gap(CARD_GAP)
                                .children(cards.iter().map(|card| card.render(&data, cx)))
                                .into_any_element(),
                        })
                        .into_any_element()
                })
                .size_full()
                // The list's own style honours vertical padding only, so the
                // horizontal inset rides on each row above.
                .py(CONTENT_INSET),
            )
            .vertical_scrollbar(&self.list_state)
    }
}
