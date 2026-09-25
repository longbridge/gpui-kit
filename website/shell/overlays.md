---
title: Overlays
description: Dialogs, the sheet and toasts, their stacking and dismissal order, and why they may only be opened from an event.
order: 7
maturity: [preview]
---

# Overlays

Dialogs, the sheet and toasts are **host** capabilities, reached through the global `window`. They are not something a script draws.

A dialog is not a floating `div`. It is a place in the window's stacking order, a focus trap, an Escape target, and a promise about what pressing the backdrop means — all of which the window's root has to decide, because only something that sees every overlay at once can order them. A script drawing its own dialog would own none of that, and two scripts drawing two dialogs would own even less.

So the script says **what** to put in front of the user, and the root says where it goes and how it leaves. What crosses the boundary is small: a function returning an element, a side to anchor to, a sentence to show.

They are on `window` rather than on `cx` because a dialog belongs to the window, not to the View that opened it: `cx.notify()` re-renders one View, `window.open_dialog()` changes what the user is looking at. `gpui-component` draws the same line, so the two halves of an application read as one vocabulary — and `window` is somewhere to grow. Overlays are what it carries today; `Window` in Rust also answers focus, size and appearance, and those land in a namespace that already exists.

## The surface

`window` is a **global**. There is nothing to import — and unlike `cx`, which every host call hands you as an argument, nothing hands you `window` either. It is simply in scope.

A callback parameter named `window` shadows it, which is ordinary scoping and not an error — and if a future callback ever hands one in, it would be this same object, because `window` is ambient: it reads the call that is running. That is also why it is not a parameter today. In Rust it has to be one, since Rust has no ambient state to read; here the read is available, which is the same reason `fs` and `store` are not parameters either.

::: warning Do not copy Rust's `|event, window, cx|`
A script handler takes `(event, cx)`. A three-parameter version binds `window` to the context and leaves `cx` undefined, and the failure reads as `close_dialog is not a function`. With `// @ts-check` the generated declarations catch it at the line where you wrote it.
:::

```js
const depth = window.open_dialog(() => confirmClear(count), {
  escape_dismissable: false,
  backdrop_dismissable: false,
});
window.close_dialog();        // -> did anything close?
window.close_all_dialogs();   // -> how many closed
window.has_active_dialog();

window.open_sheet(() => filters());           // right, the default side
window.open_sheet_at("left", () => nav());
window.close_sheet();         // -> did anything close?
window.has_active_sheet();

window.push_toast({ title: "Saved", description: "3 files", level: "success",
                    timeout: 4000, id: "save" });
window.remove_toast("save");
window.clear_toasts();
```

## Dialogs

`window.open_dialog(content, options?)` takes a **function returning an element**, not an element:

```text
expected a function returning an element; open_dialog and open_sheet take
a function, not an element and not a View class
```

The reason is lifetime, not taste. An element belongs to the arena of the render pass that built it, and a dialog outlives the call that opened it — so an element built at open time would belong to the wrong pass. The function runs when the dialog draws, and again whenever it redraws, which is the same contract `render` has.

**Whatever it closes over is the dialog's state.** There is no `props`: a dialog receives what it shows the way every other value in the script arrives, by being in scope.

```js
// confirm.js
import { v_flex, h_flex } from "gpui-base";

export default (count, onConfirm) => () =>
  v_flex()
    .w(360)
    .p(24)
    .gap(12)
    .child(`Delete ${count} completed items?`)
    .child("This cannot be undone.")
    .child(
      h_flex()
        .justify_end()
        .gap(8)
        .child(cancelButton(() => window.close_dialog()))
        .child(deleteButton((_event, cx) => { onConfirm(cx); window.close_dialog(); })),
    );
```

```js
// main.js
window.open_dialog(confirmClear(this.completed, (cx) => this.deleteCompleted(cx)));
```

Note what the root supplies and what it does not. It supplies the backdrop, the position, the layering, the focus trap and the surface it sits on; the width, the padding, the border, the type and the buttons are the script's, like everything else in this runtime.

