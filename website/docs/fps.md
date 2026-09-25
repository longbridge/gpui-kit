---
title: FPS Monitor
description: Read the gpui-fps HUD — what MAX FPS is, why it is derived rather than counted, and what each row measures.
order: -15
---

# FPS Monitor

`gpui-fps` overlays a performance HUD on a [Window](./window): a headline rate, a rolling
frame time trace, and this process' CPU, GPU and memory. It depends only on
`gpui`, so any GPUI application can use it.

```rs
use gpui_fps::fps_monitor;

fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div()
        .relative()
        .size_full()
        .child(self.content.clone())
        .when(self.show_fps, |this| this.child(fps_monitor(window, cx)))
}
```

The parent must be `relative()`, the HUD positions itself absolutely, and
whether it is on screen is the caller's to decide.

## 120 Hz is a frame budget, not a refresh promise

“120 FPS” is often shorthand for a **target on a 120 Hz display**: one refresh
period is `1 / 120 s`, about **8.33 ms**. It says nothing by itself about how
often an idle application asks for a frame, or whether a particular screen can
finish and present every frame within that budget. A 60 Hz display gives about
16.67 ms per refresh. Variable-refresh displays and platform scheduling add
further context; neither number is an application's measured FPS.

GPUI responds to invalidation: an input or state change can make a view dirty;
the window later draws and presents the updated scene. Multiple changes may be
coalesced. An animation can request another frame while it is progressing, and
should stop requesting animation frames when it settles. The GPUI render
pipeline does not require an application to rebuild its element tree 120 times
per second merely because the display is 120 Hz. Platform frame callbacks may
still occur, and some backends may present an unchanged scene without rebuilding
its elements. An FPS HUD with its own timer, an input device, a platform
compositor, or application code can also cause activity while the app looks
idle. Measure idle CPU and GPU use on the target
platform rather than inferring zero work or continuous full redraws from the
refresh rate.

<figure class="frame-timeline" data-frame-timeline>
  <div class="frame-timeline__header"><strong>When GPUI needs a new drawing pass</strong><button type="button" data-frame-timeline-toggle aria-pressed="false">Pause animation</button></div>
  <div class="frame-timeline__tracks" role="img" aria-label="Conceptual timeline: idle needs no new element tree; an input event triggers one draw; an active animation requests successive draws.">
    <div class="frame-timeline__phase" data-phase="idle"><span>Idle</span><div class="frame-timeline__ticks"></div><small>No new element tree required</small></div>
    <div class="frame-timeline__phase" data-phase="event"><span>Input</span><div class="frame-timeline__ticks"><i></i></div><small>Event → invalidation → draw</small></div>
    <div class="frame-timeline__phase" data-phase="animation"><span>Animation</span><div class="frame-timeline__ticks"><i></i><i></i><i></i><i></i></div><small>Requested frames while active</small></div>
  </div>
  <figcaption>Conceptual frame requests, not measured FPS or a claim about platform presentation of cached scenes.</figcaption>
</figure>

For a sustained **120 displayed FPS** claim, measure a representative *whole
window* in a release build, at the target size and hardware. Include realistic
data, scrolling or animation, and report a distribution of frame times and
present intervals after warmup. One small component completing `render` in
less than 8.33 ms only measures part of one frame. Layout, prepaint, paint,
submission, GPU work, the compositor, and scheduling can consume the rest.
Likewise, a low average can hide slow `P95` frames and visible stutter.

GPUI does work on both sides of the CPU/GPU boundary: application state,
element construction, layout, and paint preparation run on the CPU; the
platform renderer and compositor use the GPU where supported. Text, scene
complexity, cache behavior, graphics backend, and hardware determine the
bottleneck for a particular workload. “CPU-bound” or “GPU-bound” is a
profiling result for that workload, not a fixed property of every GPUI app.

## Immediate, retained, and hybrid describe different layers

These terms answer different questions, so a single label is easy to
misread:

| Layer | What GPUI does | What persists |
| --- | --- | --- |
| View state | An [`Entity<T>`](./entity) owns application state across updates and frames. | The Entity and its subscriptions, tasks, and child handles while owned. |
| UI description | On a needed [render pass](./render), `Render` or `RenderOnce` constructs elements from current state. This resembles immediate-style declaration. | The element description is specific to that pass; `RenderOnce` does **not** mean once per display frame. |
| Reuse and drawing | GPUI can reuse eligible [cached views](./view-cache) and keyed element state, and submits paint work for the window. | Cache and platform scene resources can outlive an individual element description. |

