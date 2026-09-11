---
title: Input Group
description: Inputs and textareas with shared frames, addons, and native actions.
---

# Input Group

`InputGroup` combines one input or textarea with text, icons, buttons, and
toolbars inside a single frame. It reuses the existing editing state and native
input engine. The group owns the frame and composition; the application owns
the text state, validation result, and actions.

Colors, corner radii, and focus policy come from the active GPUI Component theme.

## Import

```rust
use gpui_kit::{AppContext as _, ClipboardItem, ParentElement as _, Styled as _};
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, Sizable as _, Size,
    button::ButtonVariants as _,
    input::{InputContentType, InputEvent, InputState, TextareaState},
    input_group::{
        InputGroup, InputGroupAddon, InputGroupAddonAlignment,
        InputGroupButton, InputGroupButtonSize, InputGroupInput,
        InputGroupText, InputGroupTextarea,
    },
    menu::{DropdownMenu as _, PopupMenuItem},
    popover::Popover,
};
```

## Basic usage

Create the state once in the owning view's constructor and retain its Entity:

```rust
let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search…"));
```

Build the group in `render` using that retained state:

```rust
InputGroup::new("search")
    .input(InputGroupInput::new(&query).aria_label("Search components"))
    .addon(
        InputGroupAddon::new("search-icon")
            .child(Icon::new(IconName::Search).size_4()),
    )
    .addon(
        InputGroupAddon::new("search-count")
            .align(InputGroupAddonAlignment::InlineEnd)
            .child(InputGroupText::new().child("12 results")),
    )
```

Subscribe to `InputEvent::Change` when an addon or another part of the view
depends on the text. Read the value from the same state and notify the owning
view. There is no `InputGroupState` and no second copy of the text or selection.

## Anatomy and alignment

| Part | Constructor / composition | Purpose |
| --- | --- | --- |
| `InputGroup` | `new(id)`, `.input(...)`, `.addon(...)` | Shared frame and layout |
| `InputGroupInput` | `new(&Entity<InputState>)` | Single-line text control |
| `InputGroupTextarea` | `new(&Entity<TextareaState>)` | Multiline text control |
| `InputGroupAddon` | `new(id)`, `.align(...)`, `.child(...)`, `.children(...)` | Text, icons, and actions on one side |
| `InputGroupButton` | `new(id)`, `.label(...)`, `.icon(...)`, `.on_click(...)` | Compact native action |
| `InputGroupText` | `new()`, `.child(...)` | Muted helper content |

All parts implement `Styled`. Addon, Button, and Text accept ordinary children.
The root and text controls use their explicit typed slots.

`input` accepts either `InputGroupInput` or `InputGroupTextarea`, via the opaque
`InputGroupControl` conversion type. The last call replaces the control.
`addon` appends a part, preserving insertion order among addons on the same side.
Within an addon, `.child(...)` and `.children(...)` preserve the complete insertion
order of text, icons, buttons, and custom content. Direct `InputGroupButton` children
inherit the group disabled state.
Use stable IDs for dynamically added, removed, or reordered parts.

| `InputGroupAddonAlignment` | Position |
| --- | --- |
| `InlineStart` (default) | Before the text control |
| `InlineEnd` | After the text control |
| `BlockStart` | Above the row containing the control |
| `BlockEnd` | Below the row containing the control |

Inline and block addons can coexist. Builder call order does not change these
regions. Keyboard traversal follows native element order, and buttons retain
their normal desktop keyboard behavior.

## Text, icons, and loading

```rust
InputGroup::new("website")
    .input(InputGroupInput::new(&query)
        .aria_label("Website").content_type(InputContentType::Url))
    .addon(InputGroupAddon::new("scheme")
        .child(InputGroupText::new().child("https://")))
    .addon(InputGroupAddon::new("domain")
        .align(InputGroupAddonAlignment::InlineEnd)
        .child(InputGroupText::new().child(".com")))
```

An addon can contain `Icon`, `Kbd`, `Spinner`, or application-owned content.
Size icons explicitly with the normal scale helpers. Clicking ordinary addon
content or the frame's inset focuses the text control. The Story includes
search counts, currency text, keyboard hints, and progress indicators.

