---
title: GPUI Base
description: The unstyled behavior and infrastructure foundation of GPUI Kit, the Rust desktop framework.
order: 1
---

# GPUI Base

`gpui-base` is the unstyled foundation of GPUI Kit, the Rust desktop application framework. It provides interaction behavior, controlled state, focus management, accessibility semantics, animation, virtual lists, and theme tokens while leaving layout and visual design to your application.

## Choose the right layer

| Use | When |
| --- | --- |
| `gpui-base` | You are building a design system and want to own every visual choice. |
| `gpui-component` | You want a complete set of styled, ready-to-use desktop components. |

The dependency points one way: `gpui-component` builds on `gpui-base`. Applications can use either layer directly.

## Principles

- **Behavior is built in.** Controls provide consistent pointer, keyboard, focus, and state behavior.
- **Presentation is yours.** Compose GPUI style methods and children without fighting default visuals.
- **Parts stay composable.** Primitives expose their meaningful subparts instead of hiding markup behind a monolith.
- **State stays explicit.** Controlled inputs report changes and your view owns the resulting state.

## Start building

Follow [Getting started](./getting-started.md), render selectable Markdown and HTML with [TextView](./text-view.md), learn how to add [window-level text selection](./text-selection.md) to custom renderers, then explore the [primitive catalog](./primitives/index.md). Each page includes Rust snippets and a live WASM example backed by the same example crate that can run natively.

Three systems are larger than a primitive and have pages of their own. [Motion](./motion.md) provides typed transitions, springs, keyframes, presence, and sequencing. [Virtual List](./virtual-list.md) renders lists of any length by drawing only what is on screen, with per-item sizes rather than a uniform row height. [Dock](./dock.md) is a full workspace shell — nested splits, tab groups and edge docks — whose layout is pure data you can build and serialize without a window, and whose every pixel comes from renderer traits you implement. [History](./history.md) covers two smaller, deliberately distinct structures: `History` is a root/current/back/forward navigation trail, while `UndoHistory` records grouped undo and redo transactions. [Plot](./plot.md) is the unstyled foundation for charts: scales, shapes, axes, the element that turns a `Plot` into a child, and hover tracking, with colors and timing left to your design system.

## Community Design Systems

Explore third-party design systems built on GPUI Base. These projects are maintained independently; check their documentation for supported versions and project status.

- **[gpui-kit-material](https://github.com/mizorewww/gpui-kit-material)** — Material 3 and M3 Expressive components, theme tokens, and motion built on GPUI Kit and `gpui-base`. [API documentation](https://docs.rs/gpui-kit-material).
