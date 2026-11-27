//! Linux hosts Wry's WebKitGTK view in an X11 child window of the GPUI window.
//!
//! GTK cannot embed into a Wayland surface it does not own, so the GPUI application must
//! run on X11 (natively or through XWayland).

use std::{cell::Cell, rc::Rc, time::Duration};

use gpui::{
    App, Bounds, Global, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, Pixels,
    PlatformInput, Point, Window, point, px,
};
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

/// WebKitGTK answers `prefers-color-scheme` from GTK's preference, which GTK 3 does not
/// read from the desktop, so without this every page renders light.
pub(crate) fn set_theme(theme: wry::Theme) {
    use gtk::prelude::GtkSettingsExt as _;
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(matches!(theme, wry::Theme::Dark));
    }
}

/// The GPUI window as the Xlib parent Wry requires; GPUI reports its X11 window through XCB.
pub(crate) struct X11Parent(XlibWindowHandle);

impl X11Parent {
    pub(crate) fn new(window: &Window) -> wry::Result<Self> {
        Self::from_raw(HasWindowHandle::window_handle(window)?.as_raw())
    }

    fn from_raw(handle: RawWindowHandle) -> wry::Result<Self> {
        match handle {
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

/// Give the page the keyboard and GPUI focus when it is pressed, and forward the press to GPUI, which does not
/// otherwise see it, so overlays that close on an outside click (popovers, menus) close when
/// the page is clicked.
pub(crate) fn forward_mouse_down(
    webview: &wry::WebView,
    origin: Rc<Cell<Point<Pixels>>>,
    focus_handle: gpui::FocusHandle,
    window: &Window,
    cx: &App,
) {
    use gtk::{
        gdk::EventType,
        glib::Propagation,
        prelude::{Cast as _, WidgetExt as _},
    };
    use wry::WebViewExtUnix as _;

    let handle = window.window_handle();
    let cx = cx.to_async();
    webview
        .webview()
        .connect_button_press_event(move |widget, event| {
            let button = match event.button() {
                1 => MouseButton::Left,
                2 => MouseButton::Middle,
                3 => MouseButton::Right,
                _ => return Propagation::Proceed,
            };
            if event.event_type() != EventType::ButtonPress {
                return Propagation::Proceed;
            }
            keyboard::focus(widget.upcast_ref());
            let (x, y) = event.position();
            let gdk_scale = widget.scale_factor() as f32;
            let origin = origin.get();
            let focus_handle = focus_handle.clone();
            // GTK dispatches this from the GTK main loop; update GPUI once it is done.
            cx.spawn(async move |cx| {
                handle.update(cx, |_, window, cx| {
                    let scale = gdk_scale / window.scale_factor();
                    let position = origin + point(px(x as f32 * scale), px(y as f32 * scale));
                    window.dispatch_event(
                        PlatformInput::MouseDown(MouseDownEvent {
                            button,
                            position,
                            modifiers: Modifiers::default(),
                            click_count: 1,
                            first_mouse: false,
                        }),
                        cx,
                    );
                    window.dispatch_event(
                        PlatformInput::MouseUp(MouseUpEvent {
                            button,
                            position,
                            modifiers: Modifiers::default(),
                            click_count: 1,
                        }),
                        cx,
                    );
                    // GPUI focus follows the X input focus into the page, so moving GPUI focus
                    // elsewhere later gives the keyboard back to the GPUI window.
                    window.focus(&focus_handle, cx);
                })
            })
            .detach();
            Propagation::Proceed
        });
}

/// Zoom the page so its CSS pixels match GPUI's scale (GDK only supports integer scales),
/// times the page zoom the application asked for.
pub(crate) fn match_scale_factor(webview: &wry::WebView, scale_factor: f32, zoom: f64) {
    use gtk::prelude::WidgetExt as _;
    use wry::WebViewExtUnix as _;

    let gdk_scale = webview.webview().scale_factor().max(1);
    let _ = webview.zoom(f64::from(scale_factor) / f64::from(gdk_scale) * zoom);
}

/// A GPUI Fast composition surface hosting the webview: an X11 child window that GPUI cuts
/// its overlays out of, so they render above the page.
#[cfg(feature = "gpui-fast")]
pub(crate) struct NativeWebViewSurface {
    surface: gpui::WindowCompositionSurface,
    scale_factor: std::cell::Cell<f32>,
    window: gpui::AnyWindowHandle,
    cx: gpui::AsyncApp,
}

#[cfg(feature = "gpui-fast")]
impl NativeWebViewSurface {
    pub(crate) fn new(window: &Window, cx: &App) -> anyhow::Result<Self> {
        let surface = window
            .enable_window_composition()?
            .create_native_surface()?;
        Ok(Self {
            surface,
            scale_factor: std::cell::Cell::new(window.scale_factor()),
            window: window.window_handle(),
            cx: cx.to_async(),
        })
    }

    /// The surface's X11 window, as the parent Wry builds the webview in.
    pub(crate) fn parent(&self) -> wry::Result<X11Parent> {
        let handle = self
            .surface
            .platform_surface()
            .and_then(|surface| surface.platform_handle())
            .map_err(|error| wry::Error::Io(std::io::Error::other(error)))?;
        match handle.downcast::<RawWindowHandle>() {
            Ok(handle) => X11Parent::from_raw(*handle),
            Err(_) => Err(wry::Error::UnsupportedWindowHandle),
        }
    }

    pub(crate) fn set_scale_factor(&self, scale_factor: f32) {
        self.scale_factor.set(scale_factor);
    }

    /// Position the surface in the window and fill it with the webview.
    pub(crate) fn set_bounds(&self, webview: &wry::WebView, rect: wry::Rect) -> anyhow::Result<()> {
        use gpui::{DevicePixels, point, size};

        let scale_factor = f64::from(self.scale_factor.get());
        let position = rect.position.to_physical::<i32>(scale_factor);
        let extent = rect.size.to_physical::<i32>(scale_factor);
        self.surface.platform_surface()?.set_bounds(Bounds::new(
            point(DevicePixels(position.x), DevicePixels(position.y)),
            size(DevicePixels(extent.width), DevicePixels(extent.height)),
        ))?;
        webview.set_bounds(wry::Rect {
            position: PhysicalPosition::new(0, 0).into(),
            size: PhysicalSize::new(extent.width, extent.height).into(),
        })?;
        Ok(())
    }

    pub(crate) fn set_geometry(
        &self,
        webview: &wry::WebView,
        document: Bounds<Pixels>,
        visible: Bounds<Pixels>,
    ) -> anyhow::Result<()> {
        self.surface
            .platform_surface()?
            .set_bounds(visible.to_device_pixels(self.scale_factor.get()))?;
        let child = Bounds::new(document.origin - visible.origin, document.size);
        webview.set_bounds(device_bounds(child, self.scale_factor.get()))?;
        Ok(())
    }

    pub(crate) fn set_visible(&self, visible: bool) -> anyhow::Result<()> {
        self.surface.platform_surface()?.set_visible(visible)
    }

    pub(crate) fn release(self, webview: std::rc::Rc<wry::WebView>) {
        detach_from_surface(&webview);
        // Entity destruction can occur while App is borrowed. Drop this reference to Wry's view
        // (other handles may keep it alive), then remove the surface window.
        self.cx
            .clone()
            .spawn(async move |cx| {
                drop(webview);
                let result = self.window.update(cx, |_, window, _| {
                    window
                        .enable_window_composition()?
                        .remove_surface(self.surface.id())?;
                    // The rendered frame still blocks pointer input where the surface was,
                    // so draw again for hover and clicks to reach the elements beneath.
                    window.refresh();
                    anyhow::Ok(())
                });
                if let Ok(Err(error)) = result {
                    log::warn!("cannot release WebView composition surface: {error:#}");
                }
            })
            .detach();
    }
}

/// Move Wry's X11 window out of the composition surface window, unmapped, before the surface
/// is removed.
///
/// GPUI removes the surface window through its own X11 connection, which also destroys every
/// window inside it. GTK sends its requests for Wry's windows through another connection, so
/// requests it makes after that (destroying them when the webview is dropped, possibly later
/// through a `WebViewHandle`) name windows that no longer exist, and GDK aborts the process on
/// that X11 error. Outside the surface, Wry's windows stay valid until Wry destroys them.
#[cfg(feature = "gpui-fast")]
fn detach_from_surface(webview: &wry::WebView) {
    use gdkx11::{X11Display, X11Window};
    use gtk::{glib::translate::ToGlibPtr as _, prelude::*};
    use wry::WebViewExtUnix as _;
    use x11::xlib;

    // Wry builds its GTK window on its own X11 window (foreign to GDK), a child of the surface.
    let Some(container) = webview
        .webview()
        .toplevel()
        .and_then(|toplevel| toplevel.window())
        .and_then(|window| window.downcast::<X11Window>().ok())
        .map(|window| window.xid())
    else {
        return;
    };
    let Some(display) =
        gtk::gdk::Display::default().and_then(|display| display.downcast::<X11Display>().ok())
    else {
        return;
    };
    // SAFETY: `display` is a live GdkX11Display; its Xlib connection lives as long as it.
    let xdisplay = unsafe { gdkx11::ffi::gdk_x11_display_get_xdisplay(display.to_glib_none().0) };
    display.error_trap_push();
    // SAFETY: A valid Xlib connection and window; errors are caught by the trap.
    unsafe {
        let root = xlib::XDefaultRootWindow(xdisplay);
        xlib::XUnmapWindow(xdisplay, container);
        xlib::XReparentWindow(xdisplay, container, root, 0, 0);
    }
    // Wait until the server has moved it, so GPUI's removal of the surface comes after.
    display.upcast_ref::<gtk::gdk::Display>().sync();
    display.error_trap_pop_ignored();
}

/// The X11 window of a GPUI window, which hosts its webviews.
pub(crate) fn host_window(window: &Window) -> Option<std::ffi::c_ulong> {
    X11Parent::new(window).ok().map(|parent| parent.0.window)
}

/// Keyboard focus between GPUI and the webviews' GTK windows.
///
/// A webview is an X11 child window of its GPUI window. X11 sends key events to the window
/// with the input focus, but when that window is an ancestor of the window under the pointer,
/// the window under the pointer gets them. GTK selects key events on its windows (through
/// XInput2, so the event mask cannot be narrowed), so the keyboard followed the pointer: with
/// a GPUI input focused, typing over the page went to the page, and after a click into the
/// page, typing went to GPUI as soon as the pointer left it.
///
/// So a page takes the X input focus when it is clicked, and its GPUI window takes it back
/// when GPUI focus moves elsewhere (a click on GPUI content, or focus moved from code), or the
/// webview is hidden. Key events that reach a page without the X input focus are sent on to
/// its GPUI window from the page's own key signals; GDK's process-wide event handler is left
/// alone.
pub(crate) mod keyboard {
    use std::ffi::c_ulong;

    use gdkx11::{X11Display, X11Window};
    use gtk::{
        gdk,
        glib::{Propagation, translate::ToGlibPtr as _},
        prelude::*,
    };
    use x11::xlib;

    /// The GTK widget of `webview`'s page.
    pub(crate) fn page(webview: &wry::WebView) -> gtk::Widget {
        use wry::WebViewExtUnix as _;
        webview.webview().upcast()
    }

    /// Send key events that reach `page` without the X input focus on to its GPUI window
    /// `host`, so the pointer has no say in where they go.
    pub(crate) fn register(page: &gtk::Widget, host: c_ulong) {
        let forward = move |widget: &gtk::Widget, key: &gdk::EventKey| {
            let Some(xid) = page_xid(widget) else {
                return Propagation::Proceed;
            };
            let Some((display, xdisplay)) = display() else {
                return Propagation::Proceed;
            };
            if input_focus(xdisplay) == xid {
                return Propagation::Proceed;
            }
            let press = key.event_type() == gdk::EventType::KeyPress;
            send_key(&display, xdisplay, host, key, press);
            Propagation::Stop
        };
        page.connect_key_press_event(forward);
        page.connect_key_release_event(forward);
    }

    /// Give `page` the X input focus.
    pub(crate) fn focus(page: &gtk::Widget) {
        if let Some(xid) = page_xid(page)
            && let Some((display, xdisplay)) = display()
        {
            set_input_focus(&display, xdisplay, xid);
        }
    }

    /// Give `page` the X input focus if its GPUI window `host`, or a window inside it, has it.
    pub(crate) fn focus_if_active(page: &gtk::Widget, host: c_ulong) {
        let Some(xid) = page_xid(page) else {
            return;
        };
        let Some((display, xdisplay)) = display() else {
            return;
        };
        let focused = input_focus(xdisplay);
        if focused != xid && is_within(xdisplay, focused, host) {
            set_input_focus(&display, xdisplay, xid);
        }
    }

    /// Give the X input focus back to the GPUI window `host`, if `page` has it.
    pub(crate) fn release(page: &gtk::Widget, host: c_ulong) {
        let Some((display, xdisplay)) = display() else {
            return;
        };
        if page_xid(page).is_some_and(|xid| input_focus(xdisplay) == xid) {
            set_input_focus(&display, xdisplay, host);
        }
    }

    /// Whether `window` is `ancestor` or inside it.
    fn is_within(xdisplay: *mut xlib::Display, mut window: c_ulong, ancestor: c_ulong) -> bool {
        // None (0) and PointerRoot (1) are not windows.
        while window > 1 {
            if window == ancestor {
                return true;
            }
            let (mut root, mut parent) = (0, 0);
            let mut children = std::ptr::null_mut();
            let mut count = 0;
            // SAFETY: A valid Xlib connection and out pointers; the child list is freed.
            let ok = unsafe {
                let ok = xlib::XQueryTree(
                    xdisplay,
                    window,
                    &mut root,
                    &mut parent,
                    &mut children,
                    &mut count,
                );
                if !children.is_null() {
                    xlib::XFree(children.cast());
                }
                ok
            };
            if ok == 0 || parent == root {
                return false;
            }
            window = parent;
        }
        false
    }

    /// The X11 window that receives the page's key events: GDK's toplevel window of the
    /// widget. (Wry's container window above it is foreign to GDK, and key events sent to it
    /// do not reach the page.)
    fn page_xid(page: &gtk::Widget) -> Option<c_ulong> {
        xid(&page.window()?.toplevel())
    }

    fn xid(window: &gdk::Window) -> Option<c_ulong> {
        Some(window.downcast_ref::<X11Window>()?.xid())
    }

    fn display() -> Option<(X11Display, *mut xlib::Display)> {
        let display = gdk::Display::default()?.downcast::<X11Display>().ok()?;
        // SAFETY: `display` is a live GdkX11Display; its Xlib connection lives as long as it.
        let xdisplay =
            unsafe { gdkx11::ffi::gdk_x11_display_get_xdisplay(display.to_glib_none().0) };
        Some((display, xdisplay))
    }

    fn input_focus(xdisplay: *mut xlib::Display) -> c_ulong {
        let (mut window, mut revert_to) = (0, 0);
        // SAFETY: A valid Xlib connection and out pointers.
        unsafe { xlib::XGetInputFocus(xdisplay, &mut window, &mut revert_to) };
        window
    }

    /// Set the X input focus on GTK's connection, after GTK's own pending requests.
    fn set_input_focus(display: &X11Display, xdisplay: *mut xlib::Display, xid: c_ulong) {
        display.error_trap_push();
        // SAFETY: A valid Xlib connection; errors (an unmapped window cannot take the focus)
        // are caught by the trap.
        unsafe { xlib::XSetInputFocus(xdisplay, xid, xlib::RevertToParent, xlib::CurrentTime) };
        display.error_trap_pop_ignored();
    }

    /// Send a key event that GTK received to the GPUI window `host` as a core X11 key event.
    fn send_key(
        display: &X11Display,
        xdisplay: *mut xlib::Display,
        host: c_ulong,
        key: &gdk::EventKey,
        press: bool,
    ) {
        // Modifier and button bits are the same in GDK and X11; XKB adds the group.
        let state = (key.state().bits() & 0x1fff) | (u32::from(key.group() & 3) << 13);
        let mut event = xlib::XEvent::from(xlib::XKeyEvent {
            type_: if press {
                xlib::KeyPress
            } else {
                xlib::KeyRelease
            },
            serial: 0,
            send_event: xlib::True,
            display: xdisplay,
            window: host,
            // SAFETY: A valid Xlib connection.
            root: unsafe { xlib::XDefaultRootWindow(xdisplay) },
            subwindow: 0,
            time: xlib::Time::from(key.time()),
            x: 0,
            y: 0,
            x_root: 0,
            y_root: 0,
            state,
            keycode: u32::from(key.hardware_keycode()),
            same_screen: xlib::True,
        });
        display.error_trap_push();
        // SAFETY: A valid Xlib connection and key event; errors (the window is gone) are caught
        // by the trap.
        unsafe {
            xlib::XSendEvent(xdisplay, host, xlib::False, xlib::NoEventMask, &mut event);
            xlib::XFlush(xdisplay);
        }
        display.error_trap_pop_ignored();
    }
}
