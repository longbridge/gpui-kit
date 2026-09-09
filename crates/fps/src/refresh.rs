//! The refresh rate of the display a window is on, read from the platform.
//!
//! GPUI does not report it, and it cannot be recovered from the frames a
//! window presented: those gaps are whole multiples of the panel's period, so
//! they bound it from below and never from above — 41.7ms is six refreshes at
//! 144Hz and one at 24Hz, and nothing in the timing says which. Every estimate
//! tried before this read a real window wrong.
//!
//! So it is asked for. GPUI does hand out the platform's own display handle
//! through [`gpui::DisplayId`], which is a `CGDirectDisplayID` on macOS and an
//! `HMONITOR` on Windows, and on Wayland the outputs can be enumerated again
//! and matched by the identity GPUI derives from their names.
//!
//! `None` means nobody could say — a platform without a query here, a virtual
//! display, or a panel with no fixed rate — and the caller shows an uncapped
//! reading rather than one held to a guess.

use std::time::Duration;

use gpui::PlatformDisplay;

/// The period between refreshes of `display`, when the platform reports one.
pub(crate) fn display_refresh_rate(display: &dyn PlatformDisplay) -> Option<Duration> {
    platform::refresh_rate(display)
}

/// Turns a rate in hertz into the period the rest of the crate works in,
/// rejecting the zeroes platforms use to mean "no fixed rate".
#[cfg(any(target_os = "macos", target_os = "windows"))]
fn period_from_hertz(hertz: f64) -> Option<Duration> {
    (hertz > 1.).then(|| Duration::from_secs_f64(1. / hertz))
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use core_graphics::display::{CGDirectDisplayID, CGDisplay};

    pub(super) fn refresh_rate(display: &dyn PlatformDisplay) -> Option<Duration> {
        let id: u64 = display.id().into();
        // Zero rather than an error is how CoreGraphics says this display has
        // no fixed rate, which is what a built-in panel reports: on ProMotion
        // there genuinely is not one, and the nominal period would have to come
        // from CoreVideo instead.
        let mode = CGDisplay::new(id as CGDirectDisplayID).display_mode()?;
        period_from_hertz(mode.refresh_rate())
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use windows::{Win32::Graphics::Gdi::*, core::*};

    pub(super) fn refresh_rate(display: &dyn PlatformDisplay) -> Option<Duration> {
        let id: u64 = display.id().into();
        let monitor = HMONITOR(id as _);

        let mut info = MONITORINFOEXW {
            monitorInfo: MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
                ..Default::default()
            },
            ..Default::default()
        };
        if !unsafe { GetMonitorInfoW(monitor, &mut info as *mut _ as *mut MONITORINFO) }.as_bool() {
            return None;
        }

        let mut mode = DEVMODEW {
            dmSize: std::mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        let device = PCWSTR(info.szDevice.as_ptr());
        if !unsafe { EnumDisplaySettingsW(device, ENUM_CURRENT_SETTINGS, &mut mode) }.as_bool() {
            return None;
        }
        // Zero and one both mean "whatever the hardware defaults to" rather
        // than a rate, which is what a driver reports when it has none to give.
        period_from_hertz(mode.dmDisplayFrequency as f64)
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::{collections::HashMap, sync::OnceLock};
    use uuid::Uuid;
    use wayland_client::{
        Connection, Dispatch, Proxy as _, QueueHandle, WEnum,
        protocol::{wl_output, wl_registry},
    };

    /// Wayland hands each client its own object ids, so the id GPUI reports for
    /// an output means nothing on a second connection. What both sides can
    /// agree on is the output's name, which GPUI folds into the display's uuid
    /// — so the outputs are enumerated again and matched by that.
    pub(super) fn refresh_rate(display: &dyn PlatformDisplay) -> Option<Duration> {
        let uuid = display.uuid().ok()?;
        outputs().get(&uuid).copied()
    }

    /// Asked once. Outputs change when a monitor is plugged in or its mode is
    /// changed, and neither happens in the middle of reading a frame counter.
    fn outputs() -> &'static HashMap<Uuid, Duration> {
        static OUTPUTS: OnceLock<HashMap<Uuid, Duration>> = OnceLock::new();
        OUTPUTS.get_or_init(|| query_outputs().unwrap_or_default())
    }

    fn query_outputs() -> Option<HashMap<Uuid, Duration>> {
        let connection = Connection::connect_to_env().ok()?;
        let mut queue = connection.new_event_queue();
        let handle = queue.handle();
        let _registry = connection.display().get_registry(&handle, ());

        let mut state = State::default();
        // Once for the globals, once for the events the outputs send back.
        queue.roundtrip(&mut state).ok()?;
        queue.roundtrip(&mut state).ok()?;
        Some(state.rates)
    }

    #[derive(Default)]
    struct State {
        /// Name and current mode, per output object, until its `Done`.
        pending: HashMap<u32, (Option<String>, Option<Duration>)>,
        rates: HashMap<Uuid, Duration>,
    }

    impl Dispatch<wl_registry::WlRegistry, ()> for State {
        fn event(
            _: &mut Self,
            registry: &wl_registry::WlRegistry,
            event: wl_registry::Event,
            _: &(),
            _: &Connection,
            handle: &QueueHandle<Self>,
        ) {
            if let wl_registry::Event::Global {
                name,
                interface,
                version,
            } = event
                && interface == wl_output::WlOutput::interface().name
            {
                // Version 4 is where an output started naming itself, which is
                // the only thing this connection and GPUI's can match on.
                if version >= 4 {
                    registry.bind::<wl_output::WlOutput, _, _>(name, 4, handle, ());
                }
            }
        }
    }

    impl Dispatch<wl_output::WlOutput, ()> for State {
        fn event(
            state: &mut Self,
            output: &wl_output::WlOutput,
            event: wl_output::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            let id = output.id().protocol_id();
            match event {
                wl_output::Event::Name { name } => {
                    state.pending.entry(id).or_default().0 = Some(name);
                }
                wl_output::Event::Mode { flags, refresh, .. } => {
                    // Outputs advertise every mode they support; only one of
                    // them is the one being scanned out.
                    let current = matches!(flags, WEnum::Value(mode) if mode.contains(wl_output::Mode::Current));
                    if current && refresh > 0 {
                        state.pending.entry(id).or_default().1 =
                            Some(Duration::from_nanos(1_000_000_000_000 / refresh as u64));
                    }
                }
                wl_output::Event::Done => {
                    if let Some((Some(name), Some(rate))) = state.pending.remove(&id) {
                        // The same derivation GPUI uses for a Wayland display's
                        // uuid, which is what makes the two sides comparable.
                        state
                            .rates
                            .insert(Uuid::new_v5(&Uuid::NAMESPACE_DNS, name.as_bytes()), rate);
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
mod platform {
    use super::*;

    pub(super) fn refresh_rate(_display: &dyn PlatformDisplay) -> Option<Duration> {
        None
    }
}
