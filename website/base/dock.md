---
title: Dock
description: A dockable workspace — splits, tab groups, tiles, and edge docks — whose layout is pure data and whose appearance is entirely yours.
order: 6
example: dock
exampleKind: base
---

# Dock

A dockable workspace: nested splits, tab groups with draggable tabs, a free-positioning tiles canvas, and left/right/bottom docks that fold away. `gpui-base` owns all of the behavior and draws none of it.

The layout is not a tree of views. It is a value — a `PaneTree` — that you can build, compare, serialize, and edit without a `Window` or an `App` in sight. `DockArea` reconciles that value into live entities, and three renderer traits supply every pixel.

This page is long because Dock is the largest system in `gpui-base`. If you only need to stand one up, [Get started](#get-started) and [Supply the appearance](#supply-the-appearance) are enough.

## The model

Three container shapes, and nothing else:

| Container | Holds                            | Notes                                      |
| --------- | -------------------------------- | ------------------------------------------ |
| `Split`   | Other containers, along one axis | Each child slot has an optional fixed size |
| `Tabs`    | Panels, one displayed at a time  | Carries the displayed index                |
| `Tiles`   | Panels at free positions         | Each tile has bounds and a z-index         |

There is no leaf variant, so **a panel can only ever live inside a `Tabs` or a `Tiles`**. A region whose center is a single panel is still a `Tabs` holding one panel.

Four regions exist: the center, plus an optional left, right and bottom dock. Each is one independent `PaneTree`.

Two identities, both stable:

- **`NodeId`** addresses a container. It survives every edit and every normalization rule, so a container still present after a drag carries the id it had before. Ids are allocated globally, so a node id is unambiguous across all four regions.
- **`PanelId`** addresses a panel. It wraps the panel entity's `EntityId`, so it identifies that panel for as long as the entity lives — across any number of moves between groups and regions.

Neither the tree nor any node stores a GPUI entity handle.

### Key types

| Type                                                      | Role                                                                        |
| --------------------------------------------------------- | --------------------------------------------------------------------------- |
| `PaneTree`                                                | One region's layout, as pure data                                           |
| `PaneNode` / `PaneRef`                                    | A node, and the borrowed projection you `match` on                          |
| `NodeId` / `PanelId`                                      | Stable container and panel identity                                         |
| `DockArea`                                                | Owns the trees, reconciles them into entities, routes drags and persistence |
| `DockLayout`                                              | Describes a layout without constructing anything                            |
| `Panel`                                                   | What a dockable view implements — behavior only                             |
| `PanelView`                                               | Object-safe panel handle, `Arc<dyn PanelView>`                              |
| `TabGroup` / `TilesState`                                 | The entity behind a `Tabs` node and a `Tiles` node                          |
| `DockAreaRenderer` / `TabGroupRenderer` / `TilesRenderer` | Where every visual decision goes                                            |
| `DockContext` / `TabGroupContext` / `TileContext`         | Resolved state and callbacks handed to a renderer                           |
| `DockAreaState`                                           | The serializable form of a whole area                                       |

## Get started

```rust
use std::rc::Rc;
use gpui_kit::base::dock::{DockArea, DockLayout, DockPlacement};

let area = cx.new(|cx| {
    DockArea::new("workspace", Some(1), window, cx).with_renderer(Rc::new(MySkin))
});

area.update(cx, |area, cx| {
    area.set_center(
        DockLayout::h_split()
            .child(DockLayout::tabs().panel(files.clone()), Some(px(240.)))
            .child(DockLayout::tabs().panel(editor.clone()), None),
        window,
        cx,
    );
    area.set_dock(
        DockPlacement::Bottom,
        DockLayout::tabs().panel(terminal.clone()),
        window,
        cx,
    );
});
```

`DockArea::new` takes an id (yours, for your own persistence) and an optional schema version. An area built without `.with_renderer(...)` still docks, drags, resizes and persists — it simply draws nothing but the panels themselves.

## Describing a layout

`DockLayout` builds a tree without touching `window` or `cx`, because building a tree constructs no entities.

```rust
DockLayout::h_split()
    .child(DockLayout::tabs().panel(explorer.clone()), Some(px(240.)))
    .child(
        DockLayout::v_split()
            .child(
                DockLayout::tabs()
                    .panel(editor.clone())
                    .panel(diff.clone())
                    .active_index(1),
                None,
            )
            .child(DockLayout::tabs().panel(console.clone()), Some(px(180.))),
        None,
    )
```

| Builder                   | Produces                          |
| ------------------------- | --------------------------------- |
| `h_split()` / `v_split()` | A split along that axis           |
| `child(layout, size)`     | Adds a child container to a split |
| `tabs()`                  | A tab group                       |
| `panel(entity)`           | Adds a panel to a tab group       |
| `active_index(ix)`        | Which tab starts displayed        |
| `tiles()`                 | A tiles canvas                    |
| `tile(entity, bounds)`    | Places a panel on a canvas        |

Misuse — a panel added to a split, a child added to a tab group — trips a `debug_assert!` and is otherwise ignored.

### Slot sizes

The `size` in `child(layout, size)` is the slot's extent **along the split's axis**: width in an `h_split`, height in a `v_split`.

- `Some(px(240.))` fixes it.
- `None` leaves it unconstrained — the slot shares what is left with its other unconstrained siblings.

A layout with every slot `None` divides the space evenly. When a panel is later dropped beside an existing one with no size in mind, it takes half of what it lands next to.

### Normalization

Every edit runs one collapse pass to a fixpoint before returning. The rules, applied bottom up:

1. An empty `Tabs`, `Tiles`, or `Split` is removed from its parent.
2. A `Split` with one child is replaced by that child, which keeps its own `NodeId` and inherits the slot size.
3. A `Split` whose child is a `Split` of the same axis splices that child's children into itself, scaling their sizes to fill the slot.
4. `active_ix` is clamped to the panel count.
5. The center's root stays a `Split` even when empty; a dock's root is unconstrained.

Two consequences worth designing around. **You never need to avoid redundant nesting** — wrapping a node in a same-axis split is harmless, because rule 3 flattens it, which is why `split_at` needs no "reuse the parent" special case. And **there is no window in which a caller can observe a malformed tree**: no empty container, no one-child split, no out-of-range active index. Normalization is idempotent, so `normalize(normalize(t)) == normalize(t)`.

## Panels

The whole of a panel's obligation to base is a stable name.

```rust
struct FilesPanel { focus_handle: FocusHandle }

impl Panel for FilesPanel {
    fn panel_name(&self) -> &'static str {
        "FilesPanel"
    }
}

impl EventEmitter<PanelEvent> for FilesPanel {}
impl Focusable for FilesPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle { self.focus_handle.clone() }
}
impl Render for FilesPanel { /* ... */ }
```

`panel_name` identifies the panel in persisted layouts. **Once chosen, never change it** — it is the key a saved file is read back through.

### Every hook

| Method                   | Default    | When it runs                                               |
| ------------------------ | ---------- | ---------------------------------------------------------- |
| `panel_name()`           | _required_ | Any time the panel is identified or written out            |
| `visible(cx)`            | `true`     | Every render pass                                          |
| `closable(cx)`           | `true`     | Before a close is offered or applied                       |
| `zoomable(cx)`           | `true`     | Before a zoom is applied                                   |
| `on_added_to(group, ..)` | no-op      | When the panel joins a tab group, with a weak handle on it |
| `set_active(active, ..)` | no-op      | On each real edge of "is the displayed tab"                |
| `set_zoomed(zoomed, ..)` | no-op      | When the group displaying it zooms in or out               |
| `on_removed(..)`         | no-op      | When the panel leaves the dock for good                    |
| `dump(cx)`               | name only  | On `DockArea::dump`                                        |

### Lifecycle contracts

These are precise, and worth reading once:

**`set_active` fires on edges only.** It is called with the frame-end net state: exactly one notification per real change, delivered on the next tick. Never same-value repeats, never a false-then-true flip within one frame. A panel that is hidden but occupies the active slot still receives `true`, even though rendering falls back to the first visible panel.

**A removed panel is not told `false`.** `on_removed` is the deactivation signal. If you release resources in `set_active(false)`, release them in `on_removed` too.

**A moved panel never hears `on_removed`.** Dragging a panel from one group to another does not take it out of the dock, so it is told `on_added_to` again with the new group and nothing else. `on_removed` means gone: closed, or displaced by a wholesale `set_center`, `set_dock`, `remove_dock` or `load`.

**`on_added_to` precedes any `set_active`,** so a panel can store the handle and act on its first activation.

**`set_zoomed` reaches only the displayed panel.** A group has one zoom state and it is the visible panel that fills the dock. Panels in the group's other tabs hear nothing, and a panel that was not displayed when the zoom changed is never told retroactively.

**`closable` is permission, not a guarantee.** A container can still refuse — the last group of a dock does, so a dock cannot be emptied by closing.

**A hidden panel keeps its place.** `visible` returning `false` leaves the panel in the tree and in its group; it reappears where it was. A container whose panels are _all_ hidden gives up its slot, recursively — a nested split whose every leaf is hidden takes no space.

## The dock area

### Installing layouts

```rust
area.set_center(layout, window, cx);
area.set_dock(DockPlacement::Left, layout, window, cx);
area.remove_dock(DockPlacement::Left, window, cx);
```

Each replaces whatever was there. Panels that were displaced — and are not part of the new layout — receive `on_removed`.

### Adding and moving panels

```rust
area.add_panel(panel, DockPlacement::Left, Some(px(240.)), window, cx);
area.add_tile(panel, DockPlacement::Center, bounds, window, cx);
area.remove_panel(panel, window, cx);
area.move_panel(panel_id, target, window, cx);
area.split_at(node, panel_id, Placement::Right, window, cx);
```

`add_panel` lands the panel in the region's first tab group, creating the region if it has none. `add_tile` needs a tiles canvas in that region; without one it is a no-op. Both have `_view` variants taking an `Arc<dyn PanelView>` for callers holding an erased handle.

### Docks

```rust
area.has_dock(DockPlacement::Left);
area.is_dock_open(DockPlacement::Left);
area.toggle_dock(DockPlacement::Left, window, cx);
area.dock_size(DockPlacement::Left);
area.set_dock_size(DockPlacement::Left, px(280.), window, cx);
area.set_dock_collapsible(DockPlacement::Left, true, window, cx);
```

A closed dock keeps its tree and its size; reopening restores both.

### Zoom

A zoom names a **container**, not a panel — a group survives its displayed panel closing, and the next tab takes over still zoomed.

```rust
area.set_zoomed_in(node, window, cx);
area.set_zoomed_out(window, cx);
area.is_zoomed();
area.zoomed_group();   // Option<NodeId>
area.zoomed_tile();    // Option<PanelId>
```

The usual entry point is not these but `TabGroupContext::toggle_zoom` / `TileContext::toggle_zoom`, which a skin already has wherever it draws a zoom control. Zoom ends when the zoomed container leaves the dock, or when the container clears it — not when some unrelated panel is removed.

### Locking

`area.set_locked(true, window, cx)` freezes rearrangement: no drags, no drops, no closes. Reads and rendering are unaffected.

### Queries

```rust
area.layout(DockPlacement::Center);   // Option<&PaneTree>
area.panel(panel_id);                 // Option<&Arc<dyn PanelView>>
area.is_empty(DockPlacement::Left, cx);
area.is_locked();
area.bounds();
```

## Editing the tree directly

Everything above ultimately goes through `PaneTree`. You can drive it yourself — for a command palette, a keyboard shortcut, a restored session:

```rust
tree.insert_panel(panel, InsertTarget::Tabs { node, ix: None, activate: true });
tree.remove_panel(panel);
tree.move_panel(panel, target);
tree.split(node, panel, Placement::Right, Some(px(320.)));
tree.set_active(node, 2);
tree.set_sizes(node, vec![Some(px(200.)), None]);
tree.set_tile_bounds(panel, bounds);
tree.bring_to_front(panel);
```

`InsertTarget` says where a panel lands:

| Variant                           | Meaning                                            |
| --------------------------------- | -------------------------------------------------- |
| `Tabs { node, ix, activate }`     | Into an existing tab group, optionally at an index |
| `Split { node, placement, size }` | Beside a node, in a new tab group                  |
| `Tile { node, bounds }`           | Onto a tiles canvas at those bounds                |

Every edit returns an `EditResult`: `changed()`, plus `created_nodes()`, `removed_nodes()`, `removed_panels()`, `activated()`, `deactivated()`. **`removed_panels` excludes moves** — a moved panel's entity survives, so it must not receive `on_removed`.

Reading a tree:

```rust
tree.root();                      // &PaneNode
tree.node_ids();                  // Vec<NodeId>, pre-order
tree.panels();                    // impl Iterator<Item = PanelId>
tree.find_node(node_id);          // Option<&PaneNode>
tree.find_panel_node(panel_id);   // Option<NodeId>

match node.kind() {
    PaneRef::Split { axis, children, sizes } => { /* ... */ }
    PaneRef::Tabs { panels, active_ix } => { /* ... */ }
    PaneRef::Tiles { panels } => { /* ... */ }
}
```

## Supply the appearance

Nothing in `gpui_kit::base::dock` paints a color, a border, or a size. Three traits carry appearance in.

### `DockAreaRenderer`

| Method                | Supplies                                                           | Default                        |
| --------------------- | ------------------------------------------------------------------ | ------------------------------ |
| `frame`               | The area's outermost element                                       | Bare `div`                     |
| `center_frame`        | The column holding the center and bottom dock                      | Bare `div`                     |
| `split_frame`         | One split's frame                                                  | Bare `div`                     |
| `render_split_handle` | The divider between two slots                                      | `None` → base's one-pixel line |
| `render_dock`         | One dock's chrome: title strip, collapse affordance, resize handle | The content, unwrapped         |
| `build_placeholder`   | The stand-in for a panel this build cannot construct               | `None` → draws nothing         |
| `tab_group_renderer`  | _required_                                                         | —                              |
| `tiles_renderer`      | _required_                                                         | —                              |

### `TabGroupRenderer`

| Method                  | Supplies                                 | Default                      |
| ----------------------- | ---------------------------------------- | ---------------------------- |
| `frame`                 | The group's outer element                | Bare `div`                   |
| `content_frame`         | The element the displayed panel sits in  | Bare `div`                   |
| `render_tab_bar`        | The tab strip                            | Nothing                      |
| `render_active_panel`   | How the displayed panel is placed        | The panel, filling the frame |
| `render_drop_indicator` | The highlight showing where a drop lands | Nothing                      |
| `render_empty`          | What an empty group shows                | Nothing                      |

### `TilesRenderer`

| Method                  | Supplies                                 | Default     |
| ----------------------- | ---------------------------------------- | ----------- |
| `frame`                 | The canvas                               | Bare `div`  |
| `tile_frame`            | One tile's outer element                 | Bare `div`  |
| `render_drag_bar`       | _required_ — the strip that moves a tile | —           |
| `render_resize_handles` | The tile's resize affordances            | Nothing     |
| `panel_frame`           | The element the panel sits in            | Bare `div`  |
| `render_overlay`        | Anything drawn above every tile          | Nothing     |
| `grid_size`             | Snap granularity                         | No snapping |

### Contexts

A renderer never sees a drag event or a mouse position. Base attaches drag sources, drop hit-testing, focus and keyboard handling to the very elements the renderer returns, and hands it resolved state plus callbacks:

**`TabGroupContext`** — `node()`, `panels()`, `active_ix()`, `active_panel()`, `drop_indicator()`, `is_zoomed()`, `is_collapsed()`, `can_close()`, `is_locked()`, `is_draggable()`, `is_droppable()`; and the actions `select_tab()`, `close()`, `toggle_zoom()`, `drag_panel()`, `drop_panel()`, `drop_item()`.

**`TileContext`** — `node()`, `panel()`, `panel_id()`, `bounds()`, `z_index()`, `is_moving()`, `is_resizing()`, `can_close()`, `is_zoomed()`, `can_zoom()`; and `begin_move()` / `move_to()` / `end_move()`, `begin_resize()` / `resize_to()` / `end_resize()`, `bring_to_front()`, `toggle_zoom()`, `close()`.

**`DockContext`** — `placement()`, `size()`, `is_open()`, `is_collapsible()`; and `toggle()`, `resize_to()`.

```rust
impl TabGroupRenderer for MySkin {
    fn render_tab_bar(&self, group: &TabGroupContext, _: &mut Window, cx: &mut App) -> AnyElement {
        h_flex()
            .children(group.panels().iter().enumerate().map(|(ix, panel)| {
                div()
                    .child(my_title(panel, cx))
                    .when(ix == group.active_ix(), |this| this.font_semibold())
                    .on_click({
                        let group = group.clone();
                        move |_, window, cx| group.select_tab(ix, window, cx)
                    })
            }))
            .into_any_element()
    }
}
```

Every hook is optional in the same way: decline one and you get base's minimum for it. `render_split_handle` is the clearest case — return `None` and the divider falls back to a one-pixel line colored from `Theme::resizable`, so a skin with no opinion about dividers implements nothing, while one that has an opinion replaces the paint without touching the hit area, the cursor, or the drag.

## Drag and drop

A tab drag has three parts, and a skin supplies only the middle one.

**Starting.** `TabGroupContext::drag_panel(ix, cx)` returns a `DragPanel` if that tab may be dragged (it declines when the group is locked, or when the panel is the last one holding a dock open). Hand it to GPUI's `on_drag`. The preview view you return is yours; base's own `DragPanel` renders nothing, because a preview is appearance.

**Landing.** While a drag hovers, base resolves where it would land and exposes it as `TabGroupContext::drop_indicator()` — a `DropIndicator` carrying the rectangle the panel would occupy. Paint it in `render_drop_indicator`; the coordinates are relative to the content frame, so that frame must be positioned.

**Applying.** `drop_panel()` on release turns the hover into a `TabGroupEvent::Drop { panel, source, target }`, which the area applies as a single `PaneTree::move_panel`. Dropping onto the middle merges into the group; dropping towards an edge splits there; dropping onto the tab strip inserts at that index.

**Host-owned drags.** Anything of your own can be dropped into the dock. Wrap it in `AnyDrag`, and `drop_item()` reports it as `DockEvent::DragDrop { item, target }` where `target` says whether it landed on a tab group or the tiles canvas. The dock does not interpret the payload.

## Tiles canvas

A `Tiles` node holds panels at free positions instead of stacking them as tabs. `TilesState` owns the geometry: moving, resizing from any of five sides, magnetic snapping to neighbors and to a grid, z-ordering, per-gesture undo history, and single-tile zoom.

A tile's drag bar is the one required piece of a `TilesRenderer`, because a tile with no drag bar cannot be moved. Everything else has a default.

```rust
impl TilesRenderer for MySkin {
    fn render_drag_bar(&self, tile: &TileContext, _: &mut Window, _: &mut App) -> AnyElement {
        div()
            .h(px(28.))
            .on_mouse_down(MouseButton::Left, {
                let tile = tile.clone();
                move |event, window, cx| tile.begin_move(event.position, window, cx)
            })
            .into_any_element()
    }

    fn grid_size(&self, _: &App) -> Pixels {
        px(8.)
    }
}
```

Snapping geometry is exported for skins that want to preview a drop: `magnetic_snap`, `snap_edge`, `round_to_grid`, `compute_resized_bounds`, `apply_boundary_constraints`.

## Events

| Emitter      | Event                                                                               | Meaning                                                                                                          |
| ------------ | ----------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `DockArea`   | `LayoutChanged`                                                                     | Something changed. Fires on **every** edit, including each step of a tile drag — debounce before writing to disk |
| `DockArea`   | `DragDrop { item, target }`                                                         | A host-owned drag landed                                                                                         |
| `TabGroup`   | `Drop` / `DragDrop` / `ClosePanel` / `ActiveChanged` / `ZoomIn` / `ZoomOut`         | A group's intent, applied by the area                                                                            |
| `TilesState` | `BoundsChanged` / `BringToFront` / `ClosePanel` / `DragDrop` / `ZoomIn` / `ZoomOut` | A canvas's intent                                                                                                |
| `Panel`      | `ZoomIn` / `ZoomOut` / `LayoutChanged`                                              | A panel's own signal                                                                                             |

Container events are the container asking the area for something; the area is what actually edits the tree. A host normally subscribes only to `DockEvent`.

## Persistence

```rust
let state: DockAreaState = area.read(cx).dump(cx);
let json = serde_json::to_string(&state)?;

let state: DockAreaState = serde_json::from_str(&json)?;
area.update(cx, |area, cx| area.load(state, window, cx))?;
```

`DockAreaState` carries the version you passed to `DockArea::new`, the center, and each dock with its placement, size and open state. Every node writes its shape and every panel writes whatever its `dump` returned.

Panels are rebuilt through a global registry, keyed by `panel_name`:

```rust
register_panel(cx, "FilesPanel", |context, window, cx| {
    let state = context.state();   // the PanelState this panel dumped
    Arc::new(FilesPanel::restore(state, cx)) as Arc<dyn PanelView>
});
```

Three behaviors worth knowing:

- **An unregistered panel is not dropped.** It becomes a placeholder carrying the original state forward, so a layout saved by a build that had a panel yours does not know still round-trips intact instead of losing it.
- **Slot sizes are resolved on the way out.** `dump` writes the sizes the split is actually drawn at, not the ones the tree was built from, and never writes a zero.
- **`LayoutChanged` fires far more often than you want to save.** Debounce, or save on a timer or on window close.

## How this compares

Docking layouts are well-trodden. Where implementations differ is how far the layout engine is separated from what draws it — and that choice has consequences you can observe.

### Architecture

| Project                                                          | Stack            | Engine and rendering                                                          | What a consumer can change                                             |
| ---------------------------------------------------------------- | ---------------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| [Qt](https://doc.qt.io/qt-6/qdockwidget.html)                    | C++, retained    | `QMainWindow` owns four fixed dock areas; a `QDockWidget` _is_ a widget       | Subclass the widget; styling via QSS                                   |
| [AvalonDock](https://github.com/Dirkster99/AvalonDock)           | C#/WPF, retained | `LayoutRoot` tree of layout elements                                          | XAML templates and themes                                              |
| [Dear ImGui](https://github.com/ocornut/imgui/wiki/Docking)      | C++, immediate   | `DockSpace()` is a region any window may dock into; nodes are engine-internal | Style vars and colors                                                  |
| [egui_dock](https://docs.rs/egui_dock/)                          | Rust, immediate  | `DockState` holds surfaces, each with a `Tree` of `Node`s                     | `TabViewer` renders tab bodies; a `Style` struct tunes the chrome      |
| [dockview](https://dockview.dev/)                                | TypeScript, web  | Framework-agnostic engine behind thin adapters                                | CSS variables, theme object, replace the tab component                 |
| [FlexLayout](https://github.com/caplin/FlexLayout)               | TypeScript, web  | JSON model beside a React renderer                                            | `onRenderTab` callbacks and CSS                                        |
| [golden-layout](https://golden-layout.com/)                      | TypeScript, web  | Engine owns its DOM outright                                                  | CSS overrides                                                          |
| [rc-dock](https://github.com/ticlo/rc-dock)                      | TypeScript, web  | `BoxData` / `PanelData` / `TabData` model                                     | Custom tab rendering and CSS                                           |
| [VS Code](https://code.visualstudio.com/api/ux-guidelines/panel) | TypeScript, app  | Workbench owns the layout                                                     | Contributed views, themed via CSS                                      |
| [Zed](https://zed.dev/)                                          | Rust, app        | `PaneGroup` built into the application                                        | Not reusable outside its host                                          |
| **`gpui-base`**                                                  | Rust, retained   | Pure-data `PaneTree`; the engine paints nothing                               | Renderer traits return elements — there is no default look to override |

Three families are visible in that table. **Application-owned** engines (VS Code, Zed) are the most capable and the least reusable — you cannot lift them out of their host. **Widget-tree** engines (Qt, AvalonDock, golden-layout) make the dockable thing a widget, so the layout _is_ the view hierarchy. **Model-and-renderer** engines (FlexLayout, rc-dock, dockview, egui_dock, and this one) keep a separate description of the layout and hand rendering to something else.

`gpui-base` sits at the far end of the third family: the engine paints nothing at all. In a library that draws its own chrome, customization is a set of overrides layered onto a default appearance, and you are limited to the seams it chose to expose. Here a renderer returns elements and base attaches behavior to _those_ elements, so two unrelated appearances can sit over one behavior — `crates/component/src/dock` and the example below are exactly that.

### The closest relative

[egui_dock](https://docs.rs/egui_dock/) is worth a paragraph of its own, because it arrives at nearly the same shape from the other side of the retained/immediate divide. It keeps a `DockState` holding surfaces, each surface a `Tree`, each tree a hierarchy of `Node`s split into leaf and split variants — which is this design, down to the vocabulary, and it even calls its render entry point `DockArea`.

Two differences matter. Its `TabViewer` renders **tab bodies**, while the crate itself draws the tab bars and splitters through a `Style` struct; here the split is the other way round — panels render themselves, and the skin draws all the chrome, with no `Style` struct because there is nothing built in to configure. And because egui is immediate-mode, its tree is walked and re-emitted every frame by construction; here the tree is a value that changes only when edited, and reconciliation against a stable `NodeId` cache is what keeps entities alive across edits.

egui_dock also has something this does not: **undocking a tab into a floating OS window**, modeled as additional surfaces. That is a real capability gap, not a design difference.

### What the data model buys

The layout being a value rather than a widget tree is not an aesthetic preference. Three properties follow:

**A drag does not reset what it did not touch.** When containers _are_ views — the Qt and AvalonDock model — rearranging the layout means creating and dropping views, so a drag can reset state (scroll offsets, focus, in-progress input) in panels that merely shared a parent with the one being moved. Here identity is a `NodeId` that survives every edit and every normalization rule, so reconciliation is a diff against the entity cache: a steady-state pass creates and drops nothing.

**Collapse is a pure function, not a deferred cascade.** When the last panel leaves a group, the group must remove itself from its parent, which may empty the parent in turn. With containers as views this is mutual recursion between two types reaching upward through parent handles — and those handles must be installed after construction, which in GPUI means a deferred pass, which means a window in which the tree disagrees with itself. `normalize` is one post-order pass to a fixpoint: no parent pointers, no deferred work, and the tree is self-consistent the instant an edit returns.

**Editing costs no rendering.** `insert_panel`, `move_panel`, `split` and the rest operate on a value. They allocate no entities and request no layout, so a sequence of edits can run and be inspected before anything is drawn. The same property is why the whole layout algebra is tested as plain `#[test]` with no `TestAppContext` — which is why the collapse rules have the coverage they do.

The cost, stated plainly: an edit clones the tree once to diff it, and normalization walks it until it reaches a fixpoint (two passes on realistic layouts, with a hard ceiling well above that). For dock-sized trees — tens of nodes — both are negligible against a frame, and they buy the three properties above. This would be the wrong shape for a structure with thousands of nodes.

### Naming

The vocabulary follows the neighborhood where it can, which matters if you already know one of these systems:

| Concept                   | Qt                   | egui_dock   | dockview      | VS Code         | Zed         | `gpui-base` |
| ------------------------- | -------------------- | ----------- | ------------- | --------------- | ----------- | ----------- |
| Window-level container    | `QMainWindow`        | `DockState` | `DockviewApi` | Workbench       | `Workspace` | `DockArea`  |
| Tree arranging containers | —                    | `Tree`      | Gridview      | —               | `PaneGroup` | `PaneTree`  |
| Tree node                 | —                    | `Node`      | —             | —               | `Member`    | `PaneNode`  |
| Tab group                 | (stacked docks)      | `LeafNode`  | Group         | View Container  | `Pane`      | `TabGroup`  |
| Content in a tab          | `QDockWidget`        | `Tab`       | `Panel`       | `View`          | `Item`      | `Panel`     |
| Edge region               | `Qt::DockWidgetArea` | —           | —             | Panel / Sidebar | `Dock`      | `Dock`      |

One caution, because the field uses the word inconsistently: here a **`Panel` is the dockable content**, matching dockview. VS Code calls the _bottom region_ a panel; rc-dock calls the _tab container_ one. Translate the word before porting concepts from either.

Qt is the outlier worth noting: it has no separate node type at all, because `QDockWidget` is both the content and the thing the layout arranges. That is the design this one is furthest from.

## Runnable example

Everything above, on `gpui-base` alone — panels, a layout, and a skin implementing all three
renderer traits. Nothing in it depends on `gpui-component`, which is the point: base is usable on
its own, and a host that wants a different look writes a different skin.

```bash
cargo run -p gpui-base dock
```

Source: [`showcase/components/dock.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/dock.rs). It is the same file the preview at the top of this page compiles to WebAssembly.

## Integration checklist

- Give every panel a `panel_name` you will never change, and register a builder for it before calling `load`.
- Install a renderer, or accept that nothing but the panels themselves is drawn.
- Release resources in `on_removed`, not only in `set_active(false)` — a departing panel is never told `false`.
- Debounce `DockEvent::LayoutChanged` before persisting; it fires on every step of a drag.
- Prefer `visible` over removal when a panel should come back in the same place.
- Position the element you return from `content_frame`, or the drop indicator will have nothing to anchor to.
