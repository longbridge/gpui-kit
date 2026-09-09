use gpui::{Bounds, Hsla, PathBuilder, Pixels, Point, Window, px};

use super::origin_point;

pub struct Grid {
    x: Vec<Pixels>,
    y: Vec<Pixels>,
    stroke: Hsla,
    dash_array: Option<Vec<Pixels>>,
}

impl Grid {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            x: vec![],
            y: vec![],
            stroke: Default::default(),
            dash_array: None,
        }
    }

    /// Set the x of the Grid.
    pub fn x(mut self, x: Vec<impl Into<Pixels>>) -> Self {
        self.x = x.into_iter().map(|v| v.into()).collect();
        self
    }

    /// Set the y of the Grid.
    pub fn y(mut self, y: Vec<impl Into<Pixels>>) -> Self {
        self.y = y.into_iter().map(|v| v.into()).collect();
        self
    }

    /// Set the stroke color of the Grid.
    pub fn stroke(mut self, stroke: impl Into<Hsla>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Set the dash array of the Grid.
    pub fn dash_array(mut self, dash_array: &[Pixels]) -> Self {
        self.dash_array = Some(dash_array.to_vec());
        self
    }

    fn points(&self, bounds: &Bounds<Pixels>) -> Vec<(Point<Pixels>, Point<Pixels>)> {
        let size = bounds.size;
        let origin = bounds.origin;

        let mut x = self
            .x
            .iter()
            .map(|x| {
                (
                    origin_point(*x, px(0.), origin),
                    origin_point(*x, size.height, origin),
                )
            })
            .collect::<Vec<_>>();

        let y = self
            .y
            .iter()
            .map(|y| {
                (
                    origin_point(px(0.), *y, origin),
                    origin_point(size.width, *y, origin),
                )
            })
            .collect::<Vec<_>>();

        x.extend(y);
        x
    }

    /// Paint the Grid.
    pub fn paint(&self, bounds: &Bounds<Pixels>, window: &mut Window) {
        let points = self.points(bounds);

        for (start, end) in points {
            let mut builder = PathBuilder::stroke(px(1.));

            if let Some(dash_array) = &self.dash_array {
                builder = builder.dash_array(&dash_array);
            }

            builder.move_to(start);
            builder.line_to(end);
            if let Ok(line) = builder.build() {
                window.paint_path(line, self.stroke);
            }
        }
    }
}
