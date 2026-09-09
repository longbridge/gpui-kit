---
title: Scrollbar
description: Add a styled, animated scrollbar to GPUI scroll views, lists, and custom viewports.
order: 24
---

# Scrollbar

`Scrollbar` is a custom-painted scrollbar connected to a GPUI scroll handle. It
supports vertical, horizontal, and two-axis viewports; track clicks; thumb
dragging; configurable visibility modes; typed paint styles; reduced motion;
and reversible visibility and width transitions.

`gpui-base` owns the interaction and transition lifecycle. Your application or
design-system layer owns colors, geometry, timing, and entrance choreography.

## Run the example

The native showcase and WASM preview use the same implementation:

```bash
cargo run -p gpui-base-examples -- scrollbar
```

The source is available in
[`components/scrollbar.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/scrollbar.rs).

## Imports

```rust
use std::time::Duration;

use gpui_kit::{div, px, rgb, ScrollHandle, Styled as _};
use gpui_kit::base::{
    Scrollbar, ScrollbarAxis, ScrollbarEntrance, ScrollbarMode,
    ScrollbarMotion, ScrollbarStyles, ScrollbarTheme, Theme,
};
```

## Basic usage

Keep the `ScrollHandle` on persistent view state. Attach it to the scrollable
content with `track_scroll`, then overlay a `Scrollbar` in the same relative
container.

```rust
pub struct ActivityList {
    scroll_handle: ScrollHandle,
}

impl ActivityList {
    pub fn new() -> Self {
        Self {
            scroll_handle: ScrollHandle::new(),
        }
    }

    fn render_list(&self) -> impl gpui_kit::IntoElement {
        div()
            .relative()
            .size_full()
            .overflow_scroll()
            .track_scroll(&self.scroll_handle)
            .child(div().children((1..=100).map(|row| {
                div().h_8().px_2().child(format!("Activity {row}"))
            })))
            .child(Scrollbar::new(&self.scroll_handle))
    }
}
```

`Scrollbar::new` enables both axes. Use an axis-specific constructor when the
container scrolls in only one direction:

```rust
Scrollbar::vertical(&scroll_handle);
Scrollbar::horizontal(&scroll_handle);

Scrollbar::new(&scroll_handle).axis(ScrollbarAxis::Vertical);
```

The scrollbar is an absolute overlay. Its layout and hitboxes stay fixed while
the painted track and thumb animate, so entrance motion does not move content or
change the interaction geometry.

## Visibility modes

Set a mode on one scrollbar, or omit `.mode(...)` to use the global
`ScrollbarTheme` mode.

```rust
Scrollbar::vertical(&scroll_handle).mode(ScrollbarMode::Scrolling);
Scrollbar::vertical(&scroll_handle).mode(ScrollbarMode::Hover);
Scrollbar::vertical(&scroll_handle).mode(ScrollbarMode::Always);
```

| Mode        | Behavior                                                                                                                                                              |
| ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Scrolling` | Appears after scrolling or dragging. A visible scrollbar stays visible while hovered; leaving starts a fresh idle hold. Hover cannot reveal a fully hidden scrollbar. |
| `Hover`     | Appears when the pointer enters the scrollbar track.                                                                                                                  |
| `Always`    | Remains visible and skips visibility transitions.                                                                                                                     |

All modes use a 6 px resting thumb by default. Track hover keeps that width.
Thumb hover and active dragging target the 8 px active width. Width changes use
the configured `expand` duration.

Hidden track and thumb clicks are ignored. In `Scrolling` mode, a hidden thumb
also does not retain a latent hover state that could expand it on the next
scroll.

## Configure the global theme

`ScrollbarTheme` uses private fields with consuming builders and readers. Set
it during application initialization or when your design-system theme changes.

```rust
fn install_scrollbar_theme(cx: &mut gpui_kit::App) {
    let styles = ScrollbarStyles::default()
        .track(|style| {
            style
                .width(px(16.))
                .bg(rgb(0x000000).alpha(0.08))
        })
        .track_hover(|style| {
            style.bg(rgb(0x000000).alpha(0.12))
        })
        .track_active(|style| {
            style.bg(rgb(0x000000).alpha(0.16))
        })
        .thumb(|style| {
            style
                .width(px(6.))
                .inset(px(4.))
                .radius(px(3.))
                .min_length(px(48.))
                .bg(rgb(0x737373))
        })
        .thumb_hover(|style| {
            style.width(px(8.)).bg(rgb(0x525252))
        })
        .thumb_active(|style| {
            style.width(px(8.)).bg(rgb(0x404040))
        });

    let motion = ScrollbarMotion::default()
        .with_idle(Duration::from_secs(2))
        .with_enter(Duration::from_millis(300))
        .with_exit(Duration::from_millis(500))
        .with_expand(Duration::from_millis(300))
        .with_entrance(ScrollbarEntrance::Fade)
        .with_thumb_hover_entrance(ScrollbarEntrance::SlideAndFade);

    Theme::global_mut(cx).scrollbar = ScrollbarTheme::new()
        .with_mode(ScrollbarMode::Scrolling)
        .with_motion(motion)
        .with_styles(styles);
}
```

The same values can be inspected without exposing the theme's fields:

```rust
let scrollbar = &Theme::global(cx).scrollbar;
let mode = scrollbar.mode();
let motion = scrollbar.motion();
let styles = scrollbar.styles();
```

## Motion behavior

Base ships without product motion. `ScrollbarMotion::default()` uses a 2-second
behavioral idle hold, but its `enter`, `exit`, and `expand` durations are zero.
An application that does not install motion therefore gets immediate visibility
and width changes.

The example theme above produces this choreography:

| Trigger                               | Entrance                                                         |
| ------------------------------------- | ---------------------------------------------------------------- |
| Scroll in `Scrolling` or `Hover` mode | `entrance`: fade in place                                        |
| Track hover in `Hover` mode           | `entrance`: fade in place                                        |
| Thumb hover in `Hover` mode           | `thumb_hover_entrance`: slide from the nearest edge while fading |
| `Always` mode                         | Immediate; visibility motion is skipped                          |

For `SlideAndFade`, a vertical scrollbar enters from the right and a horizontal
scrollbar enters from the bottom. Opacity uses linear entrance progress;
position uses cubic ease-out. Exit opacity and position use cubic ease-in.

An interrupted transition samples its current opacity and position before
changing direction. A zero duration adopts the target immediately, including
when a transition is already running.

GPUI's reduced-motion preference also sets visibility and width durations to
zero. You do not need a separate reduced-motion theme.

## Per-instance styles

Use `.styles(...)` to override the global styles for one scrollbar. Instance
styles take precedence over theme defaults.

```rust
Scrollbar::vertical(&scroll_handle).styles(|styles| {
    styles
        .track(|style| style.width(px(14.)).bg(rgb(0xf5f5f5)))
        .track_hover(|style| style.bg(rgb(0xe5e5e5)))
        .thumb(|style| {
            style
                .width(px(6.))
                .inset(px(3.))
                .radius(px(3.))
                .min_length(px(40.))
                .bg(rgb(0x737373))
        })
        .thumb_hover(|style| style.width(px(8.)).bg(rgb(0x525252)))
        .thumb_active(|style| style.width(px(8.)).bg(rgb(0x404040)))
})
```

`ScrollbarTrackStyle` supports `bg`, `border_color`, and `width`.
`ScrollbarThumbStyle` supports `bg`, `width`, `inset`, `radius`, and
`min_length`.

## Custom viewport geometry

The viewport normally comes from `ScrollbarHandle::viewport_bounds`. Two
overrides support composite or custom-painted controls:

```rust
Scrollbar::vertical(&scroll_handle)
    .viewport_bounds(editor_content_bounds);

Scrollbar::vertical(&scroll_handle)
    .viewport_from_layout();
```

Use `viewport_bounds` when your painted viewport differs from the handle's
layout bounds. Use `viewport_from_layout` when a positioned overlay container
already represents the exact viewport, such as a table body below a fixed
header.

Override the content size only when the handle cannot report the complete
scrollable extent:

```rust
Scrollbar::vertical(&scroll_handle)
    .scroll_size(gpui_kit::size(px(800.), px(4_000.)));
```

## Custom scroll handles

`ScrollHandle`, `UniformListScrollHandle`, and `ListState` implement
`ScrollbarHandle`. Custom scroll containers can implement the same trait:

```rust
use gpui_kit::{Bounds, Pixels, Point, Size};
use gpui_kit::base::ScrollbarHandle;

impl ScrollbarHandle for MyScrollState {
    fn viewport_bounds(&self) -> Bounds<Pixels> {
        self.viewport_bounds()
    }

    fn offset(&self) -> Point<Pixels> {
        self.offset()
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        self.set_offset(offset);
    }

    fn content_size(&self) -> Size<Pixels> {
        self.content_size()
    }

    fn start_drag(&self) {
        self.set_scrollbar_dragging(true);
    }

    fn end_drag(&self) {
        self.set_scrollbar_dragging(false);
    }
}
```

`start_drag` and `end_drag` are optional. Use them when the scroll container
needs to suspend snapping, selection, or another behavior during thumb drag.
Only the actively dragged axis receives `end_drag` on mouse-up.

## Stable identity

`Scrollbar::new`, `vertical`, and `horizontal` derive an element ID from their
call site. Set an explicit stable ID when the same call site produces multiple
independent scrollbars:

```rust
Scrollbar::vertical(&scroll_handle).id(("activity-list", panel_id));
```

A stable identity preserves retained visibility and width animation state across
renders.

## Complete showcase source

The runnable example is embedded directly from the shared Rust source:

<<< ../../../crates/base/examples/showcase/components/scrollbar.rs{rust}

## Accessibility and interaction checklist

- Keep wheel, trackpad, and keyboard scrolling available on the underlying
  viewport.
- Preserve the default full-track interaction hitbox even when the painted
  thumb is narrow.
- Give the thumb adequate contrast in normal, hover, and active states.
- Do not move layout or hitboxes to implement entrance animation.
- Test `Scrolling`, `Hover`, and `Always` with reduced motion enabled.
- Test vertical, horizontal, and two-axis overflow independently.