“Retained” accurately describes the Entity state and selected caches;
“immediate-style” describes construction of a current element description;
“hybrid” is also the term in [Zed's GPUI overview](https://github.com/zed-industries/zed/blob/bcf6582ce3500df93a8a39366640173e6786cea6/crates/gpui/README.md#the-big-picture). None of those terms defines the
idle present rate or proves a performance number. The [Render](./render),
[Element](./element), and [View Cache](./view-cache) guides show the ownership
and invalidation boundaries in code.

Add `gpui-fps` from the same GPUI dependency family as the application. Render
`fps_monitor(window, cx)` at most once per window; repeated calls reuse the
same monitor and would draw it twice. The repository's runnable example is
`cargo run --release -p fps_monitor`. Use `cargo run -p fps_monitor` only when
comparing Debug builds: unoptimized framework code can change the measured
frame cost substantially. See [Installation](./installation#improve-development-runtime-performance)
for the repository's development profile.

## From an update to a displayed frame

1. A state change followed by `cx.notify()`, an animation request, or a window
   refresh invalidates the window. Several invalidations can become one draw.
2. GPUI runs `Window::draw`, building dirty views' elements and doing layout,
   prepaint, and paint work. The profiler records `draw_start`, `draw_end`, and
   the invalidation count for this window. `FRAME` uses **only**
   `draw_end - draw_start`.
3. GPUI submits the drawn scene to the platform in a separate present step.
   Its `present_end` timestamp feeds `FPS` and `INTERVAL`.

This distinction matters: a 5 ms `FRAME` says that GPUI's draw completed in
5 ms. It does not say the request waited only 5 ms, that platform submission
took 5 ms, or that the GPU and compositor displayed it within 5 ms. The
profiler also exposes `dirty_to_draw_duration()` and `PresentTiming` for
custom instrumentation, but those are separate measurements from this HUD's
`FRAME`. See GPUI's [frame timing definitions](https://docs.rs/gpui-pre/0.3.6/gpui/profiler/struct.FrameTiming.html)
and the [sampler implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/fps/src/sampler.rs).

## The headline

The big figure answers one of two questions, and the `MAX` marker says which.
**Right-click** to switch; **click** to collapse the HUD to a tag.

| | Reads | Means |
| --- | --- | --- |
| `MAX FPS` (default) | `1 / mean FRAME`, capped by the display when its rate is known | Estimated draw throughput for the sampled workload; it omits presentation and GPU completion |
| `FPS` | Rate inferred from recent present timestamps | The observed present cadence while the window has enough samples |

They are different questions. For example, this HUD requests a readout update
twice a second even if the application has no visible changes. That can give
the window a low observed `FPS` while `MAX FPS` remains high; neither figure
alone proves continuous idle rendering or sustained performance in a busier
screen.

### Why MAX is derived rather than counted

The obvious way to make a frame counter read "as fast as this UI can go" is to
keep asking for frames, the way an in-game counter does. That is not free here.
Marking any view dirty schedules a **window** draw, and GPUI re-renders every
view in that window outside an [`Entity::cached`] boundary (see [View Cache](./view-cache)) — so each frame the
HUD asked for would add window layout and [paint](./paint) work, and the CPU
row underneath would include work the HUD itself was causing.

The measured draw cost offers an estimate of CPU-side headroom for the sampled
window. Its reciprocal is `MAX FPS`, but it does not measure a sustained
animation or the complete present pipeline. The HUD does not request
continuous frames to calculate it; its twice-second readout clock is
described below.

### Why MAX is capped by asking, not by measuring

A frame drawn in 3ms reads as 333, and no panel will ever show that. Counting
presents had the ceiling for free — frames go to the compositor on vsync — and
a figure derived from frame cost has no such bound, so the cap is applied
explicitly.

It cannot be inferred. The gaps between a window's presents are whole multiples
of the panel's period, so they bound it **from below and never from above**:
41.7ms is six refreshes at 144Hz and one at 24Hz, and nothing in the timing
distinguishes them. Every estimate tried read a real window wrong — 169 and 149
from the shortest and the densest gaps, 75 from a window drawing every other
refresh, and 24 from an application whose own timer happened to fire every
41.7ms.

So the platform is asked instead. GPUI hands out the platform's own display
handle through `DisplayId`, and the HUD takes it from there:

- **macOS** — `CGDisplayCopyDisplayMode` on the `CGDirectDisplayID`. A built-in
  panel reports no fixed rate, which is the truth on ProMotion, and is read as
  no cap.
- **Windows** — `EnumDisplaySettingsW` on the monitor's device name.
- **Wayland** — the outputs are enumerated on a second connection and matched
  to GPUI's displays by the identity it derives from their names, because
  object ids are per-connection and mean nothing across one.
- **X11 and everything else** — no query, so no cap.

The answer is re-asked when the window moves to another display and not
otherwise. Where nobody will say, the reading is left uncapped rather than held
to a guess: a ceiling under the truth hides the figure the reader came for.

## The rows

| Row | Measures |
| --- | --- |
| `INTERVAL` | Mean time between presents. The same figure a platform overlay calls its frame interval, and the reciprocal of `FPS`. A wide gap between it and `MAX` is an idle window, not a slow one. |
| `FRAME` | Mean `Window::draw` cost. Graded against the frame budget: this is the row to read when something feels slow. |
| `P95` | The slow tail of the same frames, graded the same way. |
| `DROP` | Share of frames that overran the budget. |
| `INV` | Invalidations coalesced into one frame. Well above one means the window was asked to redraw more often than it could. |
| `GPU` | This process' GPU use where the platform exposes a per-process counter; otherwise the row is absent. |
| `CPU` | This process, on the scale `top` and Activity Monitor use: 100 is one saturated core, so a process spread across one and a half cores reads about 150. |
| `MEM` | Process-attributed memory: macOS physical footprint, Windows private usage, or Linux resident anonymous memory, with an RSS fallback. It is not the same counter on every platform. |

`FRAME`, `P95` and `DROP` are graded against the frame budget: one refresh of
the display the window is on, where the platform reports the rate above, and
one 60Hz frame where it does not. `frame_budget()` pins a budget of your own
instead, and the display then no longer replaces it.

`FRAME`, `P95`, `DROP`, and `INV` use the latest **120 retained draw samples**
by default, not a fixed number of seconds; `capacity()` changes that length.
`P95` is the nearest observed 95th-percentile frame, while `DROP` is the
fraction whose draw time is *strictly greater* than the budget. The trace
plots these draw costs, newest at the right. `FPS` and `INTERVAL` use present
timestamps in a rolling one-second window. The displayed numbers are
republished every 500 ms; CPU, GPU, and memory are background samples averaged
over the trailing three seconds. Resource sampling is unavailable on the web.

These windows can describe different moments. After an expensive interaction,
spikes may remain in the frame trace, and repeated slow frames in `P95`, after
`FPS` has already returned to idle. The headline is not graded because a low observed
rate can simply mean that the application has stopped asking for frames.

## Reproduce and diagnose a slowdown

1. Run `cargo run --release -p fps_monitor` from this repository. Let the
   window and HUD settle, then note the display, window size, build profile,
   and baseline `FRAME`, `P95`, `DROP`, `INV`, and resource rows. The example's
   `+ load` and `− load` controls change its curve count; compare the same
   count and window size between runs. Its scene calls
   `window.request_animation_frame()`, so it provides a sustained draw load.
2. Increase the load and watch `FRAME` against the display's budget. For a
   60 Hz target one refresh is about 16.7 ms; for 120 Hz it is about 8.3 ms.
   If `FRAME` is low but `P95` or `DROP` rises, investigate intermittent
   work. If `INV` rises while draw costs stay low, inspect redundant update
   or animation requests. If only `INTERVAL` grows, first check whether the
   window is idle, occluded, or waiting for presentation.
3. Repeat the same interaction several times after warmup. Capture a CPU
   profile for expensive view construction, layout, or paint submission; use
   the platform's GPU/compositor tools when `FRAME` is low but visible output
   still stutters. Narrow the workload, change one cause, and compare the
   same build profile and conditions again. The HUD locates a symptom; it
   does not attribute time to a particular view or GPU command.

For application-specific records, GPUI's `profiler` feature exposes
`FrameTimingCollector::new()` and `collect_unseen()`. They yield `FrameEvent::Draw`
and `FrameEvent::Present`; filter each by `window_id`, and keep the collector
alive while sampling. The trace is process-wide and is enabled by
`gpui::profiler::set_trace_enabled(true)`; disabling it clears its buffers.
The HUD manages this switch while visible. These raw records are useful when
you need to correlate a frame with an app event, including the first
invalidation timestamp (`dirty_at`) and present submission. They do not by
themselves measure GPU completion or photons on screen. See GPUI's
[collector API](https://docs.rs/gpui-pre/0.3.6/gpui/profiler/struct.FrameTimingCollector.html)
and [GPUI Kit's monitor](https://github.com/longbridge/gpui-kit/blob/main/crates/fps/src/monitor.rs).

## The first frames are not measured

A window's first frames are its most expensive — shaders, the glyph atlas, the
icons, every cache still cold — and they are not what the application costs to
run. One of them is a hundred milliseconds against a budget of sixteen, and a
HUD that has seen eight frames would report it as a twelfth of the window's
work, in amber, before the reader has done anything at all.

So the sampler discards two things: everything GPUI recorded before the HUD was
mounted, which is either somebody else's history or the cold start, and the
first few frames after it. The default reading of a window that just opened is
a healthy one.

## What the HUD itself costs

One frame every 500ms. It does not drive the frame loop, but it does need a
clock — nothing else would wake a HUD in a window that has stopped drawing, and
the figures would freeze at whatever the application last drew. That clock also
carries the CPU, GPU and memory sample.

Those frames are not measured. To GPUI the clock's `notify` is an invalidation
like any other, answered with a full draw of the window, and left in the
readings it would be a cold frame every 500ms reported as the application's
`FRAME` and `MAX`. So the clock announces each one, and the sampler leaves out
the draw that answered it — unless the application asked for that frame too, in
which case the work was wanted and the cost counts.

Hidden, the HUD costs nothing. Two ticks without being rendered — a second —
and the clock stops, the resource probe with it, and the HUD lets go of GPUI's
frame trace unless something else is holding it. The next render starts it all
again from an empty sampler: the trace buffer was cleared with the switch, and
the frames the window drew meanwhile were nobody's to report.

[`Entity::cached`]: https://docs.rs/gpui-pre/0.3.6/gpui/struct.Entity.html#method.cached
