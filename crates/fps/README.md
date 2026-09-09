# gpui-fps

A realtime performance HUD for GPUI applications: frames per
second, frame time, dropped frame rate, and this process' GPU, CPU and memory
usage.

```
┌──────────────────────────┐
│  ﹋﹏  118 FPS  ﹋︿﹏﹋   │  ← the trace runs behind the headline
│ FRAME             8.4 ms │  ← what a typical frame cost
│ P95              14.1 ms │  ← what its slow tail cost
│ DROP 0.0%       INV  1.0 │
│ GPU                31.0% │
│ CPU 142%       MEM 84 MB │
└──────────────────────────┘
```

Frame data comes from GPUI's own frame trace (`gpui::FrameTimingCollector`), so
the numbers are what the framework actually spent in `Window::draw` rather than
an estimate measured from the outside. Both the trace and the `FRAME` reading
are colored against the frame budget — green within budget, amber up to twice
the budget, red beyond.

It does not depend on `gpui-component`, so it works in any GPUI application.

## Usage

### 1. Add the dependency

```toml
[dependencies]
gpui-fps = { git = "https://github.com/longbridge/gpui-kit" }
```

It must resolve to the same `gpui` as your application. Both depending on the
same `gpui-pre` version is enough — Cargo unifies them — but a `[patch]`, a
different `gpui-pre` version, or a checkout of the Zed repository will produce
two incompatible `gpui` crates, and the error will be about mismatched `Window`
types rather than about versions.

### 2. Render it

`fps_monitor` returns an element. Render it wherever the HUD should appear; the
parent must be `relative()`, because the HUD positions itself absolutely.

```rust
use gpui::*;
use gpui_fps::fps_monitor;

struct Example {
    show_fps: bool,
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .size_full()
            .child("your app")
            .when(self.show_fps, |this| this.child(fps_monitor(window, cx)))
    }
}
```

Call it at most once per window — a second call renders the same monitor twice.
The monitor behind it is created on first use and reused afterwards, one per
window, so calling this from `render` every frame is fine.

To show it unconditionally, drop the `when` and just `.child(fps_monitor(window, cx))`.

### 3. Toggle it

There is no `toggle` function. The flag lives with you, which keeps it
observable and easy to persist next to your own settings:

```rust
actions!(example, [ToggleFps]);

// At startup:
cx.bind_keys([KeyBinding::new("cmd-alt-f", ToggleFps, None)]);

// On the element that owns the flag:
div()
    .on_action(cx.listener(|this: &mut Example, _: &ToggleFps, _, cx| {
        this.show_fps = !this.show_fps;
        cx.notify();
    }))
```

If the flag lives in a global instead — so a menu elsewhere in the window can
flip it — remember to `window.refresh()` after changing it, since mutating a
global does not by itself mark the view dirty. That is what the GPUI Component
story does: `AppState.show_fps_monitor`, toggled from **FPS Monitor** in the
settings menu, and persisted to `target/state.json` through an
`observe_global::<AppState>`.

Click the HUD to collapse it to a small tag showing just the frame rate, and
click again to expand.

## Customization

The call takes no options. For a different corner or frame budget, compose the
two pieces it uses and render the monitor yourself — it is an ordinary view, so
it can live in a status bar just as well as in an overlay:

```rust
use gpui_fps::{FpsMonitor, FpsOverlay};

let monitor = cx.new(|cx| {
    FpsMonitor::new(window, cx)
        .capacity(240)                                  // frames kept in the trace (default 120)
        .frame_budget(Duration::from_micros(6_944))     // 144Hz (default is 60Hz)
        .continuous(true)                               // default true, see below
        .show_resources(true)                           // GPU, CPU and memory (default true)
        .resource_interval(Duration::from_millis(500))  // default 500ms
});

// Embedded:
div().child(monitor.clone())
// Or pinned to a corner of a relative parent:
div().relative().child(FpsOverlay::new(&monitor).anchor(Anchor::BottomLeft))
```

The palette is not configurable. Its contrast is load bearing — see the note
below — and an application that could override it could just as easily make the
HUD unreadable.

### `continuous`

On by default, this requests a frame on every render so the window keeps drawing
back to back. That is what makes the reading behave like an in-game FPS counter,
and it carries the same caveat: **the window never idles, so the number is the
frame rate the application can sustain, not the rate it happens to be drawing
at**, and the HUD itself keeps the CPU and GPU busy.

Turn it off to measure the real workload. The HUD then only updates when the
window redraws for its own reasons, and reads zero while the window is idle.

## Notes

- The trace follows every frame, but the numbers are republished twice a second.
  Recomputed per frame they flicker through digits too fast to read. `FRAME` is
  the mean over that interval rather than the latest frame, which at this cadence
  would be an arbitrary sample.
- The frame rate is graded against the target _rate_ with a 5% tolerance, not by
  comparing `1/fps` against the budget. Under vsync a healthy 60Hz display reads
  58 to 60 and never exactly 60.00, so an exact comparison would paint a
  perfectly healthy application as over budget.
