---
title: Marker
description: A compact composable row for conversation status, notifications, loading, and separators.
---

# Marker

`Marker` is a lightweight row for status text, timeline boundaries, unread
labels, and system notices. It deliberately accepts arbitrary children instead
of defining an application-specific status enum. `MarkerIcon` and
`MarkerContent` are optional typed slots for the common icon-and-label shape;
direct children remain available for custom composition.

`Marker` is a layout and loading primitive. It does not own a notification
record, an unread count, a click action, or a live status store. Compose those
with application state and existing `Badge`, `Button`, `Link`, or navigation
components.

## Import

```rust
use gpui_kit::{ParentElement as _, StyleRefinement, Styled as _};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _,
    badge::Badge,
    button::{Button, ButtonVariants as _},
    marker::{Marker, MarkerContent, MarkerIcon, MarkerLoadingStyle, MarkerVariant},
    shimmer::{ShimmerStyle, ShimmerText},
    spinner::Spinner,
};
use std::time::Duration;
```

## Anatomy and basic usage

The typed form keeps icon and content style targets independent:

```rust
Marker::new()
    .icon(MarkerIcon::new().child(Icon::new(IconName::CircleCheck)))
    .content(MarkerContent::new().text("Online"))
```

Direct children are useful when a marker needs an application-specific layout:

```rust
Marker::new()
    .child(Icon::new(IconName::Info))
    .child("Conversation archived")
```

The default state is:

| Property | Default | Meaning |
| --- | --- | --- |
| Variant | `Plain` | A full-width status row without divider decoration. |
| Loading | `false` | No automatic loading effect. |
| Loading style | `Spinner` | Used when loading is enabled. |
| Icon slot | absent | A spinner is inserted only for spinner loading with no icon. |
| Content | absent | Add text or arbitrary child content. |
| Row minimum height | `rems(1.)` | Follows the shared typography scale. |
| Row gap | `gap_2()` | Shared compact spacing. |

Use `MarkerContent::text(...)` for text that should receive the loading shimmer.
Use `.child(...)` for arbitrary elements or text that should keep its own
rendering behavior.

## Variants

### Plain

`Plain` is the default compact status row:

```rust
Marker::new()
    .text_color(cx.theme().success)
    .icon(MarkerIcon::new().child(Icon::new(IconName::CircleCheck)))
    .content(MarkerContent::new().text("Synced"))
```

The library does not define `Online`, `Read`, `Typing`, or `Synced` values.
The application supplies the words, icon, and semantic color so the same
primitive can serve different domains.

### Separator

`Separator` adds a flexible line on each side of the content:

```rust
Marker::new()
    .with_variant(MarkerVariant::Separator)
    .content(MarkerContent::new().text("Today"))
```

The line is an internal 1-pixel decorative element. The label remains the
semantic content. Use `separator_style(...)` to refine the two lines without
having to recreate their layout:

```rust
Marker::new()
    .with_variant(MarkerVariant::Separator)
    .separator_style(
        StyleRefinement::default()
            .bg(cx.theme().ring),
    )
    .content(MarkerContent::new().text("Yesterday"))
```

### Border

`Border` adds a semantic bottom border and compact bottom padding:

```rust
Marker::new()
    .with_variant(MarkerVariant::Border)
    .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
    .content(MarkerContent::new().text("3 unread messages"))
```

The border is a visual boundary. Keep the unread count and meaning in text so
the state does not depend on color or a line alone.

## Loading styles

Set `loading(true)` without changing the marker's variant or normal layout:

```rust
Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Spinner)
    .content(MarkerContent::new().text("Loading messages…"));

Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Shimmer)
    .content(MarkerContent::new().text("Thinking…"))
```

Spinner behavior is intentionally predictable:

- `Spinner` is the default `MarkerLoadingStyle`.
- If loading uses `Spinner` and no `MarkerIcon` was supplied, a compact
  `Spinner::new().xsmall()` is inserted automatically.
- If the application supplies `MarkerIcon`, that icon wins and no automatic
  spinner is added.
