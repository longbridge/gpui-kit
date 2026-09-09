---
title: Bubble
description: A composable chat surface for text, rich content, and reaction controls.
---

# Bubble

`Bubble` is the surface-level primitive for a conversation. It owns the
alignment, the maximum content width, and the position of an optional reaction
region. `BubbleContent` owns the visible surface. Keeping those responsibilities
separate lets an application replace the content layout without reimplementing
message alignment.

`Bubble` is a presentational element. It does not own a message record, a
collapsed state, a reaction model, or a click action. Compose those behaviors
with application state and existing controls such as `Button`, `Link`,
`Collapsible`, `Tooltip`, and `Popover`.

## Import

```rust
use gpui_kit::{div, ParentElement as _, Styled as _};
use gpui_kit::component::{
    ActiveTheme as _, Colorize as _, Sizable as _,
    bubble::{
        Bubble, BubbleContent, BubbleGroup, BubbleReactionSide, BubbleReactions,
        BubbleVariant,
    },
    button::{Button, ButtonVariants as _},
    message::MessageAlignment,
};
```

## Anatomy and basic usage

The shortest form adds children to the typed content slot:

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .child("Can you review this draft?")
```

Use `content(...)` when the surface needs its own layout or style target:

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .content(
        BubbleContent::new().child(
            gpui_kit::component::h_flex()
                .gap_2()
                .child("Can you review this draft?")
                .child("📎"),
        ),
    )
```

The root is `min_w_0`, grows to the available width only for `Ghost`, and
otherwise has a maximum width of 80% of its parent. Long text wraps inside the
content slot when its child allows wrapping. An application that needs a
different conversation measure can refine the root with `w(...)`, `max_w(...)`,
or a child-specific layout.

The default state is:

| Property | Default | Meaning |
| --- | --- | --- |
| Alignment | unset | The parent may supply alignment; a standalone bubble does not force an edge. |
| Variant | `Filled` | Primary semantic surface. |
| Reactions | none | No reaction region is rendered. |
| Maximum width | `0.8` of the parent | Applies to regular variants. |
| Surface radius | `cx.theme().radius_2xl()` | Follows the active theme. |
| Content padding | `px_3()` / `py_2()` | Applied by `BubbleContent` for regular variants. |

## Alignment

`MessageAlignment::Start` and `MessageAlignment::End` are shared with
`Message`:

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .with_variant(BubbleVariant::Secondary)
    .child("Incoming message");

Bubble::new()
    .alignment(MessageAlignment::End)
    .child("Outgoing message")
```

When a bubble is placed in `MessageContent::bubble(...)`, `Message` propagates
its alignment to the content surface. Leave the bubble alignment unset in that
case so the message remains the single owner of horizontal placement. Set it
explicitly when a bubble is used on its own or when a custom parent intentionally
overrides the message row.

## Variants

`BubbleVariant` selects semantic colors and surface treatment. It does not
change the content model:

```rust
Bubble::new()
    .with_variant(BubbleVariant::Filled)
    .child("Primary response");

Bubble::new()
    .with_variant(BubbleVariant::Secondary)
    .child("Neutral incoming response");

Bubble::new()
    .with_variant(BubbleVariant::Muted)
    .child("Low-emphasis context");

Bubble::new()
    .with_variant(BubbleVariant::Tinted)
    .child("Subtle selected or emphasized response");

Bubble::new()
    .with_variant(BubbleVariant::Outline)
    .child("A response that needs a visible boundary");

Bubble::new()
    .with_variant(BubbleVariant::Ghost)
    .child("A full-width, unframed message surface");

Bubble::new()
    .with_variant(BubbleVariant::Destructive)
    .child("The operation failed; explain what the user can do next.")
```

`Filled` is the default. `Ghost` removes the surface padding, border, and
radius, and can occupy the full row. `Destructive` uses the semantic
destructive color with a theme-aware translucent surface; its meaning must
also be present in text or another non-color cue. All other variants keep the
regular content surface and derive colors from the active theme.

## Rich content and long messages

Bubble children are arbitrary GPUI elements. Compose text, code, files,
buttons, or custom layouts without a bubble-specific content enum:

```rust
use gpui_kit::{div, Styled as _};
use gpui_kit::component::{h_flex, v_flex, Icon, IconName};

Bubble::new()
    .content(
        BubbleContent::new().child(
            h_flex()
                .gap_3()
                .items_start()
                .child(Icon::new(IconName::FileText))
                .child(
                    v_flex()
                        .min_w_0()
                        .child("design-notes.pdf")
                        .child(div().text_sm().child("PDF · 2.4 MB")),
                ),
        ),
    )
