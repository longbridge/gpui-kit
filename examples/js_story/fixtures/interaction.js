import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";
import {
  demoValue,
  initializeRegisteredExamples,
  registeredExamples,
} from "../stories/registered.js";

export default class StoryInteractionFixture extends View {
  init() {
    initializeRegisteredExamples();
  }

  render(cx) {
    const compactSwitch = registeredExamples("Switch", cx).find(
      (example) => example.label === "Compact",
    );
    const defaultToggle = registeredExamples("Toggle", cx).find(
      (example) => example.label === "Default",
    );
    return v_flex()
      .size_full()
      .gap(24)
      .p(20)
      .child(
        div()
          .id("switch-fixture")
          .h(56)
          .flex()
          .items_center()
          .child(compactSwitch?.element ?? div().child("missing compact Switch")),
      )
      .child(div().child(`compact:${String(demoValue("sw-compact", false))}`))
      .child(
        div()
          .id("toggle-fixture")
          .h(56)
          .flex()
          .items_center()
          .child(defaultToggle?.element ?? div().child("missing default Toggle")),
      )
      .child(div().child(`preview:${String(demoValue("toggle-preview", false))}`));
  }
}
