// The presentation layer, as functions.
//
// The base layer ships no styled widgets, so a dock's chrome is written once,
// here, and `main.js` reads like it is using a component library. Every helper
// takes the current `cx` and reads tokens through `cx.theme()`, which costs
// nothing: a fresh description is exactly what a function call produces.

import { div } from "gpui-kit";
import { h_flex, v_flex } from "gpui-base";
/** @import { Context, Element } from "gpui-kit" */

/** The tab bar's height, and the height a dock's title strip matches. */
export const BAR = 30;

/**
 * The outline every panel body draws around itself.
 *
 * A dock area is several nested containers that all paint the same background,
 * so without an edge a panel, its group and its dock are one flat field and a
 * layout fault reads as "something looks off" rather than as "that pane is the
 * wrong size". One line per panel is enough to tell them apart, and it is the
 * example's own chrome rather than base's: a real application would rather draw
 * its own.
 *
 * @param {Element} element @param {Context} cx
 */
export const panelBorder = (element, cx) =>
  element.border(1).border_color(cx.theme().colors.border);

/** @param {string} value @param {Context} cx */
export const label = (value, cx) =>
  div().text_size(12).line_height(1).text_color(cx.theme().colors.foreground).child(value);

/** @param {string} value @param {Context} cx */
export const muted = (value, cx) =>
  div().text_size(11).line_height(1).text_color(cx.theme().colors.muted_foreground).child(value);

/**
 * One tab.
 *
 * The two dock commands on it are the whole of its behaviour: `select_tab`
 * displays it, `drag_tab` makes it the drag source carrying base's own panel
 * payload — so dropping it on another group moves the panel there.
 *
 * @param {import("gpui-base").DockGroup} group
 * @param {import("gpui-base").DockTab} tab
 * @param {Context} cx
 */
export const dockTab = (group, tab, cx) =>
  h_flex()
    .id("tab-" + tab.id)
    .h(BAR)
    .px(10)
    .gap(6)
    .items_center()
    .border_r(1)
    .border_color(cx.theme().colors.border)
    .bg(tab.active ? cx.theme().colors.background : cx.theme().colors.secondary)
    .text_color(tab.active ? cx.theme().colors.foreground : cx.theme().colors.muted_foreground)
    .text_size(12)
    .select_tab(group, tab.index)
    .drag_tab(group, tab.index)
    .child(title(tab.name))
    .when(tab.closable, (el) =>
      el.child(
        div()
          .id("close-" + tab.id)
          .px(4)
          .rounded(3)
          .text_color(cx.theme().colors.muted_foreground)
          .hover((it) => it.bg(cx.theme().colors.accent))
          .close_panel(group, tab.id)
          .child("×"),
      ),
    );

/** The readable half of `shell:<application>/<panel>`. */
export const title = (name) => {
  const slash = name.lastIndexOf("/");
  return slash === -1 ? name : name.slice(slash + 1);
};

/**
 * A dock's title strip: what it is called, and the control that collapses it.
 *
 * @param {import("gpui-base").DockRegion} dock
 * @param {Context} cx
 */
export const dockBar = (dock, cx) =>
  h_flex()
    .id("dock-bar-" + dock.placement)
    .h(BAR)
    .px(10)
    .items_center()
    .justify_between()
    .bg(cx.theme().colors.secondary)
    .border_b(1)
    .border_color(cx.theme().colors.border)
    .child(muted(dock.placement.toUpperCase(), cx))
    .child(
      div()
        .id("collapse-" + dock.placement)
        .px(6)
        .rounded(3)
        .text_size(11)
        .text_color(cx.theme().colors.muted_foreground)
        .hover((it) => it.bg(cx.theme().colors.accent))
        .toggle_dock(dock)
        .child(dock.open ? "–" : "+"),
    );

/**
 * The strip along a dock's inner edge that resizes it.
 *
 * Base clamps every position it is given against the area and the opposite
 * dock, so this is only a hit area and a colour.
 *
 * @param {import("gpui-base").DockRegion} dock
 * @param {Context} cx
 */
export const dockHandle = (dock, cx) =>
  div()
    .id("resize-" + dock.placement)
    .absolute()
    .map((el) => (dock.placement === "bottom" ? el.top(0).left(0).w_full().h(4) : el.top(0).h_full().w(4)))
    .map((el) => (dock.placement === "left" ? el.right(0) : dock.placement === "right" ? el.left(0) : el))
    .map((el) => (dock.placement === "bottom" ? el.cursor_row_resize() : el.cursor_col_resize()))
    .hover((it) => it.bg(cx.theme().colors.primary))
    .resize_dock(dock);

/** What a group with nothing to show says. @param {Context} cx */
export const emptyGroup = (cx) =>
  v_flex()
    .size_full()
    .items_center()
    .justify_center()
    .bg(cx.theme().colors.background)
    .child(muted("Drop a panel here", cx));

/**
 * The hint showing where a dragged panel would land.
 *
 * The bounds are already resolved — base snaps and clamps before a skin sees
 * them — so this only paints.
 *
 * @param {import("gpui-base").DockDrop} drop
 * @param {Context} cx
 */
export const dropHint = (drop, cx) =>
  div()
    .absolute()
    .left(drop.to.x)
    .top(drop.to.y)
    .w(drop.to.width)
    .h(drop.to.height)
    .bg(cx.theme().colors.primary)
    .opacity(0.15)
    .border(1)
    .border_color(cx.theme().colors.primary);
