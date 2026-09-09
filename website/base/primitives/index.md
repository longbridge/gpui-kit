---
title: Primitives
description: The complete catalog of user-facing gpui-base primitives.
order: 4
---

# Primitives

GPUI Base primitives provide behavior without prescribing presentation. Each page documents the public import and the smallest useful composition. The live example above the page is built from `crates/base/examples` and can also run as a native GPUI application.

## Primitive catalog

- [Accordion](./accordion.md) — A disclosure group composed from independently styleable header, trigger, and panel parts.
- [Alert Dialog](./alert-dialog.md) — A modal confirmation surface for actions that need an explicit decision.
- [Avatar](./avatar.md) — An image with composable fallback content for a person or entity.
- [Button](./button.md) — An unstyled, accessible pressable with semantic state and keyboard activation.
- [Calendar](./calendar.md) — A state-driven date grid with selection matchers and custom item rendering.
- [Checkbox](./checkbox.md) — A controlled tri-state check control with a separately styled indicator.
- [Collapsible](./collapsible.md) — A composable region that shows or hides content without prescribing its trigger styling.
- [Color Picker](./color-picker.md) — State and interaction foundations for selecting colors in a custom picker UI.
- [Combobox](./combobox.md) — A text input paired with keyboard-navigable suggestions and selection behavior.
- [Date Picker](./date-picker.md) — A focus-aware date input that composes calendar behavior with a popup.
- [Dialog](./dialog.md) — A composable modal surface with focus management, backdrop, title, and close parts.
- [Hover Card](./hover-card.md) — A delayed floating card associated with a pointer or keyboard trigger.
- [Input](./input.md) — A single-line text input with selection, masking, validation, and number stepping.
- [Textarea](./textarea.md) — A multi-line text field with fixed rows, wrapping, and auto-grow behavior.
- [Editor](./editor.md) — A source-code editor foundation with highlighting, gutter, folding, decorations, and LSP hooks.
- [Link](./link.md) — An accessible link-like control with application-defined styling.
- [Nav Stack](./nav-stack.md) — A navigation stack of views with push, pop, forward, and replace, and an animatable transition lifecycle.
- [Number Input](./number-input.md) — A numeric input with reusable increment, decrement, and step behavior.
- [OTP Input](./otp-input.md) — A multi-cell one-time-code input driven by a shared text state.
- [Pagination](./pagination.md) — A controlled page navigator with explicit current and total page state.
- [Popover](./popover.md) — An anchored floating surface with controlled or internally managed open state.
- [Popup](./popup.md) — A low-level trigger and anchored floating-content host.
- [Progress](./progress.md) — Composable track and indicator parts for reporting task completion.
- [Radio](./radio.md) — A controlled single-choice item with selectable and disabled semantics.
- [Radio Group](./radio-group.md) — Groups radio items and provides keyboard navigation for a single selection.
- [Resizable](./resizable.md) — Panel groups and resize handles for user-adjustable split layouts.
- [Scrollbar](./scrollbar.md) — An unstyled scrollbar connected to GPUI scroll or uniform-list handles.
- [Select](./select.md) — A button-like selection control backed by an anchored, keyboard-navigable popup.
- [Sheet](./sheet.md) — A modal surface that enters from an edge while managing dismissal and focus.
- [Slider](./slider.md) — A state-driven range input with independently styleable track, indicator, and thumb.
- [Switch](./switch.md) — A controlled on/off control with separately styleable track and thumb.
- [Table](./table.md) — Semantic table primitives for composing headers, bodies, rows, and cells.
- [Tabs](./tabs.md) — A tab list and accessible tab controls with controlled selection.
- [Toast](./toast.md) — A managed, animated stack of temporary status messages.
- [Toggle](./toggle.md) — A controlled two-state pressable for persistent choices such as formatting.
- [Toggle Group](./toggle-group.md) — Coordinates a set of toggle controls as a single- or multiple-selection group.
- [Tooltip](./tooltip.md) — A delayed, positioned description associated with a trigger element.
- [Tree](./tree.md) — A virtualized hierarchical list with explicit expansion and selection state.
