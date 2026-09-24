---
title: Chart
description: 支持折线图、柱状图、面积图、饼图、雷达图、K 线图和桑基图的数据可视化组件。
---

# Chart

Chart 是一组完整的数据可视化组件，提供 Line、Bar、Area、Pie、Radar、Candlestick 和 Sankey 图表。它们支持动画、自定义样式、主题配色和多种展示方式，适合仪表盘、统计分析和行情场景。

## 导入

```rust
use gpui_kit::component::chart::{
    LineChart, BarChart, AreaChart, PieChart, RadarChart, CandlestickChart, SankeyChart,
};
```

## 图表类型

### LineChart

折线图用于展示随时间变化的趋势。

#### 基础折线图

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

#### 折线图变体

```rust
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)

LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .linear()

LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .step_after()

LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .dot()

LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .stroke(cx.theme().success)
```

#### 刻度控制

```rust
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .tick_margin(1)

LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .tick_margin(2)
```

`LineChart` 同样支持 `y_domain` 和 `point_count`，用法见 AreaChart 下的「固定 Y 轴与未完成的序列」。

#### 坐标轴与辅助线

以下选项 `LineChart` 和 `AreaChart` 通用。`y_axis` 在 Y 轴刻度处显示标签，默认放在绘图区左侧的标签栏里，标签栏会按最宽的标签自动加宽，用 `y_axis_label_placement(AxisLabelPlacement::Inside)` 可以改为叠在绘图区内。`y_tick_count` 设置刻度数，刻度从基线到顶边均匀分布，两端都算在内；横向网格线也画在这些刻度上，每个标签显示比例尺在该高度对应的数值。默认的 5 个刻度就是图表一直以来的网格。`y_tick_format` 根据这个数值生成标签文字。

`x_tick_count` 只给这么多个 X 值标注，从第一个到最后一个均匀挑选，不再按 `tick_margin` 每隔几个标一个；设置了 `point_count` 时按轴上的全部点位挑选，数据增长时标签位置不变。`grid_columns` 增加纵向网格线，`grid_dashed(false)` 把网格改为实线，`reference_line` 在某个数值处画一条贯穿绘图区的虚线，颜色比网格深，`y_padding` 设置最大值上方和最小值下方保留的空白，默认上方 10px、下方 0。

```rust
use gpui_kit::component::plot::AxisLabelPlacement;

// 分时图：标签叠在图内、实线网格、标出昨收
AreaChart::new(minutes)
    .x(|d| d.time.clone())
    .y(|d| d.price)
    .y_domain(low, high)
    .y_axis(true)
    .y_axis_label_placement(AxisLabelPlacement::Inside)
    .y_tick_count(3)
    .y_tick_format(|v| format!("{v:.2}"))
    .x_tick_count(3)
    .grid_columns(4)
    .grid_dashed(false)
    .reference_line(prev_close)
    .y_padding(6., 6.)
```

### BarChart

柱状图通过矩形条形对比不同类别的数据，并可通过 `alignment` 选项切换垂直或水平方向。

#### 基础柱状图

```rust
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
```

#### 自定义柱状图

```rust
// 自定义填充颜色
//
// `fill` 闭包接收四个参数：数据项、柱子的像素边界（相对于图表原点）、
// 图表的像素边界，以及柱子的 `BarAlignment`。返回值可以是任何能转换为
// `Background` 的类型（纯色、渐变、图案等）。
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .fill(|d, _bar_bounds, _chart_bounds, _alignment| d.color)

// 显示数值标签
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .label(|d| format!("{}", d.value))

// 自定义刻度间距
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .tick_margin(2)

// 隐藏分类轴的轴线和标签
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .label_axis(false)
```

#### 柱状图渐变填充

如需让渐变方向跟随柱子方向，请使用 `fill_gradient`。闭包接收三个参数：数据项、图表的完整数据范围（`chart_range`），以及一个 `chart_to_bar` 辅助函数（将图表数值坐标映射为柱子局部的渐变位置，其中 `0.0` 表示柱子的基线端，`1.0` 表示尖端）。渐变方向由柱子的 `BarAlignment` 推导，使 stop-0 始终位于基线端、stop-1 位于尖端。

