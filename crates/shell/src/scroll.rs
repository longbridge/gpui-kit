//! A scroll area that paints scrollbars over the element it was called on.
//!
//! This lives here rather than being imported because the shell is built on
//! `gpui-base` alone: everything it draws has to be expressible in the base
//! layer, and a dependency on `gpui-component` would put the product component
//! library underneath a runtime that is supposed to sit beside it. The scrollbar
//! itself — [`gpui_base::Scrollbar`] — is a base type; only this wrapper had to
//! come along. `gpui-component` keeps its own copy behind a
//! `ScrollableElement` trait; the shell needs one call site, so it names
//! [`Scrollable`] directly and skips the trait.

use std::panic::Location;

use gpui::{
    App, Div, Element, ElementId, InteractiveElement, IntoElement, Overflow, ParentElement,
    PointRefinement, RenderOnce, ScrollHandle, StatefulInteractiveElement, StyleRefinement, Styled,
    Window, div, prelude::FluentBuilder,
};
use gpui_base::{InteractiveElementExt as _, Scrollbar, ScrollbarAxis, StyledExt as _};

/// A scrollable element wrapper that renders the original element as the scroll area and overlays scrollbars.
#[derive(IntoElement)]
pub(crate) struct Scrollable<E: InteractiveElement + Styled + ParentElement + Element> {
    id: ElementId,
    element: E,
    axis: ScrollbarAxis,
}

impl<E> Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element,
{
    #[track_caller]
    pub(crate) fn new(element: E, axis: impl Into<ScrollbarAxis>) -> Self {
        Self {
            id: caller_id(),
            element,
            axis: axis.into(),
        }
    }

    /// Set a specific element id, default is the [`std::panic::Location::caller`].
    ///
    /// Only needed when one call site creates several scrollables, which would
    /// otherwise share a single scroll position.
    pub(crate) fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }
}

impl<E> Styled for Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element,
{
    fn style(&mut self) -> &mut StyleRefinement {
        self.element.style()
    }
}

impl<E> ParentElement for Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element,
{
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.element.extend(elements)
    }
}

impl<E> InteractiveElement for Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element,
{
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.element.interactivity()
    }
}

impl<E> RenderOnce for Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element + 'static,
{
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let scroll_handle = scroll_handle_for(&self.id, window, cx);

        // Preserve the caller-requested size on the wrapper, while keeping the
        // caller's element as the actual scroll-tracked layout container.
        let root_style = root_style_from(&mut self.element, self.axis);

        let root_id = self.id.clone();
        let area_id = (self.id.clone(), "area");
        let content_id = (self.id.clone(), "content");
        let scrollbar_id = (self.id.clone(), "scrollbar");

        let mut content = self
            .element
            .id(content_id)
            .flex_none()
            .map(|this| match self.axis {
                ScrollbarAxis::Vertical => this.h_auto().min_h_full(),
                ScrollbarAxis::Horizontal => this.w_auto().min_w_full(),
                ScrollbarAxis::Both => this.size_auto().min_size_full(),
            });
        // The wrapper holds the caller's maximum size. Left on the content
        // too, it would cap the content at the area's size on the scrolled
        // axis, so nothing would overflow and the area would not scroll.
        let max_size = &mut content.style().max_size;
        match self.axis {
            ScrollbarAxis::Vertical => max_size.height = None,
            ScrollbarAxis::Horizontal => max_size.width = None,
            ScrollbarAxis::Both => *max_size = Default::default(),
        }

        // Keep the scroll area in the normal flow: its content size must
        // propagate to auto-sized ancestors (e.g. a Dialog that grows with
        // its content). An absolutely positioned scroll area would collapse
        // such ancestors to zero height.
        let scroll_area = div()
            .id(area_id)
            .size_full()
            .flex()
            .track_scroll(&scroll_handle)
            .map(|this| match self.axis {
                ScrollbarAxis::Vertical => this.flex_col().overflow_y_scroll(),
                ScrollbarAxis::Horizontal => this.flex_row().overflow_x_scroll(),
                ScrollbarAxis::Both => this.overflow_scroll(),
            })
            // On a single-axis area gpui otherwise remaps the other axis' delta
            // onto ours, so a purely horizontal swipe scrolls this vertically.
            .lock_scroll_axis()
            .child(content);

        // The wrapper lays the area out as a flex item along the scrolled
        // axis, so the area shrinks to the wrapper's size. Sized by its 100%
        // alone, the area is not held to a maximum size on the wrapper with
        // GPUI Fast: in a column whose height follows its content, inside a
        // row, the area came out as tall as that column and the wrapper
        // clipped the rows past its maximum height instead of scrolling to
        // them.
        div()
            .id(root_id)
            .size_full()
            .refine_style(&root_style)
            .relative()
            .flex()
            .map(|this| match self.axis {
                ScrollbarAxis::Horizontal => this.flex_row(),
                ScrollbarAxis::Vertical | ScrollbarAxis::Both => this.flex_col(),
            })
            .child(scroll_area)
            .child(render_scrollbar(
                scrollbar_id,
                &scroll_handle,
                self.axis,
                window,
                cx,
            ))
    }
}

