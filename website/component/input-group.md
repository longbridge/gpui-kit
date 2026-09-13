---
title: Input Group
description: Combine inputs and textareas with text, icons, buttons, and toolbars.
---

# Input Group

Use `InputGroup` to place text, icons, buttons, or toolbars around an input or
textarea inside one frame. For a simple prefix or suffix, use [Input](./input.md).

The examples below define views for an initialized GPUI Kit application. See
[Getting Started](../docs/getting-started.md) for application setup.

## Input with a clear button

Create an `InputState` once in your view and pass it to `InputGroupInput`.
Subscribe to `InputEvent::Change` to refresh anything that depends on the text.
Keep the returned `Subscription` in the view so the callback stays active.

This view shows a character count and lets the user clear the input:

```rust
use gpui_kit::{
    AppContext as _, ClickEvent, Context, Entity, IntoElement, ParentElement as _,
    Render, Styled as _, Subscription, Window, rems,
};
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    Disableable as _, Icon,
    input::{InputEvent, InputState},
    input_group::{
        InputGroup, InputGroupAddon, InputGroupAddonAlignment,
        InputGroupButton, InputGroupInput, InputGroupText,
    },
};

struct SearchField {
    query: Entity<InputState>,
    _change: Subscription,
}

impl SearchField {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search…"));
        let change = cx.subscribe(&query, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        Self { query, _change: change }
    }

    fn clear(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.query.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }
}

impl Render for SearchField {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.query.read(cx).value().chars().count();
        InputGroup::new("search")
            .max_w(rems(24.))
            .input(InputGroupInput::new(&self.query).aria_label("Search"))
            .addon(InputGroupAddon::new("search-icon")
                .child(Icon::new(IconName::Search).size_4()))
            .addon(InputGroupAddon::new("search-actions")
                .align(InputGroupAddonAlignment::InlineEnd)
                .child(InputGroupText::new().child(format!("{count} characters")))
                .child(InputGroupButton::new("clear").label("Clear")
                    .disabled(count == 0)
                    .on_click(cx.listener(Self::clear))))
    }
}
```

Read or set the value through the same state:

```rust
let value = self.query.read(cx).value();

self.query.update(cx, |state, cx| {
    state.set_value("gpui", window, cx);
});
cx.notify();
```

`InputEvent::Change` reports user edits. Setting a value with `set_value` does
not emit this event; call `cx.notify()` when other content in your view must
refresh after a programmatic update.

## Parts and alignment

| Part | Use |
| --- | --- |
| `InputGroup` | Combine one input with any number of addons |
| `InputGroupInput` | Add a single-line input using `InputState` |
| `InputGroupTextarea` | Add multiline text using `TextareaState` |
| `InputGroupAddon` | Position text, icons, buttons, or custom content |
| `InputGroupButton` | Add a compact action button |
| `InputGroupText` | Display helper text, a prefix, suffix, or counter |

Pass the input to `.input(...)` and each addon to `.addon(...)`. Use `.child(...)`
or `.children(...)` inside an addon. A later `.input(...)` replaces the earlier
input; repeated `.addon(...)` calls keep all addons.

Set an addon's position with `.align(InputGroupAddonAlignment::...)`:

| Alignment | Position |
| --- | --- |
| `InlineStart` (default) | Before the input |
| `InlineEnd` | After the input |
| `BlockStart` | Above the input row |
| `BlockEnd` | Below the input row |

You can combine all four positions. Addons on the same side and children within
an addon appear in the order you add them. Give each part a stable, distinct ID.
Clicking text, icons, or empty space in an addon focuses the input.

For example, add a protocol prefix and domain suffix to a single-line input:

```rust
InputGroup::new("website")
    .input(InputGroupInput::new(&self.query).aria_label("Website"))
    .addon(InputGroupAddon::new("protocol")
        .child(InputGroupText::new().child("https://")))
    .addon(InputGroupAddon::new("domain")
        .align(InputGroupAddonAlignment::InlineEnd)
        .child(InputGroupText::new().child(".com")))
```

## Buttons, icons, and menus

Use `.label(...)` for a text button or `.icon(...)` for an icon button. Give
icon-only buttons an `.aria_label(...)`; `.tooltip(...)` adds a visible hint.

