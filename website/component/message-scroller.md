---
title: MessageScroller
description: A virtualized message list with tail following, history insertion, unread navigation, and customizable jump controls.
---

# MessageScroller

`MessageScroller` coordinates a variable-height virtual list with the behavior
conversation screens usually need: follow the live tail, keep the current
anchor while older history is inserted, navigate to an unread row, and expose
whether the reader has left the tail.

The application owns the message collection, stable message IDs, unread
meaning, row renderer, composer, and empty/error states. The component owns only
virtual-list bookkeeping and the optional jump-to-latest affordance. There is
one state entity per scroller.

## Import

```rust
use std::{rc::Rc, time::Duration};

use gpui_kit::{
    IntoElement as _, ParentElement as _, StyleRefinement, Styled as _,
    prelude::FluentBuilder as _,
};
use gpui_kit::component::{
    ActiveTheme as _,
    button::ButtonVariants as _,
    message_scroller::{MessageScroller, MessageScrollerState},
    Sizable as _,
    v_flex,
};
```

## Create state and choose the starting position

Create the state beside the application-owned message vector. The constructor
receives the entity context because GPUI's list scroll handler notifies the
entity after the list releases its internal borrow:

```rust
let scroller = cx.new(|cx| MessageScrollerState::new(messages.len(), cx));
cx.observe(&scroller, |_, _, cx| cx.notify()).detach();
```

`MessageScrollerState::new(...)` starts with tail following enabled. That is
the expected starting position for a live conversation. A saved thread or a
deep link can choose a row after the initial data has been installed:

```rust
let saved_index = messages
    .iter()
    .position(|message| message.id == saved_message_id)
    .unwrap_or(messages.len().saturating_sub(1));

scroller.update(cx, |state, cx| {
    state.reset(messages.len(), cx);
    let _ = state.scroll_to_item(saved_index, cx);
});
```

There is no `starting_position` field or persisted scroll-offset API. Keep the
application's saved message ID or index, then resolve it to the current index
after records are loaded. `reset(...)` replaces the known row count and
re-engages tail following; call `scroll_to_item(...)` after it when the product
needs a different initial row.

## Render rows and an empty state

Pass an indexed renderer. GPUI virtualizes the rows, so the closure is called
for the rows needed by the current viewport and overdraw region:

```rust
let messages = Rc::new(messages.clone());

MessageScroller::new(
    "conversation",
    scroller.clone(),
    move |index, _window, _cx| {
        let Some(message) = messages.get(index) else {
            return gpui_kit::div().into_any_element();
        };

        gpui_kit::div()
            .id(("message-row", message.id))
            .min_w_0()
            .child(render_message(message))
            .into_any_element()
    },
)
.w_full()
.h_96()
```

The row ID in this example belongs to the application. `MessageScroller` does
not retain an index-to-ID map; the ID lets an application-owned row keep its
own element-local state when data changes.

An empty list is valid (`MessageScrollerState::new(0, cx)`), but the scroller
does not invent an empty placeholder. Render the empty, loading, error, or
permission-denied state in the surrounding view and mount the scroller once
there are rows:

```rust
if messages.is_empty() {
    empty_conversation_view.into_any_element()
} else {
    MessageScroller::new("conversation", scroller.clone(), render_message)
        .into_any_element()
}
```

Keep the empty state separate from the scroll region so it can provide a
meaningful action such as “Start a new conversation” without pretending there
is a message to scroll to.

## Append, streaming, and follow-tail behavior

Update application data and virtual-list count together. Appending while the
reader follows the tail keeps the latest row visible. Appending after the
reader scrolls up preserves the reader's position and makes the built-in jump
button available:

```rust
messages.push(new_message);
scroller.update(cx, |state, cx| {
    let _ = state.append(1, cx);
});
cx.notify();
```

Streaming token growth changes a row's height without changing the item count.
Update the message body, then remeasure that row:

```rust
messages[index].body.push_str(next_token);
scroller.update(cx, |state, cx| {
    let _ = state.remeasure_items(index..index + 1, cx);
});
cx.notify();
```

`remeasure_items(...)` preserves an item anchor while recalculating the selected
rows. Use `remeasure(...)` after a global width, typography, or theme change
that can affect many row heights. When streaming creates a new message rather
than growing an existing one, call `append(1, cx)` first; remeasure the row
again if its first render and later content have different heights.

The state readers make the follow-tail decision visible to the surrounding
view:

```rust
let following_tail = scroller.read(cx).is_following_tail();
let show_new_messages = scroller.read(cx).is_scrolled_up();
```

`is_following_tail()` is true when the list follows appended content.
`is_scrolled_up()` is true when there is scrollable content below the current
viewport and the reader is away from the end. The component does not decide
whether to show a toast, unread count, or “new messages” copy; use these readers
to drive an application-owned indicator.

Resume following and move to the latest row explicitly:

```rust
scroller.update(cx, |state, cx| state.scroll_to_end(cx));
```

Normal scrolling to the end also allows GPUI's list to resume tail following.

## Prepend earlier history

