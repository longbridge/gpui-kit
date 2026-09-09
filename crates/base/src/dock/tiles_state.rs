//! A tiles canvas's behavior, with no appearance of its own.

use crate::TestSupportExt as _;
use std::{rc::Rc, sync::Arc};

use gpui::{
    AnyElement, App, Bounds, Context, Div, Empty, EntityId, EventEmitter, FocusHandle, Focusable,
    InteractiveElement as _, IntoElement, ParentElement as _, Pixels, Point, Render, Size,
    Stateful, Styled as _, WeakEntity, Window, div, prelude::FluentBuilder as _, px,
};

use crate::UndoHistory;

use super::{
    drag::AnyDrag,
    layout::{NodeId, PanelId},
    panel::PanelView,
    tiles_geometry::{
        MINIMUM_SIZE, ResizeDrag, ResizeSide, TileChange, apply_boundary_constraints,
        compute_resized_bounds, content_size, magnetic_snap,
    },
};

/// What a tiles canvas cannot carry out on its own.
///
/// The canvas mirrors one `Tiles` node but does not own the tree that node
/// lives in, so — exactly as with [`TabGroupEvent`](super::TabGroupEvent) —
/// every outcome is reported as an intent and applied by the container
/// through `PaneTree::set_tile_bounds` / `PaneTree::bring_to_front`.
#[non_exhaustive]
pub enum TilesEvent {
    /// A tile finished moving or resizing at `bounds`.
    BoundsChanged {
        panel: PanelId,
        bounds: Bounds<Pixels>,
    },
    /// A tile was interacted with and should stack above its peers.
    BringToFront { panel: PanelId },
    /// The user asked to close `panel`, dismissing its tile.
    ClosePanel { panel: PanelId },
    /// A host-owned drag landed on the canvas. The canvas has free
    /// coordinates, so the host reads the landing position itself.
    DragDrop { item: AnyDrag },
    /// One tile asked to fill the whole dock. The container installs the
    /// *canvas* as its zoomed view, and the canvas draws that one tile with
    /// its chrome — which is where the control that zooms back out lives.
    ZoomIn { panel: PanelId },
    /// The zoomed tile gave the dock back.
    ZoomOut,
}

/// One tile, mirrored from a `Tiles` node.
#[derive(Clone)]
struct Tile {
    panel: Arc<dyn PanelView>,
    id: PanelId,
    bounds: Bounds<Pixels>,
    z_index: usize,
}

/// An in-flight move: which tile, and where the pointer and the tile were
/// when it started.
#[derive(Clone, Copy)]
struct TileMove {
    panel: PanelId,
    initial_pointer: Point<Pixels>,
    initial_bounds: Bounds<Pixels>,
}

/// An in-flight resize: which tile, and the geometry module's own drag record.
#[derive(Clone, Copy)]
struct TileResize {
    panel: PanelId,
    initial_bounds: Bounds<Pixels>,
    drag: ResizeDrag,
}

/// A tiles canvas's behavior, with no appearance of its own.
///
/// It owns the tile list mirrored from the layout tree, the in-flight move and
/// resize state, and the undo stack. Everything visible is produced by the
/// [`TilesRenderer`] the host installs.
pub struct TilesState {
    node: NodeId,
    /// Handed to the callbacks in [`TileContext`], which are built from a
    /// plain `&App` and so cannot ask for it.
    this: WeakEntity<Self>,
    tiles: Vec<Tile>,
    focus_handle: FocusHandle,
    /// The tile filling the whole dock, if one is. Driven by the container
    /// through [`Self::set_zoomed`], so the canvas and the container name the
    /// same tile or neither does.
    zoomed: Option<PanelId>,
    moving: Option<TileMove>,
    resizing: Option<TileResize>,
    history: UndoHistory<TileChange>,
    renderer: Rc<dyn TilesRenderer>,
}

