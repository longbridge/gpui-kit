---
title: Geometry
description: Work with GPUI's typed coordinates, layout lengths, and colors in practical UI code.
order: -2.8
---

# Geometry and color

GPUI uses types to say what a number *means*. A coordinate is a `Point<T>`, an extent is a `Size<T>`, and a rectangle is a `Bounds<T>`. The `T` says which unit the components use. Layout accepts lengths that may still need a parent size; drawing and hit testing usually use resolved `Pixels`. Colors distinguish a hue based representation (`Hsla`) from direct channels (`Rgba`). GPUI Kit reexports GPUI, so application examples can start with `use gpui_kit::*;`.

## Points, sizes, and bounds

| Type | Fields | Meaning |
| --- | --- | --- |
| `Point<T>` | `x`, `y` | A position in a coordinate space. |
| `Size<T>` | `width`, `height` | An extent, without a position. |
| `Bounds<T>` | `origin: Point<T>`, `size: Size<T>` | An axis aligned rectangle. |

The free functions `point(x, y)`, `size(width, height)`, and `bounds(origin, size)` infer `T` from their arguments. They also work with ordinary numeric types, but UI geometry normally uses `Pixels`:

```rust
use gpui_kit::*;

let frame: Bounds<Pixels> = bounds(
    point(px(20.), px(40.)),
    size(px(240.), px(80.)),
);
assert_eq!(frame.right(), px(260.));
assert_eq!(frame.bottom(), px(120.));
assert!(frame.contains(&point(px(20.), px(40.))));
assert!(!frame.contains(&point(px(260.), px(40.))));

let midpoint: Point<Pixels> = frame.center();
let local = point(px(35.), px(55.)).relative_to(&frame.origin);
assert_eq!(local, point(px(15.), px(15.)));
```

`right()` and `bottom()` add the extent to the origin. `contains()` includes the top and left edges but excludes the bottom and right edges, so adjacent rectangles do not both claim a point on their shared boundary. `center()`, `intersects()`, `Bounds::from_corners(...)`, and `Bounds::centered_at(...)` help with alignment and placement. A local position and a window position can both be `Point<Pixels>`: the type checks the unit, while your code must still track which origin it uses. Add the bounds origin when painting locally measured geometry in window coordinates; subtract it when interpreting a pointer position within an element.

These values typically appear after layout. A custom [`Element`](./element) receives `Bounds<Pixels>` in `prepaint` and uses the same resolved geometry for hitboxes and later [painting](./paint). A bounds value is geometry, not an interactive region by itself.

## Choose a coordinate origin

`Point<Pixels>` records a unit, not a coordinate system. In a window, the top-left of the window content is the usual origin; an element's own top-left is a different origin. State the space in variable names when both appear in one calculation:

```rust
use gpui_kit::*;

let element_bounds = bounds(point(px(100.), px(60.)), size(px(80.), px(40.));
let pointer_in_window = point(px(125.), px(75.));
let pointer_in_element = pointer_in_window.relative_to(&element_bounds.origin);
assert_eq!(pointer_in_element, point(px(25.), px(15.)));

let marker_in_element = point(px(10.), px(8.));
let marker_in_window = element_bounds.origin + marker_in_element;
assert_eq!(marker_in_window, point(px(110.), px(68.)));
```

The `bounds` passed to a custom element's `prepaint` and `paint` is already positioned in window coordinates. A pointer event's `position` is also in window coordinates. Test `element_bounds.contains(&pointer_in_window)` directly; convert to local coordinates only for work such as locating a character or handle *inside* the element. Do not add `element_bounds.origin` a second time to a rectangle already based on those bounds. Conversely, a local point cannot be compared directly with a window-space hitbox even though both have type `Point<Pixels>`.

## From layout to input and drawing

The phases answer different questions:

| Phase | Available geometry | Responsibility |
| --- | --- | --- |
| `request_layout` | Style lengths and layout nodes; some lengths still depend on the parent. | Return a `LayoutId` for the layout engine to solve. |
| `prepaint` | Resolved `Bounds<Pixels>` for this frame. | Prepare geometry and, when needed, call `window.insert_hitbox(bounds, HitboxBehavior::Normal)`. |
| `paint` | The resolved bounds and prepared state. | Draw with methods such as `window.paint_quad(fill(bounds, color))` and register frame-local input listeners. |

