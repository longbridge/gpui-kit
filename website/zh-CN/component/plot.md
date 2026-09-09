---
title: Plot
description: 用于构建自定义图表和数据可视化的底层绘图库。
---

# Plot

`plot` 模块提供了构建自定义图表所需的底层能力，包括比例尺、图形和辅助工具。高层 `Chart` 组件也是基于这些原语实现的，适合需要完全控制图表绘制逻辑的场景。

## 导入

```rust
use gpui_kit::component::plot::{
    scale::{Scale, ScaleLinear, ScaleBand, ScalePoint, ScaleOrdinal},
    shape::{Bar, Stack, Line, Area, Pie, Arc},
    PlotAxis, AxisText
};
```

## 比例尺

比例尺用于把抽象数据映射成可视化坐标或样式。

### ScaleLinear

```rust
let scale = ScaleLinear::new(
    vec![0., 100.],
    vec![0., 500.]
);

scale.tick(&50.);
```

### ScaleBand

```rust
let scale = ScaleBand::new(
    vec!["A", "B", "C"],
    vec![0., 300.]
)
.padding_inner(0.1)
.padding_outer(0.1);

scale.band_width();
scale.tick(&"A");
```

### ScalePoint

```rust
let scale = ScalePoint::new(
    vec!["A", "B", "C"],
    vec![0., 300.]
);

scale.tick(&"A");
```

### ScaleOrdinal

```rust
let scale = ScaleOrdinal::new(
    vec!["A", "B", "C"],
    vec![color1, color2, color3]
);

scale.map(&"A");
```

## 图形

### Bar

```rust
Bar::new()
    .data(data)
    .band_width(30.)
    .x(|d| x_scale.tick(&d.category))
    .y0(|d| y_scale.tick(&0.).unwrap())
    .y1(|d| y_scale.tick(&d.value))
    .fill(|d| color_scale.map(&d.category))
    .paint(&bounds, window, cx);
```

### Line

```rust
Line::new()
    .data(data)
    .x(|d| x_scale.tick(&d.date))
    .y(|d| y_scale.tick(&d.value))
    .stroke(cx.theme().chart_1)
    .stroke_width(px(2.))
    .paint(&bounds, window);
```

#### 带节点的折线

```rust
Line::new()
    .data(data)
    .x(|d| x_scale.tick(&d.date))
    .y(|d| y_scale.tick(&d.value))
    .dot()
    .dot_size(px(4.))
    .paint(&bounds, window);
```

### Area

```rust
Area::new()
    .data(data)
    .x(|d| x_scale.tick(&d.date))
    .y0(height)
    .y1(|d| y_scale.tick(&d.value))
    .fill(cx.theme().chart_1.opacity(0.5))
    .stroke(cx.theme().chart_1)
    .paint(&bounds, window);
```

### Pie 与 Arc

```rust
let pie = Pie::new()
    .value(|d| Some(d.value))
    .pad_angle(0.05);

let arcs = pie.arcs(&data);

let arc_shape = Arc::new()
    .inner_radius(0.)
    .outer_radius(100.);

for arc_data in arcs {
    arc_shape.paint(
        &arc_data,
        color_scale.map(&arc_data.data.category),
        None,
        None,
        &bounds,
        window
    );
}
```

### Stack

```rust
let stack = Stack::new()
    .data(data)
    .keys(vec!["series1", "series2"])
    .value(|d, key| match key {
        "series1" => Some(d.val1),
        "series2" => Some(d.val2),
        _ => None
    });

let series = stack.series();
```

## 组件

### PlotAxis

```rust
PlotAxis::new()
    .x(height)
    .x_label(labels)
    .stroke(cx.theme().border)
    .paint(&bounds, window, cx);
```

## 示例

### 自定义堆叠柱状图

```rust
struct StackedBarChart {
    data: Vec<DailyDevice>,
    series: Vec<StackSeries<DailyDevice>>,
}

impl StackedBarChart {
    pub fn new(data: Vec<DailyDevice>) -> Self {
        let series = Stack::new()
            .data(data.clone())
            .keys(vec!["desktop", "mobile"])
            .value(|d, key| match key {
                "desktop" => Some(d.desktop),
                "mobile" => Some(d.mobile),
                _ => None,
            })
            .series();

        Self { data, series }
    }
}

impl Plot for StackedBarChart {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        // 1. 准备比例尺
        let x = ScaleBand::new(
            self.data.iter().map(|v| v.date.clone()).collect(),
            vec![0., width],
        );

        let y = ScaleLinear::new(vec![0., max_value], vec![height, 0.]);

        // 2. 绘制坐标轴
        // ...（坐标轴绘制逻辑）

        // 3. 绘制堆叠柱
        let bar = Bar::new()
            .stack_data(&self.series)
            .band_width(x.band_width())
            .x(move |d| x.tick(&d.data.date))
            .fill(move |_| cx.theme().chart_1);

        bar.paint(&bounds, window, cx);
    }
}