impl TilesState {
    /// Only a container builds canvases: a canvas is the entity mirror of one
    /// `Tiles` node, created when that node first appears in the tree.
    pub(crate) fn new(node: NodeId, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            node,
            this: cx.weak_entity(),
            tiles: Vec::new(),
            focus_handle: cx.focus_handle(),
            zoomed: None,
            moving: None,
            resizing: None,
            history: UndoHistory::new().group_interval(std::time::Duration::from_millis(100)),
            renderer: Rc::new(BareTiles),
        }
    }

    pub fn with_renderer(mut self, renderer: Rc<dyn TilesRenderer>) -> Self {
        self.renderer = renderer;
        self
    }

    /// The `Tiles` node this canvas mirrors.
    pub fn node(&self) -> NodeId {
        self.node
    }

    /// Every tile, in stacking order (lowest first).
    pub fn tiles(&self, cx: &App) -> Vec<TileContext> {
        let mut order: Vec<usize> = (0..self.tiles.len()).collect();
        order.sort_by_key(|ix| (self.tiles[*ix].z_index, *ix));
        order
            .into_iter()
            .map(|ix| self.tile_context(ix, cx))
            .collect()
    }

    /// Mirror one `Tiles` node's membership and geometry into this canvas.
    pub(crate) fn sync_from_tree(
        &mut self,
        tiles: Vec<(Arc<dyn PanelView>, Bounds<Pixels>, usize)>,
        cx: &mut Context<Self>,
    ) {
        self.tiles = tiles
            .into_iter()
            .map(|(panel, bounds, z_index)| Tile {
                id: panel.panel_id(cx),
                panel,
                bounds,
                z_index,
            })
            .collect();
        // A tile that left the canvas must not be resurrected by an in-flight
        // gesture that outlived it.
        if self
            .moving
            .is_some_and(|drag| self.index_of(drag.panel).is_none())
        {
            self.moving = None;
        }
        if self
            .resizing
            .is_some_and(|drag| self.index_of(drag.panel).is_none())
        {
            self.resizing = None;
        }
        cx.notify();
    }

    /// The tile filling the whole dock, if one is.
    pub fn zoomed_tile(&self) -> Option<PanelId> {
        self.zoomed
    }

    /// Flip one tile's zoom.
    pub fn toggle_zoom(&mut self, panel: PanelId, window: &mut Window, cx: &mut Context<Self>) {
        let zoomed = (self.zoomed != Some(panel)).then_some(panel);
        self.set_zoomed(zoomed, window, cx);
    }

    /// Zoom one tile in, or zoom out with `None`: the flag changes, the panel
    /// is told, and the container is asked to install or clear the zoomed
    /// view.
    ///
    /// The container drives this too when it clears a zoom from outside, so
    /// the canvas cannot be left naming a tile the container is not showing.
    ///
    /// Zooming *in* is refused, leaving the flag alone, for a tile that is not
    /// on this canvas or whose panel is not zoomable. Zooming *out* is never
    /// refused: a tile that became unzoomable while zoomed still has to be
    /// able to give the dock back.
    pub(crate) fn set_zoomed(
        &mut self,
        zoomed: Option<PanelId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.zoomed == zoomed {
            return;
        }
        // The outgoing tile hears about it as well as the incoming one, so a
        // zoom moved straight from one tile to another leaves neither panel
        // believing it still fills the dock.
        let outgoing = self.zoomed.and_then(|panel| self.panel_view(panel));
        let incoming = zoomed.and_then(|panel| self.panel_view(panel));
        if zoomed.is_some() && !incoming.as_ref().is_some_and(|panel| panel.zoomable(cx)) {
            return;
        }

        self.zoomed = zoomed;
        cx.emit(match zoomed {
            Some(panel) => TilesEvent::ZoomIn { panel },
            None => TilesEvent::ZoomOut,
        });

        // Delivered outside this update so a `set_zoomed` handler may call
        // back into the canvas.
        cx.spawn_in(window, async move |_, cx| {
            _ = cx.update(|window, cx| {
                if let Some(panel) = outgoing {
                    panel.set_zoomed(false, window, cx);
                }
                if let Some(panel) = incoming {
                    panel.set_zoomed(true, window, cx);
                }
            });
        })
        .detach();
        cx.notify();
    }

    /// Undo the most recent group of tile changes.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        let Some(changes) = self.history.undo() else {
            return;
        };
        for change in changes {
            if let (Some(panel), Some(bounds)) =
                (self.panel_of(change.tile_id()), change.old_bounds())
            {
                cx.emit(TilesEvent::BoundsChanged { panel, bounds });
            }
        }
        cx.notify();
    }

    /// Redo the most recently undone group of tile changes.
    pub fn redo(&mut self, cx: &mut Context<Self>) {
        let Some(changes) = self.history.redo() else {
            return;
        };
        for change in changes {
            if let (Some(panel), Some(bounds)) =
                (self.panel_of(change.tile_id()), change.new_bounds())
            {
                cx.emit(TilesEvent::BoundsChanged { panel, bounds });
            }
        }
        cx.notify();
    }
}

impl TilesState {
    fn index_of(&self, panel: PanelId) -> Option<usize> {
        self.tiles.iter().position(|tile| tile.id == panel)
    }

