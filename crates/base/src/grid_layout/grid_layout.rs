use std::{rc::Rc, sync::Arc, time::Duration};

use gpui::{
    AbsoluteLength, AnyElement, App, AppContext as _, CursorStyle, DefiniteLength, Div, ElementId,
    Empty, Entity, InteractiveElement as _, IntoElement, MouseButton, ParentElement, Pixels,
    Refineable as _, Rems, Render, RenderOnce, Role, SharedString, Size, Stateful,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, px, relative, rems, size,
};
use smallvec::SmallVec;

use super::{
    CONTEXT, GrowHeight, GrowWidth, MoveDown, MoveLeft, MoveRight, MoveUp, SelectNextItem,
    SelectPreviousItem, ShrinkHeight, ShrinkWidth,
    layout::{self, GridArea},
    state::{DragGridItem, GridDrag, GridGesture, GridLayoutState, GridMetrics},
};
use crate::{StyledExt as _, Transition, actions::Cancel, state_style::StateStyle, transition};

/// The side of the square, at an item's trailing-bottom corner, that the
/// pointer resizes the item by.
const RESIZE_HANDLE_SIZE: Pixels = px(16.);

/// Draws one part of a grid layout — the drop placeholder or a resize handle.
type PartRenderer = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// A grid whose items the user drags and resizes, the way dashboards and
/// widget boards are arranged.
///
/// The [`GridLayoutState`] owns where every item sits; this element pairs each
/// [`GridItem`] with the [`GridArea`] of the same id and places it there. An
/// item without an area is not drawn.
///
/// Like every Base control it paints nothing of its own. Gaps come from the
/// element's own `gap`, `gap_x` and `gap_y`, as on a CSS grid; the look of the
/// drop placeholder, the resize handle, and the dragged and selected items,
/// and the motion items settle with, come from the builders below. GPUI
/// Component's `grid_layout` supplies all of them from the theme.
///
/// ```ignore
/// GridLayout::new("dashboard", &state)
///     .row_height(rems(2.5))
///     .gap_3()
///     .item(GridItem::new("revenue").child(revenue_card))
///     .item(GridItem::new("orders").child(orders_card))
/// ```
///
/// The grid is as wide as its container and as tall as its rows. Put it in a
/// scrollable region when the rows can outgrow the window.
///
/// Click an item to select it. With the grid focused, the arrow keys move the
/// selected item, Shift with the arrow keys resizes it, Tab and Shift-Tab move
/// the selection — and out of the grid past either end — and Escape cancels a
/// drag.
#[derive(IntoElement)]
pub struct GridLayout {
    id: ElementId,
    state: Entity<GridLayoutState>,
    accessibility_label: SharedString,
    items: Vec<GridItem>,
    style: StyleRefinement,
    row_height: Rems,
    transition: Option<Transition>,
    dragging_item: StyleRefinement,
    selected_item: StyleRefinement,
    placeholder: Option<PartRenderer>,
    resize_handle: Option<PartRenderer>,
}

