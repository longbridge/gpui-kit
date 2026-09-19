---
title: Toolbar
description: A themed bar of commands, usually placed at the top of a window or pane.
---

# Toolbar

Toolbar is a horizontal bar that hosts a row of actions — buttons, separators, and short labels — usually placed at the top of a window, pane, or section. It pairs with `TitleBar` above it and `StatusBar` at the bottom.

The design mirrors the toolbars found in native UI frameworks: macOS `NSToolbar` and Windows `ToolStrip`.

## Import

```rust
use gpui_kit::component::toolbar::Toolbar;
```

## Regions

Pass any `impl IntoElement` — a string, an `Icon`, a `Button`, a custom layout, etc. `left` and `right` pin items to each end; `child` / `children` add to the middle, whose alignment follows the pinned ends — centered with both `left` and `right`, end-aligned with only `left`, start-aligned otherwise (only `right`, or neither, like a plain bar). Call a method multiple times to add more.

- For a **command**, pass a ghost `Button` sized to match the toolbar — `Button::new(id).ghost()` — and chain `label`, `icon`, `tooltip`, `on_click`, etc.
- For an **icon-only button**, always add a `tooltip`; it is the accessible name as well.
- For a **separator**, pass `Separator::vertical()` with an explicit height (for example `.h_5()`).
- For a **non-interactive label**, pass a plain string — it inherits the bar's text style and has no hover.

## Usage

### Commands

```rust
Toolbar::new("toolbar")
    .left(
        Button::new("new").ghost()
            .icon(IconName::Plus)
            .label("New")
            .on_click(|_, window, cx| { /* ... */ }),
    )
    .left(Separator::vertical().h_5())
    .left(
        Button::new("undo").ghost()
            .icon(IconName::Undo2)
            .tooltip("Undo")
            .on_click(|_, window, cx| { /* ... */ }),
    )
    .right(
        Button::new("more").ghost()
            .icon(IconName::Ellipsis)
            .tooltip("More options")
            .on_click(|_, window, cx| { /* ... */ }),
    )
```

### Sizes

Use `Sizable` to change the bar height, spacing, and text size together: `xsmall` (28px), `small` (32px), `medium` (40px, default), and `large` (48px). Size the hosted buttons to match.

```rust
Toolbar::new("toolbar").small()
    .left(Button::new("new").ghost().small().icon(IconName::Plus).label("New"))
    .right(Button::new("find").ghost().small().icon(IconName::Search).tooltip("Find"))
```

### Labels and custom elements

```rust
Toolbar::new("toolbar")
    .left("Dashboard")
    .left(Separator::vertical().h_5())
    .child(
        h_flex()
            .items_center()
            .gap_1()
            .child(Icon::new(IconName::CircleCheck).xsmall())
            .child("Saved"),
    )
    .right(Button::new("settings").ghost().icon(IconName::Settings2).tooltip("Settings"))
```

### Custom styling

`Toolbar` implements `Styled`, so any style method overrides the defaults.

```rust
Toolbar::new("toolbar")
    .bg(cx.theme().secondary)
    .border_color(cx.theme().border)
    .left("Ready")
```

## Groups

Wrap related controls in `ToolbarGroup` (re-exported from `gpui_base`) to give them an accessible name, so assistive technology reads a run of controls as one unit:

```rust
use gpui_kit::component::toolbar::ToolbarGroup;

Toolbar::new("document-toolbar")
    .left(
        ToolbarGroup::new("history-group")
            .label("History")
            .gap_2() // match the bar's own item spacing
            .child(Button::new("undo").ghost().icon(IconName::Undo2).tooltip("Undo"))
            .child(Button::new("redo").ghost().icon(IconName::Redo2).tooltip("Redo")),
    )
```

Unlike Base UI's `Toolbar.Group`, a group cannot disable its children: that API propagates through React context into Base UI's own button primitives, which has no equivalent for arbitrary GPUI children. Disabling the hosted controls is the caller's job.

`Separator` and `Link` need no toolbar-specific wrappers — pass `Separator::vertical().h_5()` and the existing `Link` component directly.

## Keyboard

The toolbar exposes `Toolbar` semantics to assistive technology and owns roving keyboard focus, matching the ARIA toolbar pattern and Base UI's `Toolbar`:

| Key | Behavior |
| --- | --- |
| `←` / `→` | Move focus to the previous / next control (horizontal toolbar) |
| `↑` / `↓` | Move focus to the previous / next control (vertical toolbar) |
| `Tab` | Enter or leave the toolbar; the bar itself is not a tab stop |

Focus wraps around at the ends. Hosted inputs keep their own arrow-key caret behavior; place inputs at the trailing end of the bar. This behavior comes from the unstyled `gpui_base::Toolbar` primitive, so applications building custom toolbars on the base layer get the same contract.

## API Reference

### Toolbar

| Method            | Description                                          |
| ----------------- | ---------------------------------------------------- |
| `new()`           | Create a new, empty toolbar (medium size)            |
| `left(child)`     | Append an element to the left region (call to add more) |
| `right(child)`    | Append an element to the right region                |
| `child(c)` / `children(cs)` | Add element(s) to the middle region        |
| `with_size(size)` | Set the bar size — `xsmall`, `small`, `medium`, `large` |

Each region method takes `impl IntoElement`. `Toolbar` also implements `Styled` and `Sizable`, so style methods (`bg`, `border_color`, `py`, etc.) can override the defaults.

## Notes

- The middle (via `child` / `children`) is centered with both `left` and `right`, end-aligned with only `left`, and start-aligned otherwise (only `right`, or neither — like a plain bar).
- Keep the primary command visible; move low-frequency actions into a dropdown or overflow menu rather than hiding them behind hover.
- Colors come from the `toolbar` (background) and `toolbar_border` theme tokens, which fall back to the title bar colors.
