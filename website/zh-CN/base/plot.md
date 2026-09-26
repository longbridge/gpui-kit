---
title: Plot
description: gpui-base 中的无样式绘图能力：比例尺、图形、坐标轴、Plot element 与 hover 跟踪，可在任意设计系统中构建图表。
order: 8
---

# Plot

`gpui_kit::base::plot` 包含图表中属于行为和几何、而不属于设计的部分：把数据映射成像素的比例尺，把柱、线、面积和圆弧细分成路径的图形，坐标轴、网格和标签，把 [`Plot`] 变成子元素的 element，以及交互式 tooltip 背后的 hover 跟踪。

它不选择任何颜色、字体或动效时序。每个图形的填充和描边都由调用方传入，hover 动效则完全取决于样式层投射的配置。GPUI Component 的样式化图表，如 `LineChart`、`BarChart`、`PieChart` 等，都构建在它之上；不使用 GPUI Component 的设计系统也可以基于同一套基础实现自己的图表。

GPUI Component 以 `gpui_kit::component::plot` 原样重新导出这个模块，已有的导入路径无需修改。

## 导入

```rust
use gpui_kit::base::plot::{
    AXIS_GAP, AxisText, Grid, Plot, PlotAxis, PlotElement, PlotHover, TooltipState,
    scale::{Scale, ScaleBand, ScaleLinear, ScaleOrdinal, ScalePoint},
    shape::{Arc, Area, Bar, Line, Pie, Stack},
};
```

## 编写一个 Plot

Plot 实现 [`Plot`] trait。`paint` 接收 Plot 占据的区域，并用窗口的绘制 API 在其中作画：

```rust
use gpui_kit::base::plot::{Plot, PlotElement, scale::{Scale, ScaleLinear, ScalePoint}, shape::Line};
use gpui_kit::*;

struct Sparkline {
    values: Vec<f64>,
    stroke: Hsla,
}

impl Plot for Sparkline {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, _: &mut App) {
        let width = bounds.size.width.as_f32();
        let height = bounds.size.height.as_f32();
        let indexes: Vec<usize> = (0..self.values.len()).collect();
        let x = ScalePoint::new(indexes.clone(), vec![0., width]);
        let y = ScaleLinear::new(self.values.clone(), vec![height, 0.]);
        let values = self.values.clone();

        Line::new()
            .data(indexes)
            .x(move |i| x.tick(i))
            .y(move |i| y.tick(&values[*i]))
            .stroke(self.stroke)
            .stroke_width(1.5)
            .paint(&bounds, window);
    }
}
```

Plot 通过 [`PlotElement`] 成为 element。它会填满容器，依次执行 `prepaint`、`paint` 和 hover 流程，并绘制 Plot 返回的浮层：

```rust
impl IntoElement for Sparkline {
    type Element = PlotElement<Self>;

    fn into_element(self) -> Self::Element {
        PlotElement::new(self)
    }
}

div().h_16().w_full().child(Sparkline { values, stroke })
```

GPUI Component 的 `#[derive(IntoPlot)]` 会自动生成这段实现；不使用 GPUI Component 时手写即可。

Plot 的尺寸跟随父元素，因此需要给父元素一个明确的高度。

## 比例尺

比例尺把数据的定义域映射到像素或其他值的值域。

| 比例尺 | 定义域 | 用途 |
| --- | --- | --- |
| `ScaleLinear` | 连续数值 | 数值轴；值域可以自上而下（`vec![height, 0.]`） |
| `ScaleBand` | 离散类别 | 柱状图：`band_width()` 是每根柱子的宽度，`tick` 是起点 |
| `ScalePoint` | 离散类别 | 以类别为 x 值的折线图和面积图 |
| `ScaleOrdinal` | 离散类别 | 把类别映射到颜色 |

`ScaleLinear` 接受 `f64`，启用 `decimal` feature 后也接受 `rust_decimal::Decimal`。

## 图形

`Bar`、`Line`、`Area` 和 `Arc` 通过把数据项转换成像素的访问器完成绘制；`Pie` 根据数值计算圆弧布局，`Stack` 为堆叠柱状图和面积图计算堆叠序列。`RadialLine` 与 Sankey 布局（`Sankey`、`SankeyLink`）分别用于雷达图和流向图。各图形的用法见 [GPUI Component 中的 Plot](../component/plot.md)。

每帧都要重绘的图形可以用 [`PathCaches`] 按 Plot 的 id 在帧之间保留已细分的路径。

## 坐标轴、网格与标签

`PlotAxis` 绘制 x 轴与 y 轴线及 `AxisText` 标签，`Grid` 绘制实线或虚线的水平、垂直网格线，`PlotLabel` 在 Plot 坐标上绘制文字。`AXIS_GAP` 是 x 轴标签在 Plot 下方占用的高度。颜色和字号都是参数，字体取自外层的文字样式。

## Hover 与 tooltip

Plot 在 `Plot::id` 中返回 id 即可启用 hover。之后 `PlotElement` 每帧跟踪光标（会识别遮挡，Plot 上方打开的弹出层会清除 hover），并依次询问 Plot：

1. `tooltip_state`：把光标映射成 [`TooltipState`]，包括悬停的索引、十字线位置和数据点，或返回 `None`。
2. `hover`：在绘制前接收当前的 [`PlotHover`]。光标离开后它会继续保留，同时 `focus()` 逐渐回落到零，让强调效果在最后一个数据项上淡出，而不是突然消失。首个悬停帧上 `is_entering()` 为 true。
3. `tooltip`：返回浮层，例如十字线、数据点和提示框。浮层绘制在 Plot 之上；可能超出 Plot 边界的提示框应当用 `deferred` 包裹自身。

Base 本身不绘制任何浮层，浮层由样式层构建。在 `Plot::tooltip` 中，浮层渲染在 Plot 的 element 作用域内，因此无需传参即可读取 hover 状态：

| 函数 | 返回值 |
| --- | --- |
| `hover_focus(window, cx)` | hover 淡入的程度，范围 `0..=1`；在 Plot 之外为 `1` |
| `is_hover_entering(window, cx)` | 当前是否是首个悬停帧 |
| `pointer_spring(cx)` | 指示器跟随悬停数据项所用的弹簧 |
| `PlotHover::glide(id, target, window, cx)` | 在指示器弹簧上跟随 `target` 的位置，首个悬停帧直接采用目标值 |

## 动效

Base 不内置任何 Plot 动效：默认情况下 hover 会立即出现和消失，指示器直接跳到每个数据项。样式层通过 Base 主题投射自己的时序：

```rust
use std::time::Duration;
use gpui_kit::base::{PlotMotion, PlotTheme, Spring, Theme, motion::Transition};

let motion = PlotMotion::default()
    .with_pointer(Spring::new(Duration::from_millis(120)).with_epsilon(0.1))
    .with_enter(Transition::new(Duration::from_millis(120)))
    .with_exit(Transition::new(Duration::from_millis(120)));
Theme::global_mut(cx).plot = PlotTheme::new().with_motion(motion);
```

GPUI Component 会在主题变化时把自己的 motion token 投射到这里。动效遵循操作系统的“减少动态效果”设置，开启后所有值都会立即采用目标值。