    fn bounds_of(&self, panel: PanelId) -> Option<Bounds<Pixels>> {
        self.index_of(panel).map(|ix| self.tiles[ix].bounds)
    }

    fn panel_view(&self, panel: PanelId) -> Option<Arc<dyn PanelView>> {
        self.index_of(panel).map(|ix| self.tiles[ix].panel.clone())
    }

    /// The panel behind a history record's `EntityId`.
    fn panel_of(&self, entity: EntityId) -> Option<PanelId> {
        self.tiles
            .iter()
            .find(|tile| tile.panel.view().entity_id() == entity)
            .map(|tile| tile.id)
    }

    /// Every other tile's bounds, which is what the snapping arithmetic
    /// measures against.
    fn other_bounds(&self, panel: PanelId) -> Vec<Bounds<Pixels>> {
        self.tiles
            .iter()
            .filter(|tile| tile.id != panel)
            .map(|tile| tile.bounds)
            .collect()
    }

    fn grid_size(&self, cx: &App) -> Pixels {
        self.renderer.grid_size(cx)
    }

    fn begin_move(&mut self, panel: PanelId, pointer: Point<Pixels>, cx: &mut Context<Self>) {
        // A zoomed tile fills the dock rather than sitting at its stored
        // bounds, so there is nothing for a move to mean — the same reason a
        // zoomed tab group reports itself locked.
        if self.zoomed.is_some() {
            return;
        }
        let Some(initial_bounds) = self.bounds_of(panel) else {
            return;
        };
        self.moving = Some(TileMove {
            panel,
            initial_pointer: pointer,
            initial_bounds,
        });
        cx.emit(TilesEvent::BringToFront { panel });
        cx.notify();
    }

    fn move_to(&mut self, pointer: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(drag) = self.moving else {
            return;
        };
        let delta = pointer - drag.initial_pointer;
        let candidate = Bounds {
            origin: apply_boundary_constraints(
                drag.initial_bounds.origin + delta,
                drag.initial_bounds.size.width,
            ),
            size: drag.initial_bounds.size,
        };
        let origin = magnetic_snap(
            candidate,
            &self.other_bounds(drag.panel),
            self.grid_size(cx),
        );

        self.apply_bounds(
            drag.panel,
            Bounds {
                origin,
                size: drag.initial_bounds.size,
            },
            cx,
        );
    }

    fn end_move(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = self.moving.take() else {
            return;
        };
        self.record(drag.panel, drag.initial_bounds, cx);
    }

    fn begin_resize(
        &mut self,
        panel: PanelId,
        side: ResizeSide,
        pointer: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.zoomed.is_some() {
            return;
        }
        let Some(initial_bounds) = self.bounds_of(panel) else {
            return;
        };
        self.resizing = Some(TileResize {
            panel,
            initial_bounds,
            drag: ResizeDrag::new(side, pointer, initial_bounds),
        });
        cx.emit(TilesEvent::BringToFront { panel });
        cx.notify();
    }

    fn resize_to(&mut self, pointer: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(resize) = self.resizing else {
            return;
        };
        let previous = resize.drag.last_bounds();
        // The pointer is in window coordinates and the bounds in canvas
        // coordinates, so the moving edge is derived from how far the pointer
        // has travelled since the drag began, applied to the bounds it began
        // with — never from the pointer's position itself.
        let initial = resize.initial_bounds;
        let delta = pointer - resize.drag.start_position();
        let (new_x, new_y, new_width, new_height) = match resize.drag.side() {
            ResizeSide::Left => (Some(initial.origin.x + delta.x), None, None, None),
            ResizeSide::Right => (
                None,
                None,
                Some((initial.size.width + delta.x).max(MINIMUM_SIZE.width)),
                None,
            ),
            ResizeSide::Top => (None, Some(initial.origin.y + delta.y), None, None),
            ResizeSide::Bottom => (
                None,
                None,
                None,
                Some((initial.size.height + delta.y).max(MINIMUM_SIZE.height)),
            ),
            ResizeSide::BottomRight => (
                None,
                None,
                Some((initial.size.width + delta.x).max(MINIMUM_SIZE.width)),
                Some((initial.size.height + delta.y).max(MINIMUM_SIZE.height)),
            ),
        };

        let bounds = compute_resized_bounds(
            previous,
            new_x,
            new_y,
            new_width,
            new_height,
            &self.other_bounds(resize.panel),
            self.grid_size(cx),
        );

        self.resizing = Some(TileResize {
            drag: resize.drag.with_last_bounds(bounds),
            ..resize
        });
        self.apply_bounds(resize.panel, bounds, cx);
    }