## Buttons

Use `.child(...)` to make an action inherit the group's disabled state:

```rust
let copy_state = query.clone();
InputGroup::new("copyable-url")
    .readonly(true)
    .input(InputGroupInput::new(&query).aria_label("URL"))
    .addon(InputGroupAddon::new("url-actions")
        .align(InputGroupAddonAlignment::InlineEnd)
        .child(InputGroupButton::new("copy-url")
            .with_size(InputGroupButtonSize::IconXSmall)
            .icon(IconName::Copy)
            .aria_label("Copy URL").tooltip("Copy URL")
            .on_click(move |_, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(
                    copy_state.read(cx).value().to_string(),
                ));
            })))
```

The default button is ghost and compact. `ButtonVariants` supplies semantic
variants such as `.primary()`, `.secondary()`, and `.danger()`.
Use `.aria_label(...)`, `.tooltip(...)`, `.loading(...)`, and `.outline()` directly.
`with_button` configures additional native Button capabilities. The button implements
`Selectable`, `InteractiveElement`, and `DropdownMenu`, so it can also be used as a
Popover trigger or receive `.dropdown_menu(...)`.

| `InputGroupButtonSize` | Use |
| --- | --- |
| `XSmall` (default) | Compact text action |
| `Small` | Larger text action |
| `IconXSmall` | Compact square icon action |
| `IconSmall` | Larger square icon action |

Native buttons keep their mouse-down focus policy. Clicking an action does not
redirect focus to the text control after the action runs. Custom interactive
addon content should follow the same native focus conventions.

## Textarea and toolbars

Create a `TextareaState` for multiline content. Rows, wrapping, scrolling, and
auto-grow remain part of that state:

```rust
let message = cx.new(|cx| {
    TextareaState::new(window, cx)
        .placeholder("Write a message…").auto_grow(2, 6)
});
```

```rust
InputGroup::new("message")
    .input(InputGroupTextarea::new(&message).aria_label("Message"))
    .addon(InputGroupAddon::new("message-header")
        .align(InputGroupAddonAlignment::BlockStart)
        .border_b_1().border_color(cx.theme().border)
        .child(InputGroupText::new().child("New message")))
    .addon(InputGroupAddon::new("message-footer")
        .align(InputGroupAddonAlignment::BlockEnd)
        .child(InputGroupText::new().child("0/280"))
        .child(InputGroupButton::new("send").ml_auto().primary().label("Send")))
```

Attach an owner callback to Send and derive its disabled state from the message.
The Story demonstrates a working character counter, submission, clearing, and a
sample attachment. Use `.h(...)` on `InputGroupTextarea` for a fixed viewport.
The scrollbar belongs to the text viewport; toolbars do not scroll with the text.

## Disabled, read-only, and validation

`.disabled(true)` disables the text control and typed addon buttons and guards
the group against pointer and keyboard activation. `.readonly(true)` prevents
editing while preserving focus, selection, copying, and addon actions. A disabled
input part also disables its group. An enabled child cannot override group policy.

`.invalid(true)` displays the application's validation result on the shared
frame without rejecting edits. `InputState::validate` instead participates in
deciding whether an edit can be accepted. Place an explanatory error next to the
group and give the text control an accessible label.

The input parts preserve the existing native context menu and expose
`.context_menu(...)` to replace it. Single-line inputs retain content-type hints
and password masking through the existing state and shared integration.

## Theme and sizing

Default styling comes from the active GPUI Component theme:

| Surface | Theme source |
| --- | --- |
| Frame border | `theme.input` |
| Background | Transparent in light mode; `theme.input` at 30% opacity in dark mode |
| Foreground / helper text | `theme.foreground` / `theme.muted_foreground` |
| Frame / button corners | `theme.radius`; compact XS buttons use `theme.radius_tokens().sm` |
| Focus / error ink | `theme.ring` / `theme.danger` |
| Outward ring policy | `theme.focus_ring` and `.focus_ring(...)` |

