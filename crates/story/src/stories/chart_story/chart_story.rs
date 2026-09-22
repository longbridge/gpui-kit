use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::ElementExt as _;
use gpui_kit::component::{
    ActiveTheme, Icon, StyledExt,
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
    AnyElement, App, AppContext, Background, Context, Corners, Entity, FocusHandle, Focusable,
    FontWeight, Hsla, IntoElement, ListAlignment, ListState, ParentElement, Pixels, Render, Rgba,
    SharedString, Styled, Window, div, linear_color_stop, linear_gradient, list,
    prelude::FluentBuilder, px,
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

/// One month of a SaaS business, 2025.
#[derive(Clone, Deserialize)]
pub struct MonthlyMetric {
    pub month: SharedString,
    pub revenue: f64,
    pub last_year: f64,
    pub expenses: f64,
    pub mrr: f64,
    pub signups: f64,
    pub orders: f64,
    pub refunds: f64,
    pub conversion: f64,
    pub subscriptions: f64,
    pub active_users: f64,
    pub sessions: f64,
    pub storage_tb: f64,
    pub deploys: f64,
    pub downloads: f64,
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
struct TrafficSource {
    source: SharedString,
    visitors: f64,
}

#[derive(Clone, Deserialize)]
struct BrowserShare {
    browser: SharedString,
    share: f64,
}

#[derive(Clone, Deserialize)]
struct PlanMix {
    plan: SharedString,
    accounts: f64,
}

#[derive(Clone, Deserialize)]
struct RegionRevenue {
    region: SharedString,
    revenue: f64,
}

#[derive(Clone, Deserialize)]
struct ProductSales {
    product: SharedString,
    sales: f64,
}

#[derive(Clone, Deserialize)]
struct PageViews {
    page: SharedString,
    views: f64,
}

/// One dimension two products are scored on, out of 100.
#[derive(Clone, Deserialize)]
pub struct ProductScore {
    pub dimension: SharedString,
    pub alpha: f64,
    pub beta: f64,
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
    metrics: Vec<MonthlyMetric>,
    /// Revenue less expenses per month in `revenue`, which crosses zero.
    cash_flow: Vec<MonthlyMetric>,
    traffic_sources: Vec<TrafficSource>,
    browsers: Vec<BrowserShare>,
    plans: Vec<PlanMix>,
    regions: Vec<RegionRevenue>,
    products: Vec<ProductSales>,
    pages: Vec<PageViews>,
    product_scores: Vec<ProductScore>,
    stock_prices: Vec<StockPrice>,
    tsla_statements: Vec<(SharedString, Vec<TslaNode>, Vec<SankeyLink>)>,
}

/// `1234` as `1.2K`, `1234567` as `1.2M`; smaller numbers keep their digits.
fn compact(value: f64) -> String {
    let magnitude = value.abs();
    if magnitude >= 1_000_000. {
        format!("{:.1}M", value / 1_000_000.)
    } else if magnitude >= 10_000. {
        format!("{:.0}K", value / 1_000.)
    } else if magnitude >= 1_000. {
        format!("{:.1}K", value / 1_000.)
    } else {
        format!("{:.0}", value)
    }
}

/// `compact` with a dollar sign, keeping the sign in front of it.
fn money(value: f64) -> String {
    if value < 0. {
        format!("-${}", compact(-value))
    } else {
        format!("${}", compact(value))
    }
}

/// The percentage change from `previous` to `latest`.
fn change_percent(latest: f64, previous: f64) -> f64 {
    if previous.abs() < f64::EPSILON {
        0.
    } else {
        (latest - previous) / previous.abs() * 100.
    }
}

/// The change from the second-to-last to the last of `values`, as a percentage.
fn latest_change(values: impl IntoIterator<Item = f64>) -> f64 {
    let values: Vec<f64> = values.into_iter().collect();
    match values[..] {
        [.., previous, latest] => change_percent(latest, previous),
        _ => 0.,
    }
}

/// The change between the last `window` values and the `window` before them.
fn windowed_change(values: impl IntoIterator<Item = f64>, window: usize) -> f64 {
    let values: Vec<f64> = values.into_iter().collect();
    if values.len() < window * 2 {
        return 0.;
    }
    let latest: f64 = values[values.len() - window..].iter().sum();
    let previous: f64 = values[values.len() - window * 2..values.len() - window]
        .iter()
        .sum();
    change_percent(latest, previous)
}

/// The sentence a card leads its footer with.
enum Headline {
    /// A trend arrow and `Trending up by 5.2% this month`.
    Trend { percent: f64, period: SharedString },
    /// A plain finding, such as the share the largest slice holds.
    Text(SharedString),
}

/// A chart card: heading, legend, the chart and a footer that reads the data.
struct Card {
    title: SharedString,
    period: SharedString,
    legend: Vec<(Hsla, SharedString)>,
    chart: AnyElement,
    headline: Headline,
    note: SharedString,
    /// Whether the heading and footer sit over the middle of the card, as
    /// the round charts want.
    centered: bool,
}

impl Card {
    fn new(title: impl Into<SharedString>, period: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            period: period.into(),
            legend: vec![],
            chart: div().into_any_element(),
            headline: Headline::Text("".into()),
            note: "".into(),
            centered: false,
        }
    }

    fn chart(mut self, chart: impl IntoElement) -> Self {
        self.chart = chart.into_any_element();
        self
    }

    fn legend(mut self, color: Hsla, label: impl Into<SharedString>) -> Self {
        self.legend.push((color, label.into()));
        self
    }

    fn trend(mut self, percent: f64, period: impl Into<SharedString>) -> Self {
        self.headline = Headline::Trend {
            percent,
            period: period.into(),
        };
        self
    }

    fn headline(mut self, text: impl Into<SharedString>) -> Self {
        self.headline = Headline::Text(text.into());
        self
    }

    fn note(mut self, note: impl Into<SharedString>) -> Self {
        self.note = note.into();
        self
    }

    fn centered(mut self) -> Self {
        self.centered = true;
        self
    }

    fn render(self, cx: &App) -> impl IntoElement {
        let centered = self.centered;
        let headline = match self.headline {
            Headline::Trend { percent, period } => {
                let (icon, color, direction) = if percent >= 0. {
                    (IconName::TrendingUp, cx.theme().success, "up")
                } else {
                    (IconName::TrendingDown, cx.theme().danger, "down")
                };
                h_flex()
                    .gap_1p5()
                    .items_center()
                    .when(centered, |this| this.justify_center())
                    .child(format!(
                        "Trending {direction} by {:.1}% {period}",
                        percent.abs()
                    ))
                    .child(Icon::new(icon).size_4().text_color(color))
                    .into_any_element()
            }
            Headline::Text(text) => div().child(text).into_any_element(),
        };

        v_flex()
            .flex_1()
            .min_w_0()
            .h(CARD_HEIGHT)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius_lg)
            .p_4()
            .child(
                h_flex()
                    .items_start()
                    .justify_between()
                    .when(centered, |this| this.justify_center())
                    .child(
                        // The heading holds its width; the legend beside it is
                        // what gives way and wraps.
                        v_flex()
                            .flex_shrink_0()
                            .when(centered, |this| this.text_center())
                            .child(div().font_semibold().child(self.title))
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .text_sm()
                                    .child(self.period),
                            ),
                    )
                    .when(!self.legend.is_empty() && !centered, |this| {
                        this.child(legend(self.legend.clone(), cx))
                    }),
            )
            // The round charts have no axis to anchor a legend beside, so
            // theirs sits under the heading.
            .when(!self.legend.is_empty() && centered, |this| {
                this.child(div().pt_2().child(legend(self.legend, cx).justify_center()))
            })
            .child(div().flex_1().min_h_0().py_4().child(self.chart))
            .child(
                div()
                    .when(centered, |this| this.text_center())
                    .font_semibold()
                    .text_sm()
                    .child(headline),
            )
            .child(
                div()
                    .when(centered, |this| this.text_center())
                    .text_color(cx.theme().muted_foreground)
                    .text_sm()
                    .child(self.note),
            )
    }
}

