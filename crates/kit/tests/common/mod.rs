//! Fixtures use the production window entry point with deterministic test bounds.
#![allow(dead_code)]
use gpui_kit::{
    AnyWindowHandle, App, AppContext, Bounds, Context, Entity, Pixels, Point, Render, Size,
    TestAppContext, Window, WindowBounds, WindowHandle, WindowOptions, base::Root,
};

pub fn open_window<V: Render>(
    cx: &mut TestAppContext,
    size: Option<Size<Pixels>>,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> (WindowHandle<Root>, Entity<V>) {
    cx.update(|cx| {
        let bounds = size
            .map(|size| Bounds {
                origin: Point::default(),
                size,
            })
            .unwrap_or_else(|| Bounds::maximized(None, cx));
        let (window, content) = gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            cx,
            build,
        )
        .expect("open test window");
        (window.downcast::<Root>().expect("Base Root"), content)
    })
}

pub fn update_content<V: Render, R>(
    window: impl Into<AnyWindowHandle>,
    content: &Entity<V>,
    cx: &mut TestAppContext,
    update: impl FnOnce(&mut V, &mut Window, &mut Context<V>) -> R,
) -> gpui_kit::Result<R> {
    cx.update_window(window.into(), |_, window, cx| {
        content.update(cx, |view, cx| update(view, window, cx))
    })
}
