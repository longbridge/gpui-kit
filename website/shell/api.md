---
title: API Reference
description: Every name a script can import or reach — the four built-in modules, the cx and window globals, and the element methods that are not styles.
order: 10
---

# API Reference

An inventory of the script surface: what exists, and which module it comes from. The other pages explain why each thing works the way it does — this one is for looking a name up.

The authority is not this page. The runtime generates `gpui-kit.d.ts` for its own version and refreshes it beside your source when the application loads. That refresh is best-effort; `gpui-shell types <directory>` performs the same write and reports a failure. The generated header names the `gpui-shell` version and includes that application's HostModule registrations. Keep the file ignored, and put `// @ts-check` at the top of a script to have an editor check against it. The manifest's Git dependencies are not listed here either: they are linked into `node_modules` by the same refresh, and their names, signatures and documentation come from the packages themselves. See [Dependencies](./dependencies.md).

## The modules

Each built-in module names the public Rust layer it exposes, so an import says which layer a script depends on. The `gpui-kit` module also carries the shell bridge needed to use GPUI from JavaScript: Views, retained entities, scheduling and shared types. `gpui` is a compatibility alias for this module and exposes the same bindings, including `div`.

```js
import { View, div } from "gpui-kit";
import { div as gpuiDiv } from "gpui";
import { Button, v_flex } from "gpui-base";
import { fps_monitor } from "gpui-fps";
```

| Module       | Provides                                                                               |
| ------------ | -------------------------------------------------------------------------------------- |
| `gpui-kit`   | GPUI's own elements, plus what this runtime adds: Views, the style surface, scheduling |
| `gpui-base`  | Layout helpers, components and the theme                                               |
| `gpui-shell` | Type-only concepts owned by the shell bridge; it has no run-time exports               |
| `gpui-fps`   | The performance overlay                                                                |

Two names are never imported, for two different reasons. `window` is a real global: nothing hands it to you, it is simply in scope. `cx` is the opposite — it is never a global, and only ever arrives as an argument: `render(cx)`, `init(props, cx)`, the second argument of every handler, the parameter of a `cx.spawn` body. The standard-runtime modules — `fs/promises`, `path`, `crypto`, `process`, `net`, `websocket` and the rest — are gated by the host's grant and are documented in [Capabilities](./capabilities.md).

API shape follows the Rust original: a method on `App` is a method on `cx`, a method on `Window` is on the `window` global, an associated constructor is `Type.new(...)`, and a free function stays lowercase. Names with no direct GPUI or Base original belong to the module for the layer that implements them. Type-only names appear in these tables too, but are never run-time values.

## The `gpui-kit` module

### Elements

| Name              | What it is                                                                                           |
| ----------------- | ---------------------------------------------------------------------------------------------------- |
| `Element`         | A render-pass-owned description built by chaining methods                                            |
| `div()`           | An element with no layout of its own                                                                 |
| `svg(path)`       | A vector image from the application root, tinted by the surrounding text color                       |
| `image(path)`     | A full-color image from the application root, colors preserved                                       |
| `list(…)`         | GPUI's lazy list: rows of any height, measured as they are drawn                                     |
| `uniform_list(…)` | GPUI's uniform list: one row measured, every row placed by it                                        |
| `PathBuilder`     | The GPUI path-builder type and its factory: `fill()` and `stroke(width)` each return a `PathBuilder` |
| `Background`      | `solid`, `stop`, `linear_gradient`, `pattern_slash`, `checkerboard`                                  |

`PathBuilder.fill()` and `.stroke(width)` return a handle that chains `move_to`, `line_to`, `curve_to`, `cubic_bezier_to`, `arc_to`, `add_polygon`, `close` and `dash_array`, and ends in `build()`. Paint the result with `window.paint_path(path, background)` — the one element constructor reached through an object, because the thing it mirrors is a method on the window.

`list` and `uniform_list` are GPUI's own lazy lists and take `(id, item_count, get_key, render)`, the shape of `gpui-base`'s `v_virtual_list` without its `item_sizes`: no sizes, because GPUI measures the items itself. `uniform_list` measures one row and places every row by it, so `render(range, cx)` returns one element per item in the range as a virtual list's does. `list` measures each item it draws and keeps the sizes, so `render(index, cx)` returns one element for one item, and rows — or panels — of unequal height need not say how tall they are. Both draw only what is on screen plus a short band past the fold, scroll themselves, and pair with a `Scrollbar` by name; neither takes a `VirtualListScrollHandle`.

A string is an element too, exactly as `&str` implements `IntoElement` in GPUI: `.child("hello")` is how text is written, and the style comes from the element holding it.

### Views

| Name        | What it is                                                                |
| ----------- | ------------------------------------------------------------------------- |
| `View`      | The base class of every View; subclass it and default-export the subclass |
| `ViewClass` | A concrete `View` subclass, as `cx.new` takes it                          |
| `Entity`    | Retained ownership of one nested View: `set_props(props)`, `release()`    |

A subclass defines `init?(props, cx)`, which runs once, and `render(cx)`, which returns one `Element`, `Entity` or string and runs when the View is invalidated. An optional `update(props)` runs when a parent changes a nested View's props.

### Scheduling

| Name    | What it is                                                  |
| ------- | ----------------------------------------------------------- |
| `Task`  | A running task: `cancel()`, `is_done()`                     |
| `Timer` | `after(ms, handler, opts?)` and `every(ms, handler, opts?)` |

### Focus