```

For a long response, keep the child `min_w_0()` and choose wrapping or
truncation at the content boundary. `Bubble` does not truncate arbitrary
children. An application layout can expose a `Show more` affordance by
wrapping the content in `Collapsible`; the bubble itself has no hidden-text
state.

```rust
// The state and trigger belong to the application. The same Bubble can be
// rendered in the expanded and collapsed states.
Bubble::new()
    .with_variant(BubbleVariant::Ghost)
    .content(BubbleContent::new().child(long_response_element))
```

## Groups

`BubbleGroup` is a styleable vertical stack. It does not infer sender identity
or remove headers; the application decides which consecutive bubbles belong to
one sender:

```rust
BubbleGroup::new()
    .child(
        Bubble::new()
            .alignment(MessageAlignment::Start)
            .with_variant(BubbleVariant::Secondary)
            .child("The first paragraph belongs to Alice."),
    )
    .child(
        Bubble::new()
            .alignment(MessageAlignment::Start)
            .with_variant(BubbleVariant::Secondary)
            .child("The second paragraph uses the same group."),
    )
```

Use `MessageGroup` when the repeated unit is a complete message with avatar,
header, body, and footer. Use `BubbleGroup` when only the surface stack is
being repeated.

## Reactions and interactive content

`BubbleReactions` positions a region at the top or bottom edge. Put semantic
controls inside it. Use the typed `action(Button)` builder for a button that
should read as part of the reaction surface:

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .with_variant(BubbleVariant::Outline)
    .child("This response has feedback.")
    .reactions(
        BubbleReactions::new()
            .side(BubbleReactionSide::Bottom)
            .alignment(MessageAlignment::End)
            .action(
                Button::new("bubble-like")
                    .ghost()
                    .small()
                    .label("Like · 2"),
            )
            .action(
                Button::new("bubble-copy")
                    .ghost()
                    .small()
                    .label("Copy"),
            ),
    )
```

The defaults for `BubbleReactions` are `Bottom` and `End`. For a top-attached
region aligned to the leading edge:

```rust
BubbleReactions::new()
    .side(BubbleReactionSide::Top)
    .alignment(MessageAlignment::Start)
    .action(Button::new("bubble-more").ghost().xsmall().label("More"))
```

`action(Button)` tells `BubbleReactions` that the child is a semantic action.
When a reaction region contains any typed action, the container removes its
decorative content padding and applies the active theme's full/pill radius to
each typed button, so the buttons and reaction surface read as one control
group. The supplied `Button` remains customizable: its variant, size, icon,
`.on_click(...)` callback, and `.tooltip(...)` are preserved. The typed action
owns the pill corner radius so the button stays joined to the reaction surface;
use the generic path below when a button needs a different radius. Multiple
actions can be added with repeated `.action(...)` calls.

Use `.child(...)` for emoji, text, a custom element, or an overlay composition
that is not a direct `Button`. This generic path remains backward-compatible
and does not opt that child into the compact action treatment. If the same
region also contains any `.action(...)`, the whole reaction region still uses
the compact surface layout:

```rust
BubbleReactions::new()
    .child("👍 2")
    .action(
        Button::new("bubble-reply")
            .ghost()
            .xsmall()
            .label("Reply"),
    )
```

Nested interactive wrappers such as `Popover` remain on the generic
`.child(...)` path because `action(...)` accepts a direct `Button`. If the
wrapper's trigger should share the reaction geometry, opt into that layout
explicitly with `p_0()` on the reaction region and a theme-derived full radius
on the trigger button. The same escape hatch gives an arbitrary button its own
radius or surface treatment.

```rust
BubbleReactions::new().p_0().child(
    gpui_kit::component::popover::Popover::new("bubble-more")
        .trigger(
            Button::new("bubble-more-trigger")
                .ghost()
                .xsmall()
                .label("More")
                .rounded(cx.theme().radius_full()),
        )
        .child(Button::new("bubble-copy").label("Copy")),
)
```

The reaction container supplies the default spacing, rounded semantic surface,
and contrast border. Caller `Styled` refinements are applied after those
defaults, so an application can customize the reaction surface or the
`Button` itself. There is no separate `BubbleAction` component or reaction data
model; the application owns counts, selected state, and submitted actions.
Button focus, disabled state, and keyboard activation remain
the responsibility of `Button`. The current Button accessibility label comes
from its visible `.label(...)` value; a tooltip is supplemental. For a URL
inside a bubble use `Link`; for an in-app command use `Button`. A tooltip or
popover can wrap the relevant child using the existing overlay components.

## Custom styling and theme tokens

`Bubble`, `BubbleContent`, `BubbleGroup`, and `BubbleReactions` implement
`Styled`. Refinements are applied after the component defaults, so callers can
adjust spacing, width, typography, borders, backgrounds, and shadows at the
appropriate part boundary:

```rust
Bubble::new()
    .w_full()
    .content(
        BubbleContent::new()
            .rounded(cx.theme().radius_lg)
            .bg(cx.theme().group_box)
            .text_color(cx.theme().group_box_foreground)
            .border_1()
            .border_color(cx.theme().border)
            .px_4()
            .py_3()
            .child("Application-owned surface treatment"),
    )
```

