import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";
import { register_surfaces, selected_surface } from "story-gallery-fixture";
import { coveredBy } from "../stories/coverage.js";
import {
  initializeRegisteredExamples,
  registeredExamples,
} from "../stories/registered.js";
import {
  createVirtualListStory,
  renderVirtualListStory,
} from "../stories/virtual_list.js";

export default class AllRegisteredExamplesFixture extends View {
  init() {
    initializeRegisteredExamples();
    this.virtualList = createVirtualListStory();
    register_surfaces([
      // Tab has no standalone examples; TabBar materializes its Tab children.
      ...[...new Set(coveredBy.flatMap((entry) => entry.registrations))].filter(
        (surface) => surface !== "Tab",
      ),
      "VirtualList",
    ]);
  }

  render(cx) {
    // The host selects one surface per render so the complete inventory does
    // not share a single frame's execution budget.
    const surface = selected_surface();
    if (surface === null) return div();
    if (surface === "VirtualList") {
      return renderVirtualListStory(this.virtualList, cx);
    }
    const examples = registeredExamples(surface, cx);
    if (examples.length === 0) throw new Error(`No examples for ${surface}`);
    return v_flex()
      .w(900)
      .gap(16)
      .children(
        examples.map((example) =>
          div()
            .id(`fixture-${surface}-${example.label}`)
            .w_full()
            .child(example.element),
        ),
      );
  }
}
