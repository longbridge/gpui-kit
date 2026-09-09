use std::{panic::Location, sync::Arc};

use gpui::{
    AnyElement, App, Axis, Bounds, Element, ElementId, Entity, GlobalElementId, InspectorElementId,
    InteractiveElement as _, IntoElement, LayoutId, ParentElement, Pixels, Point, RenderOnce, Role,
    SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, Subscription, Window,
    div, prelude::FluentBuilder as _, px,
};
use gpui_base::spring;
use rust_i18n::t;

use super::{CONTEXT, scroll_mask::CarouselScrollMask, state::CarouselState};
use crate::{
    AxisExt as _, Disableable as _, ElementExt as _, Selectable as _, Sizable as _, Size,
    StyledExt as _, ThemeStyled as _,
    actions::{SelectDown, SelectFirst, SelectLast, SelectLeft, SelectRight, SelectUp},
    button::Button,
    icon::IconName,
    theme::ActiveTheme as _,
};

/// A composable carousel root.
///
/// Add one [`CarouselContent`] and any optional controls as children. Every
/// part must share the same [`CarouselState`].
#[derive(IntoElement)]
pub struct Carousel {
    id: ElementId,
    state: Entity<CarouselState>,
    style: StyleRefinement,
    accessibility_label: SharedString,
    focus_ring_enabled: bool,
    children: Vec<AnyElement>,
}

struct CarouselStateObserver {
    _subscription: Subscription,
}

impl Carousel {
    /// Creates a Carousel bound to `state`.
    pub fn new(id: impl Into<ElementId>, state: &Entity<CarouselState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            style: StyleRefinement::default(),
            accessibility_label: t!("Carousel.label").into(),
            focus_ring_enabled: true,
            children: Vec::new(),
        }
    }

    /// Sets the name announced for the carousel region.
    pub fn with_accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = label.into();
        self
    }
}

impl ParentElement for Carousel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Carousel {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl crate::FocusableExt for Carousel {
    fn focus_ring(mut self, enabled: bool) -> Self {
        self.focus_ring_enabled = enabled;
        self
    }

    fn is_focus_ring_enabled(&self) -> bool {
        self.focus_ring_enabled
    }
}

impl RenderOnce for Carousel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let observed_state = self.state.clone();
        let _observer = window.use_keyed_state(
            ("carousel-state-observer", self.state.entity_id()),
            cx,
            move |_, cx| CarouselStateObserver {
                _subscription: cx.observe(&observed_state, |_, _, cx| cx.notify()),
            },
        );
        let snapshot = self.state.read(cx);
        let axis = snapshot.axis();
        let viewport_size = snapshot.viewport_size();
        let focus_handle = window
            .use_keyed_state(("carousel-focus", self.state.entity_id()), cx, |_, cx| {
                cx.focus_handle()
            })
            .read(cx)
            .clone();
        let focus_visible = focus_handle.is_focused(window) && self.focus_ring_enabled;
        let previous_state = self.state.clone();
        let next_state = self.state.clone();
        let first_state = self.state.clone();
        let last_state = self.state.clone();

        div()
            .id(self.id)
            .relative()
            .role(Role::Region)
            .aria_label(self.accessibility_label)
            .track_focus(&focus_handle.tab_stop(true))
            .key_context(CONTEXT)
            .on_action(
                window.listener_for(&previous_state, move |state, _: &SelectLeft, _, cx| {
                    if axis.is_horizontal() && state.select_previous(cx) {
                        cx.stop_propagation();
                    }
                }),
            )
            .on_action(
                window.listener_for(&next_state, move |state, _: &SelectRight, _, cx| {
                    if axis.is_horizontal() && state.select_next(cx) {
                        cx.stop_propagation();
                    }
                }),
            )
            .on_action(
                window.listener_for(&previous_state, move |state, _: &SelectUp, _, cx| {
                    if axis.is_vertical() && state.select_previous(cx) {
                        cx.stop_propagation();
                    }
                }),
            )
            .on_action(
                window.listener_for(&next_state, move |state, _: &SelectDown, _, cx| {
                    if axis.is_vertical() && state.select_next(cx) {
                        cx.stop_propagation();
                    }
                }),
            )
            .on_action(
                window.listener_for(&first_state, |state, _: &SelectFirst, _, cx| {
                    if state.select_first(cx) {
                        cx.stop_propagation();
                    }
                }),
            )
            .on_action(
                window.listener_for(&last_state, |state, _: &SelectLast, _, cx| {
                    if state.select_last(cx) {
                        cx.stop_propagation();
                    }
                }),
            )
            .children(self.children)
            .when(focus_visible, |this| {
                this.when_some(viewport_size, |this, size| {
                    this.child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .w(size.width)
                            .h(size.height)
                            .border_1()
                            .border_color(cx.theme().transparent)
                            .rounded(cx.theme().radius)
                            .focus_ring_style(window, cx),
                    )
                })
            })
            .refine_style(&self.style)
    }
}

