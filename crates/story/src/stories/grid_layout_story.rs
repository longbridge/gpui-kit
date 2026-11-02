use gpui_kit::component::{
    ActiveTheme as _, Selectable as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonGroup},
    grid_layout::{
        GridDragHandle, GridItem, GridLayout, GridLayoutEvent, GridLayoutItem, GridLayoutState,
    },
    h_flex,
    switch::Switch,
    v_flex,
};
use gpui_kit::*;

use crate::section;

/// A dashboard tile: what it shows, and where it starts.
struct Tile {
    id: &'static str,
    title: &'static str,
    value: &'static str,
    detail: &'static str,
}

const TILES: [Tile; 7] = [
    Tile {
        id: "revenue",
        title: "Revenue",
        value: "$48,210",
        detail: "Up 12% from last week",
    },
    Tile {
        id: "orders",
        title: "Orders",
        value: "1,284",
        detail: "312 awaiting shipment",
    },
    Tile {
        id: "visitors",
        title: "Visitors",
        value: "23,906",
        detail: "Peak at 14:00",
    },
    Tile {
        id: "conversion",
        title: "Conversion",
        value: "5.4%",
        detail: "Target 6%",
    },
    Tile {
        id: "refunds",
        title: "Refunds",
        value: "18",
        detail: "Two need review",
    },
    Tile {
        id: "inventory",
        title: "Inventory",
        value: "92%",
        detail: "Three products low",
    },
    Tile {
        id: "notes",
        title: "Notes",
        value: "Pinned",
        detail: "Locked in place: other tiles flow around it",
    },
];

fn default_layout() -> Vec<GridItem> {
    vec![
        GridItem::new("revenue", 0, 0, 6, 4).with_min_size(3, 2),
        GridItem::new("orders", 6, 0, 3, 2),
        GridItem::new("visitors", 9, 0, 3, 2),
        GridItem::new("conversion", 6, 2, 6, 2).with_max_size(8, 4),
        GridItem::new("refunds", 0, 4, 4, 3),
        GridItem::new("inventory", 4, 4, 4, 3),
        GridItem::new("notes", 8, 4, 4, 3).with_locked(true),
    ]
}

pub struct GridLayoutStory {
    focus_handle: FocusHandle,
    dashboard: Entity<GridLayoutState>,
    handles: Entity<GridLayoutState>,
    last_change: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for GridLayoutStory {
    fn title() -> &'static str {
        "GridLayout"
    }

    fn description() -> &'static str {
        "A grid whose items can be dragged and resized into place."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for GridLayoutStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl GridLayoutStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let dashboard = cx.new(|cx| GridLayoutState::new(cx).with_items(default_layout()));
        let handles = cx.new(|cx| {
            GridLayoutState::new(cx).with_columns(6).with_items([
                GridItem::new("revenue", 0, 0, 3, 3),
                GridItem::new("orders", 3, 0, 3, 3),
                GridItem::new("visitors", 0, 3, 2, 2),
                GridItem::new("conversion", 2, 3, 4, 2),
            ])
        });
        let _subscriptions =
            vec![
                cx.subscribe(&dashboard, |this, _, event: &GridLayoutEvent, cx| {
                    let GridLayoutEvent::Change(items) = event;
                    // An application would persist this; the story shows it.
                    this.last_change = Some(
                        items
                            .iter()
                            .map(|item| {
                                format!(
                                    "{} ({}, {}) {}×{}",
                                    item.id, item.x, item.y, item.w, item.h
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("  ·  ")
                            .into(),
                    );
                    cx.notify();
                }),
            ];

        Self {
            focus_handle: cx.focus_handle(),
            dashboard,
            handles,
            last_change: None,
            _subscriptions,
        }
    }

    fn render_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.dashboard.read(cx);
        let compaction = state.compaction();
        let prevent_collision = state.prevent_collision();
        let editable = state.draggable() && state.resizable();

        h_flex()
            .w_full()
            .flex_wrap()
            .gap_4()
            .child(
                ButtonGroup::new("grid-layout-compaction")
                    .outline()
                    .small()
                    .child(
                        Button::new("compaction-vertical")
                            .label("Vertical")
                            .selected(compaction == Some(Axis::Vertical)),
                    )
                    .child(
                        Button::new("compaction-horizontal")
                            .label("Horizontal")
                            .selected(compaction == Some(Axis::Horizontal)),
                    )
                    .child(
                        Button::new("compaction-none")
                            .label("Free")
                            .selected(compaction.is_none()),
                    )
                    .on_click(cx.listener(|this, selected: &Vec<usize>, _, cx| {
                        let compaction = match selected.first() {
                            Some(0) => Some(Axis::Vertical),
                            Some(1) => Some(Axis::Horizontal),
                            _ => None,
                        };
                        this.dashboard
                            .update(cx, |state, cx| state.set_compaction(compaction, cx));
                    })),
            )
            .child(
                Switch::new("grid-layout-prevent-collision")
                    .label("Prevent collision")
                    .checked(prevent_collision)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.dashboard
                            .update(cx, |state, cx| state.set_prevent_collision(*checked, cx));
                    })),
            )
            .child(
                Switch::new("grid-layout-editable")
                    .label("Editable")
                    .checked(editable)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.dashboard.update(cx, |state, cx| {
                            state.set_draggable(*checked, cx);
                            state.set_resizable(*checked, cx);
                        });
                    })),
            )
            .child(
                Button::new("grid-layout-reset")
                    .outline()
                    .small()
                    .label("Reset layout")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.last_change = None;
                        this.dashboard
                            .update(cx, |state, cx| state.set_items(default_layout(), cx));
                    })),
            )
    }
}

