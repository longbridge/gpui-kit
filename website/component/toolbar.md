---
title: Toolbar
description: A transparent, sizable container for commands in headers, tab panels, and custom surfaces.
---

# Toolbar

Toolbar is a transparent horizontal container for actions — buttons, separators, and short labels — used inside panel headers, tab strips, and custom command surfaces. The surrounding container owns its background and border.

The design mirrors the toolbars found in native UI frameworks: macOS `NSToolbar` and Windows `ToolStrip`.

## Import

```rust
use gpui_kit::component::toolbar::Toolbar;
```

## Composition

Use `child` for `Sizable` controls. Toolbar applies its final size to those controls even when `.small()` or another size method appears after them in the builder chain. Use `content` for strings, separators, flexible spacers, and custom layout that should keep its own dimensions. Items render in source order.

- For a **command**, pass a `Button`; Toolbar applies its size and forces the quiet `ghost + compact` presentation. Chain `label`, `icon`, `tooltip`, `on_click`, etc. as needed.
- For an **icon-only button**, always add a `tooltip`; it is the accessible name as well.
- For a **separator**, use `content` with `Separator::vertical()` and an explicit height.
- For a **non-interactive label**, use one of the content methods with a plain string.

## Usage

### Commands

```rust
Toolbar::new("toolbar")
    .child(
        Button::new("new")
            .icon(IconName::Plus)
            .label("New")
            .on_click(|_, window, cx| { /* ... */ }),
    )
    .content(Separator::vertical().h_5())
    .child(
        Button::new("undo")
            .icon(IconName::Undo2)
            .tooltip("Undo")
            .on_click(|_, window, cx| { /* ... */ }),
    )
    .content(div().flex_1())
    .child(
        Button::new("more")
            .icon(IconName::Ellipsis)
            .tooltip("More options")
            .on_click(|_, window, cx| { /* ... */ }),
    )
```

### Sizes

Use `Sizable` to change the container height, spacing, text size, and hosted controls together: `xsmall` (28px), `small` (32px, default), and `medium` (48px). Builder order does not matter.

```rust
Toolbar::new("toolbar")
    .child(Button::new("new").icon(IconName::Plus).label("New"))
    .child(Button::new("find").icon(IconName::Search).tooltip("Find"))
    .small()
```

### Labels and custom elements

```rust
Toolbar::new("toolbar")
    .content("Dashboard")
    .content(Separator::vertical().h_5())
    .content(
        h_flex()
            .items_center()
            .gap_1()
            .child(Icon::new(IconName::CircleCheck).xsmall())
            .child("Saved"),
    )
    .content(div().flex_1())
    .child(Button::new("settings").icon(IconName::Settings2).tooltip("Settings"))
```

### Custom styling

`Toolbar` is transparent and borderless by default. It implements `Styled`, so a standalone command surface can add its own appearance.

```rust
Toolbar::new("toolbar")
    .bg(cx.theme().secondary)
    .border_color(cx.theme().border)
    .content("Ready")
```

## Groups

Wrap related controls in `ToolbarGroup` to give them an accessible name, so assistive technology reads a run of controls as one unit. The group implements `Sizable`, and a parent Toolbar propagates its final size through the group to every control:

```rust
use gpui_kit::component::toolbar::ToolbarGroup;

Toolbar::new("document-toolbar")
    .child(
        ToolbarGroup::new("history-group")
            .label("History")
            .gap_2() // match the bar's own item spacing
            .child(Button::new("undo").icon(IconName::Undo2).tooltip("Undo"))
            .child(Button::new("redo").icon(IconName::Redo2).tooltip("Redo")),
    )
```

Unlike Base UI's `Toolbar.Group`, a group cannot disable its children: that API propagates through React context into Base UI's own button primitives, which has no equivalent for arbitrary GPUI children. Disabling the hosted controls is the caller's job.

Separators and other non-sized elements use `content`. Sized controls use `child` so the toolbar can propagate its size.

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
| `new()`           | Create a new, empty toolbar (small size)             |
| `child(c)` / `children(cs)` | Add sized control(s) in source order      |
| `content(c)` / `contents(cs)` | Add non-sized content in source order    |
| `with_size(size)` | Set the bar size — `xsmall`, `small`, or `medium`       |
| `disabled(value)` | Disable roving navigation; the owner also disables hosted controls |

Control methods require `Sizable + IntoElement`; content methods accept general elements. `Toolbar` also implements `Styled` and `Sizable`.

## Notes

- Use `content(div().flex_1())` when later items need to align to the trailing edge.
- Keep the primary command visible; move low-frequency actions into a dropdown or overflow menu rather than hiding them behind hover.
- Toolbar has no default background or border; its host surface supplies them.
