use std::time::Duration;

use web_time::Instant;

use gpui::{
    App, Bounds, Context, DisplayId, Div, Hsla, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement, PathBuilder, Pixels, Point, Render, StatefulInteractiveElement as _, Styled,
    Window, canvas, div, point, prelude::FluentBuilder as _, px, relative,
};

use gpui::Task;

use crate::{
    FrameTraceGuard,
    refresh::display_refresh_rate,
    sampler::{FrameSampler, ResourceSample, minimum_resource_interval},
    style::FpsStyle,
};

/// One frame at 60Hz, the default budget a frame is judged against.
const DEFAULT_FRAME_BUDGET: Duration = Duration::from_nanos(16_666_667);
const DEFAULT_CAPACITY: usize = 120;
const DEFAULT_RESOURCE_INTERVAL: Duration = Duration::from_millis(500);

/// How far back CPU, memory and GPU are averaged over. At the default interval
/// that is six readings: long enough to settle the churn between one sample and
/// the next, short enough that a real change reaches the HUD while the reader
/// is still looking at what caused it.
#[cfg(not(target_family = "wasm"))]
const RESOURCE_WINDOW: Duration = Duration::from_secs(3);

/// Which frame the `P95` row reports. The 95th rather than the 99th: the chart
/// keeps 120 frames by default, so the 99th is the second slowest of them — one
/// frame, which moves the row on its own and reads as noise.
const FRAME_PERCENTILE: f32 = 0.95;

/// How fast the chart's y axis relaxes back down after a spike. Growth is
/// immediate so a slow frame is never clipped, while the decay is gradual so
/// the bars don't visibly rescale every frame.
const AXIS_DECAY: f32 = 0.04;

/// A fixed width keeps every row flush with the chart and stops the HUD from
/// resizing as the readings gain or lose digits. Collapsed, the HUD hugs its
/// text instead and only the figure gets a fixed box.
const HUD_WIDTH: Pixels = px(172.);
const COMPACT_FIGURE_WIDTH: Pixels = px(25.);

/// Size of every label and reading. Collapsed, the figure uses it too.
const TEXT_SIZE: Pixels = px(10.);

/// The trace sits behind the headline, so it is dimmed enough to stay out of
/// the figure's way while still showing its shape and color.
const TRACE_OPACITY: f32 = 0.35;

/// Tall enough to give the trace room to show its shape around the figure.
const HEADLINE_HEIGHT: Pixels = px(35.);

/// The headline figure. Its box has to fit four digits at [`FIGURE_SIZE`] —
/// a monospace digit runs about 0.6em, and an uncapped frame rate on a small
/// window reaches four figures — or the reading is clipped instead of merely
/// looking cramped.
const FIGURE_SIZE: Pixels = px(28.);
const FIGURE_WIDTH: Pixels = px(70.);

/// Width of the `FPS` unit, and of the empty box mirroring it on the other side
/// of the figure so the figure lands on the HUD's true center.
const UNIT_WIDTH: Pixels = px(28.);

/// How often the numbers are recomputed.
///
/// The trace keeps up with every frame, but the readings do not: recomputed
/// per frame they flicker through digits too fast to read, and the eye tracks
/// the churn rather than the value. Twice a second is slow enough to read and
/// fast enough to feel live.
const READOUT_INTERVAL: Duration = Duration::from_millis(500);

/// A monospace family that ships with the platform, so the value column stays
/// aligned without the application having to configure a font. The generic
/// `monospace` alias is not resolvable by every platform's font backend, hence
/// the concrete names.
#[cfg(target_os = "macos")]
const DEFAULT_FONT: &str = "Menlo";
#[cfg(target_os = "windows")]
const DEFAULT_FONT: &str = "Consolas";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const DEFAULT_FONT: &str = "monospace";

