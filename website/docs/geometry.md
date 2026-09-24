---
title: Geometry
description: Work with GPUI's typed coordinates, layout lengths, and colors in practical UI code.
order: -2.8
---

# Geometry and color

GPUI uses types to say what a number *means*. A coordinate is a `Point<T>`, an extent is a `Size<T>`, and a rectangle is a `Bounds<T>`. The `T` says which unit the components use. Layout accepts lengths that may still need a parent size; drawing and hit testing usually use resolved `Pixels`. Colors similarly distinguish a convenient channel representation (`Rgba`) from a hue based representation (`Hsla`). GPUI Kit reexports GPUI, so application examples can start with `use gpui_kit::*;`.

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

## RGBA and HSLA

[`Rgba`](https://docs.rs/gpui-pre/0.3.6/gpui/struct.Rgba.html) stores red, green, blue, and alpha channels as `f32` values from 0 to 1. `rgb(0x3366CC)` reads a six digit RGB hex value and sets alpha to 1; `rgba(0x3366CC80)` reads eight digits in **RRGGBBAA** order, with alpha `128 / 255` (about 0.502). GPUI's [`Hsla`](https://docs.rs/gpui-pre/0.3.6/gpui/struct.Hsla.html) stores hue, saturation, lightness, and alpha, also normalized to 0 to 1. Its `hsla(0.6, 0.8, 0.5, 1.)` constructor uses a hue fraction, not degrees, and clamps its four inputs to that range. Use `Hsla` as the default representation for theme colors and their interaction states; convert to `Rgba` when an RGB channel API or hex color is the natural input.

```rust
use gpui_kit::*;

let source: Rgba = rgb(0x3366CC);
let tint: Hsla = source.into();
let translucent = tint.opacity(0.5); // Multiply the existing alpha.
let exact_alpha = tint.alpha(0.5);   // Replace the alpha.
let again: Rgba = translucent.to_rgb();
let from_hex_alpha: Rgba = rgba(0x3366CC80);
let red_channel: f32 = from_hex_alpha.r;
```

RGBA is direct for hex assets, channel values, and compositing. HSLA is convenient when changing hue, saturation, or lightness while retaining the other components. GPUI converts between them; conversion can incur floating point rounding, so it is not a promise of byte exact round trips. HSL lightness is also not perceptual brightness: equal `l` values across different hues need not look equally bright. GPUI Kit provides `Colorize::mix_oklab` when a perceptual color interpolation is useful.

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
