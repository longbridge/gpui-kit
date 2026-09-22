---
title: Root View
description: Use the Root view to enable themes, notifications, dialogs, and other GPUI Component features in a window.
example: false
---

# Root View

[Root] is the Base-owned root view of every GPUI Kit window. `gpui_kit::open_window` is the application entry point and always creates this type. Base does not expose a separate window helper; `component::Root` re-exports the Base type.

Base owns the content and overlay host, keyboard traversal and selection copying. Calling `gpui_component::init` explicitly registers the styled window extension: dialogs, sheets, notifications, tooltips, menus, touch selection, theme and window chrome. Initialize it before creating windows. A Base-only application calls `gpui_base::init` and needs no Component or Kit dependency. Cargo feature unification does not change the root type.

This complete **Tested consumer recipe** is compiled from the isolated `gpui-kit` consumer workspace. It initializes GPUI Kit, then opens a window whose root is a `Root` wrapping the application view.

<!-- recipe:bootstrap:start -->
```rust
use gpui_kit::{
    AppContext as _, Context, IntoElement, ParentElement as _, Render, Styled as _, Window,
    WindowOptions, div,
};

pub fn run() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            // The window's root view is a `Root` wrapping the view, which
            // renders dialogs, sheets and notifications above it.
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
                cx.new(|_| BootstrapView)
            })
            .expect("failed to open window");
        });
}

struct BootstrapView;

impl Render for BootstrapView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("My application")
    }
}
```
<!-- recipe:bootstrap:end -->

`gpui_kit::open_window` is `cx.open_window` plus the `Root` wrapper. `Root` must
be the window's root view so window-level facilities such as dialogs, sheets,
notifications, focus traversal, and text selection remain available. Client-side
window borders are selected from the window's decoration mode; server-decorated
and layer-shell windows do not require Root configuration.

`open_window` returns the window and the view, so a view that must be built inside the window (it owns an `InputState`, say) can still be kept:

```rust
let (window, editor) = gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
    cx.new(|cx| Editor::new(window, cx))
})?;
```

## Closing windows and quitting

Applications define their own quit and close-window actions and key bindings. `gpui_kit::init` does not install them. Handle unsaved changes and any confirmation in the application before closing a window or quitting.

## Overlays

`Root` always mounts the dialog, sheet and notification layers above application content. Applications only call `window.open_dialog`, `window.open_sheet` or `window.push_notification`; no manual mounting or configuration is needed. Child view caching does not affect overlay rendering.

### Migrating to 0.7.0

`Root::render_dialog_layer`, `Root::render_sheet_layer` and `Root::render_notification_layer` have been removed. Delete their calls and the corresponding `.children(...)` expressions from application views. Previously customized layer positions now use the window-level Root's overlay placement.

[Root]: https://docs.rs/gpui-base/latest/gpui_base/struct.Root.html