    fn end_resize(&mut self, cx: &mut Context<Self>) {
        let Some(resize) = self.resizing.take() else {
            return;
        };
        self.record(resize.panel, resize.initial_bounds, cx);
    }

    /// Show the new geometry immediately and report it, so the container's
    /// tree and this mirror never disagree for a frame.
    fn apply_bounds(&mut self, panel: PanelId, bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        let Some(ix) = self.index_of(panel) else {
            return;
        };
        if self.tiles[ix].bounds == bounds {
            return;
        }
        self.tiles[ix].bounds = bounds;
        cx.emit(TilesEvent::BoundsChanged { panel, bounds });
        cx.notify();
    }

    /// Push one completed gesture onto the undo stack.
    fn record(&mut self, panel: PanelId, old_bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        let Some(ix) = self.index_of(panel) else {
            return;
        };
        let tile = &self.tiles[ix];
        if tile.bounds == old_bounds {
            return;
        }
        self.history.push(TileChange::bounds_change(
            tile.panel.view().entity_id(),
            old_bounds,
            tile.bounds,
        ));
        cx.notify();
    }

    /// Ask the container to close `panel`. Nothing happens for a tile that
    /// is not on this canvas, or for a panel that refuses to close.
    fn close_tile(&mut self, panel: PanelId, cx: &mut Context<Self>) {
        let closable = self
            .tiles
            .iter()
            .any(|tile| tile.id == panel && tile.panel.closable(cx));
        if !closable {
            return;
        }
        cx.emit(TilesEvent::ClosePanel { panel });
        cx.notify();
    }

    fn tile_context(&self, ix: usize, cx: &App) -> TileContext {
        let tile = &self.tiles[ix];
        let panel = tile.id;
        let canvas = self.this.clone();

        TileContext {
            node: self.node,
            panel: tile.panel.clone(),
            id: panel,
            bounds: tile.bounds,
            z_index: tile.z_index,
            moving: self.moving.is_some_and(|drag| drag.panel == panel),
            resizing: self.resizing.is_some_and(|drag| drag.panel == panel),
            closable: tile.panel.closable(cx),
            zoomed: self.zoomed == Some(panel),
            zoomable: tile.panel.zoomable(cx),
            on_begin_move: {
                let canvas = canvas.clone();
                Rc::new(move |pointer, _, cx| {
                    _ = canvas.update(cx, |canvas, cx| canvas.begin_move(panel, pointer, cx));
                })
            },
            on_move_to: {
                let canvas = canvas.clone();
                Rc::new(move |pointer, _, cx| {
                    _ = canvas.update(cx, |canvas, cx| canvas.move_to(pointer, cx));
                })
            },
            on_end_move: {
                let canvas = canvas.clone();
                Rc::new(move |_, cx| {
                    _ = canvas.update(cx, |canvas, cx| canvas.end_move(cx));
                })
            },
            on_begin_resize: {
                let canvas = canvas.clone();
                Rc::new(move |side, pointer, _, cx| {
                    _ = canvas.update(cx, |canvas, cx| {
                        canvas.begin_resize(panel, side, pointer, cx)
                    });
                })
            },
            on_resize_to: {
                let canvas = canvas.clone();
                Rc::new(move |pointer, _, cx| {
                    _ = canvas.update(cx, |canvas, cx| canvas.resize_to(pointer, cx));
                })
            },
            on_end_resize: {
                let canvas = canvas.clone();
                Rc::new(move |_, cx| {
                    _ = canvas.update(cx, |canvas, cx| canvas.end_resize(cx));
                })
            },
            on_bring_to_front: {
                let canvas = canvas.clone();
                Rc::new(move |_, cx| {
                    _ = canvas.update(cx, |_, cx| {
                        cx.emit(TilesEvent::BringToFront { panel });
                    });
                })
            },
            on_toggle_zoom: {
                let canvas = canvas.clone();
                Rc::new(move |window, cx| {
                    _ = canvas.update(cx, |canvas, cx| canvas.toggle_zoom(panel, window, cx));
                })
            },
            on_close: Rc::new(move |_, cx| {
                _ = canvas.update(cx, |canvas, cx| canvas.close_tile(panel, cx));
            }),
        }
    }
}

impl EventEmitter<TilesEvent> for TilesState {}

