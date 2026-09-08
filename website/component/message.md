---
title: Message
description: Compose sender identity, metadata, rich content, and actions into an aligned chat message.
---

# Message

`Message` is the row-level composition primitive for a conversation. It owns
the horizontal alignment and the vertical stack that contains optional sender
identity, metadata, content, and footer slots. It does not own a sender model,
timestamp formatting, delivery state, reaction state, or message actions.

Applications provide those values and compose existing components inside the
slots. This keeps the message layout reusable across direct messages, group
chat, assistant responses, system notices, and generated content.

## Import

```rust
use gpui_kit::{ParentElement as _, StyleRefinement, Styled as _};
use gpui_kit::component::{
    ActiveTheme as _, Colorize as _, Sizable as _,
    attachment::{Attachment, AttachmentContent, AttachmentTitle},
    avatar::Avatar,
    bubble::{Bubble, BubbleVariant},
    button::{Button, ButtonVariants as _},
    message::{
        Message, MessageAlignment, MessageAvatar, MessageContent, MessageFooter,
        MessageGroup, MessageHeader,
    },
};
```

## Anatomy and basic usage

All named slots are optional, so a minimal message can contain only a body:

```rust
Message::new().content(
    MessageContent::new().bubble(Bubble::new().child("Can you review this?")),
)
```

A complete message commonly combines sender identity, metadata, a bubble, and
a delivery footer:

```rust
Message::new()
    .avatar_slot(
        MessageAvatar::new()
            .child(Avatar::new().name("Alice").size_8()),
    )
    .header(
        MessageHeader::new()
            .child("Alice")
            .child("10:24 AM"),
    )
    .content(
        MessageContent::new().bubble(
            Bubble::new()
                .with_variant(BubbleVariant::Secondary)
                .child("Can you review this draft?"),
        ),
    )
    .footer(MessageFooter::new().child("Read"))
```

The default state is:

| Property | Default | Meaning |
| --- | --- | --- |
| Alignment | `MessageAlignment::Start` | Place the message at the leading edge. |
| Avatar/header/content/footer | absent | Add only the slots needed by the product. |
| Outer layout | full width, `min_w_0()`, `gap_2()` | Keeps rows usable in a virtual list. |
| Inner stack gap | `rems(0.625)` | Separates metadata, body, and footer. |
| Header/footer inset | enabled, `px_3()` | Aligns metadata with a regular bubble surface. |
| Avatar baseline | `min_w_8()`, circular muted surface | Gives sender identity a stable column. |

`Message` applies its alignment to the complete row and to the named content
stack. It reverses the outer row for `End`, so the avatar and message stack
remain a single aligned unit.

## Alignment

Use `Start` for incoming content and `End` for outgoing content:

```rust
Message::new()
    .alignment(MessageAlignment::Start)
    .avatar(Avatar::new().name("Alice").size_8())
    .header(MessageHeader::new().child("Alice").child("10:24 AM"))
    .content(MessageContent::new().bubble(
        Bubble::new()
            .with_variant(BubbleVariant::Secondary)
            .child("Incoming message"),
    ));

Message::new()
    .alignment(MessageAlignment::End)
    .avatar(Avatar::new().name("You").size_8())
    .header(MessageHeader::new().child("You").child("10:25 AM"))
    .content(MessageContent::new().bubble(Bubble::new().child("Outgoing message")))
    .footer(MessageFooter::new().child("Delivered"))
```

`MessageAlignment` is also accepted by `Bubble`. When the body is a typed
`MessageContent::bubble(...)`, the message propagates its alignment to that
bubble's surface. Leave the bubble's own alignment unset in this composition so
the row has one clear owner of placement.

## Avatar, header, content, and footer

### Avatar

`.avatar(...)` wraps any element in `MessageAvatar`. Use `.avatar_slot(...)`
when the slot itself needs styling or multiple children:

```rust
Message::new()
    .avatar_slot(
        MessageAvatar::new()
            .bg(cx.theme().transparent)
            .child(Avatar::new().name("System").size_8()),
    )
    .content(MessageContent::new().child("A system update"))
```

The avatar reserves the shared `size-8` baseline and always sits flush with
the bottom edge of the message content; the footer renders below the avatar
row, indented to the content column. The message does not require an avatar;
omit it for assistant messages, compact group chat, or system rows where
identity is already present elsewhere.

### Header

`MessageHeader` is an arbitrary horizontal metadata row. It defaults to an
extra-small, medium-weight, muted style with a `px_3()` inset:

```rust
MessageHeader::new()
    .child("Alice")
    .child("·")
    .child("10:24 AM")
```

The header does not format dates or infer sender names. Use application-owned
formatting and compose a `Tooltip` around timestamps when the full date is
useful.

### Content

