import { View, div } from "gpui-kit";
import { DockArea, dock_area, dock_content, h_flex, v_flex } from "gpui-base";

const TAB_HEIGHT = 32;

const dockTab = (group, panel, cx) =>
  h_flex()
    .id(`story-dock-tab-${panel.id}`)
    .h(TAB_HEIGHT)
    .px(10)
    .items_center()
    .border_r(1)
    .border_color(cx.theme().colors.border)
    .bg(panel.active ? cx.theme().colors.background : cx.theme().colors.secondary)
    .text_color(
      panel.active ? cx.theme().colors.foreground : cx.theme().colors.muted_foreground,
    )
    .text_size(12)
    .select_tab(group, panel.index)
    .drag_tab(group, panel.index)
    .child(panel.name.slice(panel.name.lastIndexOf("/") + 1));

class StoryDockPanel extends View {
  /** @param {{ title?: string, detail?: string }} props */
  init(props) {
    this.title = props?.title ?? "Untitled";
    this.detail = props?.detail ?? "";
  }

  /** @param {import("gpui-kit").Context} cx */
  render(cx) {
    const colors = cx.theme().colors;
    return v_flex()
      .size_full()
      .gap(8)
      .p(16)
      .bg(colors.background)
      .child(div().text_size(14).font_semibold().child(this.title))
      .child(div().text_size(12).text_color(colors.muted_foreground).child(this.detail));
  }
}

/** @param {import("gpui-kit").AsyncContext} cx */
export function createDockStory(cx) {
  DockArea.register_panel("story-files", StoryDockPanel);
  DockArea.register_panel("main.js", StoryDockPanel);
  DockArea.register_panel("app.js", StoryDockPanel);
  const dock = DockArea.new("story-gallery-dock", { version: 1 });
  dock.add_panel(
    cx.new(StoryDockPanel, { title: "Files", detail: "main.js · app.js · stories/" }),
    { name: "story-files", placement: "left", size: 180 },
  );
  dock.add_panel(
    cx.new(StoryDockPanel, {
      title: "main.js",
      detail: "The active editor panel in the center tab group.",
    }),
    { name: "main.js", placement: "center" },
  );
  dock.add_panel(
    cx.new(StoryDockPanel, {
      title: "app.js",
      detail: "A second panel demonstrates native tabs.",
    }),
    { name: "app.js", placement: "center" },
  );
  dock.on("layout_changed", (context) => context.notify());
  return dock;
}

/** @param {ReturnType<typeof createDockStory>} dock @param {import("gpui-kit").Context} cx */
export function renderDockStory(dock, cx) {
  const colors = cx.theme().colors;
  return v_flex()
    .w_full()
    .max_w(880)
    .border(1)
    .border_color(colors.border)
    .rounded(8)
    .overflow_hidden()
    .child(
      v_flex()
        .gap(4)
        .px(16)
        .py(12)
        .border_b(1)
        .border_color(colors.border)
        .child(div().text_size(13).font_semibold().child("Panel, dock, and tabs"))
        .child(
          div()
            .text_size(11)
            .text_color(colors.muted_foreground)
            .child("A files dock beside a center group containing two selectable panels."),
        ),
    )
    .child(
      dock_area(dock)
        .h(340)
        .w_full()
        .tab_bar((group, cx) =>
          h_flex()
            .id(`story-dock-tab-bar-${group.node}`)
            .h(TAB_HEIGHT)
            .w_full()
            .items_center()
            .bg(cx.theme().colors.secondary)
            .border_b(1)
            .border_color(cx.theme().colors.border)
            .drop_tab(group)
            .children(
              group.tabs
                .filter((panel) => panel.visible)
                .map((panel) => dockTab(group, panel, cx)),
            ),
        )
        .drop_indicator((drop, cx) =>
          div()
            .absolute()
            .left(drop.to.x)
            .top(drop.to.y)
            .w(drop.to.width)
            .h(drop.to.height)
            .bg(cx.theme().colors.primary)
            .opacity(0.16)
            .border(1)
            .border_color(cx.theme().colors.primary),
        )
        .dock((dock, cx) =>
          v_flex()
            .size_full()
            .bg(cx.theme().colors.background)
            .when(dock.placement === "left", (area) =>
              area.border_r(1).border_color(cx.theme().colors.border),
            )
            .child(
              div()
                .h(28)
                .px(10)
                .flex()
                .items_center()
                .text_size(11)
                .font_semibold()
                .text_color(cx.theme().colors.muted_foreground)
                .child(dock.placement === "left" ? "EXPLORER" : "DOCK"),
            )
            .child(dock_content().flex_1().overflow_hidden()),
        ),
    );
}