impl Focusable for TilesState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TilesState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let renderer = self.renderer.clone();
        let focus_handle = self.focus_handle.clone();
        // A zoomed tile fills the dock on its own, so the tiles beside it are
        // not drawn — just as the rest of the dock is not drawn behind it.
        // A zoom naming a tile that has since left the canvas draws the
        // canvas whole rather than nothing at all.
        let zoomed = self.zoomed.filter(|panel| self.index_of(*panel).is_some());
        let tiles: Vec<TileContext> = self
            .tiles(cx)
            .into_iter()
            .filter(|tile| zoomed.is_none_or(|panel| tile.id == panel))
            .collect();
        // Every tile, not the drawn subset: an overlay scrollbar measures the
        // whole canvas.
        let content = content_size(
            &self
                .tiles
                .iter()
                .map(|tile| tile.bounds)
                .collect::<Vec<_>>(),
        );

        renderer
            .frame(window, cx)
            .test_support()
            .track_focus(&focus_handle)
            .on_drop(cx.listener(|_, item: &AnyDrag, _, cx| {
                cx.emit(TilesEvent::DragDrop { item: item.clone() });
            }))
            .children(
                tiles
                    .into_iter()
                    .map(|tile| {
                        renderer
                            .tile_frame(&tile, window, cx)
                            // The only positioning base installs anywhere in
                            // the dock. A tiles canvas *is* "panels at stored
                            // coordinates": drawing one somewhere other than
                            // its own bounds would not be a different skin,
                            // it would be a different data structure. A
                            // zoomed tile is the exception — it is no longer
                            // at its coordinates, and how it fills the dock
                            // is the skin's to decide.
                            .when(!tile.zoomed, |this| {
                                this.absolute()
                                    .left(tile.bounds.origin.x)
                                    .top(tile.bounds.origin.y)
                                    .w(tile.bounds.size.width)
                                    .h(tile.bounds.size.height)
                            })
                            .child(renderer.render_drag_bar(&tile, window, cx))
                            .child(
                                renderer
                                    .panel_frame(&tile, window, cx)
                                    .child(tile.panel.view()),
                            )
                            // Nothing to resize against while zoomed, and the
                            // canvas refuses the gesture anyway.
                            .when(!tile.zoomed, |this| {
                                this.child(renderer.render_resize_handles(&tile, window, cx))
                            })
                    })
                    .collect::<Vec<_>>(),
            )
            // Last, so it paints and hit-tests above every tile. A zoomed
            // canvas draws one tile filling the dock, so there is no canvas
            // for an overlay to sit over.
            .when(zoomed.is_none(), |this| {
                this.children(renderer.render_overlay(content, window, cx))
            })
    }
}

type MovePointerHandler = Rc<dyn Fn(Point<Pixels>, &mut Window, &mut App)>;
type ResizeStartHandler = Rc<dyn Fn(ResizeSide, Point<Pixels>, &mut Window, &mut App)>;
type GestureEndHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// What a skin needs to draw one tile, and the callbacks it invokes rather
/// than reimplementing the snapping and resize arithmetic.
#[derive(Clone)]
pub struct TileContext {
    node: NodeId,
    panel: Arc<dyn PanelView>,
    id: PanelId,
    bounds: Bounds<Pixels>,
    z_index: usize,
    moving: bool,
    resizing: bool,
    closable: bool,
    zoomed: bool,
    zoomable: bool,
    on_begin_move: MovePointerHandler,
    on_move_to: MovePointerHandler,
    on_end_move: GestureEndHandler,
    on_begin_resize: ResizeStartHandler,
    on_resize_to: MovePointerHandler,
    on_end_resize: GestureEndHandler,
    on_bring_to_front: GestureEndHandler,
    on_toggle_zoom: GestureEndHandler,
    on_close: GestureEndHandler,
}

impl TileContext {
    /// The `Tiles` node this tile belongs to, for a skin that needs to name
    /// the canvas in a drag payload or a drop target.
    pub fn node(&self) -> NodeId {
        self.node
    }

    pub fn panel(&self) -> &Arc<dyn PanelView> {
        &self.panel
    }

    pub fn panel_id(&self) -> PanelId {
        self.id
    }

    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    pub fn z_index(&self) -> usize {
        self.z_index
    }

    pub fn is_moving(&self) -> bool {
        self.moving
    }

    pub fn is_resizing(&self) -> bool {
        self.resizing
    }

    pub fn is_closable(&self) -> bool {
        self.closable
    }

    /// Whether this tile is the one filling the whole dock.
    ///
    /// A zoomed tile is drawn without its stored bounds and takes no move or
    /// resize gesture, so a skin should offer the way back out here rather
    /// than the affordances of a tile that can still be dragged.
    pub fn is_zoomed(&self) -> bool {
        self.zoomed
    }

