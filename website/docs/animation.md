---
title: Animation
description: Choose GPUI element animation, GPUI Base motion, and GPUI Component motion with correct identity, interruption, and reduced-motion behavior.
order: -3.1
---

# Animation

GPUI Kit offers three levels of motion. Choose by **what owns the changing value**, not by the shape of the effect:

| Level | Use it for | State and policy |
| --- | --- | --- |
| GPUI `Animation` and `AnimationExt` | An element entering, pulsing, or running a fixed series while mounted | GPUI retains playback under the wrapper's [`ElementId`](./element_id); the caller chooses duration, easing, and visual property. |
| [GPUI Base Motion](../base/motion.md) | A target that changes during motion, an exit before unmount, keyframes, or measured reveal | Base retains each channel under a stable key and requests frames through [Window](./window) while active; the caller chooses the visual result. |
| GPUI Component motion | A styled control whose appearance follows the theme | `cx.theme().motion_tokens()` supplies semantic timing, easing, springs, and distances; components compose these with GPUI or Base. |

The application owns the semantic state: whether a dialog is open, which tab is selected, or where a slider points. An animation samples that state for presentation. Keep the result understandable at both endpoints and when motion is disabled.

For a first animation, decide whether a new event may change the destination before the motion finishes. If it can, start with a target-driven Base `transition` or `spring`. Use GPUI `with_animation` when one mounted element should play a known sequence from start to finish. The distinction matters when users click twice: a playback clock and a changing target have different interruption behavior.

## Start with a changing selection

Run the existing [Motion example](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/motion/mod.rs) from the repository root:

```sh
cargo run -p gpui-base-examples --bin motion
```

The example opens on **Sliding time**. Open the **Spring** tab and click **Focus** and **Flow**. The dark indicator moves to the selected half of the track. Switch choices before it settles to see it change direction. The labels remain visible throughout, and the selected label changes color immediately; the motion adds spatial continuity to that state.

### Keep the target in view state

`MotionExample` stores `spring_selected: bool`. The button for each option assigns its value and notifies the view:

```rust
.on_click(move |_, _, cx| {
    _ = entity.update(cx, |this, cx| {
        this.spring_selected = selected;
        cx.notify();
    });
})
```

The boolean selects **Focus** (`false`) or **Flow** (`true`). It is the durable state; the view does not need to store the indicator's current pixel position. Clicking the already selected option leaves the target unchanged.

### Sample the spring on each render

In `spring_demo`, the boolean chooses a target of 0 or 120 pixels. Base's `spring` returns the current value for this frame:

```rust
let x = spring(
    "selector-indicator",
    if self.spring_selected { 120. } else { 0. },
    Spring::new(Duration::from_millis(420)).with_damping(0.68),
    window,
    cx,
);
```

`"selector-indicator"` is a stable channel ID, so later renders find the same spring and preserve its position and velocity when the target changes. On the first render, the spring adopts its target immediately; movement begins only after a click changes that target. While it is moving, Base requests more frames. Keep sampling the channel on every render while the indicator is mounted; the click handler's `cx.notify()` announces a state change, but no render loop or timer is needed.

### Apply the sample to the indicator

The track is a relatively positioned 240-pixel parent. The indicator uses the sampled value as its left inset:

```rust
div()
    .relative()
    .w(px(240.))
    .h_10()
    .child(
        div()
            .absolute()
            .left(px(x))
            .w(px(119.))
            .h_full(),
    )
```

The source also styles the track, indicator, and option buttons. The two targets place the 119-pixel indicator under the corresponding option. These fixed widths make the coordinate calculation easy to follow in the example; a product component should use its own sizing and theme policy.

### Follow one click from input to rest

