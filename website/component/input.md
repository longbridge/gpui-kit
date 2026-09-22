---
title: Input
description: Text input component with validation, masking, and various features.
---

# Input

For multiple addons, shared frames, and textarea toolbars, see [Input Group](./input-group.md).

A single-line text input with validation, masking, prefix/suffix elements, and
different visual states. Use [Textarea](./textarea.md) for ordinary multi-line
text and [Editor](./editor.md) for source code.

## Import

```rust
use gpui_kit::component::input::{Input, InputState};
```

## Usage

### Basic Input

```rust
let input = cx.new(|cx| InputState::new(window, cx));

Input::new(&input)
```

### With Placeholder

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Enter your name...")
);

Input::new(&input)
```

### With Default Value

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .default_value("John Doe")
);

Input::new(&input)
```

### Cleanable Input

```rust
Input::new(&input)
    .cleanable(true) // Show clear button when input has value
```

### With Prefix and Suffix

```rust
use gpui_kit::component::{Icon, IconName};

// With prefix icon
Input::new(&input)
    .prefix(Icon::new(IconName::Search).small())

// With suffix button
Input::new(&input)
    .suffix(
        Button::new("info")
            .ghost()
            .icon(IconName::Info)
            .xsmall()
    )

// With both
Input::new(&input)
    .prefix(Icon::new(IconName::Search).small())
    .suffix(Button::new("btn").ghost().icon(IconName::Info).xsmall())
```

### Password Input (Masked)

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .masked(true)
        .default_value("password123")
);

Input::new(&input)
    .content_type(InputContentType::Password)
    .mask_toggle() // Shows toggle button to reveal password
```

While the value is masked, the input keeps it out of the clipboard and out of
the selection: Copy and Cut do nothing (and are disabled in the context menu),
a word-wise delete takes everything before the caret, and a double click
selects the whole value instead of one word. Paste and Select All keep working,
and revealing the value with `mask_toggle` restores all of them.

### Input Sizes

```rust
Input::new(&input).large()
Input::new(&input) // medium (default)
Input::new(&input).small()
```

### Disabled Input

```rust
Input::new(&input).disabled(true)
```

### Read-only Input

Unlike `disabled`, a read-only input keeps the normal appearance and still can
be focused, selected and copied, it only rejects the changes made by the user.

```rust
Input::new(&input).readonly(true)
```

### Clean on ESC

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .clean_on_escape() // Clear input when ESC is pressed
);

Input::new(&input)
```

### Input Validation

```rust
// Validate float numbers
let input = cx.new(|cx|
    InputState::new(window, cx)
        .validate(|s, _| s.parse::<f32>().is_ok())
);

// Regex pattern validation
let input = cx.new(|cx|
    InputState::new(window, cx)
        .pattern(regex::Regex::new(r"^[a-zA-Z0-9]*$").unwrap())
);
```

### Input Masking

```rust
// Phone number mask
let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern("(999)-999-9999")
);

// Custom pattern: AAA-###-AAA (A=letter, #=digit, 9=digit optional)
let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern("AAA-###-AAA")
);

// Number with thousands separator
use gpui_kit::component::input::MaskPattern;

let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(3),
        })
);
```

### Handle Input Events

```rust
let input = cx.new(|cx| InputState::new(window, cx));

cx.subscribe_in(&input, window, |view, state, event, window, cx| {
    match event {
        InputEvent::Change => {
            let text = state.read(cx).value();
            println!("Input changed: {}", text);
        }
        InputEvent::PressEnter { secondary } => {
            println!("Enter pressed, secondary: {}", secondary);
        }
        InputEvent::Focus => println!("Input focused"),
        InputEvent::Blur => println!("Input blurred"),
    }
});
```

### Custom Appearance

```rust
// Without default styling
Input::new(&input).appearance(false)

// Use in custom container
div()
    .border_b_2()
    .px_6()
    .py_3()
    .border_color(cx.theme().border)
    .bg(cx.theme().secondary)
    .child(Input::new(&input).appearance(false))
```

### Context Menu

```rust
// The built-in context menu can be disabled.
let input = cx.new(|cx| InputState::new(window, cx).context_menu(false));

// Or you can define a custom context menu.
Input::new(&input).context_menu(|menu, window, cx| {
    // You can define your own actions and even utilize
    // built-in actions (cut, copy, paste, etc.)
    // to avoid having to re-implement that functionality.
    menu.menu("Custom Action", Box::new(CustomAction))
        .separator()
        .menu("Cut", Box::new(input::Cut))
        .menu("Copy", Box::new(input::Copy))
        .menu("Paste", Box::new(input::Paste))
})
```