#[inline]
#[track_caller]
fn caller_id() -> ElementId {
    ElementId::CodeLocation(*Location::caller())
}

#[inline]
fn scroll_handle_for(id: &ElementId, window: &mut Window, cx: &mut App) -> ScrollHandle {
    window
        .use_keyed_state(id.clone(), cx, |_, _| ScrollHandle::default())
        .read(cx)
        .clone()
}

/// Copies the outer layout styles from the element, so the wrapper can
/// participate in the parent's layout the same way the source element would.
///
/// The scrolled axis is marked clipped here for the same reason as in
/// `gpui_component::scroll::scrollable`: a flex item only drops its
/// content-based automatic minimum size when its own overflow is not
/// [`Overflow::Visible`], and the scrolled overflow lives on the inner scroll
/// area. Without it a scroll region used as a flex item pushes its siblings out
/// of the container instead of scrolling.
#[inline]
fn root_style_from<E>(element: &mut E, axis: ScrollbarAxis) -> StyleRefinement
where
    E: Styled,
{
    let style = element.style();
    StyleRefinement {
        size: style.size.clone(),
        min_size: style.min_size.clone(),
        max_size: style.max_size.clone(),
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: style.flex_basis,
        align_self: style.align_self,
        overflow: PointRefinement {
            x: axis.has_horizontal().then_some(Overflow::Hidden),
            y: axis.has_vertical().then_some(Overflow::Hidden),
        },
        ..Default::default()
    }
}

