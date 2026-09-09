// @reference: https://d3js.org/d3-shape/line

use gpui::{
    Background, BorderStyle, Bounds, Hsla, PaintQuad, Path, PathBuilder, Pixels, Point, Window, px,
    quad, size,
};

use crate::plot::{StrokeStyle, origin_point};

#[allow(clippy::type_complexity)]
pub struct Line<T> {
    data: Vec<T>,
    x: Box<dyn Fn(&T) -> Option<f32>>,
    y: Box<dyn Fn(&T) -> Option<f32>>,
    stroke: Background,
    stroke_width: Pixels,
    stroke_style: StrokeStyle,
    dot: bool,
    dot_size: Pixels,
    dot_fill_color: Hsla,
    dot_stroke_color: Option<Hsla>,
}

impl<T> Default for Line<T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            x: Box::new(|_| None),
            y: Box::new(|_| None),
            stroke: Default::default(),
            stroke_width: px(1.),
            stroke_style: Default::default(),
            dot: false,
            dot_size: px(4.),
            dot_fill_color: gpui::transparent_black(),
            dot_stroke_color: None,
        }
    }
}

impl<T> Line<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the data of the Line.
    pub fn data<I>(mut self, data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        self.data = data.into_iter().collect();
        self
    }

    /// Set the x of the Line.
    pub fn x<F>(mut self, x: F) -> Self
    where
        F: Fn(&T) -> Option<f32> + 'static,
    {
        self.x = Box::new(x);
        self
    }

    /// Set the y of the Line.
    pub fn y<F>(mut self, y: F) -> Self
    where
        F: Fn(&T) -> Option<f32> + 'static,
    {
        self.y = Box::new(y);
        self
    }

    /// Set the stroke color of the Line.
    pub fn stroke(mut self, stroke: impl Into<Background>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Set the stroke width of the Line.
    pub fn stroke_width(mut self, stroke_width: impl Into<Pixels>) -> Self {
        self.stroke_width = stroke_width.into();
        self
    }

    /// Set the stroke style of the Line.
    pub fn stroke_style(mut self, stroke_style: StrokeStyle) -> Self {
        self.stroke_style = stroke_style;
        self
    }

    /// Show dots on the Line.
    pub fn dot(mut self) -> Self {
        self.dot = true;
        self
    }

    /// Set the size of the dots on the Line.
    pub fn dot_size(mut self, dot_size: impl Into<Pixels>) -> Self {
        self.dot_size = dot_size.into();
        self
    }

    /// Set the fill color of the dots on the Line.
    pub fn dot_fill_color(mut self, dot_fill_color: impl Into<Hsla>) -> Self {
        self.dot_fill_color = dot_fill_color.into();
        self
    }

    /// Set the stroke color of the dots on the Line.
    pub fn dot_stroke_color(mut self, dot_stroke_color: impl Into<Hsla>) -> Self {
        self.dot_stroke_color = Some(dot_stroke_color.into());
        self
    }

    /// Paint the dots on the Line.
    fn paint_dot(&self, dot: Point<Pixels>) -> PaintQuad {
        quad(
            gpui::bounds(dot, size(self.dot_size, self.dot_size)),
            self.dot_size / 2.,
            self.dot_fill_color,
            px(1.),
            self.dot_stroke_color.unwrap_or(self.dot_fill_color),
            BorderStyle::default(),
        )
    }

    fn path(&self, bounds: &Bounds<Pixels>) -> (Option<Path<Pixels>>, Vec<PaintQuad>) {
        let origin = bounds.origin;
        let mut builder = PathBuilder::stroke(self.stroke_width);
        let mut dots = vec![];
        let mut paint_dots = vec![];

        for v in self.data.iter() {
            let x_tick = (self.x)(v);
            let y_tick = (self.y)(v);

            if let (Some(x), Some(y)) = (x_tick, y_tick) {
                let pos = origin_point(px(x), px(y), origin);

                if self.dot {
                    let dot_radius = self.dot_size.as_f32() / 2.;
                    let dot_pos = origin_point(px(x - dot_radius), px(y - dot_radius), origin);
                    paint_dots.push(self.paint_dot(dot_pos));
                }

                dots.push(pos);
            }
        }

        if dots.is_empty() {
            return (None, paint_dots);
        }

        if dots.len() == 1 {
            builder.move_to(dots[0]);
            return (builder.build().ok(), paint_dots);
        }

        match self.stroke_style {
            StrokeStyle::Natural => {
                builder.move_to(dots[0]);
                let n = dots.len();
                for i in 0..n - 1 {
                    let p0 = if i == 0 { dots[0] } else { dots[i - 1] };
                    let p1 = dots[i];
                    let p2 = dots[i + 1];
                    let p3 = if i + 2 < n { dots[i + 2] } else { dots[n - 1] };

                    // Catmull-Rom to Bezier
                    let c1 = Point::new(p1.x + (p2.x - p0.x) / 6.0, p1.y + (p2.y - p0.y) / 6.0);
                    let c2 = Point::new(p2.x - (p3.x - p1.x) / 6.0, p2.y - (p3.y - p1.y) / 6.0);

                    builder.cubic_bezier_to(p2, c1, c2);
                }
            }
            StrokeStyle::Linear => {
                builder.move_to(dots[0]);
                for p in &dots[1..] {
                    builder.line_to(*p);
                }
            }
            StrokeStyle::StepAfter => {
                builder.move_to(dots[0]);
                for (i, p) in dots.windows(2).enumerate() {
                    builder.line_to(Point::new(p[1].x, p[0].y));
                    // Don't draw the vertical line for the last point
                    if i < dots.len() - 2 {
                        builder.line_to(p[1]);
                    }
                }
            }
        }

        (builder.build().ok(), paint_dots)
    }

    /// Paint the Line.
    pub fn paint(&self, bounds: &Bounds<Pixels>, window: &mut Window) {
        let (path, dots) = self.path(bounds);
        if let Some(path) = path {
            window.paint_path(path, self.stroke);
        }
        for dot in dots {
            window.paint_quad(dot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use gpui::{Bounds, point, px};

    #[test]
    fn test_line_path() {
        let data = vec![1., 2., 3.];
        let line = Line::new()
            .data(data.clone())
            .x(|v| Some(*v))
            .y(|v| Some(*v * 2.));

        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(100.), px(100.)));
        let (path, dots) = line.path(&bounds);

        assert!(path.is_some());
        assert!(dots.is_empty());

        let line_with_dots = Line::new()
            .data(data)
            .x(|v| Some(*v))
            .y(|v| Some(*v * 2.))
            .dot();

        let (_, dots) = line_with_dots.path(&bounds);
        assert_eq!(dots.len(), 3);
    }
}
