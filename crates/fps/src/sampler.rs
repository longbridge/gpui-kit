use std::{collections::VecDeque, time::Duration};

use gpui::{
    WindowId,
    profiler::{FrameEvent, FrameTiming, FrameTimingCollector},
};
use web_time::Instant;

/// Frames presented longer ago than this stop contributing to the FPS readout.
const FPS_WINDOW: Duration = Duration::from_secs(1);

/// Frames dropped on the floor before any of them count.
///
/// A window's first frames are its most expensive — shaders, the glyph atlas,
/// the icons, every cache still cold — and they are not what the application
/// costs to run. Measured, one of them is a hundred milliseconds against a
/// budget of sixteen, and a HUD that has seen eight frames reports it as a
/// twelfth of the window's work in amber. The reader has done nothing wrong
/// and there is nothing to fix, so the default reading has to be a healthy
/// one.
const WARMUP_FRAMES: u32 = 8;

/// One drawn frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FrameSample {
    /// How long `Window::draw` took for this frame.
    pub draw: Duration,
    /// How many invalidations were coalesced into this frame. A number well
    /// above one means the window was asked to redraw more often than it could.
    pub invalidations: u64,
}

/// Collects per-frame timings for a single window out of GPUI's global frame
/// trace.
///
/// GPUI records frame timings into a process-wide ring buffer, so the sampler
/// filters by window: without that, every other open window's frames would be
/// counted as this window's.
pub(crate) struct FrameSampler {
    collector: FrameTimingCollector,
    window_id: WindowId,
    samples: VecDeque<FrameSample>,
    /// When the frames still inside [`FPS_WINDOW`] were presented.
    ///
    /// The frame's own `present_end`, not the moment this sampler read the
    /// event. A HUD that draws only when the window does reads the trace in
    /// batches, and a batch stamped with the time it was read collapses every
    /// frame in it onto one instant -- the rate then depends on how often the
    /// HUD looked, not on how often the window presented.
    present_times: VecDeque<Instant>,
    warmup: u32,
    /// Whether the backlog has been discarded yet.
    ///
    /// The first read drains everything GPUI has recorded since the process
    /// started. For a HUD switched on later that is history it was not there
    /// for; for one up from the start it is the cold start. Neither is the
    /// steady state the rows below the headline are describing.
    drained_backlog: bool,
    capacity: usize,
}