| Moment | What happens | Who owns it |
| --- | --- | --- |
| Click **Flow** | The handler writes `spring_selected = true` and calls `cx.notify()`. | `MotionExample` owns the selection. |
| Next render | `spring_demo` passes the new target, `120.`, to the channel named `"selector-indicator"`. | Base retains the channel's sampled position and velocity. |
| While moving | `spring` returns a new `x` and requests a later animation frame. The indicator uses that `x` in `.left(px(x))`. | GPUI schedules requested frames; the render code describes each result. |
| At rest | `spring` returns the target without asking for another motion frame. | The view still owns `spring_selected`; a later click can retarget it. |

Try changing `Duration::from_millis(420)` to `Duration::from_millis(700)` in `spring_demo`, then run the same command again and switch **Focus** and **Flow** before the indicator settles. This changes the spring's response policy, not the click handler or semantic selection. Restore `420` afterward. For a second experiment, change the spring target for **Flow** from `120.` to `60.`. The indicator now stops between the two labels while the selected label still changes correctly. Restore `120.`: the target is presentation geometry, whereas `spring_selected` is the answer to “which choice is selected?”

To inspect reduced motion, add `cx.set_reduce_motion(true);` immediately after `gpui_base::init(cx);` in the example's `run` function, then run it again. The selected option still changes, and the spring reaches its target without animated frames. Remove the added line afterward. If the indicator does not move with motion enabled, check that the target changes, the channel ID stays stable, and `spring` is sampled while the indicator remains mounted. Give any additional moving property its own ID; sharing one channel mixes retained values.

### Let an exiting element finish before unmounting

Open **Presence** in the same example and click **Remove**. The example changes `present` to `false` immediately, but keeps rendering the notice while its opacity reaches zero. Its render path samples presence *before* deciding whether to include the notice:

```rust
let sample = Presence::new("presence-notice", self.present)
    .transition(Transition::new(Duration::from_millis(360)).easing(Easing::EaseInOut))
    .sample(window, cx);

// Keep the notice mounted through its exit phase.
if sample.should_render() {
    div().opacity(sample.progress).child("Background task")
} else {
    div()
}
```

The excerpt shows the lifecycle decision; the [full example](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/motion/mod.rs) also lays out and styles the notice. If the view instead used `if self.present` to decide whether to render it, **Remove** would unmount the notice on the first changed render and there would be no exit to draw. Click **Insert** during exit to see `Presence` reverse from the sampled value. Under reduced motion it moves directly to the appropriate endpoint. `present` remains the logical state; a visually exiting notice should not remain an active control after its action is no longer available.

## GPUI's element animation

`Animation::new(duration)` creates a one-shot, linear animation. `AnimationExt::with_animation(id, animation, animator)` wraps an `IntoElement`; the callback receives that element and an eased progress value. GPUI calls it during layout, applies the returned element's style, and requests another frame until the animation ends. The callback may change any property supported by that element, such as opacity or a transform. See the [GPUI {{gpui_pre_version}} animation source](https://docs.rs/gpui-pre/{{gpui_pre_version}}/src/gpui/elements/animation.rs.html) for the wrapper's playback rules. Here `gpui-pre` is the publication and version-alignment package name for GPUI; it is not an additional application layer.

```rust
use std::time::Duration;
use gpui_kit::*;

let entering = div()
    .child("Saved")
    .with_animation(
        ("saved-notice", generation),
        Animation::new(Duration::from_millis(180)),
        |element, progress| element.opacity(progress),
    );
```

### Try one-shot replay beside a retargetable spring

Replace `examples/hello_world/src/main.rs` with this temporary exercise, then run `cargo run -p hello_world`. It uses the existing example package and the same `gpui-kit` dependency:

```rust
use std::time::Duration;
use gpui_kit::*;
use gpui_kit::base::{Spring, spring};
use gpui_kit::component::button::*;
use gpui_kit::prelude::FluentBuilder as _;

struct MotionProbe {
    generation: usize,
    show_notice: bool,
    selected: bool,
}

impl Render for MotionProbe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let offset = spring(
            "motion-probe-indicator",
            if self.selected { 120. } else { 0. },
            Spring::new(Duration::from_millis(420)).with_damping(0.68),
            window,
            cx,
        );

        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .when(self.show_notice, |parent| {
                parent.child(
                    div().child("Saved").with_animation(
                        ("motion-probe-notice", self.generation),
                        Animation::new(Duration::from_millis(900)),
                        |element, progress| element.opacity(progress),
                    ),
                )
            })
            .child(
                Button::new("replay").label("Replay notice").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.generation += 1;
                        this.show_notice = true;
                        cx.notify();
                    },
                )),
            )
            .child(
                Button::new("hide").label("Hide notice").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.show_notice = false;
                        cx.notify();
                    },
                )),
            )
            .child(
                div().relative().w(px(240.)).h_10().child(
                    div()
                        .absolute()
                        .left(px(offset))
                        .w(px(119.))
                        .h_full()
                        .bg(rgb(0x3366cc)),
                ),
            )
            .child(
                Button::new("retarget").label("Switch target").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.selected = !this.selected;
                        cx.notify();
                    },
                )),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| MotionProbe {
                generation: 0,
                show_notice: true,
                selected: false,
            })
        })
        .expect("failed to open window");
    });
}
```

The **Saved** notice fades in once and then remains visible. Click **Replay notice**: incrementing `generation` gives the wrapper a new ID and starts a fresh fade. Click **Hide notice** before the fade finishes: `show_notice = false` removes the wrapper, so that playback stops. **Replay notice** mounts a fresh wrapper and fades in again. Click **Switch target** again while the blue bar is moving: it changes direction from its current sampled position because the Base spring keeps its channel ID while `selected` changes its target. The spring's 420 ms response is a motion scale, not a fixed completion deadline; settling ends at its configured tolerance. A click on one control does not restart the other animation. For the ID experiment, replace `("motion-probe-notice", self.generation)` with `"motion-probe-notice"` and keep the click handler. While the notice stays mounted, subsequent **Replay notice** clicks still update view state, but the finished one-shot does not fade again. Restore the tuple ID and the original example file after the exercise.

To replay the exercise with reduced motion, add `cx.set_reduce_motion(true);` immediately after `gpui_kit::init(cx);` and run it again. **Saved** appears at full opacity without a fade, the spring adopts the selected target, and **Hide notice** still removes the notice. Remove the temporary line afterward.

The bar is a visual probe of the spring value; its fixed 240-pixel track and 120-pixel target are local example geometry. In an application, the selected state must also be clear without movement. To compare a complete selector with labels and interaction states, run the existing [Motion example](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/motion/mod.rs) and open its **Spring** tab.

Bring `AnimationExt` into scope through `gpui_kit::*` (or import the trait explicitly). The ID identifies the animation wrapper, not the text or the visual property. Rendering the same wrapper at the same place with the same ID continues its playback; recreating `Animation::new(...)` on each render does **not** restart it. Change an application-owned generation in the ID when a fresh appearance should replay. Removing the wrapper ends its lifetime. A one-shot animation stays at its final value while the same wrapper remains mounted.

`with_easing(f)` maps normalized time to progress. GPUI supplies functions such as `ease_in_out` and `bounce`; a custom easing function must return a finite value. Overshoot is permitted, so clamp the resulting style property if that property has a narrower valid range. `repeat()` loops locally. `repeat_synced()` loops against an application-wide clock, useful when several indicators should share a phase. `with_max_fps(rate)` limits requests for this animation to at most that rate; invalid nonpositive or nonfinite values are ignored. See [FPS](./fps) for measuring actual frame performance. `with_animations(id, animations, |element, step, progress| ...)` plays a fixed chain and reports the active step index.

