---
title: Icon
description: Display SVG icons with various sizes, colors, and transformations.
---

# Icon

A flexible icon component that renders SVG icons from asset paths or in-memory bytes, with customizable size, color, and transformations. The built-in Lucide icons use the assets bundle; custom SVG bytes can be supplied directly with `Icon::data`.

Before you start, please make sure you have read: [Icons & Assets](../assets.md) to understand how use SVG in GPUI & GPUI Component application.

## Import

```rust
use gpui_kit::component::{Icon, IconName};
```

## Usage

### Basic Icon

```rust
// Using IconName enum directly
IconName::Heart

// Or creating an Icon explicitly
Icon::new(IconName::Heart)
```

### Icon with Custom Size

```rust
// Predefined sizes
Icon::new(IconName::Search).xsmall()   // size_3()
Icon::new(IconName::Search).small()    // size_3p5()
Icon::new(IconName::Search).medium()   // size_4() (default)
Icon::new(IconName::Search).large()    // size_6()

// Custom pixel size
Icon::new(IconName::Search).with_size(px(20.))
```

### Icon with Custom Color

```rust
// Using theme colors
Icon::new(IconName::Heart)
    .text_color(cx.theme().red)

// Using custom colors
Icon::new(IconName::Star)
    .text_color(gpui_kit::red())
```

### Rotated Icons

```rust
use gpui_kit::{Transformation, radians};

// Rotate by radians
Icon::new(IconName::ArrowUp)
    .rotate(radians(std::f32::consts::FRAC_PI_2))

// Transform with custom transformation
Icon::new(IconName::ChevronRight)
    .transform(Transformation::rotate(radians(std::f32::consts::PI)))
```

### Custom SVG Path

```rust
// Using a custom SVG file from assets
Icon::new(Icon::empty())
    .path("icons/my-custom-icon.svg")
```

### SVG Bytes

Use `data(&[u8])` to supply SVG bytes without registering an `AssetSource` path:

```rust
use gpui_kit::component::{Icon, button::Button, menu::PopupMenuItem};

let icon = Icon::default().data(include_bytes!("search.svg"));

Button::new("search").icon(icon.clone()).label("Search");
PopupMenuItem::new("Search").icon(icon);
```

`data` copies its input into shared storage, so the input need not be `'static`.
Cloning an `Icon` shares those bytes and preserves its style and transformation.
Both direct rendering and `Icon::view(cx)` retain the data source. GPUI's renderer
may copy the bytes again; this API does not promise zero-copy rendering.

The last source builder wins, including when the new source is empty:

```rust
let bytes = include_bytes!("search.svg");
Icon::default().path("icons/old.svg").data(bytes); // Uses SVG bytes
Icon::default().data(bytes).path("icons/search.svg"); // Uses the asset path
```

Bytes go through the same SVG renderer as path-based icons. They retain component
sizing, foreground colors, and button loading behavior. Use `loading_icon` to
choose a custom loading symbol:

```rust
Button::new("search")
    .icon(Icon::default().data(include_bytes!("search.svg")))
    .loading_icon(Icon::default().data(include_bytes!("loader.svg")))
    .loading(true)
    .label("Searching")
```

`NativeMenu::menu_with_icon` also accepts data-backed icons. Native menus keep
their existing platform sizing and tinting rules. Other path-based icons used
by your application or components still need an asset source.

### Custom Icon Types with SVG Bytes

An icon crate can export individual types that implement `From<T> for Icon`:

```rust
use gpui_kit::component::{Icon, button::Button};

pub struct Search;

impl From<Search> for Icon {
    fn from(_: Search) -> Self {
        Icon::default().data(include_bytes!("search.svg"))
    }
}

Button::new("search").icon(Search);
```

Existing `IconNamed` implementations continue to provide asset paths. A
data-backed type uses the conversion above without also implementing `IconNamed`.
Binary-size savings depend on which resources are referenced and on build settings.

## Available Icons

The `IconName` enum provides access to a curated set of icons. Here are some commonly used ones:

### Navigation

- `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`
- `ChevronUp`, `ChevronDown`, `ChevronLeft`, `ChevronRight`
- `ChevronsUpDown`

### Actions

- `Check`, `Close`, `Plus`, `Minus`
- `Copy`, `Delete`, `Search`, `Replace`
- `Maximize`, `Minimize`, `WindowRestore`