impl FrameSampler {
    pub(crate) fn new(window_id: WindowId, capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            collector: FrameTimingCollector::new(),
            window_id,
            samples: VecDeque::with_capacity(capacity),
            present_times: VecDeque::new(),
            warmup: WARMUP_FRAMES,
            drained_backlog: false,
            capacity,
        }
    }

    /// Drains the frames drawn since the previous call. Call once per rendered
    /// frame.
    pub(crate) fn tick(&mut self) {
        let mut draws = Vec::new();
        let mut presents = Vec::new();
        for event in self.collector.collect_unseen() {
            match event {
                FrameEvent::Draw(timing) => draws.push(timing),
                FrameEvent::Present(timing) if timing.window_id == self.window_id => {
                    presents.push(timing.present_end);
                }
                FrameEvent::Present(_) => {}
            }
        }
        if !self.drained_backlog {
            self.drained_backlog = true;
            let backlog = draws
                .iter()
                .filter(|timing| timing.window_id == self.window_id)
                .count();
            self.warmup = self.warmup.saturating_add(backlog as u32);
        }
        self.ingest_draws(draws);
        self.ingest_presents(presents, Instant::now());
    }

    pub(crate) fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(1);
        while self.samples.len() > self.capacity {
            self.samples.pop_front();
        }
    }

    /// Frames presented per second, over the frames still inside
    /// [`FPS_WINDOW`].
    ///
    /// Presented, not drawn: it is the same count the platform's own overlay
    /// keeps -- Metal's HUD counts drawables handed to the display -- so the
    /// two agree on the figure. A drawn frame that was never presented did not
    /// reach the screen and is not a frame the reader saw.
    ///
    /// `n` frames span `n - 1` intervals, so the rate is derived from the
    /// elapsed span rather than from the raw count; that keeps the readout
    /// correct before the rolling window has filled up.
    pub(crate) fn fps(&self) -> f32 {
        if self.present_times.len() < 2 {
            return 0.;
        }
        let (Some(oldest), Some(newest)) = (self.present_times.front(), self.present_times.back())
        else {
            return 0.;
        };
        let span = newest.duration_since(*oldest).as_secs_f32();
        if span <= 0. {
            return 0.;
        }
        (self.present_times.len() - 1) as f32 / span
    }

    /// Mean time between consecutive presents inside [`FPS_WINDOW`], as the
    /// platform's overlay reports its frame interval. The reciprocal of
    /// [`fps`](Self::fps); zero when there is no rate.
    pub(crate) fn present_interval(&self) -> Duration {
        let fps = self.fps();
        if fps <= 0. {
            return Duration::ZERO;
        }
        Duration::from_secs_f32(1. / fps)
    }

    pub(crate) fn samples(&self) -> impl ExactSizeIterator<Item = &FrameSample> {
        self.samples.iter()
    }

    pub(crate) fn capacity(&self) -> usize {
        self.capacity
    }

    /// Share of the retained frames that overran `budget`, in `0..1`.
    pub(crate) fn over_budget_ratio(&self, budget: Duration) -> f32 {
        if self.samples.is_empty() {
            return 0.;
        }
        let over = self
            .samples
            .iter()
            .filter(|sample| sample.draw > budget)
            .count();
        over as f32 / self.samples.len() as f32
    }

    /// Mean draw time across the retained frames.
    pub(crate) fn mean_draw(&self) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }
        let total: Duration = self.samples.iter().map(|sample| sample.draw).sum();
        total / self.samples.len() as u32
    }

    /// The draw time `percentile` of the retained frames came in at or under,
    /// as in `0.95` for the 95th.
    ///
    /// The mean beside it says what a typical frame costs; this says what the
    /// slow tail costs, and the tail is what a stutter *is*. The two separate
    /// exactly where it matters: a run of quick frames pulls the mean down over
    /// a spike, so a HUD reporting only the mean reads comfortable through jank
    /// the user can see. It is the same reason a benchmark quotes a 1% low
    /// beside its average frame rate.
    ///
    /// The rank is the nearest one rather than an interpolation between two
    /// frames, so every value the HUD shows is a frame that was really drawn.
    pub(crate) fn percentile_draw(&self, percentile: f32) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }
        let mut draws: Vec<Duration> = self.samples.iter().map(|sample| sample.draw).collect();
        draws.sort_unstable();

        let last = draws.len() - 1;
        let rank = (percentile.clamp(0., 1.) * last as f32).round() as usize;
        draws[rank.min(last)]
    }

    /// Mean number of invalidations coalesced into one retained frame.
    ///
    /// One means every redraw the window was asked for became a frame. Well
    /// above one means it was asked far more often than it could answer, which
    /// is work being thrown away — and unlike a slow frame it does not show up
    /// in the draw times at all, since each frame that *does* get drawn may be
    /// perfectly quick.
    pub(crate) fn mean_invalidations(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.;
        }
        let total: u64 = self.samples.iter().map(|sample| sample.invalidations).sum();
        total as f32 / self.samples.len() as f32
    }

    /// The slowest retained frame, used to scale the chart's y axis.
    pub(crate) fn peak_draw(&self) -> Duration {
        self.samples
            .iter()
            .map(|sample| sample.draw)
            .max()
            .unwrap_or_default()
    }

    /// Retains the cost of each drawn frame, newest last.
    fn ingest_draws(&mut self, timings: Vec<FrameTiming>) {
        for timing in timings {
            if timing.window_id != self.window_id {
                continue;
            }

            if self.warmup > 0 {
                self.warmup -= 1;
                continue;
            }

            if self.samples.len() == self.capacity {
                self.samples.pop_front();
            }
            self.samples.push_back(FrameSample {
                draw: timing.draw_duration(),
                invalidations: timing.invalidations,
            });
        }
    }

    /// Records when frames were presented and forgets the ones that have aged
    /// out of [`FPS_WINDOW`] as of `now`. `presented` must be in order.
    fn ingest_presents(&mut self, presented: impl IntoIterator<Item = Instant>, now: Instant) {
        self.present_times.extend(presented);

        while let Some(oldest) = self.present_times.front() {
            if now.duration_since(*oldest) > FPS_WINDOW {
                self.present_times.pop_front();
            } else {
                break;
            }
        }
    }
}

