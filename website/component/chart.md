---
title: Chart
description: Beautiful charts and graphs for data visualization including line, bar, area, pie, radar, candlestick, and sankey charts.
---

# Chart

A comprehensive charting library providing Line, Bar, Area, Pie, Radar, Candlestick, and Sankey charts for data visualization. The charts feature smooth animations, customizable styling, tooltips, legends, and automatic theming that adapts to your application's theme.

## Import

```rust
use gpui_kit::component::chart::{
    LineChart, BarChart, AreaChart, PieChart, RadarChart, CandlestickChart, SankeyChart,
};
```

## Chart Types

### LineChart

A line chart displays data points connected by straight line segments, perfect for showing trends over time.

#### Basic Line Chart

```rust
#[derive(Clone)]
struct DataPoint {
    x: String,
    y: f64,
}

let data = vec![
    DataPoint { x: "Jan".to_string(), y: 100.0 },
    DataPoint { x: "Feb".to_string(), y: 150.0 },
    DataPoint { x: "Mar".to_string(), y: 120.0 },
];

LineChart::new(data)
    .x(|d| d.x.clone())
    .y(|d| d.y)
```

#### Line Chart Variants

```rust
// Basic curved line (default)
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)

// Linear interpolation
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .linear()

// Step after interpolation
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .step_after()

// With dots at data points
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .dot()

// Custom stroke color
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .stroke(cx.theme().success)
```

#### Tick Control

```rust
// Show every tick
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .tick_margin(1)

// Show every 2nd tick
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .tick_margin(2)
```

### BarChart

A bar chart uses rectangular bars to show comparisons among categories. Bars can be oriented vertically or horizontally via the `alignment` option.

#### Basic Bar Chart

```rust
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
```

#### Bar Chart Customization

```rust
// Custom fill colors
//
// The `fill` closure receives the datum, the bar's bounds (in pixel space,
// relative to the chart), the chart's bounds, and the bar's `BarAlignment`.
// Any value convertible to `Background` may be returned (solid color, gradient,
// pattern, etc.).
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .fill(|d, _bar_bounds, _chart_bounds, _alignment| d.color)

// With value labels on bars
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .label(|d| format!("{}", d.value))

// Custom tick spacing
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .tick_margin(2)

// Hide the band-axis line and labels
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .label_axis(false)
```

#### Bar Chart Gradient Fills

