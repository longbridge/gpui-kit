use std::cell::Cell;

use ::windows::Win32::{
    Foundation::HWND,
    Graphics::Gdi::{CreateRectRgn, DeleteObject, HRGN, SetWindowRgn},
    UI::HiDpi::GetDpiForWindow,
};
use anyhow::{Context as _, Result};
use gpui::{Bounds, Pixels};
use wry::{
    Rect, WebViewExtWindows as _,
    dpi::{LogicalPosition, LogicalSize},
};

type ClipRect = [i32; 4];

#[derive(Default)]
pub(super) struct NativeClip(Cell<Option<ClipRect>>);

impl NativeClip {
    pub(super) fn set_geometry(
        &self,
        webview: &wry::WebView,
        document: Bounds<Pixels>,
        visible: Bounds<Pixels>,
    ) -> Result<()> {
        let hwnd = host_window(webview)?;
        // SAFETY: Wry owns the child HWND on this UI thread.
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        anyhow::ensure!(dpi != 0, "cannot read WebView host DPI");
        let (bounds, clip) = window_geometry(document, visible, f64::from(dpi) / 96.);
        webview.set_bounds(bounds)?;
        self.apply(hwnd, clip)
    }

    pub(super) fn clear(&self, webview: &wry::WebView) -> Result<()> {
        if self.0.get().is_some() {
            self.apply(host_window(webview)?, None)?;
        }
        Ok(())
    }

    fn apply(&self, hwnd: HWND, clip: Option<ClipRect>) -> Result<()> {
        if self.0.get() == clip {
            return Ok(());
        }
        // SAFETY: Coordinates are local to the Wry child HWND and are in device pixels.
        // On success Windows owns the region; on failure the caller still owns it.
        unsafe {
            let region = if let Some([left, top, right, bottom]) = clip {
                let region = CreateRectRgn(left, top, right, bottom);
                anyhow::ensure!(!region.is_invalid(), "cannot create WebView clip region");
                Some(region)
            } else {
                None::<HRGN>
            };
            if SetWindowRgn(hwnd, region, true) == 0 {
                if let Some(region) = region {
                    let _ = DeleteObject(region.into());
                }
                anyhow::bail!("cannot set WebView clip region");
            }
        }
        self.0.set(clip);
        Ok(())
    }
}

fn host_window(webview: &wry::WebView) -> Result<HWND> {
    let mut hwnd = HWND::default();
    // SAFETY: Wry creates the controller with its own child HWND as the parent.
    unsafe { webview.controller().ParentWindow(&mut hwnd) }.context("cannot get WebView host")?;
    anyhow::ensure!(!hwnd.0.is_null(), "WebView has no host window");
    Ok(hwnd)
}

fn window_geometry(
    document: Bounds<Pixels>,
    visible: Bounds<Pixels>,
    scale: f64,
) -> (Rect, Option<ClipRect>) {
    let position = |x: Pixels, y: Pixels| {
        LogicalPosition::new(f64::from(x), f64::from(y)).to_physical::<i32>(scale)
    };
    let origin = position(document.left(), document.top());
    let size = LogicalSize::new(
        f64::from(document.size.width),
        f64::from(document.size.height),
    )
    .to_physical::<i32>(scale);
    let start = position(visible.left(), visible.top());
    let end = position(visible.right(), visible.bottom());
    // Wry rounds the window origin and extent independently. For a fully visible
    // far edge, use that extent instead of subtracting two rounded positions.
    let clip = [
        (start.x - origin.x).clamp(0, size.width),
        (start.y - origin.y).clamp(0, size.height),
        if visible.right() >= document.right() {
            size.width
        } else {
            (end.x - origin.x).clamp(0, size.width)
        },
        if visible.bottom() >= document.bottom() {
            size.height
        } else {
            (end.y - origin.y).clamp(0, size.height)
        },
    ];
    (
        Rect {
            position: origin.into(),
            size: size.into(),
        },
        (clip != [0, 0, size.width, size.height]).then_some(clip),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px, size};
    use wry::dpi::PhysicalSize;

    #[test]
    fn fractional_dpi_keeps_the_document_size_and_fully_visible_edge() {
        let origin = point(px(0.5), px(0.5));
        let document = Bounds::new(origin, size(px(80.5), px(40.5)));
        let visible = Bounds::new(point(px(10.5), origin.y), size(px(70.5), px(30.)));
        let (bounds, clip) = window_geometry(document, visible, 1.25);
        assert_eq!(
            bounds.size.to_physical::<i32>(1.25),
            PhysicalSize::new(101, 51)
        );
        assert_eq!(clip, Some([12, 0, 101, 37]));
        assert_eq!(window_geometry(document, document, 1.25).1, None);
    }
}