/// A row of swatch-and-label pairs.
///
/// It shares the heading row with the title, so it has to yield width rather
/// than hold its own: shrinking lets `flex_wrap` fold a long series list onto a
/// second line instead of running out past the card.
fn legend(entries: Vec<(Hsla, SharedString)>, cx: &App) -> gpui_kit::Div {
    h_flex()
        .flex_wrap()
        .justify_end()
        .gap_3()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .children(entries.into_iter().map(|(color, label)| {
            h_flex()
                .flex_shrink_0()
                .gap_1p5()
                .items_center()
                .child(div().size_2().rounded_sm().bg(color))
                .child(label)
        }))
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

/// The shade a category takes in a single-hue chart: the first category in
/// the full color, each next one a step more transparent, so a pie or a bar
/// group reads as one ramp rather than five competing hues.
fn shade(base: Hsla, index: usize) -> Hsla {
    base.alpha(1. - 0.14 * index as f32)
}

/// A stable ramp position for a category name, so the same category keeps its
/// shade across the cards that show it.
fn color_index(name: &str) -> usize {
    match name {
        "Direct" | "Chrome" | "Free" | "N. America" => 0,
        "Organic Search" | "Safari" | "Starter" | "Europe" => 1,
        "Social" | "Edge" | "Pro" | "APAC" => 2,
        "Referral" | "Firefox" | "Enterprise" | "LatAm" => 3,
        _ => 4,
    }
}

/// A fill that fades from `color` at the top of an area to nothing at its
/// baseline.
fn area_gradient(color: Hsla) -> Background {
    linear_gradient(
        0.,
        linear_color_stop(color.opacity(0.45), 1.),
        linear_color_stop(color.opacity(0.), 0.),
    )
}

/// A fill that shades a bar across its width, from `color` on one edge to a
/// lighter tint on the other, so the bar reads with a little depth while its
/// length stays evenly colored.
fn bar_shading(color: Hsla, alignment: BarAlignment) -> Background {
    let angle = if alignment.is_horizontal() { 0. } else { 90. };
    linear_gradient(
        angle,
        linear_color_stop(color, 0.),
        linear_color_stop(color.opacity(0.7), 1.),
    )
}

/// The corner radii that round only the tip end of a vertical bar.
fn rounded_tip() -> Corners<Pixels> {
    Corners {
        top_left: px(6.),
        top_right: px(6.),
        bottom_left: px(0.),
        bottom_right: px(0.),
    }
}

impl ChartCard {
    fn render(self, data: &ChartData, cx: &App) -> AnyElement {
        let accent = cx.theme().chart_2;
        let mid = cx.theme().chart_3;
        let deep = cx.theme().chart_4;
        let card = match self {
            Self::AreaStacked => Card::new("Visitors", "April – June 2025")
                .legend(accent, "Desktop")
                .legend(deep, "Mobile")
                .chart(
                    AreaChart::new(data.daily_devices.clone())
                        .x(|d| d.date.clone())
                        .y(|d| d.desktop)
                        .stroke(accent)
                        .fill(area_gradient(accent))
                        .name("Desktop")
                        .y(|d| d.mobile)
                        .stroke(deep)
                        .fill(area_gradient(deep))
                        .name("Mobile")
                        .tick_margin(8)
                        .id("area-chart-stacked"),
                )
                .trend(
                    windowed_change(data.daily_devices.iter().map(|d| d.desktop + d.mobile), 7),
                    "this week",
                )
                .note(format!(
                    "{} visitors over the last three months",
                    compact(
                        data.daily_devices
                            .iter()
                            .map(|d| d.desktop + d.mobile)
                            .sum()
                    )
                )),
            Self::Pie => {
                let total: f64 = data.traffic_sources.iter().map(|d| d.visitors).sum();
                let top = data
                    .traffic_sources
                    .iter()
                    .max_by(|a, b| a.visitors.total_cmp(&b.visitors));
                let mut card = Card::new("Traffic Sources", "June 2025")
                    .centered()
                    .chart(
                        PieChart::new(data.traffic_sources.clone())
                            .value(|d| d.visitors as f32)
                            .outer_radius(90.)
                            .color(move |d| shade(mid, color_index(&d.source)))
                            .name("Visitors")
                            .id("pie-chart"),
                    )
                    .note(format!("{} visitors across five channels", compact(total)));
                if let Some(top) = top {
                    card = card.headline(format!(
                        "{} brings {:.0}% of traffic",
                        top.source,
                        top.visitors / total * 100.
                    ));
                }
                for source in &data.traffic_sources {
                    card = card.legend(
                        shade(mid, color_index(&source.source)),
                        source.source.clone(),
                    );
                }
                card
            }
            Self::PieDonut => {
                let leader = &data.browsers[0];
                let mut card = Card::new("Browser Share", "June 2025")
                    .centered()
                    .chart(
                        div()
                            .relative()
                            .size_full()
                            .child(
                                PieChart::new(data.browsers.clone())
                                    .value(|d| d.share as f32)
                                    .inner_radius(58.)
                                    .outer_radius(90.)
                                    .color(move |d| shade(mid, color_index(&d.browser)))
                                    .name("Share")
                                    .id("pie-chart-donut"),
                            )
                            // The headline figure sits in the hole of the ring.
                            .child(
                                v_flex()
                                    .absolute()
                                    .inset_0()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_2xl()
                                            .font_semibold()
                                            .child(format!("{:.0}%", leader.share)),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(leader.browser.clone()),
                                    ),
                            ),
                    )
                    .headline(format!(
                        "{} leads by {:.0} points",
                        leader.browser,
                        leader.share - data.browsers[1].share
                    ))
                    .note("Share of sessions by browser family");
                for browser in &data.browsers {
                    card = card.legend(
                        shade(mid, color_index(&browser.browser)),
                        browser.browser.clone(),
                    );
                }
                card
            }
            Self::PiePadAngle => {
                let total: f64 = data.plans.iter().map(|d| d.accounts).sum();
                let paid: f64 = data
                    .plans
                    .iter()
                    .filter(|d| d.plan != "Free")
                    .map(|d| d.accounts)
                    .sum();
                let mut card = Card::new("Plan Mix", "June 2025")
                    .centered()
                    .chart(
                        PieChart::new(data.plans.clone())
                            .value(|d| d.accounts as f32)
                            .inner_radius(56.)
                            .outer_radius(90.)
                            .pad_angle(4. / 100.)
                            .color(move |d| shade(mid, color_index(&d.plan)))
                            .name("Accounts")
                            .id("pie-chart-pad-angle"),
                    )
                    .headline(format!(
                        "{:.0}% of accounts are on a paid plan",
                        paid / total * 100.
                    ))
                    .note(format!("{} accounts in total", compact(total)));
                for plan in &data.plans {
                    card = card.legend(shade(mid, color_index(&plan.plan)), plan.plan.clone());
                }
                card
            }
            Self::PieLabel => {
                let total: f64 = data.regions.iter().map(|d| d.revenue).sum();
                Card::new("Revenue by Region", "Q2 2025")
                    .centered()
                    .chart(
                        PieChart::new(data.regions.clone())
                            .value(|d| d.revenue as f32)
                            .inner_radius(48.)
                            .outer_radius(76.)
                            .color(move |d| shade(mid, color_index(&d.region)))
                            .label(|d| d.region.clone())
                            .name("Revenue")
                            .id("pie-chart-label"),
                    )
                    .headline(format!(
                        "{} of {} comes from the two largest regions",
                        money(data.regions[0].revenue + data.regions[1].revenue),
                        money(total)
                    ))
                    .note("Recognized revenue, in US dollars")
            }
            Self::Radar => {
                let average = data.product_scores.iter().map(|d| d.alpha).sum::<f64>()
                    / data.product_scores.len() as f64;
                Card::new("Product Score", "Alpha, Q2 review")
                    .centered()
                    .chart(
                        RadarChart::new(data.product_scores.clone())
                            .label(|d| d.dimension.clone())
                            .value(|d| d.alpha)
                            .stroke(accent)
                            .fill(accent.opacity(0.3))
                            .name("Alpha")
                            .max_value(100.)
                            .id("radar-chart"),
                    )
                    .headline(format!("Scores {average:.0} on average"))
                    .note("Six review dimensions, scored out of 100")
            }
            Self::RadarMultiple => {
                let (alpha, beta) = data
                    .product_scores
                    .iter()
                    .fold((0., 0.), |(a, b), d| (a + d.alpha, b + d.beta));
                Card::new("Alpha vs Beta", "Q2 review")
                    .centered()
                    .legend(accent, "Alpha")
                    .legend(deep, "Beta")
                    .chart(
                        RadarChart::new(data.product_scores.clone())
                            .label(|d| d.dimension.clone())
                            .value(|d| d.alpha)
                            .stroke(accent)
                            .fill(accent.opacity(0.25))
                            .name("Alpha")
                            .value(|d| d.beta)
                            .stroke(deep)
                            .fill(deep.opacity(0.25))
                            .name("Beta")
                            .max_value(100.)
                            .id("radar-chart-multiple"),
                    )
                    .headline(if alpha >= beta {
                        format!("Alpha leads by {:.0} points overall", alpha - beta)
                    } else {
                        format!("Beta leads by {:.0} points overall", beta - alpha)
                    })
                    .note("Alpha wins on usability, Beta on reliability")
            }
            Self::RadarDots => Card::new("Review Grades", "Alpha, Q2 review")
                .centered()
                .chart(
                    RadarChart::new(data.product_scores.clone())
                        // An element label: the dimension name over a grade badge.
                        .label({
                            let muted_foreground = cx.theme().muted_foreground;
                            let badge_radius = cx.theme().radius_full();

                            move |d: &ProductScore| {
                                let grade = match d.alpha {
                                    v if v >= 85. => "A",
                                    v if v >= 70. => "B",
                                    _ => "C",
                                };

                                v_flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted_foreground)
                                            .child(d.dimension.clone()),
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
                        .value(|d| d.alpha)
                        .name("Alpha")
                        .stroke(accent)
                        .fill(accent.opacity(0.25))
                        .max_value(100.)
                        .dot()
                        // A badge label is far taller than a line of text, so
                        // pull the ring in to leave it room.
                        .outer_radius(64.)
                        .id("radar-chart-dots"),
                )
                .headline("Two dimensions graded A")
                .note("A from 85, B from 70, C below"),
            Self::RadarLinesOnly => Card::new("Beta Profile", "Q2 review")
                .centered()
                .legend(deep, "Beta")
                .chart(
                    RadarChart::new(data.product_scores.clone())
                        .label(|d| d.dimension.clone())
                        .value(|d| d.beta)
                        .name("Beta")
                        .stroke(deep)
                        .fill(gpui_kit::transparent_black())
                        .max_value(100.)
                        .grid_levels(5)
                        .id("radar-chart-lines-only"),
                )
                .headline("Strongest on reliability and support")
                .note("Outline only, five grid rings"),
            Self::Bar => Card::new("Monthly Revenue", "2025")
                .chart(
                    BarChart::new(data.metrics.clone())
                        .band(|d| d.month.clone())
                        .value(|d| d.revenue)
                        .name("Revenue")
                        .fill(move |_, _, _, _| accent)
                        .corner_radii(rounded_tip())
                        .id("bar-chart"),
                )
                .trend(
                    latest_change(data.metrics.iter().map(|d| d.revenue)),
                    "this month",
                )
                .note(format!(
                    "{} recognized this year",
                    money(data.metrics.iter().map(|d| d.revenue).sum())
                )),
            Self::BarMixed => {
                let mut card = Card::new("Revenue by Region", "Q2 2025")
                    .chart(
                        BarChart::new(data.regions.clone())
                            .band(|d| d.region.clone())
                            .value(|d| d.revenue)
                            .name("Revenue")
                            .label(|d| money(d.revenue))
                            .fill(move |d, _, _, _| shade(mid, color_index(&d.region)))
                            .corner_radii(rounded_tip())
                            .id("bar-chart-mixed"),
                    )
                    .headline(format!(
                        "{} ahead of {} by {}",
                        data.regions[0].region,
                        data.regions[1].region,
                        money(data.regions[0].revenue - data.regions[1].revenue)
                    ))
                    .note("One shade per region");
                for region in &data.regions {
                    card = card.legend(
                        shade(mid, color_index(&region.region)),
                        region.region.clone(),
                    );
                }
                card
            }
            Self::BarStacked => {
                let days: Vec<_> = data.daily_devices.iter().take(8).cloned().collect();
                let total: f64 = days
                    .iter()
                    .map(|d| d.desktop + d.mobile + d.tablet + d.watch)
                    .sum();
                Card::new("Visitors by Device", "First week of April")
                    .legend(cx.theme().chart_4, "Desktop")
                    .legend(cx.theme().chart_3, "Mobile")
                    .legend(cx.theme().chart_2, "Tablet")
                    .legend(cx.theme().chart_1, "Watch")
                    .chart(StackedBarChart::new(days))
                    .headline(format!("{} visitors in eight days", compact(total)))
                    .note("Stacked by device, a custom Plot")
            }
            Self::BarRounded => Card::new("Signups", "2025")
                .chart(
                    BarChart::new(data.metrics.clone())
                        .band(|d| d.month.clone())
                        .value(|d| d.signups)
                        .name("Signups")
                        .label(|d| compact(d.signups))
                        .fill(move |_, _, _, _| accent)
                        .corner_radii(px(8.))
                        .id("bar-chart-rounded"),
                )
                .trend(
                    latest_change(data.metrics.iter().map(|d| d.signups)),
                    "this month",
                )
                .note(format!(
                    "{} new accounts this year; fully rounded bars",
                    compact(data.metrics.iter().map(|d| d.signups).sum())
                )),
            Self::BarBottomAligned => Card::new("Orders", "2025")
                .chart(
                    BarChart::new(data.metrics.clone())
                        .band(|d| d.month.clone())
                        .value(|d| d.orders)
                        .name("Orders")
                        .label(|d| compact(d.orders))
                        .fill(move |_, _, _, _| accent)
                        .id("bar-chart-bottom"),
                )
                .trend(
                    latest_change(data.metrics.iter().map(|d| d.orders)),
                    "this month",
                )
                .note("Bottom aligned: bars grow up from the axis"),
            Self::BarTopAligned => Card::new("Refunds", "2025")
                .chart(
                    BarChart::new(data.metrics.clone())
                        .band(|d| d.month.clone())
                        .value(|d| d.refunds)
                        .name("Refunds")
                        .label(|d| compact(d.refunds))
                        .fill(move |_, _, _, _| mid)
                        .alignment(BarAlignment::Top)
                        .id("bar-chart-top"),
                )
                .trend(
                    latest_change(data.metrics.iter().map(|d| d.refunds)),
                    "this month",
                )
                .note("Top aligned: bars hang from the axis"),
            Self::BarLeftAligned => Card::new("Top Products", "Units sold, Q2 2025")
                .chart(
                    BarChart::new(data.products.clone())
                        .band(|d| d.product.clone())
                        .value(|d| d.sales)
                        .name("Units")
                        .label(|d| compact(d.sales))
                        .fill(move |_, _, _, _| accent)
                        .alignment(BarAlignment::Left)
                        .id("bar-chart-left"),
                )
                .headline(format!(
                    "{} outsells the next product by {}",
                    data.products[0].product,
                    compact(data.products[0].sales - data.products[1].sales)
                ))
                .note("Left aligned: labels beside horizontal bars"),
            Self::BarRightAligned => Card::new("Page Views", "June 2025")
                .chart(
                    BarChart::new(data.pages.clone())
                        .band(|d| d.page.clone())
                        .value(|d| d.views)
                        .name("Views")
                        .label(|d| compact(d.views))
                        .fill(move |_, _, _, _| deep)
                        .alignment(BarAlignment::Right)
                        .id("bar-chart-right"),
                )
                .headline(format!("{} is the most visited page", data.pages[0].page))
                .note("Right aligned: bars grow leftward"),
            Self::BarNegative => {
                let positive = cx.theme().chart_bullish;
                let negative = cx.theme().chart_bearish;
                let net: f64 = data.cash_flow.iter().map(|d| d.revenue).sum();
                Card::new("Net Cash Flow", "2025")
                    .legend(positive, "Surplus")
                    .legend(negative, "Deficit")
                    .chart(
                        BarChart::new(data.cash_flow.clone())
                            .band(|d| d.month.clone())
                            .value(|d| d.revenue)
                            .name("Net")
                            .label(|d| money(d.revenue))
                            .fill(
                                move |d, _, _, _| {
                                    if d.revenue >= 0. { positive } else { negative }
                                },
                            )
                            .value_axis(true)
                            .id("bar-chart-negative"),
                    )
                    .headline(format!("{} net for the year", money(net)))
                    .note("Revenue less expenses; the axis sits at zero")
            }
            Self::BarGradientBottom => Card::new("Downloads", "2025")
                .chart(
                    BarChart::new(data.metrics.clone())
                        .band(|d| d.month.clone())
                        .value(|d| d.downloads)
                        .name("Downloads")
                        .label(|d| compact(d.downloads))
                        .fill(move |_, _, _, alignment| bar_shading(accent, alignment))
                        .id("bar-chart-gradient-bottom"),
                )
                .trend(
                    latest_change(data.metrics.iter().map(|d| d.downloads)),
                    "this month",
                )
                .note("Shaded across the width, not along the length"),
            Self::BarGradientTop => Card::new("Refunds", "2025")
                .chart(
                    BarChart::new(data.metrics.clone())
                        .band(|d| d.month.clone())
                        .value(|d| d.refunds)
                        .name("Refunds")
                        .label(|d| compact(d.refunds))
                        .alignment(BarAlignment::Top)
                        .fill(move |_, _, _, alignment| bar_shading(deep, alignment))
                        .id("bar-chart-gradient-top"),
                )
                .trend(
                    latest_change(data.metrics.iter().map(|d| d.refunds)),
                    "this month",
                )
                .note("The shading stays with the bar as it hangs"),
            Self::BarGradientLeft => Card::new("Page Views", "June 2025")
                .chart(
                    BarChart::new(data.pages.clone())
                        .band(|d| d.page.clone())
                        .value(|d| d.views)
                        .name("Views")
                        .label(|d| compact(d.views))
                        .alignment(BarAlignment::Left)
                        .fill(move |_, _, _, alignment| bar_shading(accent, alignment))
                        .id("bar-chart-gradient-left"),
                )
                .headline(format!(
                    "{} views across the top five pages",
                    compact(data.pages.iter().map(|d| d.views).sum())
                ))
                .note("Horizontal bars shade top to bottom"),
            Self::BarGradientRight => Card::new("Top Products", "Units sold, Q2 2025")
                .chart(
                    BarChart::new(data.products.clone())
                        .band(|d| d.product.clone())
                        .value(|d| d.sales)
                        .name("Units")
                        .label(|d| compact(d.sales))
                        .alignment(BarAlignment::Right)
                        .fill(move |_, _, _, alignment| bar_shading(deep, alignment))
                        .id("bar-chart-gradient-right"),
                )
                .headline(format!(
                    "{} units across the catalogue",
                    compact(data.products.iter().map(|d| d.sales).sum())
                ))
                .note("The same shading on right-aligned bars"),
            Self::BarGradientPerBar => Card::new("Sessions", "2025")
                .chart(
                    BarChart::new(data.metrics.clone())
                        .band(|d| d.month.clone())
                        .value(|d| d.sessions)
                        .name("Sessions")
                        .label(|d| compact(d.sessions))
                        .fill(move |_, _, _, alignment| bar_shading(mid, alignment))
                        .corner_radii(px(6.))
                        .id("bar-chart-gradient-per-bar"),
                )
                .trend(
                    latest_change(data.metrics.iter().map(|d| d.sessions)),
                    "this month",
                )
                .note("Fully rounded bars keep their shading"),
            Self::BarGradientDiagonal => {
                let c1 = cx.theme().chart_1;
                let c2 = cx.theme().chart_5;
                Card::new("Orders", "2025")
                    .chart(
                        BarChart::new(data.metrics.clone())
                            .band(|d| d.month.clone())
                            .value(|d| d.orders)
                            .name("Orders")
                            .label(|d| compact(d.orders))
                            .fill(move |_, bar, chart, _| {
                                // Project the bar's corners onto the chart's
                                // bottom-left → top-right diagonal so each bar
                                // shows the slice of a chart-wide diagonal
                                // gradient corresponding to its own footprint.
                                let w = chart.size.width.max(f32::EPSILON);
                                let h = chart.size.height.max(f32::EPSILON);
                                let denom = w * w + h * h;
                                let project =
                                    |x: f32, y: f32| -> f32 { (x * w + (h - y) * h) / denom };
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
                            .id("bar-chart-gradient-diagonal"),
                    )
                    .trend(
                        latest_change(data.metrics.iter().map(|d| d.orders)),
                        "this month",
                    )
                    .note("One diagonal gradient across all bars")
            }
            Self::Line => Card::new("Monthly Recurring Revenue", "2025")
                .chart(
                    LineChart::new(data.metrics.clone())
                        .x(|d| d.month.clone())
                        .y(|d| d.mrr)
                        .stroke(accent)
                        .name("MRR")
                        .id("line-chart"),
                )
                .trend(
                    latest_change(data.metrics.iter().map(|d| d.mrr)),
                    "this month",
                )
                .note(format!(
                    "{} MRR in December, up from {} in January",
                    money(data.metrics[data.metrics.len() - 1].mrr),
                    money(data.metrics[0].mrr)
                )),
            Self::LineLinear => Card::new("Conversion Rate", "2025")
                .chart(
                    LineChart::new(data.metrics.clone())
                        .x(|d| d.month.clone())
                        .y(|d| d.conversion)
                        .stroke(deep)
                        .linear()
                        .name("Conversion %")
                        .id("line-chart-linear"),
                )
                .trend(
                    latest_change(data.metrics.iter().map(|d| d.conversion)),
                    "this month",
                )
                .note("Visitors who signed up; straight segments"),
            Self::LineStepAfter => Card::new("Active Subscriptions", "2025")
                .chart(
                    LineChart::new(data.metrics.clone())
                        .x(|d| d.month.clone())
                        .y(|d| d.subscriptions)
                        .stroke(accent)
                        .step_after()
                        .name("Subscriptions")
                        .id("line-chart-step-after"),
                )
                .trend(
                    latest_change(data.metrics.iter().map(|d| d.subscriptions)),
                    "this month",
                )
                .note("Counts change on renewal day; step after"),
            Self::LineDots => Card::new("Deploys", "Per month, 2025")
                .chart(
                    LineChart::new(data.metrics.clone())
                        .x(|d| d.month.clone())
                        .y(|d| d.deploys)
                        .dot()
                        .stroke(cx.theme().chart_5)
                        .name("Deploys")
                        .id("line-chart-dots"),
                )
                .trend(
                    latest_change(data.metrics.iter().map(|d| d.deploys)),
                    "this month",
                )
                .note(format!(
                    "{} production deploys this year",
                    compact(data.metrics.iter().map(|d| d.deploys).sum())
                )),
            Self::Area => Card::new("Active Users", "2025")
                .chart(
                    AreaChart::new(data.metrics.clone())
                        .x(|d| d.month.clone())
                        .y(|d| d.active_users)
                        .stroke(accent)
                        .fill(accent.opacity(0.3))
                        .name("Active users")
                        .id("area-chart"),
                )
                .trend(
                    latest_change(data.metrics.iter().map(|d| d.active_users)),
                    "this month",
                )
                .note("Monthly active users; a flat fill"),
            Self::AreaLinear => Card::new("Sessions", "2025")
                .chart(
                    AreaChart::new(data.metrics.clone())
                        .x(|d| d.month.clone())
                        .y(|d| d.sessions)
                        .stroke(deep)
                        .fill(deep.opacity(0.3))
                        .linear()
                        .name("Sessions")
                        .id("area-chart-linear"),
                )
                .trend(
                    latest_change(data.metrics.iter().map(|d| d.sessions)),
                    "this month",
                )
                .note("Straight segments between months"),
            Self::AreaStepAfter => Card::new("Storage Used", "Terabytes, 2025")
                .chart(
                    AreaChart::new(data.metrics.clone())
                        .x(|d| d.month.clone())
                        .y(|d| d.storage_tb)
                        .stroke(mid)
                        .fill(mid.opacity(0.3))
                        .step_after()
                        .name("TB")
                        .id("area-chart-step-after"),
                )
                .headline(format!(
                    "{:.1} TB provisioned, from {:.1} TB in January",
                    data.metrics[data.metrics.len() - 1].storage_tb,
                    data.metrics[0].storage_tb
                ))
                .note("Capacity is added in steps"),
            Self::AreaGradient => Card::new("Revenue vs Last Year", "2025")
                .legend(accent, "2025")
                .legend(cx.theme().chart_1, "2024")
                .chart(
                    AreaChart::new(data.metrics.clone())
                        .x(|d| d.month.clone())
                        .y(|d| d.last_year)
                        .stroke(cx.theme().chart_1)
                        .fill(area_gradient(cx.theme().chart_1))
                        .name("2024")
                        .y(|d| d.revenue)
                        .stroke(accent)
                        .fill(area_gradient(accent))
                        .name("2025")
                        .id("area-chart-gradient"),
                )
                .trend(
                    change_percent(
                        data.metrics.iter().map(|d| d.revenue).sum(),
                        data.metrics.iter().map(|d| d.last_year).sum(),
                    ),
                    "year over year",
                )
                .note("Gradient fills fade to the baseline"),
            // Forty sessions do not fit forty labels, so every card thins them.
            Self::Candlestick => self.candlestick(data, "Daily", 0.8, 5, "candlestick-chart"),
            Self::CandlestickNarrow => {
                self.candlestick(data, "Narrow bodies", 0.5, 5, "candlestick-chart-narrow")
            }
            Self::CandlestickWide => {
                self.candlestick(data, "Wide bodies", 1.0, 5, "candlestick-chart-wide")
            }
            Self::CandlestickTickMargin => self.candlestick(
                data,
                "Every tenth label",
                0.8,
                10,
                "candlestick-chart-tick-margin",
            ),
            Self::Sankey(index) => {
                let Some((period, nodes, links)) = data.tsla_statements.get(index) else {
                    return div().into_any_element();
                };

                // Sqrt value scale keeps the huge revenue flow from
                // dwarfing the small profit/expense ones.
                let chart = SankeyChart::new(nodes.clone(), links.clone())
                    .id(("sankey-chart", index))
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

                let revenue = nodes.first();
                Card::new("TSLA Income Statement", period.clone())
                    .chart(chart)
                    .headline(match revenue {
                        Some(node) => format!(
                            "{} of revenue, {}",
                            money(node.value),
                            match node.growth {
                                Some(growth) if growth >= 0. => {
                                    format!("up {:.1}% year over year", growth)
                                }
                                Some(growth) => format!("down {:.1}% year over year", -growth),
                                None => "flat year over year".to_string(),
                            }
                        ),
                        None => "No revenue reported".to_string(),
                    })
                    .note("How revenue flows into profit and expenses")
            }
        };

        card.render(cx).into_any_element()
    }

    /// A card over the 40-session price series with the given body width and
    /// label stride.
    fn candlestick(
        self,
        data: &ChartData,
        variant: &'static str,
        body_width_ratio: f32,
        tick_margin: usize,
        id: &'static str,
    ) -> Card {
        let (first, last) = (
            &data.stock_prices[0],
            &data.stock_prices[data.stock_prices.len() - 1],
        );
        let high = data
            .stock_prices
            .iter()
            .map(|d| d.high)
            .fold(f64::MIN, f64::max);
        let low = data
            .stock_prices
            .iter()
            .map(|d| d.low)
            .fold(f64::MAX, f64::min);
        Card::new(
            "ACME Daily",
            format!("{} – {} 2025 · {variant}", first.date, last.date),
        )
        .chart(
            CandlestickChart::new(data.stock_prices.clone())
                .x(|d| d.date.clone())
                .open(|d| d.open)
                .high(|d| d.high)
                .low(|d| d.low)
                .close(|d| d.close)
                .body_width_ratio(body_width_ratio)
                .tick_margin(tick_margin)
                .id(id),
        )
        .trend(change_percent(last.close, first.open), "over 40 sessions")
        .note(format!(
            "Closed at ${:.2}; ranged ${:.2} – ${:.2}",
            last.close, low, high
        ))
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

fn fixture<T: for<'de> Deserialize<'de>>(json: &str) -> T {
    serde_json::from_str(json).expect("fixture is valid JSON")
}

impl ChartStory {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let daily_devices: Vec<DailyDevice> =
            fixture(include_str!("../../fixtures/daily-devices.json"));
        let metrics: Vec<MonthlyMetric> =
            fixture(include_str!("../../fixtures/monthly-metrics.json"));
        let stock_prices: Vec<StockPrice> =
            fixture(include_str!("../../fixtures/stock-prices.json"));
        let tsla: TslaIncomeStatement =
            fixture(include_str!("../../fixtures/tsla-income-statement.json"));
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

        // Net cash flow rides in the `revenue` field so the bar chart reads it
        // with the same accessor.
        let cash_flow = metrics
            .iter()
            .map(|d| MonthlyMetric {
                revenue: d.revenue - d.expenses,
                ..d.clone()
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
                metrics,
                cash_flow,
                traffic_sources: fixture(include_str!("../../fixtures/traffic-sources.json")),
                browsers: fixture(include_str!("../../fixtures/browsers.json")),
                plans: fixture(include_str!("../../fixtures/plans.json")),
                regions: fixture(include_str!("../../fixtures/regions.json")),
                products: fixture(include_str!("../../fixtures/products.json")),
                pages: fixture(include_str!("../../fixtures/pages.json")),
                product_scores: fixture(include_str!("../../fixtures/product-scores.json")),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_and_money() {
        assert_eq!(compact(0.), "0");
        assert_eq!(compact(437.), "437");
        assert_eq!(compact(1_234.), "1.2K");
        assert_eq!(compact(38_400.), "38K");
        assert_eq!(compact(1_234_567.), "1.2M");
        assert_eq!(money(38_400.), "$38K");
        assert_eq!(money(-2_500.), "-$2.5K");
    }

    #[test]
    fn test_changes() {
        assert_eq!(change_percent(110., 100.), 10.);
        assert_eq!(change_percent(90., 100.), -10.);
        assert_eq!(change_percent(5., 0.), 0.);

        assert_eq!(latest_change([100., 120., 150.]), 25.);
        assert_eq!(latest_change([100.]), 0.);

        // The last two values against the two before them.
        assert_eq!(windowed_change([1., 1., 2., 2.], 2), 100.);
        assert_eq!(windowed_change([1., 2., 3.], 2), 0.);
    }
}
