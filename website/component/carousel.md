---
title: Carousel
description: A composable carousel for browsing related content.
---

# Carousel

Carousel displays one or more related items in a snapping viewport. It supports horizontal and vertical layouts, keyboard navigation, pointer and trackpad gestures, looping, and controlled selection.

## Import

```rust
use gpui_kit::Axis;
use gpui_kit::component::carousel::{
    Carousel, CarouselContent, CarouselEvent, CarouselItem, CarouselNext,
    CarouselPagination, CarouselPaginationItem, CarouselPrevious, CarouselState,
};
```

## Usage

Create one `CarouselState` for the content and pass it to every carousel part.

```rust
let state = cx.new(|_| CarouselState::new(3));

Carousel::new("projects-carousel", &state)
    .child(
        CarouselContent::new(&state)
            .child(CarouselItem::new("project-1", 0, &state).child("Project one"))
            .child(CarouselItem::new("project-2", 1, &state).child("Project two"))
            .child(CarouselItem::new("project-3", 2, &state).child("Project three")),
    )
    .child(CarouselPrevious::new(&state))
    .child(CarouselNext::new(&state))
```

`CarouselContent` owns the viewport and snap layout. `CarouselItem` identifies one logical slide. The previous and next controls automatically become disabled at the corresponding boundary.

Keep the state's item count equal to the number of direct `CarouselItem` children. A state and its scroll handle belong to one viewport.

## Composition

Build a Carousel from one content viewport, its items, and optional controls:

```text
Carousel
├── CarouselContent
│   ├── CarouselItem
│   └── CarouselItem
├── CarouselPrevious
└── CarouselNext
```

Constrain the Carousel with `.w_full().max_w_96()` on its root, or style `CarouselContent` when the viewport itself needs a custom width or height. Use `track_style` only for inner-track adjustments such as spacing. The root lays out its flow children as a column with a 16px gap, so a `CarouselPagination` placed after the content keeps its distance; restyle the root for another arrangement.

## Multiple items

`CarouselItem` implements `Styled`. Set its flex basis to show more than one item in the viewport, and pair a negative leading margin on the content track with matching leading padding on every item to tune the gap between them. This is the same paired spacing model shadcn/ui uses.

```rust
use gpui_kit::{ParentElement as _, StyleRefinement, Styled as _, relative};

let state = cx.new(|_| CarouselState::new(6));

CarouselContent::new(&state)
    .track_style(StyleRefinement::default().ml_neg_1())
    .children((0..6).map(|index| {
        CarouselItem::new(("project", index), index, &state)
            .flex_basis(relative(1. / 3.))
            .pl_1()
            .child(format!("Project {}", index + 1))
    }))
```

The flex basis controls item geometry; it is separate from the semantic `Size` used by buttons and other controls.

Horizontal carousels default to `.ml_neg_4()` on the content track and `.pl_4()` on items. Vertical carousels use the corresponding `.mt_neg_4()` and `.pt_4()` pair. Override both sides with the same spacing scale so the first item stays aligned with the viewport while the visual gap changes.

## Orientation

Use `with_axis` when creating the state:

```rust
let state = cx.new(|_| {
    CarouselState::new(3).with_axis(Axis::Vertical)
});
```

Horizontal carousels use Left and Right. Vertical carousels use Up and Down.
Give vertical `CarouselContent` an explicit height so each full-height item has a viewport to snap within.

The Carousel root is a tab stop, so keyboard navigation also works when optional controls are omitted. Home and End select the first and last items. Clicking inside the carousel or on one of its controls focuses it for keyboard navigation without drawing the focus ring; the ring appears only when focus arrives from the keyboard.

## Looping

Enable looping to wrap navigation from the last item to the first:

```rust
let state = cx.new(|_| CarouselState::new(5).with_looping(true));
```

## Controlled selection

`CarouselState` can be controlled by application state. Use `with_selected_index` for the initial selection and `set_selected_index` for programmatic changes.

```rust
let state = cx.new(|_| CarouselState::new(4).with_selected_index(1));

state.update(cx, |state, cx| {
    state.set_selected_index(3, cx);
});
```

Subscribe to `CarouselEvent::Change` when the application needs to mirror the selected item:

```rust
cx.subscribe(&state, |this, _, event: &CarouselEvent, cx| {
    let CarouselEvent::Change(index) = event;
    this.selected_index = *index;
    cx.notify();
});
```

## Events

| Event | Description |
| --- | --- |
| `CarouselEvent::Change(index)` | Emitted when user navigation selects a new item. |

Keyboard navigation and previous/next controls use the same state transition and emit the same event. Pointer and trackpad gestures select the nearest snap point when the gesture ends. A mouse-wheel notch moves one item, and a gesture that begins at an edge scrolls the surrounding container instead.

## Pagination indicators

Pagination is optional and does not impose one visual treatment. Compose indicators with `CarouselPaginationItem`, then style or fill each item as needed:

```rust
CarouselPagination::new().children((0..3).map(|index| {
    CarouselPaginationItem::new(("project-page", index), index, &state)
        .child((index + 1).to_string())
}))
```

`CarouselPaginationItem` uses the same selection transition as pointer, keyboard, and previous/next navigation.

## Control size

`CarouselPrevious`, `CarouselNext`, and `CarouselPaginationItem` implement `Sizable`. Apply the same semantic size to the controls when they should scale together:

```rust
use gpui_kit::component::{Sizable as _, Size};

CarouselPrevious::new(&state).with_size(Size::Large);
CarouselNext::new(&state).with_size(Size::Large);
```

Previous and next controls default to `Size::Medium`. Pagination items default to `Size::XSmall`.

## Custom controls

`CarouselPrevious` and `CarouselNext` implement `ParentElement` and `Styled`. Without children they display the direction-appropriate chevron. Add a child to replace that visible content while preserving automatic navigation and disabled boundary states. `accessibility_label` also replaces the control's tooltip.

```rust
use gpui_kit::ParentElement as _;

CarouselPrevious::new(&state)
    .accessibility_label("Previous project")
    .child("Back");

CarouselNext::new(&state)
    .accessibility_label("Next project")
    .child("Forward");
```

For a completely custom control, omit the corresponding Carousel part and compose any control with the public state API:

```rust
use gpui_kit::ParentElement as _;
use gpui_kit::component::{Disableable as _, button::Button};

let previous_state = state.clone();
let previous_disabled = !state.read(cx).has_previous();

Button::new("projects-previous")
    .label("Back")
    .disabled(previous_disabled)
    .on_click(move |_, _, cx| {
        previous_state.update(cx, |state, cx| {
            state.select_previous(cx);
        });
    })
```

## Accessibility

The carousel exposes a labelled region and each item reports its position within the set. Use `accessibility_label` when the default "Carousel" label does not describe the content.

Carousel animation follows the application's reduced-motion preference.