/// A realtime performance HUD: frames per second, a rolling frame time chart,
/// and this process' GPU, CPU and memory usage.
///
/// This is a view rather than a stateless component on purpose: driving
/// redraws goes through [`Window::request_animation_frame`], which notifies the
/// *current* view, and from inside a stateless component that would be whoever
/// rendered the HUD — dirtying the host's own state to move a frame counter.
///
/// The HUD never asks for a frame of its own. A dirty view schedules a *window*
/// draw and GPUI re-renders every view outside an [`Entity::cached`] boundary,
/// so a HUD that drove the frame loop to keep its counter moving would be
/// paying a full layout and paint per frame — and reporting that cost in the
/// resource row as if it were the application's. The headline is derived from
/// what a frame costs instead, which answers the same question for free and
/// leaves the readings measuring the application alone.
///
/// ```no_run
/// # use gpui::*;
/// # use gpui_fps::FpsMonitor;
/// # fn example(window: &mut Window, cx: &mut App) {
/// let monitor = cx.new(|cx| FpsMonitor::new(window, cx).capacity(240));
/// # }
/// ```
/// The numbers as last published to the screen.
#[derive(Clone, Copy, Default)]
struct Readout {
    /// The rate a full redraw of this window could sustain: the reciprocal of
    /// `frame_millis`.
    ///
    /// Derived rather than counted, because counting it would mean causing it.
    /// A frame rate measured from presents is only the rate the application
    /// happens to be drawing at, and the only way to make that number mean
    /// "as fast as this UI can go" is to keep the window drawing back to back
    /// — which costs a full layout and paint per frame and lands in the
    /// resource row right underneath. The frame cost answers the same question
    /// without being paid for.
    ///
    /// A ceiling the frame cost can prove, not one the display can show: a
    /// window whose frames cost 3ms could redraw 333 times a second, on a
    /// panel that would scan out sixty of them.
    max_fps: f32,
    /// Frames presented per second: the rate the window is actually drawing
    /// at, which an idle application drives to zero. The reciprocal of
    /// `interval_millis`.
    fps: f32,
    /// Mean time between presents, in milliseconds: the platform overlay's
    /// "frame interval".
    interval_millis: f32,
    /// Mean `Window::draw` cost of the retained frames, in milliseconds.
    frame_millis: f32,
    /// The slow tail of the same frames `frame_millis` is the mean of.
    percentile_millis: f32,
    dropped_percent: f32,
    /// Mean invalidations coalesced into one frame; one means none were wasted.
    invalidations: f32,
}

/// The rate a full redraw could sustain: what a frame's cost implies, held to
/// what the panel can scan out.
///
/// The cap is the half the derivation loses. Counting presents could never
/// exceed the refresh rate — frames go to the compositor on vsync, so the
/// bound came for free — while a frame drawn in 3ms reads as 333, a rate
/// nobody could ever see. `display` is `None` where the platform would not say
/// what the panel runs at, and an uncapped reading is better than one held to
/// a guess: see [`crate::refresh`] for why guessing was tried and abandoned.
fn sustainable_rate(mean_draw: Duration, display: Option<Duration>) -> f32 {
    let mean_draw = mean_draw.as_secs_f32();
    if mean_draw <= 0. {
        return 0.;
    }
    let rate = 1. / mean_draw;
    match display.map(|period| period.as_secs_f32()) {
        Some(period) if period > 0. => rate.min(1. / period),
        _ => rate,
    }
}

/// Which question the headline answers.
///
/// Both readings come out of the same samples, so switching is free — which is
/// the whole point. The rate a UI can hold and the rate it is holding are
/// different questions, and the only expensive way to answer the first is to
/// stop the second from being answerable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Headline {
    /// The rate a full redraw could sustain, from what one costs.
    Max,
    /// The rate the window is drawing at.
    Observed,
}

pub struct FpsMonitor {
    sampler: FrameSampler,
    readout: Readout,
    readout_at: Option<Instant>,
    style: FpsStyle,
    frame_budget: Duration,
    headline: Headline,
    /// The panel's refresh period, and which display it was asked about, so
    /// that moving the window to another monitor re-asks and staying on one
    /// does not ask again every frame.
    display: Option<(DisplayId, Option<Duration>)>,
    show_resources: bool,
    resource_interval: Duration,
    resources: Option<ResourceSample>,
    compact: bool,
    /// Upper bound of the chart's y axis, in seconds.
    axis_max: f32,
    clock: Option<Task<()>>,
    _frame_trace: FrameTraceGuard,
}

