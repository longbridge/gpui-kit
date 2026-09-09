//! What the runtime is actually spending, in numbers a test can assert on.
//!
//! The central claim of this runtime is that script cost follows application
//! activity rather than frame rate (`snapshot.rs`). A claim that cannot be
//! observed cannot be regression-tested, so it is a counter rather than a
//! comment: `tests/snapshot.rs` renders a clean view repeatedly and asserts that
//! [`RuntimeMetrics::script_renders`] has not moved, and the shell story shows
//! both counters live while a feed drives the view.
//!
//! Two counters, and the gap between them is the whole point. The window root
//! caches a clean view's GPUI subtree, so neither counter follows frames that
//! have no dirty script view:
//!
//! ```text
//! script_renders    ── follows cx.notify(), reloads, theme changes
//! materializations  ── follows dirty script-view renders
//! ```
//!
//! # What a `VirtualList` does to the two
//!
//! A virtualized list is the one component that enters the VM from inside a
//! frame: GPUI decides which rows exist while it is laying the list out, so the
//! item renderer is called from layout rather than from a script render. That
//! changes how these counters read, and the change is deliberate rather than
//! incidental:
//!
//! * **`script_renders` does not move.** It counts entries into the script's
//!   `render` — snapshot builds — and an item renderer is not one. The claim it
//!   backs, that script cost follows application activity rather than frame
//!   rate, is still exactly what it measures.
//! * **`materialize_time` does move, and now includes VM time.** Describing a
//!   window of rows and turning it into elements are timed together and added
//!   here, because both are spent on the frame's budget, which is the question
//!   this total answers. `materializations` deliberately does not move with
//!   them: a frame with a list in it materializes one snapshot and renders two
//!   or more windows of rows.
//!
//! So on a view containing a virtual list, `mean_materialize` is no longer pure
//! Rust, and `script_render_time` is no longer all of the script's cost. Both
//! remain the right number for the question each asks.
//!
//! Timing uses `instant`, which is `std::time::Instant` everywhere except wasm,
//! where `std::time::Instant::now` panics outright.

use std::{cell::Cell, time::Duration};

/// A reading of the runtime's counters.
///
/// Values are a snapshot taken at the moment
/// [`ShellRuntime::read_metrics`](crate::ShellRuntime::read_metrics) was called;
/// nothing here keeps updating behind the caller's back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimeMetrics {
    script_renders: u64,
    script_render_time: Duration,
    slowest_script_render: Duration,
    native_time: Duration,
    frame_script_calls: u64,
    materializations: u64,
    materialize_time: Duration,
    structure_repeats: u64,
    structure_changes: u64,
}

impl RuntimeMetrics {
    /// How many times script `render` has been entered.
    pub fn script_renders(&self) -> u64 {
        self.script_renders
    }

    /// Total time spent inside script `render`.
    ///
    /// The whole pass, which is more than JavaScript: every builder call
    /// crossing into Rust, every `SpecOp` recorded, and every HostModule call
    /// the script makes while describing itself. [`native_time`] is how much of
    /// it was the last of those.
    ///
    /// [`native_time`]: Self::native_time
    pub fn script_render_time(&self) -> Duration {
        self.script_render_time
    }

    /// The slowest single script render in this reading.
    ///
    /// Reported next to the mean because the two disagree in a way worth
    /// seeing. A mean that drifts with system load is wall-clock contention —
    /// the render did not get slower, it got interrupted. A mean near the floor
    /// with a much larger maximum is a collection, or a first render paying for
    /// something the rest do not.
    pub fn slowest_script_render(&self) -> Duration {
        self.slowest_script_render
    }

    /// Of [`script_render_time`], how much was spent inside host functions the
    /// script called — a `quotes()` imported from a HostModule, and the like.
    ///
    /// Subtracting it leaves the part that is genuinely the script describing
    /// itself: JavaScript, the boundary crossings, and the arena.
    ///
    /// [`script_render_time`]: Self::script_render_time
    pub fn native_time(&self) -> Duration {
        self.native_time
    }

