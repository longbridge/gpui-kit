import { View } from "gpui-kit";
import { createDockStory, renderDockStory } from "../stories/dock.js";

export default class DockStoryFixture extends View {
  init(_props, cx) {
    this.dock = createDockStory(cx);
  }

  render(cx) {
    return renderDockStory(this.dock, cx);
  }
}
