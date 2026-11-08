import { div } from "gpui-kit";
import { Scrollbar, v_flex, v_virtual_list } from "gpui-base";

export function createVirtualListStory() {
  return { selectedId: "project-42" };
}

/** @param {{ selectedId: string }} story @param {import("gpui-kit").Context} cx */
export function renderVirtualListStory(story, cx) {
  const colors = cx.theme().colors;
  const rowCount = 10000;
  return v_flex()
    .w_full()
    .max_w(880)
    .gap(12)
    .child(
      v_flex()
        .gap(4)
        .child(div().text_size(13).font_semibold().child("10,000 projects"))
        .child(
          div()
            .text_size(11)
            .text_color(colors.muted_foreground)
            .child(
              "Only visible rows are built; hover a row to see its domain ID, click it to keep that ID selected.",
            ),
        ),
    )
    .child(
      v_flex()
        .relative()
        .h(320)
        .w_full()
        .border(1)
        .border_color(colors.border)
        .rounded(8)
        .overflow_hidden()
        .child(
          v_virtual_list(
            "project-list",
            rowCount,
            32,
            (index) => `project-${index + 1}`,
            (range) =>
              Array.from({ length: range.end - range.start }, (_unused, offset) => {
                const index = range.start + offset;
                const id = `project-${index + 1}`;
                return div()
                  .id(id)
                  .group("project-row")
                  .h(32)
                  .px(12)
                  .flex()
                  .items_center()
                  .justify_between()
                  .border_b(1)
                  .border_color(colors.border)
                  .bg(story.selectedId === id ? colors.muted : colors.background)
                  .child(`Project ${index + 1}`)
                  // A style rather than a handler, so a row built during
                  // layout may carry it: the pointer crossing the list runs
                  // no script.
                  .child(
                    div()
                      .text_size(11)
                      .text_color(colors.muted_foreground)
                      .opacity(0)
                      .group_hover("project-row", (el) => el.opacity(1))
                      .child(id),
                  );
              }),
          )
            .size_full()
            .on_item_click((id, context) => {
              story.selectedId = id;
              context.notify();
            }),
        )
        .child(Scrollbar.vertical("project-list").absolute().inset_0()),
    );
}