```rust
use gpui_kit::linear_color_stop;

// 单柱渐变：每个柱子都从半透明基线渐变到完全不透明的尖端，
// 与该柱子的具体数值无关。
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

// 跨图表渐变：每根柱子展示同一条覆盖整个图表数值范围的渐变中
// 对应自身值域的那一段。超出 `[0, 1]` 的 stop 会被裁剪到柱子内，
// 颜色会在裁剪点处插值，使整体效果保持连续。
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

`fill` 与 `fill_gradient` 互斥——设置其中一个会清空另一个。

#### 柱状图对齐方式

`BarAlignment` 用于控制柱子的方向以及基线所在的一侧，需从 `gpui_kit::component::plot::shape` 导入。

```rust
use gpui_kit::component::plot::shape::BarAlignment;

// 默认：垂直方向 - 向上
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .alignment(BarAlignment::Bottom)

// 垂直方向 - 向下
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .alignment(BarAlignment::Top)

// 水平方向 - 向右
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .alignment(BarAlignment::Left)

// 水平方向 - 向左
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .alignment(BarAlignment::Right)
```

#### 柱状图圆角

为柱状条形设置圆角。可传入任意可转换为 `Corners<Pixels>` 的值——
使用单个 `px(..)` 表示四角统一圆角，或手动构造 `Corners`
仅对特定角进行圆角处理（例如仅对柱顶一端进行圆角）。

```rust
use gpui_kit::{px, Corners};

// 所有柱条统一 4px 圆角
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .corner_radii(px(4.))

// 仅顶部圆角（适用于底部对齐柱状图的柱顶一端）
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

#### 柱状图负值

柱条从零点而非绘图区边缘开始生长，因此负值会向零线的另一侧延伸。
分类轴线跟随零点位置，每个分类标签也会移动到自身柱条未占用的那一侧。
无需任何配置——数据中包含负值时即以此方式渲染。

```rust
// `growth` 可为负值；零线以下的柱条向下绘制
BarChart::new(data)
    .band(|d| d.quarter.clone())
    .value(|d| d.growth)
    .label(|d| format!("{:+.0}%", d.growth))
```

#### 柱状图数值轴

使用 `value_axis` 显示数值刻度标签，并通过 `value_tick_count` 设置数值轴上的刻度数。
刻度从基线到远端均匀分布，两端都算在内，网格线和刻度标签都由它决定，两者始终一致。
`tick_margin` 则是分类轴上的步长：`tick_margin(2)` 表示每隔一个分类保留一个标签。

```rust
// 纵向柱状图的数值标签位于左侧，横向柱状图位于下方
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .value_axis(true)

// 7 个刻度（默认为 5 个）
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .value_axis(true)
    .value_tick_count(7)
```

`value_axis_label_placement(AxisLabelPlacement::Inside)` 把标签叠在绘图区内、紧挨各自的网格线，柱子不必让出标签栏的空间；`value_tick_format` 生成标签文字。`band_count` 让分类轴按比数据更多的格位数铺开，数据较少时每根柱保持原有宽度，只占前面几格。`band_tick_count` 只给这么多个分类标注，从第一个到最后一个均匀挑选（设置了 `band_count` 时按全部格位挑选）；`grid_dashed(false)` 把网格改为实线。

```rust
use gpui_kit::component::plot::AxisLabelPlacement;

// 最近 20 天每天一个值，不管目前有几天数据
BarChart::new(days)
    .band(|d| d.date.clone())
    .value(|d| d.value)
    .value_axis(true)
    .value_axis_label_placement(AxisLabelPlacement::Inside)
    .value_tick_count(2)
    .value_tick_format(|v| format!("{v:.2}"))
    .band_count(20)
    .band_tick_count(2)
    .grid_dashed(false)
```

#### 柱状图标签与间距

`label_color` 给每根柱的 `label` 文字单独配色，数值可以跟随柱子的颜色，不必统一用前景色。`padding_inner` 和 `padding_outer` 分别设置柱子之间、首尾两端的间距，以占一个分类宽度的比例计，默认是 0.4 和 0.2。`min_length` 让每根柱子至少画这么多像素长，数量为 0 的分档也能在基线上留一截柱桩。

```rust
// 分布图：细柱、数值跟随柱色、0 值留柱桩
BarChart::new(buckets)
    .band(|d| d.range.clone())
    .value(|d| d.count)
    .fill(|d, _, _, _| d.color)
    .label(|d| d.count.to_string())
    .label_color(|d| d.color)
    .padding_inner(0.6)
    .min_length(2.)
```

柱桩朝柱子本该生长的方向延伸：从零线向外，负值朝负方向，零值朝正方向。带 `label` 的纵向柱状图会在最高的柱子上方留出一行文字的高度，保证它的标签不超出图表。