impl FpsMonitor {
    pub fn new(window: &Window, _cx: &mut Context<Self>) -> Self {
        let frame_budget = DEFAULT_FRAME_BUDGET;
        Self {
            sampler: FrameSampler::new(window.window_handle().window_id(), DEFAULT_CAPACITY),
            readout: Readout::default(),
            readout_at: None,
            style: FpsStyle::default(),
            frame_budget,
            headline: Headline::Max,
            display: None,
            show_resources: true,
            resource_interval: DEFAULT_RESOURCE_INTERVAL,
            resources: None,
            compact: false,
            axis_max: frame_budget.as_secs_f32() * 2.,
            clock: None,
            _frame_trace: FrameTraceGuard::acquire(),
        }
    }

    /// How many frames the chart keeps. Defaults to 120.
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.sampler.set_capacity(capacity);
        self
    }

    /// The per-frame budget used for the chart's baseline and bar colors.
    /// Defaults to one 60Hz frame; set it to `1/144s` on a high refresh rate
    /// display.
    pub fn frame_budget(mut self, budget: Duration) -> Self {
        self.frame_budget = budget;
        self.axis_max = budget.as_secs_f32() * 2.;
        self
    }

    pub(crate) fn set_frame_budget(&mut self, budget: Duration) {
        self.frame_budget = budget;
        self.axis_max = budget.as_secs_f32() * 2.;
    }

    /// Whether to sample and show CPU, memory and GPU usage. Defaults to
    /// `true`, and is always off on the web.
    ///
    /// The GPU reading is left out on its own where the platform publishes no
    /// counter for it, so turning this on does not guarantee three readings.
    pub fn show_resources(mut self, show_resources: bool) -> Self {
        self.show_resources = show_resources;
        self
    }

    /// How often CPU, memory and GPU are resampled. Defaults to 500ms, and is
    /// clamped up to the shortest interval that yields a meaningful CPU delta.
    pub fn resource_interval(mut self, interval: Duration) -> Self {
        self.resource_interval = interval;
        self
    }

    /// The clock that republishes the readings, started on the first render so
    /// that the builder methods have already been applied by the time its
    /// interval is read.
    ///
    /// Nothing else wakes the HUD. It does not drive the frame loop, and a
    /// window that has stopped drawing produces no renders to refresh it from,
    /// so without this the figures would freeze at whatever the application
    /// last drew — exactly when a frozen `137` is most likely to be read as
    /// the truth.
    #[cfg(not(target_family = "wasm"))]
    fn start_clock(&mut self, cx: &mut Context<Self>) {
        use crate::sampler::ResourceProbe;

        if self.clock.is_some() {
            return;
        }

        let show_resources = self.show_resources;
        let interval = if show_resources {
            self.resource_interval.max(minimum_resource_interval())
        } else {
            READOUT_INTERVAL
        };
        self.clock = Some(cx.spawn(async move |this, cx| {
            let executor = cx.background_executor().clone();
            // Probing walks the process table, so it never runs on the render
            // thread. The probe moves in and out of each background task rather
            // than living behind a lock. A platform that cannot provide one
            // still gets the clock; it just has no resource row to fill.
            let mut probe = if show_resources {
                executor
                    .spawn(async { ResourceProbe::new(RESOURCE_WINDOW) })
                    .await
            } else {
                None
            };

            loop {
                executor.timer(interval).await;

                let sample = match probe.take() {
                    Some(mut owned) => {
                        let (returned, sample) = executor
                            .spawn(async move {
                                let sample = owned.sample();
                                (owned, sample)
                            })
                            .await;
                        probe = Some(returned);
                        sample
                    }
                    None => None,
                };

                let alive = this.update(cx, |this, cx| {
                    if sample.is_some() {
                        this.resources = sample;
                    }
                    cx.notify();
                });
                if alive.is_err() {
                    break;
                }
            }
        }));
    }

    #[cfg(target_family = "wasm")]
    fn start_clock(&mut self, cx: &mut Context<Self>) {
        let _ = minimum_resource_interval();

        if self.clock.is_some() {
            return;
        }
        self.clock = Some(cx.spawn(async move |this, cx| {
            let executor = cx.background_executor().clone();
            loop {
                executor.timer(READOUT_INTERVAL).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        }));
    }

    /// Re-asks the platform for the refresh rate when the window has moved to
    /// another display, and not otherwise: the answer is a property of the
    /// panel, and on some platforms asking is a round trip.
    fn update_display(&mut self, window: &Window, cx: &App) {
        let Some(display) = window.display(cx) else {
            return;
        };
        let id = display.id();
        if self.display.map(|(asked, _)| asked) != Some(id) {
            self.display = Some((id, display_refresh_rate(display.as_ref())));
        }
    }

    /// Republishes the readings if [`READOUT_INTERVAL`] has passed.
    fn update_readout(&mut self) {
        let now = Instant::now();
        let due = self
            .readout_at
            .is_none_or(|at| now.duration_since(at) >= READOUT_INTERVAL);
        if !due {
            return;
        }

        self.readout = Readout {
            max_fps: sustainable_rate(
                self.sampler.mean_draw(),
                self.display.and_then(|(_, refresh_rate)| refresh_rate),
            ),
            fps: self.sampler.fps(),
            interval_millis: self.sampler.present_interval().as_secs_f32() * 1000.,
            // The mean over the interval rather than the latest frame, which
            // at this cadence would be an arbitrary sample.
            frame_millis: self.sampler.mean_draw().as_secs_f32() * 1000.,
            percentile_millis: self.sampler.percentile_draw(FRAME_PERCENTILE).as_secs_f32() * 1000.,
            dropped_percent: self.sampler.over_budget_ratio(self.frame_budget) * 100.,
            invalidations: self.sampler.mean_invalidations(),
        };
        self.readout_at = Some(now);
    }

    /// Grows immediately to fit the slowest retained frame and decays back
    /// slowly, so a single spike doesn't make the whole chart jump.
    fn update_axis(&mut self) {
        let floor = self.frame_budget.as_secs_f32() * 2.;
        let target = self.sampler.peak_draw().as_secs_f32().max(floor);
        self.axis_max = if target > self.axis_max {
            target
        } else {
            self.axis_max + (target - self.axis_max) * AXIS_DECAY
        };
    }

    /// The frame time trace, drawn behind the readings so it fills the HUD
    /// instead of taking a band of its own. It is dimmed to stay legible under
    /// the text.
    fn render_chart(&self) -> impl IntoElement {
        let style = self.style;
        let budget = self.frame_budget.as_secs_f32();
        let axis_max = self.axis_max.max(f32::EPSILON);
        let capacity = self.sampler.capacity();
        let samples: Vec<(f32, Hsla)> = self
            .sampler
            .samples()
            .map(|sample| {
                let seconds = sample.draw.as_secs_f32();
                (
                    (seconds / axis_max).clamp(0., 1.),
                    style.level_color(seconds, budget).opacity(TRACE_OPACITY),
                )
            })
            .collect();

        canvas(
            |_, _, _| (),
            move |bounds: Bounds<Pixels>, _, window, _| {
                let slot = bounds.size.width / capacity as f32;
                // Fewer samples than the capacity means the chart is still
                // filling up; keep the newest frame pinned to the right edge so
                // the history scrolls instead of stretching.
                let leading = capacity.saturating_sub(samples.len());
                let points: Vec<(Point<Pixels>, Hsla)> = samples
                    .iter()
                    .enumerate()
                    .map(|(index, (ratio, color))| {
                        (
                            point(
                                bounds.origin.x + slot * (leading + index) as f32 + slot / 2.,
                                bounds.origin.y + bounds.size.height * (1. - *ratio),
                            ),
                            *color,
                        )
                    })
                    .collect();

                // The line is drawn as runs of equal color rather than one
                // segment per frame: a single path can only carry one color,
                // and in the common case where nothing is dropped the whole
                // chart collapses into one path.
                let mut start = 0;
                while start + 1 < points.len() {
                    // A segment is as slow as the frame it ends on, so the
                    // color of the later point decides the run.
                    let color = points[start + 1].1;
                    let mut path = PathBuilder::stroke(px(1.));
                    path.move_to(points[start].0);

                    let mut end = start + 1;
                    while end < points.len() && points[end].1 == color {
                        path.line_to(points[end].0);
                        end += 1;
                    }

                    if let Ok(path) = path.build() {
                        window.paint_path(path, color);
                    }
                    // Share the boundary point with the next run so the line
                    // stays connected across a color change.
                    start = end - 1;
                }
            },
        )
        .absolute()
        .inset_0()
    }

    /// The headline reading, with the frame time trace painted behind it.
    ///
    /// The trace lives in this row rather than spanning the whole HUD because
    /// this is its emptiest part — the figure is centered and short, leaving
    /// both flanks open — so the trace stays readable instead of being cut up
    /// by the denser rows below.
    ///
    /// The figure is centered in a fixed box so neither the unit nor the group
    /// shifts as the count gains or loses a digit; the two share a bottom edge.
    fn render_headline(&self, rate: f32, color: Hsla) -> Div {
        let style = self.style;

        div()
            .relative()
            .overflow_hidden()
            .w_full()
            .h(HEADLINE_HEIGHT)
            .child(self.render_chart())
            .child(
                div()
                    .flex()
                    .size_full()
                    .items_end()
                    .justify_center()
                    .gap_1()
                    // The box that balances the unit on the right. Without it
                    // the unit's own width pushes the figure off center by half
                    // of it, which reads as misalignment — so the mode marker
                    // goes here, where it costs no layout and lands where it
                    // is read: immediately before the figure it qualifies.
                    .child(
                        div()
                            .w(UNIT_WIDTH)
                            .text_right()
                            .text_color(style.muted)
                            .when(self.headline == Headline::Max, |this| this.child("MAX")),
                    )
                    .child(
                        div()
                            .w(FIGURE_WIDTH)
                            .text_center()
                            .text_size(FIGURE_SIZE)
                            .line_height(relative(1.))
                            .text_color(color)
                            .child(format!("{rate:.0}")),
                    )
                    .child(div().w(UNIT_WIDTH).text_color(style.muted).child("FPS")),
            )
    }
}

