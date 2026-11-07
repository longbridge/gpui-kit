---
title: Shimmer
description: Theme-aware loading text with configurable sweep timing, spread, direction, and reduced-motion behavior.
---

# Shimmer

`ShimmerText` renders readable text with a moving highlight for short-lived
loading or generated-content states. `ShimmerStyle` is the reusable appearance
and timing value shared by `ShimmerText`, `Marker`, and attachment titles.

The utility keeps text as the layout owner, so typography, wrapping, and
truncation remain ordinary GPUI text behavior. It does not replace text with a
skeleton block, own loading state, or announce progress to assistive technology.
Keep a meaningful label in the text and let the surrounding application own the
operation state.

## When to use

- “Thinking…” or “Generating…” while an AI response is being produced.
- File titles in an `Uploading` or `Processing` state.
- Lightweight text placeholders for short background work.

Use `Skeleton` for placeholder layout blocks, `Spinner` for a rotating
indeterminate control, and plain text when the state does not need motion.

## Import

```rust
use std::time::Duration;

use gpui_kit::{ParentElement as _, Styled as _};
use gpui_kit::component::{
    shimmer::{ShimmerStyle, ShimmerText},
    ActiveTheme as _,
};
```

## Basic usage

Use the default theme-aware shimmer for a loading label:

```rust
ShimmerText::new("Thinking…")
```

`ShimmerText` implements `Styled`, so it inherits the surrounding text style
and can be refined like other GPUI elements:

```rust
ShimmerText::new("Generating a response…")
    .text_sm()
    .text_color(cx.theme().muted_foreground)
    .max_w_full()
```

The default configuration is:

| Property | Default | Meaning |
| --- | --- | --- |
| Duration | 2 seconds | One complete sweep. |
| Highlight color | theme-aware | Derived from the active text/background/theme. |
| Spread | relative `0.3` | Highlight half-width as a fraction of text width; a fixed `Pixels` width is also accepted. |
| Direction | left to right | `reverse(false)`. |
| Repetition | looping | `once(false)`. |
| Reduced motion | static text | Animation frames are skipped while text remains visible. |

The highlight follows the active theme rather than assuming a white highlight.
This keeps the effect legible in both light and dark themes. An explicit color
is available when a product's semantic accent requires it.

## Configure `ShimmerStyle`

Create a reusable style when multiple labels should share the same motion:

```rust
let loading_style = ShimmerStyle::new()
    .duration(Duration::from_secs(3))
    .highlight_color(cx.theme().primary)
    .spread(0.45)
    .reverse(true)
    .once(false);

ShimmerText::new("Indexing files…").with_shimmer_style(loading_style);
ShimmerText::new("Building response…").with_shimmer_style(loading_style);
```

The individual configuration methods are also available directly on
`ShimmerText`:

```rust
ShimmerText::new("Uploading…")
    .duration(Duration::from_secs(4))
    .spread(0.5)
    .reverse(true)
    .once(true)
```

Use `with_shimmer_style(...)` when the style is shared or built conditionally;
use the direct methods when a one-off label is clearer.

### Duration

`duration(...)` sets one complete sweep. Values below one millisecond are
clamped to one millisecond, so a zero duration does not disable animation. Use
`once(true)` for a one-shot effect or render ordinary text when the operation is
not loading.

```rust
ShimmerText::new("Preparing preview…")
    .duration(Duration::from_millis(900))
```

### Highlight color

Leave `highlight_color` unset to use the theme-aware default. Use a semantic
theme color when the loading state belongs to a product accent:

```rust
ShimmerText::new("Syncing…")
    .highlight_color(cx.theme().primary)
```

An explicit color is painted over the text and must have enough contrast in
both themes. Avoid raw palette values in component call sites; use
`cx.theme().primary`, `muted_foreground`, or another semantic token.

### Spread

`spread(...)` controls the highlight half-width. A bare `f32` is relative to
the text width: finite values are clamped to the inclusive `0.05..=1.0` range.
A `Pixels` value is an absolute half-width with a one-pixel minimum, keeping
the band the same physical width across labels of different lengths. Non-finite
values leave the current spread unchanged:

```rust
ShimmerText::new("Loading a narrow label…").spread(0.15);
ShimmerText::new("Loading a broad label…").spread(0.7);
ShimmerText::new("Fixed-width highlight…").spread(px(48.));
```

Use a smaller spread for dense status rows and a broader spread for a short
assistant label. Prefer the relative form so the text's width remains the
scale; use an absolute spread when aligned labels should share one band width.

### Direction and play-once

`reverse(true)` sweeps from right to left. `once(true)` completes one sweep and
does not loop:

```rust
ShimmerText::new("Finalizing…")
    .reverse(true)
    .once(true)
```

There is no public angle, pause, progress, or RTL-specific builder. If a
product needs those behaviors, keep the loading text static or own a separate
animation component until the API is intentionally extended.

## Compose with Marker

`Marker` uses `ShimmerStyle` only when it is loading with the `Shimmer` style.
Use `MarkerContent::text(...)` to give the component a text run that can receive
the highlight:

```rust
use gpui_kit::component::marker::{Marker, MarkerContent, MarkerLoadingStyle};

Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Shimmer)
    .with_shimmer_style(
        ShimmerStyle::new()
            .duration(Duration::from_secs(3))
            .spread(0.4)
            .reverse(true),
    )
    .content(MarkerContent::new().text("Searching conversation history…"))
```

If `MarkerContent` contains only arbitrary elements, Marker uses a gentle
opacity animation for the content slot instead of trying to repaint those
elements as text. Icons and separator lines remain static. The spinner loading
style does not use shimmer.