- `MarkerVariant::Separator` still renders its divider lines while loading.
- `MarkerVariant::Border` still renders its border while loading.

Shimmer is text-aware when content was added with `.text(...)`:

```rust
Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Shimmer)
    .content(MarkerContent::new().text("Generating a response…"))
```

Arbitrary `MarkerContent` children are still supported. When there is no typed
text child, the content slot receives a gentle opacity animation instead. Icons
and separator lines stay static. When reduced motion is enabled, text is
rendered without animation and the marker remains readable.

## Shimmer configuration

Use one `ShimmerStyle` for a marker's text effect:

```rust
Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Shimmer)
    .with_shimmer_style(
        ShimmerStyle::new()
            .duration(Duration::from_secs(3))
            .highlight_color(cx.theme().primary)
            .spread(0.45)
            .reverse(true)
            .once(false),
    )
    .content(MarkerContent::new().text("Processing files…"))
```

The `ShimmerStyle` defaults are a two-second repeating sweep, theme-aware
highlight color, `0.3` normalized spread, left-to-right direction, and looping.
`duration(...)` clamps values below one millisecond. `spread(...)` accepts a
relative `f32` (clamped to `0.05..=1.0`) or an absolute `Pixels` half-width;
non-finite values leave the current spread unchanged.
`reverse(true)` changes the direction, and `once(true)` stops after one sweep.

For a marker-independent loading label, use `ShimmerText` directly:

```rust
ShimmerText::new("Uploading report.pdf…")
    .with_shimmer_style(ShimmerStyle::new().spread(0.4))
    .text_sm()
    .text_color(cx.theme().muted_foreground)
```

`ShimmerText` inherits typography and text color through `Styled`, preserves
wrapping and truncation, and uses the active theme's background and foreground
to keep the highlight readable in light and dark modes.

## Icons, content, and interactive children

`MarkerIcon` is a compact `size_4()` slot. `MarkerContent` is a `min_w_0()`
slot, so a long label can choose its own wrapping or truncation:

```rust
Marker::new()
    .icon(MarkerIcon::new().child(Icon::new(IconName::Bell)))
    .content(
        MarkerContent::new()
            .child("Unread notifications")
            .child(Badge::new().count(3)),
    )
```

Interactive children are allowed, but `Marker` does not make the row itself a
control:

```rust
Marker::new()
    .content(
        MarkerContent::new()
            .text("New messages")
            .child(Button::new("open-messages").ghost().xsmall().label("Open")),
    )
```

Use `Button` for an in-app command and `Link` for a URL. Keep focus and action
semantics on those controls. If a whole marker should be clickable, compose a
semantic control around the content at the application boundary instead of
adding a click listener to this layout element.

## Custom styling and theme tokens

`Marker`, `MarkerIcon`, and `MarkerContent` implement `Styled`. Refinements are
applied after the default layout and theme colors:

```rust
Marker::new()
    .px_3()
    .py_2()
    .rounded(cx.theme().radius)
    .bg(cx.theme().accent)
    .text_color(cx.theme().accent_foreground)
    .icon(MarkerIcon::new().child(Icon::new(IconName::Star)))
    .content(MarkerContent::new().text("Pinned message"))
```

The separator lines have a separate `StyleRefinement`, so their color and
height can be customized without changing the content or marker's own surface:

```rust
Marker::new()
    .with_variant(MarkerVariant::Separator)
    .separator_style(
        StyleRefinement::default()
            .bg(cx.theme().border),
    )
    .content(MarkerContent::new().text("New day"))
```

Prefer semantic theme roles (`muted_foreground`, `border`, `ring`, `accent`,
and their foreground tokens) to raw colors. Radius, spacing, typography, and
separator geometry follow the shared design scale; typed style refinements can
adapt a marker to a denser toolbar or a larger empty-state boundary.

## Accessibility and motion guidance

- Include the status, boundary, or unread count in text. Icons, border lines,
  opacity, and color are supporting cues only.