Use semantic theme roles (`primary`, `muted`, `group_box`, `border`,
`destructive`, and their foreground colors) instead of raw palette values.
Radii come from the active theme, so a custom theme can make all conversation
surfaces more square or more rounded consistently. The component uses the
shared spacing and typography scale; a product-specific scale should be owned
by the surrounding design-system layer and passed through its own builders.

Style the group and reaction region independently when the composition needs a
different rhythm:

```rust
BubbleGroup::new()
    .gap_3()
    .child(Bubble::new().child("First"))
    .child(Bubble::new().child("Second"));

BubbleReactions::new()
    .px_2()
    .bg(cx.theme().background)
    .border_color(cx.theme().ring)
    .action(Button::new("bubble-reaction").ghost().xsmall().label("👍"))
```

## Accessibility and state guidance

- Use visible text, an icon with a label, or an accessible `Button` label to
  communicate reactions and actions. Color and a bubble variant are not
  sufficient status announcements.
- Keep keyboard actions inside `Button`, `Link`, `Collapsible`, `Tooltip`, or
  `Popover`. `Bubble` and `BubbleReactions` are layout elements and do not
  create focus targets themselves.
- Preserve readable contrast when overriding a surface. Pair a custom
  background with the matching semantic foreground token or an explicitly
  verified theme role.
- For loading or generated content, render a meaningful text label and use
  `ShimmerText` or `Marker` for motion. Respect the application's reduced-motion
  behavior; the shimmer utility renders static text when reduced motion is
  requested.
- A failed or destructive bubble should include the error and the next action,
  not only a red surface.

## When to use another component

Use `Message` when sender identity, metadata, or a footer belongs to the same
row. Use `Marker` for a compact status or timeline boundary. Use `GroupBox` or
an application-owned surface for a non-conversational document. Use a plain
`div()`/`h_flex()` when the row has no shared bubble behavior; adding a bubble
only to obtain padding makes the hierarchy harder to read.

## API reference

### `Bubble`

| Method | Default | Purpose |
| --- | --- | --- |
| `new()` | filled, no alignment, no reactions | Create a bubble. |
| `alignment(MessageAlignment)` | unset | Place the bubble at the leading or trailing edge. |
| `with_variant(BubbleVariant)` | `Filled` | Select the semantic surface treatment. |
| `content(BubbleContent)` | empty typed content | Replace the visible content surface; direct children move into it. |
| `reactions(BubbleReactions)` | none | Attach a reaction region. |

`Bubble` also implements `ParentElement` for the direct `.child(...)` form and
`Styled` for root layout refinements.

### `BubbleContent`

| Method | Default | Purpose |
| --- | --- | --- |
| `new()` | empty | Create the visible surface slot. |
| `.child(...)` | — | Add arbitrary GPUI elements. |
| `Styled` methods | component defaults | Refine padding, radius, colors, typography, and layout. |

The parent `Bubble` supplies its variant and alignment to this slot. A
standalone `BubbleContent` therefore has the default `Filled` treatment.

### `BubbleGroup`

| Method | Default | Purpose |
| --- | --- | --- |
| `new()` | empty vertical stack | Create a group. |
| `.child(...)` | — | Add consecutive bubbles. |
| `Styled` methods | `gap_2()` | Refine group spacing and layout. |

### `BubbleReactions`

| Method | Default | Purpose |
| --- | --- | --- |
| `new()` | bottom, end aligned | Create a reaction region. |
| `side(BubbleReactionSide)` | `Bottom` | Attach it above or below the bubble. |
| `alignment(MessageAlignment)` | `End` | Align children along the bubble edge. |
| `action(Button)` | — | Add a typed action that shares the reaction surface and full/pill radius. |
| `.child(...)` | — | Add emoji, text, or arbitrary GPUI elements. |
| `Styled` methods | themed reaction surface | Refine spacing, colors, and layout. |

### Related types

- [`BubbleVariant`] — `Filled`, `Secondary`, `Muted`, `Tinted`, `Outline`,
  `Ghost`, and `Destructive`.
- [`BubbleReactionSide`] — `Top` or `Bottom`.
- [`MessageAlignment`] — `Start` or `End`.

[Bubble]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.Bubble.html
[BubbleContent]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.BubbleContent.html
[BubbleGroup]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.BubbleGroup.html
[BubbleReactions]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.BubbleReactions.html
[BubbleVariant]: https://docs.rs/gpui-component/latest/gpui_component/bubble/enum.BubbleVariant.html
[BubbleReactionSide]: https://docs.rs/gpui-component/latest/gpui_component/bubble/enum.BubbleReactionSide.html
[MessageAlignment]: https://docs.rs/gpui-component/latest/gpui_component/message/enum.MessageAlignment.html