`MessageContent` is a full-width, minimum-width-safe vertical stack. It accepts
arbitrary elements and has a typed `.bubble(...)` convenience that records
whether a ghost bubble is present:

```rust
MessageContent::new()
    .bubble(Bubble::new().child("First paragraph"))
    .bubble(Bubble::new().child("Second paragraph"))
```

Use `.child(...)` for attachments, code blocks, images, or custom rich content:

```rust
MessageContent::new()
    .bubble(Bubble::new().child("Here is the file:"))
    .child(
        Attachment::new().content(
            AttachmentContent::new()
                .title(AttachmentTitle::new("quarterly-report.pdf")),
        ),
    )
```

Typed bubbles are useful when the surrounding header and footer should respond
to the `Ghost` variant. Arbitrary `.child(...)` values are still fully
composable, but their concrete type is erased and they do not set that
ghost-surface metadata.

### Footer

`MessageFooter` is another arbitrary horizontal metadata row. Use it for
delivery state, reactions, or actions composed from existing controls:

```rust
MessageFooter::new()
    .child("Delivered")
    .child(Button::new("reply").ghost().xsmall().label("Reply"))
    .child(Button::new("copy").ghost().xsmall().label("Copy"))
```

Footer uses the same extra-small muted default and `px_3()` inset as the header.
The footer does not own a delivery-state enum or action semantics.

## Rich content and actions

Compose the existing component that owns each behavior:

```rust
Message::new()
    .content(
        MessageContent::new()
            .bubble(Bubble::new().child("The export is ready."))
            .child(
                Attachment::new()
                    .content(AttachmentContent::new().title(AttachmentTitle::new("export.zip"))),
            ),
    )
    .footer(
        MessageFooter::new()
            .child(Button::new("download-export").label("Download"))
            .child(Button::new("share-export").ghost().label("Share")),
    )
```

Use `Button` for commands, `Link` for URLs, `Attachment` for files, and
`Bubble` for conversational surfaces. This keeps disabled, loading, focus,
keyboard, and accessible-name behavior on the control that owns it. A message
does not become clickable merely because it contains a button.

Long or multiline content remains the responsibility of the child element. Keep
custom children `min_w_0()` when they contain long text or horizontal layouts;
`Message` already applies `w_full()` and `min_w_0()` to its own row and stack.

## Grouping

`MessageGroup` is a styleable vertical stack for consecutive messages. It does
not decide which sender owns a message or automatically remove metadata:

```rust
MessageGroup::new()
    .child(
        Message::new()
            .avatar(Avatar::new().name("Alice").size_8())
            .header(MessageHeader::new().child("Alice"))
            .content(MessageContent::new().bubble(
                Bubble::new()
                    .with_variant(BubbleVariant::Secondary)
                    .child("The first message."),
            )),
    )
    .child(
        Message::new()
            .avatar_slot(MessageAvatar::new().bg(cx.theme().transparent))
            .content(MessageContent::new().bubble(
                Bubble::new()
                    .with_variant(BubbleVariant::Secondary)
                    .child("The follow-up keeps the same sender context."),
            )),
    )
```

Use `BubbleGroup` when only the bubbles are grouped and there is no message
header, avatar, or footer. Use `MessageGroup` when each item is a full row.

## Ghost surfaces and content insets

The typed `MessageContent::bubble(...)` builder records a ghost bubble. In that
case, `Message` removes the default header and footer insets so metadata lines
up with the unframed content:

```rust
Message::new()
    .header(MessageHeader::new().child("System").child("Just now"))
    .content(MessageContent::new().bubble(
        Bubble::new()
            .with_variant(BubbleVariant::Ghost)
            .child("The conversation has been archived."),
    ))
    .footer(MessageFooter::new().child("No further action required"))
```

Override this behavior explicitly on either named metadata slot:

```rust
MessageHeader::new()
    .content_inset(true)
    .child("Keep the regular header inset");

MessageFooter::new()
    .content_inset(false)
    .child("Align the footer with a custom surface")
```

`content_inset(...)` takes precedence over inherited ghost behavior. A typed
ghost bubble is required for automatic inheritance; an arbitrary child that
happens to look like a ghost surface cannot be inspected by `Message`.

The inner slot stack can also be refined independently:

```rust
Message::new()
    .with_stack_style(StyleRefinement::default().gap_3())
    .content(MessageContent::new().child("A wider message rhythm"))
```

## Custom styling and theme tokens

`Message`, `MessageGroup`, `MessageAvatar`, `MessageHeader`, `MessageContent`,
and `MessageFooter` implement `Styled`. Style the part that owns the visual
decision:

```rust
Message::new()
    .p_3()
    .rounded(cx.theme().radius_lg)
    .bg(cx.theme().muted.opacity(0.35))
    .header(MessageHeader::new().px_0().child("System"))
    .content(MessageContent::new().child("Archived"))
    .footer(MessageFooter::new().px_0().child("Just now"))
```

