//! Platform-specific window configuration utilities.
//!
//! Provides cross-platform APIs to configure window behavior beyond what
//! [`WindowOptions`] offers:
//!
//! - **Skip taskbar** — hide the window from the taskbar / dock / alt-tab
//! - **Click-through** — mouse events pass through the window
//! - **Always on top** — window stays above other windows
//!
//! # Platform support
//!
//! | Feature | macOS | Windows | Linux/X11 | Wayland |
//! | ------- | ----- | ------- | --------- | ------- |
//! | Skip taskbar | ✅ | ✅ | ✅ | ❌ |
//! | Always on top | ✅ | ✅ | ✅ | ❌ |
//! | Click-through | ✅ | ✅ | ⚠️ Partial | ❌ |
//!
//! Wayland is not supported at the client level due to protocol limitations.
//! On Linux/X11, click-through uses `_NET_WM_WINDOW_TYPE_SPLASH` as a hint but
//! does not implement true mouse-pass-through (that requires the XShape
//! extension and is left for future work).
//!
//! # Example
//!
//! ```no_run
//! use gpui_kit::base::window_ext;
//!
//! // After creating a window, configure it as an overlay:
//! window_ext::make_overlay(&window).expect("configure overlay");
//! ```
//!
//! [`WindowOptions`]: gpui::WindowOptions

use anyhow::Result;
use gpui::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Window behavior configuration.
///
/// Construct with the builder methods and pass to [`configure`].
///
/// # Example
///
/// ```
/// use gpui_kit::base::window_ext::WindowBehavior;
///
/// let behavior = WindowBehavior::new()
///     .skip_taskbar(true)
///     .click_through(true)
///     .always_on_top(true);
/// ```
#[derive(Clone, Debug, Default)]
pub struct WindowBehavior {
    skip_taskbar: bool,
    click_through: bool,
    always_on_top: bool,
}

impl WindowBehavior {
    /// Create a new `WindowBehavior` with all options disabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the window is hidden from the taskbar, dock, and alt-tab.
    pub fn is_skip_taskbar(&self) -> bool {
        self.skip_taskbar
    }

    /// Whether mouse events pass through the window.
    pub fn is_click_through(&self) -> bool {
        self.click_through
    }

    /// Whether the window stays above other windows.
    pub fn is_always_on_top(&self) -> bool {
        self.always_on_top
    }

    /// Set whether to hide the window from the taskbar.
    pub fn with_skip_taskbar(mut self, skip: bool) -> Self {
        self.skip_taskbar = skip;
        self
    }

    /// Set whether mouse events pass through the window.
    pub fn with_click_through(mut self, click_through: bool) -> Self {
        self.click_through = click_through;
        self
    }

    /// Set whether the window should stay on top of other windows.
    pub fn with_always_on_top(mut self, on_top: bool) -> Self {
        self.always_on_top = on_top;
        self
    }
}

/// Apply a [`WindowBehavior`] configuration to a window.
///
/// Call this after the window has been created (e.g. inside the
/// `open_window` closure, or from a `cx.defer` callback).
///
/// Returns an error if the platform-specific handle cannot be obtained or
/// the OS call fails.
///
/// # Example
///
/// ```no_run
/// use gpui_kit::base::window_ext::{self, WindowBehavior};
///
/// // Inside a window handler:
/// let behavior = WindowBehavior::new()
///     .with_skip_taskbar(true)
///     .with_always_on_top(true);
/// window_ext::configure(&window, &behavior)?;
/// ```
pub fn configure(window: &Window, behavior: &WindowBehavior) -> Result<()> {
    let handle = HasWindowHandle::window_handle(window).map_err(|e| {
        anyhow::Error::msg(format!("failed to get window handle: {e}"))
    })?;
    match handle.as_raw() {
        #[cfg(target_os = "macos")]
        RawWindowHandle::AppKit(handle) => configure_macos(handle, behavior),
        #[cfg(target_os = "windows")]
        RawWindowHandle::Win32(handle) => configure_windows(handle, behavior),
        #[cfg(target_os = "linux")]
        RawWindowHandle::Xlib(handle) => configure_xlib(window, handle, behavior),
        #[cfg(target_os = "linux")]
        RawWindowHandle::Wayland(_) => {
            return Err(anyhow::Error::msg(
                "Wayland does not support window_ext features at the client level",
            ));
        }
        other => {
            return Err(anyhow::Error::msg(format!(
                "unsupported window backend: {other:?}"
            )));
        }
    }
}