```rust
use gpui_kit::component::input_group::InputGroupButtonSize;

InputGroupButton::new("clear-icon")
    .with_size(InputGroupButtonSize::IconXSmall)
    .icon(IconName::X)
    .aria_label("Clear search")
    .tooltip("Clear search")
    .on_click(cx.listener(Self::clear))
```

| `InputGroupButtonSize` | Use |
| --- | --- |
| `XSmall` (default) | Compact text button |
| `Small` | Larger text button |
| `IconXSmall` | Compact square icon button |
| `IconSmall` | Larger square icon button |

Buttons default to ghost styling. Import `button::ButtonVariants` to use
`.primary()`, `.secondary()`, or `.danger()`. Use `.outline()` for an outline,
`.disabled(true)` to disable an action, and `.loading(true)` to show progress
and prevent repeated clicks. Clicking a button runs its action without moving
focus back to the input afterwards.

For an action menu, use `.dropdown_menu(...)` with the [menu API](./menu.md).
For contextual help, pass an `InputGroupButton` to [Popover](./popover.md)'s
`.trigger(...)`, then add the Popover to an addon. Use `.with_button(...)` to
configure other [Button](./button.md) options.

## Textarea with a counter and submit action

Use `TextareaState` with `InputGroupTextarea`. `.auto_grow(min, max)` grows the
input between the given row counts; longer content scrolls. Use `.rows(n)` for
a fixed row count or `InputGroupTextarea::h(...)` for a fixed height.

This complete view counts characters, disables submission for empty or oversized
drafts, and displays the submitted text below the composer. Submitting clears
and focuses the textarea.

```rust
use gpui_kit::{
    AppContext as _, ClickEvent, Context, Entity, IntoElement, ParentElement as _,
    Render, SharedString, Styled as _, Subscription, Window, rems,
};
use gpui_kit::component::{
    Disableable as _, button::ButtonVariants as _, v_flex,
    input::{InputEvent, TextareaState},
    input_group::{
        InputGroup, InputGroupAddon, InputGroupAddonAlignment,
        InputGroupButton, InputGroupText, InputGroupTextarea,
    },
};

struct MessageComposer {
    message: Entity<TextareaState>,
    submitted: Option<SharedString>,
    _change: Subscription,
}

impl MessageComposer {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let message = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("Write a message…")
                .auto_grow(2, 6)
        });
        let change = cx.subscribe(&message, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        Self { message, submitted: None, _change: change }
    }

    fn submit(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.message.read(cx).value();
        if value.trim().is_empty() || value.chars().count() > 280 {
            return;
        }
        self.submitted = Some(value);
        self.message.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }
}

impl Render for MessageComposer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let value = self.message.read(cx).value();
        let count = value.chars().count();
        v_flex().max_w(rems(28.)).gap_2()
            .child(InputGroup::new("message")
                .invalid(count > 280)
                .input(InputGroupTextarea::new(&self.message).aria_label("Message"))
                .addon(InputGroupAddon::new("message-footer")
                    .align(InputGroupAddonAlignment::BlockEnd)
                    .child(InputGroupText::new().child(format!("{count}/280")))
                    .child(InputGroupButton::new("submit").ml_auto().primary().label("Submit")
                        .disabled(value.trim().is_empty() || count > 280)
                        .on_click(cx.listener(Self::submit)))))
            .children(self.submitted.as_ref().map(|text| format!("Submitted: {text}")))
    }
}
```

Use `BlockStart` for a heading or toolbar above the textarea. Addons stay in place
while the text scrolls. See [Textarea](./textarea.md) for more text options.

## Disabled, read-only, and validation

| Method | Effect |
| --- | --- |
| `.disabled(true)` | Disables the input and direct `InputGroupButton` children |
| `.readonly(true)` | Prevents editing while allowing focus, selection, copying, and addon actions |
| `.invalid(true)` | Shows an error state while allowing further edits |

An input part with `.disabled(true)` also disables its group. Pass the disabled
flag to custom interactive addon content and wrapped controls separately.

Set `.invalid(...)` from your validation result and show an explanation next to
the group. To reject particular edits, use [`InputState::validate`](./input.md).
Give each input an `.aria_label(...)`, even if you also name the group.

Use `.content_type(...)` on `InputGroupInput` for hints such as a URL or email
address. Password masking is configured with `InputState::masked`. Both input
parts support `.context_menu(...)` for a custom right-click menu.

## Sizes and custom styles