/// A sample of the resource usage shown beside the frame numbers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ResourceSample {
    /// CPU used by this process, on the scale `top`, Activity Monitor and
    /// Task Manager's per-process column all use: 100 is one saturated logical
    /// core, so a process spread across a core and a half reads 140.
    ///
    /// Deliberately not divided by the core count. Normalizing so that 100 is
    /// the whole machine makes the reading depend on hardware that has nothing
    /// to do with the application — the same work reads 12% on a four core
    /// laptop and 2% on a twenty-four core desktop — and it compresses every
    /// interesting value into the bottom of the range, where a UI thread
    /// pinning a core reads 4% and looks idle.
    pub cpu_percent: f32,
    /// Memory this process is responsible for, in bytes: private memory rather
    /// than the resident set, which is mostly shared library pages. See
    /// [`crate::memory`].
    pub memory_bytes: u64,
    /// The share of the GPU this process is using, and `None` on a platform
    /// that does not attribute GPU time per process. See [`crate::gpu`].
    pub gpu_percent: Option<f32>,
}

/// Averages the resource readings taken inside a trailing window.
///
/// Each of the three is a coarse sample of something that moves between one
/// sample and the next: CPU is the share of a single interval, GPU the share of
/// another, memory a snapshot of an allocator that grows and releases in steps.
/// Published raw at the sampling cadence they jump by tens of percent between
/// readings that describe the same steady workload, and the eye tracks the
/// churn instead of the value. Averaging over a window a few samples long keeps
/// the readings legible without hiding a real change for longer than the window.
#[cfg(not(target_family = "wasm"))]
#[derive(Debug)]
pub(crate) struct ResourceHistory {
    samples: VecDeque<(Instant, ResourceSample)>,
    window: Duration,
}

#[cfg(not(target_family = "wasm"))]
impl ResourceHistory {
    pub(crate) fn new(window: Duration) -> Self {
        Self {
            samples: VecDeque::new(),
            window,
        }
    }

