use std::rc::Rc;

use gpui::{
    Anchor, App, Bounds, HitboxBehavior, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement as _, Pixels, Point, RenderOnce, Styled as _, TouchDragEvent,
    TouchPhase, Window, canvas, deferred, fill, point, px, size,
};
use gpui_base::{Positioner, SelectionEdge};

use crate::ActiveTheme as _;

/// How wide the finger may miss the knob and still take it.
pub(super) const HANDLE_HIT_SIZE: Pixels = px(44.);
/// The knob's diameter.
const KNOB_SIZE: Pixels = px(10.);
/// The bar down the caret line.
const BAR_WIDTH: Pixels = px(2.);
/// Room the knob takes beyond the line, kept clear by the edit menu.
pub(super) const KNOB_EXTENT: Pixels = px(12.);

/// What a handle's owner gets told as the finger moves it.
pub(crate) type DragHandler =
    Rc<dyn Fn(SelectionEdge, TouchPhase, Point<Pixels>, &mut Window, &mut App)>;
/// Where a surface was painted this frame, for owners that must protect it.
pub(crate) type SurfaceHandler = Rc<dyn Fn(Bounds<Pixels>, &mut Window, &mut App)>;

/// A grab handle at one end of a touch selection: a bar down the caret line
/// with a knob at its outer end, the start's above and the end's below.
///
/// The handle claims the touch drag that begins on it before the window can
/// take the drag for panning, and blocks the mouse so a stray tap on it does
/// not reach the text underneath. A mouse can drag it too, which is how it is
/// exercised without a touch screen.
#[derive(IntoElement)]
pub(crate) struct SelectionHandle {
    edge: SelectionEdge,
    /// The caret line box the handle marks, in window coordinates.
    caret: Bounds<Pixels>,
    dragging: bool,
    on_drag: DragHandler,
    on_paint: Option<SurfaceHandler>,
}

impl SelectionHandle {
    pub(crate) fn new(edge: SelectionEdge, caret: Bounds<Pixels>, on_drag: DragHandler) -> Self {
        Self {
            edge,
            caret,
            dragging: false,
            on_drag,
            on_paint: None,
        }
    }

    /// Marks the handle as the one the finger currently holds, so the drag
    /// keeps reaching it wherever the finger goes.
    pub(crate) fn dragging(mut self, dragging: bool) -> Self {
        self.dragging = dragging;
        self
    }

    pub(crate) fn on_paint(mut self, on_paint: SurfaceHandler) -> Self {
        self.on_paint = Some(on_paint);
        self
    }

    /// The touch target, centered on the caret and reaching out past the knob.
    fn hit_bounds(&self) -> Bounds<Pixels> {
        let top = match self.edge {
            SelectionEdge::Start => self.caret.top() - KNOB_EXTENT,
            SelectionEdge::End => self.caret.top(),
        };
        Bounds::new(
            point(self.caret.left() - HANDLE_HIT_SIZE / 2., top),
            size(HANDLE_HIT_SIZE, self.caret.size.height + KNOB_EXTENT),
        )
    }
}

impl RenderOnce for SelectionHandle {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let hit = self.hit_bounds();
        let edge = self.edge;
        let caret = self.caret;
        let dragging = self.dragging;
        let on_drag = self.on_drag;
        let on_paint = self.on_paint;
        deferred(
            Positioner::corner(Anchor::TopLeft, hit.origin)
                .margin(px(0.))
                .child(
                    canvas(
                        move |bounds, window, _| {
                            window.insert_hitbox(bounds, HitboxBehavior::BlockMouse)
                        },
                        move |bounds, hitbox, window, cx| {
                            let color = cx.theme().selection.alpha(1.);
                            let bar = Bounds::new(
                                point(caret.left() - BAR_WIDTH / 2., caret.top()),
                                size(BAR_WIDTH, caret.size.height),
                            );
                            let knob_top = match edge {
                                SelectionEdge::Start => caret.top() - KNOB_SIZE,
                                SelectionEdge::End => caret.bottom(),
                            };
                            let knob = Bounds::new(
                                point(caret.left() - KNOB_SIZE / 2., knob_top),
                                size(KNOB_SIZE, KNOB_SIZE),
                            );
                            window.paint_quad(fill(bar, color));
                            window.paint_quad(fill(knob, color).corner_radii(KNOB_SIZE / 2.));
                            if let Some(on_paint) = on_paint.as_ref() {
                                on_paint(bounds, window, cx);
                            }

                            // Touch: the drag is offered on the first touch,
                            // before it can become a tap, a long press or a pan.
                            window.on_mouse_event({
                                let hitbox = hitbox.clone();
                                let on_drag = on_drag.clone();
                                move |event: &TouchDragEvent, phase, window, cx| {
                                    if !phase.bubble() {
                                        return;
                                    }
                                    match event.phase {
                                        TouchPhase::Started => {
                                            if window.default_prevented()
                                                || !hitbox.is_hovered(window)
                                            {
                                                return;
                                            }
                                            window.prevent_default();
                                        }
                                        _ if !dragging => return,
                                        _ => {}
                                    }
                                    cx.stop_propagation();
                                    on_drag(edge, event.phase, event.position, window, cx);
                                }
                            });

                            // Mouse: the same drag for a pointer.
                            window.on_mouse_event({
                                let hitbox = hitbox.clone();
                                let on_drag = on_drag.clone();
                                move |event: &MouseDownEvent, phase, window, cx| {
                                    if !phase.bubble()
                                        || event.button != MouseButton::Left
                                        || !hitbox.is_hovered(window)
                                    {
                                        return;
                                    }
                                    cx.stop_propagation();
                                    on_drag(edge, TouchPhase::Started, event.position, window, cx);
                                }
                            });
                            if dragging {
                                window.on_mouse_event({
                                    let on_drag = on_drag.clone();
                                    move |event: &MouseMoveEvent, phase, window, cx| {
                                        if phase.bubble()
                                            && event.pressed_button == Some(MouseButton::Left)
                                        {
                                            on_drag(
                                                edge,
                                                TouchPhase::Moved,
                                                event.position,
                                                window,
                                                cx,
                                            );
                                        }
                                    }
                                });
                                window.on_mouse_event({
                                    let on_drag = on_drag.clone();
                                    move |event: &MouseUpEvent, phase, window, cx| {
                                        if phase.bubble() && event.button == MouseButton::Left {
                                            on_drag(
                                                edge,
                                                TouchPhase::Ended,
                                                event.position,
                                                window,
                                                cx,
                                            );
                                        }
                                    }
                                });
                            }
                        },
                    )
                    .w(hit.size.width)
                    .h(hit.size.height),
                ),
        )
        .with_priority(gpui_base::POPUP_PRIORITY)
    }
}