For gradient fills aligned to the bar's orientation, use `fill_gradient`. The closure receives the datum, the chart's full data range, and a `chart_to_bar` helper that maps a chart-value coordinate to a bar-local gradient position (`0.0` is the bar's base, `1.0` is its tip). The gradient angle is derived from the bar's `BarAlignment` so stop-0 sits at the base and stop-1 at the tip.

```rust
use gpui_kit::linear_color_stop;

// Per-bar gradient: every bar fades from a translucent base to its full color
// at the tip, regardless of its value.
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .fill_gradient(|d, _chart_range, _chart_to_bar| {
        let c = d.color;
        [
            linear_color_stop(c.opacity(0.3), 0.0),
            linear_color_stop(c, 1.0),
        ]
    })

// Chart-wide gradient: each bar shows the slice of a single gradient
// spanning the chart's full data range. Stops outside `[0, 1]` are clipped
// to the bar with colors interpolated at the clip points.
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .fill_gradient(|d, chart_range, chart_to_bar| {
        let c = d.color;
        [
            linear_color_stop(c.opacity(0.3), chart_to_bar(*chart_range.start())),
            linear_color_stop(c,              chart_to_bar(*chart_range.end())),
        ]
    })
```

`fill` and `fill_gradient` are mutually exclusive — setting one clears the other.

#### Bar Chart Alignment

`BarAlignment` controls the bar orientation and the side where the baseline sits. Import it from `gpui_kit::component::plot::shape`.

```rust
use gpui_kit::component::plot::shape::BarAlignment;

// Default: vertical bars growing upward from the bottom
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .alignment(BarAlignment::Bottom)

// Vertical bars growing downward from the top
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .alignment(BarAlignment::Top)

// Horizontal bars growing rightward from the left
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .alignment(BarAlignment::Left)

// Horizontal bars growing leftward from the right
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .alignment(BarAlignment::Right)
```

#### Bar Chart Corner Radii

Round the bar rectangles. Pass any value convertible into `Corners<Pixels>` —
use a single `px(..)` for uniform rounding, or construct `Corners` manually to
round only specific corners (e.g. just the tip end of each bar).

```rust
use gpui_kit::{px, Corners};

// Uniform 4px rounded corners on every bar
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .corner_radii(px(4.))

// Round only the top corners (tip end for bottom-aligned bars)
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .corner_radii(Corners {
        top_left: px(4.),
        top_right: px(4.),
        bottom_left: px(0.),
        bottom_right: px(0.),
    })
```

#### Bar Chart Negative Values

Bars grow from zero rather than from the edge of the plot, so negative values
extend to the opposite side of the zero line. The band-axis line follows zero,
and each category label moves to whichever side its own bar leaves empty. No
configuration is needed — a data set containing negative values renders this way.

```rust
// `growth` may be negative; bars below the zero line are drawn downward
BarChart::new(data)
    .band(|d| d.quarter.clone())
    .value(|d| d.growth)
    .label(|d| format!("{:+.0}%", d.growth))
```

#### Bar Chart Value Axis

Show tick labels for the value scale with `value_axis`, and control how many even
intervals the scale is divided into with `value_tick_count`. The count drives both
the grid line spacing and the tick labels, so the two always agree.

Note that `value_tick_count` is a count, whereas `tick_margin` is a stride over
the band-axis categories — `tick_margin(2)` keeps every second category label.

```rust
// Value labels left of vertical bars, below horizontal ones
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .value_axis(true)

// Divide the value scale into 6 intervals instead of the default 4
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .value_axis(true)
    .value_tick_count(6)
```

### AreaChart

An area chart displays quantitative data visually, similar to a line chart but with the area below the line filled.

#### Basic Area Chart

```rust
AreaChart::new(data)
    .x(|d| d.time.clone())
    .y(|d| d.value)
```

#### Stacked Area Charts

```rust
// Multi-series area chart
AreaChart::new(data)
    .x(|d| d.date.clone())
    .y(|d| d.desktop)  // First series
    .stroke(cx.theme().chart_1)
    .fill(cx.theme().chart_1.opacity(0.4))
    .y(|d| d.mobile)   // Second series
    .stroke(cx.theme().chart_2)
    .fill(cx.theme().chart_2.opacity(0.4))
```

#### Area Chart Styling

```rust
use gpui_kit::{linear_gradient, linear_color_stop};

// With gradient fill
AreaChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .fill(linear_gradient(
        0.,
        linear_color_stop(cx.theme().chart_1.opacity(0.4), 1.),
        linear_color_stop(cx.theme().background.opacity(0.3), 0.),
    ))

// Different interpolation styles
AreaChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .linear()  // or .step_after()
```

### PieChart

A pie chart displays data as slices of a circular chart, ideal for showing proportions.

#### Basic Pie Chart

```rust
PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
```

#### Donut Chart

```rust
PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
    .inner_radius(60.) // Creates donut effect
```

#### Pie Chart Customization

```rust
// Custom colors
PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
    .color(|d| d.color)

// With padding between slices
PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
    .inner_radius(60.)
    .pad_angle(4. / 100.) // 4% padding
```

### RadarChart

A radar chart displays multivariate data as closed polygons around a center, ideal for comparing multiple series across several dimensions.

#### Basic Radar Chart

```rust
RadarChart::new(data)
    .label(|d| d.month.clone())
    .value(|d| d.desktop)
```

#### Multiple Series

```rust
// Each `.value()` call adds a series, paired with the matching
// `.stroke()` / `.fill()` calls. Colors default to the theme
// chart colors, cycled per series.
RadarChart::new(data)
    .label(|d| d.month.clone())
    .value(|d| d.desktop)
    .stroke(cx.theme().chart_1)
    .value(|d| d.mobile)
    .stroke(cx.theme().chart_2)
```

#### Element Labels

`label` accepts either a string or a custom element. Return
`element.into_any_element()` to render anything you like around the outer ring —
an icon, several lines, per-dimension colors.

```rust
RadarChart::new(data)
    .label({
        let foreground = cx.theme().foreground;
        let muted_foreground = cx.theme().muted_foreground;

        move |d: &Device| {
            v_flex()
                .items_center()
                .child(div().text_xs().text_color(foreground).child(d.month.clone()))
                .child(
                    div()
                        .text_xs()
                        .text_color(muted_foreground)
                        .child(format!("{:.0}", d.desktop)),
                )
                .into_any_element()
        }
    })
    .value(|d| d.desktop)
```

Each label is measured at its natural size and pushed radially outward from its
dimension, so even a tall one clears the outer ring. Element labels style
themselves, so `.label_color()` does not apply to them, and they supply no
tooltip title (a string label does).

The ring is not shrunk to make room: the default outer radius is 40% of the
chart's height, so a label much taller than a line of text needs a smaller
`.outer_radius()` to keep it inside the chart's bounds.

#### Radar Chart Customization

```rust
// Vertex dots and custom fill
RadarChart::new(data)
    .label(|d| d.month.clone())
    .value(|d| d.desktop)
    .stroke(cx.theme().chart_2)
    .fill(cx.theme().chart_2.opacity(0.2))
    .dot()

// Fixed outer ring value and grid rings
RadarChart::new(data)
    .label(|d| d.month.clone())
    .value(|d| d.desktop)
    .max_value(400.)
    .grid_levels(5)
    .outer_radius(120.)
```

### CandlestickChart

A candlestick chart displays financial data using OHLC (Open, High, Low, Close) values, perfect for visualizing stock prices and market trends.

#### Basic Candlestick Chart

```rust
#[derive(Clone)]
struct StockPrice {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

let data = vec![
    StockPrice { date: "Jan".to_string(), open: 100.0, high: 110.0, low: 95.0, close: 105.0 },
    StockPrice { date: "Feb".to_string(), open: 105.0, high: 115.0, low: 100.0, close: 112.0 },
    StockPrice { date: "Mar".to_string(), open: 112.0, high: 120.0, low: 108.0, close: 115.0 },
];

CandlestickChart::new(data)
    .x(|d| d.date.clone())
    .open(|d| d.open)
    .high(|d| d.high)
    .low(|d| d.low)
    .close(|d| d.close)
```

#### Candlestick Chart Customization

```rust
// Adjust body width ratio (default: 0.6)
CandlestickChart::new(data)
    .x(|d| d.date.clone())
    .open(|d| d.open)
    .high(|d| d.high)
    .low(|d| d.low)
    .close(|d| d.close)
    .body_width_ratio(0.4) // Narrower bodies

// Custom tick spacing
CandlestickChart::new(data)
    .x(|d| d.date.clone())
    .open(|d| d.open)
    .high(|d| d.high)
    .low(|d| d.low)
    .close(|d| d.close)
    .tick_margin(2) // Show every 2nd tick
```

#### Candlestick Chart Colors

The candlestick chart automatically uses theme colors:

- **Bullish** (close > open): `bullish` color (green)
- **Bearish** (close < open): `bearish` color (red)

### SankeyChart

A sankey diagram visualizes flows between nodes, ideal for financial statements, energy flows, and traffic analysis. The layout algorithm mirrors [d3-sankey](https://github.com/d3/d3-sankey).

#### Basic Sankey Chart

```rust
use gpui_kit::component::plot::shape::SankeyLink;

#[derive(Clone)]
struct FlowNode {
    pub name: SharedString,
}

let nodes = vec![
    FlowNode { name: "Revenue".into() },
    FlowNode { name: "Gross Profit".into() },
    FlowNode { name: "Cost".into() },
];

// Links reference nodes by their index in `nodes`.
let links = vec![
    SankeyLink::new(0, 1, 45.0),
    SankeyLink::new(0, 2, 55.0),
];

SankeyChart::new(nodes, links)
    .node_label(|d| d.name.clone())
    .value_label(|_, value| format!("{:.1}", value).into())
```

The value label is drawn above the name label. Its closure receives the node's computed throughput (the larger of incoming and outgoing flow).

#### Node Alignment

```rust
use gpui_kit::component::plot::shape::SankeyAlign;

// Justify (default): nodes without outgoing links move to the last column
SankeyChart::new(nodes, links).node_align(SankeyAlign::Justify)

// Left: nodes stay at their topological depth
SankeyChart::new(nodes, links).node_align(SankeyAlign::Left)

// Also available: SankeyAlign::Right, SankeyAlign::Center
```

#### Sankey Chart Styling

```rust
SankeyChart::new(nodes, links)
    .node_width(8.)             // Node bar width (default: 10)
    .node_padding(20.)          // Vertical gap between nodes in a column (default: 16)
    .node_corner_radius(px(2.)) // Corner radius of node bars (default: 0)
    .node_color(|d| d.color)    // Per-node color; defaults to the theme chart palette
    .link_opacity(0.4)          // Ribbon opacity (default: 0.3)
    .min_link_width(2.)         // Minimum ribbon thickness (default: 1)
    .iterations(10)             // Layout relaxation passes (default: 6)
```

Link ribbons are filled with a horizontal gradient from the source node color to the target node color.

#### Custom Labels

For full control over the label lines, use `labels` — one `SankeyLabel` per line, top to bottom, each with its own color and font size. It takes precedence over `node_label`/`value_label` when set. For example, a financial-statement label with a year-over-year change line:

```rust
use gpui_kit::component::chart::SankeyLabel;

SankeyChart::new(nodes, links).labels(move |d: &FlowNode, value| {
    let arrow = if d.growth >= 0. { "▲" } else { "▼" };
    let growth_color = if d.growth >= 0. { green } else { red };
    vec![
        SankeyLabel::new(format!("{:.1}", value)),
        SankeyLabel::new(format!("{} {:+.2}%", arrow, d.growth)).color(growth_color),
        SankeyLabel::new(d.name.clone()).color(muted),
    ]
})
```

Line color defaults to the theme foreground and font size to 10; the chart keeps handling placement, alignment and margin reservation. A first/last-column label wider than its reserved margin is truncated with a trailing ellipsis rather than drawn outside the plot, so break or shorten long labels yourself if you want the full text on multiple lines.

#### Compressing Large Value Ranges

Node heights are linear in flow value by default, so a large value range (e.g. 200:1) leaves the small flows nearly invisible and the dominant flow oversized. Set `value_scale(SankeyValueScale::Sqrt)` to compress the range — the component sizes nodes by the square root of the value, so small flows stay visible without pre-transforming the data, and labels still receive the raw values:

```rust
use gpui_kit::component::plot::shape::SankeyValueScale;

SankeyChart::new(nodes, links).value_scale(SankeyValueScale::Sqrt)
```

Every node stays exactly filled by its ribbons under either scale, so children always match their parent's height.

## Data Structures

### Example Data Types

```rust
// Time series data
#[derive(Clone)]
struct DailyDevice {
    pub date: String,
    pub desktop: f64,
    pub mobile: f64,
}

// Category data with styling
#[derive(Clone)]
struct MonthlyDevice {
    pub month: String,
    pub desktop: f64,
    pub color_alpha: f32,
}

impl MonthlyDevice {
    pub fn color(&self, base_color: Hsla) -> Hsla {
        base_color.alpha(self.color_alpha)
    }
}

// Financial data
#[derive(Clone)]
struct StockPrice {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}

// Sankey flow: nodes are referenced by index (from gpui_kit::component::plot::shape)
pub struct SankeyLink {
    pub source: usize,
    pub target: usize,
    pub value: f64,
}
```

## Chart Configuration

### Container Setup

```rust
fn chart_container(
    title: &str,
    chart: impl IntoElement,
    center: bool,
    cx: &mut Context<ChartStory>,
) -> impl IntoElement {
    v_flex()
        .flex_1()
        .h_full()
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius_lg)
        .p_4()
        .child(
            div()
                .when(center, |this| this.text_center())
                .font_semibold()
                .child(title.to_string()),
        )
        .child(
            div()
                .when(center, |this| this.text_center())
                .text_color(cx.theme().muted_foreground)
                .text_sm()
                .child("Data period label"),
        )
        .child(div().flex_1().py_4().child(chart))
        .child(
            div()
                .when(center, |this| this.text_center())
                .font_semibold()
                .text_sm()
                .child("Summary statistic"),
        )
        .child(
            div()
                .when(center, |this| this.text_center())
                .text_color(cx.theme().muted_foreground)
                .text_sm()
                .child("Additional context"),
        )
}
```

### Theme Integration

```rust
// Charts automatically use theme colors
let chart = LineChart::new(data)
    .x(|d| d.date.clone())
    .y(|d| d.value)
    .stroke(cx.theme().chart_1); // Uses theme chart colors

// Available theme chart colors:
// cx.theme().chart_1
// cx.theme().chart_2
// cx.theme().chart_3
// ... up to chart_5
```

## API Reference

- [LineChart]
- [BarChart]
- [AreaChart]
- [PieChart]
- [RadarChart]
- [CandlestickChart]
- [SankeyChart]

## Examples

### Sales Dashboard

```rust
#[derive(Clone)]
struct SalesData {
    month: String,
    revenue: f64,
    profit: f64,
    region: String,
}

fn sales_dashboard(data: Vec<SalesData>, cx: &mut Context<Self>) -> impl IntoElement {
    v_flex()
        .gap_4()
        .child(
            h_flex()
                .gap_4()
                .child(
                    chart_container(
                        "Monthly Revenue",
                        LineChart::new(data.clone())
                            .x(|d| d.month.clone())
                            .y(|d| d.revenue)
                            .stroke(cx.theme().chart_1)
                            .dot(),
                        false,
                        cx,
                    )
                )
                .child(
                    chart_container(
                        "Profit Breakdown",
                        PieChart::new(data.clone())
                            .value(|d| d.profit as f32)
                            .outer_radius(80.)
                            .color(|d| match d.region.as_str() {
                                "North" => cx.theme().chart_1,
                                "South" => cx.theme().chart_2,
                                "East" => cx.theme().chart_3,
                                "West" => cx.theme().chart_4,
                                _ => cx.theme().chart_5,
                            }),
                        true,
                        cx,
                    )
                )
        )
        .child(
            chart_container(
                "Regional Performance",
                BarChart::new(data)
                    .band(|d| d.region.clone())
                    .value(|d| d.revenue)
                    .fill(|d, _, _, _| match d.region.as_str() {
                        "North" => cx.theme().chart_1,
                        "South" => cx.theme().chart_2,
                        "East" => cx.theme().chart_3,
                        "West" => cx.theme().chart_4,
                        _ => cx.theme().chart_5,
                    })
                    .label(|d| format!("${:.0}k", d.revenue / 1000.)),
                false,
                cx,
            )
        )
}
```

### Multi-Series Time Chart

```rust
#[derive(Clone)]
struct DeviceUsage {
    date: String,
    desktop: f64,
    mobile: f64,
    tablet: f64,
}

fn device_usage_chart(data: Vec<DeviceUsage>, cx: &mut Context<Self>) -> impl IntoElement {
    chart_container(
        "Device Usage Over Time",
        AreaChart::new(data)
            .x(|d| d.date.clone())
            .y(|d| d.desktop)
            .stroke(cx.theme().chart_1)
            .fill(linear_gradient(
                0.,
                linear_color_stop(cx.theme().chart_1.opacity(0.4), 1.),
                linear_color_stop(cx.theme().background.opacity(0.3), 0.),
            ))
            .y(|d| d.mobile)
            .stroke(cx.theme().chart_2)
            .fill(linear_gradient(
                0.,
                linear_color_stop(cx.theme().chart_2.opacity(0.4), 1.),
                linear_color_stop(cx.theme().background.opacity(0.3), 0.),
            ))
            .y(|d| d.tablet)
            .stroke(cx.theme().chart_3)
            .fill(linear_gradient(
                0.,
                linear_color_stop(cx.theme().chart_3.opacity(0.4), 1.),
                linear_color_stop(cx.theme().background.opacity(0.3), 0.),
            ))
            .tick_margin(3),
        false,
        cx,
    )
}
```

### Financial Chart

```rust
#[derive(Clone)]
struct StockData {
    date: String,
    price: f64,
    volume: u64,
}

#[derive(Clone)]
struct StockOHLC {
    date: String,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

fn stock_chart(ohlc_data: Vec<StockOHLC>, price_data: Vec<StockData>, cx: &mut Context<Self>) -> impl IntoElement {
    v_flex()
        .gap_4()
        .child(
            chart_container(
                "Stock Price - Candlestick",
                CandlestickChart::new(ohlc_data.clone())
                    .x(|d| d.date.clone())
                    .open(|d| d.open)
                    .high(|d| d.high)
                    .low(|d| d.low)
                    .close(|d| d.close)
                    .tick_margin(3),
                false,
                cx,
            )
        )
        .child(
            chart_container(
                "Stock Price - Line",
                LineChart::new(price_data.clone())
                    .x(|d| d.date.clone())
                    .y(|d| d.price)
                    .stroke(cx.theme().chart_1)
                    .linear()
                    .tick_margin(5),
                false,
                cx,
            )
        )
        .child(
            chart_container(
                "Trading Volume",
                BarChart::new(price_data)
                    .band(|d| d.date.clone())
                    .value(|d| d.volume as f64)
                    .fill(|d, _, _, _| {
                        if d.volume > 1000000 {
                            cx.theme().chart_1
                        } else {
                            cx.theme().muted_foreground.opacity(0.6)
                        }
                    })
                    .tick_margin(5),
                false,
                cx,
            )
        )
}
```

## Customization Options

### Color Schemes

```rust
// Theme-based colors (recommended)
LineChart::new(data)
    .x(|d| d.x.clone())
    .y(|d| d.y)
    .stroke(cx.theme().chart_1)

// Custom color palette
let colors = [
    cx.theme().success,
    cx.theme().warning,
    cx.theme().destructive,
    cx.theme().info,
    cx.theme().chart_1,
];

BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .fill(|d, _, _, _| colors[d.category_index % colors.len()])
```

### Responsive Design

```rust
// Container with responsive sizing
div()
    .flex_1()
    .min_h(px(300.))
    .max_h(px(600.))
    .w_full()
    .child(
        LineChart::new(data)
            .x(|d| d.x.clone())
            .y(|d| d.y)
    )
```

### Grid and Axis Styling

Charts automatically include:

- Grid lines with dashed appearance
- X-axis labels with smart positioning
- Y-axis scaling starting from zero
- Responsive tick spacing based on `tick_margin`

## Performance Considerations

### Large Datasets

```rust
// For large datasets, consider data sampling
let sampled_data: Vec<_> = data
    .iter()
    .step_by(5) // Show every 5th point
    .cloned()
    .collect();

LineChart::new(sampled_data)
    .x(|d| d.date.clone())
    .y(|d| d.value)
    .tick_margin(3) // Reduce tick density
```

### Memory Optimization

```rust
// Use efficient data accessors
LineChart::new(data)
    .x(|d| d.date.clone()) // Clone only when necessary
    .y(|d| d.value)        // Direct field access
```

## Integration Examples

### With State Management

```rust
struct ChartComponent {
    data: Vec<DataPoint>,
    chart_type: ChartType,
    time_range: TimeRange,
}

impl ChartComponent {
    fn render_chart(&self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.chart_type {
            ChartType::Line => LineChart::new(self.filtered_data())
                .x(|d| d.date.clone())
                .y(|d| d.value)
                .into_any_element(),
            ChartType::Bar => BarChart::new(self.filtered_data())
                .band(|d| d.date.clone())
                .value(|d| d.value)
                .into_any_element(),
            ChartType::Area => AreaChart::new(self.filtered_data())
                .x(|d| d.date.clone())
                .y(|d| d.value)
                .into_any_element(),
        }
    }

    fn filtered_data(&self) -> Vec<DataPoint> {
        self.data
            .iter()
            .filter(|d| self.time_range.contains(&d.date))
            .cloned()
            .collect()
    }
}
```

### Real-time Updates

```rust
struct LiveChart {
    data: Vec<DataPoint>,
    max_points: usize,
}

impl LiveChart {
    fn add_data_point(&mut self, point: DataPoint) {
        self.data.push(point);
        if self.data.len() > self.max_points {
            self.data.remove(0); // Remove oldest point
        }
    }

    fn render(&self, cx: &mut Context<Self>) -> impl IntoElement {
        LineChart::new(self.data.clone())
            .x(|d| d.timestamp.clone())
            .y(|d| d.value)
            .linear()
            .dot()
    }
}
```

[LineChart]: https://docs.rs/gpui-component/latest/gpui_component/chart/struct.LineChart.html
[BarChart]: https://docs.rs/gpui-component/latest/gpui_component/chart/struct.BarChart.html
[AreaChart]: https://docs.rs/gpui-component/latest/gpui_component/chart/struct.AreaChart.html
[PieChart]: https://docs.rs/gpui-component/latest/gpui_component/chart/struct.PieChart.html
[RadarChart]: https://docs.rs/gpui-component/latest/gpui_component/chart/struct.RadarChart.html
[CandlestickChart]: https://docs.rs/gpui-component/latest/gpui_component/chart/struct.CandlestickChart.html
