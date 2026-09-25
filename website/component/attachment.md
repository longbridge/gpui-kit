---
title: Attachment
description: A composable file and media attachment surface with lifecycle states, previews, and actions.
---

# Attachment

`Attachment` presents one file or media item. It provides stable layout for a
media preview, metadata, and optional actions, draws the lifecycle status, and
offers the two controls every composer needs — remove and retry — while leaving
upload state, selection, and navigation in the application. Each public slot is
styleable and accepts arbitrary GPUI children.

The component is intentionally a composition primitive. `AttachmentActions`
does not invent an attachment-specific action model; put `Button`, `Link`, or
another semantic control inside it. `AttachmentGroup` only owns horizontal
spacing and scrolling. Selection and preview behavior remain application
concerns.

## Import

```rust
use gpui_kit::{Axis, ParentElement as _, Styled as _};
use gpui_kit::component::{
    ActiveTheme as _, Colorize as _, Icon, IconName, Sizable as _, Size,
    attachment::{
        Attachment, AttachmentActions, AttachmentContent, AttachmentDescription,
        AttachmentGroup, AttachmentMedia, AttachmentStatus, AttachmentTitle,
    },
    button::{Button, ButtonVariants as _},
    badge::Badge,
    progress::Progress,
    shimmer::ShimmerStyle,
    spinner::Spinner,
};
```

## Anatomy and basic usage

The typed builders make the common file shape explicit:

```rust
Attachment::new()
    .media(AttachmentMedia::new().child(Icon::new(IconName::FileText)))
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("quarterly-report.pdf"))
            .description(AttachmentDescription::new("PDF · 2.4 MB")),
    )
    .actions(
        AttachmentActions::new().child(
            Button::new("remove-report")
                .ghost()
                .xsmall()
                .icon(IconName::Close)
                .label("Remove"),
        ),
    )
```

The slots are optional. A media-only attachment, metadata-only attachment, or
action-only attachment is valid when the product needs it:

```rust
Attachment::new()
    .media(AttachmentMedia::new().child(Icon::new(IconName::FileText)));

Attachment::new().content(
    AttachmentContent::new()
        .title(AttachmentTitle::new("notes.txt"))
        .description(AttachmentDescription::new("TXT · 12 KB")),
)
```

The default state is:

| Property | Default | Meaning |
| --- | --- | --- |
| Status | `Complete` | The item is ready. |
| Size | `Medium` | Uses the standard conversation density. |
| Axis | `Horizontal` | Media, metadata, and actions share one row. |
| Media/content/actions | absent | Add only the slots the item needs. |
| Surface | `background` and `foreground` | The card surface, separated by the border like shadcn's `bg-card`. |
| Radius | `radius_tokens().lg` (`md` for `XSmall`) | Shared semantic radius. |
| Geometry | 56 px tall, 232 px wide with content, 38 px media (`Medium`) | The composer chip; see [Sizes and axes](#sizes-and-axes). |

`Attachment` never owns a product-level file model. Keep the file ID and state
in the parent view, then render the current record into this element.

## Media and image previews

Use children for an icon-style media slot and `src(...)` for an image preview:

```rust
Attachment::new()
    .media(
        AttachmentMedia::new()
            .src("https://example.com/previews/sdk.svg")
            .overlay(Icon::new(IconName::Download)),
    )
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("sdk-preview.svg"))
            .description(AttachmentDescription::new("SVG · 1280 × 720")),
    )
```

The image is rendered with `ObjectFit::Cover` inside the media bounds. Children
and `overlay(...)` are painted above the image. `overlay(...)` centers an
element over the whole media area, which is useful for a spinner, play icon, or
preview action:

```rust
Attachment::new()
    .status(AttachmentStatus::Uploading)
    .axis(Axis::Vertical)
    .media(
        AttachmentMedia::new()
            .src(preview_url)
            .overlay(Spinner::new().small()),
    )
```

The slot draws the lifecycle status itself. A source image keeps its colors
and takes a scrim: a translucent dark layer with a white spinner while
`Uploading` or `Processing`, a darker one with the retry control (see
[Remove and retry controls](#remove-and-retry-controls)) or an alert glyph once
`Failed`. Custom overlays are painted above the scrim. With no source, the
media slot is a themed muted area that shows a spinner in the primary color
while in progress and, once failed, the destructive semantic surface and
foreground with an alert glyph; its children come back with `Complete`.

An image tile — a vertical attachment without content — is a square the media
fills edge to edge, its corners one border width tighter than the card's so the
two stay concentric.

`AttachmentMedia` is independently styleable. Use `with_size(...)` to override
the inherited media size, or use normal GPUI refinements for a custom preview
ratio and surface:

```rust
AttachmentMedia::new()
    .with_size(Size::Large)
    .aspect_ratio(16. / 9.)
    .rounded(cx.theme().radius_lg)
    .child(Icon::new(IconName::Image))
```

An explicit media size takes precedence over the attachment size. A vertical
attachment makes the media full width and square by default; the media's own
style can replace that geometry when the application has a different preview
design.

## Lifecycle states

`AttachmentStatus` has five explicit states. The parent status is passed to the
typed title, description, media, and action layout during rendering:

| State | Surface/layout behavior | Recommended content |
| --- | --- | --- |
| `Pending` | Dashed border; preview is not dimmed. | “Ready to upload” and a start action. |
| `Uploading` | Media shows a spinner, or a progress ring with `progress(...)` (over a scrim on an image); a chip draws a bar along its bottom edge; typed title shimmers. | A description such as “Uploading”; the percentage is appended for you. |
| `Processing` | Media shows a spinner (over a scrim on an image); typed title shimmers. | “Processing…” and a non-destructive wait state. |
| `Failed` | Destructive border and description; media shows the retry control or alert glyph with `on_retry`, the ban glyph without one (a rejection). | Error reason plus `on_retry`, or `tooltip(...)` with the reason and `on_remove`. |
| `Complete` | Ready surface; preview is full opacity. | File metadata and normal actions. |

```rust
Attachment::new()
    .status(AttachmentStatus::Uploading)
    .media(AttachmentMedia::new().child(Icon::new(IconName::FileText)))
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("design-assets.zip"))
            .description(AttachmentDescription::new("Uploading · 68%"))
            .child(Progress::new("attachment-progress").value(68.)),
    )
    .actions(
        AttachmentActions::new()
            .child(Button::new("cancel-upload").ghost().xsmall().label("Cancel")),
    )
```

The status helpers are useful when application state maps to presentation:

```rust
match status {
    AttachmentStatus::Pending => "Ready to upload",
    AttachmentStatus::Uploading => "Uploading…",
    AttachmentStatus::Processing => "Processing…",
    AttachmentStatus::Failed => "Upload failed",
    AttachmentStatus::Complete => "Ready",
}
```

`is_pending()`, `is_uploading()`, `is_processing()`, `is_failed()`,
`is_complete()`, and `is_in_progress()` are pure readers. They do not update
the attachment or the application upload task.

## Status inheritance and overrides

Titles and descriptions added through their typed builders inherit the parent
status. An explicit child status wins over the inherited value:

```rust
Attachment::new()
    .status(AttachmentStatus::Failed)
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("archive.zip"))
            .description(
                AttachmentDescription::new("Previous upload completed")
                    .status(AttachmentStatus::Complete),
            ),
    )
```

Use typed `.title(...)` and `.description(...)` whenever loading shimmer or
failure coloring should follow the attachment. The generic `.child(...)` form
still accepts arbitrary elements, but it cannot inspect the erased child's
status and therefore does not inherit automatically:

```rust
AttachmentContent::new()
    .title(AttachmentTitle::new("status-aware-title"))
    .description(AttachmentDescription::new("status-aware-description"))
    .child(custom_metadata_element)
```

Customize an in-progress title with a reusable shimmer style:

```rust
AttachmentTitle::new("transcript.pdf")
    .with_shimmer_style(
        ShimmerStyle::new()
            .duration(std::time::Duration::from_secs(3))
            .spread(0.45)
            .reverse(true)
            .once(false),
    )
```

`AttachmentDescription` uses the destructive semantic color only for an
explicit or inherited `Failed` status. The words in the description should
still state what happened; color is a supporting cue.

## Sizes and axes

`Attachment` implements `Sizable`. The convenience builders map to `Size`:

```rust
Attachment::new().xsmall();
Attachment::new().small();
Attachment::new(); // medium (default)
Attachment::new().large();
Attachment::new().w_auto() // let the chip hug its content instead of the fixed width
```

The named sizes set the whole geometry as one scale, in rems so it follows the
root font size:

| Size | Chip height | Chip width | Media | Title |
| --- | --- | --- | --- | --- |
| `XSmall` | 40 px | 176 px | 28 px | 11 px |
| `Small` | 48 px | 200 px | 32 px | 12 px |
| `Medium` | 56 px | 232 px | 38 px | 13 px |
| `Large` | 64 px | 272 px | 44 px | 14 px |

A horizontal card takes the fixed width only when it carries content, so a row
of chips lines up and long names truncate instead of stretching the card. An
image tile is a square with the chip's height. `Size::Size(...)` scales the
`Medium` geometry from a custom base value. Refine with the normal GPUI width
methods (`w_auto()`, `w_full()`, `w(...)`) when the product needs another
measure; named sizes are preferable for a coherent theme.

Horizontal is the default and keeps the media, metadata, and actions in one
row. Vertical moves the preview above the metadata and places actions over the
preview's upper trailing corner:

```rust
Attachment::new()
    .axis(Axis::Vertical)
    .large()
    .media(AttachmentMedia::new().src(preview_url))
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("presentation.png"))
            .description(AttachmentDescription::new("PNG · 1920 × 1080")),
    )
    .actions(
        AttachmentActions::new()
            .child(Button::new("remove-presentation").ghost().xsmall().label("Remove")),
    )
```

The vertical default is square media. Set a media aspect ratio or size when
the content needs a landscape preview. `AttachmentContent` and
`AttachmentActions` remain independent slots, so an application can omit one
or place additional controls in either.

## Content and actions

`AttachmentContent` keeps titles and descriptions in a vertical metadata stack.
It also accepts custom children for progress, badges, or a second line:

```rust
AttachmentContent::new()
    .title(AttachmentTitle::new("report.pdf"))
    .description(AttachmentDescription::new("PDF · 2.4 MB"))
    .child(Badge::new().count(3))
```

Use `AttachmentActions` for one or more existing semantic controls:

```rust
AttachmentActions::new()
    .child(Button::new("download").ghost().xsmall().label("Download"))
    .child(Button::new("remove").danger().xsmall().label("Remove"))
```

`AttachmentActions` only supplies layout and does not make its children
focusable, clickable, or disabled. A tooltip is supplemental; the current
`Button` implementation derives its accessibility label from `.label(...)`, so
use a visible label when an action must have a named accessible control. An
icon-only button with only `.tooltip(...)` is not a substitute for that label.

## Whole-card click

Set `.id(...)` and `.on_click(...)` to make the whole card activate, e.g. to
open a preview. The click layer is painted below `AttachmentActions`, so action
buttons stay independently clickable:

```rust
Attachment::new()
    .id("design-attachment")
    .on_click(|_, window, cx| {
        // Open the preview.
    })
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("design-mockups.png"))
            .description(AttachmentDescription::new("PNG · 1.8 MB")),
    )
    .actions(
        AttachmentActions::new()
            .child(Button::new("remove").ghost().xsmall().icon(IconName::Close)),
    )
```

The handler takes effect only together with `.id(...)`; click state needs that
stable identity. A clickable card shows a muted hover surface so it reads as
interactive. What activation means — a dialog, a browser, a file viewer, or a
selection — stays with the application. Keep destructive and secondary commands
in `AttachmentActions` so they never depend on the card's primary activation,
and offer the card's primary action as a `Button` or `Link` somewhere reachable
from the keyboard: the click layer itself is a pointer convenience and takes no
focus.

## Remove and retry controls

A composer removes attachments and retries failed uploads. Both controls are
built in, so they look the same in every product and need no wrapper:

```rust
Attachment::new()
    .id(("attachment", item.id))
    .status(item.status)
    .on_remove(cx.listener(move |this, _, _, cx| this.remove(item.id, cx)))
    .on_retry(cx.listener(move |this, _, _, cx| this.retry(item.id, cx)))
    .axis(Axis::Vertical)
    .media(AttachmentMedia::new().src(thumbnail))
```

`on_remove` rides a small surface-colored disc with a hairline border on the
card's upper trailing corner, the way a card's close control usually looks. It appears on hover on
desktop and stays visible on touch platforms; the card reserves the overhang,
so a row of cards keeps its alignment. `on_retry` takes effect only while the
status is `Failed`: an image preview gets a round button in its scrim, and a
typed description gets a localized “Retry” link after its text. A failed
attachment without `on_retry` reads as a rejection and shows the ban glyph
instead. All of these key their element state on `.id(...)`, so they take
effect only together with it. What removing or retrying means stays with the
application.

Two more builders complete the composer picture:

```rust
Attachment::new()
    .id(("attachment", item.id))
    .status(AttachmentStatus::Uploading)
    .progress(item.percent)               // 0..=100
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("Q3 statement.pdf"))
            .description(AttachmentDescription::new("Uploading")),
    );

Attachment::new()
    .id(("attachment", item.id))
    .status(AttachmentStatus::Failed)
    .tooltip("Image exceeds 20 MB limit · Remove to send")
    .on_remove(cx.listener(move |this, _, _, cx| this.remove(item.id, cx)))
    .axis(Axis::Vertical)
    .media(AttachmentMedia::new().src(thumbnail))
```

`progress(percent)` turns the uploading spinner into a determinate ring, draws
a thin primary bar along a horizontal card's bottom edge, and appends
“· 62%” to a typed description; it is ignored in every other status.
`tooltip(text)` shows the text while the card is hovered, which is where the
reason for a failure or a rejection belongs.

## Groups

`AttachmentGroup` provides a horizontally scrollable row with the shared group
gap. Its ID is required because it owns GPUI's element-local scroll state:

```rust
AttachmentGroup::new("message-attachments")
    .child(first_attachment)
    .child(second_attachment)
    .child(third_attachment)
```

The group is `w_full()`, `min_w_0()`, and uses horizontal scrolling. It does not
provide selection, snapping, reorder handles, a “+N more” overflow label, or a
preview dialog. Compose those behaviors in an application-owned wrapper. Keep
the ID stable for the lifetime of the conversation row.

Two builders help a composer or message row that overflows:

```rust
AttachmentGroup::new("composer-attachments")
    // Fade each edge into the surface behind the row while it hides content.
    .with_edge_fade(cx.theme().background)
    // Drive the scrolling yourself, e.g. from paging buttons.
    .track_scroll(&self.attachments_scroll)
    .children(attachments)
```

`with_edge_fade(color)` draws a short gradient at an edge only while more
attachments continue past it; a row that fits shows none. The fades sit above
the attachments and take no pointer events. `track_scroll(&handle)` replaces
the group's own scroll state with the caller's `ScrollHandle`, so the
application can move the row and read its offset.

## Custom styling and theme tokens

`Attachment`, `AttachmentGroup`, and every named slot implement `Styled`.
Refinements are applied after component defaults, which gives developers control
over the surface, spacing, media geometry, typography, and action layout:

```rust
Attachment::new()
    .w_full()
    .rounded(cx.theme().radius_lg)
    .bg(cx.theme().group_box)
    .border_color(cx.theme().ring)
    .media(
        AttachmentMedia::new()
            .rounded(cx.theme().radius_lg)
            .bg(cx.theme().primary.opacity(0.12))
            .text_color(cx.theme().primary)
            .child(Icon::new(IconName::FileText)),
    )
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("custom-theme.json").text_color(cx.theme().primary))
            .description(AttachmentDescription::new("JSON · 16 KB")),
    )
```

Prefer semantic roles from `cx.theme()` (`background`, `muted`, `border`,
`destructive`, `foreground`, and their foreground counterparts) to raw colors.
The component's default radii, spacing, and typography follow the shared design
scale; application-specific density can be expressed with `Size` and typed
style refinements at the composition boundary.

Use `AttachmentContent::title(...)` and `.description(...)` for status-aware
metadata, `.child(...)` for arbitrary custom content, child `.status(...)` for
an explicit override, `AttachmentTitle::with_shimmer_style(...)` for loading
motion, and `AttachmentMedia::overlay(...)` for controls above an image.

## Accessibility and state guidance

- Include the file name and useful type/size information in text. An icon-only
  media preview is not enough to identify the attachment.
- Put upload, retry, remove, download, and preview actions in semantic
  `Button` or `Link` controls. A tooltip is supplemental; for the current
  `Button` API, use `.label(...)` when the action needs an accessible name.
- Describe `Pending`, `Uploading`, `Processing`, and `Failed` in text or a
  control state. The dashed border, opacity, shimmer, and destructive color are
  supporting cues.
- Keep progress determinate when the application knows a byte or item count;
  use `Progress` as a child rather than duplicating progress semantics in
  `Attachment`.
- Loading shimmer is disabled by `ShimmerText` when reduced motion is enabled.
  Keep a readable title and description visible in that mode.
- Ensure a vertical overlay action remains reachable from the keyboard; it must
  not be available only through image hover.

## Component boundaries

These boundaries are deliberate:

- Use `Button` directly instead of an attachment-specific action component.
  This preserves Button variants, sizes, loading, disabled behavior, focus, and
  event handling.
- Use `Progress` directly instead of an attachment-specific progress wrapper.
- Use `.id(...)` with `.on_click(...)` for whole-card activation. The card only
  reports the click; whether that opens a dialog, a browser, a file viewer, or
  toggles a selection stays with the application.
- Use `AttachmentGroup` only for the shared horizontal row and overflow. Use an
  application-owned container for selection, reordering, snapping, or custom
  scroll controls.

## API reference

### `Attachment`

| Method | Default | Purpose |
| --- | --- | --- |
| `new()` | `Complete`, `Medium`, `Horizontal`, no slots | Create an attachment. |
| `id(ElementId)` | none | Stable identity for the built-in controls: click layer, remove, retry. |
| `on_click(handler)` | none | Whole-card activation; requires `id(...)` and stays below the actions. |
| `on_remove(handler)` | none | Corner remove control; requires `id(...)`. |
| `on_retry(handler)` | none | Retry control while `Failed`; requires `id(...)`. |
| `progress(percent)` | none | Determinate ring, bottom bar and “· 62%” while `Uploading`. |
| `tooltip(text)` | none | Hover tooltip, e.g. the failure reason; requires `id(...)`. |
| `status(AttachmentStatus)` | `Complete` | Set lifecycle styling. |
| `axis(Axis)` | `Horizontal` | Choose horizontal or vertical layout. |
| `with_size(Size)` | `Medium` | Set a named or custom size. |
| `xsmall()` / `small()` / `large()` | — | Sizable shortcuts. |
| `media(AttachmentMedia)` | none | Add a preview slot. |
| `content(AttachmentContent)` | none | Add metadata. |
| `actions(AttachmentActions)` | none | Add action controls. |

### `AttachmentMedia`

| Method | Default | Purpose |
| --- | --- | --- |
| `new()` | no source, no children | Create a media slot. |
| `src(ImageSource)` | none | Render an image preview. |
| `with_size(Size)` | inherited attachment size | Override media density. |
| `overlay(element)` | none | Center an element over the media, above the status treatment. |
| `child(element)` | — | Add an icon or custom content above the preview. |
| `Styled` methods | themed muted media | Refine geometry, radius, background, and typography. |

### `AttachmentContent`, `AttachmentTitle`, and `AttachmentDescription`

| Method | Default | Purpose |
| --- | --- | --- |
| `AttachmentContent::new()` | empty vertical metadata stack | Create content. |
| `.title(AttachmentTitle)` | — | Add a status-aware single-line title. |
| `.description(AttachmentDescription)` | — | Add a status-aware single-line description. |
| `AttachmentTitle::new(text)` | no explicit child status | Create a title. |
| `AttachmentTitle::status(status)` | inherits parent | Override title loading state. |
| `AttachmentTitle::with_shimmer_style(style)` | default shimmer | Customize title animation. |
| `AttachmentDescription::new(text)` | no explicit child status | Create a description. |
| `AttachmentDescription::status(status)` | inherits parent | Override description color state. |
| `.child(element)` | — | Add progress, badges, or custom metadata. |

### `AttachmentActions` and `AttachmentGroup`

| Method | Default | Purpose |
| --- | --- | --- |
| `AttachmentActions::new()` | empty action layout | Create the action slot. |
| `.child(element)` | — | Add Button, Link, or another control. |
| `AttachmentGroup::new(id)` | stable ID required | Create a horizontal scrolling group. |
| `AttachmentGroup::track_scroll(&ScrollHandle)` | own scroll state | Scroll the row through the caller's handle. |
| `AttachmentGroup::with_edge_fade(color)` | none | Fade an edge into `color` while it hides attachments. |
| `AttachmentGroup::child(element)` | — | Add attachments to the group. |

### Related types

- [`AttachmentStatus`] — `Pending`, `Uploading`, `Processing`, `Failed`, and
  `Complete`.
- [`Size`] — `XSmall`, `Small`, `Medium`, `Large`, or a custom `Pixels` value.
- [`Axis`] — `Horizontal` or `Vertical` from GPUI.
- [`ShimmerStyle`] — shared loading animation configuration.

[Attachment]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.Attachment.html
[AttachmentMedia]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentMedia.html
[AttachmentContent]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentContent.html
[AttachmentTitle]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentTitle.html
[AttachmentDescription]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentDescription.html
[AttachmentActions]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentActions.html
[AttachmentGroup]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentGroup.html
[AttachmentStatus]: https://docs.rs/gpui-component/latest/gpui_component/attachment/enum.AttachmentStatus.html
[Size]: https://docs.rs/gpui-component/latest/gpui_component/enum.Size.html
[Axis]: https://docs.rs/gpui/latest/gpui/enum.Axis.html
[ShimmerStyle]: https://docs.rs/gpui-component/latest/gpui_component/shimmer/struct.ShimmerStyle.html
