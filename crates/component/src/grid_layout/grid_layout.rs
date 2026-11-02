use std::sync::Arc;

use gpui::{
    AnyElement, App, AppContext as _, CursorStyle, ElementId, Empty, Entity,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement, Pixels, Render, RenderOnce,
    Role, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, px, relative,
};
use gpui_base::{Transition, transition};
use smallvec::SmallVec;

use super::{
    CONTEXT, GrowHeight, GrowWidth, MoveDown, MoveLeft, MoveRight, MoveUp, SelectNextItem,
    SelectPreviousItem, ShrinkHeight, ShrinkWidth,
    layout::{self, GridItem},
    state::{DragGridItem, GridDrag, GridGesture, GridLayoutState, GridMetrics},
};
use crate::{StyledExt as _, ThemeStyled as _, actions::Cancel, theme::ActiveTheme as _};

/// A grid whose items the user can drag and resize, the way dashboards and
/// widget boards are arranged.
///
/// The [`GridLayoutState`] owns where every item sits; this element pairs each
/// [`GridLayoutItem`] with the layout entry of the same id and draws it there.
/// An item without a layout entry is not drawn.
///
/// The grid is as wide as its container and as tall as its rows. Put it in a
/// scrollable region when the rows can outgrow the window.
///
/// ```ignore
/// GridLayout::new("dashboard", &state)
///     .item(GridLayoutItem::new("revenue").child(revenue_card))
///     .item(GridLayoutItem::new("orders").child(orders_card))
/// ```
///
/// Click an item to select it. With the grid focused, the arrow keys move the
/// selected item, Shift with the arrow keys resizes it, Tab and Shift-Tab move
/// the selection, and Escape cancels a drag.
#[derive(IntoElement)]
pub struct GridLayout {
    id: ElementId,
    state: Entity<GridLayoutState>,
    accessibility_label: SharedString,
    items: Vec<GridLayoutItem>,
    style: StyleRefinement,
}

impl GridLayout {
    pub fn new(id: impl Into<ElementId>, state: &Entity<GridLayoutState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            accessibility_label: "Grid layout".into(),
            items: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    /// Name the grid for assistive technology. Defaults to "Grid layout".
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = label.into();
        self
    }

    /// Add an item, drawn where the layout entry with its id says.
    pub fn item(mut self, item: GridLayoutItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add several items.
    pub fn items(mut self, items: impl IntoIterator<Item = GridLayoutItem>) -> Self {
        self.items.extend(items);
        self
    }
}

impl Styled for GridLayout {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// One item of a [`GridLayout`], paired with its layout entry by id.
///
/// The item fills the cells its entry spans. It paints nothing of its own, so
/// its content supplies the surface — usually a card or group box. Style the
/// item to round it to match, so the drag placeholder and focus ring follow the
/// same corners.
pub struct GridLayoutItem {
    id: SharedString,
    accessibility_label: Option<SharedString>,
    handle_only: bool,
    style: StyleRefinement,
    children: SmallVec<[AnyElement; 2]>,
}

impl GridLayoutItem {
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

