use std::rc::Rc;

use gpui::{
    App, Bounds, Hitbox, HitboxBehavior, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, RenderOnce, Styled as _, TouchDragEvent, TouchPhase, Window,
    canvas, deferred, fill, point, px, size,
};
use gpui_base::{SelectionEdge, TouchSelectionSnapshot};

use crate::ActiveTheme as _;

/// How wide the finger may miss the knob and still take it.
const HANDLE_HIT_SIZE: Pixels = px(44.);
/// The knob's diameter.
const KNOB_SIZE: Pixels = px(10.);
/// The bar down the caret line.
const BAR_WIDTH: Pixels = px(2.);
/// Room the knob takes beyond the line, kept clear by the edit menu.
pub(super) const KNOB_EXTENT: Pixels = px(12.);

/// Reads the touch selection as laid out by the time the handles paint.
pub(crate) type SnapshotSource = Rc<dyn Fn(&Window, &App) -> Option<TouchSelectionSnapshot>>;
/// What a handle's owner gets told as the finger moves it.
pub(crate) type DragHandler =
    Rc<dyn Fn(SelectionEdge, TouchPhase, Point<Pixels>, &mut Window, &mut App)>;
/// Where a surface was painted this frame, for owners that must protect it.
pub(crate) type SurfaceHandler = Rc<dyn Fn(Bounds<Pixels>, &mut Window, &mut App)>;

/// The grab handles at the ends of a touch selection: a bar down each caret
/// line with a knob at its outer end, the start's above and the end's below.
///
/// The handles read the selection's geometry as they paint, after the text
/// that owns it has painted this frame, so they sit on the text as it is now
/// rather than as it was a frame ago while it scrolls.
///
/// A handle claims the touch drag that begins on it before the window can
/// take the drag for panning, and blocks the mouse so a stray tap on it does
/// not reach the text underneath. A mouse can drag it too, which is how it is
/// exercised without a touch screen.
#[derive(IntoElement)]
pub(crate) struct SelectionHandles {
    source: SnapshotSource,
    on_drag: DragHandler,
    on_paint: Option<SurfaceHandler>,
}

/// One handle laid out for this frame.
struct LaidOutHandle {
    edge: SelectionEdge,
    caret: Bounds<Pixels>,
    hitbox: Hitbox,
}

impl SelectionHandles {
    pub(crate) fn new(source: SnapshotSource, on_drag: DragHandler) -> Self {
        Self {
            source,
            on_drag,
            on_paint: None,
        }
    }

    pub(crate) fn on_paint(mut self, on_paint: SurfaceHandler) -> Self {
        self.on_paint = Some(on_paint);
        self
    }

    /// The touch target, centered on the caret and reaching out past the knob.
    fn hit_bounds(edge: SelectionEdge, caret: Bounds<Pixels>) -> Bounds<Pixels> {
        let top = match edge {
            SelectionEdge::Start => caret.top() - KNOB_EXTENT,
            SelectionEdge::End => caret.top(),
        };
        Bounds::new(
            point(caret.left() - HANDLE_HIT_SIZE / 2., top),
            size(HANDLE_HIT_SIZE, caret.size.height + KNOB_EXTENT),
        )
    }

    /// Lays out a handle for each end of a non-empty selection that is in
    /// view, and returns which end the finger holds.
    fn layout(
        source: &SnapshotSource,
        window: &mut Window,
        cx: &mut App,
    ) -> (Vec<LaidOutHandle>, Option<SelectionEdge>) {
        let Some(snapshot) = source(window, cx) else {
            return (Vec::new(), None);
        };
        let window_bounds = Bounds::new(Point::default(), window.viewport_size());
        let mut handles = Vec::with_capacity(2);
        if !snapshot.is_empty() {
            for edge in [SelectionEdge::Start, SelectionEdge::End] {
                // No handle for an end scrolled out of its owner, nor for one
                // outside the window.
                let caret = snapshot.edge(edge);
                if !snapshot.is_edge_visible(edge) || !window_bounds.contains(&caret.origin) {
                    continue;
                }
                let hitbox =
                    window.insert_hitbox(Self::hit_bounds(edge, caret), HitboxBehavior::BlockMouse);
                handles.push(LaidOutHandle {
                    edge,
                    caret,
                    hitbox,
                });
            }
        }
        (handles, snapshot.dragging())
    }

