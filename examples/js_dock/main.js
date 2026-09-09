// A workspace: a dockable layout drawn entirely by script.
//
// `gpui-base` supplies the behaviour — splits, tab groups, docks, drag and
// drop, zoom, and a layout that is pure data — and draws no chrome at all. An
// area with no chrome still docks, drags, resizes and persists; it simply
// paints nothing but the panels. Everything you can see here is in `ui.js`.
//
//   cargo run -p gpui-shell -- examples/js_dock

import { View, div } from "gpui-kit";
import { DockArea, dock_area, dock_content, v_flex } from "gpui-base";
/** @import { AsyncContext, Context } from "gpui-kit" */
import {
  BAR,
  dockBar,
  dockHandle,
  dockTab,
  dropHint,
  emptyGroup,
  label,
  muted,
  panelBorder,
} from "./ui.js";

/** Where the layout is kept between runs. */
const LAYOUT = "workspace.layout";

/**
 * One panel's body.
 *
 * It is an ordinary view, and that is the point: a panel is a view that a dock
 * happens to be holding. The two extra methods are what carries its state
 * across a restart — `serialize()` is read when the layout is saved,
 * `deserialize(data)` is handed back what it wrote.
 */
class Document extends View {
  /** @param {{ caption?: string }} props */
  init(props) {
    this.caption = props?.caption ?? "Untitled";
    this.edits = 0;
  }

  // Runs without a host call: return a value and touch nothing else.
  serialize() {
    return { caption: this.caption, edits: this.edits };
  }

  /** @param {{ caption: string, edits: number }} data */
  deserialize(data) {
    this.caption = data.caption;
    this.edits = data.edits;
  }

  /** @param {Context} cx */
  render(cx) {
    return panelBorder(v_flex(), cx)
      .size_full()
      .p(16)
      .gap(8)
      .bg(cx.theme().colors.background)
      .child(label(this.caption, cx))
      .child(muted(this.edits + " edits", cx))
      .child(
        div()
          .id("edit-" + this.caption)
          .px(10)
          .py(6)
          .rounded(6)
          .text_size(12)
          .bg(cx.theme().colors.primary)
          .text_color(cx.theme().colors.primary_foreground)
          .on_click((_event, cx) => {
            this.edits += 1;
            cx.notify();
          })
          .child("Edit"),
      );
  }
}

/** A panel with no state of its own, to show one that needs no hooks. */
class Files extends View {
  /** @param {Context} cx */
  render(cx) {
    return panelBorder(v_flex(), cx)
      .size_full()
      .p(12)
      .gap(6)
      .bg(cx.theme().colors.background)
      .children(["main.js", "ui.js", "README.md"].map((name) => muted(name, cx)));
  }
}

/**
 * The right dock's occupant.
 *
 * A second *kind* of panel rather than a third document, because a side dock is
 * only interesting when what it holds is not what the centre holds -- an
 * inspector beside the thing it inspects.
 */
class Outline extends View {
  /** @param {Context} cx */
  render(cx) {
    return panelBorder(v_flex(), cx)
      .size_full()
      .p(16)
      .gap(6)
      .bg(cx.theme().colors.background)
      .child(label("Outline", cx))
      .children(["imports", "Document", "Files", "Workspace"].map((each) => muted(each, cx)));
  }
}

export default class Workspace extends View {
  /** @param {unknown} _props @param {AsyncContext} cx */
  init(_props, cx) {
    // Registered before anything is loaded: this is what lets a saved layout
    // find the class again. Both panels are registered, including the one with
    // no serialize() — a panel with no payload still needs a way back.
    DockArea.register_panel("document", Document);
    DockArea.register_panel("files", Files);
    DockArea.register_panel("outline", Outline);

    this.dock = DockArea.new("workspace", { version: 1 });
    this.saving = null;

    const saved = localStorage.getItem(LAYOUT);
    if (saved) {
      // Restores the tree, the dock sizes and every panel's own payload.
      this.dock.load(JSON.parse(saved));
    } else {
      // One of each: a left dock, a centre holding two tabs, and a right dock.
      // Both side docks are here on purpose -- they are laid out on opposite
      // sides of the same row, and a left-only example cannot tell a dock that
      // is placed correctly from one that merely happens to be first.
      this.dock.add_panel(cx.new(Files), { name: "files", placement: "left", size: 200 });
      this.dock.add_panel(cx.new(Document, { caption: "main.js" }), { name: "document" });
      this.dock.add_panel(cx.new(Document, { caption: "ui.js" }), { name: "document" });
      this.dock.add_panel(cx.new(Outline), { name: "outline", placement: "right", size: 220 });
    }

    // Fires on every edit, including each step of a drag — so the write is on a
    // timer rather than on the event.
    this.dock.on("layout_changed", (cx) => {
      cx.notify();
      if (this.saving) return;
      this.saving = cx.timer.after(500, () => {
        this.saving = null;
        localStorage.setItem(LAYOUT, JSON.stringify(this.dock.dump()));
      });
    });
  }

