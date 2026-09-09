// A standalone native-motion ScriptView.  Every animated length target is a
// number, therefore an absolute pixel length that materialize::animate_length
// can sample on GPUI frames.
import { View, div } from "gpui-kit";
import { Button, h_flex, v_flex } from "gpui-base";
/** @import { Context, Element } from "gpui-kit" */

// The stage is `w_full()`, so its width belongs to whatever panel holds it,
// while a motion target has to be an absolute pixel length. These are therefore
// chosen to fit the narrowest panel worth drawing this in — the card's far edge
// lands at 348 — rather than measured from a container nothing here can ask.
const REST_LEFT = 20;
const TRAVEL = 180;
const REST_WIDTH = 132;
const ACTIVE_WIDTH = 148;

const segment = (id, label, active, onClick, cx) =>
  Button.new(id)
    .selected(active)
    .h(30)
    .px(12)
    .rounded(cx.theme().radius.sm)
    .border(1)
    .border_color(active ? cx.theme().colors.border : cx.theme().colors.muted)
    .bg(active ? cx.theme().colors.background : cx.theme().colors.muted)
    .hover((style) => style.bg(cx.theme().colors.secondary))
    .focus((style) => style.border_color(cx.theme().colors.foreground))
    .on_click(onClick)
    .child(
      div()
        .text_size(12)
        .font_medium()
        .text_color(
          active ? cx.theme().colors.foreground : cx.theme().colors.muted_foreground,
        )
        .child(active ? `${label} ✓` : label),
    );

const action = (label, active, onClick, cx) =>
  Button.new("motion-trigger")
    .h(36)
    .px(14)
    .rounded(cx.theme().radius.md)
    .border(1)
    .border_color(cx.theme().colors.border)
    .bg(cx.theme().colors.background)
    .hover((style) => style.bg(cx.theme().colors.secondary))
    .focus((style) => style.border_color(cx.theme().colors.foreground))
    .on_click(onClick)
    .child(
      div()
        .text_size(12)
        .font_medium()
        .text_color(cx.theme().colors.foreground)
        .child(active ? "Send back" : label),
    );

export default class MotionBoard extends View {
  init() {
    this.policy = "transition";
    this.active = false;
  }

  render(cx) {
    const spring = this.policy === "spring";
    const active = this.active;

    return v_flex()
      .w_full()
      .gap(12)
      .child(
        v_flex()
          .gap(4)
          .child(
            div()
              .text_size(14)
              .font_semibold()
              .text_color(cx.theme().colors.foreground).child("Native motion"),
          )
          .child(
            div()
              .text_size(12)
              .text_color(cx.theme().colors.muted_foreground)
              .child(
                spring
                  ? "Spring samples pixel left, width, and opacity targets on native frames."
                  : "Transition samples pixel left, width, and opacity targets on native frames.",
              ),
          ),
      )
      .child(
        h_flex()
          .w_full()
          .items_center()
          .gap(12)
          .child(
            h_flex()
              .id("motion-policy-segment")
              .gap(2)
              .p(2)
              .rounded(cx.theme().radius.md)
              .border(1)
              .border_color(cx.theme().colors.border)
              .bg(cx.theme().colors.muted)
              .child(
                segment(
                  "motion-transition",
                  "Transition",
                  !spring,
                  (_, cx) => this.select("transition", cx),
                  cx,
                ),
              )
              .child(
                segment(
                  "motion-spring",
                  "Spring",
                  spring,
                  (_, cx) => this.select("spring", cx),
                  cx,
                ),
              ),
          )
          .child(
            action("Run motion", active, (_, cx) => {
              this.active = !this.active;
              cx.notify();
            }, cx),
          ),
      )
      .child(
        // The stage clips: everything inside it is positioned absolutely, and an
        // absolute child of a `relative()` box spills past the border rather
        // than being contained by it. The travel below fits a narrow panel, and
        // this keeps a narrower one honest.
        div()
          .relative()
          .w_full()
          .h(176)
          .overflow_hidden()
          .rounded(cx.theme().radius.md)
          .border(1)
          .border_color(cx.theme().colors.border)
          .bg(cx.theme().colors.muted)
          .children(this.track(cx))
          .child(this.runner(cx)),
      );
  }

