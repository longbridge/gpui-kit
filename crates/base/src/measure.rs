use std::sync::LazyLock;

use gpui::SharedString;

/// Read once: measurement is a startup debug flag, and callers such as the
/// table query it for every rendered cell.
static MEASUREMENT_ENABLED: LazyLock<bool> = LazyLock::new(|| {
    std::env::var_os("ZED_MEASUREMENTS").is_some()
        || std::env::var_os("GPUI_MEASUREMENTS").is_some()
});

#[inline]
pub fn measurement_enabled() -> bool {
    *MEASUREMENT_ENABLED
}

/// Measures `f` when `if_` is true and measurement logging is enabled.
#[inline]
#[track_caller]
pub fn measure_if(name: impl Into<SharedString>, if_: bool, f: impl FnOnce()) {
    if if_ && measurement_enabled() {
        let measure = Measure::new(name);
        f();
        measure.end();
    } else {
        f();
    }
}

/// Measures `f` when measurement logging is enabled.
#[inline]
#[track_caller]
pub fn measure(name: impl Into<SharedString>, f: impl FnOnce()) {
    measure_if(name, true, f);
}

/// An elapsed-time measurement emitted through `tracing` when ended.
pub struct Measure {
    name: SharedString,
    start: instant::Instant,
}

impl Measure {
    #[track_caller]
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            start: instant::Instant::now(),
        }
    }

    #[track_caller]
    pub fn end(self) {
        let duration = self.start.elapsed();
        tracing::trace!("{} in {:?}", self.name, duration);
    }
}