Use `with_stack_style(...)` for the vertical stack, slot refinements for
header/content/footer typography and spacing, and the child component's own
API for bubble, attachment, or button surfaces. Radius, spacing, typography,
and colors should come from the active semantic theme or shared scale. Avoid
raw colors at message call sites so the same composition works in light and
dark themes.

## Accessibility and state guidance

- Keep sender identity and message content in readable text. An avatar alone
  should not be the only indication of who sent a message.
- Put commands in semantic `Button` or `Link` controls. For the current
  `Button` API, use a visible `.label(...)` when a footer action needs an
  accessible name; a tooltip is supplemental.
- Delivery, failure, streaming, and unread states belong in text or semantic
  controls. Do not communicate them with alignment, color, or opacity alone.
- Preserve the header/footer inset when it is the visual relationship that
  aligns metadata with the surface. If a custom surface removes it, verify the
  reading order and keyboard order still match the visual order.
- Keep multiline content readable at the application's minimum window width;
  use `min_w_0()` on nested horizontal content and avoid hover-only actions.
- Motion for generated content belongs to `ShimmerText` or another motion-aware
  component. Reduced-motion behavior should leave the message text present and
  understandable.

## Component boundaries

The GPUI component intentionally does not add provider or domain layers:

- `Message` owns row alignment and slot layout.
- The application owns sender records, timestamps, delivery state, reactions,
  permissions, message IDs, and persistence.
- `Bubble`, `Attachment`, `Button`, `Link`, and `Marker` own their own visual or
  behavioral primitives and are composed through message slots.
- `MessageGroup` only supplies a vertical stack. It does not infer sender
  changes or collapse headers.

If a product needs a specific “assistant message” or “group chat message” with
fixed metadata policy, wrap `Message` in an application component. Keep that
domain policy out of the general-purpose primitive.

## API reference

### `Message`

| Method | Default | Purpose |
| --- | --- | --- |
| `new()` | `Start`, no slots | Create a message row. |
| `alignment(MessageAlignment)` | `Start` | Set leading or trailing alignment. |
| `with_stack_style(StyleRefinement)` | component stack defaults | Refine the inner vertical stack. |
| `avatar(element)` | none | Wrap an element in `MessageAvatar`. |
| `avatar_slot(MessageAvatar)` | none | Set a fully configured avatar slot. |
| `header(MessageHeader)` | none | Set sender and metadata content. |
| `content(MessageContent)` | none | Set the message body. |
| `footer(MessageFooter)` | none | Set delivery, reactions, or actions. |

`Message` also implements `Styled` for the outer row.

### `MessageGroup`

| Method | Default | Purpose |
| --- | --- | --- |
| `new()` | empty vertical stack | Create a message group. |
| `.child(element)` | — | Add complete messages. |
| `Styled` methods | `gap_2()` | Refine group spacing and layout. |

### `MessageAvatar`

| Method | Default | Purpose |
| --- | --- | --- |
| `new()` | empty circular `size_8` baseline | Create an identity slot. |
| `.child(element)` | — | Add Avatar or another identity element. |
| `Styled` methods | muted surface and full radius | Refine size, background, and alignment. |

### `MessageHeader` and `MessageFooter`

| Method | Default | Purpose |
| --- | --- | --- |
| `new()` | empty extra-small metadata row | Create the slot. |
| `content_inset(bool)` | inherited or `true` | Keep or remove the default `px_3()` inset. |
| `.child(element)` | — | Add text, metadata, reactions, or controls. |
| `Styled` methods | muted, medium-weight, `text_xs()` | Refine the slot. |

### `MessageContent`

| Method | Default | Purpose |
| --- | --- | --- |
| `new()` | empty full-width vertical stack | Create the body slot. |
| `bubble(Bubble)` | — | Add a typed bubble and propagate ghost metadata. |
| `.child(element)` | — | Add arbitrary rich content. |
| `Styled` methods | `min_w_0()`, `gap(rems(0.625))` | Refine body layout. |

### Related types

- [`MessageAlignment`] — `Start` or `End`.
- [`Bubble`] — conversational surface content.
- [`Attachment`] — files and media.
- [`MessageScroller`] — virtualized conversation rows and tail following.

[Message]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.Message.html
[MessageGroup]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageGroup.html
[MessageAvatar]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageAvatar.html
[MessageHeader]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageHeader.html
[MessageContent]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageContent.html
[MessageFooter]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageFooter.html
[MessageAlignment]: https://docs.rs/gpui-component/latest/gpui_component/message/enum.MessageAlignment.html
[Bubble]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.Bubble.html
[Attachment]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.Attachment.html
[MessageScroller]: https://docs.rs/gpui-component/latest/gpui_component/message_scroller/struct.MessageScroller.html