    pub(crate) fn push(&mut self, sample: ResourceSample, now: Instant) {
        self.samples.push_back((now, sample));
        while let Some((at, _)) = self.samples.front() {
            if now.duration_since(*at) > self.window {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }

    /// The mean of the retained readings, or `None` before the first one has
    /// landed.
    ///
    /// The GPU share is averaged over the readings that carried one rather than
    /// over all of them: a momentary gap in a counter that is otherwise being
    /// published would otherwise read as a dip towards zero.
    pub(crate) fn mean(&self) -> Option<ResourceSample> {
        if self.samples.is_empty() {
            return None;
        }

        let count = self.samples.len() as f32;
        let cpu_percent = self
            .samples
            .iter()
            .map(|(_, sample)| sample.cpu_percent)
            .sum::<f32>()
            / count;
        // Summed wide: the byte counts are large and the window is only bounded
        // by time, so a fast sampling cadence must not be able to overflow it.
        let memory_bytes = self
            .samples
            .iter()
            .map(|(_, sample)| u128::from(sample.memory_bytes))
            .sum::<u128>()
            / self.samples.len() as u128;

        let (gpu_total, readings) = self
            .samples
            .iter()
            .filter_map(|(_, sample)| sample.gpu_percent)
            .fold((0., 0u32), |(total, count), percent| {
                (total + percent, count + 1)
            });

        Some(ResourceSample {
            cpu_percent,
            memory_bytes: memory_bytes as u64,
            gpu_percent: (readings > 0).then(|| gpu_total / readings as f32),
        })
    }
}

/// Samples this process' CPU, memory and GPU usage.
///
/// Refreshing is a blocking syscall walk, so this must be driven from a
/// background thread rather than from the render loop.
#[cfg(not(target_family = "wasm"))]
pub(crate) struct ResourceProbe {
    system: sysinfo::System,
    pid: sysinfo::Pid,
    /// `None` when this platform publishes no per-process GPU counter, which
    /// the HUD shows by leaving the reading out rather than at a flat zero.
    gpu: Option<crate::gpu::GpuProbe>,
    /// `None` when this platform publishes no private-memory counter, and the
    /// reading falls back to the resident set `sysinfo` reports.
    memory: Option<crate::memory::MemoryProbe>,
    history: ResourceHistory,
}

#[cfg(not(target_family = "wasm"))]
impl ResourceProbe {
    /// Returns `None` when the current process id cannot be determined, which
    /// is the only way sampling can be unavailable on a supported platform.
    pub(crate) fn new(window: Duration) -> Option<Self> {
        let pid = sysinfo::get_current_pid().ok()?;

        let mut probe = Self {
            system: sysinfo::System::new(),
            pid,
            gpu: crate::gpu::GpuProbe::new(),
            memory: crate::memory::MemoryProbe::new(),
            history: ResourceHistory::new(window),
        };
        // The first refresh only establishes the baseline; `cpu_usage` is a
        // delta against the previous refresh and reads zero until then.
        probe.refresh();
        Some(probe)
    }

    /// Takes a reading and returns the mean over the window, so the HUD is
    /// handed a number that is already smoothed. Averaging belongs on this side
    /// of the thread boundary: the probe is what knows the cadence the readings
    /// arrive at, and the render thread should not be doing arithmetic over a
    /// history it would otherwise have to keep.
    pub(crate) fn sample(&mut self) -> Option<ResourceSample> {
        let reading = self.read()?;
        self.history.push(reading, Instant::now());
        self.history.mean()
    }

    fn read(&mut self) -> Option<ResourceSample> {
        self.refresh();
        // Both are sampled before the process is borrowed out of `self.system`,
        // so the borrows of `self` do not overlap.
        let gpu_percent = self.gpu.as_mut().and_then(crate::gpu::GpuProbe::sample);
        let private = self
            .memory
            .as_mut()
            .and_then(crate::memory::MemoryProbe::sample);

        let process = self.system.process(self.pid)?;
        Some(ResourceSample {
            // Left on `sysinfo`'s own scale, which is 100 per saturated core.
            cpu_percent: process.cpu_usage(),
            memory_bytes: private.unwrap_or_else(|| process.memory()),
            gpu_percent,
        })
    }

    fn refresh(&mut self) {
        self.system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[self.pid]),
            false,
            sysinfo::ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory(),
        );
    }
}

/// The shortest interval at which CPU usage can be meaningfully resampled.
#[cfg(not(target_family = "wasm"))]
pub(crate) fn minimum_resource_interval() -> Duration {
    sysinfo::MINIMUM_CPU_UPDATE_INTERVAL
}