| Option | Default | Effect |
| --- | --- | --- |
| `escape_dismissable` | `true` | Whether Escape closes it |
| `backdrop_dismissable` | `true` | Whether pressing the backdrop closes it |

An unknown option is refused rather than ignored, which is the point:

```text
unknown option `escapeDismissable` for window.open_dialog(content, options);
expected escape_dismissable or backdrop_dismissable
```

A silently ignored `escapeDismissable` would look like it worked, and the dialog would be dismissable anyway.

`open_dialog` returns the **new depth of the stack**, not a handle. The root addresses dialogs by position and never by identity, so a handle would have to promise "close *this* dialog", which is not an operation that exists. The depth is what a script can use — to assert one opened, or to unwind to a known level. `close_dialog` returns whether it found one to close; `close_all_dialogs` returns how many it closed.

::: warning Do not carry `cx` into the dialog
The `cx` in the handler that opened the dialog belongs to that handler. By the time the dialog's own button is pressed, it is stale, and using it reports a stale-context error. Close over **data**, and take `cx` from the dialog's own callback arguments — which is why the example above passes `onConfirm` a `cx` rather than capturing one.

The overlay calls themselves have no such hazard: they are ambient, like `fs` and `store`, so there is no handle to hold past its call.
:::

## The sheet

```js
window.open_sheet(() => filtersPanel(filters));
window.open_sheet_at("left", () => navigation());
```

At most one sheet is open at a time. `window.open_sheet` anchors it to the right; `window.open_sheet_at` takes `"left"`, `"right"`, `"top"` or `"bottom"`. It has no options at all, because there is only ever one and it is dismissed by Escape or by its overlay whenever no dialog is above it.

```text
unknown sheet placement `middle`; expected left, right, top or bottom
```

## Toasts

A toast is the one overlay that is **data rather than a View** — no function, no instance, nothing for the script to render — which is why its whole content crosses the boundary as an options object.

| Field | Default | Meaning |
| --- | --- | --- |
| `title` | required | The sentence the user reads |
| `description` | — | A second line |
| `level` | `info` | `info`, `success`, `warning` or `error` |
| `timeout` | 5 s | Milliseconds, or `null` to stay until dismissed |
| `id` | generated | Identity, for replacing and dismissing |

An omitted `timeout` keeps the default and an explicit `null` makes the toast sticky, so the two cannot be collapsed into one option.

The `id` is what turns a repeated failure into one standing message instead of a pile. The `--watch` loop uses exactly this: a failed reload posts a sticky error toast with a fixed id, so saving a broken file five times replaces the message rather than stacking five of them, and the next successful reload retracts it with `remove_toast`.

```text
unknown toast level `fatal`; expected info, success, warning or error
```

Three toasts are mounted at once. Older ones stay in the manager and reappear as newer ones leave, so a burst is throttled rather than lost.

## The window itself

The same `window` global answers questions about the window, not only about what is floating over it.

```js
render(cx) {
  const { width, height } = window.viewport_size();
  return v_flex()
    .when(width < 600, (el) => el.flex_col())
    .text_size(window.rem_size() * 0.875);
}
```

**Measurements are legal from `render`**, and that is the point of them: a View that lays itself out from the window's size has to ask during the pass that draws it.

| Member | What it answers |
| --- | --- |
| `rem_size()` / `line_height()` | The window's type metrics, in pixels |
| `viewport_size()` | The drawable area |
| `bounds()` | Where the window sits on screen and how big it is — larger than the viewport by its title bar |
| `mouse_position()` | Where the pointer is, in window coordinates |
| `appearance()` | `"light"` or `"dark"` |
| `is_window_active()` / `is_fullscreen()` / `is_maximized()` | The platform window's state |

**Calls that change the window are refused from `render`**, for the reason `cx.notify()` is: a frame that changes the window it is drawing into is a frame arguing with itself.