GPUI also exposes `AnimationExt::with_spring(id, SpringAnimation<T>, animator)` for a spring-driven element. Its stable ID preserves position and velocity across target changes. A newly mounted spring starts at its target unless `SpringAnimation::from(...)` supplies a starting value. Use this when the spring can live naturally on one element; Base's `spring` is useful when a separately keyed value feeds more than one part of a composition.

The GPUI spring's `SpringPlayback` controls whether it runs, pauses, stops, completes, or cancels. Reduced motion snaps a **running** spring to its target; paused and stopped playback keeps its playback state. Choose the target and playback state from application state rather than treating the wrapper as the source of truth.

### What interruption means here

`with_animation` is a playback of elapsed time, not a transition toward a changing target. Changing the callback's captured endpoint under the same ID continues the existing clock; changing the ID starts a fresh playback. Neither operation automatically samples the current rendered value as the new starting point. For a selection indicator that must reverse smoothly when clicked again, use a target-driven spring or Base `transition` instead.

GPUI's `AnimationExt` honors `App::reduce_motion()`: a one-shot renders at its end, a repeating animation renders at its start, and neither schedules animation frames. Keep a loading state visible through text or another static cue; a stopped spinner by itself cannot tell the user what is happening.

Use the same sequence of questions for each new effect: What application state changes? Which element or channel ID remains stable between renders? What value is sampled on the first render? What happens on a second input before completion? When does the effect stop requesting frames? What remains understandable with reduced motion? These questions catch most apparent “animation did not run” bugs before changing a timing curve.

## GPUI Base Motion: keyed values and lifecycles

Import these APIs from `gpui_kit::base` when an application depends on `gpui-kit`:

```rust
use gpui_kit::*;
use gpui_kit::base::{Easing, Transition, transition};
use std::time::Duration;

let opacity = transition(
    ("save-panel", "opacity"),
    if open { 1.0 } else { 0.0 },
    Transition::new(Duration::from_millis(180)).easing(Easing::EaseOut),
    window,
    cx,
);

div().opacity(opacity)
```

`Transition` here is a **timing policy** for a value, not a styled element. `transition` samples and returns the value; `transition_with_status` also reports `Idle`, `Delayed`, `Running`, or `Finished`. On a changed target, the channel starts at its currently sampled value. A direct reversal shortens the return duration to match the remaining distance. Base requests frames only while the channel is delayed or running. Sample the channel on every render while its owner is present, including when the result is visually hidden, so its retained value settles correctly.

Every independently moving value needs its own stable channel ID. Namespace channels by domain object and property, for example `(project_element_id.clone(), "opacity")` and `(project_element_id.clone(), "height")`, where `project_element_id` is a stable `ElementId`. Two channels sharing an ID can overwrite retained state; changing IDs every render loses continuity. A reorderable list needs item IDs, not row indexes. GPUI's wrapper ID and Base's channel ID serve different lifecycles even when they describe the same visual element.

Use Base `spring(id, target, Spring, window, cx)` when the target may move again before settling. It preserves position **and velocity** on retarget. During direct pointer manipulation, use `Spring::with_travel(false)` so the value tracks the pointer; restore travel on release. A spring's `epsilon` is measured in the target's units, so a pixel offset may need a coarser tolerance than normalized opacity.

Base also provides the following choices:

| Need | API | Lifecycle detail |
| --- | --- | --- |
| Authored value stops | `Keyframes`, `Timing`, `animate_keyframes` | Same ID continues playback; include an application generation in the ID to replay. Timing supports delays, iterations, and playback direction. |
| Distinct ordered steps | `Sequence` | Each step begins at the previous step's absolute end time; the ID plays once until deliberately changed. Changing the active step's target restarts from its sampled value. A sequence does not reverse automatically. |
| Keep content mounted through exit | `Presence` | Sample while logically closed; render until `should_render()` becomes false. Reopening during exit reverses from the current sample. |
| Delay repeated items | `Stagger` | Computes a delay by index and origin; it does not own the list or its IDs. |
| Expand content of unknown height | `MotionReveal` | Measures the child and clips its visible height by caller-supplied progress. It does not sample or animate progress itself. |

