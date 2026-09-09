// A confirmation dialog, opened by the list when an action destroys work.
//
// A function returning an element, not a view class. It runs when the dialog
// draws — an element belongs to the render pass that built it, and a dialog
// outlives the call that opened it — so everything it shows comes from what the
// caller closed over rather than from a `props` object handed across.

import { v_flex, h_flex } from "gpui-base";
import { SPACE, button, label, muted } from "./ui.js";

/**
 * @param {number} count
 * @param {import("gpui-kit").Context} cx
 * @param {(cx: import("gpui-kit").Context) => void} onConfirm
 */
export default (count, cx, onConfirm) => () =>
  v_flex()
    .w(360)
    .gap(SPACE.md)
    .child(label(`Delete ${count} completed ${count === 1 ? "item" : "items"}?`, cx))
    .child(muted("This cannot be undone.", cx))
    .child(
      h_flex()
        .justify_end()
        .gap(SPACE.sm)
        .pt(SPACE.sm)
        .child(button("cancel", "Cancel", () => window.close_dialog(), cx, { variant: "ghost" }))
        .child(
          button(
            "confirm",
            "Delete",
            (_event, cx) => {
              onConfirm(cx);
              window.close_dialog();
            },
            cx,
            { variant: "primary" },
          ),
        ),
    );