    /// Move the item only from a [`GridDragHandle`] inside it, leaving the rest
    /// of its content free for its own pointer interaction.
    pub fn drag_handle_only(mut self, handle_only: bool) -> Self {
        self.handle_only = handle_only;
        self
    }
}

impl Styled for GridLayoutItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for GridLayoutItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// The part of an item the pointer moves it by, for an item built with
/// [`GridLayoutItem::drag_handle_only`] — a card's header, say.
///
/// ```ignore
/// GridLayoutItem::new("orders")
///     .drag_handle_only(true)
///     .child(GridDragHandle::new(&state, "orders").child("Orders"))
///     .child(orders_table)
/// ```
#[derive(IntoElement)]
pub struct GridDragHandle {
    state: Entity<GridLayoutState>,
    item: SharedString,
    style: StyleRefinement,
    children: SmallVec<[AnyElement; 2]>,
}

impl GridDragHandle {
    pub fn new(state: &Entity<GridLayoutState>, item: impl Into<SharedString>) -> Self {
        Self {
            state: state.clone(),
            item: item.into(),
            style: StyleRefinement::default(),
            children: SmallVec::new(),
        }
    }
}

impl Styled for GridDragHandle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for GridDragHandle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for GridDragHandle {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = ElementId::NamedChild(
            Arc::new(ElementId::from(self.item.clone())),
            "grid-drag-handle".into(),
        );
        div()
            .id(id)
            .refine_style(&self.style)
            .children(self.children)
            .when(
                self.state
                    .read(cx)
                    .is_gesture_allowed(&self.item, GridGesture::Move),
                |this| {
                    this.on_drag(
                        drag_payload(&self.state, &self.item, GridGesture::Move),
                        start_drag(&self.state),
                    )
                },
            )
    }
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
            state.begin_drag(drag.item.clone(), drag.gesture, window, cx)
        });
        cx.new(|_| GridDragPreview)
    }
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
    fn of(item: &GridItem, columns: usize, row_height: Pixels, gap: Pixels) -> Self {
        let columns = columns.max(1) as f32;
        Self {
            left: item.x as f32 / columns,
            width: item.w as f32 / columns,
            top: (row_height + gap) * item.y as f32,
            height: (row_height * item.h as f32 + gap * item.h.saturating_sub(1) as f32)
                .max(px(0.)),
        }
    }

    /// The frame of an item under the pointer: its start position plus the
    /// pointer's travel, kept inside the grid.
    fn following(drag: &GridDrag, metrics: GridMetrics, columns: usize) -> Option<Self> {
        let start = drag.start_item()?;
        let track = metrics.width + metrics.gap;
        if track <= px(0.) {
            return None;
        }
        let (mut left, mut top) = (metrics.left(start.x), metrics.top(start.y));
        let (mut width, mut height) = (metrics.span_width(start.w), metrics.span_height(start.h));
        match drag.gesture {
            GridGesture::Move => {
                left = (left + drag.delta.x).clamp(px(0.), (metrics.width - width).max(px(0.)));
                top = (top + drag.delta.y).max(px(0.));
            }
            GridGesture::Resize => {
                let (min_w, min_h) = start.clamp_size(1, 1, columns);
                let (max_w, _) = start.clamp_size(usize::MAX, 1, columns);
                width = (width + drag.delta.x)
                    .clamp(metrics.span_width(min_w), metrics.span_width(max_w));
                height = (height + drag.delta.y).max(metrics.span_height(min_h));
                if let Some(max_h) = start.max_h {
                    height = height.min(metrics.span_height(max_h));
                }
            }
        }
        Some(Self {
            left: left / track,
            width: (width + metrics.gap) / track,
            top,
            height,
        })
    }
}

impl RenderOnce for GridLayout {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let grid = state_id(&self.state);
        let columns = state.columns();
        let gap = state.resolved_gap(cx);
        let row_height = state.resolved_row_height(window);
        let displayed = state.displayed_items(cx).to_vec();
        let drag = state.active_drag(cx);
        let active = drag.map(|drag| (drag.item.clone(), drag.gesture));
        let following = drag.and_then(|drag| ItemFrame::following(drag, drag.metrics?, columns));
        let focus_handle = state.focus_handle().clone();
        let show_selection = state.is_keyboard_active() && focus_handle.is_focused(window);
        let selected = state.selected_item().cloned();
        let rows = layout::rows(&displayed);
        let height = (row_height * rows as f32 + gap * rows.saturating_sub(1) as f32).max(px(0.));

        let motion = cx.theme().motion_tokens();
        let settle = Transition::new(motion.duration_normal).easing(motion.easing_move.clone());

        let mut items = self.items;
        // The item under the pointer paints last, above the ones it displaces.
        if let Some((active_id, _)) = &active {
            items.sort_by_key(|item| &item.id == active_id);
        }

        let placeholder = active.as_ref().and_then(|(id, _)| {
            displayed
                .iter()
                .find(|item| &item.id == id)
                .map(|item| ItemFrame::of(item, columns, row_height, gap))
        });
        let placeholder_radius = items
            .iter()
            .find(|item| active.as_ref().is_some_and(|(id, _)| id == &item.id))
            .and_then(|item| item.style.corner_radii.top_left)
            .unwrap_or_else(|| cx.theme().radius_lg.into());

