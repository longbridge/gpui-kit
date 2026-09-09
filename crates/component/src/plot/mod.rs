mod axis;
mod grid;
pub mod label;
pub mod scale;
pub mod shape;
pub mod tooltip;

pub use gpui_component_macros::IntoPlot;

use std::{fmt::Debug, ops::Add};

use gpui::{
    AnyElement, App, Bounds, ElementId, IntoElement, Path, PathBuilder, Pixels, Point, Window,
    point, px,
};

pub use axis::{AXIS_GAP, AxisLabelSide, AxisText, PlotAxis};
pub use grid::Grid;
pub use label::PlotLabel;

use tooltip::TooltipState;

pub trait Plot: IntoElement {
    /// Lay out and place the child elements this plot hosts (e.g. element labels).
    ///
    /// Called during the element's prepaint phase, so implementations may use
    /// [`AnyElement::layout_as_root`] / [`AnyElement::prepaint_at`] to measure and
    /// position children — neither is legal from [`Plot::paint`]. The returned
    /// elements are painted right after `paint`, below the tooltip overlay.
    ///
    /// Runs before [`Plot::tooltip_state`] and [`Plot::tooltip`], so anything
    /// resolved here can be reused by them.
    ///
    /// The default returns no children.
    fn prepaint(
        &mut self,
        _bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Vec<AnyElement> {
        vec![]
    }

    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App);

    /// A stable element id that enables interactive tooltip support for this plot.
    ///
    /// Return `Some(id)` to opt in to tooltips; the id must be unique among sibling
    /// elements. Returning `None` (the default) disables all tooltip behavior, leaving
    /// the plot a pure, non-interactive element identical to the pre-tooltip behavior.
    fn id(&self) -> Option<ElementId> {
        None
    }

    /// Map the cursor to the tooltip state to display.
    ///
    /// `position` is the cursor position relative to the plot's top-left origin (already
    /// origin-subtracted), and `bounds` is the painted area. Return the [`TooltipState`]
    /// to display (highlighted index, crosshair point, dots, side), or `None` to show
    /// nothing. Only called while the cursor is inside `bounds`.
    ///
    /// The default returns `None`.
    fn tooltip_state(
        &self,
        _position: Point<Pixels>,
        _bounds: Bounds<Pixels>,
        _cx: &App,
    ) -> Option<TooltipState> {
        None
    }

    /// Render the tooltip overlay for the active [`TooltipState`].
    ///
    /// `cursor` is the live cursor position (relative to the plot origin) and `bounds` is the
    /// plot's painted area, so the tooltip box can follow the cursor (pass `cursor` and
    /// `bounds.size` to [`tooltip::Tooltip::new`]). Return the overlay element; it is painted
    /// absolutely positioned above the plot graphics but below sibling content drawn after
    /// the plot ([`tooltip::Tooltip`] defers its box to paint above everything). The default
    /// returns `None`.
    fn tooltip(
        &self,
        _state: &TooltipState,
        _cursor: Point<Pixels>,
        _bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<AnyElement> {
        None
    }
}

#[derive(Clone, Copy, Default)]
pub enum StrokeStyle {
    #[default]
    Natural,
    Linear,
    StepAfter,
}

pub fn origin_point<T>(x: T, y: T, origin: Point<T>) -> Point<T>
where
    T: Default + Clone + Debug + PartialEq + Add<Output = T>,
{
    point(x, y) + origin
}

pub fn polygon<T>(points: &[Point<T>], bounds: &Bounds<Pixels>) -> Option<Path<Pixels>>
where
    T: Default + Clone + Copy + Debug + Into<f32> + PartialEq,
{
    let mut path = PathBuilder::stroke(px(1.));
    let points = &points
        .iter()
        .map(|p| {
            point(
                px(p.x.into() + bounds.origin.x.as_f32()),
                px(p.y.into() + bounds.origin.y.as_f32()),
            )
        })
        .collect::<Vec<_>>();
    path.add_polygon(points, false);
    path.build().ok()
}