### Files & Folders

- `File`, `Folder`, `FolderOpen`, `FolderClosed`
- `BookOpen`, `Inbox`

### UI Elements

- `Menu`, `Settings`, `Settings2`, `Ellipsis`, `EllipsisVertical`
- `Eye`, `EyeOff`, `Bell`, `Info`

### Social & External

- `GitHub`, `Globe`, `ExternalLink`
- `Heart`, `HeartOff`, `Star`, `StarOff`
- `ThumbsUp`, `ThumbsDown`

### Status & Alerts

- `CircleCheck`, `CircleX`, `TriangleAlert`
- `Loader`, `LoaderCircle`

### Panels & Layout

- `PanelLeft`, `PanelRight`, `PanelBottom`
- `PanelLeftOpen`, `PanelRightOpen`, `PanelBottomOpen`
- `LayoutDashboard`, `Frame`

### Users & Profile

- `User`, `CircleUser`, `Bot`

### Other

- `Calendar`, `Map`, `Palette`, `Inspector`
- `Sun`, `Moon`, `Building2`

## Icon Sizes

The Icon component supports several predefined sizes:

| Size        | Method                | CSS Class    | Pixels |
| ----------- | --------------------- | ------------ | ------ |
| Extra Small | `.xsmall()`           | `size_3()`   | 12px   |
| Small       | `.small()`            | `size_3p5()` | 14px   |
| Medium      | `.medium()` (default) | `size_4()`   | 16px   |
| Large       | `.large()`            | `size_6()`   | 24px   |
| Custom      | `.with_size(px(n))`   | -            | n px   |

## Build you own `IconName`.

You can define your own `IconName` to have more specific icons for your application. We have `IconNamed` trait for you to implement for your.

```rust
use gpui_kit::component::IconNamed;

pub enum IconName {
    Encounters,
    Monsters,
    Spells,
}

impl IconNamed for IconName {
    fn path(self) -> gpui_kit::SharedString {
        match self {
            IconName::Encounters => "icons/encounters.svg",
            IconName::Monsters => "icons/monsters.svg",
            IconName::Spells => "icons/spells.svg",
        }
        .into()
    }
}

// This allows for the following interactions (works with anything that has the `.icon(icon)` method.
Button::new("my-button").icon(IconName::Spells);
Icon::new(IconName::Monsters);
```

If you want to directly `render` a custom `IconName` you must implement the `RenderOnce` trait and derive `IntoElement` on the `IconName`.

```rust
impl RenderOnce for IconName {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Icon::empty().path(self.path())
    }
}

// Now you can use it directly in your element tree:
div()
    .child(IconName::Monsters)
```

## Examples

### Icon in Button

```rust
use gpui_kit::component::button::Button;

Button::new("like-btn")
    .icon(
        Icon::new(IconName::Heart)
            .text_color(cx.theme().red)
            .large()
    )
    .label("Like")
```

### Animated Loading Icon

```rust
Icon::new(IconName::LoaderCircle)
    .text_color(cx.theme().muted_foreground)
    .medium()
    // Add rotation animation in your render logic
```

### Status Icons

```rust
// Success
Icon::new(IconName::CircleCheck)
    .text_color(cx.theme().green)

// Error
Icon::new(IconName::CircleX)
    .text_color(cx.theme().red)

// Warning
Icon::new(IconName::TriangleAlert)
    .text_color(cx.theme().yellow)
```

### Navigation Icons

```rust
// Back button
Icon::new(IconName::ArrowLeft)
    .medium()
    .text_color(cx.theme().foreground)

// Dropdown indicator
Icon::new(IconName::ChevronDown)
    .small()
    .text_color(cx.theme().muted_foreground)
```

### Custom Icon from Assets

```rust
// Using a custom SVG file
Icon::empty()
    .path("icons/my-brand-logo.svg")
    .large()
    .text_color(cx.theme().primary)
```

## Notes

- Icons are rendered as SVG elements and support full CSS styling
- The default size matches the current text size if no explicit size is set
- Icons are flex-shrink-0 by default to prevent unwanted shrinking in flex layouts
- All icon paths are relative to the assets bundle root
- Icons from Lucide.dev are designed to work well at 16px and scale nicely to other sizes
