---
title: Popover
description: A floating overlay that displays rich content relative to a trigger element.
---

# Popover

Popover component for displaying floating content that appears when interacting with a trigger element. Supports multiple positioning options, custom content, different trigger methods, and automatic dismissal behaviors. Perfect for tooltips, menus, forms, and other contextual information.

## Import

```rust
use gpui_kit::component::popover::{Popover};
```

## Usage

### Basic Popover

:::info
Any element that implements [Selectable] can be used as a trigger, for example, a [Button].

Any element that implements [RenderOnce] or [Render] can be used as popover content, use `.child(...)` to add children directly.
:::

```rust
use gpui_kit::ParentElement as _;
use gpui_kit::component::{button::Button, popover::Popover};

Popover::new("basic-popover")
    .trigger(Button::new("trigger").label("Click me").outline())
    .child("Hello, this is a popover!")
    .child("It appears when you click the button.")
```

### Popover with Custom Positioning

The `anchor` method controls how the popover attaches to the trigger, using the [`Anchor`] type.

Imagine the popover has a pointer tip (like a speech bubble's tail). The anchor is where that tip sits relative to the trigger — `Anchor::TopLeft` places it at the trigger's top-left corner, `Anchor::BottomRight` at the bottom-right, and so on. The popover then hangs off that point.

For example, `Anchor::TopLeft` places the popover just below the trigger, left-aligned to it:

```text
[ Trigger ]
┌──────────────┐
│   Popover    │
└──────────────┘
```

```rust
use gpui_kit::component::Anchor;

// Anchored to the trigger's top corners
Popover::new("top-left")
    .anchor(Anchor::TopLeft)
    .trigger(Button::new("btn").label("Top Left").outline())
    .child("Anchored to the trigger's top-left")

Popover::new("top-center")
    .anchor(Anchor::TopCenter)
    .trigger(Button::new("btn").label("Top Center").outline())
    .child("Anchored to the trigger's top-center")

Popover::new("top-right")
    .anchor(Anchor::TopRight)
    .trigger(Button::new("btn").label("Top Right").outline())
    .child("Anchored to the trigger's top-right")

// Anchored to the trigger's bottom corners
Popover::new("bottom-left")
    .anchor(Anchor::BottomLeft)
    .trigger(Button::new("btn").label("Bottom Left").outline())
    .child("Anchored to the trigger's bottom-left")

Popover::new("bottom-center")
    .anchor(Anchor::BottomCenter)
    .trigger(Button::new("btn").label("Bottom Center").outline())
    .child("Anchored to the trigger's bottom-center")

Popover::new("bottom-right")
    .anchor(Anchor::BottomRight)
    .trigger(Button::new("btn").label("Bottom Right").outline())
    .child("Anchored to the trigger's bottom-right")
```

### View in Popover

You can add any `Entity<T>` that implemented [Render] as the popover content.

```rust
let view = cx.new(|_| MyView::new());

Popover::new("form-popover")
    .anchor(Anchor::BottomLeft)
    .trigger(Button::new("show-form").label("Open Form").outline())
    .child(view.clone())
```

### Add content by `content` method

The `content` method allows you to create more complex popover content using a closure. This is useful when
you need to build dynamic content or need access to the popover's context.

This method will let us to have `&mut PopoverState`, `&mut Window` and `&mut Context<PopoverState>` parameters in the
closure is to allow you to interact with the popover's state and the overall application context if needed.

:::warning
This `content` callback will called every time on render the popover.
So, you should avoid creating new elements or entities in the content closure
or other heavy operations that may impact performance.
:::

And `content` will works with `child`, `children` methods together.

```rust
use gpui_kit::ParentElement as _;
use gpui_kit::component::popover::Popover;

Popover::new("complex-popover")
    .anchor(Anchor::BottomLeft)
    .trigger(Button::new("complex").label("Complex Content").outline())
    .content(|_, _, _| {
        div()
            .child("This popover has complex content.")
            .child(
                Button::new("action-btn")
                    .label("Perform Action")
                    .outline()
            )
    })
```

### Right-Click Popover

Sometimes you may want to show a popover on right-click, for example, to create a special your ownen context menu. The `mouse_button` method allows you to specify which mouse button triggers the popover.

```rust
use gpui_kit::MouseButton;

Popover::new("context-menu")
    .anchor(Anchor::BottomRight)
    .mouse_button(MouseButton::Right)
    .trigger(Button::new("right-click").label("Right Click Me").outline())
    .child("Context Menu")
    .child(Separator::horizontal())
    .child("This is a custom context menu.")
```

### Dismiss Popover manually

If you want to dismiss the popover programmatically from within the content, you can emit a `DismissEvent`. In this case, you should use `content` method to create the popover content so you have access to the `cx: &mut Context<PopoverState>`.

```rust
use gpui_kit::component::{DismissEvent, popover::Popover};

Popover::new("dismiss-popover")
    .trigger(Button::new("dismiss").label("Dismiss Popover").outline())
    .content(|_, cx| {
        div()
            .child("Click the button below to dismiss this popover.")
            .child(
                Button::new("close-btn")
                    .label("Close Popover")
                    .on_click(cx.listener(|_, _, _, cx| {
                        // NOTE: Here `cx` is `&mut Context<PopoverState>` type, so we can emit DismissEvent.
                        cx.emit(DismissEvent);
                    }))
            )
    })
```

### Styling Popover

Like the others components in GPUI Component, the `appearance(false)` method can be used to disable the default styling of the popover, allowing you to fully customize its appearance.

And the `Popover` has implemented the [Styled] trait, so you can use all the styling methods provided by GPUI to style the popover content as you like.

```rust
// For custom styled popovers or when you want full control
Popover::new("custom-popover")
    .appearance(false)
    .trigger(Button::new("custom").label("Custom Style"))
    .bg(cx.theme().accent)
    .text_color(cx.theme().accent_foreground)
    .p_6()
    .rounded_xl()
    .shadow_2xl()
    .child("Fully custom styled popover")
```

### Control Open State

There have `open` and `on_open_change` methods to control the open state of the popover programmatically.

This is useful when you want to synchronize the popover's open state with other UI elements or application state.

:::tip
When you use `open` to control the popover's open state, that means you have take full control of it,
so you need to update the state in `on_open_change` callback to keep the popover working correctly.
:::

```rust
use gpui_kit::component::popover::Popover;

struct MyView {
    popover_open: bool,
}

Popover::new("controlled-popover")
    .open(self.open)
    .on_open_change(cx.listener(|this, open: &bool, _, cx| {
        this.popover_open = *open;
        cx.notify();
    }))
    .trigger(Button::new("control-btn").label("Control Popover").outline())
    .child("This popover's open state is controlled programmatically.")
```

### Default Open

The `default_open` method allows you to set the initial open state of the popover when it is first rendered.

Please note that if you use the `open` method to control the popover's open state, the `default_open` setting will be ignored.

```rust
use gpui_kit::component::popover::Popover;

Popover::new("default-open-popover")
    .default_open(true)
    .trigger(Button::new("default-open-btn").label("Default Open").outline())
    .child("This popover is open by default when first rendered.")
```

[Button]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.Button.html
[Selectable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Selectable.html
[Render]: https://docs.rs/gpui/latest/gpui/trait.Render.html
[RenderOnce]: https://docs.rs/gpui/latest/gpui/trait.RenderOnce.html
[Styled]: https://docs.rs/gpui/latest/gpui/trait.Styled.html
[`Anchor`]: https://docs.rs/gpui-component/latest/gpui_component/enum.Anchor.html