See the [Base Motion guide](../base/motion.md) for the full signatures, validation rules, examples, and benchmark. Its `Transition` is distinct from the older `gpui_kit::base::animation::EffectTransition`, which wraps GPUI `with_animation` to apply predefined fade, slide, width, and height effects. For new target-driven work, use `base::motion` primitives and apply the sampled value yourself.

## GPUI Component: semantic motion policy

Styled components use `cx.theme().motion_tokens()` for a shared policy. `MotionTokens` has `duration_instant`, `duration_fast`, `duration_normal`, and `duration_slow`; `easing_enter`, `easing_exit`, and `easing_move`; `spring_control` and `spring_move`; and `distance_short` and `distance_medium`. The defaults are a coherent scale, not a rule that every control must animate. Read tokens from the active theme so a product can tune them in one place.

Try the existing styled control with `cargo run -p gpui-component-story -- SwitchStory`. Click an enabled switch twice quickly: its checked state changes with each click, and its thumb changes direction during travel. A disabled switch in the same story does not respond. In [`crates/component/src/switch.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/switch.rs), follow `checked` into the thumb's target offset and `cx.theme().motion_tokens().spring_move` into the Base `spring` call. In [`crates/component/src/theme/motion.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/theme/motion.rs), temporarily change the default `spring_move` duration from 280 ms to 600 ms and rerun the story: the same checked states now settle more slowly. Restore 280 ms afterward. The story owns the checked boolean, the component maps it to a visual target, and the theme supplies motion policy.

For example, the [Switch source](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/switch.rs) samples a Base spring on `(self.id.clone(), "thumb")` toward the checked or unchecked thumb offset, using `cx.theme().motion_tokens().spring_move`. The [resizable handle source](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/resizable.rs) samples separate length and opacity channels with `duration_fast` and `easing_move`; the hairline remains present even when the indicator fades. [Collapsible](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/collapsible.rs) opts into measured, reversible reveal through `.motion_id(id)`. Without that ID, it mounts and unmounts immediately.

Some components use GPUI's element wrapper for a fixed animation: [Spinner](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/spinner.rs) repeats a rotation, while [Popover](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/popover.rs) animates its entrance. The choice follows whether a component needs continuing target state or a fixed playback, not whether it belongs to the styled layer.

## Reduced motion and frame ownership

GPUI stores the preference on `App`; `cx.reduce_motion()` reads it and `cx.set_reduce_motion(...)` can set it. `gpui_kit::init(cx)` initializes Base's system preference handling. Base reads macOS and Windows at initialization, follows the Linux desktop portal preference as it arrives and changes, and leaves the flag alone on other targets. If the application explicitly sets the flag, Base leaves that application choice in control. Call `gpui_kit::base::apply_system_reduce_motion(cx)` to reread macOS or Windows after initialization.

GPUI element animation adopts a static endpoint as described above. Base's finite transitions, springs, keyframes, presence, and sequences snap to their appropriate target or final state and stop requesting motion frames. `MotionReveal` only consumes progress: when using it directly, pass an endpoint under reduced motion or drive it with a reduced-motion-aware sampler. It may still request a frame when the child's measured height changes. Infinite activity should remain legible in a static state. If a custom element owns its own clock, check `cx.reduce_motion()` and request frames only while useful motion remains; do not unconditionally call `cx.notify()`, `window.refresh()`, or `window.request_animation_frame()` from render.

Use motion to explain appearance, dismissal, expansion, or spatial continuity. Prefer short opacity or transform changes over large layout animation when they communicate the same relationship. Coordinate keyboard focus, hit targets, and semantic state with the transition; painted movement alone does not announce a new state to assistive technology. See [Accessibility](./accessibility) for the semantic side of UI changes.