  /** @param {Context} cx */
  render(cx) {
    return v_flex()
      .size_full()
      .bg(cx.theme().colors.background)
      .child(this.toolbar(cx))
      .child(
        dock_area(this.dock)
          .flex_1()
          .tab_bar((group, cx) =>
            div()
              .id("tab-bar-" + group.node)
              .flex()
              .h(BAR)
              .w_full()
              .bg(cx.theme().colors.secondary)
              .border_b(1)
              .border_color(cx.theme().colors.border)
              // The bar itself accepts a drop, so a tab dragged onto it joins
              // this group at the end rather than splitting beside it.
              .drop_tab(group)
              .children(group.tabs.filter((each) => each.visible).map((each) => dockTab(group, each, cx))),
          )
          .empty_group((_group, cx) => emptyGroup(cx))
          .drop_indicator((drop, cx) => dropHint(drop, cx))
          // Whatever this returns replaces the dock's content, so the panels go
          // where `dock_content()` is.
          // Chrome only: base sizes the dock itself, so this decorates the
          // box rather than declaring it. The border is on the side the dock
          // faces the centre from, so the seam between them is one line and not
          // two, and so it is obvious at a glance which column is which.
          .dock((dock, cx) =>
            v_flex()
              .size_full()
              .relative()
              .bg(cx.theme().colors.background)
              .when(dock.placement === "left", (it) =>
                it.border_r(1).border_color(cx.theme().colors.border),
              )
              .when(dock.placement === "right", (it) =>
                it.border_l(1).border_color(cx.theme().colors.border),
              )
              .when(dock.placement === "bottom", (it) =>
                it.border_t(1).border_color(cx.theme().colors.border),
              )
              .child(dockBar(dock, cx))
              .child(dock_content().flex_1().overflow_hidden())
              .child(dockHandle(dock, cx)),
          ),
      );
  }

  /**
   * One side dock's collapse control, drawn in the toolbar rather than in the
   * dock.
   *
   * This is the whole reason it is here. A collapsed side dock has no width and
   * base draws nothing for it -- correctly: a strip of empty column beside the
   * centre would be worse than none. But a control that lives in a dock goes
   * with it, so a dock collapsed from its own chrome is a dock with no way back.
   * A bottom dock keeps a strip for exactly this reason; a side dock cannot, so
   * whoever collapses it owns the way to reopen it.
   *
   * The state is read from the dock every time rather than mirrored here: the
   * edge can also be dragged shut, and a copy of the flag would start lying the
   * first time that happened.
   *
   * @param {import("gpui-base").DockPlacement} placement
   * @param {string} caption
   * @param {Context} cx
   */
  dockToggle(placement, caption, cx) {
    if (!this.dock.has_dock(placement)) return div();
    const open = this.dock.is_dock_open(placement);
    return div()
      .id("toggle-" + placement)
      .px(8)
      .py(3)
      .rounded(4)
      .text_size(11)
      .bg(open ? cx.theme().colors.accent : cx.theme().colors.secondary)
      .text_color(open ? cx.theme().colors.foreground : cx.theme().colors.muted_foreground)
      .hover((it) => it.bg(cx.theme().colors.accent))
      .on_click((_event, cx) => {
        this.dock.toggle_dock(placement);
        cx.notify();
      })
      .child(caption);
  }

  /** @param {Context} cx */
  toolbar(cx) {
    const open = this.dock.panels().filter((panel) => panel.placement === "center").length;
    return div()
      .flex()
      .h(BAR)
      .px(10)
      .gap(10)
      .items_center()
      // The window's own surface, not the tab bars': this row is the
      // application's chrome and the row below it is the dock's, and a shared
      // fill made the two read as one strip.
      .bg(cx.theme().colors.background)
      .border_b(1)
      .border_color(cx.theme().colors.border)
      .child(label("Workspace", cx))
      .child(muted(open + " open", cx))
      .child(this.dockToggle("left", "Files", cx))
      .child(this.dockToggle("right", "Outline", cx))
      .child(
        div()
          .id("new-document")
          .px(8)
          .py(3)
          .rounded(4)
          .text_size(11)
          .text_color(cx.theme().colors.muted_foreground)
          .hover((it) => it.bg(cx.theme().colors.accent))
          .on_click((_event, cx) => {
            this.dock.add_panel(cx.new(Document, { caption: "note " + (open + 1) }), {
              name: "document",
            });
            cx.notify();
          })
          .child("New"),
      )
      .child(
        div()
          .id("reset-layout")
          .px(8)
          .py(3)
          .rounded(4)
          .text_size(11)
          .text_color(cx.theme().colors.muted_foreground)
          .hover((it) => it.bg(cx.theme().colors.accent))
          .on_click((_event, cx) => {
            localStorage.removeItem(LAYOUT);
            cx.notify();
          })
          .child("Forget layout"),
      );
  }
}