### AreaChart

面积图类似折线图，但会填充曲线下方的区域。

#### 基础面积图

```rust
AreaChart::new(data)
    .x(|d| d.time.clone())
    .y(|d| d.value)
```

#### 多系列面积图

```rust
AreaChart::new(data)
    .x(|d| d.date.clone())
    .y(|d| d.desktop)
    .stroke(cx.theme().chart_1)
    .fill(cx.theme().chart_1.opacity(0.4))
    .y(|d| d.mobile)
    .stroke(cx.theme().chart_2)
    .fill(cx.theme().chart_2.opacity(0.4))
```

#### 样式

```rust
use gpui_kit::{linear_gradient, linear_color_stop};

AreaChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .fill(linear_gradient(
        0.,
        linear_color_stop(cx.theme().chart_1.opacity(0.4), 1.),
        linear_color_stop(cx.theme().background.opacity(0.3), 0.),
    ))

AreaChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .linear()
```

#### 固定 Y 轴与未完成的序列

Y 轴默认从 0 开始拟合数据。`y_domain` 把它固定在给定区间，价格、资产这类离 0 很远的数值就不会被压成顶部的一条线。`point_count` 让 X 轴按比数据更多的点数排布，尚未完成的序列（比如当天的分时）只占前面一段。`LineChart` 同样支持这两个方法。

```rust
// 分时缩略图：美股一个交易日 390 个分钟点位。
AreaChart::new(minutes)
    .x(|d| d.time.clone())
    .y(|d| d.price)
    .linear()
    .y_domain(low, high)
    .point_count(390)
    .x_axis(false)
    .grid(false)
    .interactive(false)
```

固定区间和默认一样，在最大值上方留出 10px；序列会被裁剪在绘图区内，超出区间的值止于边缘。`min` 与 `max` 相等时什么都不画，数值全相同的序列需要先自行放宽区间。平滑曲线（natural）会在最高点和最低点附近冲过头，区间贴着数据取值时建议用 `linear`。

第 i 条数据固定落在第 i 个点位，所以数据必须从第一个点位开始连续，中间缺一条会让后面的数据都向左错一位。

### PieChart

饼图适合展示占比关系。

#### 基础饼图

```rust
PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
```

#### 环形图

```rust
PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
    .inner_radius(60.)
```

#### 自定义

```rust
PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
    .color(|d| d.color)

PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
    .inner_radius(60.)
    .pad_angle(4. / 100.)
```

### RadarChart

雷达图以围绕中心的闭合多边形展示多维数据，适合对比多个系列在各维度上的表现。

#### 基础雷达图

```rust
RadarChart::new(data)
    .label(|d| d.month.clone())
    .value(|d| d.desktop)
```

#### 多系列

```rust
// 每次调用 `.value()` 新增一个系列，与随后的 `.stroke()` / `.fill()`
// 一一配对。颜色默认按主题图表色循环取用。
RadarChart::new(data)
    .label(|d| d.month.clone())
    .value(|d| d.desktop)
    .stroke(cx.theme().chart_1)
    .value(|d| d.mobile)
    .stroke(cx.theme().chart_2)
```

#### 元素标签

`label` 既接受字符串，也接受自定义元素。返回 `element.into_any_element()`
即可在外圈周围渲染任意内容——图标、多行、按维度换色都可以。

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

每个标签按自然尺寸测量，并沿径向朝外推开，所以即使很高也不会压到外圈上。
元素标签自带样式，因此 `.label_color()` 对它无效，也不会提供 tooltip 标题（字符串标签会）。

外圈不会为标签自动让位：默认外圈半径是图表高度的 40%，所以标签比单行文字高很多时，
需要调小 `.outer_radius()` 才能让它留在图表范围内。

#### 自定义

```rust
// 顶点圆点与自定义填充
RadarChart::new(data)
    .label(|d| d.month.clone())
    .value(|d| d.desktop)
    .stroke(cx.theme().chart_2)
    .fill(cx.theme().chart_2.opacity(0.2))
    .dot()

// 固定外圈最大值与网格环数
RadarChart::new(data)
    .label(|d| d.month.clone())
    .value(|d| d.desktop)
    .max_value(400.)
    .grid_levels(5)
    .outer_radius(120.)
```

### CandlestickChart

K 线图适合展示金融行情中的 OHLC 数据。

#### 基础 K 线图

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

#### 自定义

