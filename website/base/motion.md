---
title: Motion
description: Typed transitions, springs, keyframes, presence, stagger, and reduced-motion behavior in gpui-base.
order: 4
example: motion
exampleKind: base
---

# Motion

`gpui-base` owns deterministic motion sampling and lifecycle while leaving every visual choice to the application. It provides stable keyed state, interruption, reversal, animation-frame requests, and reduced-motion behavior without imposing product timing or styling.

Run the interactive companion for this guide:

```bash
cargo run -p gpui-base-examples --bin motion
```

The example contains five separate demos. Use the tabs at the top to inspect one capability at a time.

## Capability map

| Demo | API | What it demonstrates |
| --- | --- | --- |
| Sliding time | `transition` | Four independently rolling digits, 08:00–20:00, with targets changing faster than the transition settles |
| Spring | `spring` | A segmented-control indicator that preserves velocity when rapidly retargeted |
| Keyframes | `Keyframes`, `Timing`, `animate_keyframes` | A repeating multi-stop activity signal |
| Stagger | `Stagger` | Allocation-free timing offsets across a list |
| Presence | `Presence` | Exit animation that keeps content mounted until it becomes absent |

The library also exposes `Easing`, `Discrete`, `MotionTransform`, and `MotionReveal`. They compose with the same primitives rather than requiring separate animation runtimes.

## Target transitions

Use `transition` for a value moving toward a target over a known duration. Every independently animated value needs a stable ID.

```rust
let opacity = transition(
    ("save-dialog", "opacity"),
    if open { 1.0 } else { 0.0 },
    Transition::new(Duration::from_millis(180)).easing(Easing::EaseOut),
    window,
    cx,
);
```

Retargeting starts at the currently sampled value. Direct reversal shortens the return duration, so reversing early does not spend a full duration retracing a short distance. `transition_with_status` additionally returns `Idle`, `Delayed`, `Running`, or `Finished`.

`Easing` includes CSS keyword curves, cubic Bézier curves, all CSS step positions, and piecewise `linear()` stops. Invalid parameters return typed errors.

## Springs

Use `spring` when the target may change while moving. It preserves both position and velocity, which makes it suitable for selection indicators and settling spatial values.

```rust
let x = spring(
    "selected-indicator",
    selected_x,
    Spring::new(Duration::from_millis(420)).with_damping(0.72),
    window,
    cx,
);
```

Do not make a pointer-controlled value chase the pointer through a spring. Set `with_travel(false)` during direct manipulation and restore travel after release.

`with_damping` requires a finite, non-negative ratio; `with_epsilon` requires a finite value greater than zero and interprets it in the target's own units. The builders panic for invalid trusted constants. Use `try_with_damping` and `try_with_epsilon` for configuration or user-provided values. Normalized values normally keep the `0.001` default; pixel motion can use a coarser tolerance such as `0.1`.

## Keyframes and timing

`Keyframes` describes validated value stops. `Timing` uses absolute elapsed time and supports signed delays, finite or infinite iterations, and normal, reverse, or alternating playback.

```rust
let frames = Keyframes::try_new([
    Keyframe::new(0.0, 0.25),
    Keyframe::new(0.45, 1.0).ease(Easing::EaseOut),
    Keyframe::new(1.0, 0.25),
])?;

let opacity = animate_keyframes(
    "activity",
    &frames,
    Timing::new(Duration::from_millis(1400))
        .iterations(IterationCount::Infinite),
    window,
    cx,
).value;
```

Offsets must start at `0`, end at `1`, and be monotonic. Use `Discrete` when a value cannot be interpolated.

`animate_keyframes` retains its playback start time under the supplied stable ID. Re-rendering with the same ID continues the current sequence. To replay it, include an application-owned generation in the ID, such as `("notification-enter", generation)`, and increment that generation for each replay.

## Presence and stagger

`Presence` separates logical visibility from physical mounting. Its phases are entering, present, exiting, and absent. Render while `should_render()` is true and use `progress` for the chosen visual properties. Reopening during exit reverses from the current sample.

`Stagger` calculates a delay for an index from the first, last, center, or a chosen origin. It does not allocate a schedule or own list identity:

```rust
let stagger = Stagger::new(Duration::from_millis(80), StaggerOrigin::First);
let delay = stagger.delay(index, item_count);
```

## Measured reveal

`MotionReveal` measures a child at its natural size and clips its visible height by progress. `Collapsible::motion_id(...)` is the convenient control-level facade. Without a motion ID, the control keeps immediate mount/unmount behavior.

## Reduced motion and performance

Transitions, springs, keyframes, presence, and reveal-compatible controls honor GPUI's reduced-motion preference. Finite motion snaps to the target, synchronizes retained state, and leaves no pending animation frame. Motion must never be the only way state is communicated.

The pure steady sampling paths measured by the benchmark—timing/easing, keyframe lookup, analytic spring integration, and stagger delay calculation—are allocation-free. Keyed transition, spring, presence, and reveal lifecycles are covered by GPUI retained-state and frame-request tests because those updates belong to the framework lifecycle rather than the pure sampler. Sampling uses absolute elapsed time, and keyframe lookup uses binary search. Run the release benchmark with:

```bash
cargo bench -p gpui-base --bench motion
```

Choose the smallest suitable primitive: `transition` for duration-based targets, `spring` for changing spatial targets, keyframes for authored sequences, `Presence` for exit-before-unmount, and `Stagger` for list choreography.

## Benchmark results

Measured on Linux x86_64 with a release build, 31 batches, and 200 iterations per batch:

| Workload | Median | P95 | Worst | Allocations |
| --- | ---: | ---: | ---: | ---: |
| 1,000 scalar timing + easing samples | 26.490 µs | 26.567 µs | 27.290 µs | 0 |
| 1,000 keyframe samples, 2 frames | 21.656 µs | 21.707 µs | 21.729 µs | 0 |
| 1,000 keyframe samples, 8 frames | 25.197 µs | 25.251 µs | 25.269 µs | 0 |
| 1,000 keyframe samples, 32 frames | 27.932 µs | 27.969 µs | 27.971 µs | 0 |
| 1,000 analytic spring integration samples | 6.042 µs | 6.106 µs | 6.216 µs | 0 |
| 1,000 stagger delay calculations | 0.574 µs | 0.583 µs | 0.587 µs | 0 |

The scalar timing/easing workload remains below its 100 µs median budget. These figures are a reproducible development baseline rather than a cross-platform guarantee; run the benchmark on each target platform when platform-specific performance matters.