    /// Calls into JavaScript made from GPUI's frame path.
    pub fn frame_script_calls(&self) -> u64 {
        self.frame_script_calls
    }

    /// What one script render costs without the host calls inside it.
    pub fn mean_script_only(&self) -> Duration {
        mean(
            self.script_render_time.saturating_sub(self.native_time),
            self.script_renders,
        )
    }

    /// What one script render spends in host functions.
    pub fn mean_native(&self) -> Duration {
        mean(self.native_time, self.script_renders)
    }

    /// How many times a snapshot has been turned into GPUI elements. A cached,
    /// clean window frame does not move this counter.
    pub fn materializations(&self) -> u64 {
        self.materializations
    }

    /// Total time spent materializing, which is the part of the runtime that
    /// belongs to the frame budget.
    pub fn materialize_time(&self) -> Duration {
        self.materialize_time
    }

    pub fn mean_script_render(&self) -> Duration {
        mean(self.script_render_time, self.script_renders)
    }

    pub fn mean_materialize(&self) -> Duration {
        mean(self.materialize_time, self.materializations)
    }

    /// Rebuilds that produced the same *shape* as the description they
    /// replaced — the same components, the same builder methods, the same tree
    /// — differing only in the values inside it.
    ///
    /// This is the measurement a template cache rests on, and it is reported
    /// rather than acted on: nothing in the runtime skips work when a shape
    /// repeats. §20.7 of `docs/gpui-shell.md` explains why the number has to
    /// come first, and what it does and does not license.
    ///
    /// A view's first build has no predecessor and is counted in neither this
    /// nor [`structure_changes`], so the two sum to the rebuilds that had one
    /// rather than to [`script_renders`].
    ///
    /// [`structure_changes`]: Self::structure_changes
    /// [`script_renders`]: Self::script_renders
    pub fn structure_repeats(&self) -> u64 {
        self.structure_repeats
    }

    /// Rebuilds whose shape differed from the description they replaced: a
    /// branch taken differently, a row added, a style method that was not
    /// called last time.
    pub fn structure_changes(&self) -> u64 {
        self.structure_changes
    }

    /// What fraction of rebuilds with a predecessor repeated its shape, in the
    /// range `0.0..=1.0`, or `None` when no rebuild has had one yet.
    ///
    /// The ceiling on what a template cache could reach, not a prediction of
    /// what it would save: §20.7's third problem is that a repeated shape still
    /// has to mint this render's handlers.
    pub fn structure_repeat_rate(&self) -> Option<f64> {
        let compared = self.structure_repeats + self.structure_changes;
        (compared > 0).then(|| self.structure_repeats as f64 / compared as f64)
    }

    /// What this reading gained over an earlier one.
    ///
    /// Rates are what a live readout wants — "script renders in the last
    /// second" says something "script renders since start-up" does not — and a
    /// difference of two readings is the honest way to get one without the
    /// runtime having to know what a second is.
    pub fn since(&self, earlier: &Self) -> Self {
        Self {
            script_renders: self.script_renders.saturating_sub(earlier.script_renders),
            script_render_time: self
                .script_render_time
                .saturating_sub(earlier.script_render_time),
            // A maximum cannot be differenced. Reporting the run's worst is the
            // honest answer for a reading that covers part of it.
            slowest_script_render: self.slowest_script_render,
            native_time: self.native_time.saturating_sub(earlier.native_time),
            frame_script_calls: self
                .frame_script_calls
                .saturating_sub(earlier.frame_script_calls),
            materializations: self
                .materializations
                .saturating_sub(earlier.materializations),
            materialize_time: self
                .materialize_time
                .saturating_sub(earlier.materialize_time),
            structure_repeats: self
                .structure_repeats
                .saturating_sub(earlier.structure_repeats),
            structure_changes: self
                .structure_changes
                .saturating_sub(earlier.structure_changes),
        }
    }
}

fn mean(total: Duration, count: u64) -> Duration {
    match u32::try_from(count) {
        Ok(0) | Err(_) => Duration::ZERO,
        Ok(count) => total / count,
    }
}

