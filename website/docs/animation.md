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
| [GPUI Base Motion](/base/motion) | A target that changes during motion, an exit before unmount, keyframes, or measured reveal | Base retains each channel under a stable key and requests frames through [Window](./window) while active; the caller chooses the visual result. |
| GPUI Component motion | A styled control whose appearance follows the theme | `cx.theme().motion_tokens()` supplies semantic timing, easing, springs, and distances; components compose these with GPUI or Base. |

The application owns the semantic state: whether a dialog is open, which tab is selected, or where a slider points. An animation samples that state for presentation. Keep the result understandable at both endpoints and when motion is disabled.

## GPUI's element animation

`Animation::new(duration)` creates a one-shot, linear animation. `AnimationExt::with_animation(id, animation, animator)` wraps an `IntoElement`; the callback receives that element and an eased progress value. GPUI calls it during layout, applies the returned element's style, and requests another frame until the animation ends. The callback may change any property supported by that element, such as opacity or a transform.

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

Bring `AnimationExt` into scope through `gpui_kit::*` (or import the trait explicitly). The ID identifies the animation wrapper, not the text or the visual property. Rendering the same wrapper at the same place with the same ID continues its playback; recreating `Animation::new(...)` on each render does **not** restart it. Change an application-owned generation in the ID when a fresh appearance should replay. Removing the wrapper ends its lifetime. A one-shot animation stays at its final value while the same wrapper remains mounted.

`with_easing(f)` maps normalized time to progress. GPUI supplies functions such as `ease_in_out` and `bounce`; a custom easing function must return a finite value. Overshoot is permitted, so clamp the resulting style property if that property has a narrower valid range. `repeat()` loops locally. `repeat_synced()` loops against an application-wide clock, useful when several indicators should share a phase. `with_max_fps(rate)` limits requests for this animation to at most that rate; invalid nonpositive or nonfinite values are ignored. `with_animations(id, animations, |element, step, progress| ...)` plays a fixed chain and reports the active step index.

GPUI also exposes `AnimationExt::with_spring(id, SpringAnimation<T>, animator)` for a spring-driven element. Its stable ID preserves position and velocity across target changes. A newly mounted spring starts at its target unless `SpringAnimation::from(...)` supplies a starting value. Use this when the spring can live naturally on one element; Base's `spring` is useful when a separately keyed value feeds more than one part of a composition.

The GPUI spring's `SpringPlayback` controls whether it runs, pauses, stops, completes, or cancels. Reduced motion snaps a **running** spring to its target; paused and stopped playback keeps its playback state. Choose the target and playback state from application state rather than treating the wrapper as the source of truth.

### What interruption means here

`with_animation` is a playback of elapsed time, not a transition toward a changing target. Changing the callback's captured endpoint under the same ID continues the existing clock; changing the ID starts a fresh playback. Neither operation automatically samples the current rendered value as the new starting point. For a selection indicator that must reverse smoothly when clicked again, use a target-driven spring or Base `transition` instead.

GPUI's `AnimationExt` honors `App::reduce_motion()`: a one-shot renders at its end, a repeating animation renders at its start, and neither schedules animation frames. Keep a loading state visible through text or another static cue; a stopped spinner by itself cannot tell the user what is happening.

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

See the [Base Motion guide](/base/motion) for the full signatures, validation rules, examples, and benchmark. Its `Transition` is distinct from the older `gpui_kit::base::animation::EffectTransition`, which wraps GPUI `with_animation` to apply predefined fade, slide, width, and height effects. For new target-driven work, use `base::motion` primitives and apply the sampled value yourself.

## GPUI Component: semantic motion policy

Styled components use `cx.theme().motion_tokens()` for a shared policy. `MotionTokens` has `duration_instant`, `duration_fast`, `duration_normal`, and `duration_slow`; `easing_enter`, `easing_exit`, and `easing_move`; `spring_control` and `spring_move`; and `distance_short` and `distance_medium`. The defaults are a coherent scale, not a rule that every control must animate. Read tokens from the active theme so a product can tune them in one place.

For example, the [Switch source](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/switch.rs) samples a Base spring on `(self.id.clone(), "thumb")` toward the checked or unchecked thumb offset, using `cx.theme().motion_tokens().spring_move`. The [resizable handle source](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/resizable.rs) samples separate length and opacity channels with `duration_fast` and `easing_move`; the hairline remains present even when the indicator fades. [Collapsible](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/collapsible.rs) opts into measured, reversible reveal through `.motion_id(id)`. Without that ID, it mounts and unmounts immediately.

Some components use GPUI's element wrapper for a fixed animation: [Spinner](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/spinner.rs) repeats a rotation, while [Popover](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/popover.rs) animates its entrance. The choice follows whether a component needs continuing target state or a fixed playback, not whether it belongs to the styled layer.

## Reduced motion and frame ownership

GPUI stores the preference on `App`; `cx.reduce_motion()` reads it and `cx.set_reduce_motion(...)` can set it. `gpui_kit::init(cx)` initializes Base's system preference handling. Base reads macOS and Windows at initialization, follows the Linux desktop portal preference as it arrives and changes, and leaves the flag alone on other targets. If the application explicitly sets the flag, Base leaves that application choice in control. Call `gpui_kit::base::apply_system_reduce_motion(cx)` to reread macOS or Windows after initialization.

GPUI element animation adopts a static endpoint as described above. Base's finite transitions, springs, keyframes, presence, and sequences snap to their appropriate target or final state and stop requesting motion frames. `MotionReveal` only consumes progress: when using it directly, pass an endpoint under reduced motion or drive it with a reduced-motion-aware sampler. It may still request a frame when the child's measured height changes. Infinite activity should remain legible in a static state. If a custom element owns its own clock, check `cx.reduce_motion()` and request frames only while useful motion remains; do not unconditionally call `cx.notify()`, `window.refresh()`, or `window.request_animation_frame()` from render.

Use motion to explain appearance, dismissal, expansion, or spatial continuity. Prefer short opacity or transform changes over large layout animation when they communicate the same relationship. Coordinate keyboard focus, hit targets, and semantic state with the transition; painted movement alone does not announce a new state to assistive technology.
