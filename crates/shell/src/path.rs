//! Native GPUI path materialization.

use gpui::{
    App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, Point, Refineable as _, Style, StyleRefinement, Window, point, px,
};

use crate::{
    spec::{BackgroundSpec, SpecOp},
    value::Bridged,
};

#[derive(Clone, Copy)]
enum Coordinate {
    Pixels(f32),
    Percent(f32),
}

impl Coordinate {
    fn parse(value: &Bridged) -> Option<Self> {
        match value {
            Bridged::Number(value) if value.is_finite() => Some(Self::Pixels(*value as f32)),
            Bridged::Str(value) => value
                .strip_suffix('%')
                .and_then(|value| value.parse::<f32>().ok())
                .filter(|value| value.is_finite())
                .map(|value| Self::Percent(value / 100.)),
            _ => None,
        }
    }

    fn resolve(self, start: Pixels, length: Pixels) -> Pixels {
        match self {
            Self::Pixels(value) => start + px(value),
            Self::Percent(value) => start + length * value,
        }
    }
}

fn command_point(args: &[Bridged], offset: usize, bounds: Bounds<Pixels>) -> Option<Point<Pixels>> {
    let x = Coordinate::parse(args.get(offset)?)?;
    let y = Coordinate::parse(args.get(offset + 1)?)?;
    Some(point(
        x.resolve(bounds.origin.x, bounds.size.width),
        y.resolve(bounds.origin.y, bounds.size.height),
    ))
}

pub(crate) struct NativePath {
    fill: bool,
    background: BackgroundSpec,
    stroke_width: Pixels,
    dash_array: Vec<Pixels>,
    commands: Vec<SpecOp>,
    style: StyleRefinement,
}

impl NativePath {
    pub(crate) fn new(
        fill: bool,
        background: BackgroundSpec,
        stroke_width: f32,
        commands: Vec<SpecOp>,
        style: StyleRefinement,
    ) -> Self {
        let dash_array = commands
            .iter()
            .find_map(|op| match op {
                SpecOp::Method("dash_array", args) => Some(
                    args.iter()
                        .filter_map(|value| value.as_pixels().ok())
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default();
        Self {
            fill,
            background,
            stroke_width: px(stroke_width),
            dash_array,
            commands,
            style,
        }
    }

    /// An invalid colour draws nothing rather than failing the frame, as it
    /// always has for a path.
    fn background(&self) -> Option<gpui::Background> {
        self.background.resolve().ok()
    }

    fn build(&self, bounds: Bounds<Pixels>) -> Option<gpui::Path<Pixels>> {
        let mut builder = if self.fill {
            gpui::PathBuilder::fill()
        } else {
            gpui::PathBuilder::stroke(self.stroke_width)
        };
        if !self.dash_array.is_empty() {
            builder = builder.dash_array(&self.dash_array);
        }

        for command in &self.commands {
            let SpecOp::Method(name, args) = command else {
                continue;
            };
            match *name {
                "move_to" => builder.move_to(command_point(args, 0, bounds)?),
                "line_to" => builder.line_to(command_point(args, 0, bounds)?),
                "curve_to" => builder.curve_to(
                    command_point(args, 0, bounds)?,
                    command_point(args, 2, bounds)?,
                ),
                "cubic_bezier_to" => builder.cubic_bezier_to(
                    command_point(args, 0, bounds)?,
                    command_point(args, 2, bounds)?,
                    command_point(args, 4, bounds)?,
                ),
                "arc_to" => builder.arc_to(
                    command_point(args, 0, Bounds::new(point(px(0.), px(0.)), bounds.size))?,
                    args.get(2)?.as_pixels().ok()?,
                    args.get(3)?.is_truthy(),
                    args.get(4)?.is_truthy(),
                    command_point(args, 5, bounds)?,
                ),
                "close" => builder.close(),
                "dash_array" => {}
                _ => {}
            }
        }
        builder.build().ok()
    }
}

impl IntoElement for NativePath {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativePath {
    type RequestLayoutState = ();
    type PrepaintState = Option<gpui::Path<Pixels>>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.refine(&self.style);
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
        self.build(bounds)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        path: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        if let Some(path) = path.take() {
            if let Some(background) = self.background() {
                window.paint_path(path, background);
            }
        }
    }
}
