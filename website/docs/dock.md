---
title: Dock
description: Production-ready dock layouts with styled tabs, split panes, edge docks, and persistent state.
order: -6
example: dock
---

# Dock

Dock builds application workspaces from draggable tab groups, nested splits, and collapsible left, right, and bottom docks. It is the layout foundation used by Longbridge in production, not an isolated UI demo.

`gpui-base` owns the data model, layout calculation, and drag-and-drop behavior. `gpui-component` supplies the polished controls and visual language. Use `gpui_kit::component::dock` when you want a Dock ready to fit into a real application.

For the renderer-independent architecture and custom-renderer API, see [Dock — gpui-base](/base/dock).

## Create a dock area

Create the area through `DockSkin`. Keep the returned skin if you want to change its appearance later.

```rust
use gpui_kit::component::dock::{DockArea, DockSkin};

struct Workspace {
    dock_area: Entity<DockArea>,
    dock_skin: Rc<DockSkin>,
}

impl Workspace {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (dock_area, dock_skin) =
            DockSkin::dock_area("main-dock", Some(1), window, cx);

        Self { dock_area, dock_skin }
    }
}
```

The optional version belongs to your saved layout schema. Increase it when your application can no longer restore an older layout.

## Define a panel

A styled Dock panel implements `BasePanel` for identity and persistence, and `Panel` for its title, tab, and toolbar presentation.

```rust
use gpui_kit::component::dock::{BasePanel, Panel, PanelEvent};

struct FilesPanel {
    focus_handle: FocusHandle,
}

impl EventEmitter<PanelEvent> for FilesPanel {}

impl Focusable for FilesPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BasePanel for FilesPanel {
    fn panel_name(&self) -> &'static str {
        "FilesPanel"
    }
}

impl Panel for FilesPanel {
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        "Files"
    }
}

impl Render for FilesPanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p_3().child("Project files")
    }
}
```

Wrap styled panels with `panel_handle`. This preserves the `gpui-component` panel chrome when the base Dock stores the panel behind its renderer-independent handle.

## Describe the initial layout

`DockLayout` is a value: compose it before a window exists, serialize it, compare it, or generate it from application state. Tabs and splits can be nested freely.

```rust
use gpui_kit::component::dock::{DockLayout, panel_handle};

let files = cx.new(|cx| FilesPanel {
    focus_handle: cx.focus_handle(),
});
let editor = cx.new(|cx| EditorPanel::new(cx));

let layout = DockLayout::h_split()
    .child(
        DockLayout::tabs().panel_view(panel_handle(files), cx),
        Some(px(240.)),
    )
    .child(
        DockLayout::tabs().panel_view(panel_handle(editor), cx),
        None,
    );

self.dock_area.update(cx, |area, cx| {
    area.set_center(layout, window, cx);
});
```

Use `h_split()` and `v_split()` for rows and columns, `tabs()` for a tab group, and `tiles()` for a free-positioning canvas. A `None` split size fills the remaining space.

The Dock area also supports left, right, and bottom regions through `DockPlacement`. Panels can be added, removed, activated, zoomed, and moved at runtime; user operations emit `DockEvent`, including `LayoutChanged` for persistence.

## Restore and persist layouts

Dump the entire workspace as `DockAreaState`, store it with Serde, then load it on the next launch.

```rust
use gpui_kit::component::dock::DockAreaState;

// Save.
let state = self.dock_area.read(cx).dump(cx);
let json = serde_json::to_string_pretty(&state)?;

// Restore.
let state: DockAreaState = serde_json::from_str(&json)?;
self.dock_area.update(cx, |area, cx| {
    area.load(state, window, cx)
})?;
```

Register every restorable panel name during application initialization. The registered factory recreates the styled panel behind a `panel_handle`.

```rust
register_panel(cx, "FilesPanel", |state, window, cx| {
    let panel = cx.new(|cx| FilesPanel::from_state(state, window, cx));
    panel_handle(panel)
});
```

Dock state retains compatibility with layouts saved by earlier releases. Keep a sensible fallback layout for removed application panels or deliberate schema changes.

## Style the workspace

`DockSkin` keeps rendering decisions outside the layout engine. You can configure the common panel presentation without changing Dock behavior:

```rust
self.dock_skin.set_panel_style(PanelStyle::default(), cx);
self.dock_skin.set_toggle_button_visible(true, cx);
self.dock_skin
    .set_tiles_scrollbar_mode(Some(ScrollbarMode::Auto), cx);
```

For complete control, implement the renderer traits in `gpui-base`. The same layout data and operations can then drive an entirely different Dock style.

## Runnable example

The repository includes a complete workspace with edge docks, runtime panel operations, layout persistence, and keyboard actions:

```sh
cargo run -p example-dock
```

See [`examples/dock/src/main.rs`](https://github.com/longbridge/gpui-kit/blob/main/examples/dock/src/main.rs) for the full implementation.