#[inline]
fn render_scrollbar(
    id: impl Into<ElementId>,
    scroll_handle: &ScrollHandle,
    axis: ScrollbarAxis,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    // Do not render scrollbar when inspector is picking elements,
    // to allow us to pick the background elements.
    let is_inspector_picking = window.is_inspector_picking(cx);
    if is_inspector_picking {
        return div();
    }

    div()
        .absolute()
        .inset_0()
        .debug_selector(|| "scrollbar-overlay".to_string())
        .child(
            Scrollbar::new(scroll_handle)
                .id(id)
                .axis(axis)
                .viewport_from_layout(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        Bounds, Context, Pixels, Render, ScrollDelta, ScrollEvent, TestAppContext,
        VisualTestContext, canvas, point, px,
    };
    use std::{cell::Cell, rc::Rc};

    struct FlexItemScrollableTest {
        header_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
        last_row_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
        footer_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    }

    impl Render for FlexItemScrollableTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let header_bounds = self.header_bounds.clone();
            let footer_bounds = self.footer_bounds.clone();
            // A fixed-height column of header, flexible scroll area, footer.
            // The content is far taller than the room left for the area, so
            // the area must shrink into the remaining 60px and scroll.
            gpui_base::v_flex()
                .w(px(100.))
                .h(px(100.))
                .child(
                    div().h(px(20.)).flex_shrink_0().child(
                        canvas(
                            move |bounds, _, _| header_bounds.set(Some(bounds)),
                            |_, _, _, _| {},
                        )
                        .size_full(),
                    ),
                )
                .child(Scrollable::new(
                    gpui_base::v_flex()
                        .flex_1()
                        .children((0..5).map(|_| div().h(px(50.)).flex_shrink_0()))
                        .child(tracked_block(px(50.), &self.last_row_bounds)),
                    ScrollbarAxis::Vertical,
                ))
                .child(
                    div().h(px(20.)).flex_shrink_0().child(
                        canvas(
                            move |bounds, _, _| footer_bounds.set(Some(bounds)),
                            |_, _, _, _| {},
                        )
                        .size_full(),
                    ),
                )
        }
    }

    #[gpui::test]
    fn scrollable_flex_item_shrinks_below_its_content(cx: &mut TestAppContext) {
        // Debug selectors are disabled in release builds; capture laid-out
        // bounds directly so the Shell's release test suite checks the layout.
        let header_bounds = Rc::new(Cell::new(None));
        let last_row_bounds = Rc::new(Cell::new(None));
        let footer_bounds = Rc::new(Cell::new(None));
        let (_, cx) = cx.add_window_view(|_, _| FlexItemScrollableTest {
            header_bounds: header_bounds.clone(),
            last_row_bounds: last_row_bounds.clone(),
            footer_bounds: footer_bounds.clone(),
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        // Header and footer stay inside the 100px column, so the area took the
        // 60px left over instead of its content height.
        assert_eq!(
            header_bounds.get().expect("header was laid out").top(),
            px(0.)
        );
        assert_eq!(
            footer_bounds.get().expect("footer was laid out").top(),
            px(80.)
        );

        // Scrolled to the end, the last row ends where the footer starts.
        cx.simulate_event(ScrollEvent {
            position: point(px(50.), px(50.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-1000.))),
            ..Default::default()
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        assert_eq!(
            last_row_bounds
                .get()
                .expect("last row was laid out")
                .bottom(),
            px(80.)
        );
    }

    fn tracked_block(height: Pixels, bounds: &Rc<Cell<Option<Bounds<Pixels>>>>) -> Div {
        let bounds = bounds.clone();
        div()
            .h(height)
            .flex_shrink_0()
            .child(canvas(move |b, _, _| bounds.set(Some(b)), |_, _, _, _| {}).size_full())
    }

    struct MaxHeightScrollableTest {
        first_row_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
        last_row_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
        below_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    }

    impl Render for MaxHeightScrollableTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            // Six rows of 50px in an area that may grow to 100px, between two
            // blocks in a column as tall as its content, in a row. In such a
            // column the area's full height once came out as the column's,
            // 140px, rather than the 100px of the wrapper around it.
            gpui_base::h_flex().w(px(100.)).child(
                gpui_base::v_flex()
                    .w_full()
                    .child(div().h(px(20.)).flex_shrink_0())
                    .child(Scrollable::new(
                        gpui_base::v_flex()
                            .max_h(px(100.))
                            .child(tracked_block(px(50.), &self.first_row_bounds))
                            .children((0..4).map(|_| div().h(px(50.)).flex_shrink_0()))
                            .child(tracked_block(px(50.), &self.last_row_bounds)),
                        ScrollbarAxis::Vertical,
                    ))
                    .child(tracked_block(px(20.), &self.below_bounds)),
            )
        }
    }

    #[gpui::test]
    fn scrollable_with_a_max_height_scrolls_what_overflows_it(cx: &mut TestAppContext) {
        let first_row_bounds = Rc::new(Cell::new(None));
        let last_row_bounds = Rc::new(Cell::new(None));
        let below_bounds = Rc::new(Cell::new(None));
        let (_, cx) = cx.add_window_view(|_, _| MaxHeightScrollableTest {
            first_row_bounds: first_row_bounds.clone(),
            last_row_bounds: last_row_bounds.clone(),
            below_bounds: below_bounds.clone(),
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        let first_row = first_row_bounds.get().expect("first row was laid out");
        let top = first_row.top();
        // The area stops at its maximum height.
        assert_eq!(
            below_bounds.get().expect("block was laid out").top() - top,
            px(100.)
        );

        let mut scroll = |dy: f32| {
            cx.simulate_event(ScrollEvent {
                position: point(first_row.center().x, top + px(10.)),
                delta: ScrollDelta::Pixels(point(px(0.), px(dy))),
                ..Default::default()
            });
            cx.update(|window, cx| {
                _ = window.draw(cx);
            });
        };
        // The rows past it scroll into view, down to the last one, which
        // ends at the area's bottom edge.
        scroll(-40.);
        assert_eq!(
            first_row_bounds
                .get()
                .expect("first row was laid out")
                .top()
                - top,
            px(-40.)
        );
        scroll(-1000.);
        assert_eq!(
            last_row_bounds
                .get()
                .expect("last row was laid out")
                .bottom()
                - top,
            px(100.)
        );
    }

    struct MaxWidthScrollableTest {
        first_column_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    }

    impl Render for MaxWidthScrollableTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let first_column_bounds = self.first_column_bounds.clone();
            // Six columns of 50px in an area that may grow to 100px wide.
            gpui_base::h_flex().h(px(40.)).child(Scrollable::new(
                gpui_base::h_flex()
                    .h_full()
                    .max_w(px(100.))
                    .child(
                        div().w(px(50.)).h_full().flex_shrink_0().child(
                            canvas(
                                move |bounds, _, _| first_column_bounds.set(Some(bounds)),
                                |_, _, _, _| {},
                            )
                            .size_full(),
                        ),
                    )
                    .children((0..5).map(|_| div().w(px(50.)).h_full().flex_shrink_0())),
                ScrollbarAxis::Horizontal,
            ))
        }
    }

    #[gpui::test]
    fn scrollable_with_a_max_width_scrolls_what_overflows_it(cx: &mut TestAppContext) {
        let first_column_bounds = Rc::new(Cell::new(None));
        let (_, cx) = cx.add_window_view(|_, _| MaxWidthScrollableTest {
            first_column_bounds: first_column_bounds.clone(),
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        cx.simulate_event(ScrollEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-40.), px(0.))),
            ..Default::default()
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        assert_eq!(
            first_column_bounds
                .get()
                .expect("first column was laid out")
                .left(),
            px(-40.)
        );
    }
}