impl Render for FpsMonitor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sampler.tick();
        self.update_display(window, cx);
        self.update_readout();
        self.update_axis();
        self.start_clock(cx);

        let style = self.style;
        let budget = self.frame_budget;
        let Readout {
            max_fps,
            fps,
            interval_millis,
            frame_millis,
            percentile_millis,
            dropped_percent: dropped,
            invalidations,
        } = self.readout;
        // Printed plain, never graded. It is the reciprocal of `FRAME`, which
        // is graded already, and grading the same measurement twice in two
        // units would just say the same thing louder.
        let fps_color = style.foreground;
        let resources = self.resources.filter(|_| self.show_resources);
        let compact = self.compact;
        let headline = self.headline;
        let rate = match headline {
            Headline::Max => max_fps,
            Headline::Observed => fps,
        };

        div()
            .id("gpui-fps-hud")
            .flex()
            .bg(style.background)
            .font_family(DEFAULT_FONT)
            .text_size(TEXT_SIZE)
            .text_color(style.muted)
            .on_click(cx.listener(|this, _, _, cx| {
                this.compact = !this.compact;
                cx.notify();
            }))
            // The `MAX` marker is what says which of the two the figure is.
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _, _, cx| {
                    this.headline = match this.headline {
                        Headline::Max => Headline::Observed,
                        Headline::Observed => Headline::Max,
                    };
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .map(|this| {
                if compact {
                    // Collapsed, the HUD is one small tag: the figure drops to
                    // the same size as its unit, the box shrinks to the text,
                    // and everything else is dropped, so it sits over the
                    // interface without competing with it.
                    this.items_center()
                        .gap_1()
                        .px_1p5()
                        .py_0p5()
                        .rounded(px(3.))
                        .when(headline == Headline::Max, |this| this.child("MAX"))
                        .child(
                            div()
                                .w(COMPACT_FIGURE_WIDTH)
                                .text_right()
                                .text_color(fps_color)
                                .child(format!("{rate:.0}")),
                        )
                        .child("FPS")
                } else {
                    this.flex_col()
                        .w(HUD_WIDTH)
                        .px_2()
                        .py_1p5()
                        .rounded(px(4.))
                        .child(self.render_headline(rate, fps_color))
                        .child(reading(
                            // The same figure the platform overlay calls its
                            // frame interval: time between presents. Where the
                            // headline says how fast this UI could go, this
                            // says how often it actually went — a wide gap
                            // between them is an idle window, not a slow one.
                            "INTERVAL",
                            format!("{interval_millis:.1} ms"),
                            style.foreground,
                            style,
                        ))
                        .child(reading(
                            "FRAME",
                            format!("{frame_millis:.1} ms"),
                            // Graded against the budget, and the first reading
                            // in the HUD that is: the rate above says how often
                            // frames happened, this says whether they were
                            // affordable. It is the one to read when something
                            // feels slow.
                            style.level_color(frame_millis / 1000., budget.as_secs_f32()),
                            style,
                        ))
                        .child(reading(
                            // Graded the same way, so the two millisecond rows
                            // read as one measurement seen twice: what a frame
                            // usually costs, and what its slow tail costs.
                            "P95",
                            format!("{percentile_millis:.1} ms"),
                            style.level_color(percentile_millis / 1000., budget.as_secs_f32()),
                            style,
                        ))
                        .child(
                            // Dropped frames and wasted invalidations share a
                            // row: both count redundant work rather than
                            // measuring a duration, so neither belongs in the
                            // millisecond column above.
                            row()
                                .child(pair(
                                    "DROP",
                                    format!("{dropped:.1}%"),
                                    style.level_color(if dropped > 0. { 1. } else { 0. }, 0.5),
                                    style,
                                ))
                                .child(pair(
                                    "INV",
                                    format!("{invalidations:.1}"),
                                    // Ungraded, unlike every other reading in
                                    // the HUD. One per frame is the ideal, but
                                    // it is not the floor here: in continuous
                                    // mode the monitor requests an animation
                                    // frame of its own on every render, so an
                                    // application invalidating once a frame
                                    // measures two and a healthy HUD would sit
                                    // permanently in the red. The baseline
                                    // depends on that switch and on how the
                                    // application drives its own redraws, which
                                    // is not something the HUD can grade — so
                                    // the number is reported and the reading is
                                    // left to whoever knows what to expect.
                                    style.foreground,
                                    style,
                                )),
                        )
                        .when_some(
                            resources.and_then(|resources| resources.gpu_percent),
                            |this, gpu| {
                                this.child(reading(
                                    "GPU",
                                    format!("{gpu:.1}%"),
                                    style.foreground,
                                    style,
                                ))
                            },
                        )
                        .when_some(resources, |this, resources| {
                            this.child(
                                // CPU and memory share a row: both are coarse
                                // background samples, unlike the per-frame
                                // numbers.
                                row()
                                    .child(pair(
                                        "CPU",
                                        format_cpu(resources.cpu_percent),
                                        style.foreground,
                                        style,
                                    ))
                                    .child(pair(
                                        "MEM",
                                        format_bytes(resources.memory_bytes),
                                        style.foreground,
                                        style,
                                    )),
                            )
                        })
                }
            })
    }
}