    /// Whether this tile's panel allows zooming at all. Where the zoom
    /// control appears is the skin's decision; whether there is one to offer
    /// is not.
    pub fn is_zoomable(&self) -> bool {
        self.zoomable
    }

    /// Pointer positions are in window coordinates: every gesture is resolved
    /// against the position the gesture started at, so the skin never has to
    /// convert into canvas space.
    pub fn begin_move(&self, pointer: Point<Pixels>, window: &mut Window, cx: &mut App) {
        (self.on_begin_move)(pointer, window, cx);
    }

    pub fn move_to(&self, pointer: Point<Pixels>, window: &mut Window, cx: &mut App) {
        (self.on_move_to)(pointer, window, cx);
    }

    pub fn end_move(&self, window: &mut Window, cx: &mut App) {
        (self.on_end_move)(window, cx);
    }

    pub fn begin_resize(
        &self,
        side: ResizeSide,
        pointer: Point<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        (self.on_begin_resize)(side, pointer, window, cx);
    }

    pub fn resize_to(&self, pointer: Point<Pixels>, window: &mut Window, cx: &mut App) {
        (self.on_resize_to)(pointer, window, cx);
    }

    pub fn end_resize(&self, window: &mut Window, cx: &mut App) {
        (self.on_end_resize)(window, cx);
    }

    pub fn bring_to_front(&self, window: &mut Window, cx: &mut App) {
        (self.on_bring_to_front)(window, cx);
    }

    /// Flip this tile between filling the whole dock and sitting at its
    /// stored bounds.
    ///
    /// Zooming *in* is refused when [`Self::is_zoomable`] is false, so a skin
    /// that offers a Zoom control should gate it on that. Zooming out is
    /// never refused.
    pub fn toggle_zoom(&self, window: &mut Window, cx: &mut App) {
        (self.on_toggle_zoom)(window, cx);
    }

    /// Dismiss this tile. Refused when [`Self::is_closable`] is false, so a
    /// skin that offers a Close control should gate it on that.
    pub fn close(&self, window: &mut Window, cx: &mut App) {
        (self.on_close)(window, cx);
    }
}

/// Appearance for a tiles canvas. Base draws none of it.
///
/// Like [`TabGroupRenderer`](super::TabGroupRenderer), the frame hooks return
/// the element itself rather than wrapping one: base attaches focus and drop
/// handling to the canvas frame and the stored bounds to the tile frame, so a
/// wrapper would put the hit area and the painted area on different elements.
/// That is also why there is no `wrap_canvas` hook — it would be exactly the
/// wrapper the `TabGroupRenderer` review ruled out.
#[allow(unused_variables)]
pub trait TilesRenderer: 'static {
    /// The canvas frame, which base tracks focus and drop handling on.
    fn frame(&self, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        div().id("tiles")
    }

    /// One tile's frame, which base positions at the tile's stored bounds.
    fn tile_frame(&self, tile: &TileContext, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        div().id("tile")
    }

    /// The strip the tile is dragged by. Its height is
    /// [`DRAG_BAR_HEIGHT`](super::DRAG_BAR_HEIGHT), which base's snapping
    /// arithmetic and the skin must agree on.
    fn render_drag_bar(&self, tile: &TileContext, window: &mut Window, cx: &mut App) -> AnyElement;

    /// The tile's resize affordances. Their hit size is
    /// [`HANDLE_SIZE`](super::HANDLE_SIZE).
    fn render_resize_handles(
        &self,
        tile: &TileContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        Empty.into_any_element()
    }

    /// The frame around one tile's panel view.
    ///
    /// A wrapper, unlike the other frame hooks, and for the same reason
    /// [`DockAreaRenderer::split_frame`](super::DockAreaRenderer::split_frame)
    /// is one: base attaches nothing to it, so there is no hit area to keep
    /// together with the painted area. It exists because base draws the panel
    /// as an ordinary child of the tile frame, and a panel that does not size
    /// itself would otherwise have no size at all — the old canvas wrapped it
    /// in `h_flex().overflow_hidden().size_full()`, and nothing else can put
    /// that back.
    fn panel_frame(&self, tile: &TileContext, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        div().id(("tile-panel", tile.panel_id().as_u64()))
    }

    /// An element drawn above every tile — a scrollbar, a drop hint.
    ///
    /// Rendered last, so it paints and hit-tests above the tiles. That
    /// ordering is the whole point: the canvas frame is the scroll container,
    /// so an overlay placed with the frame's own children would sit beneath
    /// every tile. Not called while a tile is zoomed, because then the canvas
    /// is one tile filling the dock rather than a canvas.
    ///
    /// `content` is the scrollable extent of every tile measured from the
    /// canvas origin, which a scrollbar needs and a hook holding only
    /// `&mut App` could not work out.
    fn render_overlay(
        &self,
        content: Size<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        None
    }

    /// The grid a tile snaps to when no neighbouring edge is close enough.
    ///
    /// The old canvas read this off the theme, which base cannot see; the
    /// default is the ten-pixel grid the original rounded to.
    fn grid_size(&self, cx: &App) -> Pixels {
        px(10.)
    }
}