### Touch Selection

On a touch screen, a long press selects the word under the finger and keeps
following the finger while it stays down. Lifting it opens an edit menu over
the selection with the commands that apply — `Cut`, `Copy`, `Paste`, and
`Select All` — and puts a grab handle at each end of the selection. Dragging a
handle moves that end; the other end stays put, and a multi-line input scrolls
when the finger reaches its edge. A long press on whitespace or in an empty
field places the caret and offers `Paste` and `Select All`.

The handles and the menu belong to the selection the gesture made. They
disappear as soon as anything else moves the selection — a tap, typing, an
arrow key, `Escape` — and the menu steps aside while the content scrolls under
a finger. Tapping the selected text brings the menu back.

Cut, Copy, and Paste go through the input's own actions, so a custom key
binding or an open completion menu sees them the same way. A read-only input
offers only `Copy` and `Select All`; a masked input keeps its value out of the
clipboard.

### Paste Hook

`on_paste` intercepts the clipboard before the default text insertion, so
pasted images and copied files can live in app-owned state instead of being
silently dropped. It is available on `Input`, `Textarea` and `Editor`.

```rust
use gpui_kit::ClipboardEntry;

let view = cx.entity().downgrade();
Textarea::new(&self.composer).on_paste(move |item, _, cx| {
    let images: Vec<_> = item.entries().iter().filter_map(|entry| match entry {
        ClipboardEntry::Image(image) => Some(image.clone()),
        _ => None,
    }).collect();
    if images.is_empty() {
        return false; // fall through to the default text insertion
    }
    view.update(cx, |this, cx| {
        // Store the images beside the input, e.g. as `Attachment`s.
        this.attachments.extend(images);
        cx.notify();
    }).ok();
    true // consumed, the input inserts nothing
})
```

Return `true` when the handler took the paste: the `input::Paste` action
stops there and the input inserts nothing. Return `false` to let the action
reach the engine, which inserts `clipboard.text()` as before. Copied files
arrive as `ClipboardEntry::ExternalPaths` through the same hook.

Known limit: on web `read_from_clipboard()` is `None` (text arrives through
the platform input handler); image paste there needs async clipboard access
and permission, and is out of scope.

## Examples

### Search Input

```rust
let search = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Search...")
);

Input::new(&search)
    .prefix(Icon::new(IconName::Search).small())
```

### Currency Input

```rust
let amount = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(2),
        })
);

div()
    .child(Input::new(&amount))
    .child(format!("Value: {}", amount.read(cx).value()))
```

### Form with Multiple Inputs

```rust
struct FormView {
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
}

v_flex()
    .gap_3()
    .child(Input::new(&self.name_input))
    .child(Input::new(&self.email_input))
```

## Atomic inline tokens

Use inline tokens for mentions, file references or commands that should be selected
and deleted as a whole. For example, an input can display “Alice” as a token while
`value()` and Copy return its text, `@alice`.

### Insert a reference

Create the input state once, then insert a token when the user picks a reference:

```rust
use gpui_kit::component::input::{InlineToken, Input, InputState};

let input = cx.new(|cx| InputState::new(window, cx));

input.update(cx, |state, cx| {
    state.replace_with_token(
        InlineToken::new("person-1", "@alice").with_label("Alice"),
        window,
        cx,
    ).expect("valid reference");
});

Input::new(&input)
```

`replace_with_token` replaces the selection, or inserts at the caret. It does not
add a space. To replace a completion query such as `@ali`, use
`replace_range_with_token(range, token, window, cx)`. Rust ranges are half-open
UTF-8 byte ranges; use byte offsets such as those returned by `str::find`.

The ID names the referenced resource, so two mentions of the same person carry
the same ID. Use `text` for the value to copy or submit and `with_label` for its
displayed name. Omit `with_label` to display the text itself.

Users can move the caret to either side of a token, click it to select it, or
delete it with Backspace/Delete. A selection that crosses part of a token
includes the whole token. Undo/Redo restores both its text and reference.
Pasting inserts plain text.

### Customize appearance and opening a reference