The frame and compact buttons have no resting elevation shadow. Focus and error
rings have zero blur and themed ink. Hairlines and ring widths are control
boundaries; text, spacing, and control sizing use the normal relative scale.
Default border and background colors transition over 150 ms and respect reduced motion.
The error border takes precedence over focus, with a 20% error ring in light mode
and 40% in dark mode. Disabled groups retain their error indication; disabled
controls do not acquire a focus ring.
The group supports `.xsmall()`, `.small()`, and `.large()` through `Sizable`.
Medium is the default; select it explicitly with `.with_size(Size::Medium)`. Use `Styled` to refine the group and its parts.

Existing `Input::prefix` and `Input::suffix` remain unchanged. They suit simple
standalone adornments; Input Group adds shared frames and toolbar composition.

## Internal and state styles

Use each part's `Styled` methods for its outer frame. Stable internal parts have
their own style builders:

| Component | Style builder | Target |
| --- | --- | --- |
| `InputGroupInput`, `InputGroupTextarea` | `editor_style` | Editing viewport: padding, typography, background, and text alignment |
| `InputGroupButton` | `label_style` | The text supplied by `label`, independently of custom children |
| `InputGroupButton` | `icon_style` | The icon or loading icon, after its default size and color |
| `InputGroup` | `focused_style` | The frame while the editor is focused and valid |
| `InputGroup` | `invalid_style` | The invalid frame, including when disabled |
| `InputGroup` | `disabled_style` | The disabled frame; interaction remains disabled |

```rust
InputGroup::new("draft")
    .focused_style(|style| style.border_color(cx.theme().primary))
    .invalid_style(|style| style.bg(cx.theme().danger.opacity(0.05)))
    .disabled_style(|style| style.opacity(0.7))
    .input(InputGroupTextarea::new(&message)
        .editor_style(|style| style.p_3().text_base()))
    .addon(InputGroupAddon::new("draft-actions")
        .align(InputGroupAddonAlignment::BlockEnd)
        .child(InputGroupButton::new("send").primary().label("Send")
            .icon(IconName::ArrowUp)
            .label_style(|style| style.font_semibold())
            .icon_style(|style| style.size_4())))
```

Each closure runs immediately and receives the accumulated `StyleRefinement`.
Repeated calls preserve earlier overrides unless they set the same property.
Only overrides are retained; theme and size defaults are resolved during render.
Addon and Text already expose their content through `Styled` and ordinary children.

Textarea padding is passed to the native editing engine, so caret hit testing,
selection, IME bounds, and scrolling use the same insets. Relative padding tracks
the measured viewport width after layout and resize. Text color and typography
style the editable text; placeholder, caret, and selection colors retain their
semantic theme tokens.

State defaults override the ordinary frame style; each state builder then
overrides that state's defaults. Invalid border and ring styling takes precedence
over focused or disabled styling. A state-specific `border_color` also tints its
default outward ring. The ring uses the final border widths and corner radii;
`focus_ring(false)` suppresses it when supplying your own shadow treatment.

JavaScript exposes the same six method names. Its function receives a
`StyleDeclaration` with style methods, `when`, and `map`. The function runs during
description building; native repainting reuses the recorded refinement.
Children, event handlers, and returning a different element are rejected.

```javascript
new InputGroupTextarea(this.message)
  .editor_style(style => style.p_3().text_base());

new InputGroupButton("send").label("Send").icon("icons/arrow-up.svg")
  .label_style(style => style.font_semibold())
  .icon_style(style => style.size_4());
```

## More compositions

The native and JavaScript galleries include these additional recipes:

| Recipe | What to try |
| --- | --- |
| Alignment | Leading/trailing icons and a header/footer around a single-line input |
| Icons | Email, a username with a checkmark, and multiple trailing icons |
| Text addons | Currency, protocol/domain suffix, and a work-email suffix |
| Tooltips | Password requirements and notification-email help |
| Dropdown menus | Change/reset the filename, select a search scope, and select a country code |
| Popover | Open address details and dismiss with Escape |
| Labels and descriptions | An `@` label and a label inside a block-start addon |
| Text and icon actions | Clear, reset, and copy a project name |
| Spinner placement | Leading, trailing, and text-plus-spinner loading states |
| Textarea variants | No addons, header, remaining-character footer, invalid, and disabled |
| Comment composer | Cancel the draft or post it and retain the submitted text |
| Auto-growing textarea | Grow from one to eight rows, submit, and clear |
| Form composition | Combine standalone Input, Input Group, Field, and GroupBox |