/// The live counters, owned by the runtime.
///
/// `Cell` rather than an atomic because the VM and GPUI's `App` are both
/// main-thread only, and rather than a `RefCell` because a counter that could
/// panic on a re-entrant borrow would be a poor thing to put on the render path.
#[derive(Default)]
pub(crate) struct Metrics {
    script_renders: Cell<u64>,
    script_render_nanos: Cell<u64>,
    slowest_script_render_nanos: Cell<u64>,
    native_nanos: Cell<u64>,
    frame_script_calls: Cell<u64>,
    materializations: Cell<u64>,
    materialize_nanos: Cell<u64>,
    structure_repeats: Cell<u64>,
    structure_changes: Cell<u64>,
}

impl Metrics {
    /// Times `build`, which is one entry into script `render`.
    pub fn time_script_render<R>(&self, build: impl FnOnce() -> R) -> R {
        let started = instant::Instant::now();
        let result = build();
        let elapsed = elapsed_nanos(started);

        self.script_renders.set(self.script_renders.get() + 1);
        self.script_render_nanos
            .set(self.script_render_nanos.get() + elapsed);
        self.slowest_script_render_nanos
            .set(self.slowest_script_render_nanos.get().max(elapsed));
        result
    }

    /// Times one host function called from script.
    ///
    /// Nested inside [`time_script_render`] when it happens during a render,
    /// which is the usual case: this is the part of a render that is the host
    /// answering rather than the script describing.
    ///
    /// [`time_script_render`]: Self::time_script_render
    pub fn time_native<R>(&self, call: impl FnOnce() -> R) -> R {
        let started = instant::Instant::now();
        let result = call();
        self.native_nanos
            .set(self.native_nanos.get() + elapsed_nanos(started));
        result
    }

    /// Times one script call GPUI makes from inside a frame: a window of a
    /// virtualized list's items, or one piece of a dock's chrome — the script
    /// call that describes it and the walk that turns it into elements.
    ///
    /// Added to the materialize total without moving the materialize count.
    /// The count is materializations *of a snapshot*, and these are not — they
    /// happen several times inside a single frame, from inside GPUI's layout
    /// pass rather than from `materialize`. The time belongs there all the
    /// same: it is spent on the frame's budget, which is the question that
    /// total answers. See [`Self::time_materialize`] and the note in this
    /// module's comment.
    pub fn time_frame_script<R>(&self, build: impl FnOnce() -> R) -> R {
        let started = instant::Instant::now();
        let result = build();
        self.frame_script_calls
            .set(self.frame_script_calls.get() + 1);
        self.materialize_nanos
            .set(self.materialize_nanos.get() + elapsed_nanos(started));
        result
    }

    /// Times `build`, which is one materialization of a snapshot.
    pub fn time_materialize<R>(&self, build: impl FnOnce() -> R) -> R {
        let started = instant::Instant::now();
        let result = build();
        self.materializations.set(self.materializations.get() + 1);
        self.materialize_nanos
            .set(self.materialize_nanos.get() + elapsed_nanos(started));
        result
    }

    /// Records that a rebuild either repeated the shape of the description it
    /// replaced or did not.
    ///
    /// Called only when there *was* a predecessor. A view's first build is not
    /// a data point about whether structure repeats.
    pub fn record_structure(&self, repeated: bool) {
        let counter = if repeated {
            &self.structure_repeats
        } else {
            &self.structure_changes
        };
        counter.set(counter.get() + 1);
    }

    pub fn read(&self) -> RuntimeMetrics {
        RuntimeMetrics {
            script_renders: self.script_renders.get(),
            script_render_time: Duration::from_nanos(self.script_render_nanos.get()),
            slowest_script_render: Duration::from_nanos(self.slowest_script_render_nanos.get()),
            native_time: Duration::from_nanos(self.native_nanos.get()),
            frame_script_calls: self.frame_script_calls.get(),
            materializations: self.materializations.get(),
            materialize_time: Duration::from_nanos(self.materialize_nanos.get()),
            structure_repeats: self.structure_repeats.get(),
            structure_changes: self.structure_changes.get(),
        }
    }
}

fn elapsed_nanos(started: instant::Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}
