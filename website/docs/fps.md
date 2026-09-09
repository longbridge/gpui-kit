---
title: FPS Monitor
description: Read the gpui-fps HUD — what MAX FPS is, why it is derived rather than counted, and what each row measures.
order: -5
---

# FPS Monitor

`gpui-fps` overlays a performance HUD on a window: a headline rate, a rolling
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

## The headline

The big figure answers one of two questions, and the `MAX` marker says which.
**Right-click** to switch; **click** to collapse the HUD to a tag.

| | Reads | Means |
| --- | --- | --- |
| `MAX FPS` (default) | `1 / FRAME`, capped by the display | The rate a full redraw of this window could sustain |
| `FPS` | Frames presented per second | The rate the window is actually drawing at |

They are different questions, and an application that draws on demand answers
them very differently: a window sitting idle draws twice a second and could
draw a hundred and twenty times a second, and only one of those numbers is a
performance problem.

### Why MAX is derived rather than counted

The obvious way to make a frame counter read "as fast as this UI can go" is to
keep asking for frames, the way an in-game counter does. That is not free here.
Marking any view dirty schedules a **window** draw, and GPUI re-renders every
view in that window outside an [`Entity::cached`] boundary — so each frame the
HUD asked for would be a full layout and paint of the application, and the CPU
row underneath would be reporting work the HUD itself was causing. On the story
gallery's Table page that was ~62% CPU with nobody touching the window.

The frame cost already answers the question. `FRAME` is what a full redraw
costs, so its reciprocal is the rate those redraws could sustain, and nothing
has to be drawn to find it. The HUD never requests a frame.

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
| `CPU` | This process, on the scale `top` and Activity Monitor use: 100 is one saturated core, so a process spread across a core and a half reads 140. |
| `MEM` | Resident set. |

`FRAME`, `P95` and `DROP` are graded against the budget set by
`frame_budget()`, which defaults to one 60Hz frame. Set it to `1/144s` on a
high refresh rate display, or the chart will grade healthy frames amber.

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

[`Entity::cached`]: https://docs.rs/gpui/latest/gpui/struct.Entity.html
