//! AppKit adapter for gpui-fast's native window composition.
//! The existing Wry parent stays unchanged for focus_parent(); only WKWebView
//! is reattached. All positioning goes through the surface instead of Wry's
//! original parent coordinates.
use std::{cell::Cell, rc::Rc};

use anyhow::{Context as _, Result};
use gpui::{AnyWindowHandle, AsyncApp, Bounds, Window, WindowCompositionSurface, point, px, size};
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use objc2_foundation::{NSPoint, NSRect, NSSize};
use wry::WebViewExtMacOS as _;

pub(super) struct NativeWebViewSurface {
    surface: WindowCompositionSurface,
    original_parent: Retained<NSView>,
    scale_factor: Cell<f32>,
    window: AnyWindowHandle,
    cx: AsyncApp,
}

impl NativeWebViewSurface {
    pub(super) fn new(webview: &wry::WebView, window: &Window, cx: &gpui::App) -> Result<Self> {
        let composition = window.enable_window_composition()?;
        let surface = composition.create_native_surface()?;
        let attached = (|| {
            let handle = surface.platform_surface()?.platform_handle()?;
            let parent = *handle
                .downcast::<usize>()
                .map_err(|_| anyhow::anyhow!("native surface is not an AppKit view"))?;
            anyhow::ensure!(parent != 0, "native surface has no AppKit view");
            let view = webview.webview();
            // SAFETY: Wry and the composition surface own these AppKit views;
            // this constructor runs on GPUI's main/UI thread.
            unsafe {
                let original_parent = view.superview().context("WebView has no parent")?;
                (&*(parent as *const NSView)).addSubview(&view);
                Ok(original_parent)
            }
        })();
        match attached {
            Ok(original_parent) => Ok(Self {
                surface,
                original_parent,
                scale_factor: Cell::new(window.scale_factor()),
                window: window.window_handle(),
                cx: cx.to_async(),
            }),
            Err(error) => {
                composition.remove_surface(surface.id())?;
                Err(error)
            }
        }
    }

    pub(super) fn set_scale_factor(&self, scale_factor: f32) {
        self.scale_factor.set(scale_factor);
    }

    pub(super) fn set_bounds(&self, webview: &wry::WebView, rect: wry::Rect) -> Result<()> {
        let scale = self.scale_factor.get();
        let (position, extent) = logical_bounds(rect, scale);
        let bounds = Bounds::new(
            point(px(position.x as f32), px(position.y as f32)),
            size(px(extent.width as f32), px(extent.height as f32)),
        );
        self.surface
            .platform_surface()?
            .set_bounds(bounds.to_device_pixels(scale))?;
        // WKWebView remains owned by Wry, on its creating UI thread.
        // Its frame is local to the managed surface, whose frame is window-relative.
        webview.webview().setFrame(NSRect::new(
            NSPoint::new(0., 0.),
            NSSize::new(extent.width, extent.height),
        ));
        Ok(())
    }

    pub(super) fn set_visible(&self, visible: bool) -> Result<()> {
        self.surface.platform_surface()?.set_visible(visible)
    }

    pub(super) fn release(self, webview: Rc<wry::WebView>) {
        // Entity destruction can occur while App is borrowed. Run cleanup on the
        // foreground executor, retaining Wry until the managed surface is removed.
        self.cx
            .clone()
            .spawn(async move |cx| {
                self.original_parent.addSubview(&webview.webview());
                let result = self.window.update(cx, |_, window, _| {
                    window
                        .enable_window_composition()?
                        .remove_surface(self.surface.id())
                });
                if let Ok(Err(error)) = result {
                    log::warn!("cannot release WebView composition surface: {error:#}");
                }
            })
            .detach();
    }
}

// Staging positions must remain window-relative at every display scale.
fn logical_bounds(
    rect: wry::Rect,
    scale: f32,
) -> (wry::dpi::LogicalPosition<f64>, wry::dpi::LogicalSize<f64>) {
    (
        rect.position.to_logical(f64::from(scale)),
        rect.size.to_logical(f64::from(scale)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use wry::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};

    #[test]
    fn retina_physical_bounds_keep_the_child_inside_its_container() {
        let (position, extent) = logical_bounds(
            wry::Rect {
                position: PhysicalPosition::new(400, 160).into(),
                size: PhysicalSize::new(1280, 960).into(),
            },
            2.,
        );
        assert_eq!(position, LogicalPosition::new(200., 80.));
        assert_eq!(extent, LogicalSize::new(640., 480.));
    }

    #[test]
    fn offscreen_logical_loading_bounds_do_not_shift_on_retina() {
        let rect = wry::Rect {
            position: LogicalPosition::new(-20000., -20000.).into(),
            size: LogicalSize::new(640., 480.).into(),
        };
        for scale in [1., 2.] {
            let (position, extent) = logical_bounds(rect, scale);
            assert_eq!(position, LogicalPosition::new(-20000., -20000.));
            assert_eq!(extent, LogicalSize::new(640., 480.));
        }
    }
}