| Name          | What it is                                                  |
| ------------- | ----------------------------------------------------------- |
| `FocusHandle` | A focus target the script owns; [its members](#focushandle) |

### Shared types

| Name               | What it is                                                                                                                         |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| `Length`           | A number (pixels), `"12px"`, `"1.5rem"`, `"50%"` or `"auto"`                                                                       |
| `DefiniteLength`   | The same without `"auto"`                                                                                                          |
| `AbsoluteLength`   | Pixels or rems only                                                                                                                |
| `Axis`             | `"horizontal"` or `"vertical"`, mirroring `gpui_kit::Axis`                                                                         |
| `Color`            | A `gpui-base` `ColorToken`, or a `#rgb` / `#rrggbb` / `#rrggbbaa` literal                                                          |
| `Role`             | An accessibility role, mirroring `gpui_kit::Role` in snake_case                                                                    |
| `Anchor`           | Which corner of an anchored surface is pinned to its trigger                                                                       |
| `MouseButton`      | `"left"`, `"right"` or `"middle"`                                                                                                  |
| `ClickEvent`       | `click_count`, `modifiers`                                                                                                         |
| `MouseMoveEvent`   | `position`, `local_position`, `bounds`, `modifiers`                                                                                |
| `MouseButtonEvent` | `button`, `click_count`, `position`, `modifiers`, and the local geometry once painted                                              |
| `ScrollWheelEvent` | `delta` in pixels, `delta_lines` when the device reported lines, `touch_phase`                                                     |
| `KeyEvent`         | `keystroke` (the whole chord; the platform modifier is spelled `cmd` on every platform), `key`, `key_char`, `modifiers`, `is_held` |
| `ActionEvent`      | `action` — the script's own name for it                                                                                            |
| `KeyBinding`       | One entry of `cx.bind_keys`: `keystroke`, `action`, optional `context`                                                             |
| `Size`             | `width`, `height`                                                                                                                  |
| `Modifiers`        | `shift`, `control`, `alt`, `platform`                                                                                              |
| `Point`            | `x`, `y`                                                                                                                           |
| `Path`             | Immutable native geometry produced by `PathBuilder.build()`                                                                        |
| `Background`       | A reusable native background from `Background.solid(...)` or another factory: `opacity(factor)`, `color_space(space)`              |
| `BackgroundStop`   | One gradient stop, from `Background.stop(color, percentage)`                                                                       |

#### `FocusHandle`

Created with `cx.focus_handle()`, handed to an element with `track_focus(handle)`, and released with `release()`.

| Method                  | What it does                                      |
| ----------------------- | ------------------------------------------------- |
| `focus(): void`         | Moves the keyboard onto the element tracking it   |
| `is_focused(): boolean` | Whether that element currently has it             |
| `release(): boolean`    | Releases it and reports whether it was still live |

## The `gpui-shell` module

These are type-only concepts introduced by the JavaScript bridge itself. Import them only for type checking; the module has no run-time values.

| Name               | What it is                                                                                                                                                                                                                          |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `LengthString`     | The string forms accepted by the shell's length bridge                                                                                                                                                                              |
| `PathCoordinate`   | Pixels, or a percentage of the painted element's bounds                                                                                                                                                                             |
| `Props`            | The property bag carried across the JavaScript View bridge                                                                                                                                                                          |
| `ElementBounds`    | A shell event `Point` with `width` and `height`                                                                                                                                                                                     |
| `ScopePhase`       | `"render"`, `"event"`, `"task"`, `"layout"` or `"none"`                                                                                                                                                                             |
| `TaskOptions`      | `{ owner?: View \| null }` — the View the task is cancelled with. Defaults to the running View; `null` outlives every View                                                                                                          |
| `DialogOptions`    | `{ escape_dismissable?: boolean, backdrop_dismissable?: boolean }`, both `true` by default                                                                                                                                          |
| `ToastOptions`     | `{ title: string, description?: string, level?: "info" \| "success" \| "warning" \| "error", timeout?: number \| null, id?: string }`. `level` defaults to `"info"`; `timeout` to five seconds, and `null` keeps it until dismissed |
| `MotionProperty`   | `"opacity"`, `"width"`, `"height"`, `"left"`, `"top"`                                                                                                                                                                               |
| `MotionEasing`     | `"linear"`, `"ease-in"`, `"ease-out"`, `"ease-in-out"`                                                                                                                                                                              |
| `TransitionPolicy` | `duration`, `delay`, `easing`                                                                                                                                                                                                       |
| `SpringPolicy`     | `response`, `damping`, `epsilon`                                                                                                                                                                                                    |

`ScopePhase` describes which shell call owns the current `Context`. It is unrelated to GPUI's `DispatchPhase`, which controls capture and bubble ordering during event dispatch.

## The `cx` context

There are two context lifetimes with the same methods. `Context`, received by `render` and event handlers, belongs to that host call; retaining it beyond the call, including across an `await`, reports a stale-context error. `AsyncContext`, described below, is the flavour intended to survive an `await`.

| Member                     | What it is                                                                                         |
| -------------------------- | -------------------------------------------------------------------------------------------------- |
| `notify()`                 | Requests a re-render; throws during `render`, because notifying yourself while rendering is a loop |
| `bind_keys(bindings)`      | Installs key bindings and answers how many; `App::bind_keys`                                       |
| `stop_propagation()`       | Keeps this event from reaching the handlers above; `App::stop_propagation`                         |
| `propagate()`              | Undoes that within the same dispatch; `App::propagate`                                             |
| `phase()`                  | Which `ScopePhase` the call is in                                                                  |
| `theme()`                  | The current `gpui_kit::base::Theme` semantic token projection                                      |
| `open_url(url)`            | Hands an absolute `http`/`https` URL to the system handler                                         |
| `read_from_clipboard()`    | The clipboard's text, or `undefined` when it holds none                                            |
| `write_to_clipboard(text)` | Replaces the clipboard's text                                                                      |
| `focus_handle()`           | A new `FocusHandle`; belongs in `init` or an event handler, never in `render`                      |
| `new(Class, props?)`       | Creates a retained nested View and answers the `Entity` that owns it                               |
| `spawn(body, opts?)`       | Runs `body(cx)` and adopts the promise it returns, so a rejection is reported                      |
| `sleep(ms?)`               | Resolves after `ms` on GPUI's foreground executor                                                  |
| `timer`                    | The `Timer`: `after` and `every`                                                                   |

Several of these name the GPUI method they mirror: `open_url` is `App::open_url`, `read_from_clipboard` and `write_to_clipboard` are `App::read_from_clipboard` and `App::write_to_clipboard`, `focus_handle` is `App::focus_handle` (GPUI has no `FocusHandle::new`, and neither does this), `new` is `AppContext::new`, and `spawn` is `App::spawn`.

### `AsyncContext`

`AsyncContext` extends `Context` and adds no members. The difference is lifetime, not surface: an ordinary `Context` speaks for one host call and reports clearly once that call has returned, while an `AsyncContext` names no call at all — it resolves whichever is running when a member is used, and refuses only when none is. It is the mirror of GPUI's `AsyncApp`.

Three places hand one out: `init`, the body of `cx.spawn`, and the callbacks of `cx.timer`. Those are the three whose job is to set up or continue work that outlives the call it was started from.

## The `window` global

The global has the `Window` type exported by `gpui-kit`. Nothing hands it to you and there is nothing to import at the call site. Every call reads the host call that is running now and throws outside one, so there is no handle to hold and nothing that can go stale. An overlay belongs to the window rather than to the View that opened it, which is why these are here and not on `Context`.

| Member                                                                              | What it is                                                                    |
| ----------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `open_dialog(content, options?)`                                                    | Opens a dialog and answers the stack's new depth                              |
| `close_dialog()`                                                                    | Closes the topmost dialog, and answers whether it found one                   |
| `close_all_dialogs()`                                                               | Closes every dialog, and answers how many                                     |
| `has_active_dialog()`                                                               | Whether any dialog is open; legal from `render`, unlike the rest              |
| `open_sheet(content)`                                                               | Opens the sheet on the right, replacing whatever was there                    |
| `open_sheet_at(placement, content)`                                                 | The same, anchored at the `gpui-base` `Placement` you name                    |
| `close_sheet()`                                                                     | Closes the sheet, and answers whether one was open                            |
| `has_active_sheet()`                                                                | Whether the sheet is open; legal from `render`                                |
| `push_toast(options)`                                                               | Posts a toast and answers its id                                              |
| `remove_toast(id)`                                                                  | Retracts one toast, and answers whether it was still showing                  |
| `clear_toasts()`                                                                    | Retracts every toast, and answers how many                                    |
| `paint_path(path, background)`                                                      | Paints immutable geometry with a native background; `Window::paint_path`      |
| `dispatch_action(action)`                                                           | Dispatches an action down this window's focus path; `Window::dispatch_action` |
| `rem_size()` / `line_height()`                                                      | The window's type metrics, in pixels                                          |
| `viewport_size()` / `bounds()`                                                      | The drawable area, and where the window sits on screen                        |
| `mouse_position()`                                                                  | Where the pointer is, in window coordinates                                   |
| `appearance()`                                                                      | `"light"` or `"dark"`                                                         |
| `is_window_active()` / `is_fullscreen()` / `is_maximized()`                         | The platform window's state                                                   |
| `set_rem_size(size)`                                                                | Rescales everything expressed in rems                                         |
| `refresh()`                                                                         | Redraws every View in the window                                              |
| `focus_next()` / `focus_prev()`                                                     | Moves the keyboard one tab stop                                               |
| `activate_window()` / `minimize_window()` / `zoom_window()` / `toggle_fullscreen()` | Platform window controls                                                      |
| `localStorage`                                                                      | Web Storage backed by a file the host placed; survives a restart              |
| `sessionStorage`                                                                    | Web Storage held in memory; goes with the process                             |

The measurements — everything from `rem_size()` down to `is_maximized()` — are legal from `render`, because a View that sizes itself from the window has to ask during the pass that draws it. Everything that _changes_ the window is refused there, for the reason `cx.notify()` is: a frame that changes the window it is drawing into is a frame arguing with itself.

`open_dialog`, `open_sheet` and `open_sheet_at` take a **function returning an element**, not an element: a dialog outlives the call that opened it, and the function runs again whenever it redraws. Everything here except the two `has_active_*` queries and `paint_path` is illegal from `render`. See [Overlays](./overlays.md).

### Storage

The [Web Storage API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API), unchanged. Both stores are also bare globals — `localStorage.getItem(k)` and `window.localStorage.getItem(k)` are the same call — because that is true in a browser too.

| Member                | What it is                                     |
| --------------------- | ---------------------------------------------- |
| `length`              | How many keys are stored                       |
| `key(index)`          | The key at that position, or `null`            |
| `getItem(key)`        | The value, or `null` when the key is unset     |
| `setItem(key, value)` | Stores it, converting the value to a string    |
| `removeItem(key)`     | Forgets one key                                |
| `clear()`             | Forgets all of them                            |
| `flush()`             | Resolves once the writes have reached the disk |

Values are strings, so structure goes through `JSON.stringify` and `JSON.parse` exactly as it would on the web. `flush()` is the one addition: a browser never needs it, because its storage is synchronous all the way down. `localStorage` is capability-gated and throws when the host did not grant it; `sessionStorage` never is, because nothing it holds leaves the process. See [Capabilities](./capabilities.md#storage).

## The `gpui-base` module

The components here own behavior, focus and what a screen reader hears, and draw next to nothing themselves. The picture is the script's, written with the [style surface](./styling.md). Each name links to the component's own page in the [gpui-base documentation](../base/index.md), which is where its full Rust surface and its behavior are described.

### Layout

| Name                                                   | What it is                                                                    |
| ------------------------------------------------------ | ----------------------------------------------------------------------------- |
| `h_flex()`                                             | A row                                                                         |
| `v_flex()`                                             | A column                                                                      |
| [`h_resizable(id)`](../base/primitives/resizable.md)   | A row of panes with draggable dividers; sizes live in the window under the id |
| [`v_resizable(id)`](../base/primitives/resizable.md)   | The same, stacked                                                             |
| [`resizable_panel()`](../base/primitives/resizable.md) | One pane of a resizable group, and legal nowhere else                         |

### Controls

| Name                                                  | What it is                                                                                                                  |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| [`Button`](../base/primitives/button.md)              | Activation, focus, disabled and selected state                                                                              |
| [`Link`](../base/primitives/link.md)                  | An external HTTP(S) resource opened through the system browser                                                              |
| [`Checkbox`](../base/primitives/checkbox.md)          | A controlled toggle; draw the indicator yourself                                                                            |
| [`Switch`](../base/primitives/switch.md)              | A controlled switch                                                                                                         |
| [`Radio`](../base/primitives/radio.md)                | One option in a group; reports `true` only, never a deselection                                                             |
| [`Toggle`](../base/primitives/toggle.md)              | A button that stays down                                                                                                    |
| [`RadioGroup`](../base/primitives/radio-group.md)     | A set of radios announced as one group; holds no selection                                                                  |
| [`ToggleGroup`](../base/primitives/toggle-group.md)   | A set of toggles announced as a toolbar                                                                                     |
| [`Tabs`](../base/primitives/tabs.md)                  | A tab list that holds no selection of its own                                                                               |
| [`Tab`](../base/primitives/tabs.md)                   | One tab: `selected(...)` in, `on_click(...)` out                                                                            |
| [`Progress`](../base/primitives/progress.md)          | The announcement, not the bar; `Progress.new(...)` alone draws nothing                                                      |
| [`ProgressTrack`](../base/primitives/progress.md)     | The groove: a plain element you size and color                                                                              |
| [`ProgressIndicator`](../base/primitives/progress.md) | The filled part; set its width from the percentage you announced                                                            |
| [`Avatar`](../base/primitives/avatar.md)              | Renders its `image` slot, or its `fallback` when there is none; no circle, size or background of its own                    |
| [`AvatarImage`](../base/primitives/avatar.md)         | The image slot: `AvatarImage.new(path)`, and legal nowhere else                                                             |
| [`AvatarFallback`](../base/primitives/avatar.md)      | The fallback slot: an ordinary box holding initials, a shape or an `svg`                                                    |
| [`Pagination`](../base/primitives/pagination.md)      | A navigation landmark carrying the announced label; the page buttons are yours                                              |
| `pagination_items(current, total, visible?)`          | Which page numbers to draw and where the gaps fall. `visible` defaults to 7, floors at 5; one page or fewer answers nothing |
| [`Accordion`](../base/primitives/accordion.md)        | A group holding items                                                                                                       |
| [`AccordionItem`](../base/primitives/accordion.md)    | One item: `open(...)` in, the trigger's `on_change(...)` out; it passes its `open` down to both halves                      |
| [`AccordionHeader`](../base/primitives/accordion.md)  | The heading: `AccordionHeader.new(trigger)`, with `aria_level(n)` announcing its level (default 3)                          |
| [`AccordionPanel`](../base/primitives/accordion.md)   | The revealed region. Out of the tree while shut, unless `keep_mounted(true)`                                                |
| [`AccordionTrigger`](../base/primitives/accordion.md) | The button: announces the expanded state, and `on_change` asks for the other one                                            |
| [`CalendarState`](../base/primitives/calendar.md)     | Retained calendar state: the month grid, the month being shown, and the chosen date                                         |
| [`SliderState`](../base/primitives/slider.md)         | Retained slider state, and where a drag writes                                                                              |
| [`Slider`](../base/primitives/slider.md)              | The root: announces the value and owns the release                                                                          |
| [`SliderTrack`](../base/primitives/slider.md)         | The press and drag surface                                                                                                  |
| [`SliderIndicator`](../base/primitives/slider.md)     | The groove, and the box every pointer position is measured against                                                          |
| [`SliderThumb`](../base/primitives/slider.md)         | The knob; the shell gives it a place, you give it a look                                                                    |

All four slider parts take the same `SliderState`, and all four are needed — a slider with no `SliderIndicator` cannot be moved at all.

### Text editing

| Name                                                | What it is                                                                      |
| --------------------------------------------------- | ------------------------------------------------------------------------------- |
| [`InputState`](../base/primitives/input.md)         | Retained text state: `InputState.new({ placeholder, value })`                   |
| [`Input`](../base/primitives/input.md)              | The frame around retained text state                                            |
| [`NumberInput`](../base/primitives/number-input.md) | A spinbutton over the same `InputState`, with three slots that all carry weight |
| [`TextareaState`](../base/primitives/textarea.md)   | Retained multi-line text state; `rows` is an option                             |
| [`Textarea`](../base/primitives/textarea.md)        | The frame around retained multi-line state                                      |
| [`OtpState`](../base/primitives/otp-input.md)       | Retained one-time-code state; the length is fixed when it is created            |
| [`OtpInput`](../base/primitives/otp-input.md)       | A fixed-length code whose cells the shell draws and the script styles           |

There is no numeric state type: an `InputState` becomes a number state by being given `set_step`, `set_min` and `set_max`.

### Containers and overlays

| Name                                               | What it is                                                                              |
| -------------------------------------------------- | --------------------------------------------------------------------------------------- |
| [`Collapsible`](../base/primitives/collapsible.md) | Renders its `content` slot only while `open`; no role, chevron or trigger               |
| [`Popover`](../base/primitives/popover.md)         | A surface anchored to a trigger and opened by a press                                   |
| [`HoverCard`](../base/primitives/hover-card.md)    | The same, opened by resting the pointer, with its own open state                        |
| [`Popup`](../base/primitives/popup.md)             | The bare anchored surface: `Popup.new(id, trigger)`, opened by filling `content`        |
| [`Select`](../base/primitives/select.md)           | A combobox root: the role, the announced open state, the keyboard — none of the picture |
| [`Combobox`](../base/primitives/combobox.md)       | The same root, announced as a combobox whose trigger is an editable field               |
| [`DatePicker`](../base/primitives/date-picker.md)  | A date-picker root: `DatePicker.new(id, focus_handle)`; it holds no date                |

Two gaps are worth knowing before you build on these: arrow-key navigation of an open `Select` or `Combobox` list is yours to wire (the pieces are there — see below), and Enter and Escape do not reach a `DatePicker`. Both are described where they bite, in the declarations for each type.

### Tables and lists

| Name                                                 | What it is                                                             |
| ---------------------------------------------------- | ---------------------------------------------------------------------- |
| [`Table`](../base/primitives/table.md)               | A semantic table root, composed the way HTML composes one              |
| [`TableHeader`](../base/primitives/table.md)         | The header row group                                                   |
| [`TableBody`](../base/primitives/table.md)           | The body row group                                                     |
| [`TableRow`](../base/primitives/table.md)            | One row: `.new(id, row_index)`, one-based                              |
| [`TableHead`](../base/primitives/table.md)           | One column header: `.new(id, column_index)`, one-based                 |
| [`TableCell`](../base/primitives/table.md)           | One data cell: `.new(id, column_index)`, one-based                     |
| [`TableCaption`](../base/primitives/table.md)        | The visual slot a caption belongs in; it carries no caption role       |
| [`v_virtual_list(…)`](../base/virtual-list.md)       | A vertical list that describes only what is on screen                  |
| [`h_virtual_list(…)`](../base/virtual-list.md)       | The same along the other axis; `item_sizes` are widths                 |
| [`VirtualListScrollHandle`](../base/virtual-list.md) | A virtual list's scroll position, kept across frames                   |
| [`Scrollbar`](../base/primitives/scrollbar.md)       | `new(id)`, `horizontal(id)`, `vertical(id)` — a bar you place yourself |

Both virtual lists take `(id, item_count, item_sizes, get_key, render)`. `render(range, cx)` is the only callback in this API that the host calls _during_ a frame, which is why handlers, retained state and `cx.notify()` are all refused inside it.

### Dock

| Name                                   | What it is                                                                                   |
| -------------------------------------- | -------------------------------------------------------------------------------------------- |
| `DockArea.new(id, options?)`           | A dockable layout, retained: `options` is `{ version?: number }`                             |
| `DockArea.register_panel(name, Class)` | Teaches the runtime to rebuild `name`'s panel from `Class`; answers with the namespaced name |
| `dock_area(area)`                      | Draws one, and carries the six chrome handlers                                               |
| `dock_content()`                       | Where a dock's own panels go inside the chrome drawn around them                             |

The area's methods are `add_panel(view, options)`, `remove_panel(id)`, `panels()`, `dump()`, `load(state)`, `has_dock`, `is_dock_open`, `toggle_dock`, `remove_dock`, `dock_size`, `set_dock_size`, `set_dock_collapsible`, `is_locked`, `set_locked`, `is_zoomed`, `zoom_out`, `on("layout_changed", handler)` and `release()`.

**Every edit is applied once the call that made it has returned**, in the order the calls were made — a panel's body comes from `cx.new(Class)`, which is still being constructed — so `panels()` and `dump()` read the layout as it was before this turn's edits. See [Dock and Panels](./dock.md).

### Retained handles

Each is created once — in `init` or an event handler, never in `render` — and every one of them has `release(): boolean`, which returns whether it was still live. Using a handle after releasing it throws.

`on(...)` replaces the handler for that event rather than adding a second one, and answers whether there was one before.

#### `InputState`

From `InputState.new(options?)`, where `options` is `{ placeholder?: string, value?: string }`.

| Method                                 | What it does                                                                              |
| -------------------------------------- | ----------------------------------------------------------------------------------------- |
| `value(): string`                      | The current text                                                                          |
| `set_value(next: string): void`        | Replaces it                                                                               |
| `on(event, handler): boolean`          | `event` is `"change"`, `"submit"`, `"focus"` or `"blur"`; the handler takes `(event, cx)` |
| `set_step(step: number \| null): void` | The `NumberInput` step, or `null` for none                                                |
| `set_min(min: number \| null): void`   | The numeric floor, or `null`                                                              |
| `set_max(max: number \| null): void`   | The numeric ceiling, or `null`                                                            |
| `set_masked(masked: boolean): void`    | Whether the text is drawn as a password                                                   |
| `set_loading(loading: boolean): void`  | Whether the field shows its loading state                                                 |

#### `TextareaState`

From `TextareaState.new(options?)`, where `options` is `{ placeholder?: string, value?: string, rows?: number }`.

| Method                                                    | What it does                                                         |
| --------------------------------------------------------- | -------------------------------------------------------------------- |
| `value(): string`                                         | The current text                                                     |
| `set_value(next: string): void`                           | Replaces it                                                          |
| `on(event, handler): boolean`                             | `"change"`, `"submit"`, `"focus"` or `"blur"`, handler `(event, cx)` |
| `set_rows(rows: number): void`                            | The visible row count                                                |
| `set_auto_grow(min_rows: number, max_rows: number): void` | Grows with its content between the two                               |
| `set_soft_wrap(wrap: boolean): void`                      | Whether long lines wrap                                              |

#### `SliderState`

From `SliderState.new(options?)`, where `options` is `{ min?, max?, step?, scale?: "linear" | "logarithmic", value?: SliderValue }`. The defaults are `0..100` in steps of `1`, starting at `min`. A `"logarithmic"` scale needs a `min` above zero.

| Method                               | What it does                                                               |
| ------------------------------------ | -------------------------------------------------------------------------- |
| `value(): SliderValue`               | The current value: a number, or `[start, end]` for a range                 |
| `set_value(next: SliderValue): void` | Replaces it                                                                |
| `min_value(): number`                | The floor it was built with                                                |
| `max_value(): number`                | The ceiling                                                                |
| `step_value(): number`               | The step                                                                   |
| `on(event, handler): boolean`        | `"change"` while dragging or `"release"` at the end; handler `(value, cx)` |

#### `OtpState`

From `OtpState.new(length, options?)`, where `options` is `{ value?: string, masked?: boolean }`. The length is fixed at creation.

| Method                              | What it does                                                                                         |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `value(): string`                   | The digits entered so far                                                                            |
| `set_value(next: string): void`     | Replaces them                                                                                        |
| `len(): number`                     | How many digits it holds                                                                             |
| `is_masked(): boolean`              | Whether they are drawn masked                                                                        |
| `set_masked(masked: boolean): void` | Changes that                                                                                         |
| `focus(): void`                     | Moves the keyboard into it                                                                           |
| `on(event, handler): boolean`       | `"change"` after each edit, `"complete"` when filled, or `"focus"` / `"blur"`; handler `(event, cx)` |

#### `VirtualListScrollHandle`

From `VirtualListScrollHandle.new()`, handed to a list with `track_scroll(handle)`.

| Method                                           | What it does                                                                                |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------- |
| `scroll_to_item(index: number, strategy?): void` | Puts an item on screen before the next frame; `strategy` is `"top"` (default) or `"center"` |
| `scroll_to_bottom(): void`                       | Scrolls to the end                                                                          |

### Calendar

`CalendarState` exists for `month_days()` — which dates fall in which week, where the neighbouring months' days go, and how many weeks this month needs. You draw the cells.

```js
const grid = this.calendar.month_days()[0];
v_flex().children(
  grid.map((week) =>
    h_flex().children(
      week.map((day) =>
        Button.new(day)
          .selected(day === this.calendar.value())
          .on_click((_, cx) => {
            this.calendar.set_value(day);
            cx.notify();
          })
          .child(String(Number(day.slice(8)))),
      ),
    ),
  ),
);
```

Base's `Calendar` element is **not** bound, and that is a decision rather than an omission: it walks the same grid calling a renderer once per cell — up to forty-two crossings into JavaScript per frame, from inside GPUI's layout pass, for cells that carry no behavior. Reading the grid here and drawing it yourself is the same work without them.

Dates are `"YYYY-MM-DD"`: sorting them as text sorts them by time, and `new Date(s)` reads one — which is where a weekday name or a localized month label comes from.

| Method                          | What it does                                                                                                      |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `month_days()`                  | The grid, as months of weeks of days. Every week is seven days; the first and last carry the neighbouring months' |
| `year()` / `month()`            | The year and month (1–12) the grid is for                                                                         |
| `today()`                       | Today, as the state read it when it was created                                                                   |
| `value()` / `set_value(next)`   | The selection: one day, a `[start, end]` range, or `null`                                                         |
| `next_month()` / `prev_month()` | Moves the grid a month either way; illegal from `render`                                                          |
| `on("change", handler)`         | The only event, reporting a date being selected                                                                   |

### Theme

| Name                  | What it is                                                                     |
| --------------------- | ------------------------------------------------------------------------------ |
| `set_theme(theme)`    | Replaces `gpui-base`'s active semantic tokens with an application-owned theme  |
| `ColorToken`          | The semantic color names defined by the installed palette                      |
| `Theme`               | What `cx.theme()` answers: the semantic tokens plus `appearance` and `is_dark` |
| `SemanticThemeTokens` | `colors`, `spacing`, `radius`                                                  |
| `ColorTokens`         | One `Color` per semantic role                                                  |
| `SpacingTokens`       | `xxs` `xs` `sm` `md` `lg` `xl` `xxl`                                           |
| `RadiusTokens`        | `none` `sm` `md` `lg` `xl` `full`                                              |

Reading the theme is `cx.theme()`. `set_theme` remains in `gpui-base` because the theme belongs to that layer. Mutation still requires a live host call and is legal only from an event handler or task, never from `render` or layout.

### Other types

| Name                    | What it is                                                                                                    |
| ----------------------- | ------------------------------------------------------------------------------------------------------------- |
| `ScrollbarMode`         | `"scrolling"`, `"hover"` or `"always"`                                                                        |
| `ItemRange`             | A virtual list's visible items, as a half-open `[start, end)`                                                 |
| `SliderValue`           | A number, or `[start, end]` for a range slider                                                                |
| `InputEvent`            | The text-state event payload; submit events carry optional `secondary` and `shift` flags                      |
| `OtpEvent`              | The currently empty OTP event payload; read the value from `OtpState`                                         |
| `PartType`              | The shared `new()` shape used by `gpui-base` sub-parts without their own identity                             |
| `Placement`             | `"top"`, `"bottom"`, `"left"` or `"right"`, mirroring `gpui_kit::base::Placement`                             |
| `ComponentType`         | The shared `new(id)` shape used by identity-bearing `gpui-base` component constructors                        |
| `DockPlacement`         | `"center"`, `"left"`, `"right"` or `"bottom"`                                                                 |
| `DockPanel`             | One panel as `panels()` reports it: `id`, `name`, `placement`, `node`, `index`, `active`, and its three flags |
| `DockGroup` / `DockTab` | A tab group and one of its tabs, as `tab_bar` and `empty_group` are given them                                |
| `DockRegion`            | One dock, as the `dock` handler is given it                                                                   |
| `DockTile`              | One tile, with already-resolved bounds                                                                        |
| `DockDrop`              | Where a dragged panel would land                                                                              |
| `TileResizeSide`        | `"left"`, `"right"`, `"top"`, `"bottom"` or `"bottom_right"`                                                  |

### Composition patterns

Five of these components are not one element but an arrangement, and the tables above cannot say so. Each snippet below is the smallest thing that works; all of them were checked against the runtime.

**A controlled control.** `Checkbox`, `Switch`, `Radio` and `Toggle` hold no state: you read the value in and write it back out. Nothing is drawn for you, so the indicator is a child.

```js
Checkbox.new("done")
  .checked(this.checked)
  .on_change((checked, cx) => {
    this.checked = checked;
    cx.notify();
  })
  .child(this.checked ? "done" : "not done");
```

**`Progress` announces; the bar is yours.** The root carries the role and the `0..=100` a screen reader reads, and draws nothing at all.

```js
Progress.new("upload")
  .value(62)
  .child(
    ProgressTrack.new()
      .w(200)
      .h(6)
      .bg(cx.theme().colors.muted)
      .child(ProgressIndicator.new().w(124).h(6).bg(cx.theme().colors.primary)),
  );
```

**A slider is four parts, and all four are needed** — a slider with no `SliderIndicator` cannot be moved, because that is the box every pointer position is measured against. All four take the same state.

```js
Slider.new(this.volume).child(
  SliderTrack.new(this.volume)
    .w(200)
    .h(16)
    .child(SliderIndicator.new(this.volume).h(4).bg(cx.theme().colors.primary))
    .child(
      SliderThumb.new(this.volume).w(12).h(12).bg(cx.theme().colors.background),
    ),
);
```

**`Select` owns the keyboard, `Popup` owns the surface.** The root holds the combobox role and the open state; the list is a `Popup` inside it. It needs two focus handles — one for the trigger, one for the content — and without the first nothing on screen has the keyboard.

```js
Select.new("mode")
  .accessibility_label("Mode")
  .open(this.open)
  .track_focus(this.trigger)
  .content_focus_handle(this.list)
  .on_open_change((open, cx) => {
    this.open = open;
    cx.notify();
  })
  .child(
    Popup.new("mode-list", trigger)
      .anchor("bottom_left")
      .when(this.open, (el) => el.content(list)),
  );
```

Arrow-key navigation of an open list is yours to write: base expects whatever is inside to run the highlight from its own key bindings, and nothing does that for you. The pieces are here — put `on_key_down` on the content element the keyboard was moved to, or bind ↑ / ↓ to actions under a `key_context` of your own. Out of the box the pointer works, Escape closes, Enter and ↓ open, and the highlight does not move.

**A virtual list and its scrollbar are paired by name.** The list paints no bar of its own, and nothing checks the pairing before it runs, so both halves are needed.

```js
v_flex()
  .relative()
  .h(200)
  .child(
    v_virtual_list(
      "rows",
      rows.length,
      28,
      (index) => rows[index].id,
      (range) =>
        rows.slice(range.start, range.end).map((row) => div().child(row.name)),
    ).size_full(),
  )
  .child(Scrollbar.vertical("rows").absolute().inset_0());
```

**A nested View is created once and mounted as a child.** `cx.new` belongs in `init` or an event handler; the entity is a child wherever a child is taken.

```js
init(props, cx) {
  this.chart = cx.new(PriceChart, { symbol });
}
render() {
  return v_flex().child(this.chart);
}
```

## The `gpui-fps` module

| Name            | What it is                                                                    |
| --------------- | ----------------------------------------------------------------------------- |
| `fps_monitor()` | The native `gpui-fps` HUD, shared once per window and pinned to the top right |

Its parent must be `relative()`. The HUD owns its own presentation; ordinary styles and children do not apply to it.

## Element methods

Every element shares one prototype, so every method below type-checks on every element — which component a method actually suits is not expressed by the types. A behavior builder handed to a component that does not honour it is reported in the log rather than dropped in silence.

Element builder methods answer the same element, so a chain is one expression. `map` is the exception: like GPUI's `FluentBuilder.map`, it returns exactly what its callback returns. An element is consumed when it is used as a child and belongs to the render pass that built it.

### Composition

| Method                    | What it does                                                                                            |
| ------------------------- | ------------------------------------------------------------------------------------------------------- |
| `map(transform)`          | Passes the current element to `transform` and returns its result, matching GPUI's fluent builder helper |
| `child(value)`            | Adds one child: an element, an `Entity`, or a string, number or boolean                                 |
| `children(iterable)`      | Adds several, in order                                                                                  |
| `when(condition, branch)` | Applies `branch` when `condition` is truthy, keeping the chain in one piece                             |
| `id(name)`                | A stable name for this element, used as its identity                                                    |

### Slots

A slot is not a child: the element is consumed by the component and rendered where the component decides.

| Method                      | What it does                                                                               |
| --------------------------- | ------------------------------------------------------------------------------------------ |
| `content(element)`          | The content of a `Collapsible`, `Popover`, `HoverCard` or `Popup`                          |
| `image(element)`            | An `Avatar`'s image slot; takes an `AvatarImage`                                           |
| `fallback(element)`         | An `Avatar`'s fallback slot; takes an `AvatarFallback`                                     |
| `header(element)`           | An `AccordionItem`'s header slot; takes an `AccordionHeader`                               |
| `panel(element)`            | An `AccordionItem`'s panel slot; takes an `AccordionPanel`                                 |
| `trigger(element)`          | The trigger of a `Popover` or `HoverCard`                                                  |
| `input(element)`            | The editor slot of a `NumberInput`; empty draws the bare editor                            |
| `decrement_button(element)` | The look of a `NumberInput`'s decrement button — replayed onto base's button, not rendered |
| `increment_button(element)` | The increment button, replayed the same way                                                |
| `controls_right()`          | Stacks both step buttons to the right of the text                                          |

### Events

| Method                           | What it delivers                                                                    |
| -------------------------------- | ----------------------------------------------------------------------------------- |
| `on_click(handler)`              | `(ClickEvent, cx)` on activation                                                    |
| `on_mouse_move(handler)`         | `(MouseMoveEvent, cx)` while the element is hovered                                 |
| `on_hover(handler)`              | `(hovered, cx)` on both pointer entry and exit                                      |
| `on_key_down(handler)`           | `(KeyEvent, cx)` while this element holds the keyboard                              |
| `on_key_up(handler)`             | `(KeyEvent, cx)` on the same focus path                                             |
| `on_mouse_down(button, handler)` | `(MouseButtonEvent, cx)` on a press of that button                                  |
| `on_mouse_up(button, handler)`   | `(MouseButtonEvent, cx)` on its release                                             |
| `on_mouse_down_out(handler)`     | `(MouseButtonEvent, cx)` on a press anywhere outside this element                   |
| `on_scroll_wheel(handler)`       | `(ScrollWheelEvent, cx)` on wheel or trackpad scrolling                             |
| `on_action(action, handler)`     | `(ActionEvent, cx)` when that named action is dispatched to this element or into it |
| `on_change(handler)`             | `(checked, cx)` on a toggle; the script owns the new value                          |
| `on_step(handler)`               | `("increment" \| "decrement", cx)`, and it **replaces** built-in stepping           |
| `on_item_click(handler)`         | `(key, cx)` when a virtual list row is clicked, keyed rather than indexed           |
| `on_open_change(handler)`        | `(open, cx)` when something other than the script changed a `Popover`'s open state  |
| `on_confirm(handler)`            | Enter in an open `Select` or `Combobox`; no payload                                 |
| `on_dismiss(handler)`            | Escape in an open `Select` or `Combobox`, before `on_open_change(false)`            |
| `on_resize(handler)`             | `(sizes, cx)` once a resizable group's drag has ended                               |

### Actions and key bindings

An action is the level above a keystroke. `cx.bind_keys` says which chord means `"save"`, in which context; `on_action("save", ...)` on an element says what `"save"` does. A menu item or a toolbar button dispatching the same name through `window.dispatch_action("save")` reaches the same handler, and neither end has to know about the other.

```js
init(_props, cx) {
  cx.bind_keys([{ keystroke: "cmd-s", action: "save", context: "Editor" }]);
}

render(_cx) {
  return div()
    .key_context("Editor")
    .track_focus(this.handle)
    .on_action("save", (event, cx) => this.save(cx));
}
```

`context` is a predicate matched against the `key_context(...)` an element declares, so one chord can mean one thing in a list and another in an editor. Registering several `on_action`s on one element is fine and they are independent; an action none of them claims carries on to an element further out.

That group — `on_key_down`, `on_key_up`, the four pointer handlers, `on_action` and `key_context` — is wired on `div`, `h_flex`, `v_flex`, `Button`, `Link`, `Checkbox`, `Switch`, `Radio`, `Toggle`, `Tabs` and `Tab`. On any other component the handler is recorded and never reaches GPUI, and the log says so — wrap it and write the handler on the wrapper.

Wired is not the same as reachable. A key travels the focus path, so a component that accepts no focus handle — `Tab` — hears presses and never hears keys, however well both are wired.

### Control state

| Method                 | What it sets                                                                                                                                    |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `disabled(value)`      | Blocks activation and reports the state; draw it yourself                                                                                       |
| `selected(value)`      | The selected state of a `Button`                                                                                                                |
| `checked(value)`       | The controlled value of a `Checkbox`, `Switch` or `Radio`                                                                                       |
| `pressed(value)`       | The controlled state of a `Toggle`                                                                                                              |
| `value(percent)`       | The announced progress percentage, clamped to `0..=100`; it moves nothing on screen                                                             |
| `indeterminate(value)` | Withdraws a `Progress` value from the accessibility tree                                                                                        |
| `open(value)`          | Whether a `Collapsible` renders its content, or a surface is showing                                                                            |
| `default_open(value)`  | Whether an uncontrolled `Popover` starts open                                                                                                   |
| `keep_mounted(value)`  | Whether a shut `AccordionPanel` stays in the tree. Off by default; on, its content keeps a scroll position or a half-typed field across a close |
| `start(value)`         | Which thumb of a range slider a `SliderThumb` is                                                                                                |
| `href(url)`            | The absolute HTTP(S) target of a `Link`                                                                                                         |

### Accessibility

| Method                         | What it announces                                                                         |
| ------------------------------ | ----------------------------------------------------------------------------------------- |
| `accessibility_label(text)`    | What a screen reader says; an icon-only control announces nothing without it              |
| `role(name)`                   | What this element announces itself as — plain elements, `Button` and `Checkbox` only      |
| `aria_selected(value)`         | The selected state of an option in a list the script built                                |
| `aria_active_descendant()`     | This element as the focused one while an ancestor holds the keyboard                      |
| `set_position(position, size)` | One-based position and total size — "tab 2 of 5"                                          |
| `row_count(count)`             | A `Table`'s total rows, including unrendered ones                                         |
| `column_count(count)`          | A `Table`'s total columns                                                                 |
| `aria_level(level)`            | An `AccordionHeader`'s announced heading level, default 3; it announces, it sizes nothing |
| `axis(value)`                  | A `RadioGroup`'s or `ToggleGroup`'s orientation; semantic only, it lays out nothing       |
| `tooltip(text)`                | A pointer-only hover label, and no substitute for `accessibility_label`                   |

### Focus and keyboard

| Method                         | What it does                                                          |
| ------------------------------ | --------------------------------------------------------------------- |
| `track_focus(handle)`          | Makes this element what the handle means                              |
| `content_focus_handle(handle)` | Where a `Select` or `Combobox` moves the keyboard when it opens       |
| `tab_index(index)`             | Where this element sits in the Tab order; it also makes it a tab stop |
| `tab_stop(value)`              | Whether Tab can land here, without changing its place in the order    |

### Scrolling and panels

| Method                                              | What it does                                                     |
| --------------------------------------------------- | ---------------------------------------------------------------- |
| `overflow_scroll()`                                 | Owns wheel and touch scrolling on both axes                      |
| `overflow_x_scroll()` / `overflow_y_scroll()`       | The same on one axis                                             |
| `overflow_scrollbar()`                              | Scrolls both axes and paints base-layer bars                     |
| `overflow_x_scrollbar()` / `overflow_y_scrollbar()` | The same on one axis                                             |
| `mode(value)`                                       | A `Scrollbar`'s visibility policy; omitted, it follows the theme |
| `scroll_size(width, height)`                        | The content size a `Scrollbar` measures its thumb against        |
| `viewport_from_layout()`                            | Makes a `Scrollbar` take its viewport from its own box           |
| `track_scroll(handle)`                              | Gives a virtual list a scroll position the script can drive      |
| `with_item_to_measure_index(index)`                 | Which item a virtual list measures across the axis it scrolls    |
| `size_range(min, max?)`                             | How far a `resizable_panel()` may be dragged, in pixels          |

### Anchored surfaces

| Method                    | What it sets                                                              |
| ------------------------- | ------------------------------------------------------------------------- |
| `anchor(value)`           | Which corner is pinned to the trigger; clamped into the window either way |
| `mouse_button(value)`     | Which pointer button opens a `Popover`                                    |
| `open_delay(ms)`          | How long the pointer must rest on a `HoverCard` trigger; default 600      |
| `close_delay(ms)`         | How long a `HoverCard` waits before closing; default 300                  |
| `overlay_closable(value)` | Whether pressing outside an open `Popover` closes it                      |

### Dock commands

What an element a dock's chrome drew _does_. A cached chrome description has no
script event-handler lifetime, so it may not register one — a command carries
no script value instead, and base does the work. Every one takes the object its
handler was given as its first argument, and they belong on a `div`, an
`h_flex` or a `v_flex`.

| Method                         | On    | What it does                                      |
| ------------------------------ | ----- | ------------------------------------------------- |
| `select_tab(group, index)`     | click | Displays that tab                                 |
| `close_panel(group, panel_id)` | click | Closes the panel, if its group allows it          |
| `toggle_zoom(group)`           | click | Zooms the group in, or back out                   |
| `drag_tab(group, index)`       | drag  | Makes the element the drag source for that tab    |
| `drop_tab(group, index?)`      | drop  | Accepts a dragged panel here; no index appends    |
| `toggle_dock(dock)`            | click | Opens or closes the dock                          |
| `resize_dock(dock)`            | drag  | Drags the dock's edge; base clamps every position |
| `move_tile(tile)`              | drag  | Moves the tile around its canvas                  |
| `resize_tile(tile, side)`      | drag  | Drags one edge or corner                          |
| `raise_tile(tile)`             | press | Brings the tile above the others                  |
| `toggle_tile_zoom(tile)`       | click | Zooms the tile to fill its dock                   |
| `close_tile(tile)`             | click | Closes the tile                                   |

### Dock chrome

Six handlers, all optional, and legal only on a `dock_area(...)`. Each is first
called from inside GPUI's layout pass and given base's resolved state. Its
description is cached until that state or handler changes.

| Method                         | Draws                                                                 |
| ------------------------------ | --------------------------------------------------------------------- |
| `tab_bar(handler)`             | The tab bar above a group's displayed panel                           |
| `empty_group(handler)`         | What a group with no displayed panel shows                            |
| `drop_indicator(handler)`      | Where a dragged panel would land                                      |
| `dock(handler)`                | One dock's frame around its content; place `dock_content()` inside it |
| `tile_drag_bar(handler)`       | The strip a tile is dragged by                                        |
| `tile_resize_handles(handler)` | A tile's resize affordances                                           |

### Motion

| Method                         | What it does                                               |
| ------------------------------ | ---------------------------------------------------------- |
| `transition(property, policy)` | Animates later target changes entirely in native GPUI code |
| `spring(property, policy?)`    | Springs them instead                                       |

The property is one of `"opacity"`, `"width"`, `"height"`, `"left"`, `"top"`, and the frames never enter JavaScript.

### Style templates

Each takes a function that receives a detached element to collect styles on; its return value is ignored, so a chain and a block body both work.

| Method                       | What it styles                                                           |
| ---------------------------- | ------------------------------------------------------------------------ |
| `hover(declare)`             | While the pointer is over the element                                    |
| `active(declare)`            | While the element is pressed                                             |
| `focus(declare)`             | While the element has focus                                              |
| `range_style(declare)`       | The filled part of a `SliderIndicator` — how it looks, never where it is |
| `cell_style(declare)`        | Every cell of an `OtpInput`; without it there is nothing on screen       |
| `cell_active_style(declare)` | Layered on top, for the cell the next digit lands in                     |
| `caret_style(declare)`       | The blinking mark in that cell while it is empty                         |

### Style methods

Everything else on an element is a style. There are two families, and they never overlap:

- **Methods that take an argument**, bound by hand: the size, padding, margin, position, flex, border, radius and paint families. Which length type each accepts follows its Rust signature, so `.p("auto")` is a type error for the same reason it throws at run time.
- **No-argument methods**, generated from GPUI's reflection table: `flex_col`, `items_center`, `gap_2`, `rounded_md`, `text_sm`, `size_full`, `truncate` and the rest. The generated declarations are the inventory for the GPUI version in your build.

Both are covered in [Styling](./styling.md), along with the length and color grammars and the tokens the palette defines.

## HostModule registrations

A module the host registered in Rust is imported by name, like any other module:

```js
import { quotes } from "market";
```

It is not part of any built-in module. The generated declarations carry one `declare module` per registered module, so both the module name and every export name are checked. See [HostModule](./host-module.md).