impl GridLayout {
    pub fn new(id: impl Into<ElementId>, state: &Entity<GridLayoutState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            accessibility_label: "Grid layout".into(),
            items: Vec::new(),
            style: StyleRefinement::default(),
            row_height: rems(2.5),
            transition: None,
            dragging_item: StyleRefinement::default(),
            selected_item: StyleRefinement::default(),
            placeholder: None,
            resize_handle: None,
        }
    }

    /// Name the grid for assistive technology. Defaults to "Grid layout".
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = label.into();
        self
    }

    /// Add an item, placed where the area with its id says.
    pub fn item(mut self, item: GridItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add several items.
    pub fn items(mut self, items: impl IntoIterator<Item = GridItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// The height of one row. Defaults to `2.5rem`, so rows follow the
    /// application's zoom.
    pub fn row_height(mut self, row_height: Rems) -> Self {
        self.row_height = row_height;
        self
    }

    /// How items ease into their cells when the layout changes. Without one
    /// they move at once; the item under the pointer always tracks it exactly.
    pub fn transition(mut self, transition: Transition) -> Self {
        self.transition = Some(transition);
        self
    }

    /// The style of the item under the pointer, layered over its own.
    pub fn dragging_item(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.dragging_item = build(StateStyle::default()).into_refinement();
        self
    }

    /// The style of the selected item while the grid has keyboard focus,
    /// layered over its own. A pointer press selects without showing it.
    pub fn selected_item(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.selected_item = build(StateStyle::default()).into_refinement();
        self
    }

    /// What marks the cells the dragged item will land in. It fills those
    /// cells; without one, nothing marks them.
    pub fn placeholder(
        mut self,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.placeholder = Some(Rc::new(render));
        self
    }

    /// What a resizable item shows in the trailing-bottom corner the pointer
    /// resizes it by. The corner works without one.
    pub fn resize_handle(
        mut self,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.resize_handle = Some(Rc::new(render));
        self
    }
}

impl Styled for GridLayout {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// One item of a [`GridLayout`], paired with the [`GridArea`] of the same id.
///
/// The item fills the cells its area covers. Style it to match its content —
/// round it like the card inside it, say — so the dragged and selected styles
/// follow the same corners.
pub struct GridItem {
    id: SharedString,
    accessibility_label: Option<SharedString>,
    handle_only: bool,
    style: StyleRefinement,
    children: SmallVec<[AnyElement; 2]>,
}

impl GridItem {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            accessibility_label: None,
            handle_only: false,
            style: StyleRefinement::default(),
            children: SmallVec::new(),
        }
    }

    /// Name the item for assistive technology — usually its visible title.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Move the item only from a [`grid_drag_handle`] inside it, leaving the
    /// rest of its content free for its own pointer interaction.
    pub fn drag_handle_only(mut self, handle_only: bool) -> Self {
        self.handle_only = handle_only;
        self
    }
}

impl Styled for GridItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for GridItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// The part of an item the pointer moves it by, for an item built with
/// [`GridItem::drag_handle_only`] — a card's header, say.
///
/// ```ignore
/// GridItem::new("orders")
///     .drag_handle_only(true)
///     .child(grid_drag_handle(&state, "orders").child("Orders"))
///     .child(orders_table)
/// ```
pub fn grid_drag_handle(
    state: &Entity<GridLayoutState>,
    item: impl Into<SharedString>,
) -> Stateful<Div> {
    let item = item.into();
    div()
        .id(ElementId::NamedChild(
            Arc::new(ElementId::from(item.clone())),
            "grid-drag-handle".into(),
        ))
        .on_drag(
            drag_payload(state, &item, GridGesture::Move),
            start_drag(state),
        )
}

/// The view GPUI draws under the pointer during a drag. The dragged item
/// itself follows the pointer inside the grid, so this draws nothing.
struct GridDragPreview;

impl Render for GridDragPreview {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        Empty
    }
}

fn drag_payload(
    state: &Entity<GridLayoutState>,
    item: &SharedString,
    gesture: GridGesture,
) -> DragGridItem {
    DragGridItem {
        grid: state.entity_id(),
        item: item.clone(),
        gesture,
    }
}

fn start_drag(
    state: &Entity<GridLayoutState>,
) -> impl Fn(&DragGridItem, gpui::Point<Pixels>, &mut Window, &mut App) -> Entity<GridDragPreview>
+ 'static {
    let state = state.clone();
    move |drag, _, window, cx| {
        state.update(cx, |state, cx| {
            // A handle may outlive the permission to move: then the drag
            // carries nothing and the layout ignores it.
            if state.is_gesture_allowed(&drag.item, drag.gesture) {
                state.begin_drag(drag.item.clone(), drag.gesture, window, cx)
            }
        });
        cx.new(|_| GridDragPreview)
    }
}