/// A row carrying two [`pair`]s, pushed to either inner edge.
fn row() -> Div {
    div().flex().w_full().justify_between().gap_2().py(px(1.))
}

/// A `LABEL value` pair kept together, for rows that carry more than one
/// reading. The label stays muted so it reads as a caption, not as data.
fn pair(label: &'static str, value: String, value_color: Hsla, style: FpsStyle) -> Div {
    div()
        .flex()
        .gap_1()
        .child(div().text_color(style.muted).child(label))
        .child(div().text_color(value_color).child(value))
}

/// One `LABEL … value` row. The value is right aligned against the HUD's inner
/// edge, so in a monospace font every row's digits line up in a column and
/// nothing shifts as the readings change width.
fn reading(label: &'static str, value: String, value_color: Hsla, style: FpsStyle) -> Div {
    div()
        .flex()
        .w_full()
        .justify_between()
        .gap_2()
        .py(px(1.))
        .child(div().text_color(style.muted).child(label))
        .child(div().text_color(value_color).child(value))
}

/// A CPU reading on the single core scale, which passes 100 as soon as the
/// process spreads over more than one core and reaches the core count times a
/// hundred when it saturates the machine.
///
/// A tenth is worth showing while the reading is small, where it is the
/// difference between idle and a busy timer; past ten the extra digit only
/// churns, and dropping it also keeps the reading inside the row's share of the
/// HUD on a machine with enough cores to reach four figures.
fn format_cpu(percent: f32) -> String {
    if percent < 10. {
        format!("{percent:.1}%")
    } else {
        format!("{percent:.0}%")
    }
}

