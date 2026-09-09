---
title: Dock and Panels
description: A dockable layout drawn entirely by script — panels that survive a restart, chrome you draw yourself, and commands instead of callbacks.
order: 13
---

# Dock and Panels

A View that can only fill a window is not much of an application. A **dock area** turns a script View into a _panel_: draggable, dockable, zoomable, and still where the user left it after a restart.

```js
import { View, div } from "gpui-kit";
import { DockArea, dock_area, v_flex } from "gpui-base";

class Notes extends View {
  render() {
    return div().p(16).child("Notes");
  }
}

export default class Workspace extends View {
  init(_props, cx) {
    DockArea.register_panel("notes", Notes);
    this.dock = DockArea.new("workspace");
    this.dock.add_panel(cx.new(Notes), {
      name: "notes",
      placement: "left",
      size: 240,
    });
  }

  render() {
    return dock_area(this.dock).size_full();
  }
}
```

That already docks, drags, resizes, zooms and persists. It draws no tab bar, because **base draws no chrome at all** — see [Drawing the chrome](#drawing-the-chrome).

## What base brings, and what it does not

`gpui_kit::base::dock` has the hard half of a docking system: a layout that is **pure data**, a `PanelRegistry` that rebuilds a panel from a name in a persisted file, and a per-panel payload that rides along with it. Containers are addressed by a stable node id and panels by a stable panel id, so a drag rearranges a value rather than tearing down and rebuilding Views.

What it does not have is a look. The engine paints nothing — no tab bar, no dock frame, no drag handle, no drop hint — and hands every one of those back to you as a callback that returns elements. That is not a limitation to work around; it is why the whole thing is usable from a script at all. Appearance is not a set of overrides on a default look, because there is no default look.

## The area is retained

`DockArea.new(id)` creates state that lives across frames, like `InputState` does, and for a reason none of the other handles share: **the layout is what the user changed.** A drag, a resize, a closed tab and a collapsed dock all happen without your script rendering. A dock rebuilt from a description would put every one of them back the way the last render described it.

So it is created once, in `init`, and `render` only _draws_ it:

```js
init() { this.dock = DockArea.new("workspace", { version: 1 }); }
render() { return dock_area(this.dock).size_full(); }
```

`DockArea.new` needs a live host call, so it belongs in `init` or an event handler — never in `render`. So does every method that changes the layout; calling one from `render` is refused where it was written rather than producing a frame that draws one layout and describes another.

## Edits take effect when the call returns

A panel's body comes from `cx.new(Class)`, which is itself still being constructed when you hand it over, and `load` builds panels of its own. Neither can happen while your script is running. So **every edit is queued and applied once the call that made it has returned**, in the order the calls were made.

The practical consequence is one line long: `panels()` and `dump()` read the layout as it was _before_ this turn's edits.

```js
init(_props, cx) {
  this.dock = DockArea.new("workspace");
  this.dock.add_panel(cx.new(Notes), { name: "notes" });
  this.dock.panels();          // still empty — the add has not been applied
  this.dock.on("layout_changed", (cx) => {
    this.dock.panels();        // three panels, a dock size, a moved tab
    cx.notify();
  });
}
```

`layout_changed` fires on every edit, including each step of a tile drag, so save on a timer rather than on the event.

## Panels

A panel is a View that a dock happens to be holding. `add_panel` takes the View and says where it goes:

```js
this.dock.add_panel(cx.new(Editor, { file }), {
  name: "editor", // required — what a saved layout files it under
  placement: "center", // "center" | "left" | "right" | "bottom"
  size: 240, // seeds the dock's extent when the panel is the first in it
  closable: true,
  zoomable: true,
  visible: true,
});
```

`name` is required because it is not decoration: it is what a saved layout writes and what `register_panel` finds the class again by. It is namespaced for you — `shell:<application>/<name>` — so two applications that both call a panel `inbox` never collide, and no script panel can shadow a host one.

`panels()` reports what is there, including where:

```js
this.dock.panels();
// [{ id, name, placement, node, index, active, visible, closable, zoomable }, …]
```

`id` is what `remove_panel(id)` takes, and what a close button hands to `close_panel`.

## Surviving a restart

Two halves, and you need both.

**Register the class**, so a saved layout can rebuild it:

```js
DockArea.register_panel("editor", Editor);
```

**Save and restore the layout**, which is plain data:

```js
init(_props, cx) {
  DockArea.register_panel("editor", Editor);
  this.dock = DockArea.new("workspace", { version: 1 });

  const saved = localStorage.getItem("layout");
  if (saved) this.dock.load(JSON.parse(saved));
  else this.dock.add_panel(cx.new(Editor), { name: "editor" });

  this.dock.on("layout_changed", () =>
    localStorage.setItem("layout", JSON.stringify(this.dock.dump())));
}
```

A panel's own state rides along with its position. Two optional methods on the View class carry it:

| Method              | When                            | Note                                                                                          |
| ------------------- | ------------------------------- | --------------------------------------------------------------------------------------------- |
| `serialize()`       | The layout is saved             | Runs **without a host call**: return plain data and touch nothing else — no entities, no `cx` |
| `deserialize(data)` | Right after the View is rebuilt | A real host call, so this one may touch entities                                              |

`version` is yours to bump when the shape of what you save changes; base refuses to load a layout written under a different one, so an old file is ignored rather than half-understood.

### An uninstalled application keeps its place

This is the property worth designing around.

If nothing is registered under a panel's name — the application was uninstalled, or a class was renamed — the panel is **not dropped**. A draw-nothing placeholder stands in and reports the state it was handed, so the next save writes the panel — name, payload and position — back out unchanged. Uninstall an application, use the window for a week, reinstall it: its panels come back where they were, with the state they had.

The same holds one step further in. A panel that _is_ registered but whose class throws on construction is carried forward the same way, so a broken script costs that panel's contents for the session rather than its place in the layout.

## Drawing the chrome

Six handlers, all optional, hung on the `dock_area(...)` element:

| Handler                          | Draws                                       |
| -------------------------------- | ------------------------------------------- |
| `tab_bar(group => …)`            | The tab bar above a group's displayed panel |
| `empty_group(group => …)`        | What a group with no displayed panel shows  |
| `drop_indicator(drop => …)`      | Where a dragged panel would land            |
| `dock(dock => …)`                | One dock's frame around its content         |
| `tile_drag_bar(tile => …)`       | The strip a tile is dragged by              |
| `tile_resize_handles(tile => …)` | A tile's resize affordances                 |

Each is first called from inside GPUI's layout pass and is given base's **resolved** state — never a drag event, a mouse position or a hit test, because base attaches all of that to the elements it gets back. The resulting description is cached by handler and resolved state, so unchanged frames replay it in Rust without entering JavaScript.

```js
dock_area(this.dock)
  .size_full()
  .tab_bar((group, cx) =>
    h_flex()
      .h(30)
      .bg(cx.theme().colors.secondary)
      .children(
        group.tabs
          .filter((tab) => tab.visible)
          .map((tab) =>
            h_flex()
              .id("tab-" + tab.id)
              .px(10)
              .items_center()
              .bg(
                tab.active
                  ? cx.theme().colors.background
                  : cx.theme().colors.secondary,
              )
              .select_tab(group, tab.index)
              .drag_tab(group, tab.index)
              .child(tab.name)
              .child(
                div()
                  .id("x-" + tab.id)
                  .close_panel(group, tab.id)
                  .child("×"),
              ),
          ),
      ),
  );
```

### Commands, not callbacks

Look at that tab again: it carries `select_tab` and `drag_tab`, not `on_click`. That is the one rule of this API worth understanding rather than memorising.

A chrome description is cached and can outlive the handler call that produced it. A script callback registered inside one would therefore have no sound event lifetime, and every changed native state could create another. Registering one is refused where it is written; chrome uses native commands instead.

A **command** carries no script value at all. It names a container in the area and what to ask it, and base does the work:

| Command                        | On    | Does                                           |
| ------------------------------ | ----- | ---------------------------------------------- |
| `select_tab(group, index)`     | click | Displays that tab                              |
| `close_panel(group, panel_id)` | click | Closes the panel, if its group allows it       |
| `toggle_zoom(group)`           | click | Zooms the group in, or back out                |
| `drag_tab(group, index)`       | drag  | Makes the element the drag source for that tab |
| `drop_tab(group, index?)`      | drop  | Accepts a dragged panel here; no index appends |
| `toggle_dock(dock)`            | click | Opens or closes the dock                       |
| `resize_dock(dock)`            | drag  | Drags the dock's edge                          |
| `move_tile(tile)`              | drag  | Moves the tile around its canvas               |
| `resize_tile(tile, side)`      | drag  | Drags one edge or corner                       |
| `raise_tile(tile)`             | press | Brings the tile above the others               |
| `toggle_tile_zoom(tile)`       | click | Zooms the tile to fill its dock                |
| `close_tile(tile)`             | click | Closes the tile                                |

Every one takes the object its handler was given as its first argument. They belong on a `div`, an `h_flex` or a `v_flex`: a `Button` builds its own interior and has nowhere to put one.

Base clamps, snaps and rounds everything a drag produces before the next frame sees it, so a resize handle is a hit area and a colour and nothing else.

### The dock handler places its own content

`dock` is the only handler handed an element as well as state, and whatever it returns _replaces_ the dock's content. Put `dock_content()` where the panels belong:

```js
.dock((dock, cx) =>
  v_flex()
    .size_full()
    .relative()
    .child(
      h_flex()
        .h(30)
        .justify_between()
        .child(dock.placement.toUpperCase())
        .child(div().id("collapse").toggle_dock(dock).child(dock.open ? "–" : "+")),
    )
    .child(dock_content().flex_1())
    .child(div().absolute().right(0).w(4).h_full().cursor_col_resize().resize_dock(dock)),
)
```

A handler that forgets `dock_content()` still shows its panels — they are drawn after what it returned, with a warning — rather than silently losing them.

## Tiles

A region can be a free-floating canvas instead of a tab group. Pass `bounds` and the panel becomes a tile:

```js
this.dock.add_panel(cx.new(Chart), {
  name: "chart",
  placement: "center",
  bounds: { x: 40, y: 40, width: 320, height: 240 },
});
```

Tiles need their own two handlers, because base draws nothing there either: `tile_drag_bar` (whose height is fixed at base's drag-bar height, which the snapping arithmetic assumes) and `tile_resize_handles`. Both get a `tile` with **already-resolved** bounds.

## The whole surface

```js
area.add_panel(view, options);
area.remove_panel(id);
area.panels();
area.dump();
area.load(state);
area.has_dock(placement);
area.is_dock_open(placement);
area.toggle_dock(placement);
area.remove_dock(placement);
area.dock_size(placement);
area.set_dock_size(placement, size);
area.set_dock_collapsible(placement, collapsible);
area.is_locked();
area.set_locked(locked);
area.is_zoomed();
area.zoom_out();
area.on("layout_changed", handler);
area.release();
```

A locked area cannot be rearranged or dropped into. Dock and tile resizing stays available, so “lock layout” freezes where panels live without freezing their usable size.

## A complete example

```bash
cargo run -p gpui-shell -- examples/js_dock
```

`examples/js_dock/` is a workspace: a file list in the left dock, documents in the center, a tab bar and dock frame drawn in `ui.js`, and a layout written to `localStorage` on a timer. It is the shortest complete thing that uses every part of this page.

## From Rust

`gpui_kit::shell::dock` is public, so a host can reach the same seam without a script. `ScriptPanel` wraps a `ScriptView` as a `gpui_kit::base::dock::Panel`; `register_panel(application, panel, script, cx)` teaches the registry to rebuild it from a `PanelScript`; `ScriptDockSkin` forwards all three of base's renderer traits to one `DockChrome`. `tab_group_data`, `dock_data`, `tile_data` and `drop_indicator_data` are the JSON conversions the engine hands to script code, and are useful to a host writing its own binding.