Insert older records at the front and tell the state the number of inserted
rows. `prepend(...)` uses GPUI list splicing to preserve the visible item
anchor:

```rust
let earlier_messages = load_earlier_messages();
let count = earlier_messages.len();
messages.splice(0..0, earlier_messages);

scroller.update(cx, |state, cx| {
    let _ = state.prepend(count, cx);
});
cx.notify();
```

Use `splice(old_range, count, cx)` for replacements or deletions elsewhere in
the collection. The range is half-open and must stay within the current item
count; invalid ranges return `false` and leave the state unchanged.

For a “Load earlier” control, keep the loading state in the application, fetch
the records, splice the vector, then call `prepend`. Do not call `reset` for
ordinary history pagination because reset intentionally returns to tail
following and loses the incremental anchor semantics.

## Unread and arbitrary navigation

Unread identity belongs to the application. Resolve a stable message ID to the
current vector index, then use `scroll_to_item(...)`:

```rust
if let Some(index) = messages.iter().position(|message| message.id == unread_id) {
    scroller.update(cx, |state, cx| {
        let _ = state.scroll_to_item(index, cx);
    });
}
```

`scroll_to_item(...)` returns `false` for an out-of-range index. It is the
single navigation primitive: an unread boundary, a search result, a bookmarked
message, a reply target, and a deep link all resolve to an index in the
application first.

The current API does not expose turn anchors, peek previews, visible IDs, or
stable-ID navigation. Map domain IDs to the current index in the application;
keep an index map if lookup cost matters. The scroller does not know whether a
row is a turn, a reply, an unread boundary, or a search result.

## Dynamic row heights and structural updates

Rows may contain multiline text, attachments, streamed content, or an
application-owned composer and can therefore have different heights. The
underlying GPUI list measures rendered rows. Keep the renderer's height-affecting
data in the owning view and notify the state after a mutation:

| Change | State operation |
| --- | --- |
| Add rows at the tail | `append(count, cx)` |
| Add rows at the front | `prepend(count, cx)` |
| Replace/delete a range | `splice(range, count, cx)` |
| Token growth in known rows | `remeasure_items(range, cx)` |
| Global width/font/theme change | `remeasure(cx)` |
| Replace the whole conversation | `reset(item_count, cx)` |

Do not mutate the vector length without the matching state operation. The
renderer receives an index, so data and virtual-list count must remain aligned
for the same render pass.

## Jump-to-latest controls

The built-in jump button is enabled by default and appears when
`is_scrolled_up()` becomes true. It is a configured `Button` with a secondary
variant, icon-button sizing, full radius, arrow-down icon, theme border/background,
and a localized tooltip label. It keeps the scroll action owned by the state:

```rust
MessageScroller::new("conversation", scroller.clone(), render_message)
    .with_jump_button_label("Jump to newest")
    .with_jump_button_transition(Duration::from_millis(250))
```

Use `Duration::ZERO` to disable the enter/leave transition. Reduced-motion
preferences use the final state immediately regardless of the configured
duration.

Refine its style after the built-in defaults:

```rust
MessageScroller::new("conversation", scroller.clone(), render_message)
    .with_jump_button_style(
        StyleRefinement::default()
            .bg(cx.theme().primary)
            .border_color(cx.theme().primary)
            .text_color(cx.theme().primary_foreground),
    )
```

Use the renderer callback when the application needs a different Button
variant, size, icon, or instance style. The callback receives the fully
configured button and must return a `Button`; the built-in scroll action stays
attached:

```rust
MessageScroller::new("conversation", scroller.clone(), render_message)
    .with_jump_button_renderer(|button| button.outline().small().label("Latest"))
```

During its leave transition the built-in button is rendered disabled while its
opacity reaches zero. A renderer should preserve that state rather than force a
disabled button to be active. The button's accessible name comes from its
`.label(...)` value; `with_jump_button_label(...)` supplies the tooltip label
only. Set a visible label in `with_jump_button_renderer(...)` when the jump
action needs a named accessible control. The current public renderer callback
can change Button styling and content, but it does not expose a separate
accessibility-label builder.

Disable the built-in affordance when the surrounding view provides its own:

```rust
MessageScroller::new("conversation", scroller.clone(), render_message)
    .jump_button(false)
```

Compose an application-owned button from `is_scrolled_up()` and
`scroll_to_end(...)` when its placement, text, or accessibility contract needs
to be product-specific.

## Scrollbar and style slots

The root implements `Styled`, and the internal regions have separate style
refinements:

```rust
MessageScroller::new("conversation", scroller.clone(), render_message)
    .p_2()
    .bg(cx.theme().group_box)
    .with_content_style(StyleRefinement::default().bg(cx.theme().background))
    .with_list_style(StyleRefinement::default().p_4())
    .with_row_style(StyleRefinement::default().pb_6())
    .scrollbar(false)
```

The boundaries are:

- Root `Styled` methods refine the full-width element that owns the viewport.
- `with_content_style(...)` refines the viewport containing the list and
  optional vertical scrollbar.
