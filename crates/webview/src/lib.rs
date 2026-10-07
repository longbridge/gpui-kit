#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;

#[cfg(all(feature = "gpui-fast", target_os = "macos"))]
mod composition;

use std::{cell::Cell, ops::Deref, rc::Rc};

use wry::{
    Rect,
    dpi::{self, LogicalSize},
};

use gpui::{
    App, Bounds, ContentMask, DismissEvent, Element, ElementId, Entity, EventEmitter, FocusHandle,
    Focusable, GlobalElementId, Hitbox, InteractiveElement, IntoElement, LayoutId, MouseDownEvent,
    ParentElement as _, Pixels, Render, Size, Style, Styled as _, Window, canvas, div,
};

/// An owned, UI-thread-local handle to the raw wry webview.
///
/// Cloning this handle prolongs the native webview's lifetime. Dropping the owning [`WebView`]
/// entity hides the child view, but final native destruction waits until all handle and frame
/// clones are dropped. All handles must be dropped before the parent window is destroyed.
#[derive(Clone)]
pub struct WebViewHandle(Rc<wry::WebView>);

impl WebViewHandle {
    /// Get the raw wry webview.
    pub fn raw(&self) -> &wry::WebView {
        &self.0
    }
}

/// A webview based on wry WebView.
///
/// [experimental]
pub struct WebView {
    focus_handle: FocusHandle,
    webview: Rc<wry::WebView>,
    visible: Cell<bool>,
    bounds: Bounds<Pixels>,
    #[cfg(all(feature = "gpui-fast", target_os = "macos"))]
    composition: Option<composition::NativeWebViewSurface>,
}

impl Drop for WebView {
    fn drop(&mut self) {
        self.hide();
        #[cfg(all(feature = "gpui-fast", target_os = "macos"))]
        if let Some(surface) = self.composition.take() {
            surface.release(self.webview.clone());
        }
    }
}

impl WebView {
    /// Create a new WebView from a wry WebView.
    pub fn new(webview: wry::WebView, window: &mut Window, cx: &mut App) -> Self {
        let _ = webview.set_bounds(Rect::default());

        #[cfg(all(feature = "gpui-fast", target_os = "macos"))]
        let composition = match composition::NativeWebViewSurface::new(&webview, window, cx) {
            Ok(surface) => Some(surface),
            Err(error) => {
                log::warn!("WebView composition unavailable: {error:#}");
                None
            }
        };
        #[cfg(not(all(feature = "gpui-fast", target_os = "macos")))]
        let _ = window;

        Self {
            focus_handle: cx.focus_handle(),
            visible: Cell::new(true),
            bounds: Bounds::default(),
            webview: Rc::new(webview),
            #[cfg(all(feature = "gpui-fast", target_os = "macos"))]
            composition,
        }
    }

    /// Set window-relative bounds, including offscreen loading bounds.
    /// On macOS with GPUI Fast, position the managed container and its child together.
    pub fn set_bounds(&self, bounds: Rect) -> wry::Result<()> {
        #[cfg(all(feature = "gpui-fast", target_os = "macos"))]
        if let Some(surface) = &self.composition {
            return surface
                .set_bounds(&self.webview, bounds)
                .map_err(|error| wry::Error::Io(std::io::Error::other(error)));
        }
        self.webview.set_bounds(bounds)
    }

    /// Set visibility and report native errors without discarding the loaded page.
    pub fn set_visible(&self, visible: bool) -> wry::Result<()> {
        // A focus error must not leave a closing native view covering the window.
        let focus_result = if !visible && self.visible.get() {
            self.webview.focus_parent()
        } else {
            Ok(())
        };
        self.webview.set_visible(visible)?;
        #[cfg(all(feature = "gpui-fast", target_os = "macos"))]
        if let Some(surface) = &self.composition {
            surface
                .set_visible(visible)
                .map_err(|error| wry::Error::Io(std::io::Error::other(error)))?;
        }
        self.visible.set(visible);
        focus_result
    }

    /// Show the webview.
    pub fn show(&mut self) {
        let _ = self.set_visible(true);
    }

    /// Hide the webview.
    pub fn hide(&mut self) {
        let _ = self.set_visible(false);
    }

    /// Get whether the webview is visible.
    pub fn visible(&self) -> bool {
        self.visible.get()
    }

    /// Get the current bounds of the webview.
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    /// Go back in the webview history.
    pub fn back(&mut self) -> anyhow::Result<()> {
        Ok(self.webview.evaluate_script("history.back();")?)
    }

    /// Load a URL in the webview.
    pub fn load_url(&mut self, url: &str) {
        let _ = self.webview.load_url(url);
    }

    /// Get the raw wry webview.
    pub fn raw(&self) -> &wry::WebView {
        &self.webview
    }

    /// Get an owned, UI-thread-local handle to the raw wry webview.
    pub fn handle(&self) -> WebViewHandle {
        WebViewHandle(self.webview.clone())
    }
}

impl Deref for WebView {
    type Target = wry::WebView;

    fn deref(&self) -> &Self::Target {
        &self.webview
    }
}

impl Focusable for WebView {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for WebView {}

impl Render for WebView {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity().clone();

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .child({
                let view = cx.entity().clone();
                canvas(
                    move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds),
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full()
            })
            .child(WebViewElement::new(self.webview.clone(), view, window, cx))
    }
}

/// A webview element can display a wry webview.
pub struct WebViewElement {
    parent: Entity<WebView>,
    view: Rc<wry::WebView>,
}

impl WebViewElement {
    /// Create a new webview element from a wry WebView.
    pub fn new(
        view: Rc<wry::WebView>,
        parent: Entity<WebView>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self {
        Self { view, parent }
    }
}

impl IntoElement for WebViewElement {
    type Element = WebViewElement;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for WebViewElement {
    type RequestLayoutState = ();
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            size: Size::full(),
            flex_shrink: 1.,
            ..Default::default()
        };

        // If the parent view is no longer visible, we don't need to layout the webview
        let id = window.request_layout(style, [], cx);
        (id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if !self.parent.read(cx).visible() {
            return None;
        }

        let parent = self.parent.read(cx);
        #[cfg(all(feature = "gpui-fast", target_os = "macos"))]
        if let Some(surface) = &parent.composition {
            surface.set_scale_factor(window.scale_factor());
        }
        let _ = parent.set_bounds(Rect {
            size: dpi::Size::Logical(LogicalSize {
                width: bounds.size.width.into(),
                height: bounds.size.height.into(),
            }),
            position: dpi::Position::Logical(dpi::LogicalPosition::new(
                bounds.origin.x.into(),
                bounds.origin.y.into(),
            )),
        });

        // Create a hitbox to handle mouse event
        Some(window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal))
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        let bounds = hitbox.clone().map(|h| h.bounds).unwrap_or(bounds);
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            let webview = self.view.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, _, _, _| {
                if !bounds.contains(&event.position) {
                    // Click white space to blur the input focus
                    let _ = webview.focus_parent();
                }
            });
        });
    }
}
