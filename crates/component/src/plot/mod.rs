mod axis;
mod grid;
pub mod label;
mod path_cache;
pub mod scale;
pub mod shape;
pub mod tooltip;

pub use gpui_component_macros::IntoPlot;

use std::{fmt::Debug, ops::Add};

use gpui::{
    AnyElement, App, Bounds, ElementId, IntoElement, Path, PathBuilder, Pixels, Point, Window,
    point, px,
};

pub use axis::{AXIS_GAP, AxisLabelPlacement, AxisLabelSide, AxisText, PlotAxis};
pub use grid::Grid;
pub use label::PlotLabel;
pub use path_cache::{PathCache, PathCaches, ShapeKey};

use tooltip::{PlotHover, TooltipState};

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
    /// Return `Some(id)` to opt in to tooltips and hover motion; the id must be unique
    /// among sibling elements. Returning `None` (the default for a hand-written plot)
    /// disables all tooltip behavior, leaving the plot a pure, non-interactive element.
    ///
    /// The charts in [`crate::chart`] always return `Some`: their id defaults to the
    /// source location they were constructed at, and `id` renames it.
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

    /// Receive the datum in focus this frame, before [`Plot::tooltip`] and
    /// [`Plot::paint`] run.
    ///
    /// `hover` carries the [`TooltipState`] the cursor resolved to, and it
    /// lingers after the cursor leaves while [`PlotHover::focus`] eases back to
    /// zero, so a hover-driven presentation can fade out over the last datum
    /// instead of vanishing. `None` means nothing is hovered and nothing is
    /// fading.
    ///
    /// Called on every frame the plot has an [`Plot::id`], so this is where a
    /// plot samples its hover motion ([`gpui_base::transition`],
    /// [`gpui_base::spring`]) and keeps the result for the other two methods.
    /// The default ignores the hover.
    fn hover(&mut self, _hover: Option<&PlotHover>, _window: &mut Window, _cx: &mut App) {}

    /// Render the tooltip overlay for the active [`TooltipState`].
    ///
    /// `cursor` is the live cursor position (relative to the plot origin) and `bounds` is the
    /// plot's painted area, so the tooltip box can follow the cursor (pass `cursor` and
    /// `bounds.size` to [`tooltip::Tooltip::new`]). Return the overlay element; it is painted
    /// absolutely positioned above the plot graphics but below sibling content drawn after
    /// the plot ([`tooltip::Tooltip`] defers its box to paint above everything). The default
    /// returns `None`.
    ///
    /// Also called while the hover fades out, with the lingering `state` and the
    /// last `cursor`; a [`tooltip::Tooltip`] returned here fades with the hover
    /// and glides its crosshair and dots between data on its own
    /// ([`tooltip::Tooltip::glide`]).
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

#[derive(Clone, Copy, Default, Hash, PartialEq, Eq)]
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
