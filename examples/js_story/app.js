import { View, div } from "gpui-kit";
import { Input, InputState, h_flex, v_flex } from "gpui-base";
import { catalog, filterCatalog, route } from "./catalog.js";
import { createDockStory, renderDockStory } from "./stories/dock.js";
import { initializeRegisteredExamples } from "./stories/registered.js";
import {
  createVirtualListStory,
  renderVirtualListStory,
} from "./stories/virtual_list.js";

/** A sidebar gallery that stays entirely on the documented script surface. */
export default class StoryGallery extends View {
  /** @param {unknown} _props @param {import("gpui-kit").AsyncContext} _cx */
  init(_props, cx) {
    initializeRegisteredExamples();
    this.search = InputState.new({ placeholder: "Search components…" });
    this.activeId = catalog[0].id;
    this.highlightedId = this.activeId;
    this.routeFocus = new Map(
      catalog.map((story) => [story.id, cx.focus_handle()]),
    );
    this.dockStory = createDockStory(cx);
    this.virtualListStory = createVirtualListStory();
    this.search.on("change", (_event, cx) => {
      this.ensureHighlighted(this.visibleRoutes());
      cx.notify();
    });
    this.search.on("submit", (_event, cx) => {
      const visible = this.visibleRoutes();
      const story =
        visible.find((candidate) => candidate.id === this.highlightedId) ??
        visible[0];
      if (!story) return;
      this.select(story.id, cx);
      this.focusRoute(story.id);
    });
  }

  /** @param {string} id @param {import("gpui-kit").Context} cx */
  select(id, cx) {
    this.activeId = id;
    this.highlightedId = id;
    cx.notify();
  }

  /** @returns {import("./catalog.js").StoryRoute[]} */
  visibleRoutes() {
    return filterCatalog(this.search.value());
  }

  /** @param {import("./catalog.js").StoryRoute[]} visible */
  ensureHighlighted(visible) {
    if (visible.some((story) => story.id === this.highlightedId)) return;
    this.highlightedId = visible[0]?.id ?? null;
  }

  /** @param {string} id */
  focusRoute(id) {
    this.routeFocus.get(id)?.focus();
  }

  /** @param {number} direction @param {import("gpui-kit").Context} cx */
  moveHighlight(direction, cx) {
    const visible = this.visibleRoutes();
    if (visible.length === 0) return;
    const index = visible.findIndex((story) => story.id === this.highlightedId);
    const nextIndex =
      index === -1
        ? direction > 0
          ? 0
          : visible.length - 1
        : (index + direction + visible.length) % visible.length;
    const next = visible[nextIndex];
    this.highlightedId = next.id;
    this.focusRoute(next.id);
    cx.notify();
  }

  /** @param {import("gpui-kit").KeyEvent} event @param {import("gpui-kit").Context} cx */
  handleNavigationKey(event, cx) {
    if (event.key === "down") this.moveHighlight(1, cx);
    if (event.key === "up") this.moveHighlight(-1, cx);
    if (event.key === "enter" && this.highlightedId) {
      this.select(this.highlightedId, cx);
    }
  }

  /** @param {import("gpui-kit").Context} cx */
  render(cx) {
    const active = route(this.activeId);
    const colors = cx.theme().colors;

    return h_flex()
      .size_full()
      .bg(colors.background)
      .child(this.sidebar(cx))
      .child(
        v_flex()
          .flex_1()
          // A wide child — a table, a chart — would otherwise refuse to shrink
          // and push the sidebar off screen. The vertical axis needs no such
          // line: the scroll area clips the axis it scrolls, so it may shrink.
          .min_w_0()
          .overflow_y_scrollbar()
          .p(28)
          .gap(20)
          .child(
            v_flex()
              .gap(6)
              .child(
                div()
                  .text_size(24)
                  .font_semibold()
                  .text_color(colors.foreground)
                  .child(active.title),
              )
              .child(
                div()
                  .text_size(13)
                  .text_color(colors.muted_foreground)
                  .child(active.description),
              ),
          )
          .child(
            active.id === "dock"
              ? renderDockStory(this.dockStory, cx)
              : active.id === "virtual-list"
                ? renderVirtualListStory(this.virtualListStory, cx)
                : active.render(cx),
          ),
      );
  }

  /** @param {import("gpui-kit").Context} cx */
  sidebar(cx) {
    const colors = cx.theme().colors;
    const visible = this.visibleRoutes();
    /** @type {Map<string, import("./catalog.js").StoryRoute[]>} */
    const groups = new Map();
    for (const story of visible) {
      const entries = groups.get(story.group) ?? [];
      entries.push(story);
      groups.set(story.group, entries);
    }

    return v_flex()
      .w(270)
      .flex_none()
      // `h_flex` centres its children on the cross axis, so a column that
      // should fill the window has to say so; without this the sidebar is
      // sized by its content and centred, which pushes its header off the top.
      .h_full()
      .border_r(1)
      .border_color(colors.border)
      .bg(colors.surface)
      .on_key_down((event, context) => this.handleNavigationKey(event, context))
      .child(
        v_flex()
          .gap(12)
          .p(16)
          .border_b(1)
          .border_color(colors.border)
          .child(
            div()
              .text_size(16)
              .font_semibold()
              .text_color(colors.foreground)
              .child("GPUI Component Story"),
          )
          .child(
            Input.new(this.search)
              .h(30)
              .px(10)
              .border(1)
              .border_color(colors.input)
              .bg(colors.background)
              .text_size(12),
          )
          .child(
            div()
              .text_size(11)
              .text_color(colors.muted_foreground)
              .child(`${visible.length} of ${catalog.length} routes`),
          ),
      )
      .child(
        v_flex()
          .flex_1()
          .overflow_y_scrollbar()
          .p(8)
          .gap(14)
          .children(
            [...groups.entries()].map(([group, stories]) =>
              v_flex()
                .gap(2)
                .child(
                  div()
                    .px(8)
                    .py(4)
                    .text_size(11)
                    .font_semibold()
                    .text_color(colors.muted_foreground)
                    .child(group),
                )
                .children(
                  stories.map((story) => this.navigationItem(story, cx)),
                ),
            ),
          )
          .when(visible.length === 0, (element) =>
            element.child(
              div()
                .p(12)
                .text_size(12)
                .text_color(colors.muted_foreground)
                .child("No matching component routes."),
            ),
          ),
      );
  }

  /** @param {import("./catalog.js").StoryRoute} story @param {import("gpui-kit").Context} cx */
  navigationItem(story, cx) {
    const colors = cx.theme().colors;
    const selected = story.id === this.activeId;
    const highlighted = story.id === this.highlightedId;
    return (
      div()
        .id(`route-${story.id}`)
        .track_focus(this.routeFocus.get(story.id))
        .accessibility_label(`Open ${story.title} story`)
        .w_full()
        .h(28)
        .px(8)
        .flex()
        .items_center()
        .rounded(5)
        .bg(selected || highlighted ? colors.muted : colors.surface)
        .text_size(12)
        .text_color(selected ? colors.foreground : colors.muted_foreground)
        .hover((element) =>
          element.bg(colors.muted).text_color(colors.foreground),
        )
        // The parent list owns Up/Down/Enter. Each item owns a retained focus
        // target, so arrow navigation moves the native focus rather than only
        // repainting a highlight. Submitting a search picks its first result
        // and transfers focus here.
        .on_click((_event, context) => this.select(story.id, context))
        .child(story.title)
    );
  }
}