The default group size is Medium. Import `Sizable` to use `.xsmall()`, `.small()`,
`.large()`, or `.with_size(Size::Medium)`. Colors and corners follow your [Theme](./theme.md).
Use `Styled` methods to set the group's width, spacing, and other appearance.

Use these methods to customize a specific part or state:

| Component | Method | What it styles |
| --- | --- | --- |
| `InputGroupInput`, `InputGroupTextarea` | `editor_style` | Text area padding, typography, background, and alignment |
| `InputGroupButton` | `label_style` | The text supplied by `.label(...)` |
| `InputGroupButton` | `icon_style` | The icon, including its loading state |
| `InputGroup` | `focused_style` | The frame while the input is focused and valid |
| `InputGroup` | `invalid_style` | The frame when invalid, including while disabled |
| `InputGroup` | `disabled_style` | The disabled frame |

For example, customize the input and its clear button in `SearchField`:

```rust
use gpui_kit::component::{ActiveTheme as _, Sizable as _, StyledExt as _};

InputGroup::new("styled-search")
    .small()
    .max_w(rems(24.))
    .focused_style(|style| style.border_color(cx.theme().primary))
    .invalid_style(|style| style.bg(cx.theme().danger.opacity(0.05)))
    .disabled_style(|style| style.opacity(0.7))
    .input(InputGroupInput::new(&self.query)
        .aria_label("Search")
        .editor_style(|style| style.px_3().text_base()))
    .addon(InputGroupAddon::new("styled-actions")
        .align(InputGroupAddonAlignment::InlineEnd)
        .child(InputGroupButton::new("styled-clear").label("Clear").icon(IconName::X)
            .label_style(|style| style.font_semibold())
            .icon_style(|style| style.size_4())
            .on_click(cx.listener(Self::clear))))
```

Repeated style calls combine; the last value for a property wins. State styles
override the ordinary frame style. Invalid border and ring styling takes priority
over focus and disabled styling; customizing a disabled style keeps interaction
disabled. A state's `border_color` also changes its ring color. Import
`FocusableExt` and use `.focus_ring(false)` to hide the default ring.

`editor_style` changes editable text, but placeholder, caret, and selection
colors follow the Theme. Percentage padding is relative to the editing area's
width. Style an addon or helper text directly with its `Styled` methods.

## JavaScript

Import the same parts from `gpui-component`. Create text states in `View.init`.
Use `.value(...)` and `.on_change(...)` for a controlled input:

```javascript
import { View } from "gpui-kit";
import {
  InputState, InputGroup, InputGroupInput, InputGroupAddon, InputGroupButton,
} from "gpui-component";

export default class Search extends View {
  init() {
    this.input = InputState("Search…");
    this.query = "";
  }

  render() {
    return new InputGroup("search")
      .input(new InputGroupInput(this.input)
        .aria_label("Search").value(this.query)
        .on_change((value, cx) => { this.query = value; cx.notify(); }))
      .addon(new InputGroupAddon("actions").align("inline-end")
        .child(new InputGroupButton("clear").label("Clear")
          .disabled(this.query.length === 0)
          .on_click((_event, cx) => { this.query = ""; cx.notify(); })));
  }
}
```

Programmatic `.value(...)` updates do not call `on_change`. Setting the same value
keeps the selection and undo history. Omit `.value(...)` to let the input keep its
own value, and use `on_change(value, cx)` when you need to react to edits.

`InputGroupTextarea` accepts `TextareaState` and supports `.rows(n)` and
`.auto_grow(min, max)`. Both input parts provide `.placeholder(...)`.
`InputGroupInput` also provides `.masked(bool)` and `.content_type(...)`, with
values such as `email_address`, `url`, and `new_password`.

Set group size with `.size("small")`; available values are `xsmall`, `small`,
`medium`, and `large`. Button sizes are `xsmall`, `small`, `icon-xsmall`, and
`icon-small`. Button icons take an asset path, such as `.icon("icons/search.svg")`.

The six style methods above are also available in JavaScript:

```javascript
new InputGroupInput(this.input)
  .editor_style(style => style.px(12).text_base());

new InputGroupButton("clear").label("Clear").icon("icons/x.svg")
  .label_style(style => style.font_semibold())
  .icon_style(style => style.size_4());
```

A style callback can call style methods, `when`, and `map`. Return the supplied
style or return nothing; children and event handlers belong on the component.
Run `gpui-component-shell types <application>` to generate editor completion.