    fn paint_handle(handle: &LaidOutHandle, window: &mut Window, cx: &mut App) {
        let caret = handle.caret;
        let color = cx.theme().selection.alpha(1.);
        let bar = Bounds::new(
            point(caret.left() - BAR_WIDTH / 2., caret.top()),
            size(BAR_WIDTH, caret.size.height),
        );
        let knob_top = match handle.edge {
            SelectionEdge::Start => caret.top() - KNOB_SIZE,
            SelectionEdge::End => caret.bottom(),
        };
        let knob = Bounds::new(
            point(caret.left() - KNOB_SIZE / 2., knob_top),
            size(KNOB_SIZE, KNOB_SIZE),
        );
        window.paint_quad(fill(bar, color));
        window.paint_quad(fill(knob, color).corner_radii(KNOB_SIZE / 2.));
    }

    /// The drag begins on whichever handle is under the finger or pointer;
    /// its moves and its end are routed by which end is being dragged, so
    /// they keep coming even if that handle is not laid out for a frame.
    fn listen(
        handles: &[LaidOutHandle],
        dragging: Option<SelectionEdge>,
        on_drag: &DragHandler,
        window: &mut Window,
    ) {
        for handle in handles {
            // Touch: the drag is offered on the first touch, before it can
            // become a tap, a long press or a pan.
            window.on_mouse_event({
                let hitbox = handle.hitbox.clone();
                let edge = handle.edge;
                let on_drag = on_drag.clone();
                move |event: &TouchDragEvent, phase, window, cx| {
                    if !phase.bubble()
                        || event.phase != TouchPhase::Started
                        || window.default_prevented()
                        || !hitbox.is_hovered(window)
                    {
                        return;
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                    on_drag(edge, TouchPhase::Started, event.position, window, cx);
                }
            });
            // Mouse: the same drag for a pointer.
            window.on_mouse_event({
                let hitbox = handle.hitbox.clone();
                let edge = handle.edge;
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
        }

        let Some(edge) = dragging else {
            return;
        };
        window.on_mouse_event({
            let on_drag = on_drag.clone();
            move |event: &TouchDragEvent, phase, window, cx| {
                if phase.bubble() && event.phase != TouchPhase::Started {
                    cx.stop_propagation();
                    on_drag(edge, event.phase, event.position, window, cx);
                }
            }
        });
        window.on_mouse_event({
            let on_drag = on_drag.clone();
            move |event: &MouseMoveEvent, phase, window, cx| {
                if phase.bubble() && event.pressed_button == Some(MouseButton::Left) {
                    on_drag(edge, TouchPhase::Moved, event.position, window, cx);
                }
            }
        });
        window.on_mouse_event({
            let on_drag = on_drag.clone();
            move |event: &MouseUpEvent, phase, window, cx| {
                if phase.bubble() && event.button == MouseButton::Left {
                    on_drag(edge, TouchPhase::Ended, event.position, window, cx);
                }
            }
        });
    }
}

impl RenderOnce for SelectionHandles {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let source = self.source;
        let on_drag = self.on_drag;
        let on_paint = self.on_paint;
        deferred(
            canvas(
                move |_, window, cx| Self::layout(&source, window, cx),
                move |_, (handles, dragging), window, cx| {
                    for handle in &handles {
                        Self::paint_handle(handle, window, cx);
                        if let Some(on_paint) = on_paint.as_ref() {
                            on_paint(handle.hitbox.bounds, window, cx);
                        }
                    }
                    Self::listen(&handles, dragging, &on_drag, window);
                },
            )
            .absolute()
            .size_0(),
        )
        .with_priority(gpui_base::POPUP_PRIORITY)
    }
}