        let rendered = items
            .into_iter()
            .filter_map(|item| {
                let entry = displayed.iter().find(|entry| entry.id == item.id)?;
                let gesture = active
                    .as_ref()
                    .filter(|(id, _)| id == &item.id)
                    .map(|(_, gesture)| *gesture);
                let target = match (gesture, following) {
                    (Some(_), Some(frame)) => frame,
                    _ => ItemFrame::of(entry, columns, row_height, gap),
                };
                // The item under the pointer tracks it exactly; everything else,
                // and that item once released, eases into its cells.
                let policy = if gesture.is_some() {
                    Transition::new(std::time::Duration::ZERO)
                } else {
                    settle.clone()
                };
                let frame = animate_frame(&grid, &item.id, target, policy, window, cx);
                Some(render_item(
                    &self.state,
                    item,
                    entry,
                    frame,
                    gap,
                    gesture,
                    selected.as_ref() == Some(&entry.id),
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
                    |state, event: &gpui::DragMoveEvent<DragGridItem>, window, cx| {
                        if event.drag(cx).grid == cx.entity_id() {
                            state.drag_to(event.event.position, event.bounds, window, cx);
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
                // The column track: as wide as the grid plus one gap, so each
                // column owns an equal fraction of it and every item reaches
                // its cells with relative units alone. Each item gives the
                // trailing gap back as padding.
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .bottom_0()
                        .right(-gap)
                        .when_some(placeholder, |track, frame| {
                            track.child(
                                div()
                                    .absolute()
                                    .left(relative(frame.left))
                                    .w(relative(frame.width))
                                    .top(frame.top)
                                    .h(frame.height)
                                    .pr(gap)
                                    .child(
                                        div()
                                            .size_full()
                                            .rounded(placeholder_radius)
                                            .bg(cx.theme().primary.opacity(0.08))
                                            .border_1()
                                            .border_dashed()
                                            .border_color(cx.theme().primary.opacity(0.5)),
                                    ),
                            )
                        })
                        .children(rendered),
                ),
        )
    }
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

#[allow(clippy::too_many_arguments)]
fn render_item(
    state: &Entity<GridLayoutState>,
    item: GridLayoutItem,
    entry: &GridItem,
    frame: ItemFrame,
    gap: Pixels,
    gesture: Option<GridGesture>,
    selected: bool,
    show_selection: bool,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let snapshot = state.read(cx);
    let movable = !item.handle_only && snapshot.is_gesture_allowed(&entry.id, GridGesture::Move);
    let resizable = snapshot.is_gesture_allowed(&entry.id, GridGesture::Resize);
    let group: SharedString = format!("grid-layout-item-{}", entry.id).into();
    let item_id = ElementId::NamedChild(
        Arc::new(ElementId::from(entry.id.clone())),
        "grid-layout-item".into(),
    );

    div()
        .id(item_id)
        .role(Role::Group)
        .when_some(item.accessibility_label, |this, label| {
            this.aria_label(label)
        })
        .aria_selected(selected)
        .absolute()
        .left(relative(frame.left))
        .w(relative(frame.width))
        .top(frame.top)
        .h(frame.height)
        .pr(gap)
        .on_mouse_down(MouseButton::Left, {
            let state = state.clone();
            let id = entry.id.clone();
            move |event, window, cx| {
                state.update(cx, |state, cx| state.press(&id, event.position, window, cx))
            }
        })
        .when(movable, |this| {
            this.on_drag(
                drag_payload(state, &entry.id, GridGesture::Move),
                start_drag(state),
            )
        })
        .child(
            div()
                .group(group.clone())
                .relative()
                .size_full()
                .refine_style(&item.style)
                .when(gesture.is_some(), |this| this.shadow_lg())
                .children(item.children)
                .when(resizable, |this| {
                    this.child(resize_grip(state, &entry.id, group, gesture.is_some(), cx))
                })
                .when(selected && show_selection, |this| {
                    this.focus_ring_style(window, cx)
                }),
        )
        .into_any_element()
}

/// The trailing-bottom corner the pointer resizes an item by.
fn resize_grip(
    state: &Entity<GridLayoutState>,
    item: &SharedString,
    group: SharedString,
    engaged: bool,
    cx: &App,
) -> impl IntoElement {
    let color = cx.theme().muted_foreground;
    div()
        .id(ElementId::NamedChild(
            Arc::new(ElementId::from(item.clone())),
            "grid-layout-resize".into(),
        ))
        .absolute()
        .right_0()
        .bottom_0()
        .size_4()
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
        .child(
            div()
                .absolute()
                .right(px(3.))
                .bottom(px(3.))
                .size(px(6.))
                .border_r_2()
                .border_b_2()
                .border_color(color.opacity(if engaged { 0.9 } else { 0.35 }))
                .group_hover(group, |this| this.border_color(color.opacity(0.9))),
        )
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{
        Context, Modifiers, MouseButton, Point, TestAppContext, VisualTestContext, point, rems,
    };

    use super::*;
    use crate::grid_layout::GridLayoutEvent;

    /// Twelve 90px columns with 10px gaps: column `n` starts at `100 * n`.
    const WIDTH: Pixels = px(1190.);

    struct Harness {
        state: Entity<GridLayoutState>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(WIDTH).child(
                GridLayout::new("grid", &self.state)
                    .item(GridLayoutItem::new("a").child(div().size_full()))
                    .item(GridLayoutItem::new("b").child(div().size_full())),
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
                    .with_gap(px(10.))
                    .with_row_height(rems(2.5))
                    .with_items([
                        GridItem::new("a", 0, 0, 4, 2),
                        GridItem::new("b", 4, 0, 4, 2),
                    ])
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

    fn place(state: &Entity<GridLayoutState>, cx: &mut VisualTestContext, id: &str) -> GridItem {
        state.read_with(cx, |state, _| state.item(id).unwrap().clone())
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
        assert_eq!(place(&state, cx, "a").x, 0);
        assert!(events.borrow().is_empty());

        release(cx, point(px(450.), px(30.)));
        let (a, b) = (place(&state, cx, "a"), place(&state, cx, "b"));
        assert_eq!((a.x, a.y), (4, 0));
        assert_eq!((b.x, b.y), (4, 2));
        assert_eq!(events.borrow().len(), 1);
        assert!(state.read_with(cx, |state, _| state.active_item().is_none()));
    }

    #[gpui::test]
    fn dragging_the_corner_resizes_an_item(cx: &mut TestAppContext) {
        let (state, events, cx) = harness(cx);
        let row_height = cx.update(|window, _| rems(2.5).to_pixels(window.rem_size()));
        // Item `b` spans 400..790 across and two rows down.
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

        let b = place(&state, cx, "b");
        assert_eq!((b.x, b.w, b.h), (4, 6, 2));
        assert_eq!(place(&state, cx, "a").x, 0);
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

        assert_eq!(place(&state, cx, "a").x, 0);
        assert!(events.borrow().is_empty());
    }

    #[gpui::test]
    fn arrow_keys_move_the_clicked_item(cx: &mut TestAppContext) {
        let (state, events, cx) = harness(cx);

        cx.simulate_click(point(px(50.), px(30.)), Modifiers::default());
        draw(cx);
        cx.simulate_keystrokes("right shift-down");
        draw(cx);

        let a = place(&state, cx, "a");
        assert_eq!((a.x, a.h), (1, 3));
        assert_eq!(events.borrow().len(), 2);
    }

    #[gpui::test]
    fn test_grid_layout_builder(cx: &mut TestAppContext) {
        let state = cx.update(|cx| cx.new(|cx| GridLayoutState::new(cx)));
        let grid = GridLayout::new("grid", &state)
            .item(GridLayoutItem::new("a").drag_handle_only(true))
            .items([GridLayoutItem::new("b"), GridLayoutItem::new("c")])
            .p_2();
        assert_eq!(grid.items.len(), 3);
        assert!(grid.items[0].handle_only);
        assert!(grid.style.padding.left.is_some());
    }
}