/// Resolve the grid's gap from its own style: the column gap across, the row
/// gap down. A gap given as a fraction has nothing to be a fraction of here,
/// so it counts as none.
fn resolve_gap(style: &StyleRefinement, rem_size: Pixels) -> Size<Pixels> {
    let resolve = |length: Option<DefiniteLength>| match length {
        Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(pixels))) => pixels,
        Some(DefiniteLength::Absolute(AbsoluteLength::Rems(rems))) => rems.to_pixels(rem_size),
        _ => px(0.),
    };
    size(resolve(style.gap.width), resolve(style.gap.height))
}

/// Where an item is drawn, as fractions of the column track for its
/// horizontal extent and pixels for its vertical one.
///
/// Fractions keep idle items exact at any container width without measuring
/// it; see [`GridLayout::render`] for the track that makes them work.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ItemFrame {
    left: f32,
    width: f32,
    top: Pixels,
    height: Pixels,
}

impl ItemFrame {
    fn of(area: &GridArea, columns: usize, row_height: Pixels, gap: Size<Pixels>) -> Self {
        let columns = columns.max(1) as f32;
        Self {
            left: area.col as f32 / columns,
            width: area.col_span as f32 / columns,
            top: (row_height + gap.height) * area.row as f32,
            height: (row_height * area.row_span as f32
                + gap.height * area.row_span.saturating_sub(1) as f32)
                .max(px(0.)),
        }
    }

    /// The frame of an item under the pointer: its start position plus the
    /// pointer's travel, kept inside the grid.
    fn following(drag: &GridDrag, metrics: GridMetrics, columns: usize) -> Option<Self> {
        let start = drag.start_area()?;
        let track = metrics.width + metrics.gap.width;
        if track <= px(0.) {
            return None;
        }
        let (mut left, mut top) = (metrics.left(start.col), metrics.top(start.row));
        let (mut width, mut height) = (
            metrics.span_width(start.col_span),
            metrics.span_height(start.row_span),
        );
        match drag.gesture {
            GridGesture::Move => {
                left = (left + drag.delta.x).clamp(px(0.), (metrics.width - width).max(px(0.)));
                top = (top + drag.delta.y).max(px(0.));
            }
            GridGesture::Resize => {
                let (min_cols, min_rows) = start.clamp_size(1, 1, columns);
                let (max_cols, _) = start.clamp_size(usize::MAX, 1, columns);
                width = (width + drag.delta.x)
                    .clamp(metrics.span_width(min_cols), metrics.span_width(max_cols));
                height = (height + drag.delta.y).max(metrics.span_height(min_rows));
                if let Some(max_rows) = start.max_row_span {
                    height = height.min(metrics.span_height(max_rows));
                }
            }
        }
        Some(Self {
            left: left / track,
            width: (width + metrics.gap.width) / track,
            top,
            height,
        })
    }
}

impl RenderOnce for GridLayout {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let grid = state_id(&self.state);
        let columns = state.column_count();
        let gap = resolve_gap(&self.style, window.rem_size());
        let row_height = self.row_height.to_pixels(window.rem_size());
        let displayed = state.displayed_areas(cx).to_vec();
        let drag = state.active_drag(cx);
        let active = drag.map(|drag| (drag.item.clone(), drag.gesture));
        let following = drag.and_then(|drag| ItemFrame::following(drag, drag.metrics?, columns));
        let focus_handle = state.focus_handle().clone();
        let show_selection = state.is_keyboard_active() && focus_handle.is_focused(window);
        let selected = state.selected_item().cloned();
        let rows = layout::rows(&displayed);
        let height =
            (row_height * rows as f32 + gap.height * rows.saturating_sub(1) as f32).max(px(0.));
        let settle = self
            .transition
            .clone()
            .unwrap_or_else(|| Transition::new(Duration::ZERO));

        let mut items = self.items;
        // The item under the pointer paints last, above the ones it displaces.
        if let Some((active_id, _)) = &active {
            items.sort_by_key(|item| &item.id == active_id);
        }

        let placeholder = active.as_ref().and_then(|(id, _)| {
            displayed
                .iter()
                .find(|area| &area.id == id)
                .map(|area| ItemFrame::of(area, columns, row_height, gap))
        });

