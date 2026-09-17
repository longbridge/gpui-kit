---
title: Root View
description: Use the Root view to enable themes, notifications, dialogs, and other GPUI Component features in a window.
example: false
---

# Root View

The [Root] component for as the root provider of GPUI Component features in a window. We must to use [Root] as the **first level child** of a window to enable GPUI Component features.

This is important, if we don't use [Root] as the first level child of a window, there will have some unexpected behaviors.

This complete **Tested consumer recipe** is compiled from the isolated `gpui-kit` consumer workspace. It initializes GPUI Kit before creating a window, makes `Root` the first-level view, and renders every Root overlay layer.

<!-- recipe:bootstrap:start -->
```rust
use gpui_kit::component::Root;
use gpui_kit::{
    AppContext as _, Context, IntoElement, ParentElement as _, Render, Styled as _, Window,
    WindowOptions, div,
};

pub fn run() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            cx.spawn(async move |cx| {
                cx.open_window(WindowOptions::default(), |window, cx| {
                    let view = cx.new(|_| BootstrapView);
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("failed to open window");
            })
            .detach();
        });
}

struct BootstrapView;

impl Render for BootstrapView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child("My application")
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
```
<!-- recipe:bootstrap:end -->

## Window Border

By default, [Root] renders GPUI Component's client-side window border wrapper. For
layer-shell fullscreen windows or other surfaces that should not render this
wrapper, disable it with `bordered(false)`:

```rs
cx.new(|cx| Root::new(view, window, cx).bordered(false))
```

## Overlays

We have dialogs, sheets, notifications, we need placement for them to show, so [Root] provides methods to render these overlays:

- [Root::render_dialog_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_dialog_layer) - Render the current opened modals.
- [Root::render_sheet_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_sheet_layer) - Render the current opened drawers.
- [Root::render_notification_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_notification_layer) - Render the notification list.

Put these layers in the `render` method of the first-level view below `Root`; the tested recipe above shows their required `window, cx` arguments.

:::tip
Here the example we used `children` method, it because if there is no opened dialogs, sheets, notifications, these methods will return `None`, so GPUI will not render anything.
:::

[Root]: https://docs.rs/gpui-component/latest/gpui_component/root/struct.Root.html
