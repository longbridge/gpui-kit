import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";
import {
  initializeRegisteredExamples,
  registeredExamples,
} from "../stories/registered.js";

export default class InputStoryFixture extends View {
  init() {
    initializeRegisteredExamples();
  }

  render(cx) {
    const inputExample = registeredExamples("Input", cx)[0];
    return v_flex()
      .size_full()
      .gap(16)
      .p(20)
      .child(
        div()
          .id("input-target")
          .w(420)
          .child(inputExample.element),
      );
  }
}
