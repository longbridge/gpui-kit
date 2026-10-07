//! Linux hosts Wry's WebKitGTK view in an X11 child window of the GPUI window.
//!
//! GTK cannot embed into a Wayland surface it does not own, so the GPUI application must
//! run on X11 (natively or through XWayland).

use std::time::Duration;

use gpui::{App, Bounds, Global, Pixels, Window};
use wry::dpi::{PhysicalPosition, PhysicalSize};
use wry::raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, WindowHandle, XlibWindowHandle,
};

/// How often pending GTK events are dispatched between GPUI frames.
const GTK_POLL_INTERVAL: Duration = Duration::from_millis(16);

/// Marks that GTK is initialized and its main loop is driven by GPUI.
struct GtkMainLoop;

impl Global for GtkMainLoop {}

/// Initialize GTK on the X11 backend once and drive its main loop from GPUI.
pub(crate) fn ensure_gtk(cx: &mut App) -> wry::Result<()> {
    if cx.has_global::<GtkMainLoop>() {
        return Ok(());
    }

    if !gtk::is_initialized_main_thread() {
        // Wry downcasts the default GDK display to X11.
        gtk::gdk::set_allowed_backends("x11");
        gtk::init().map_err(|error| wry::Error::Io(std::io::Error::other(error)))?;
    }

    cx.set_global(GtkMainLoop);
    cx.spawn(async move |cx| {
        loop {
            while gtk::events_pending() {
                gtk::main_iteration_do(false);
            }
            cx.background_executor().timer(GTK_POLL_INTERVAL).await;
        }
    })
    .detach();

    Ok(())
}

/// The GPUI window as the Xlib parent Wry requires; GPUI reports its X11 window through XCB.
pub(crate) struct X11Parent(XlibWindowHandle);

impl X11Parent {
    pub(crate) fn new(window: &Window) -> wry::Result<Self> {
        match HasWindowHandle::window_handle(window)?.as_raw() {
            RawWindowHandle::Xcb(handle) => {
                let mut xlib = XlibWindowHandle::new(handle.window.get().into());
                xlib.visual_id = handle.visual_id.map_or(0, |id| id.get().into());
                Ok(Self(xlib))
            }
            RawWindowHandle::Xlib(handle) => Ok(Self(handle)),
            _ => Err(wry::Error::Io(std::io::Error::other(
                "WebView on Linux requires an X11 window; start the application with \
                 `gpui_kit::platform::linux(WindowingModes::X11)`",
            ))),
        }
    }
}

impl HasWindowHandle for X11Parent {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: The X11 window outlives the webview, which is destroyed with the GPUI window.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Xlib(self.0)) })
    }
}

/// Wry scales bounds by GDK's factor, which ignores GPUI's, so pass device pixels.
pub(crate) fn device_bounds(bounds: Bounds<Pixels>, scale_factor: f32) -> wry::Rect {
    let device = |value: Pixels| f32::from(value) * scale_factor;
    wry::Rect {
        position: PhysicalPosition::new(device(bounds.origin.x), device(bounds.origin.y)).into(),
        size: PhysicalSize::new(device(bounds.size.width), device(bounds.size.height)).into(),
    }
}