  /** @param {"transition" | "spring"} policy @param {Context} cx */
  select(policy, cx) {
    this.policy = policy;
    cx.notify();
  }

  /** @param {Context} cx */
  runner(cx) {
    const active = this.active;
    return this.motion(
      v_flex()
        .id("motion-runner")
        .absolute()
        .top(48)
        // Pixels, because that is what a motion target has to be: the whole
        // point is that GPUI samples an absolute length on native frames. So
        // the travel is a number the stage can hold at any sensible panel
        // width, and `track` marks the same two stations.
        .left(active ? REST_LEFT + TRAVEL : REST_LEFT)
        .w(active ? ACTIVE_WIDTH : REST_WIDTH)
        .h(80)
        .gap(8)
        .p(12)
        .rounded(cx.theme().radius.md)
        .border(1)
        .border_color(cx.theme().colors.border)
        .bg(cx.theme().colors.background)
        .opacity(active ? 1 : 0.72)
        .child(
          h_flex()
            .w_full()
            .gap(6)
            .child(
              div()
                .text_size(12)
                .font_semibold()
                .text_color(cx.theme().colors.foreground).child("AAPL"),
            )
            .child(div().flex_1())
            .child(div().w(6).h(6).rounded(3).bg(cx.theme().colors.accent))
            .child(div().text_size(10).text_color(cx.theme().colors.accent).child("Live")),
        )
        .child(
          h_flex()
            .w_full()
            .items_center()
            .gap(8)
            .child(
              div()
                .text_size(16)
                .font_semibold()
                .text_color(cx.theme().colors.foreground).child("$228.26"),
            )
            .child(div().text_size(11).font_medium().text_color(cx.theme().colors.accent).child("+1.84%")),
        ),
    );
  }

  /** @param {Context} cx */
  track(cx) {
    const end = REST_LEFT + TRAVEL;
    return [
      div()
        .absolute()
        .top(18)
        .left(REST_LEFT)
        .text_size(9)
        .text_color(cx.theme().colors.muted_foreground).child("OPEN"),
      div()
        .absolute()
        .top(18)
        .left(end)
        .text_size(9)
        .text_color(cx.theme().colors.muted_foreground).child("LIVE TICK"),
      div()
        .absolute()
        .top(88)
        .left(REST_LEFT + 4)
        .w(TRAVEL + ACTIVE_WIDTH - 12)
        .h(1)
        .bg(cx.theme().colors.border),
      div()
        .absolute()
        .top(84)
        .left(REST_LEFT)
        .w(8)
        .h(8)
        .rounded(4)
        .bg(cx.theme().colors.accent),
      div()
        .absolute()
        .top(84)
        .left(end + ACTIVE_WIDTH - 8)
        .w(8)
        .h(8)
        .rounded(4)
        .bg(cx.theme().colors.border),
      // Left and right both, so the sentence wraps inside the stage instead of
      // running past its border on a narrow panel.
      div()
        .absolute()
        .top(144)
        .left(REST_LEFT)
        .right(REST_LEFT)
        .text_size(10)
        .text_color(cx.theme().colors.muted_foreground).child("Native frames interpolate the card; JavaScript only changes its target."),
    ];
  }

  /** @param {Element} element */
  motion(element) {
    if (this.policy === "spring") {
      return element
        .spring("left", { response: 360, damping: 0.72 })
        .spring("width", { response: 300, damping: 0.8 })
        .spring("opacity", { response: 220, damping: 1 });
    }
    return element
      .transition("left", { duration: 340, easing: "ease-in-out" })
      .transition("width", { duration: 260, easing: "ease-out" })
      .transition("opacity", { duration: 180, easing: "ease-out" });
  }
}
