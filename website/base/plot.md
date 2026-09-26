---
title: Plot
description: Unstyled plotting in gpui-base — scales, shapes, axes, the Plot element, and hover tracking — for building charts in any design system.
order: 8
---

# Plot

`gpui_kit::base::plot` holds the parts of a chart that are behavior and geometry rather than design: scales that map data to pixels, shapes that tessellate bars, lines, areas and arcs, axes, grids and labels, the element that turns a [`Plot`] into a child, and the hover tracking behind an interactive tooltip.

It chooses no colors, fonts or timing. Every shape takes its fill and stroke from the caller, and hover motion is whatever the styled layer projects. The styled charts of GPUI Component — `LineChart`, `BarChart`, `PieChart` and the rest — are built on it, and a design system that does not use GPUI Component can build its own on the same foundation.

GPUI Component re-exports this module unchanged as `gpui_kit::component::plot`, so existing imports keep working.

## Import

```rust
use gpui_kit::base::plot::{
    AXIS_GAP, AxisText, Grid, Plot, PlotAxis, PlotElement, PlotHover, TooltipState,
    scale::{Scale, ScaleBand, ScaleLinear, ScaleOrdinal, ScalePoint},
    shape::{Arc, Area, Bar, Line, Pie, Stack},
};
```

## Write a plot

A plot implements [`Plot`]. `paint` receives the bounds the plot fills and draws into them with the window's painting API:

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

The plot becomes an element through [`PlotElement`], which fills its container, runs `prepaint`, `paint` and the hover pipeline, and paints the overlay the plot returns:

```rust
impl IntoElement for Sparkline {
    type Element = PlotElement<Self>;

    fn into_element(self) -> Self::Element {
        PlotElement::new(self)
    }
}

div().h_16().w_full().child(Sparkline { values, stroke })
```

GPUI Component's `#[derive(IntoPlot)]` writes that impl for you; without GPUI Component, write it by hand.

A plot sizes itself to its parent, so give the parent a definite height.

## Scales

Scales map a domain of data to a range of pixels, or of any other value.

| Scale | Domain | Use |
| --- | --- | --- |
| `ScaleLinear` | Continuous numbers | Value axes; the range can run top-down (`vec![height, 0.]`) |
| `ScaleBand` | Discrete categories | Bars: `band_width()` is each bar's width, `tick` its start |
| `ScalePoint` | Discrete categories | Line and area charts over categorical x values |
| `ScaleOrdinal` | Discrete categories | Mapping categories to colors |

`ScaleLinear` accepts `f64`, and `rust_decimal::Decimal` with the `decimal` feature.

## Shapes

`Bar`, `Line`, `Area` and `Arc` paint themselves given accessors that turn a datum into pixels; `Pie` lays arcs out from values, and `Stack` computes stacked series for stacked bars and areas. `RadialLine` and the Sankey layout (`Sankey`, `SankeyLink`) serve radar and flow charts. See [Plot in GPUI Component](../component/plot.md) for a tour of each shape.

Shapes that repaint every frame can keep their tessellated paths across frames with [`PathCaches`], keyed by the plot's id.

## Axes, grids and labels

`PlotAxis` draws an x and y axis line with `AxisText` labels, `Grid` draws horizontal and vertical grid lines, solid or dashed, and `PlotLabel` paints free text at plot coordinates. `AXIS_GAP` is the height the x-axis labels take below the plot. Colors and font sizes are arguments; the text face comes from the surrounding text style.

## Hover and tooltips

A plot opts into hover by returning an id from `Plot::id`. `PlotElement` then tracks the cursor each frame, occlusion-aware so an open popup above the plot clears it, and asks the plot three questions:

1. `tooltip_state` — map the cursor to a [`TooltipState`]: the hovered index, the crosshair point and the data dots, or `None`.
2. `hover` — receive the hovered [`PlotHover`] before painting. It lingers after the cursor leaves while `progress()` eases back to zero, so emphasis fades out over the last datum instead of vanishing. `is_entering()` is true on the first hovered frame.
3. `tooltip` — return the overlay: a crosshair, dots, a tooltip box. The overlay paints above the plot; a box that may overflow the plot should wrap itself in `deferred`.

Base draws no overlay of its own; the styled layer builds it. Inside `Plot::tooltip`, an overlay renders within the plot's element scope, so it can read the hover without being handed it:

| Function | Returns |
| --- | --- |
| `hover_progress(window, cx)` | How far the hover has faded in, `0..=1`; `1` outside a plot |
| `is_hover_entering(window, cx)` | Whether this is the first hovered frame |
| `pointer_spring(cx)` | The spring a pointer follows the hovered datum with |
| `PlotHover::glide(id, target, window, cx)` | A position following `target` on the pointer spring, adopted at once on the entering frame |

## Motion

Base installs no plot motion: by default the hover appears and disappears at once and pointers jump to each datum. The styled layer projects its timing through the Base theme:

```rust
use std::time::Duration;
use gpui_kit::base::{PlotMotion, PlotTheme, Spring, Theme, motion::Transition};

let motion = PlotMotion::default()
    .with_pointer(Spring::new(Duration::from_millis(120)).with_epsilon(0.1))
    .with_enter(Transition::new(Duration::from_millis(120)))
    .with_exit(Transition::new(Duration::from_millis(120)));
Theme::global_mut(cx).plot = PlotTheme::new().with_motion(motion);
```

GPUI Component projects its motion tokens here whenever its theme changes. Motion honors the operating system's reduced-motion preference, under which every value adopts its target at once.