- The backdrop is nearly opaque (alpha 0.92) on purpose. GPUI cannot read the
  pixels under an element, so the HUD has no way to adapt to what it covers; the
  only way to stay readable over any window background is to keep that
  background out of the composite. At 0.92 every foreground clears 4.5:1 even
  over pure white.
- GPUI records frame timings into a process-wide buffer, so the monitor filters
  by window id. Each window needs its own monitor to get its own numbers, which
  is what `fps_monitor` does for you.
- Frame tracing is a global switch that clears its buffer when disabled, so
  monitors reference count it and never turn it off while another monitor — or
  the host application's own profiling — still needs it.
- The headline is graded on the frame _rate_ and `FRAME` on the frame _time_,
  which is why they can disagree. A window that is idle draws a handful of
  frames a second, so the headline goes red while every one of those frames was
  in fact drawn well inside the budget — `FRAME` staying green is what says the
  application is fine and simply has nothing to redraw.
- `FRAME` is the mean draw time and `P95` the time 95% of the retained frames
  came in under. A run of quick frames pulls a mean down over a spike, so the
  mean alone reads comfortable through jank the user can see; the two together
  say both what a frame usually costs and what its slow tail costs. `P95` is
  robust to a single outlier by construction — the chart and its axis are what
  show that one.
- `INV` is the mean number of invalidations coalesced into one frame. One means
  every redraw the window was asked for became a frame; well above one means it
  was asked far more often than it could answer, and the excess is work being
  thrown away. It does not show up in the frame times at all, since each frame
  that _is_ drawn may be perfectly quick. It is the one reading the HUD does not
  grade: in continuous mode the monitor requests an animation frame of its own
  every render, so an application invalidating once a frame measures two, and
  the baseline depends on a switch the HUD cannot judge against.
- CPU, memory and GPU are sampled on a background thread, and each reading is
  the mean over a trailing three second window — they are coarse samples of
  quantities that move between one sample and the next, and published raw they
  churn too fast to read. Resource sampling is unavailable on the web.
- `CPU` is on the single-core scale that `top`, Activity Monitor and Task
  Manager's per-process column all use: **100 is one saturated logical core**, so
  a process spread over a core and a half reads 140. It is deliberately not
  divided by the core count — normalizing so that 100 is the whole machine makes
  the same work read 12% on a four core laptop and 2% on a twenty-four core
  desktop, and pushes every interesting value into the bottom of the range,
  where a UI thread pinning a core looks idle.
- `MEM` is the memory this process is _responsible for_, not its resident set.
  RSS counts the read-only pages of every shared library the process maps, which
  on a windowed application is a graphics stack running to hundreds of megabytes
  of code it neither allocated nor can release — and which every other window on
  the machine is mapping at the same time, so the number moves when a different
  program starts. Each platform reads the counter its own activity monitor
  shows, and falls back to RSS where there is none:
  - **macOS** — `ri_phys_footprint` from `proc_pid_rusage`, the counter behind
    Activity Monitor's Memory column and the one jetsam judges a process on.
  - **Windows** — `PrivateUsage` from `GetProcessMemoryInfo`, this process'
    private commit, which Task Manager shows as its commit size.
  - **Linux** — `RssAnon` from `/proc/self/status`: resident anonymous memory,
    the heap and stacks and private mappings, and none of the files it maps.
    `Private_Dirty` from `smaps_rollup` is the closer analogue of the other two,
    but it is computed by walking every mapping under the address space lock —
    ~425µs a read against ~5µs — and a HUD should not perturb what it measures
    to account for a few megabytes of relocations.
- `GPU` is this process' own share, like the CPU beside it and unlike a
  device-wide reading, which would move for work the application cannot act on.
  Each platform is read where it attributes GPU time per process, and none of
  them needs a vendor SDK or elevated privileges:
  - **macOS** — `accumulatedGPUTime` on the accelerator clients this process
    owns in the IO registry, which is what Activity Monitor's GPU column shows.
  - **Windows** — the `GPU Engine` PDH counters, filtered to this process' own
    instances, which is what Task Manager's GPU column shows.
  - **Linux** — `drm-engine-*` in `/proc/self/fdinfo`, which is what `nvtop` and
    `intel_gpu_top` read.

  Where several engines can run at once — Windows and Linux — the reading is the
  busiest engine type rather than their sum, so it stays inside 100% while the
  GPU still has headroom.

- **The GPU row is left out entirely where no per-process counter is reachable**,
  rather than reading a flat zero: the web, an Intel Mac, whose accelerator
  clients do not publish `AppUsage`, and a Linux driver such as nvidia's
  proprietary one, which keeps its accounting in NVML instead of `fdinfo`.

## Example

```bash
cargo run -p fps_monitor
```

A port of three.js' `webgl_lines_colors` demo — Hilbert curves smoothed with a
centripetal Catmull-Rom spline — whose curve count can be dialed up and down, so
the trace can be watched reacting to real rendering load.
