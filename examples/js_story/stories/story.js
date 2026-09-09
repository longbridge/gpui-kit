// The gallery imports only public script modules. Registered surfaces render
// through gpui-component; infrastructure entries remain honest status panels.
import { div } from "gpui-kit";
import { v_flex } from "gpui-base";
import { coveredSurfaces } from "./coverage.js";
import { registeredExamples } from "./registered.js";
import { surfaceStatus } from "./status.js";

/**
 * @param {StoryDefinition} story
 * @returns {StoryRoute}
 */
export function pendingStory(story) {
  const surfaces = coveredSurfaces(story.id);
  const states = surfaces.map(surfaceStatus);
  if (
    states.length > 0 &&
    states.every((state) => state?.status === "registered")
  ) {
    return {
      ...story,
      availability: "registered",
      render: (cx) => registeredPanel(story, surfaces, cx),
    };
  }
  return {
    ...story,
    availability: "infrastructure",
    render: (cx) => availabilityPanel(story, cx),
  };
}

/** @param {StoryDefinition} story @param {string[]} surfaces @param {import("gpui-kit").Context} cx */
function registeredPanel(story, surfaces, cx) {
  return v_flex()
    .id(`story-${story.id}`)
    .w_full()
    .max_w(880)
    .gap(24)
    .children(
      surfaces.flatMap((surface) =>
        registeredExamples(surface, cx).map((example) => storySection(example, cx)),
      ),
    );
}

/**
 * A JavaScript counterpart to the Rust Story `section(...)` helper: one clear
 * boundary, a title/description header, and a presentation area that lets the
 * component remain the focal point.
 * @param {{ label: string, description?: string, element: unknown }} example
 * @param {import("gpui-kit").Context} cx
 */
function storySection(example, cx) {
  const colors = cx.theme().colors;
  return v_flex()
    .w_full()
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
        .child(
          div()
            .text_size(13)
            .font_semibold()
            .text_color(colors.foreground)
            .child(example.label),
        )
        .when(Boolean(example.description), (header) =>
          header.child(
            div()
              .text_size(11)
              .text_color(colors.muted_foreground)
              .child(example.description ?? ""),
          ),
        ),
    )
    .child(
      div()
        .w_full()
        .min_h(96)
        .p(24)
        .flex()
        .items_center()
        .justify_center()
        .bg(colors.background)
        .child(
          /** @type {import("gpui-kit").Element} */ (
            /** @type {unknown} */ (example.element)
          ),
        ),
    );
}

/** @param {StoryDefinition} story @param {import("gpui-kit").Context} cx */
function availabilityPanel(story, cx) {
  const colors = cx.theme().colors;

  return v_flex()
    .id(`story-${story.id}`)
    .w_full()
    .max_w(760)
    .gap(16)
    .p(24)
    .bg(colors.surface)
    .border(1)
    .border_color(colors.border)
    .rounded(8)
    .child(
      div()
        .text_size(18)
        .font_semibold()
        .text_color(colors.foreground)
        .child("Infrastructure coverage"),
    )
    .child(
      div()
        .text_size(13)
        .text_color(colors.muted_foreground)
        .child(
          "This Story route documents a non-renderable inventory entry. It is exercised through the controls that consume it, not through a fabricated constructor.",
        ),
    )
    .child(
      div()
        .px(12)
        .py(8)
        .bg(colors.muted)
        .rounded(6)
        .text_size(12)
        .text_color(colors.foreground)
        .child(`Inventory scope: ${story.api}`),
    )
    .child(
      v_flex()
        .gap(6)
        .child(
          div()
            .text_size(12)
            .font_semibold()
            .text_color(colors.foreground)
            .child("Planned examples"),
        )
        .children(
          story.states.map((state) =>
            div()
              .text_size(12)
              .text_color(colors.muted_foreground)
              .child(`• ${state}`),
          ),
        ),
    );
}

/** @typedef {import("../catalog.js").StoryRoute} StoryRoute */
/** @typedef {Omit<StoryRoute, "availability" | "render"> & { availability?: string }} StoryDefinition */