#[derive(Default, PartialEq)]
struct CarouselGeometry {
    viewport: Bounds<Pixels>,
    items: Vec<Bounds<Pixels>>,
    has_runway: bool,
    revision: usize,
}

impl CarouselGeometry {
    fn read(state: &CarouselState, has_runway: bool, rendered_item_count: usize) -> Self {
        let handle = state.scroll_handle();
        let item_offset = usize::from(has_runway);
        Self {
            viewport: handle.bounds(),
            items: (0..state.item_count().min(rendered_item_count))
                .filter_map(|ix| handle.bounds_for_item(ix + item_offset))
                .collect(),
            has_runway,
            revision: 0,
        }
    }

    fn same_layout(&self, other: &Self) -> bool {
        self.viewport == other.viewport
            && self.items == other.items
            && self.has_runway == other.has_runway
    }
}

/// The clipped viewport and snap track for Carousel items.
#[derive(IntoElement)]
pub struct CarouselContent {
    state: Entity<CarouselState>,
    style: StyleRefinement,
    track_style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl CarouselContent {
    /// Creates content bound to `state`.
    pub fn new(state: &Entity<CarouselState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            track_style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    /// Sets style overrides for the inner flex track.
    ///
    /// Use this for paired Carousel spacing such as a negative leading margin.
    /// The [`Styled`] implementation applies to the clipped viewport itself.
    pub fn track_style(mut self, style: StyleRefinement) -> Self {
        self.track_style = style;
        self
    }
}

impl ParentElement for CarouselContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CarouselContent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// A layout-transparent proxy that paints one real item in the closest loop
/// cycle. The child keeps its original layout id, so ScrollHandle geometry
/// continues to address logical items without cloning their elements.
struct CarouselLoopItem {
    child: AnyElement,
    index: usize,
    state: Entity<CarouselState>,
}

impl IntoElement for CarouselLoopItem {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for CarouselLoopItem {
    type RequestLayoutState = ();
    type PrepaintState = Point<Pixels>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let offset = self.state.read(cx).loop_item_offset(self.index);
        window.with_element_offset(offset, |window| {
            self.child.prepaint(window, cx);
        });
        offset
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

impl RenderOnce for CarouselContent {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let entity_id = self.state.entity_id();
        let snapshot = self.state.read(cx);
        let axis = snapshot.axis();
        let selected_ix = snapshot.selected_index();
        let item_count = snapshot.item_count();
        let handle = snapshot.scroll_handle().clone();
        let interacting = snapshot.is_interacting();
        let motion_revision = snapshot.motion_revision();
        let loop_runway = snapshot.loop_runway();
        let loop_layout_transitioning = snapshot.is_loop_layout_transitioning();
        let state_snap_target = selected_ix.and_then(|ix| snapshot.motion_target_for(ix));

        let geometry = window.use_keyed_state(
            ElementId::NamedChild(
                Arc::new(("carousel-geometry", entity_id).into()),
                "content".into(),
            ),
            cx,
            |_, _| CarouselGeometry::default(),
        );
        let geometry_revision = geometry.read(cx).revision;

        let current = axis_value(handle.offset(), axis);
        let target = if loop_layout_transitioning {
            current
        } else {
            state_snap_target
                .map(|target| axis_value(target, axis))
                .or_else(|| {
                    selected_ix.and_then(|ix| {
                        snap_offset(&handle, axis, ix + usize::from(loop_runway.is_some()))
                    })
                })
                .unwrap_or(current)
        };
        let target = if interacting { current } else { target };
        let snap_spring = cx.theme().motion_tokens().spring_move.with_epsilon(0.5);
        let animated = spring(
            (
                ("carousel-content", entity_id),
                SharedString::from(format!("offset-{motion_revision}-{geometry_revision}")),
            ),
            target.as_f32(),
            snap_spring.with_travel(!interacting),
            window,
            cx,
        );
        let mut offset = handle.offset();
        set_axis_value(&mut offset, axis, px(animated));
        set_axis_value(
            &mut offset,
            if axis.is_horizontal() {
                Axis::Vertical
            } else {
                Axis::Horizontal
            },
            Pixels::ZERO,
        );
        handle.set_offset(offset);
        if !interacting {
            let rendered = offset;
            if let Some(rebased) = self
                .state
                .update(cx, |state, cx| state.settle_loop_motion(rendered, cx))
            {
                offset = rebased;
                handle.set_offset(offset);
            }
        }

        let geometry_state = self.state.clone();
        let viewport_id: ElementId = ("carousel-content", entity_id).into();

        let rendered_item_count = self.children.len();
        let loop_state = self.state.clone();
        let children = self
            .children
            .into_iter()
            .enumerate()
            .map(move |(index, child)| CarouselLoopItem {
                child,
                index,
                state: loop_state.clone(),
            });
        let runway_spacer = |runway: Pixels| {
            div()
                .flex_none()
                .when(axis.is_horizontal(), |this| this.w(runway))
                .when(axis.is_vertical(), |this| this.h(runway))
        };
        let has_runway = loop_runway.is_some();

        div()
            .relative()
            .w_full()
            .refine_style(&self.style)
            .overflow_hidden()
            .child(
                div()
                    .id(viewport_id.clone())
                    .w_full()
                    .when(axis.is_vertical(), |this| this.h_full())
                    .flex()
                    .when(axis.is_horizontal(), |this| this.flex_row().ml_neg_4())
                    .when(axis.is_vertical(), |this| this.flex_col().mt_neg_4())
                    .track_scroll(&handle)
                    .when_some(loop_runway, |this, runway| {
                        this.child(runway_spacer(runway))
                    })
                    .children(children)
                    .when_some(loop_runway, |this, runway| {
                        this.child(runway_spacer(runway))
                    })
                    .refine_style(&self.track_style),
            )
            .child(CarouselScrollMask::new(axis, &self.state).id(viewport_id))
            .on_prepaint(move |_, _, cx| {
                let next = CarouselGeometry::read(
                    geometry_state.read(cx),
                    has_runway,
                    rendered_item_count,
                );
                if !geometry.read(cx).same_layout(&next) {
                    geometry_state.update(cx, |state, _| {
                        state.set_geometry_with_runway(
                            next.viewport,
                            next.items.clone(),
                            next.has_runway,
                        );
                    });
                    geometry.update(cx, |current, cx| {
                        current.viewport = next.viewport;
                        current.items = next.items;
                        current.has_runway = next.has_runway;
                        current.revision = current.revision.wrapping_add(1);
                        cx.notify();
                    });
                }
            })
            .when(item_count == 0, |this| this.invisible())
    }
}

/// One logical slide in a [`CarouselContent`].
#[derive(IntoElement)]
pub struct CarouselItem {
    id: ElementId,
    index: usize,
    state: Entity<CarouselState>,
    style: StyleRefinement,
    accessibility_label: Option<SharedString>,
    children: Vec<AnyElement>,
}

impl CarouselItem {
    /// Creates the item at the zero-based `index` used by `state`.
    pub fn new(id: impl Into<ElementId>, index: usize, state: &Entity<CarouselState>) -> Self {
        Self {
            id: id.into(),
            index,
            state: state.clone(),
            style: StyleRefinement::default(),
            accessibility_label: None,
            children: Vec::new(),
        }
    }

    /// Replaces the generated "Slide N of M" accessibility label.
    pub fn with_accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl ParentElement for CarouselItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CarouselItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CarouselItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let axis = state.axis();
        let count = state.item_count();
        let label = self.accessibility_label.unwrap_or_else(|| {
            t!(
                "Carousel.slide",
                current = self.index.saturating_add(1),
                total = count
            )
            .into()
        });

        div()
            .id(self.id)
            .role(Role::Group)
            .aria_label(label)
            .aria_position_in_set(self.index.saturating_add(1))
            .aria_size_of_set(count)
            .min_w_0()
            .min_h_0()
            .flex_none()
            .when(axis.is_horizontal(), |this| this.w_full().pl_4())
            .when(axis.is_vertical(), |this| this.h_full().pt_4())
            .children(self.children)
            .refine_style(&self.style)
    }
}

/// A previous-slide control positioned around the Carousel viewport.
#[derive(IntoElement)]
pub struct CarouselPrevious {
    state: Entity<CarouselState>,
    size: Size,
    style: StyleRefinement,
    accessibility_label: Option<SharedString>,
    children: Vec<AnyElement>,
}

impl CarouselPrevious {
    /// Creates a previous-slide control bound to `state`.
    pub fn new(state: &Entity<CarouselState>) -> Self {
        Self {
            state: state.clone(),
            size: Size::Medium,
            style: StyleRefinement::default(),
            accessibility_label: None,
            children: Vec::new(),
        }
    }

    /// Replaces the generated previous-slide accessibility label and tooltip.
    pub fn with_accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl crate::Sizable for CarouselPrevious {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for CarouselPrevious {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for CarouselPrevious {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for CarouselPrevious {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        carousel_control(
            self.state,
            self.size,
            self.style,
            self.accessibility_label,
            self.children,
            false,
            cx,
        )
    }
}

/// A next-slide control positioned around the Carousel viewport.
#[derive(IntoElement)]
pub struct CarouselNext {
    state: Entity<CarouselState>,
    size: Size,
    style: StyleRefinement,
    accessibility_label: Option<SharedString>,
    children: Vec<AnyElement>,
}

impl CarouselNext {
    /// Creates a next-slide control bound to `state`.
    pub fn new(state: &Entity<CarouselState>) -> Self {
        Self {
            state: state.clone(),
            size: Size::Medium,
            style: StyleRefinement::default(),
            accessibility_label: None,
            children: Vec::new(),
        }
    }

    /// Replaces the generated next-slide accessibility label and tooltip.
    pub fn with_accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl crate::Sizable for CarouselNext {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for CarouselNext {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for CarouselNext {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for CarouselNext {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        carousel_control(
            self.state,
            self.size,
            self.style,
            self.accessibility_label,
            self.children,
            true,
            cx,
        )
    }
}

fn carousel_control(
    state: Entity<CarouselState>,
    size: Size,
    style: StyleRefinement,
    accessibility_label: Option<SharedString>,
    children: Vec<AnyElement>,
    next: bool,
    cx: &mut App,
) -> impl IntoElement {
    let snapshot = state.read(cx);
    let axis = snapshot.axis();
    let viewport_size = snapshot.viewport_size();
    let disabled = if next {
        !snapshot.has_next()
    } else {
        !snapshot.has_previous()
    };
    let (name, default_label, icon) = match (axis, next) {
        (Axis::Horizontal, false) => ("previous", t!("Carousel.previous"), IconName::ChevronLeft),
        (Axis::Horizontal, true) => ("next", t!("Carousel.next"), IconName::ChevronRight),
        (Axis::Vertical, false) => ("previous", t!("Carousel.previous"), IconName::ChevronUp),
        (Axis::Vertical, true) => ("next", t!("Carousel.next"), IconName::ChevronDown),
    };
    let label = accessibility_label.unwrap_or_else(|| default_label.into());
    let has_custom_content = !children.is_empty();
    let id = ElementId::NamedChild(
        Arc::new(("carousel-control", state.entity_id()).into()),
        name.into(),
    );

    div()
        .absolute()
        .top_0()
        .left_0()
        .when_some(viewport_size, |this, size| {
            this.w(size.width).h(size.height)
        })
        .when(viewport_size.is_none(), |this| this.right_0().bottom_0())
        .child(
            Button::new(id)
                .outline()
                .with_size(size)
                .when(!has_custom_content, |this| this.icon(icon))
                .accessibility_label(label.clone())
                .tooltip(label)
                .disabled(disabled)
                .absolute()
                .rounded_full_style(cx)
                .when(axis.is_horizontal() && !next, |this| {
                    this.right_full().mr_4().top_0().bottom_0().my_auto()
                })
                .when(axis.is_horizontal() && next, |this| {
                    this.left_full().ml_4().top_0().bottom_0().my_auto()
                })
                .when(axis.is_vertical() && !next, |this| {
                    this.bottom_full().mb_4().left_0().right_0().mx_auto()
                })
                .when(axis.is_vertical() && next, |this| {
                    this.top_full().mt_4().left_0().right_0().mx_auto()
                })
                .when(!disabled, |this| {
                    this.on_click(move |_, _, cx| {
                        state.update(cx, |state, cx| {
                            if next {
                                state.select_next(cx);
                            } else {
                                state.select_previous(cx);
                            }
                        });
                    })
                })
                .children(children)
                .refine_style(&style),
        )
}

/// A composable container for Carousel pagination items.
#[derive(IntoElement)]
pub struct CarouselPagination {
    id: ElementId,
    style: StyleRefinement,
    accessibility_label: SharedString,
    children: Vec<AnyElement>,
}

impl CarouselPagination {
    /// Creates an empty pagination container.
    #[track_caller]
    pub fn new() -> Self {
        Self {
            id: ElementId::CodeLocation(*Location::caller()),
            style: StyleRefinement::default(),
            accessibility_label: t!("Carousel.pagination").into(),
            children: Vec::new(),
        }
    }

    /// Sets the name announced for the pagination group.
    pub fn with_accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = label.into();
        self
    }
}

impl ParentElement for CarouselPagination {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CarouselPagination {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CarouselPagination {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .role(Role::Group)
            .aria_label(self.accessibility_label)
            .flex()
            .items_center()
            .justify_center()
            .gap_2()
            .children(self.children)
            .refine_style(&self.style)
    }
}

/// One application-styled pagination control for a Carousel item.
#[derive(IntoElement)]
pub struct CarouselPaginationItem {
    id: ElementId,
    index: usize,
    state: Entity<CarouselState>,
    size: Size,
    style: StyleRefinement,
    accessibility_label: Option<SharedString>,
    children: Vec<AnyElement>,
}

impl CarouselPaginationItem {
    /// Creates a pagination item for the zero-based `index`.
    pub fn new(id: impl Into<ElementId>, index: usize, state: &Entity<CarouselState>) -> Self {
        Self {
            id: id.into(),
            index,
            state: state.clone(),
            size: Size::XSmall,
            style: StyleRefinement::default(),
            accessibility_label: None,
            children: Vec::new(),
        }
    }

    /// Replaces the generated "Go to slide N" accessibility label.
    pub fn with_accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl crate::Sizable for CarouselPaginationItem {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ParentElement for CarouselPaginationItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CarouselPaginationItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CarouselPaginationItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let selected = self.state.read(cx).selected_index() == Some(self.index);
        let disabled = self.index >= self.state.read(cx).item_count();
        let label = self.accessibility_label.unwrap_or_else(|| {
            t!(
                "Carousel.go_to_slide",
                current = self.index.saturating_add(1)
            )
            .into()
        });
        let state = self.state;
        let index = self.index;

        Button::new(self.id)
            .compact()
            .with_size(self.size)
            .selected(selected)
            .accessibility_label(label)
            .disabled(disabled)
            .children(self.children)
            .when(!disabled, |this| {
                this.on_click(move |_, _, cx| {
                    state.update(cx, |state, cx| {
                        state.select_index(index, cx);
                    });
                })
            })
            .refine_style(&self.style)
    }
}

fn axis_value(point: Point<Pixels>, axis: Axis) -> Pixels {
    if axis.is_horizontal() {
        point.x
    } else {
        point.y
    }
}

fn set_axis_value(point: &mut Point<Pixels>, axis: Axis, value: Pixels) {
    if axis.is_horizontal() {
        point.x = value;
    } else {
        point.y = value;
    }
}

fn snap_offset(handle: &gpui::ScrollHandle, axis: Axis, index: usize) -> Option<Pixels> {
    let viewport = handle.bounds();
    let item = handle.bounds_for_item(index)?;
    let target = if axis.is_horizontal() {
        viewport.left() - item.left()
    } else {
        viewport.top() - item.top()
    };
    let max = axis_value(handle.max_offset(), axis).max(Pixels::ZERO);
    Some(target.clamp(-max, Pixels::ZERO))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, Context, Render, point};
    use gpui_base::FocusableExt as _;

    #[test]
    fn axis_helpers_only_change_the_requested_coordinate() {
        let mut value = point(px(3.), px(7.));
        set_axis_value(&mut value, Axis::Horizontal, px(11.));
        assert_eq!(value, point(px(11.), px(7.)));
        set_axis_value(&mut value, Axis::Vertical, px(-5.));
        assert_eq!(value, point(px(11.), px(-5.)));
    }

    #[gpui::test]
    fn carousel_controls_accept_semantic_sizes(cx: &mut gpui::TestAppContext) {
        let state = cx.update(|cx| cx.new(|_| CarouselState::new(2)));

        assert_eq!(CarouselPrevious::new(&state).size, Size::Medium);
        assert_eq!(CarouselNext::new(&state).size, Size::Medium);
        assert_eq!(CarouselPrevious::new(&state).large().size, Size::Large);
        assert_eq!(CarouselNext::new(&state).xsmall().size, Size::XSmall);
        assert_eq!(
            CarouselPaginationItem::new("pagination", 0, &state)
                .small()
                .size,
            Size::Small
        );
    }

    #[gpui::test]
    fn carousel_focus_ring_is_configurable(cx: &mut gpui::TestAppContext) {
        let state = cx.update(|cx| cx.new(|_| CarouselState::new(2)));

        assert!(Carousel::new("carousel", &state).is_focus_ring_enabled());
        assert!(
            !Carousel::new("carousel", &state)
                .focus_ring(false)
                .is_focus_ring_enabled()
        );
    }

    struct KeyboardHarness {
        state: Entity<CarouselState>,
    }

    impl Render for KeyboardHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().tab_group().child(
                Carousel::new("carousel", &self.state)
                    .w(px(100.))
                    .h(px(100.))
                    .child(
                        CarouselContent::new(&self.state)
                            .h(px(100.))
                            .children((0..3).map(|index| {
                                CarouselItem::new(("carousel-item", index), index, &self.state)
                                    .child(index.to_string())
                            })),
                    ),
            )
        }
    }

    fn assert_contextual_navigation_keys(cx: &mut gpui::TestAppContext, axis: Axis) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|_| CarouselState::new(3).with_axis(axis)));
        let (_, cx) = cx.add_window_view({
            let state = state.clone();
            move |_, _| KeyboardHarness { state }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));

        cx.update(|window, cx| window.focus_next(cx));
        let (primary, secondary) = if axis.is_horizontal() {
            ("right", "down")
        } else {
            ("down", "right")
        };
        cx.simulate_keystrokes(primary);
        assert_eq!(
            state.read_with(cx, |state, _| state.selected_index()),
            Some(1)
        );
        cx.simulate_keystrokes(secondary);
        assert_eq!(
            state.read_with(cx, |state, _| state.selected_index()),
            Some(1)
        );
        cx.simulate_keystrokes("end");
        assert_eq!(
            state.read_with(cx, |state, _| state.selected_index()),
            Some(2)
        );
        cx.simulate_keystrokes("home");
        assert_eq!(
            state.read_with(cx, |state, _| state.selected_index()),
            Some(0)
        );
    }

    #[gpui::test]
    fn horizontal_carousel_dispatches_contextual_navigation_keys(cx: &mut gpui::TestAppContext) {
        assert_contextual_navigation_keys(cx, Axis::Horizontal);
    }

    #[gpui::test]
    fn vertical_carousel_dispatches_contextual_navigation_keys(cx: &mut gpui::TestAppContext) {
        assert_contextual_navigation_keys(cx, Axis::Vertical);
    }

    #[gpui::test]
    fn carousel_controls_accept_accessibility_labels_and_children(cx: &mut gpui::TestAppContext) {
        let state = cx.update(|cx| cx.new(|_| CarouselState::new(2)));
        let previous = CarouselPrevious::new(&state)
            .with_accessibility_label("Previous project")
            .child("Back");
        let next = CarouselNext::new(&state)
            .with_accessibility_label("Next project")
            .child("Forward");

        assert_eq!(
            previous.accessibility_label.as_deref(),
            Some("Previous project")
        );
        assert_eq!(next.accessibility_label.as_deref(), Some("Next project"));
        assert_eq!(previous.children.len(), 1);
        assert_eq!(next.children.len(), 1);
    }
}
