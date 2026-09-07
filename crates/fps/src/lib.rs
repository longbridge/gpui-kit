//! A realtime performance HUD for GPUI applications: frames per second, a
//! rolling frame time chart, and this process' GPU, CPU and memory usage.
//!
//! Frame data comes from GPUI's own frame trace
//! ([`gpui::FrameTimingCollector`]). The interval counts frames *presented*,
//! stamped with their own present time, so it agrees with the platform's
//! overlay (Metal's HUD counts the same drawables); the frame cost is what the
//! framework actually spent in `Window::draw`, rather than an approximation
//! measured from the outside. The headline rate is derived from that cost —
//! the HUD never drives the frame loop, so nothing it reports is something it
//! caused.
//!
//! Render it wherever it should appear, guarded by your own flag:
//!
//! ```no_run
//! # use gpui::*;
//! # use gpui_fps::fps_monitor;
//! # struct Example { show_fps: bool }
//! # impl Render for Example {
//! fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
//!     div()
//!         .relative()
//!         .size_full()
//!         .child("your app")
//!         .when(self.show_fps, |this| this.child(fps_monitor(window, cx)))
//! }
//! # }
//! ```
//!
//! The returned overlay can change its corner and its frame budget. A custom palette or an
//! embedded rather than overlaid HUD is built by composing [`FpsMonitor`] and
//! [`FpsOverlay`] directly.
//!
//! This crate depends only on `gpui`, so it can be used from any GPUI
//! application.

#[cfg(not(target_family = "wasm"))]
mod gpu;
#[cfg(not(target_family = "wasm"))]
mod memory;
mod monitor;
mod overlay;
mod refresh;
mod sampler;
mod style;

pub use monitor::FpsMonitor;
pub use overlay::FpsOverlay;

use std::{collections::HashMap, sync::Mutex};

use gpui::{App, AppContext as _, Entity, Global, Window, WindowId};

/// The performance HUD, pinned to the top right of its parent.
///
/// The parent element must be `relative()`, since the HUD positions itself
/// absolutely. Call this at most once per window — a second call renders the
/// same monitor twice.
///
/// Whether the HUD is on screen is the caller's to decide; render this only
/// when it should be visible:
///
/// ```no_run
/// # use gpui::*;
/// # use gpui_fps::fps_monitor;
/// # struct Example { show_fps: bool }
/// # impl Render for Example {
/// fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
///     div()
///         .relative()
///         .size_full()
///         .child("your app")
///         .when(self.show_fps, |this| this.child(fps_monitor(window, cx)))
/// }
/// # }
/// ```
///
/// The monitor behind it is created on first use and reused afterwards, one per
/// window, so this can be called straight from `render` every frame.
pub fn fps_monitor(window: &mut Window, cx: &mut App) -> FpsOverlay {
    let window_id = window.window_handle().window_id();
    let existing = cx
        .try_global::<Monitors>()
        .and_then(|state| state.0.get(&window_id).cloned());
    let monitor = match existing {
        Some(monitor) => monitor,
        None => {
            let monitor = cx.new(|cx| FpsMonitor::new(window, cx));
            cx.default_global::<Monitors>()
                .0
                .insert(window_id, monitor.clone());
            monitor
        }
    };

    FpsOverlay::new(&monitor)
}

/// The monitor [`fps_monitor`] reuses for each window.
///
/// Entries outlive their window; the leak is one small entity per window that
/// ever showed the HUD, which is not worth tracking window closes for.
#[derive(Default)]
struct Monitors(HashMap<WindowId, Entity<FpsMonitor>>);

impl Global for Monitors {}

struct TraceState {
    /// Number of live [`FrameTraceGuard`]s.
    refs: usize,
    /// Whether frame tracing was already on when the first guard was taken,
    /// meaning the host application owns the switch and we must leave it alone.
    owned_by_host: bool,
}

static TRACE_STATE: Mutex<TraceState> = Mutex::new(TraceState {
    refs: 0,
    owned_by_host: false,
});

/// Keeps GPUI's frame trace enabled for as long as it is alive.
///
/// [`gpui::profiler::set_trace_enabled`] is a process-wide switch, and turning it
/// off clears the recorded buffer. A monitor therefore must not disable it
/// while another monitor — or the host application's own profiling — still
/// depends on it, so guards are reference counted and the switch is only
/// restored by the last one. If tracing was already on before the first guard,
/// it is never turned off.
pub(crate) struct FrameTraceGuard {
    _private: (),
}

impl FrameTraceGuard {
    /// Enables frame tracing if it isn't already on.
    pub(crate) fn acquire() -> Self {
        if let Ok(mut state) = TRACE_STATE.lock() {
            if state.refs == 0 {
                // Returns false when the value was already `true`, which means
                // somebody else turned tracing on and owns restoring it.
                state.owned_by_host = !gpui::profiler::set_trace_enabled(true);
            }
            state.refs += 1;
        }
        Self { _private: () }
    }
}

impl Drop for FrameTraceGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = TRACE_STATE.lock() {
            state.refs = state.refs.saturating_sub(1);
            if state.refs == 0 && !state.owned_by_host {
                gpui::profiler::set_trace_enabled(false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropping_an_inner_guard_keeps_tracing_on_for_the_outer_guard() {
        let outer = FrameTraceGuard::acquire();
        let inner = FrameTraceGuard::acquire();
        assert!(gpui::profiler::trace_enabled());

        drop(inner);
        assert!(
            gpui::profiler::trace_enabled(),
            "the outer guard still needs the trace"
        );

        drop(outer);
    }
}
