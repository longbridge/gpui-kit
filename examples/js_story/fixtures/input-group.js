import { View, div } from "gpui-kit";
import { demoValue, initializeRegisteredExamples, registeredExamples } from "../stories/registered.js";

export default class InputGroupInteraction extends View {
  init() {
    initializeRegisteredExamples();
  }

  render(cx) {
    const comment = registeredExamples("InputGroup", cx).find(example => example.label === "Comment composer");
    return div().p(16).w(460).child(comment.element)
      .child(div().child(`Draft: ${demoValue("input-group-extra:comment:value", "") || "—"}`));
  }
}