Tokens render as an `InputToken` by default. The `token` slot supplies the
element for each token; return one with an icon from it, and use
`on_token_click` to open the reference:

```rust
use gpui_kit::component::{
    IconName,
    input::{InputToken, InlineTokenClickEvent},
};

Input::new(&input)
    .token(|token, _, _| {
        InputToken::new(token).icon(IconName::File)
    })
    .on_token_click(|event: &InlineTokenClickEvent, _, _| {
        // Look up event.token().id() and open its resource.
    });
```

You can also return your own single-row element. Keep it within the input's line
height; content wider than the available row is clipped. Read selection and
readonly/disabled state from the renderer's context. Do not edit the input from
the renderer; event callbacks may update it. Keep hover and selection styles the
same size. Tokens are measured whenever they render, so an element that grows once
its data arrives reflows on the next frame.

A click selects the token and then opens it; dragging or Shift-selecting a token
does not open it. Readonly inputs allow
opening references; disabled inputs do not. To offer a keyboard shortcut for
opening an exactly selected token, bind `ActivateToken` to a key of your choice;
assistive technology reaches the same listener through the token's click action.
Add a menu item for it through `context_menu` when your application has a name
for the reference, such as "Open file".

If your token includes a button, consume its mouse-down and click events so that
it does not also open the reference. Apply `token.is_disabled()` to every child
action, including accessibility actions, and `token.is_readonly()` to actions
that change the content.

### Save, restore and submit

Use `content()` to keep the text and references together when saving a draft:

```rust
let draft = input.read(cx).content();

// Restore the saved draft later: `set_value` takes plain text or content.
input.update(cx, |state, cx| {
    state.set_value(draft, window, cx);
});
```

To restore data from your own storage, build an `InputContent` from the text and
attach each token to its byte range. `with_token` validates the range against the
text as you go, so a content value is always consistent by the time it is set:

```rust
use gpui_kit::component::input::InputContent;

let draft = InputContent::new("Ask @alice")
    .with_token(4..10, InlineToken::new("person-1", "@alice").with_label("Alice"))?;
```

At submission time, read a fresh `content()`: `text()` is the message, and
`tokens()` contains the references still present in it. Use each token's ID to
look up its resource and handle missing resources before sending.

`set_value` clears undo history and does not emit `InputEvent::Change`. Passing
plain text removes every token, even if the text is unchanged; passing content
restores its tokens, except in modes that cannot show them. Use `replace_all` for
an undoable plain-text replacement. Token edits,
including adding a reference to existing text, emit `InputEvent::Change`.
Programmatic setters can update readonly or disabled inputs, so check these
states in application commands that should be unavailable to users.

### Validation and supported inputs

Tokens work with Input and Textarea. They are not available in Editor,
NumberInput, formatted masks or password fields. A token's ID must not be blank;
its text and label must be nonempty, single-line strings without control
characters. Ranges cannot overlap or split a Unicode grapheme (such as an emoji
or a character with a combining accent), and each token's text must match its
range when restoring a draft.

Token operations return `Result<_, InlineTokenError>`. A rejected operation leaves
the input unchanged. If insertion returns `CompositionActive`, wait until the
user finishes composing with their input method before inserting the token.

### JavaScript

Create and keep an `InputState` in `init()`, then render an Input with that state.
JavaScript ranges use **UTF-16 string offsets**, matching `slice()` and `indexOf()`:

```javascript
import { Input, InputState } from "gpui-component";

// In init():
this.input = InputState();
this.input.set_value({
  text: "🙂 @alice",
  tokens: [{
    range: { start: 3, end: 9 },
    token: { id: "person-1", text: "@alice", label: "Alice" },
  }],
});

// In render():
new Input(this.input)
  .on_token_click((event, cx) => {
    // Look up event.token.id and open its resource.
  });
```

Use `replace_with_token` or `replace_range_with_token` to insert references,
`content()` and `set_value(content)` to save and restore drafts, and `tokens()` to
read the current references. To remove a reference, pass its current range to
`set_selected_range`, then call `replace("")`. Returned snapshots are independent
objects; changing one does not update the input. Make edits from initialization,
event or task callbacks, not from renderers.

Token validation errors expose an `error.code`, such as `InvalidBoundary` or
`CompositionActive`; invalid argument shapes also throw. Textarea offers the same
methods on `TextareaState()`. If you use `gpui-base`, construct these states with
`InputState.new()` or `TextareaState.new()` instead.