/// The renderer a canvas starts with: the tiles and nothing else.
pub(crate) struct BareTiles;

impl TilesRenderer for BareTiles {
    fn render_drag_bar(&self, _: &TileContext, _: &mut Window, _: &mut App) -> AnyElement {
        Empty.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{Bounds, Entity, TestAppContext, VisualTestContext, point, size};

    use super::*;
    use crate::ElementExt as _;
    use crate::dock::{
        DockArea, DockAreaRenderer, DockLayout, TabGroupRenderer, test_support::TestPanel,
    };

    /// What each hook drew, in the order the frame prepainted it.
    ///
    /// Prepaint order, not call order: the overlay has to be *below the tiles
    /// in the element tree*, and a renderer that computed it early and added
    /// it late would still be called first. Prepaint walks the tree, so this
    /// records the property that matters.
    #[derive(Default)]
    struct DrawOrder {
        painted: Vec<&'static str>,
        content: Option<Size<Pixels>>,
        /// The contexts the drag bar was handed, so a test can drive a tile
        /// through the same seam a skin would.
        tiles: Vec<TileContext>,
    }

    struct OrderRecorder {
        order: Rc<RefCell<DrawOrder>>,
    }

    impl TilesRenderer for OrderRecorder {
        fn render_drag_bar(&self, tile: &TileContext, _: &mut Window, _: &mut App) -> AnyElement {
            self.order.borrow_mut().tiles.push(tile.clone());
            Empty.into_any_element()
        }

        fn panel_frame(&self, tile: &TileContext, _: &mut Window, _: &mut App) -> Stateful<Div> {
            let order = self.order.clone();
            div()
                .id(("tile", tile.panel_id().as_u64()))
                .on_prepaint(move |_, _, _| order.borrow_mut().painted.push("tile"))
        }

        fn render_overlay(
            &self,
            content: Size<Pixels>,
            _: &mut Window,
            _: &mut App,
        ) -> Option<AnyElement> {
            let order = self.order.clone();
            Some(
                div()
                    .on_prepaint(move |_, _, _| {
                        let mut order = order.borrow_mut();
                        order.painted.push("overlay");
                        order.content = Some(content);
                    })
                    .into_any_element(),
            )
        }
    }

    impl TabGroupRenderer for OrderRecorder {
        fn render_tab_bar(
            &self,
            _: &super::super::TabGroupContext,
            _: &mut Window,
            _: &mut App,
        ) -> AnyElement {
            Empty.into_any_element()
        }
    }

    impl DockAreaRenderer for OrderRecorder {
        fn tab_group_renderer(&self) -> Rc<dyn TabGroupRenderer> {
            Rc::new(OrderRecorder {
                order: self.order.clone(),
            })
        }

        fn tiles_renderer(&self) -> Rc<dyn TilesRenderer> {
            Rc::new(OrderRecorder {
                order: self.order.clone(),
            })
        }
    }

    fn setup_order(
        cx: &mut TestAppContext,
    ) -> (
        Entity<DockArea>,
        Rc<RefCell<DrawOrder>>,
        &mut VisualTestContext,
    ) {
        cx.update(|cx| {
            let _ = crate::Theme::global_mut(cx);
        });
        let order: Rc<RefCell<DrawOrder>> = Rc::default();
        let renderer = Rc::new(OrderRecorder {
            order: order.clone(),
        });
        let (area, cx) = cx.add_window_view(|window, cx| {
            DockArea::new("tiles-order", None, window, cx).with_renderer(renderer)
        });
        (area, order, cx)
    }

    /// The canvas overlay is drawn after every tile.
    ///
    /// This is the whole reason the hook exists rather than the skin adding a
    /// scrollbar to the canvas frame: the frame is the scroll container, and
    /// base appends the tiles to whatever it carries, so anything placed there
    /// paints and hit-tests underneath every tile. Move
    /// `children(render_overlay(..))` above `children(tiles)` in
    /// `TilesState::render` and this fails.
    #[gpui::test]
    fn the_canvas_overlay_is_drawn_after_every_tile(cx: &mut TestAppContext) {
        let (area, order, cx) = setup_order(cx);

        cx.update(|window, cx| {
            let first = TestPanel::new("First", cx);
            let second = TestPanel::new("Second", cx);
            let layout = DockLayout::tiles()
                .tile(
                    first,
                    Bounds {
                        origin: point(px(20.), px(20.)),
                        size: size(px(100.), px(80.)),
                    },
                )
                .tile(
                    second,
                    Bounds {
                        origin: point(px(140.), px(20.)),
                        size: size(px(100.), px(80.)),
                    },
                );
            area.update(cx, |area, cx| area.set_center(layout, window, cx));
        });
        cx.run_until_parked();
        order.borrow_mut().painted.clear();
        cx.update(|window, cx| window.draw(cx).clear(cx));

        assert_eq!(
            order.borrow().painted,
            vec!["tile", "tile", "overlay"],
            "the overlay must come after every tile, or it paints beneath them"
        );

        // And it is handed the whole canvas, not the tile it happens to sit
        // over: 20..240 across and 20..100 down, measured from the origin.
        assert_eq!(
            order.borrow().content,
            Some(size(px(240.), px(100.))),
            "the overlay is given the canvas's scrollable extent"
        );
    }

    /// A resize is resolved against the pointer's travel since the drag
    /// began, never against the pointer's position: pointer positions are
    /// window coordinates and tile bounds are canvas coordinates, and the two
    /// differ by the canvas's own offset in the window. Reading the position
    /// directly widened the tile by that offset the moment a drag started.
    #[gpui::test]
    fn a_resize_tracks_the_pointer_travel_not_its_window_position(cx: &mut TestAppContext) {
        let (area, order, cx) = setup_order(cx);

        cx.update(|window, cx| {
            let panel = TestPanel::new("Only", cx);
            let layout = DockLayout::tiles().tile(
                panel,
                Bounds {
                    origin: point(px(20.), px(20.)),
                    size: size(px(100.), px(100.)),
                },
            );
            area.update(cx, |area, cx| area.set_center(layout, window, cx));
        });
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let tile = order.borrow().tiles.last().cloned().expect("the tile drew");

        // The pointer's window position is nowhere near the tile's canvas
        // bounds, as it never is once the canvas sits offset in the window.
        let start = point(px(500.), px(300.));
        cx.update(|window, cx| tile.begin_resize(ResizeSide::Right, start, window, cx));
        cx.update(|window, cx| tile.resize_to(start, window, cx));
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(
            order.borrow().tiles.last().unwrap().bounds().size.width,
            px(100.),
            "a pointer that has not moved must not resize the tile"
        );

        // 32px of travel: 100 + 32 puts the right edge at 152, and the
        // ten-pixel grid rounds it to 150.
        cx.update(|window, cx| tile.resize_to(start + point(px(32.), px(0.)), window, cx));
        cx.update(|window, cx| tile.end_resize(window, cx));
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(
            order.borrow().tiles.last().unwrap().bounds().size.width,
            px(130.),
            "the tile grows by the pointer's travel, grid-rounded"
        );
    }

    /// A zoomed tile fills the dock, so there is no canvas to overlay.
    #[gpui::test]
    fn a_zoomed_canvas_draws_no_overlay(cx: &mut TestAppContext) {
        let (area, order, cx) = setup_order(cx);

        let panel = cx.update(|window, cx| {
            let panel = TestPanel::new("Only", cx);
            let layout = DockLayout::tiles().tile(
                panel.clone(),
                Bounds {
                    origin: point(px(20.), px(20.)),
                    size: size(px(100.), px(80.)),
                },
            );
            area.update(cx, |area, cx| area.set_center(layout, window, cx));
            panel
        });
        cx.run_until_parked();

        // Zoomed through the seam a skin uses, not a back door.
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let tile = order.borrow().tiles.last().cloned().expect("the tile drew");
        cx.update(|window, cx| tile.toggle_zoom(window, cx));
        cx.run_until_parked();
        assert_eq!(
            cx.read(|cx| area.read(cx).zoomed_tile()),
            Some(PanelId::from(panel.entity_id())),
            "the tile is the one filling the dock"
        );
        order.borrow_mut().painted.clear();
        cx.update(|window, cx| window.draw(cx).clear(cx));

        assert_eq!(
            order.borrow().painted,
            vec!["tile"],
            "a zoomed tile fills the dock, so no overlay is drawn over it"
        );
    }
}