The layout tree, hitboxes in the dispatch tree, and painted scene are separate. Painting a rectangle does not make it clickable; inserting a hitbox does not draw it. The hitbox returned by `insert_hitbox` can be carried as `PrepaintState` into `paint`, where a listener can test `hitbox.is_hovered_at(event.position, window)`. A hitbox records the content mask active when it was inserted, so establish clipping before inserting child hitboxes. See the [custom Element walkthrough](./element) for a complete event handler.

## Scrolling, clipping, and a worked calculation

A scroll viewport and its content have different origins. GPUI Kit's [virtual list implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/virtual_list.rs) uses a **negative** scroll offset, adds that offset when positioning items in window space, and prepaints them under a `ContentMask` for the viewport. Its `visible_range` first selects items in content space; the mask then limits drawing and input to the visible region. Selecting a visible item does not itself clip its pixels.

### Run the coordinate calculation

From the repository root, create `examples/hello_world/src/bin/geometry_walkthrough.rs` (create the `bin` directory if needed). This is a second binary in the existing `hello_world` package, not a new crate. Paste the complete program below, then run `cargo run -p hello_world --bin geometry_walkthrough`.

The vertical viewport starts at window `(100, 60)` and has size `80 × 40`. Item A starts at content `y = 20`, has height `20`, and the scroll offset is `-30`. The pointer position is in window coordinates. The program converts it to item-local coordinates and calculates each item's intersection with the viewport:

```rust
use gpui_kit::*;

fn main() {
    let viewport = bounds(point(px(100.), px(60.)), size(px(80.), px(40.)));
    let scroll_y = px(-30.);
    let item_a = bounds(
        viewport.origin + point(px(0.), px(20.) + scroll_y),
        size(px(80.), px(20.)),
    );
    assert_eq!(item_a.origin, point(px(100.), px(50.)));

    let pointer_in_window = point(px(105.), px(65.));
    let pointer_in_item = pointer_in_window.relative_to(&item_a.origin);
    assert_eq!(pointer_in_item, point(px(5.), px(15.)));

    let visible_a = item_a.intersect(&viewport);
    assert_eq!(visible_a.origin, point(px(100.), px(60.)));
    assert_eq!(visible_a.size, size(px(80.), px(10.)));
    assert!(visible_a.contains(&pointer_in_window));

    let item_b = bounds(
        viewport.origin + point(px(0.), px(80.) + scroll_y),
        size(px(80.), px(20.)),
    );
    assert_eq!(item_b.origin.y, px(110.));
    assert!(!item_b.intersects(&viewport));

    println!(
        "Item A: local pointer = ({}, {}), visible height = {}; Item B visible = {}",
        pointer_in_item.x.as_f32(),
        pointer_in_item.y.as_f32(),
        visible_a.size.height.as_f32(),
        item_b.intersects(&viewport),
    );
}
```

The expected output is `Item A: local pointer = (5, 15), visible height = 10; Item B visible = false`. The assertions also fail immediately if the scroll offset sign or coordinate origin is wrong. Item A begins 10 pixels above the viewport; only its bottom 10 pixels intersect it. Item B begins at window `y = 110`, beyond the viewport's bottom edge at `y = 100`. You can remove the exercise file after running it.

`intersect()` computes a rectangle; it does **not** clip drawing or input by itself. In a real custom element, apply the viewport's `ContentMask` while prepainting and painting children, so child hitboxes inherit the mask and painted pixels are clipped. Use the resulting hitbox to resolve pointer handling. Ordinary scroll containers manage this for you. For a custom one, keep content positioning, clipping, and hitboxes in the same coordinate space, and clamp the scroll offset to the content range as the virtual list does.

Common mistakes are treating `relative(0.5)` as 0.5 pixels before layout, comparing local coordinates with window bounds, subtracting a negative scroll offset when positioning content, or assuming `contains()` clips a painted child. For a mismatch, write down the origin and unit of each intermediate value, then inspect the resolved bounds and current content mask.

## Edges, sides, and placement

