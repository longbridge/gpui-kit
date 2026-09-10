---
title: Empty
description: Composable empty states with media, text, actions, and custom content.
---

# Empty

`Empty` presents missing content, empty results, and first-use states. Its named
slots provide the layout and visual hierarchy; the application decides when to
show it and owns the state and actions of its children.

The component is stateless and lives entirely in GPUI Component, using its
theme and native controls.

## Import

```rust
use gpui_kit::{ParentElement as _, Styled as _, rems};
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _,
    avatar::{Avatar, AvatarGroup},
    button::{Button, ButtonVariants as _},
    empty::{
        Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia,
        EmptyMediaVariant, EmptyTitle,
    },
    input::{Input, InputState},
    link::Link,
};
```

Import the component through `gpui_kit::component::empty`; GPUI's own
`gpui_kit::Empty` is a separate element that renders nothing.

## Basic usage

```rust
Empty::new()
    .header(
        EmptyHeader::new()
            .media(
                EmptyMedia::new()
                    .with_variant(EmptyMediaVariant::Icon)
                    .child(Icon::new(IconName::Folder)),
            )
            .title(EmptyTitle::new().child("No projects yet"))
            .description(
                EmptyDescription::new()
                    .child("Create your first project to get started."),
            ),
    )
    .content(
        EmptyContent::new()
            .flex_row()
            .flex_wrap()
            .justify_center()
            .gap_2()
            .child(Button::new("create-project").label("Create project…"))
            .child(
                Button::new("import-project")
                    .outline()
                    .label("Import project…"),
            ),
    )
    .child(
        Link::new("empty-help")
            .href("https://gpui-kit.com/docs/getting-started")
            .text_sm()
            .child("Learn more"),
    )
```

Attach normal Button callbacks for application actions. The extra root child
appears after `EmptyContent`, so a help link can remain separate from the
primary content group.

## Anatomy

| Part | Composition | Purpose |
| --- | --- | --- |
| `Empty` | `.header(EmptyHeader)`, `.content(EmptyContent)`, `.child(...)` | Overall alignment and spacing |
| `EmptyHeader` | `.media(EmptyMedia)`, `.title(EmptyTitle)`, `.description(EmptyDescription)` | Media and explanatory content |
| `EmptyMedia` | `.with_variant(...)`, `.child(...)` | Icon, image, avatar, or arbitrary media |
| `EmptyTitle` | `.child(...)` | Title text or custom content |
| `EmptyDescription` | `.child(...)` | Wrapping text or rich supporting content |
| `EmptyContent` | `.child(...)` | Actions, inputs, or other controls |

All parts have `new()` and `Default` constructors and implement `Styled`.
All parts except `EmptyHeader` implement `ParentElement`.

Named slots are optional and have replacement semantics: calling `.header(...)`
twice keeps the second header. Rendering always places the header before the
content, and media before title before description, regardless of the order in
which those setters are called. Direct root children are appended after both
named slots, in their own insertion order; they are not inserted into the content
slot. Replacing a slot leaves the other slots and extra root children intact.

## Outline

The default Empty has a transparent background and no visible border. Add a
border through `Styled`; its default border style is dashed.

```rust
Empty::new()
    .border_1()
    .header(
        EmptyHeader::new()
            .title(EmptyTitle::new().child("Cloud storage is empty"))
            .description(
                EmptyDescription::new()
                    .child("Upload files to access them anywhere."),
            ),
    )
```

Use `.border_color(...)` to refine its semantic color.

## Background

Apply a semantic surface directly, without adding a component variant:

```rust
Empty::new()
    .bg(cx.theme().muted.opacity(0.3))
    .header(
        EmptyHeader::new()
            .title(EmptyTitle::new().child("No notifications"))
            .description(
                EmptyDescription::new()
                    .child("New notifications will appear here."),
            ),
    )
```

## Avatar

The default media variant adds no frame, background, or fixed size. An existing
Avatar retains its own image, fallback, size, and appearance.

```rust
EmptyHeader::new()
    .media(
        EmptyMedia::new().child(
            Avatar::new()
                .name("Alex Morgan")
                .src("https://avatars.githubusercontent.com/u/5518?v=4"),
        ),
    )
    .title(EmptyTitle::new().child("Alex is offline"))
    .description(
        EmptyDescription::new()
            .child("Leave a message for Alex to read when they're back."),
    )
```

## Avatar group

Multiple avatars use the same media slot. The group owns avatar overlap and
size; Empty does not inspect or modify its children.

```rust
EmptyHeader::new()
    .media(
        EmptyMedia::new().child(
            AvatarGroup::new()
                .child(Avatar::new().name("Alex Morgan"))
                .child(Avatar::new().name("Taylor Lee"))
                .child(Avatar::new().name("Sam Chen")),
        ),
    )
    .title(EmptyTitle::new().child("No team members"))
    .description(
        EmptyDescription::new()
            .child("Invite your team to collaborate on this project."),
    )
```

## Inputs and custom content

Retain an `Entity<InputState>` in the owning view, then compose the existing
Input in `EmptyContent`:

```rust
EmptyContent::new()
    .child(
        Input::new(&self.search)
            .prefix(Icon::new(IconName::Search).size_4())
            .cleanable(true),
    )
    .child(
        EmptyDescription::new()
            .child("Search by name or try a different keyword."),
    )
```

The application handles input events and switches between results and Empty.
Each rendered input retains its own state entity and focus. Empty has no input
state, validation, submission, or loading policy.

## Constrained layouts

Refine the root and the individual slots together to build a compact,
leading-aligned empty state:

```rust
Empty::new()
    .max_w(rems(20.))
    .p_4()
    .items_start()
    .text_left()
    .header(
        EmptyHeader::new()
            .items_start()
            .title(EmptyTitle::new().child("No shared files"))
            .description(
                EmptyDescription::new()
                    .child("Add files so your team can review and edit them together."),
            ),
    )
    .content(
        EmptyContent::new()
            .items_start()
            .child(Button::new("add-files").outline().label("Add files…")),
    )
```

The root fills the available width and can grow within a flex layout. Header
and content use the available width up to 24 rem. Text wraps naturally, and
Empty does not clip its children or own a scroll region. The parent supplies
the viewport and any required scrolling. Custom media should fit its container;
action rows can use `.flex_wrap()` when space is constrained.

## Styling defaults

| Part | Default |
| --- | --- |
| Root | Centered column, `p_6()`, `gap_4()`, theme `radius_tokens().xl` |
| Header | `gap_2()`, centered items, maximum width 24 rem |
| Media | Centered column sized to its content, `mb_2()`, does not shrink |
| Icon media | `size_8()`, muted background, foreground text, theme `radius_tokens().lg` |
| Title | `text_sm()`, medium weight |
| Description | `text_sm()`, line height 1.625, muted foreground |
| Content | Centered column, `gap_2p5()`, `text_sm()`, maximum width 24 rem |

Instance styles override defaults and media-variant styles. Icon media supplies
a one-rem font size that an unsized GPUI Component `Icon` inherits; an explicit
icon size is preserved. Arbitrary SVG/image children keep their own sizing.
Typography uses the application's font and rem scale. GPUI's native wrapping
and letter spacing apply; CSS `text-balance` and `tracking-tight` are not
reimplemented by this component.

Empty does not create focus targets or automatically announce itself as an
alert or live status. Its Button and Input children retain their normal focus
and keyboard behavior. Choose application commands as Buttons and external
resources as Links.