## Compose with Attachment

An attachment title automatically shimmers while its inherited or explicit
status is `Uploading` or `Processing`. Customize that title without replacing
the attachment composition:

```rust
use gpui_kit::component::attachment::{
    Attachment, AttachmentContent, AttachmentDescription, AttachmentStatus, AttachmentTitle,
};

Attachment::new()
    .status(AttachmentStatus::Processing)
    .content(
        AttachmentContent::new()
            .title(
                AttachmentTitle::new("transcript.pdf").with_shimmer_style(
                    ShimmerStyle::new()
                        .highlight_color(cx.theme().primary)
                        .spread(0.45),
                ),
            )
            .description(AttachmentDescription::new("Processing document…")),
    )
```

The title's explicit status overrides the parent status. Generic children added
with `AttachmentContent::child(...)` do not inherit attachment state because
their concrete type is erased; use the typed title builder when the loading
behavior matters.

## Use with messages and bubbles

`ShimmerText` is an ordinary element and can be placed anywhere a text child is
accepted:

```rust
use gpui_kit::component::{
    bubble::{Bubble, BubbleContent, BubbleVariant},
    message::{Message, MessageContent},
};

Message::new()
    .content(
        MessageContent::new().bubble(
            Bubble::new()
                .with_variant(BubbleVariant::Ghost)
                .content(BubbleContent::new().child(
                    ShimmerText::new("The assistant is thinking…"),
                )),
        ),
    )
```

The application should switch from shimmer text to the final message content
when generation completes. Do not leave an animated label running after the
operation has ended.

## Styling, theme, and reduced motion

`ShimmerText` implements `Styled`; style its font, size, color, wrapping, and
layout at the call site:

```rust
ShimmerText::new("Loading project data…")
    .text_base()
    .font_medium()
    .text_color(cx.theme().foreground)
    .max_w_full()
```

The animation reads the active theme's foreground, background, and dark/light
mode when no explicit highlight color is provided. A custom theme therefore
changes the default shimmer without requiring per-label overrides. Explicit
colors remain the caller's responsibility for contrast.

When `cx.reduce_motion()` is true, `ShimmerText` renders `StyledText` without
requesting animation frames. Marker follows the same rule for typed text and
keeps arbitrary content static. This is a rendering behavior, not a separate
builder option; applications should keep the label meaningful in both modes.

## Accessibility guidance

- Keep a meaningful text label visible to assistive technology. “Thinking…” or
  “Uploading report.pdf…” is more useful than an unlabeled animated band.
- Do not rely on the highlight color, direction, or motion to communicate
  success, failure, or percentage.
- Stop or replace the shimmer when the operation completes, fails, or is
  cancelled.
- Respect reduced-motion preferences. The utility leaves static text in place,
  so no separate motion-only fallback is required.
- Use semantic `Button` or `Link` controls for cancel, retry, and navigation;
  shimmer itself is not interactive.
- Verify an explicit highlight color in both light and dark themes and avoid
  low-contrast combinations.

## When not to use Shimmer

Shimmer communicates activity, not progress.

- Use `Progress` for a known percentage.
- Use `Spinner` for a compact rotating indicator.
- Use `Skeleton` for multi-line placeholder layout.
- Render ordinary text once a stable, completed, or failed state exists; do
  not leave the animation running.

## API reference

### `ShimmerStyle`

| Method | Default | Purpose |
| --- | --- | --- |
| `new()` | same as `Default` | Create a theme-aware two-second looping style. |
| `duration(Duration)` | 2 seconds | Set one sweep duration; clamps below 1 ms. |
| `highlight_color(Hsla)` | theme-aware | Override the highlight color. |
| `spread(f32 \| Pixels)` | relative `0.3` | Set half-width: `f32` is relative and clamps to `0.05..=1.0`; `Pixels` is absolute with a 1px minimum. |
| `reverse(bool)` | `false` | Reverse the sweep direction. |
| `once(bool)` | `false` | Play one sweep instead of looping. |

### `ShimmerText`

| Method | Default | Purpose |
| --- | --- | --- |
| `new(text)` | default style, generated identity | Create loading text. |
| `id(ElementId)` | text-based identity | Distinguish identical sibling labels. |
| `with_shimmer_style(ShimmerStyle)` | default style | Apply a reusable configuration. |
| `duration(Duration)` | 2 seconds | Set duration directly. |
| `highlight_color(Hsla)` | theme-aware | Set color directly. |
| `spread(f32 \| Pixels)` | relative `0.3` | Set spread directly. |
| `reverse(bool)` | `false` | Set direction directly. |
| `once(bool)` | `false` | Set repetition directly. |
| `Styled` methods | inherited text style | Refine typography, color, wrapping, and layout. |

### Related components

- [`Marker`] — status rows with spinner or shimmer loading styles.
- [`AttachmentTitle`] — status-aware file title with shimmer customization.
- [`Progress`] — determinate progress.
- [`Spinner`] — compact indeterminate progress.

[ShimmerStyle]: https://docs.rs/gpui-component/latest/gpui_component/shimmer/struct.ShimmerStyle.html
[ShimmerText]: https://docs.rs/gpui-component/latest/gpui_component/shimmer/struct.ShimmerText.html
[Marker]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.Marker.html
[AttachmentTitle]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentTitle.html
[Progress]: https://docs.rs/gpui-component/latest/gpui_component/progress/struct.Progress.html
[Spinner]: https://docs.rs/gpui-component/latest/gpui_component/spinner/struct.Spinner.html
