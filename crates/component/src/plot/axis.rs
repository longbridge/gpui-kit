use gpui::{
    App, Bounds, FontWeight, Hsla, PathBuilder, Pixels, Point, SharedString, TextAlign, Window,
    point, px,
};

use super::{
    label::PlotLabel, label::TEXT_GAP, label::TEXT_HEIGHT, label::TEXT_SIZE, label::Text,
    origin_point,
};

pub const AXIS_GAP: f32 = 18.;

/// Which side of an axis line the tick labels render on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AxisLabelSide {
    /// X-axis: labels below the line. Y-axis: labels right of the line. (Default.)
    #[default]
    End,
    /// X-axis: labels above the line. Y-axis: labels left of the line.
    Start,
}

pub struct AxisText {
    pub text: SharedString,
    pub tick: Pixels,
    pub color: Hsla,
    pub font_size: Pixels,
    pub align: TextAlign,
}

impl AxisText {
    pub fn new(text: impl Into<SharedString>, tick: impl Into<Pixels>, color: Hsla) -> Self {
        Self {
            text: text.into(),
            tick: tick.into(),
            color,
            font_size: TEXT_SIZE.into(),
            align: TextAlign::Left,
        }
    }

    pub fn font_size(mut self, font_size: impl Into<Pixels>) -> Self {
        self.font_size = font_size.into();
        self
    }

    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }
}

#[derive(Default)]
pub struct PlotAxis {
    x: Option<Pixels>,
    x_label: PlotLabel,
    x_axis: bool,
    x_label_side: AxisLabelSide,
    y: Option<Pixels>,
    y_label: PlotLabel,
    y_axis: bool,
    y_label_side: AxisLabelSide,
    stroke: Hsla,
}

impl PlotAxis {
    pub fn new() -> Self {
        Self {
            x_axis: true,
            ..Default::default()
        }
    }

    /// Set the x-axis of the Axis.
    pub fn x(mut self, x: impl Into<Pixels>) -> Self {
        self.x = Some(x.into());
        self
    }

    /// Show or hide the x-axis of the Axis.
    ///
    /// Default is true.
    pub fn x_axis(mut self, x_axis: bool) -> Self {
        self.x_axis = x_axis;
        self
    }

    /// Set the x-label of the Axis.
    pub fn x_label(mut self, label: impl IntoIterator<Item = AxisText>) -> Self {
        if let Some(x) = self.x {
            let side = self.x_label_side;
            self.x_label = label
                .into_iter()
                .map(|t| {
                    let y = match side {
                        AxisLabelSide::End => x + px(TEXT_GAP * 3.),
                        AxisLabelSide::Start => x - px(TEXT_GAP + TEXT_HEIGHT),
                    };
                    Text {
                        text: t.text,
                        origin: point(t.tick, y),
                        color: t.color,
                        font_size: t.font_size,
                        font_weight: FontWeight::NORMAL,
                        align: t.align,
                    }
                })
                .into();
        }
        self
    }

    /// Set which side of the x-axis line tick labels render on.
    pub fn x_label_side(mut self, side: AxisLabelSide) -> Self {
        self.x_label_side = side;
        self
    }

    /// Set the y-axis of the Axis.
    pub fn y(mut self, y: impl Into<Pixels>) -> Self {
        self.y = Some(y.into());
        self
    }

    /// Show or hide the y-axis of the Axis.
    ///
    /// Default is true.
    pub fn y_axis(mut self, y_axis: bool) -> Self {
        self.y_axis = y_axis;
        self
    }

    /// Set the y-label of the Axis.
    pub fn y_label(mut self, label: impl IntoIterator<Item = AxisText>) -> Self {
        if let Some(y) = self.y {
            let side = self.y_label_side;
            self.y_label = label
                .into_iter()
                .map(|t| {
                    let x = match side {
                        AxisLabelSide::End => y + px(TEXT_GAP),
                        AxisLabelSide::Start => y - px(TEXT_GAP),
                    };
                    Text {
                        text: t.text,
                        origin: point(x, t.tick - px(TEXT_SIZE / 2.)),
                        color: t.color,
                        font_size: t.font_size,
                        font_weight: FontWeight::NORMAL,
                        align: t.align,
                    }
                })
                .into();
        }
        self
    }

    /// Set which side of the y-axis line tick labels render on.
    pub fn y_label_side(mut self, side: AxisLabelSide) -> Self {
        self.y_label_side = side;
        self
    }

    /// Set the stroke color of the Axis.
    pub fn stroke(mut self, stroke: impl Into<Hsla>) -> Self {
        self.stroke = stroke.into();
        self
    }

    fn draw_axis(&self, start_point: Point<Pixels>, end_point: Point<Pixels>, window: &mut Window) {
        let mut builder = PathBuilder::stroke(px(1.));
        builder.move_to(start_point);
        builder.line_to(end_point);
        if let Ok(path) = builder.build() {
            window.paint_path(path, self.stroke);
        }
    }

    /// Paint the Axis.
    pub fn paint(&self, bounds: &Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let origin = bounds.origin;

        // X axis
        if let Some(x) = self.x {
            if self.x_axis {
                self.draw_axis(
                    origin_point(px(0.), x, origin),
                    origin_point(bounds.size.width, x, origin),
                    window,
                );
            }
        }
        self.x_label.paint(bounds, window, cx);

        // Y axis
        if let Some(y) = self.y {
            if self.y_axis {
                self.draw_axis(
                    origin_point(y, px(0.), origin),
                    origin_point(y, bounds.size.height, origin),
                    window,
                );
            }
        }
        self.y_label.paint(bounds, window, cx);
    }
}