        let parts = ItemParts {
            state: &self.state,
            gap,
            dragging_item: &self.dragging_item,
            selected_item: &self.selected_item,
            resize_handle: self.resize_handle.as_ref(),
        };
        let rendered = items
            .into_iter()
            .filter_map(|item| {
                let area = displayed.iter().find(|area| area.id == item.id)?;
                let gesture = active
                    .as_ref()
                    .filter(|(id, _)| id == &item.id)
                    .map(|(_, gesture)| *gesture);
                let target = match (gesture, following) {
                    (Some(_), Some(frame)) => frame,
                    _ => ItemFrame::of(area, columns, row_height, gap),
                };
                // The item under the pointer tracks it exactly; everything else,
                // and that item once released, eases into its cells.
                let policy = if gesture.is_some() {
                    Transition::new(Duration::ZERO)
                } else {
                    settle.clone()
                };
                let frame = animate_frame(&grid, &item.id, target, policy, window, cx);
                let selected = selected.as_ref() == Some(&area.id);
                Some(parts.render(
                    item,
                    area,
                    frame,
                    gesture,
                    selected,
                    show_selection,
                    window,
                    cx,
                ))
            })
            .collect::<Vec<_>>();

        div().id(self.id).w_full().refine_style(&self.style).child(
            div()
                .id("grid-layout-surface")
                .role(Role::Group)
                .aria_label(self.accessibility_label)
                .key_context(CONTEXT)
                .track_focus(&focus_handle)
                .relative()
                .w_full()
                .h(height)
                .on_drag_move(window.listener_for(
                    &self.state,
                    move |state, event: &gpui::DragMoveEvent<DragGridItem>, _, cx| {
                        if event.drag(cx).grid == cx.entity_id() {
                            state.drag_to(event.event.position, event.bounds, row_height, gap, cx);
                        }
                    },
                ))
                // A drop anywhere ends the gesture, inside the grid or not;
                // capturing keeps a child that stops propagation from
                // swallowing the release.
                .capture_any_mouse_up(
                    window.listener_for(&self.state, |state, _, _, cx| state.end_drag(cx)),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    window.listener_for(&self.state, |state, _, _, cx| state.end_drag(cx)),
                )
                .on_action(
                    window.listener_for(&self.state, |state, _: &Cancel, window, cx| {
                        if !state.cancel_drag(window, cx) {
                            cx.propagate();
                        }
                    }),
                )
                .on_action(keyboard(
                    &self.state,
                    window,
                    |state: &mut GridLayoutState, _: &MoveLeft, cx| state.nudge_selected(-1, 0, cx),
                ))
                .on_action(keyboard(&self.state, window, |state, _: &MoveRight, cx| {
                    state.nudge_selected(1, 0, cx)
                }))
                .on_action(keyboard(&self.state, window, |state, _: &MoveUp, cx| {
                    state.nudge_selected(0, -1, cx)
                }))
                .on_action(keyboard(&self.state, window, |state, _: &MoveDown, cx| {
                    state.nudge_selected(0, 1, cx)
                }))
                .on_action(keyboard(
                    &self.state,
                    window,
                    |state, _: &ShrinkWidth, cx| state.resize_selected(-1, 0, cx),
                ))
                .on_action(keyboard(&self.state, window, |state, _: &GrowWidth, cx| {
                    state.resize_selected(1, 0, cx)
                }))
                .on_action(keyboard(
                    &self.state,
                    window,
                    |state, _: &ShrinkHeight, cx| state.resize_selected(0, -1, cx),
                ))
                .on_action(keyboard(
                    &self.state,
                    window,
                    |state, _: &GrowHeight, cx| state.resize_selected(0, 1, cx),
                ))
                .on_action(keyboard(
                    &self.state,
                    window,
                    |state, _: &SelectNextItem, cx| state.select_adjacent(true, cx),
                ))
                .on_action(keyboard(
                    &self.state,
                    window,
                    |state, _: &SelectPreviousItem, cx| state.select_adjacent(false, cx),
                ))
                // The column track: as wide as the grid plus one column gap, so
                // each column owns an equal fraction of it and every item
                // reaches its cells with relative units alone. Each item gives
                // the trailing gap back as padding.
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .bottom_0()
                        .right(-gap.width)
                        .when_some(
                            placeholder.zip(self.placeholder.as_ref()),
                            |track, (frame, render)| {
                                track.child(
                                    place(div(), frame, gap)
                                        .child(div().size_full().child(render(window, cx))),
                                )
                            },
                        )
                        .children(rendered),
                ),
        )
    }
}