- A marker is presentational by default. Set `.id(...)` and `.role(Role::Status)`
  on a row that reports streaming or loading progress so assistive technology
  announces its updates; the role needs the stable identity an id provides.
- Keep interactive content in `Button` or `Link` so it receives keyboard focus,
  activation, and disabled state. For the current `Button` API, use a visible
  `.label(...)` when the action needs an accessible name; a tooltip is
  supplemental.
- Do not use `Marker` as an unlabeled icon-only status. Add a visible or
  accessible text label when the icon has meaning.
- `MarkerContent::text(...)` remains visible when reduced motion is enabled;
  only the shimmer frame updates are skipped. Arbitrary children also retain
  their static content.
- Loading text should describe the operation (“Generating…”, “Uploading…”)
  rather than communicate only through animation.
- Keep sufficient contrast after custom styling in both light and dark themes.

## When to use another component

- Use `Badge` for only a count, dot, or short classification.
- Use `Separator::horizontal().label(...)` when the product needs only a
  labeled divider and no marker loading or icon composition.
- Use `Tag` for a standalone labeled status that is not part of a conversation
  row.
- Use `h_flex()` when the row has no shared marker behavior.
- Use `Message` or `Bubble` when the content is a conversational message with
  sender identity or a message surface.

## API reference

### `Marker`

| Method | Default | Purpose |
| --- | --- | --- |
| `new()` | `Plain`, not loading, spinner style | Create a marker. |
| `with_variant(MarkerVariant)` | `Plain` | Choose plain, separator, or border treatment. |
| `loading(bool)` | `false` | Enable or disable loading rendering. |
| `with_loading_style(MarkerLoadingStyle)` | `Spinner` | Choose spinner or shimmer. |
| `with_shimmer_style(ShimmerStyle)` | default style | Configure text shimmer. |
| `separator_style(StyleRefinement)` | theme border line | Refine separator lines. |
| `id(ElementId)` | none | Give the marker a stable identity for the accessibility tree. |
| `role(Role)` | presentational | Announce the row to assistive technology, e.g. `Role::Status` for streaming updates; requires `id(...)`. |
| `icon(MarkerIcon)` | none | Add a typed icon slot. |
| `content(MarkerContent)` | none | Add a typed content slot. |
| `.child(element)` | — | Add arbitrary children. |
| `Styled` methods | compact themed row | Refine the marker's layout, colors, and typography. |

### `MarkerIcon`

| Method | Default | Purpose |
| --- | --- | --- |
| `new()` | empty `size_4()` slot | Create an icon slot. |
| `.child(element)` | — | Add an icon, badge, spinner, or custom element. |
| `Styled` methods | `size_4()` compact slot | Refine icon geometry and layout. |

### `MarkerContent`

| Method | Default | Purpose |
| --- | --- | --- |
| `new()` | empty `min_w_0()` slot | Create content. |
| `text(text)` | static text until loading is enabled | Add text that can receive shimmer. |
| `.child(element)` | — | Add arbitrary rich content. |
| `Styled` methods | inherited text and compact layout | Refine wrapping, colors, spacing, and typography. |

### Related types

- [`MarkerVariant`] — `Plain`, `Separator`, and `Border`.
- [`MarkerLoadingStyle`] — `Spinner` or `Shimmer`.
- [`ShimmerStyle`] and [`ShimmerText`] — reusable loading text controls.

[Marker]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.Marker.html
[MarkerIcon]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.MarkerIcon.html
[MarkerContent]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.MarkerContent.html
[MarkerVariant]: https://docs.rs/gpui-component/latest/gpui_component/marker/enum.MarkerVariant.html
[MarkerLoadingStyle]: https://docs.rs/gpui-component/latest/gpui_component/marker/enum.MarkerLoadingStyle.html
[ShimmerStyle]: https://docs.rs/gpui-component/latest/gpui_component/shimmer/struct.ShimmerStyle.html
[ShimmerText]: https://docs.rs/gpui-component/latest/gpui_component/shimmer/struct.ShimmerText.html