#[cfg(target_family = "wasm")]
pub(crate) fn minimum_resource_interval() -> Duration {
    Duration::from_millis(200)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sampler past its warm-up, which is where every statistic below is
    /// measured from. The warm-up itself is covered by its own test.
    fn warmed_sampler(window_id: WindowId, capacity: usize) -> FrameSampler {
        let mut sampler = FrameSampler::new(window_id, capacity);
        sampler.warmup = 0;
        sampler.drained_backlog = true;
        sampler
    }

    // GPUI stamps frames with `scheduler::Instant`, which re-exports
    // `std::time::Instant` off the web, so tests can build one without pulling
    // in the `scheduler` crate.
    fn timing(window_id: WindowId, draw: Duration) -> FrameTiming {
        coalesced(window_id, draw, 1)
    }

    /// A frame that answered `invalidations` requests to redraw at once.
    fn coalesced(window_id: WindowId, draw: Duration, invalidations: u64) -> FrameTiming {
        let start = std::time::Instant::now();
        FrameTiming {
            window_id,
            dirty_at: None,
            invalidations,
            draw_start: start,
            draw_end: start + draw,
        }
    }

    /// A sampler holding one frame per entry in `draws`, in milliseconds.
    fn sampler_of(draws: &[u64]) -> FrameSampler {
        let window_id = WindowId::from(1);
        let mut sampler = warmed_sampler(window_id, 256);
        for millis in draws {
            sampler.ingest_draws(vec![timing(window_id, Duration::from_millis(*millis))]);
        }
        sampler
    }

    #[test]
    fn ignores_frames_from_other_windows() {
        let ours = WindowId::from(1);
        let theirs = WindowId::from(2);
        let mut sampler = warmed_sampler(ours, 8);

        sampler.ingest_draws(vec![
            timing(ours, Duration::from_millis(8)),
            timing(theirs, Duration::from_millis(40)),
            timing(ours, Duration::from_millis(9)),
        ]);

        assert_eq!(sampler.samples().len(), 2);
        assert_eq!(sampler.peak_draw(), Duration::from_millis(9));
    }

    #[test]
    fn drops_oldest_samples_beyond_capacity() {
        let window_id = WindowId::from(1);
        let mut sampler = warmed_sampler(window_id, 2);

        for millis in [5, 6, 7] {
            sampler.ingest_draws(vec![timing(window_id, Duration::from_millis(millis))]);
        }

        let draws: Vec<_> = sampler.samples().map(|sample| sample.draw).collect();
        assert_eq!(
            draws,
            vec![Duration::from_millis(6), Duration::from_millis(7)]
        );
    }

    /// Feeds `count` presents spaced `interval` apart and returns the resulting
    /// rate.
    fn measure_fps(count: u64, interval: Duration) -> f32 {
        let window_id = WindowId::from(1);
        let mut sampler = warmed_sampler(window_id, 256);
        let start = Instant::now();

        for frame in 0..count {
            let presented = start + interval * frame as u32;
            sampler.ingest_presents([presented], presented);
        }
        sampler.fps()
    }

    #[test]
    fn the_cold_start_never_reaches_the_readings() {
        let window_id = WindowId::from(1);
        let mut sampler = FrameSampler::new(window_id, 256);
        let budget = Duration::from_micros(16_667);

        // What a window costs before any cache is warm, followed by what it
        // costs to run. Ingested straight, without the drain `tick` does, so
        // only the warm-up itself is under test.
        sampler.ingest_draws(vec![timing(window_id, Duration::from_millis(100))]);
        for _ in 1..WARMUP_FRAMES {
            sampler.ingest_draws(vec![timing(window_id, Duration::from_millis(40))]);
        }
        for _ in 0..20 {
            sampler.ingest_draws(vec![timing(window_id, Duration::from_millis(5))]);
        }

        assert_eq!(sampler.mean_draw(), Duration::from_millis(5));
        assert_eq!(sampler.percentile_draw(0.95), Duration::from_millis(5));
        assert_eq!(
            sampler.over_budget_ratio(budget),
            0.,
            "a window that opened is not a window that is dropping frames"
        );
    }

    #[test]
    fn fps_is_taken_from_when_frames_were_presented_not_when_they_were_read() {
        let window_id = WindowId::from(1);
        let mut sampler = warmed_sampler(window_id, 256);
        let start = Instant::now();
        let interval = Duration::from_millis(10);

        // A whole batch of presents read at once, long after the first of them:
        // what a HUD that draws only when the window does sees. Stamped with
        // the time they were read, 61 frames would collapse onto one instant
        // and report no rate at all; stamped with their own times they cover
        // 600ms => 100 fps.
        let presents: Vec<Instant> = (0..61).map(|frame| start + interval * frame).collect();
        let read_at = start + Duration::from_millis(600);
        sampler.ingest_presents(presents, read_at);
        assert!((sampler.fps() - 100.).abs() < 0.5, "{}", sampler.fps());
        assert!(
            (sampler.present_interval().as_secs_f32() * 1000. - 10.).abs() < 0.1,
            "{:?}",
            sampler.present_interval()
        );

        // Read again a second later with nothing new: everything has aged
        // out of the window, and the honest rate is zero.
        sampler.ingest_presents([], read_at + Duration::from_millis(1_100));
        assert_eq!(sampler.fps(), 0.);
        assert_eq!(sampler.present_interval(), Duration::ZERO);
    }

    #[test]
    fn fps_is_frames_divided_by_the_span_they_cover() {
        // The rate is `(n - 1) / span`, not `n / span`: n frames delimit n - 1
        // intervals. Counting the frames instead would over-report by
        // `1 / span`, which is a whole frame per second at these rates.
        //
        // 11 frames spaced 10ms apart cover 100ms => 10 intervals => 100 fps.
        assert!((measure_fps(11, Duration::from_millis(10)) - 100.).abs() < 0.5);

        // The same span sampled more finely reports the same rate.
        assert!((measure_fps(101, Duration::from_millis(1)) - 1000.).abs() < 5.);
    }

    #[test]
    fn fps_matches_the_common_refresh_rates() {
        for (interval_micros, expected) in [
            (16_667, 60.), // 60Hz
            (8_333, 120.), // 120Hz
            (33_333, 30.), // 30Hz
            (6_944, 144.), // 144Hz
        ] {
            let interval = Duration::from_micros(interval_micros);
            // A full second of frames at that interval.
            let count = 1_000_000 / interval_micros;
            let measured = measure_fps(count, interval);
            assert!(
                (measured - expected).abs() < 1.,
                "{interval_micros}us frames measured {measured}, expected {expected}"
            );
        }
    }

    #[test]
    fn fps_needs_two_frames_to_have_a_rate_at_all() {
        // A single frame delimits no interval, so there is nothing to divide by
        // and the honest answer is zero rather than a guess.
        assert_eq!(measure_fps(0, Duration::from_millis(10)), 0.);
        assert_eq!(measure_fps(1, Duration::from_millis(10)), 0.);
        assert!(measure_fps(2, Duration::from_millis(10)) > 0.);
    }

    /// The resource history only exists off the web, where there is a process
    /// to sample in the first place.
    #[cfg(not(target_family = "wasm"))]
    mod resource_history {
        use super::*;

        fn resources(
            cpu_percent: f32,
            memory_bytes: u64,
            gpu_percent: Option<f32>,
        ) -> ResourceSample {
            ResourceSample {
                cpu_percent,
                memory_bytes,
                gpu_percent,
            }
        }

        #[test]
        fn resource_readings_average_over_the_window() {
            let mut history = ResourceHistory::new(Duration::from_secs(3));
            let start = Instant::now();

            // Four readings a second apart, all of them still inside a three
            // second window: the oldest is exactly the window's age, and the
            // window is inclusive of it.
            for (second, cpu) in [(0, 400.), (1, 100.), (2, 200.), (3, 300.)] {
                history.push(
                    resources(cpu, 100 * 1024 * 1024, Some(cpu / 10.)),
                    start + Duration::from_secs(second),
                );
            }

            let mean = history.mean().expect("four readings have landed");
            assert!((mean.cpu_percent - 250.).abs() < 0.01);
            assert!((mean.gpu_percent.expect("every reading carried one") - 25.).abs() < 0.01);
            assert_eq!(mean.memory_bytes, 100 * 1024 * 1024);

            // A fifth a second later pushes the first out, so the reading that
            // was four times the others stops weighing on the mean.
            history.push(
                resources(100., 100 * 1024 * 1024, Some(10.)),
                start + Duration::from_secs(4),
            );
            let mean = history.mean().expect("the window still holds four");
            assert!((mean.cpu_percent - 175.).abs() < 0.01);
        }

        /// The averaged CPU stays on the single core scale rather than being folded
        /// back into `0..=100`: a process holding two cores busy reads 200 whether
        /// it is averaged or not.
        #[test]
        fn averaging_does_not_cap_the_cpu_reading() {
            let mut history = ResourceHistory::new(Duration::from_secs(3));
            let now = Instant::now();

            history.push(resources(150., 0, None), now);
            history.push(resources(250., 0, None), now);

            let mean = history.mean().expect("two readings have landed");
            assert!((mean.cpu_percent - 200.).abs() < 0.01);
        }

        #[test]
        fn a_gap_in_the_gpu_counter_does_not_read_as_a_dip() {
            let mut history = ResourceHistory::new(Duration::from_secs(3));
            let now = Instant::now();

            // The middle reading missed the counter. Averaging it in as a zero
            // would report 40%; the honest mean is over the two that carried one.
            history.push(resources(0., 0, Some(60.)), now);
            history.push(resources(0., 0, None), now);
            history.push(resources(0., 0, Some(60.)), now);

            let mean = history.mean().expect("three readings have landed");
            assert_eq!(mean.gpu_percent, Some(60.));
        }

        #[test]
        fn a_platform_with_no_gpu_counter_stays_without_one() {
            let mut history = ResourceHistory::new(Duration::from_secs(3));
            assert!(history.mean().is_none(), "nothing has been sampled yet");

            history.push(resources(12., 0, None), Instant::now());
            assert_eq!(
                history.mean().expect("one reading has landed").gpu_percent,
                None
            );
        }
    }

    #[test]
    fn simultaneous_frames_do_not_divide_by_zero() {
        let window_id = WindowId::from(1);
        let mut sampler = warmed_sampler(window_id, 64);
        let now = Instant::now();

        // Three presents on one instant -- what a trace with no clock behind
        // it would say -- span nothing, and nothing is divided by that.
        sampler.ingest_presents([now, now, now], now);

        assert_eq!(sampler.fps(), 0.);
    }

    #[test]
    fn the_percentile_is_the_frame_at_the_nearest_rank() {
        // Twenty frames, so the 95th percentile is rank 0.95 * 19 = 18.05,
        // which rounds to the second slowest.
        let mut draws: Vec<u64> = (1..=20).collect();
        draws.reverse();
        let sampler = sampler_of(&draws);

        assert_eq!(sampler.percentile_draw(0.95), Duration::from_millis(19));
        assert_eq!(sampler.percentile_draw(1.), Duration::from_millis(20));
        assert_eq!(sampler.percentile_draw(0.), Duration::from_millis(1));
    }

    #[test]
    fn the_percentile_separates_a_stutter_the_mean_absorbs() {
        // Eighteen quick frames and two that took twenty times as long: the
        // shape a stutter has. The mean stays inside a 60Hz budget while the
        // tail is well past it, which is the whole reason the row exists.
        let mut draws = vec![4; 18];
        draws.extend([80, 80]);
        let sampler = sampler_of(&draws);

        assert!(sampler.mean_draw() < Duration::from_millis(12));
        assert_eq!(sampler.percentile_draw(0.95), Duration::from_millis(80));
    }

    #[test]
    fn one_slow_frame_in_twenty_does_not_move_the_percentile() {
        // The complement of the test above, and the reason a percentile is
        // worth having over `peak_draw`: a single frame is the top 5% of
        // twenty, so it stays out of the 95th. The chart still shows it, and
        // the axis is still scaled to it — the row is for the tail that
        // *persists*, not for every outlier.
        let mut draws = vec![4; 19];
        draws.push(80);
        let sampler = sampler_of(&draws);

        assert_eq!(sampler.percentile_draw(0.95), Duration::from_millis(4));
        assert_eq!(sampler.peak_draw(), Duration::from_millis(80));
    }

    #[test]
    fn an_empty_sampler_has_no_percentile_rather_than_a_guess() {
        let sampler = warmed_sampler(WindowId::from(1), 8);
        assert_eq!(sampler.percentile_draw(0.95), Duration::ZERO);
        assert_eq!(sampler.mean_invalidations(), 0.);
    }

    #[test]
    fn invalidations_average_over_the_retained_frames() {
        let window_id = WindowId::from(1);
        let mut sampler = warmed_sampler(window_id, 8);

        // A window asked to redraw five times for every three frames it drew.
        for invalidations in [1, 3, 1] {
            sampler.ingest_draws(vec![coalesced(
                window_id,
                Duration::from_millis(4),
                invalidations,
            )]);
        }

        assert!((sampler.mean_invalidations() - 5. / 3.).abs() < f32::EPSILON);
    }

    #[test]
    fn frames_outside_the_rolling_window_stop_counting() {
        let window_id = WindowId::from(1);
        let mut sampler = warmed_sampler(window_id, 64);
        let start = Instant::now();

        for frame in 0..10 {
            let presented = start + Duration::from_millis(frame * 10);
            sampler.ingest_draws(vec![timing(window_id, Duration::from_millis(4))]);
            sampler.ingest_presents([presented], presented);
        }
        assert!(sampler.fps() > 0.);

        // Two seconds later the window has gone idle: every retained frame is
        // now older than the rolling window, so the rate collapses to zero.
        sampler.ingest_presents([], start + Duration::from_secs(2));
        assert_eq!(sampler.fps(), 0.);
        // The chart history survives so the last known shape stays on screen.
        assert_eq!(sampler.samples().len(), 10);
    }
}