| Member | What it does |
| --- | --- |
| `set_rem_size(size)` | Rescales everything expressed in rems |
| `refresh()` | Redraws every View in the window |
| `focus_next()` / `focus_prev()` | Moves the keyboard one tab stop |
| `dispatch_action(action)` | Dispatches an action down this window's focus path |
| `activate_window()` / `minimize_window()` / `zoom_window()` / `toggle_fullscreen()` | Platform window controls |

`zoom_window()` is the platform's own zoom, not a scale factor — `set_rem_size` is the one that rescales.

## Stacking and dismissal

Painted back to front:

1. **Content** — the script's root View.
2. **Sheet** — at most one, anchored to an edge. A sheet is a *place* in the window, so it sits below the dialog stack: a dialog raised from inside a sheet must be readable, and a sheet raised under a dialog must not cover it.
3. **Dialog stack** — in open order, oldest at the bottom.
4. **Toasts** — above everything. A toast reports the outcome of the action the user just took, and an open dialog is exactly the situation where that outcome matters most, so it is the one layer that is never occluded.

Only the topmost dialog draws a backdrop: a stack of three dims the window once, not three times, and that single backdrop is what separates the live dialog from the inert ones behind it.

Dismissal is always **one layer, never a cascade**:

- **Escape** closes the topmost dialog only. Lower dialogs render with keyboard handling disabled, so repeated Escapes unwind the stack one dialog per press and never reach the sheet while a dialog is open.
- `escape_dismissable: false` withdraws the **key binding**, not the underlying cancel action. A close control the script puts inside the dialog still works — which is what makes an undismissable dialog one the user must answer rather than one they cannot leave.
- **Backdrop press** closes the topmost dialog, and only if it was opened with `backdrop_dismissable`.
- **Enter does nothing** at this layer. Base's dialog host treats Enter as "confirm and close"; that belongs to the dialog's own primary button, which the script owns, so the root vetoes the built-in confirmation rather than guessing which content wanted it.
- A **sheet** is dismissed by Escape or by its overlay only when no dialog is open, because a dialog above it holds focus.
- `close_all_dialogs` is the one operation that unwinds the whole stack, and it leaves the sheet alone.

**Focus** is restored through the stack's own history. Opening an overlay records what was focused and focuses the overlay; closing it restores that handle. Closing the second dialog returns focus to the first, and closing the first returns it to whatever the window was on before either opened. Tab and Shift-Tab honour the focus trap, so tabbing inside an overlay cycles within it rather than walking into the content behind it.

## The `ScopePhase` rule

**An overlay may only be opened or closed from an event handler or a task.**

```text
window.open_dialog(content, options) is not allowed during the `render` phase;
overlays may only be opened or closed while handling an event or a task
```

Opening or closing an overlay mutates the window, and the render pass is reading it. GPUI's borrow model has no way to express "the script may notify from here but not from there", so the runtime carries the [`ScopePhase`](./state.md#scope-phases) explicitly and every overlay entry point refuses `render`, `layout`, and being called from outside any host call at all — in the last case there is no window to reach either.

The refusal names the phase it came from, because that is the only clue the author has.

`window.has_active_dialog()` and `window.has_active_sheet()` are the exception, and read the same rule: they ask a question rather than change anything, and a View that draws itself differently while a dialog is up has to ask during the pass that draws it.

## Overlays need a `ShellRoot`

Every one of these calls reaches the window's root View. A window whose first View is not a `ShellRoot` refuses them, and says which mistake it was — a host wiring problem, not a script one:

```text
window.open_dialog(content, options) needs a ShellRoot as the window's first View;
this window was opened with another View
```

See [Getting started](./getting-started.md#add-the-runtime-to-a-rust-application).

## Not there yet

- **A result from a dialog.** `open_dialog` returns a depth, not a promise that settles when the dialog closes. Close over a callback, as the example above does, or have the dialog write back to state the opener reads.
- **Tooltips and context menus.** Popover and HoverCard are available as anchored surfaces; dedicated tooltip and context-menu APIs are not yet exposed.
- **Positioning options.** A dialog is centred and a sheet is edge-anchored; neither can be placed.