/// The surface every tile shares; the grid item supplies only the place.
fn tile_card(cx: &App) -> Div {
    v_flex()
        .size_full()
        .overflow_hidden()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
}

fn tile_body(tile: &Tile, cx: &App) -> impl IntoElement {
    v_flex()
        .gap_1()
        .child(div().text_2xl().font_semibold().child(tile.value))
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(tile.detail),
        )
}

fn tile_title(tile: &Tile, cx: &App) -> impl IntoElement {
    div()
        .text_sm()
        .font_medium()
        .text_color(cx.theme().muted_foreground)
        .child(tile.title)
}

impl Render for GridLayoutStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let radius = cx.theme().radius_lg;

        v_flex()
            .w_full()
            .gap_6()
            .child(
                section("Dashboard")
                    .description(
                        "Drag a tile to move it and drag its corner to resize it. Click a tile, \
                         then use the arrow keys to move it and Shift with the arrow keys to \
                         resize it.",
                    )
                    .w_full()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_3()
                            .child(self.render_controls(cx))
                            .child(
                                GridLayout::new("dashboard-grid", &self.dashboard)
                                    .accessibility_label("Dashboard")
                                    .items(TILES.iter().map(|tile| {
                                        GridLayoutItem::new(tile.id)
                                            .accessibility_label(tile.title)
                                            .rounded(radius)
                                            .child(
                                                tile_card(cx)
                                                    .p_4()
                                                    .gap_3()
                                                    .justify_between()
                                                    .child(tile_title(tile, cx))
                                                    .child(tile_body(tile, cx)),
                                            )
                                    })),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .text_color(cx.theme().muted_foreground)
                                    .child(self.last_change.clone().unwrap_or_else(|| {
                                        "Rearrange a tile to see the saved layout.".into()
                                    })),
                            ),
                    ),
            )
            .child(
                section("Drag Handle")
                    .description(
                        "Tiles move only by their header, leaving the body free for its own \
                         pointer interaction.",
                    )
                    .w_full()
                    .child(GridLayout::new("handle-grid", &self.handles).items(
                        TILES[..4].iter().map(|tile| {
                            GridLayoutItem::new(tile.id)
                                .accessibility_label(tile.title)
                                .rounded(radius)
                                .drag_handle_only(true)
                                .child(
                                    tile_card(cx)
                                        .child(
                                            GridDragHandle::new(&self.handles, tile.id)
                                                .px_4()
                                                .py_2()
                                                .border_b_1()
                                                .border_color(cx.theme().border)
                                                .bg(cx.theme().muted)
                                                .child(tile_title(tile, cx)),
                                        )
                                        .child(div().p_4().child(tile_body(tile, cx))),
                                )
                        }),
                    )),
            )
    }
}