Each text control has its own retained state. Counters use Unicode character
counts. Submission examples keep their result in the Story; they do not send
messages or save contacts to an external service.

### Dropdown menu in an addon

`InputGroupButton` supports `DropdownMenu` directly and can be supplied as addon
content. This preserves the menu's pressed state, keyboard navigation, and
focus restoration. The following menu resets the retained filename:

```rust
let filename = self.filename.clone();
InputGroup::new("filename")
    .input(InputGroupInput::new(&self.filename).aria_label("File name"))
    .addon(InputGroupAddon::new("filename-menu").align(InputGroupAddonAlignment::InlineEnd)
        .child(InputGroupButton::new("filename-more").label("More")
            .dropdown_menu(move |menu, _, _| {
                let filename = filename.clone();
                menu.item(PopupMenuItem::new("Reset filename")
                    .on_click(move |_, window, cx| {
                        filename.update(cx, |state, cx| {
                            state.set_value("notes.txt", window, cx);
                        });
                    }))
            })))
```

### Popover in an addon

Use the existing Popover with an `InputGroupButton` trigger for contextual content.
Popover owns the open state; the input keeps its existing editing state:

```rust
InputGroup::new("website-details")
    .input(InputGroupInput::new(&self.website).aria_label("Website"))
    .addon(InputGroupAddon::new("website-info")
        .child(Popover::new("address-details")
            .trigger(InputGroupButton::new("address-details-trigger")
                .with_size(InputGroupButtonSize::IconXSmall)
                .icon(IconName::Info).aria_label("Address details"))
            .child("The protocol prefix is separate from the editable hostname."))
        .child(InputGroupText::new().child("https://")))
```

The native Story uses compact icon triggers. The JavaScript Popover host
provides a labeled trigger, so its example places the details control in a
block-start addon. All addon content uses `.child(...)`. Direct button parts inherit
disabled state; menu and popover wrappers retain their own component contracts.

### Native auto-grow

Use `TextareaState::auto_grow` for native auto-sizing:

```rust
let draft = cx.new(|cx| {
    TextareaState::new(window, cx)
        .placeholder("An automatically growing textarea…")
        .auto_grow(1, 8)
});
```

Pass it to `InputGroupTextarea`, refine typography with `Styled`, and place
Submit in a block-end addon. The group's control slot still accepts the native
typed input or textarea; this recipe does not introduce a custom-control slot.

## JavaScript shell

All six parts are registered in `gpui-component-shell`, reusing the existing
`InputState` and `TextareaState` constructors. The shell's common `.input(...)`
slot accepts either `InputGroupInput` or `InputGroupTextarea`; repeated calls
replace the control. Create states in `View.init`:

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

`value(...)` updates native text only when it differs. Programmatic changes are
silent; equal values preserve selection and undo history. Omit `value` for
uncontrolled retained text. `on_change(value, cx)` reports edits. Re-rendering
updates callbacks without adding duplicate subscriptions.

`InputGroupTextarea` adds `.rows(n)` and `.auto_grow(min, max)`; `.placeholder`
is available on both controls. `InputGroupInput` adds `.masked(bool)` and
`.content_type(...)`, whose literals follow `InputContentType` in snake_case,
such as `email_address`, `url`, and `new_password`.
Shell button sizes are `xsmall`, `small`, `icon-xsmall`, and `icon-small`.
Use `.icon("icons/search.svg")` for button icons and `.aria_label(...)` for their names.
`.child(...)` remains available for custom button content.

The native and JavaScript Stories both include working input groups. Regenerate
declarations with `gpui-component-shell types <application>` for editor completion.
