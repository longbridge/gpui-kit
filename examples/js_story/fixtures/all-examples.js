import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";
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
  }

  render(cx) {
    const surfaces = [...new Set(coveredBy.flatMap((entry) => entry.registrations))];
    return v_flex()
      .w(900)
      .gap(16)
      .children(
        surfaces.flatMap((surface) =>
          registeredExamples(surface, cx).map((example) =>
            div()
              .id(`fixture-${surface}-${example.label}`)
              .w_full()
              .child(example.element),
          ),
        ),
      )
      .child(renderVirtualListStory(this.virtualList, cx));
  }
}