fn format_bytes(bytes: u64) -> String {
    const MIB: f64 = 1024. * 1024.;
    const GIB: f64 = MIB * 1024.;

    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.2} GB", bytes / GIB)
    } else {
        format!("{:.0} MB", bytes / MIB)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;

    #[test]
    fn the_headline_rate_is_what_a_frame_costs_and_the_panel_allows() {
        let sixty = Duration::from_micros(16_667);
        // A cheap frame on a 60Hz panel is not 333 frames anyone could see.
        assert!((sustainable_rate(Duration::from_millis(3), Some(sixty)) - 60.).abs() < 0.01);
        // A frame that costs more than a refresh sets the rate itself.
        assert_eq!(
            sustainable_rate(Duration::from_millis(20), Some(sixty)),
            50.
        );
        // Where the platform will not say, an uncapped reading beats a guess.
        assert!((sustainable_rate(Duration::from_millis(3), None) - 333.33).abs() < 0.1);
        // No frames drawn yet is no rate, not an infinite one.
        assert_eq!(sustainable_rate(Duration::ZERO, Some(sixty)), 0.);
    }

    #[gpui::test]
    fn test_fps_monitor_builder(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let budget = Duration::from_micros(6_944);
            let monitor = cx.new(|cx| {
                FpsMonitor::new(window, cx)
                    .capacity(240)
                    .frame_budget(budget)
                    .show_resources(false)
                    .resource_interval(Duration::from_secs(2))
            });

            let monitor = monitor.read(cx);
            assert_eq!(monitor.sampler.capacity(), 240);
            assert_eq!(monitor.frame_budget, budget);
            assert!(!monitor.show_resources);
            assert_eq!(monitor.resource_interval, Duration::from_secs(2));
            // The axis floor tracks the budget so a 144Hz budget doesn't leave
            // the chart scaled for 60Hz frames.
            assert_eq!(monitor.axis_max, budget.as_secs_f32() * 2.);
        });
    }

    #[test]
    fn formats_memory_by_magnitude() {
        assert_eq!(format_bytes(184 * 1024 * 1024), "184 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.00 GB");
    }

    /// The reading is on the single core scale, so it passes 100 and keeps
    /// going — the row must show that rather than round it away or clip it.
    #[test]
    fn formats_cpu_on_the_single_core_scale() {
        // A process spread over a core and a half, which under a scale where
        // 100 is the whole machine would have read 5.8% on a 24 core desktop.
        assert_eq!(format_cpu(140.), "140%");
        // Saturating every core of a big machine still has somewhere to go.
        assert_eq!(format_cpu(2400.), "2400%");
        // Small readings keep the tenth that distinguishes them.
        assert_eq!(format_cpu(0.4), "0.4%");
        assert_eq!(format_cpu(9.9), "9.9%");
        assert_eq!(format_cpu(12.4), "12%");
    }
}
