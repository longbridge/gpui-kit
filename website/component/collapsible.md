---
title: Collapsible
description: An interactive element which expands/collapses.
---

# Collapsible

An interactive element which expands/collapses.

## Import

```rust
use gpui_kit::component::collapsible::Collapsible;
```

## Usage

### Basic Use

```rust
Collapsible::new()
    .max_w_128()
    .gap_1()
    .open(self.open)
    .child(
        "This is a collapsible component. \
        Click the header to expand or collapse the content.",
    )
    .content(
        "This is the full content of the Collapsible component. \
        It is only visible when the component is expanded. \n\
        You can put any content you like here, including text, images, \
        or other UI elements.",
    )
    .child(
        h_flex().justify_center().child(
            Button::new("toggle1")
                .icon(IconName::ChevronDown)
                .label("Show more")
                .when(open, |this| {
                    this.icon(IconName::ChevronUp).label("Show less")
                })
                .xsmall()
                .link()
                .on_click({
                    cx.listener(move |this, _, _, cx| {
                        this.open = !this.open;
                        cx.notify();
                    })
                }),
        ),
    )
```

We can use `open` method to control the collapsed state. If false, the `content` method added child elements will be hidden.

### Animated reveal

Opt into a reversible, measured height reveal with a stable motion ID:

```rust
Collapsible::new()
    .motion_id("advanced-options")
    .open(self.open)
    .content(options)
```

The content remains mounted while closed so it can be measured and immediately reverse if toggled mid-animation. Without `motion_id`, the component keeps the immediate mount/unmount behavior. See the [GPUI Base Motion guide](/base/motion) for timing, reduced-motion, and performance details.

[Collapsible]: https://docs.rs/gpui-component/latest/gpui_component/collapsible/struct.Collapsible.html