/// Position an element over the cells `frame` describes.
fn place<E: Styled>(element: E, frame: ItemFrame, gap: Size<Pixels>) -> E {
    element
        .absolute()
        .left(relative(frame.left))
        .w(relative(frame.width))
        .top(frame.top)
        .h(frame.height)
        .pr(gap.width)
}

/// A keyboard command on the selected item, which passes the key on when it
/// does not apply — at an edge, or with nothing selected.
fn keyboard<A: gpui::Action>(
    state: &Entity<GridLayoutState>,
    window: &Window,
    command: impl Fn(&mut GridLayoutState, &A, &mut gpui::Context<GridLayoutState>) -> bool + 'static,
) -> impl Fn(&A, &mut Window, &mut App) + 'static {
    window.listener_for(state, move |state, action: &A, _, cx| {
        if !command(state, action, cx) {
            cx.propagate();
        }
    })
}

fn state_id(state: &Entity<GridLayoutState>) -> ElementId {
    ElementId::NamedChild(
        Arc::new(ElementId::View(state.entity_id())),
        "grid-layout".into(),
    )
}

fn animate_frame(
    grid: &ElementId,
    item: &SharedString,
    target: ItemFrame,
    policy: Transition,
    window: &mut Window,
    cx: &mut App,
) -> ItemFrame {
    let key = ElementId::NamedChild(Arc::new(grid.clone()), item.clone());
    ItemFrame {
        left: transition(
            (key.clone(), "left"),
            target.left,
            policy.clone(),
            window,
            cx,
        ),
        width: transition(
            (key.clone(), "width"),
            target.width,
            policy.clone(),
            window,
            cx,
        ),
        top: transition((key.clone(), "top"), target.top, policy.clone(), window, cx),
        height: transition((key, "height"), target.height, policy, window, cx),
    }
}

/// What every item of one grid is rendered with.
struct ItemParts<'a> {
    state: &'a Entity<GridLayoutState>,
    gap: Size<Pixels>,
    dragging_item: &'a StyleRefinement,
    selected_item: &'a StyleRefinement,
    resize_handle: Option<&'a PartRenderer>,
}

