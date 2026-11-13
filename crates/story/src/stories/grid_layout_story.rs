use gpui_kit::component::{
    ActiveTheme as _, Selectable as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonGroup},
    grid_layout::{
        GridArea, GridItem, GridLayoutEvent, GridLayoutState, grid_drag_handle, grid_layout,
    },
    h_flex,
    switch::Switch,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
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

fn default_layout() -> Vec<GridArea> {
    vec![
        GridArea::new("revenue")
            .col_span(6)
            .row_span(4)
            .min_span(3, 2),
        GridArea::new("orders").col(6).col_span(3).row_span(2),
        GridArea::new("visitors").col(9).col_span(3).row_span(2),
        GridArea::new("conversion")
            .col(6)
            .row(2)
            .col_span(6)
            .row_span(2)
            .max_span(8, 4),
        GridArea::new("refunds").row(4).col_span(4).row_span(3),
        GridArea::new("inventory")
            .col(4)
            .row(4)
            .col_span(4)
            .row_span(3),
        GridArea::new("notes")
            .col(8)
            .row(4)
            .col_span(4)
            .row_span(3)
            .locked(true),
    ]
}

pub struct GridLayoutStory {
    focus_handle: FocusHandle,
    dashboard: Entity<GridLayoutState>,
    handles: Entity<GridLayoutState>,
    last_change: Option<SharedString>,
    /// The tile the dashboard has selected, as its `Select` events report it.
    selected: Option<SharedString>,
    /// Shows the dashboard as one column without changing its layout.
    stacked: bool,
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
        let dashboard = cx.new(|cx| GridLayoutState::new(cx).default_areas(default_layout()));
        let handles = cx.new(|cx| {
            GridLayoutState::new(cx).cols(6).default_areas([
                GridArea::new("revenue").col_span(3).row_span(3),
                GridArea::new("orders").col(3).col_span(3).row_span(3),
                GridArea::new("visitors").row(3).col_span(2).row_span(2),
                GridArea::new("conversion")
                    .col(2)
                    .row(3)
                    .col_span(4)
                    .row_span(2),
            ])
        });
        let _subscriptions =
            vec![
                cx.subscribe(&dashboard, |this, _, event: &GridLayoutEvent, cx| {
                    let areas = match event {
                        GridLayoutEvent::Change(areas) => areas,
                        // A settings panel would follow this; the story names it.
                        GridLayoutEvent::Select(id) => {
                            this.selected = id.clone();
                            cx.notify();
                            return;
                        }
                        _ => return,
                    };
                    // An application would persist this; the story shows it.
                    this.last_change = Some(
                        areas
                            .iter()
                            .map(|area| {
                                format!(
                                    "{} ({}, {}) {}×{}",
                                    area.id, area.col, area.row, area.col_span, area.row_span
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
            selected: None,
            stacked: false,
            _subscriptions,
        }
    }

    fn render_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.dashboard.read(cx);
        let compaction = state.compaction_axis();
        let prevent_collision = state.is_collision_prevented();
        let editable = state.is_draggable() && state.is_resizable();

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
                Switch::new("grid-layout-one-column")
                    .label("One column")
                    .checked(self.stacked)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.stacked = *checked;
                        cx.notify();
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
                            .update(cx, |state, cx| state.set_areas(default_layout(), cx));
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
                                grid_layout("dashboard-grid", &self.dashboard, cx)
                                    .when(self.stacked, |grid| grid.reflow(1))
                                    .accessibility_label("Dashboard")
                                    .items(TILES.iter().map(|tile| {
                                        GridItem::new(tile.id)
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
                                        match &self.selected {
                                            Some(id) => format!("Selected: {id}").into(),
                                            None => {
                                                "Rearrange a tile to see the saved layout.".into()
                                            }
                                        }
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
                    .child(grid_layout("handle-grid", &self.handles, cx).items(
                        TILES[..4].iter().map(|tile| {
                            GridItem::new(tile.id)
                                .accessibility_label(tile.title)
                                .rounded(radius)
                                .drag_handle_only(true)
                                .child(
                                    tile_card(cx)
                                        .child(
                                            grid_drag_handle(&self.handles, tile.id)
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