`Edges<T>` holds four independent values in `top`, `right`, `bottom`, `left` order. Use `Edges<Pixels>` for resolved insets such as padding, borders, or the space reserved around a window. `Edges::all(value)` gives every side the same value; specify fields when they differ:

```rust
use gpui_kit::*;

let padding: Edges<Pixels> = Edges {
    top: px(8.),
    right: px(12.),
    bottom: px(8.),
    left: px(12.),
};
let uniform = Edges::all(px(4.));
```

The `Edges` imported by `use gpui_kit::*` is GPUI's type. GPUI Kit also has `gpui_kit::base::Edges<T>` (reexported as `gpui_kit::component::Edges<T>`) for values that need serialization or a JSON schema. They have the same four fields but are different Rust types; use the one required by the API you call.

[`Placement`](https://docs.rs/gpui-base/latest/gpui_base/enum.Placement.html) is GPUI Kit's choice of **one side** of a trigger: `Top`, `Right`, `Bottom`, or `Left`. It is not a rectangle or a set of four insets. For example, `Positioner::side` treats `Placement::Bottom` as a *preferred* side; it may flip to `Top` if the popup does not fit below the trigger, then clamps the result inside the viewport. `ResolvedPosition::placement` reports the side actually chosen:

```rust
use gpui_kit::*;
use gpui_kit::base::{Align, Placement, Positioner};

let trigger_bounds = bounds(point(px(40.), px(40.)), size(px(100.), px(32.)));
let popup = Positioner::side(trigger_bounds)
    .placement(Placement::Bottom)
    .align(Align::Start)
    .offset(px(8.))
    .child(div().child("Menu"));
```

Here `trigger_bounds` is a `Bounds<Pixels>` in window coordinates. `Side` is the narrower GPUI Kit enum for `Left` or `Right`, and `Axis` expresses `Horizontal` or `Vertical`. GPUI's `Anchor` identifies a reference point such as `TopLeft` or `BottomCenter`; `Corners<T>` holds four corner values, often radii. Neither is a substitute for `Placement`. See [Window](./window) for window-local coordinates and scale.

## Why `Pixels` instead of `int` or `float`?

`px(12.)` produces `Pixels`, a wrapper around `f32`. Fractions matter for text metrics, animation, and positioning before rasterization. Integers would discard that precision. A bare `f32` could mean a coordinate, a scale factor, an opacity, or a fraction of a parent; it gives the compiler no way to catch a mix-up. For example, adding two pixel distances is meaningful, and multiplying a distance by a scalar stays in pixels:

```rust
let inset: Pixels = px(8.);
let width: Pixels = px(120.) - inset * 2.;
let raw: f32 = width.as_f32(); // Convert only at an API boundary that needs f32.
```

`Pixels` are GPUI's logical UI pixels, not necessarily physical display pixels. `Pixels::scale(factor)` produces `ScaledPixels`; `DevicePixels` represents integer device pixel counts. For example, `px(12.).scale(2.)` is 24 scaled pixels, but it is still a distinct type from `DevicePixels(24)`. Keep those units distinct when crossing a display or raster boundary. The wrapper cannot prove that two `Point<Pixels>` values share the same origin, or that a width is nonnegative; those remain application responsibilities.

## Lengths before layout, pixels after layout

A [style](./style) length can depend on context. GPUI expresses this with nested types:

| Type | Values | Use |
| --- | --- | --- |
| `AbsoluteLength` | `Pixels` or `Rems` | A fixed UI length or one based on the root text scale. |
| `DefiniteLength` | `AbsoluteLength` or a parent fraction | A length with a specified value, possibly relative. |
| `Length` | `DefiniteLength` or `Auto` | A layout value that may be chosen by the layout engine. |

`px(24.)` makes `Pixels`; `rems(1.5)` makes `Rems`; `relative(0.5)` makes `DefiniteLength::Fraction(0.5)`, or half the relevant parent dimension. `auto()` makes `Length::Auto`. Conversions from `Pixels`, `Rems`, and `DefiniteLength` into `Length` are available. Which values a style method accepts depends on that method's signature; let inference handle the conversion when using a builder:

```rust
use gpui_kit::*;

let panel = div()
    .w(relative(0.5))
    .min_w(px(240.))
    .h(rems(3.));
```

The width remains relative until layout knows the parent. A rem needs the root rem size. `auto` asks layout to choose a value under its rules; it is not zero. GPUI passes these values to the layout engine and receives pixel bounds.

`Percentage` is a separate `Percentage(f32)` wrapper made with `percentage(0.25)`. Its helper expects a fraction from `0.0` to `1.0` (asserted in debug builds), and GPUI can convert it to `Radians` as a portion of a full turn. It is **not** the type used by `relative(0.25)` for 25% layout width. For relative layout, use `relative`; for a percentage of a circle, use `percentage`.

## HSLA and RGBA

GPUI's [`Hsla`](https://docs.rs/gpui-pre/0.3.6/gpui/struct.Hsla.html) stores hue, saturation, lightness, and alpha as values from 0 to 1. Its `hsla(0.6, 0.8, 0.5, 1.)` constructor uses a hue fraction, not degrees, and clamps its four inputs to that range. Use `Hsla` as the default representation for theme colors and their interaction states. [`Rgba`](https://docs.rs/gpui-pre/0.3.6/gpui/struct.Rgba.html) stores red, green, blue, and alpha channels in the same range. `rgb(0x3366CC)` reads a six digit RGB hex value and sets alpha to 1; `rgba(0x3366CC80)` reads eight digits in **RRGGBBAA** order, with alpha `128 / 255` (about 0.502). Convert to `Rgba` when an RGB channel API or hex color is the natural input.

```rust
use gpui_kit::*;

let tint: Hsla = hsla(0.6, 0.8, 0.5, 1.);
let translucent = tint.opacity(0.5); // Multiply the existing alpha.
let exact_alpha = tint.alpha(0.5);   // Replace the alpha.
let again: Rgba = translucent.to_rgb();
let source: Rgba = rgb(0x3366CC);
let from_hex_alpha: Rgba = rgba(0x3366CC80);
let red_channel: f32 = from_hex_alpha.r;
```

HSLA makes it easier to change hue, saturation, or lightness while retaining the other components. RGBA is direct for hex assets, channel values, and compositing. GPUI converts between them; conversion can incur floating point rounding, so it is not a promise of byte exact round trips. HSL lightness is also not perceptual brightness: equal `l` values across different hues need not look equally bright. GPUI Kit provides `Colorize::mix_oklab` when a perceptual color interpolation is useful.

GPUI's `Hsla::blend(other)` places `other` over `self` by converting through RGBA. Its `Rgba::blend` interpolates RGB channels using the overlay alpha and keeps the receiver's alpha; it is useful for the opaque background case, but should not be described as a general alpha compositing equation for two translucent layers.

## Theme colors and interaction states

GPUI Kit stores semantic colors as `Hsla` theme values and exposes resolved tokens for components. A primary button uses `button_primary`, `button_primary_hover`, and `button_primary_active` tokens for its states. Use those tokens for an existing component instead of inventing a local lightness adjustment. Themes may supply each token explicitly, including a background gradient. When a token is absent, GPUI Kit derives a fallback from the theme: primary hover blends the background with primary after multiplying primary's existing alpha by 0.9; primary active multiplies primary's lightness by 0.9 in light mode or 0.8 in dark mode. Button primary state tokens fall back to those primary state tokens. Other variants have their own fallbacks, so there is no universal hover formula.

For a custom solid color, `Colorize` has `lighten`, `darken`, `hue`, `saturation`, and `lightness` on `Hsla`:

```rust
use gpui_kit::*;
use gpui_kit::component::Colorize;

let base: Hsla = hsla(0.6, 0.8, 0.5, 1.);
let darker = base.darken(0.1); // l = base.l * (1 - 0.1)
let quieter = base.opacity(0.6); // a = base.a * 0.6
```

`lighten(f)` multiplies lightness by `1 + f`; `darken(f)` multiplies it by `1 - f`. They do not add or subtract percentage points. Unlike the `hsla(...)` constructor, `Colorize::lighten` does not clamp its result and can produce an `l` above 1, so inspect resulting colors before using them as a theme. `Colorize::opacity` and GPUI's `Hsla::opacity` both multiply alpha. Use semantic theme tokens for control states, and check text contrast and both theme modes when defining new colors.