impl ItemParts<'_> {
    #[allow(clippy::too_many_arguments)]
    fn render(
        &self,
        item: GridItem,
        area: &GridArea,
        frame: ItemFrame,
        gesture: Option<GridGesture>,
        selected: bool,
        show_selection: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let state = self.state;
        let snapshot = state.read(cx);
        let movable = !item.handle_only && snapshot.is_gesture_allowed(&area.id, GridGesture::Move);
        let resizable = snapshot.is_gesture_allowed(&area.id, GridGesture::Resize);
        let item_id = ElementId::NamedChild(
            Arc::new(ElementId::from(area.id.clone())),
            "grid-layout-item".into(),
        );
        let mut surface_style = item.style;
        if selected && show_selection {
            surface_style.refine(self.selected_item);
        }
        if gesture.is_some() {
            surface_style.refine(self.dragging_item);
        }

        place(div().id(item_id), frame, self.gap)
            .role(Role::Group)
            .when_some(item.accessibility_label, |this, label| {
                this.aria_label(label)
            })
            .aria_selected(selected)
            .on_mouse_down(MouseButton::Left, {
                let state = state.clone();
                let id = area.id.clone();
                move |event, window, cx| {
                    state.update(cx, |state, cx| state.press(&id, event.position, window, cx))
                }
            })
            .when(movable, |this| {
                this.on_drag(
                    drag_payload(state, &area.id, GridGesture::Move),
                    start_drag(state),
                )
            })
            .child(
                div()
                    .relative()
                    .size_full()
                    .refine_style(&surface_style)
                    .children(item.children)
                    .when(resizable, |this| {
                        this.child(self.resize_corner(&area.id, window, cx))
                    }),
            )
            .into_any_element()
    }

    /// The trailing-bottom corner the pointer resizes an item by.
    fn resize_corner(
        &self,
        item: &SharedString,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let state = self.state;
        div()
            .id(ElementId::NamedChild(
                Arc::new(ElementId::from(item.clone())),
                "grid-layout-resize".into(),
            ))
            .absolute()
            .right_0()
            .bottom_0()
            .size(RESIZE_HANDLE_SIZE)
            .cursor(CursorStyle::ResizeUpLeftDownRight)
            // The press resizes; it must not also begin the item's move.
            .on_mouse_down(MouseButton::Left, {
                let state = state.clone();
                let id = item.clone();
                move |event, window, cx| {
                    cx.stop_propagation();
                    state.update(cx, |state, cx| state.press(&id, event.position, window, cx))
                }
            })
            .on_drag(
                drag_payload(state, item, GridGesture::Resize),
                start_drag(state),
            )
            .when_some(self.resize_handle, |this, render| {
                this.child(render(window, cx))
            })
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{Context, Modifiers, Point, TestAppContext, VisualTestContext, point};

    use super::*;
    use crate::grid_layout::{GridLayoutEvent, layout::tests::area};

    /// Twelve 90px columns with 10px gaps: column `n` starts at `100 * n`.
    const WIDTH: Pixels = px(1190.);

    struct Harness {
        state: Entity<GridLayoutState>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(WIDTH).child(
                GridLayout::new("grid", &self.state)
                    .gap(px(10.))
                    .item(GridItem::new("a").child(div().size_full()))
                    .item(GridItem::new("b").child(div().size_full())),
            )
        }
    }

    fn harness(
        cx: &mut TestAppContext,
    ) -> (
        Entity<GridLayoutState>,
        Rc<RefCell<Vec<GridLayoutEvent>>>,
        &mut VisualTestContext,
    ) {
        cx.update(crate::init);
        let state = cx.update(|cx| {
            cx.new(|cx| {
                GridLayoutState::new(cx)
                    .default_areas([area("a", 0, 0, 4, 2), area("b", 4, 0, 4, 2)])
            })
        });
        let events = Rc::new(RefCell::new(Vec::new()));
        cx.update(|cx| {
            let events = events.clone();
            cx.subscribe(&state, move |_, event: &GridLayoutEvent, _| {
                events.borrow_mut().push(event.clone())
            })
            .detach();
        });
        let (_, cx) = cx.add_window_view({
            let state = state.clone();
            move |_, _| Harness { state }
        });
        draw(cx);
        (state, events, cx)
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn drag(cx: &mut VisualTestContext, from: Point<Pixels>, path: &[Point<Pixels>]) {
        cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
        draw(cx);
        for &to in path {
            cx.simulate_mouse_move(to, Some(MouseButton::Left), Modifiers::default());
            draw(cx);
        }
    }

    fn release(cx: &mut VisualTestContext, at: Point<Pixels>) {
        cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::default());
        draw(cx);
    }

    fn place_of(state: &Entity<GridLayoutState>, cx: &mut VisualTestContext, id: &str) -> GridArea {
        state.read_with(cx, |state, _| state.area(id).unwrap().clone())
    }

    #[gpui::test]
    fn dragging_an_item_moves_it_and_commits_on_release(cx: &mut TestAppContext) {
        let (state, events, cx) = harness(cx);

        drag(
            cx,
            point(px(50.), px(30.)),
            &[point(px(60.), px(30.)), point(px(450.), px(30.))],
        );
        // While the pointer is down, only the preview changes.
        assert_eq!(place_of(&state, cx, "a").col, 0);
        assert!(events.borrow().is_empty());

        release(cx, point(px(450.), px(30.)));
        let (a, b) = (place_of(&state, cx, "a"), place_of(&state, cx, "b"));
        assert_eq!((a.col, a.row), (4, 0));
        assert_eq!((b.col, b.row), (4, 2));
        assert_eq!(events.borrow().len(), 1);
        assert!(state.read_with(cx, |state, _| state.active_item().is_none()));
    }

    #[gpui::test]
    fn dragging_the_corner_resizes_an_item(cx: &mut TestAppContext) {
        let (state, events, cx) = harness(cx);
        let row_height = cx.update(|window, _| rems(2.5).to_pixels(window.rem_size()));
        // Item `b` spans 400..790 across and two rows, with one gap, down.
        let corner = point(px(784.), row_height * 2. + px(4.));

        drag(
            cx,
            corner,
            &[
                corner + point(px(10.), px(0.)),
                corner + point(px(200.), px(0.)),
            ],
        );
        release(cx, corner + point(px(200.), px(0.)));

        let b = place_of(&state, cx, "b");
        assert_eq!((b.col, b.col_span, b.row_span), (4, 6, 2));
        assert_eq!(place_of(&state, cx, "a").col, 0);
        assert_eq!(events.borrow().len(), 1);
    }

    #[gpui::test]
    fn escape_cancels_a_drag(cx: &mut TestAppContext) {
        let (state, events, cx) = harness(cx);

        drag(
            cx,
            point(px(50.), px(30.)),
            &[point(px(60.), px(30.)), point(px(450.), px(30.))],
        );
        assert!(state.read_with(cx, |state, _| state.active_item().is_some()));
        cx.simulate_keystrokes("escape");
        draw(cx);
        release(cx, point(px(450.), px(30.)));

        assert_eq!(place_of(&state, cx, "a").col, 0);
        assert!(events.borrow().is_empty());
    }

    #[gpui::test]
    fn arrow_keys_move_the_clicked_item_and_tab_leaves_the_grid(cx: &mut TestAppContext) {
        let (state, events, cx) = harness(cx);

        cx.simulate_click(point(px(50.), px(30.)), Modifiers::default());
        draw(cx);
        cx.simulate_keystrokes("right shift-down");
        draw(cx);

        let a = place_of(&state, cx, "a");
        assert_eq!((a.col, a.row_span), (1, 3));
        assert_eq!(events.borrow().len(), 2);

        // Tab reaches `b`, and the next Tab is not the grid's to handle.
        cx.simulate_keystrokes("tab");
        assert_eq!(
            state.read_with(cx, |state, _| state.selected_item().cloned()),
            Some("b".into())
        );
        cx.simulate_keystrokes("tab");
        assert_eq!(
            state.read_with(cx, |state, _| state.selected_item().cloned()),
            Some("b".into())
        );
    }

    #[gpui::test]
    fn test_grid_layout_builder(cx: &mut TestAppContext) {
        let state = cx.update(|cx| cx.new(|cx| GridLayoutState::new(cx)));
        let grid = GridLayout::new("grid", &state)
            .row_height(rems(3.))
            .gap_x(px(8.))
            .gap_y(px(12.))
            .dragging_item(|style| style.opacity(0.5))
            .selected_item(|style| style.opacity(0.75))
            .placeholder(|_, _| div().into_any_element())
            .resize_handle(|_, _| div().into_any_element())
            .item(GridItem::new("a").drag_handle_only(true))
            .items([GridItem::new("b"), GridItem::new("c")]);
        assert_eq!(grid.items.len(), 3);
        assert!(grid.items[0].handle_only);
        assert_eq!(grid.row_height, rems(3.));
        assert_eq!(grid.dragging_item.opacity, Some(0.5));
        assert_eq!(grid.selected_item.opacity, Some(0.75));
        assert!(grid.placeholder.is_some() && grid.resize_handle.is_some());
        assert_eq!(resolve_gap(&grid.style, px(16.)), size(px(8.), px(12.)));
    }
}