```rust
CandlestickChart::new(data)
    .x(|d| d.date.clone())
    .open(|d| d.open)
    .high(|d| d.high)
    .low(|d| d.low)
    .close(|d| d.close)
    .body_width_ratio(0.4)

CandlestickChart::new(data)
    .x(|d| d.date.clone())
    .open(|d| d.open)
    .high(|d| d.high)
    .low(|d| d.low)
    .close(|d| d.close)
    .tick_margin(2)
```

收盘高于开盘的 K 线使用主题的 `chart.bullish` 色，收盘不高于开盘的使用 `chart.bearish` 色。红涨绿跌的市场把两者对调即可：

```rust
CandlestickChart::new(data)
    .x(|d| d.date.clone())
    .open(|d| d.open)
    .high(|d| d.high)
    .low(|d| d.low)
    .close(|d| d.close)
    .bullish(cx.theme().danger)
    .bearish(cx.theme().success)
```

### SankeyChart

桑基图用于展示节点之间的流量关系，适合财报资金流向、能源流动和流量分析等场景。布局算法对标 [d3-sankey](https://github.com/d3/d3-sankey)。

#### 基础桑基图

```rust
use gpui_kit::component::plot::shape::SankeyLink;

#[derive(Clone)]
struct FlowNode {
    pub name: SharedString,
}

let nodes = vec![
    FlowNode { name: "营业收入".into() },
    FlowNode { name: "毛利润".into() },
    FlowNode { name: "营业成本".into() },
];

// 连接通过节点在 `nodes` 中的索引引用节点。
let links = vec![
    SankeyLink::new(0, 1, 45.0),
    SankeyLink::new(0, 2, 55.0),
];

SankeyChart::new(nodes, links)
    .node_label(|d| d.name.clone())
    .value_label(|_, value| format!("{:.1}", value).into())
```

数值标签显示在名称标签上方，闭包会收到节点的吞吐量（进出流量的较大值）。

#### 节点对齐

```rust
use gpui_kit::component::plot::shape::SankeyAlign;

// Justify（默认）：没有出边的节点移到最后一列
SankeyChart::new(nodes, links).node_align(SankeyAlign::Justify)

// Left：节点保持在自己的拓扑深度列
SankeyChart::new(nodes, links).node_align(SankeyAlign::Left)

// 还支持：SankeyAlign::Right、SankeyAlign::Center
```

#### 样式

```rust
SankeyChart::new(nodes, links)
    .node_width(8.)             // 节点条宽度（默认 10）
    .node_padding(20.)          // 同列节点垂直间距（默认 16）
    .node_corner_radius(px(2.)) // 节点条圆角（默认 0）
    .node_color(|d| d.color)    // 每个节点的颜色，默认循环主题图表配色
    .link_opacity(0.4)          // 连接带透明度（默认 0.3）
    .min_link_width(2.)         // 连接带最小粗细（默认 1）
    .iterations(10)             // 布局松弛迭代次数（默认 6）
```

连接带使用从源节点颜色到目标节点颜色的水平渐变填充。

#### 自定义标签

需要完全控制标签行时使用 `labels`——每行一个 `SankeyLabel`，从上到下排列，每行可单独设置颜色和字号。设置后优先于 `node_label`/`value_label`。例如带同比涨跌幅行的财报标签：

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

行颜色默认为主题前景色，字号默认 10；摆位、对齐和边距预留仍由组件负责。首/末列标签若超出预留边距，会被截断并加省略号，而不会画到图表外；若想让长标签完整分多行显示，请自行折断或缩短。

#### 压缩数值跨度

节点高度默认与流量值成线性关系，数值跨度很大时（如 200:1）小流量几乎不可见、主流量过大。设置 `value_scale(SankeyValueScale::Sqrt)` 即可压缩跨度——组件按值的平方根来定节点高度，小流量保持可见，且无需预处理数据，标签仍显示真实值：

```rust
use gpui_kit::component::plot::shape::SankeyValueScale;

SankeyChart::new(nodes, links).value_scale(SankeyValueScale::Sqrt)
```

无论用哪种缩放，每个节点都被其连接精确填满，所以子节点高度始终与父节点匹配。

## 悬停与 Tooltip

图表默认就会对光标做命中测试，为光标所在的数据显示 tooltip，并按图表类型强调这条数据，无需额外开启：

```rust
LineChart::new(data)
    .x(|d| d.date.clone())
    .y(|d| d.value)
    .name("Desktop") // tooltip 行中的系列名
```

| 图表 | 悬停时 |
| --- | --- |
| `LineChart`、`AreaChart` | 十字线和每个系列的圆点沿折线滑到悬停的数据点，圆点外扩出一圈光晕。 |
| `BarChart` | 与柱同宽的高亮条滑到悬停的柱，其余柱淡出到它后面。 |
| `PieChart` | 悬停的扇区从圆环中抬起，其余扇区淡出；tooltip 显示数值与占比，设了 `tooltip_value` 则用它。 |
| `RadarChart` | 每个系列的圆点沿多边形滑到悬停的辐条。 |
| `CandlestickChart` | 高亮条滑到悬停的 K 线；tooltip 列出开盘、最高、最低、收盘。 |
| `SankeyChart` | 悬停节点的连接保持颜色，其余淡出；tooltip 显示节点的名称与流量，设了 `tooltip_name` / `tooltip_value` 则用它们。 |

tooltip 框跟随光标，靠近边缘时翻向绘图区中心。`AreaChart` 与 `RadarChart` 每个系列各取一个 `.name()`，在对应的 `.y()` / `.value()` 之后调用。

tooltip 的一行由色块、名称、数值三部分组成。`PieChart::tooltip_name` 与 `SankeyChart::tooltip_name` 用光标所在的数据项来填这个名称——扇区名、节点名——对于每个数据项只有一个数字的图表，这正是那一行想要的。不设时，饼图回落到 `name`（整个系列共用的那一个名字），桑基图的行则完全没有名称：只剩一个色块和一个数字，中间空着。

`name` 顶替不了它。它说的是这些数字在计量什么，对每个扇区都一样，因此永远说不出 tooltip 讲的是哪一块。把 `label` 拿来做标题也不行——`label` 会同时在圆环外画引线标签，桑基图的 `node_label` 同理会把名字写在节点旁边。

`PieChart::tooltip_value` 用来替换它那一行的文本（默认写的是原始数值加占比）。只要原始数值不是该给用户看的东西就应该设它——本身已是比例的数值默认会显示成 `0.35 (35.0%)`；而用调整过的值绘制的图表（例如为了让极小扇区可见而设的下限）会把调整后的数字当作真实数据报出来：

```rust
PieChart::new(holdings)
    .value(|d| d.ratio.max(MIN_VISIBLE))     // 按下限绘制
    .tooltip_name(|d| d.name.clone())        // 不画引线也能命名
    .tooltip_value(|d, _, _| pct(d.ratio))   // 按真实值显示
```

### 标识

这些行为都以 `ElementId` 为键，图表默认取自己的构造位置作为 id——只写出一次的图表因此天然唯一，绝大多数图表都是这种情况。若同一处构造被渲染成多个同级图表，需要分别命名，否则它们会共用同一份悬停状态与缓存：

```rust
shares.iter().enumerate().map(|(i, share)| PieChart::new(share.clone()).id(("share", i)))
```

`GlobalElementId` 是整条 id 栈，因此本身已带 id 的同级元素（例如 `List`、`uniform_list` 绘制的行）会自动把其下的图表区分开，无需额外处理。

### 关闭交互

`interactive(false)` 把整层连同 hitbox 一起去掉，语义等同 Highcharts 的 `enableMouseTracking` 或 ECharts 的 `silent`。两种场景需要它：纯装饰的图表，以及上面盖着别的元素的图表：

```rust
AreaChart::new(placeholder).interactive(false) // 骨架屏、缩略图
AreaChart::new(range).interactive(false)       // 拖拽手柄下面的底图
```

第二种尤其要注意：普通 hitbox **不会挡住它后面的 hitbox**，盖在图表上的元素被悬停时，图表**同样**算被悬停，十字线会在它下面继续跟着跑。只能让图表让位。

### 动效

强调效果使用样式层的 motion tokens（`cx.theme().motion_tokens()`）驱动：十字线、高亮条、圆点等指示器以快速弹簧跟随悬停的数据，饼图扇区以 control 弹簧抬起，整个覆盖层在光标落到数据上时淡入、离开后淡出。动效遵循操作系统的减弱动态效果偏好，开启后所有值立即到达目标。

### 缓存

图表还会跨帧保留较重的几何计算，因为它在屏幕上的每一帧都会重绘：折线与面积的描边、饼图扇区在投影点不变时保持已细分的路径，桑基图在数据、设置和尺寸不变时保留布局。这份缓存挂在同一个 id 上，因此共用 id 的图表会互相冲刷缓存——这是同级图表需要分别命名的另一个理由；而 `interactive(false)` 的图表没有自己的 id，每次绘制都会重算几何。

### 自定义 Plot

自定义 [`Plot`] 需要手动接入——那里的 `Plot::id` 仍默认返回 `None`：在 `Plot::id` 返回 id，在 `Plot::tooltip_state` 解析光标所在的数据，在 `Plot::tooltip` 构建覆盖层。这里返回的 `Tooltip` 会自己为悬停加动画，和内置图表一样：整个覆盖层随悬停淡入淡出；十字线和圆点按指针 spring 滑到每个悬停的数据点，光标落下的那一帧直接就位；圆点的 `halo` 随悬停淡入逐渐放大。十字线只沿它标记的那条轴滑动，所以同时跟随光标的那条线不会滞后。传入数据点本身即可，其余交给 tooltip：

```rust
fn tooltip(&self, state: &TooltipState, cursor: Point<Pixels>, bounds: Bounds<Pixels>, _: &mut Window, cx: &mut App) -> Option<AnyElement> {
    Some(
        Tooltip::new(cursor, bounds.size)
            .cross_line(CrossLine::new(state.cross_line).band(px(24.)))
            .title("Title")
            .row(cx.theme().chart_1, "Series", "42")
            .into_any_element(),
    )
}
```

如果还要强调 plot 自己的图形——让悬停柱子周围的柱子变淡、让扇区弹出——就实现 `Plot::hover`。它在每帧的 `tooltip` 与 `paint` 之前运行，收到当前聚焦的 [`PlotHover`]；它携带 `TooltipState`，光标离开后会保留一段时间，`hover.focus()` 逐渐回到零，因此在这里采样动效并把结果存到 `self`。`hover.glide` 让一个位置按 tooltip 所用的同一个 spring 移动；把结果交给十字线，并用 `Tooltip::glide(false)` 关掉 tooltip 自己的滑动，避免重复做 spring：

```rust
fn hover(&mut self, hover: Option<&PlotHover>, window: &mut Window, cx: &mut App) {
    self.band_center =
        hover.map(|hover| hover.glide(("my-plot", "band"), hover.state().cross_line.x, window, cx));
}
```

## 数据结构示例

```rust
#[derive(Clone)]
struct DailyDevice {
    pub date: String,
    pub desktop: f64,
    pub mobile: f64,
}

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

#[derive(Clone)]
struct StockPrice {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}

// 桑基图连接：通过索引引用节点（来自 gpui_kit::component::plot::shape）
pub struct SankeyLink {
    pub source: usize,
    pub target: usize,
    pub value: f64,
}
```

## 图表配置

### 容器布局

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

### 主题集成

```rust
let chart = LineChart::new(data)
    .x(|d| d.date.clone())
    .y(|d| d.value)
    .stroke(cx.theme().chart_1);
```

可用主题色为 `cx.theme().chart_1` 到 `cx.theme().chart_5`（主题文件中的 `chart.1` 到 `chart.5`）。

## API 参考

- [LineChart]
- [BarChart]
- [AreaChart]
- [PieChart]
- [RadarChart]
- [CandlestickChart]
- [SankeyChart]

## 示例

### 销售仪表盘

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

### 多系列时间图

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

### 金融图表

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

## 自定义选项

### 配色

```rust
LineChart::new(data)
    .x(|d| d.x.clone())
    .y(|d| d.y)
    .stroke(cx.theme().chart_1)

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

### 响应式容器

```rust
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

### 默认样式

图表默认会自动包含：

- 虚线网格
- 自动定位的 X 轴标签
- 从 0 开始的 Y 轴刻度
- 基于 `tick_margin` 的刻度稀疏控制

## 性能建议

### 大数据集

```rust
let sampled_data: Vec<_> = data
    .iter()
    .step_by(5)
    .cloned()
    .collect();

LineChart::new(sampled_data)
    .x(|d| d.date.clone())
    .y(|d| d.value)
    .tick_margin(3)
```

### 内存优化

```rust
LineChart::new(data)
    .x(|d| d.date.clone())
    .y(|d| d.value)
```

## 集成示例

### 结合状态管理

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

### 实时更新

```rust
struct LiveChart {
    data: Vec<DataPoint>,
    max_points: usize,
}

impl LiveChart {
    fn add_data_point(&mut self, point: DataPoint) {
        self.data.push(point);
        if self.data.len() > self.max_points {
            self.data.remove(0);
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