- `with_list_style(...)` refines the GPUI virtual list after its default
  `px_3()` / `py_2()` padding. GPUI lists offset rows only vertically, so the
  horizontal padding component — the default and any refinement — is carried
  by every row wrapper.
- `with_row_style(...)` refines the full-width wrapper around each rendered row;
  the default wrapper includes `pb_8()` between rows, like a CSS gap. The
  list's own bottom padding owns the gap between the last row and whatever
  sits below the transcript.
- `scrollbar(false)` hides the built-in vertical scrollbar; it does not disable
  scrolling or remove keyboard/wheel interaction.
- `with_bottom_fade(color)` fades the transcript's bottom edge into the given
  color, so a partially visible row melts into the surrounding surface instead
  of clipping mid-line. It shows only while the reader is away from the live
  edge — at the bottom nothing is clipped. Pass the color of the surface
  behind the scroller; the fade is off by default.

Use theme roles such as `group_box`, `background`, `border`, and `foreground`
for custom surfaces. Keep content padding in the surrounding conversation
shell when it belongs to the shell's header/composer relationship; use list or
row style when it belongs to every transcript row.

## Virtualization, accessibility, and application boundaries

`MessageScroller` delegates viewport layout, variable-height measurement,
scroll anchoring, and overdraw to GPUI's `ListState`. Only visible rows and the
configured overdraw region need rendering. Keep row closures deterministic and
avoid doing network work or mutating the message collection during rendering.

For keyboard and screen-reader behavior:

- Keep the scroller inside a layout with a real height and `min_h_0()` so the
  scroll region can receive wheel and keyboard navigation.
- The transcript viewport announces itself as a log region (`Role::Log`), so
  assistive technology can treat appended rows as live additions.- Wheel scrolling over the transcript is contained: while the list can move,
  the event never scrolls an ancestor scroller; at the top or bottom edge it
  chains to the ancestor, matching platform scroll containers.
- Give rows meaningful text and stable application IDs; an index by itself is
  not a user-facing label.
- Give the jump control an explicit visible label when it must be exposed as a
  named accessible action. Its tooltip is supplemental.
- Place “Load earlier”, retry, composer, and unread controls outside the list in
  semantic `Button` or `Link` controls.
- Keep empty, loading, error, and permission states readable without relying on
  animation or scrollbar position.

The component intentionally has no React-style Provider, Viewport, Content, or
Item exports. GPUI's list already supplies those layers; an indexed renderer is
the item boundary. It also has no turn-anchor, peek, visible-range, or
stable-ID API. Those concepts vary by product and belong in the application
model around this component.

## API reference

### `MessageScrollerState`

| Method | Default/return | Purpose |
| --- | --- | --- |
| `new(item_count, cx)` | tail following enabled | Create state for the current row count. |
| `item_count()` | current count | Read the virtual-list row count. |
| `is_scrolled_up()` | `false` until away from tail | Report whether a jump/new-content affordance is useful. |
| `is_following_tail()` | `true` initially | Report whether appended rows are followed. |
| `reset(item_count, cx)` | re-engages tail | Replace the row count and reset list state. |
| `splice(range, count, cx)` | `true` if valid | Replace a half-open range while preserving list bookkeeping. |
| `append(count, cx)` | `splice` at tail | Add rows at the end. |
| `prepend(count, cx)` | `splice` at index 0 | Add earlier rows while preserving the current anchor. |
| `remeasure(cx)` | — | Remeasure all rows after global layout changes. |
| `remeasure_items(range, cx)` | `true` if valid | Remeasure selected dynamic rows. |
| `scroll_to_item(index, cx)` | `false` if out of range | Navigate to an arbitrary row index. |
| `scroll_to_end(cx)` | tail following enabled | Move to the latest row and resume following. |

### `MessageScroller`

| Method | Default | Purpose |
| --- | --- | --- |
| `new(id, state, renderer)` | scrollbar and jump button enabled | Create a virtualized scroller. |
| `scrollbar(bool)` | `true` | Show or hide the internal scrollbar. |
| `jump_button(bool)` | `true` | Show or hide the built-in jump control. |
| `with_jump_button_label(label)` | `Jump to latest` | Set the jump tooltip/localized label. |
| `with_content_style(style)` | empty refinement | Style the viewport and scrollbar region. |
| `with_list_style(style)` | list `px_3()` / `py_2()` | Style the virtual list. |
| `with_row_style(style)` | row `pb_8()` | Style every rendered row wrapper. |
| `with_jump_button_style(style)` | themed secondary button | Refine the built-in button. |
| `with_jump_button_renderer(callback)` | default Button | Adjust the configured button while keeping its action. |
| `with_jump_button_transition(duration)` | 200 ms | Set enter/leave duration; reduced motion skips it. |
| `with_bottom_fade(color)` | off | Fade the bottom edge into the surrounding surface color. |
| `Styled` methods | full-size, clipped root | Style the outer scroller element. |

[MessageScroller]: https://docs.rs/gpui-component/latest/gpui_component/message_scroller/struct.MessageScroller.html
[MessageScrollerState]: https://docs.rs/gpui-component/latest/gpui_component/message_scroller/struct.MessageScrollerState.html
