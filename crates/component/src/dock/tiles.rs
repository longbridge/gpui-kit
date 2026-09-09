//! The gpui-component appearance for a tiles canvas.
//!
//! `gpui_base::dock::TilesState` owns the geometry — snapping, the resize
//! arithmetic, the undo stack, the zoom flag — and draws none of it. The tile
//! frame, its title bar and its resize affordances are here.

use std::rc::Rc;

use gpui::{
    AnyElement, App, AppContext as _, Context, Div, DragMoveEvent, Empty, InteractiveElement as _,
    IntoElement, MouseButton, MouseDownEvent, ParentElement as _, Pixels, Render, ScrollHandle,
    Size, Stateful, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_base::dock::{
    DRAG_BAR_HEIGHT, HANDLE_SIZE, NodeId, ResizeSide, TileContext, TilesRenderer,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Icon, IconName, Selectable as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    dock::{PanelHandle, SkinShared, tab_panel::panel_title},
    h_flex,
    menu::{DropdownMenu as _, PopupMenuItem},
    scroll::Scrollbar,
    v_flex,
};

/// How far a resize handle sticks out past the tile's edge.
const HANDLE_OFFSET: Pixels = px(-4.);

/// The payload a tile drag carries, so one canvas ignores another's drags.
#[derive(Clone)]
pub struct DragMoving(NodeId);

impl Render for DragMoving {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// The payload a tile resize carries, for the same reason.
#[derive(Clone)]
pub struct DragResizing(NodeId);

impl Render for DragResizing {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// One tiles canvas's appearance.
///
/// Built per canvas — `DockAreaRenderer::tiles_renderer` is called once per
/// container — so the scroll position belongs to the canvas it scrolls.
pub(crate) struct TilesSkin {
    shared: Rc<SkinShared>,
    scroll_handle: ScrollHandle,
}

impl TilesSkin {
    pub(crate) fn new(shared: Rc<SkinShared>) -> Self {
        Self {
            shared,
            scroll_handle: ScrollHandle::default(),
        }
    }

    /// One edge or corner handle.
    fn resize_handle(
        &self,
        tile: &TileContext,
        id: &'static str,
        side: ResizeSide,
        build: impl FnOnce(Stateful<Div>) -> Stateful<Div>,
    ) -> Stateful<Div> {
        let node = tile.node();

        build(div().id(id).absolute())
            .on_mouse_down(MouseButton::Left, {
                let tile = tile.clone();
                move |event: &MouseDownEvent, window, cx| {
                    tile.begin_resize(side, event.position, window, cx);
                    cx.stop_propagation();
                }
            })
            .on_drag(DragResizing(node), |drag, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| drag.clone())
            })
            .on_drag_move({
                let tile = tile.clone();
                move |event: &DragMoveEvent<DragResizing>, window, cx| {
                    if event.drag(cx).0 != node {
                        return;
                    }
                    tile.resize_to(event.event.position, window, cx);
                }
            })
    }

    /// The trailing controls of a tile's title bar.
    ///
    /// A tile has no tab bar to hang a toolbar off, so this is where its zoom,
    /// close and ellipsis menu live. The entries use click handlers rather
    /// than the [`ToggleZoom`](super::ToggleZoom) and
    /// [`ClosePanel`](super::ClosePanel) actions: those are dispatched to a
    /// focused tab group, and a tile is not one.
    fn render_tile_controls(
        &self,
        tile: &TileContext,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let handle = PanelHandle::of(tile.panel());
        let control = handle.and_then(|handle| handle.zoom_control(cx));
        let zoomed = tile.is_zoomed();
        let toolbar_zoom =
            tile.is_zoomable() && control.is_some_and(|control| control.toolbar_visible());
        let menu_zoom = tile.is_zoomable() && control.is_some_and(|control| control.menu_visible());
        let closable = tile.is_closable();
        let buttons = handle.and_then(|handle| handle.toolbar_buttons(window, cx));
        let panel = handle.map(|handle| handle.panel());

        h_flex()
            .gap_1()
            .flex_shrink_0()
            .occlude()
            .when_some(buttons, |this, buttons| {
                this.children(
                    buttons
                        .into_iter()
                        .map(|button| button.xsmall().ghost().tab_stop(false)),
                )
            })
            .when_some(
                match (zoomed, toolbar_zoom) {
                    (true, _) => Some(("zoom-out", IconName::Minimize, t!("Dock.Zoom Out"))),
                    (false, true) => Some(("zoom-in", IconName::Maximize, t!("Dock.Zoom In"))),
                    (false, false) => None,
                },
                |this, (id, icon, tooltip)| {
                    this.child(
                        Button::new(id)
                            .icon(icon)
                            .xsmall()
                            .ghost()
                            .tab_stop(false)
                            .tooltip(tooltip)
                            .selected(zoomed)
                            .on_click({
                                let tile = tile.clone();
                                move |_, window, cx| tile.toggle_zoom(window, cx)
                            }),
                    )
                },
            )
            .child(
                Button::new("menu")
                    .icon(IconName::Ellipsis)
                    .xsmall()
                    .ghost()
                    .tab_stop(false)
                    .dropdown_menu({
                        let tile = tile.clone();
                        move |menu, window, cx| {
                            menu.when_some(panel.clone(), |menu, panel| {
                                panel.dropdown_menu(menu, window, cx)
                            })
                            .separator()
                            .item(
                                PopupMenuItem::new(match zoomed {
                                    true => t!("Dock.Zoom Out"),
                                    false => t!("Dock.Zoom In"),
                                })
                                .disabled(!menu_zoom && !zoomed)
                                .on_click({
                                    let tile = tile.clone();
                                    move |_, window, cx| tile.toggle_zoom(window, cx)
                                }),
                            )
                            .when(closable, |menu| {
                                menu.separator().item(
                                    PopupMenuItem::new(t!("Dock.Close")).on_click({
                                        let tile = tile.clone();
                                        move |_, window, cx| tile.close(window, cx)
                                    }),
                                )
                            })
                        }
                    })
                    .anchor(gpui::Anchor::TopRight),
            )
    }
}

impl TilesRenderer for TilesSkin {
    fn frame(&self, _: &mut Window, cx: &mut App) -> Stateful<Div> {
        div()
            .id("tiles")
            .relative()
            .size_full()
            .bg(cx.theme().tokens.tiles)
            .track_scroll(&self.scroll_handle)
            .overflow_scroll()
    }

    fn tile_frame(&self, tile: &TileContext, _: &mut Window, cx: &mut App) -> Stateful<Div> {
        v_flex()
            .id(("tile", tile.panel_id().as_u64()))
            .occlude()
            // No `overflow_hidden` here: the resize handles hang past the
            // tile's edge, and a content mask would cut their hit areas down
            // to the sliver inside it. The panel is clipped by `panel_frame`.
            .bg(cx.theme().tokens.background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().tile_radius)
            // Room for the title bar, which is positioned over the padding so
            // the panel below it is never covered. Base draws the panel view
            // as a plain child, so this is the only way to keep the two from
            // overlapping.
            .pt(DRAG_BAR_HEIGHT)
            // Base installs the stored bounds on an ordinary tile and nothing
            // at all on a zoomed one — how a zoomed tile fills the dock is
            // this skin's decision.
            .when(tile.is_zoomed(), |this| this.size_full())
            // One extra pixel past the stored bounds, so a snapped neighbor's
            // border overlaps this tile's instead of stacking beside it into
            // a double-width line. Base pins `w`/`h` to the stored bounds
            // after this hook, so the growth rides on the min size, which
            // wins over the pinned size and which base leaves alone.
            .when(!tile.is_zoomed(), |this| {
                this.min_w(tile.bounds().size.width + px(1.))
                    .min_h(tile.bounds().size.height + px(1.))
            })
            .on_mouse_down(MouseButton::Left, {
                let tile = tile.clone();
                move |_, window, cx| tile.bring_to_front(window, cx)
            })
            // A gesture can end with the pointer anywhere, so both halves are
            // needed; each is a no-op unless this tile is the one moving.
            .on_mouse_up(MouseButton::Left, {
                let tile = tile.clone();
                move |_, window, cx| {
                    tile.end_move(window, cx);
                    tile.end_resize(window, cx);
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let tile = tile.clone();
                move |_, window, cx| {
                    tile.end_move(window, cx);
                    tile.end_resize(window, cx);
                }
            })
    }

    fn render_drag_bar(&self, tile: &TileContext, window: &mut Window, cx: &mut App) -> AnyElement {
        let node = tile.node();
        let handle = PanelHandle::of(tile.panel());
        let title_style = handle.and_then(|handle| handle.title_style(cx));

        h_flex()
            .id("drag-bar")
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h(DRAG_BAR_HEIGHT)
            .items_center()
            .gap_1()
            .pl_3()
            .pr_2()
            .when_some(title_style, |this, style| {
                // The tile frame does not clip its children, so a painted
                // title bar rounds its own top corners to stay inside the
                // frame's.
                this.bg(style.background)
                    .text_color(style.foreground)
                    .rounded_t(cx.theme().tile_radius)
            })
            .child(
                div()
                    .flex_1()
                    .min_w_16()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(panel_title(tile.panel(), window, cx)),
            )
            .children(handle.and_then(|handle| handle.title_suffix(window, cx)))
            .child(self.render_tile_controls(tile, window, cx))
            // A zoomed tile is not at its stored bounds, so there is nothing
            // for a move to mean; base refuses the gesture too.
            .when(!tile.is_zoomed(), |this| {
                this.cursor_grab()
                    .on_mouse_down(MouseButton::Left, {
                        let tile = tile.clone();
                        move |event: &MouseDownEvent, window, cx| {
                            tile.begin_move(event.position, window, cx);
                        }
                    })
                    .on_drag(DragMoving(node), |drag, _, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| drag.clone())
                    })
                    .on_drag_move({
                        let tile = tile.clone();
                        move |event: &DragMoveEvent<DragMoving>, window, cx| {
                            if event.drag(cx).0 != node {
                                return;
                            }
                            tile.move_to(event.event.position, window, cx);
                        }
                    })
            })
            .into_any_element()
    }

    fn render_resize_handles(
        &self,
        tile: &TileContext,
        _: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let bounds = tile.bounds();

        // A passive full-tile box so each handle is positioned against the
        // tile rather than against whatever the flow put it next to. It
        // registers no interaction of its own, so it does not shadow the panel
        // underneath.
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .child(
                self.resize_handle(tile, "left-resize-handle", ResizeSide::Left, |this| {
                    this.cursor_ew_resize()
                        .top_0()
                        .left(HANDLE_OFFSET)
                        .w(HANDLE_SIZE)
                        .h(bounds.size.height)
                }),
            )
            .child(
                self.resize_handle(tile, "right-resize-handle", ResizeSide::Right, |this| {
                    this.cursor_ew_resize()
                        .top_0()
                        .right(HANDLE_OFFSET)
                        .w(HANDLE_SIZE)
                        .h(bounds.size.height)
                }),
            )
            .child(
                self.resize_handle(tile, "top-resize-handle", ResizeSide::Top, |this| {
                    this.cursor_ns_resize()
                        .left_0()
                        .top(HANDLE_OFFSET)
                        .w(bounds.size.width)
                        .h(HANDLE_SIZE)
                }),
            )
            .child(
                self.resize_handle(tile, "bottom-resize-handle", ResizeSide::Bottom, |this| {
                    this.cursor_ns_resize()
                        .left_0()
                        .bottom(HANDLE_OFFSET)
                        .w(bounds.size.width)
                        .h(HANDLE_SIZE)
                }),
            )
            .child(
                Icon::new(IconName::ResizeCorner)
                    .size_3()
                    .absolute()
                    .right(px(1.))
                    .bottom(px(1.))
                    .text_color(cx.theme().muted_foreground.opacity(0.5)),
            )
            .child(self.resize_handle(
                tile,
                "corner-resize-handle",
                ResizeSide::BottomRight,
                |this| {
                    this.cursor_nwse_resize()
                        .right(HANDLE_OFFSET)
                        .bottom(HANDLE_OFFSET)
                        .size_3()
                },
            ))
            .into_any_element()
    }

    /// The old canvas wrapped a tile's panel in exactly this, and base draws
    /// the panel as a plain child, so without it a panel that does not size
    /// itself has no size.
    fn panel_frame(&self, tile: &TileContext, _: &mut Window, _: &mut App) -> Stateful<Div> {
        h_flex()
            .id(("tile-panel", tile.panel_id().as_u64()))
            .overflow_hidden()
            .size_full()
    }

    /// The canvas scrollbar.
    ///
    /// It has to be an overlay rather than one of the frame's own children:
    /// the frame is the scroll container and base appends the tiles after
    /// whatever the frame carries, so a scrollbar placed there would paint and
    /// hit-test underneath every tile.
    fn render_overlay(
        &self,
        content: Size<Pixels>,
        _: &mut Window,
        _: &mut App,
    ) -> Option<AnyElement> {
        Some(
            Scrollbar::new(&self.scroll_handle)
                .scroll_size(content)
                .when_some(self.shared.tiles_scrollbar_mode(), |this, mode| {
                    this.mode(mode)
                })
                .into_any_element(),
        )
    }

    fn grid_size(&self, cx: &App) -> Pixels {
        cx.theme().tile_grid_size
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use gpui::{Bounds, TestAppContext, point, size};
    use gpui_base::dock::{DockArea, DockLayout};

    use super::*;
    use crate::dock::{DockSkin, panel_handle, test_support::MeasuredProbe};

    /// A tile's panel view has to be given a size.
    ///
    /// Base draws the panel as an ordinary child of the tile frame, so
    /// without [`TilesSkin::panel_frame`]'s `size_full` the panel measures
    /// zero — the same defect the tab group had, in the other container. It
    /// would be invisible in the story example, whose panels happen to be
    /// `size_full` themselves.
    #[gpui::test]
    fn a_tiles_panel_gets_the_height_below_its_drag_bar(cx: &mut TestAppContext) {
        cx.update(|cx| {
            crate::init(cx);
        });
        let height = Rc::new(Cell::new(px(0.)));
        let (area, cx) = cx.add_window_view(|window, cx| {
            let skin = DockSkin::new(cx);
            DockArea::new("skin", None, window, cx).with_renderer(skin)
        });

        let measured = height.clone();
        let bounds = Bounds {
            origin: point(px(20.), px(20.)),
            size: size(px(380.), px(280.)),
        };
        cx.update(|window, cx| {
            let panel = MeasuredProbe::new(measured, cx);
            let layout = DockLayout::tiles().tile_view(panel_handle(panel), bounds, cx);
            area.update(cx, |area, cx| area.set_center(layout, window, cx));
        });
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let panel_height = height.get();
        assert!(
            panel_height > px(0.),
            "the tile's panel must receive height; it got {panel_height:?}"
        );
        // The tile is 280 tall; the drag bar takes 30 and the border 2, so the
        // panel gets the rest. Asserted as a range rather than a number so a
        // border-width change is not a test failure.
        assert!(
            panel_height > px(200.) && panel_height < bounds.size.height,
            "the panel should fill what the drag bar leaves of a 280px tile; \
             it got {panel_height:?}"
        );
    }
}