/// Convenience: configure as an overlay window (skip taskbar + click-through + always on top).
pub fn make_overlay(window: &Window) -> Result<()> {
    configure(
        window,
        &WindowBehavior::new()
            .with_skip_taskbar(true)
            .with_click_through(true)
            .with_always_on_top(true),
    )
}

/// Convenience: configure as a floating window (skip taskbar + always on top, no click-through).
pub fn make_floating(window: &Window) -> Result<()> {
    configure(
        window,
        &WindowBehavior::new()
            .with_skip_taskbar(true)
            .with_always_on_top(true),
    )
}

/// Convenience: enable click-through only.
pub fn set_click_through(window: &Window, enabled: bool) -> Result<()> {
    configure(window, &WindowBehavior::new().with_click_through(enabled))
}

// ---------------------------------------------------------------------------
// macOS implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn configure_macos(
    handle: raw_window_handle::AppKitWindowHandle,
    behavior: &WindowBehavior,
) -> Result<()> {
    use objc2::msg_send;
    use objc2_app_kit::NSView;

    // Get the NSWindow from the NSView handle.
    let ns_view_ptr = handle.ns_view.as_ptr() as *mut NSView;
    if ns_view_ptr.is_null() {
        return Err(anyhow::Error::msg("ns_view is null".into()));
    }
    let ns_view = unsafe { &*ns_view_ptr };
    let Some(ns_window) = ns_view.window() else {
        return Err(anyhow::Error::msg(
            "ns_view is not attached to a window".into(),
        ));
    };
    // Use a reference for msg_send! (Retained<NSWindow> is not MessageReceiver).
    let ns_window_ref = &*ns_window;

    unsafe {
        // Configure collection behavior (taskbar / alt-tab / spaces).
        if behavior.is_skip_taskbar() || behavior.is_always_on_top() {
            let mut collection_behavior: usize = 0;

            if behavior.is_skip_taskbar() {
                // NSWindowCollectionBehavior::CanJoinAllSpaces = 1 << 0
                collection_behavior |= 1 << 0;
                // NSWindowCollectionBehavior::Stationary = 1 << 6
                collection_behavior |= 1 << 6;
                // NSWindowCollectionBehavior::IgnoresCycle = 1 << 7 (hide from Alt-Tab)
                collection_behavior |= 1 << 7;
                // NSWindowCollectionBehavior::ExcludedFromWindowsMenu = 1 << 8
                collection_behavior |= 1 << 8;
            }

            if behavior.is_always_on_top() {
                // NSWindowCollectionBehavior::CanJoinAllSpaces = 1 << 0
                collection_behavior |= 1 << 0;
            }

            let _: () = msg_send![ns_window_ref, setCollectionBehavior: collection_behavior];
        }

        // Configure window level (always on top).
        // NSNormalWindowLevel = 0, NSFloatingWindowLevel = 3.
        // Use 5 to stay above floating windows (e.g. menus).
        if behavior.is_always_on_top() {
            const WINDOW_LEVEL_ABOVE_FLOATING: i32 = 5;
            let _: () = msg_send![ns_window_ref, setLevel: WINDOW_LEVEL_ABOVE_FLOATING];
        }

        // Configure click-through.
        if behavior.is_click_through() {
            let _: () = msg_send![ns_window_ref, setIgnoresMouseEvents: true];
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Windows implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn configure_windows(
    handle: raw_window_handle::Win32WindowHandle,
    behavior: &WindowBehavior,
) -> Result<()> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, SET_WINDOW_POS_FLAGS, WS_EX_LAYERED,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, SetWindowLongW, SetWindowPos,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    };

    let hwnd: HWND = handle.hwnd.get() as *mut _;
    if hwnd.is_null() {
        return Err(anyhow::Error::msg("HWND is null".into()));
    }

    unsafe {
        // Build extended window styles.
        let mut ex_style: u32 = 0;

        if behavior.is_skip_taskbar() {
            // WS_EX_TOOLWINDOW: hide from taskbar.
            ex_style |= WS_EX_TOOLWINDOW;
            // WS_EX_NOACTIVATE: don't steal focus.
            ex_style |= WS_EX_NOACTIVATE;
        }

        if behavior.is_click_through() {
            // WS_EX_TRANSPARENT: click-through.
            ex_style |= WS_EX_TRANSPARENT;
            // WS_EX_LAYERED: required for transparent styles.
            ex_style |= WS_EX_LAYERED;
        }

        // Apply extended styles.
        let result = SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style as i32);
        if result == 0 {
            let err = std::io::Error::last_os_error();
            return Err(anyhow::Error::msg(format!(
                "SetWindowLongW failed: {err}"
            )));
        }

        // Build set-window-pos flags.
        let flags: SET_WINDOW_POS_FLAGS =
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW;

        // Set z-order.
        let z_order = if behavior.is_always_on_top() {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        };
        let result = SetWindowPos(hwnd, z_order, 0, 0, 0, 0, flags);
        if result == 0 {
            let err = std::io::Error::last_os_error();
            return Err(anyhow::Error::msg(format!(
                "SetWindowPos failed: {err}"
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Linux/X11 implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn configure_xlib(
    _window: &Window,
    handle: raw_window_handle::XlibWindowHandle,
    behavior: &WindowBehavior,
) -> Result<()> {
    use x11_dl::xlib;

    // X11 requires a Display connection. We open a new one here because
    // GPUI does not expose its display handle. This is intentionally a fresh
    // connection per call — caching would require lifetime management that
    // does not fit this API.
    let xlib = match x11_dl::xlib::Xlib::open() {
        Ok(x) => x,
        Err(e) => {
            return Err(anyhow::Error::msg(format!(
                "failed to load Xlib: {e}"
            )))
        }
    };

    unsafe {
        let display = (xlib.XOpenDisplay)(std::ptr::null());
        if display.is_null() {
            return Err(anyhow::Error::msg("XOpenDisplay failed"));
        }
        let window = handle.window as xlib::Window;

        // Helper: intern an atom.
        let intern_atom = |name: &[u8]| -> xlib::Atom {
            (xlib.XInternAtom)(display, name.as_ptr() as *const i8, 0)
        };

        // Helper: set a single Atom property on the window.
        let set_atom_property = |property: xlib::Atom, value: xlib::Atom| {
            (xlib.XChangeProperty)(
                display,
                window,
                property,
                xlib::XA_ATOM,
                32,
                xlib::PropModeReplace,
                &value as *const _ as *const u8,
                1,
            );
        };

        // Set window type to DOCK to hide from taskbar/pager.
        if behavior.is_skip_taskbar() {
            let atom_net_wm_window_type = intern_atom(b"_NET_WM_WINDOW_TYPE\0");
            let atom_dock = intern_atom(b"_NET_WM_WINDOW_TYPE_DOCK\0");
            set_atom_property(atom_net_wm_window_type, atom_dock);

            // Also set SKIP_TASKBAR state.
            let atom_net_wm_state = intern_atom(b"_NET_WM_STATE\0");
            let atom_skip_taskbar = intern_atom(b"_NET_WM_STATE_SKIP_TASKBAR\0");
            set_atom_property(atom_net_wm_state, atom_skip_taskbar);
        }

        // Always on top.
        if behavior.is_always_on_top() {
            let atom_net_wm_state = intern_atom(b"_NET_WM_STATE\0");
            let atom_above = intern_atom(b"_NET_WM_STATE_ABOVE\0");
            set_atom_property(atom_net_wm_state, atom_above);
        }

        // Click-through on X11 is not fully implementable without the Shape
        // extension. We set the window type to SPLASH as a hint (some WMs
        // treat splash windows as non-interactive), but true mouse-pass-through
        // requires XShapeCombineRegion and is left for future work.
        if behavior.is_click_through() {
            let atom_net_wm_window_type = intern_atom(b"_NET_WM_WINDOW_TYPE\0");
            let atom_splash = intern_atom(b"_NET_WM_WINDOW_TYPE_SPLASH\0");
            set_atom_property(atom_net_wm_window_type, atom_splash);
        }

        (xlib.XFlush)(display);
        (xlib.XCloseDisplay)(display);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_behavior_is_all_disabled() {
        let b = WindowBehavior::new();
        assert!(!b.is_skip_taskbar());
        assert!(!b.is_click_through());
        assert!(!b.is_always_on_top());
    }

    #[test]
    fn builder_flags_roundtrip() {
        let b = WindowBehavior::new()
            .with_skip_taskbar(true)
            .with_click_through(true)
            .with_always_on_top(true);
        assert!(b.is_skip_taskbar());
        assert!(b.is_click_through());
        assert!(b.is_always_on_top());
    }

    #[test]
    fn builder_can_disable_individually() {
        let b = WindowBehavior::new()
            .with_skip_taskbar(true)
            .with_click_through(false);
        assert!(b.is_skip_taskbar());
        assert!(!b.is_click_through());
        assert!(!b.is_always_on_top());
    }
}
