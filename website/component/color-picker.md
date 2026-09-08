---
title: ColorPicker
description: A comprehensive color selection interface with support for multiple color formats, presets, and alpha channel.
---

# ColorPicker

A versatile color picker component that provides an intuitive interface for color selection. Features include color palettes, hex input, featured colors, and support for various color formats including RGB, HSL, and hex values with alpha channel support.

## Import

```rust
use gpui_kit::component::color_picker::{ColorPicker, ColorPickerState, ColorPickerEvent};
```

## Usage

### Basic Color Picker

```rust
use gpui_kit::{Entity, Window, Context};

// Create color picker state
let color_picker = cx.new(|cx|
    ColorPickerState::new(window, cx)
        .default_value(cx.theme().primary)
);

// Create the color picker component
ColorPicker::new(&color_picker)
```

### With Event Handling

```rust
use gpui_kit::{Subscription, Entity};

let color_picker = cx.new(|cx| ColorPickerState::new(window, cx));

let _subscription = cx.subscribe(&color_picker, |this, _, ev, _| match ev {
    ColorPickerEvent::Change(color) => {
        if let Some(color) = color {
            println!("Selected color: {}", color.to_hex());
            // Handle color change
        }
    }
});

ColorPicker::new(&color_picker)
```

### Setting Default Color

```rust
use gpui_kit::Hsla;

let color_picker = cx.new(|cx|
    ColorPickerState::new(window, cx)
        .default_value(cx.theme().blue) // Set default color
);
```

### Different Sizes

```rust
// Small color picker
ColorPicker::new(&color_picker).small()

// Medium color picker (default)
ColorPicker::new(&color_picker)

// Large color picker
ColorPicker::new(&color_picker).large()

// Extra small color picker
ColorPicker::new(&color_picker).xsmall()
```

### With Custom Featured Colors

```rust
use gpui_kit::Hsla;

let featured_colors = vec![
    cx.theme().red,
    cx.theme().green,
    cx.theme().blue,
    cx.theme().yellow,
    // Add your custom colors
];

ColorPicker::new(&color_picker)
    .featured_colors(featured_colors)
```

### With Icon Instead of Color Square

```rust
use gpui_kit::component::IconName;

ColorPicker::new(&color_picker)
    .icon(IconName::Palette)
```

### With Label

```rust
ColorPicker::new(&color_picker)
    .label("Background Color")
```

### Custom Anchor Position

```rust
use gpui_kit::Anchor;

ColorPicker::new(&color_picker)
    .anchor(Anchor::TopRight) // Dropdown opens to top-right
```

## Color Selection Interface

### Color Palettes

The color picker includes predefined color palettes organized by color family:

- **Stone**: Neutral grays and stone colors
- **Red**: Red color variations from light to dark
- **Orange**: Orange color variations
- **Yellow**: Yellow color variations
- **Green**: Green color variations
- **Cyan**: Cyan color variations
- **Blue**: Blue color variations
- **Purple**: Purple color variations
- **Pink**: Pink color variations

Each palette provides multiple shades and tints of the base color, allowing for precise color selection.

### Featured Colors Section

A customizable section at the top of the picker that displays frequently used or brand colors. If not specified, defaults to theme colors:

- Primary colors from the current theme
- Light variants of theme colors
- Essential UI colors (red, blue, green, yellow, cyan, magenta)

### Hex Input Field

A text input field that allows direct entry of hex color values:

- Supports standard 6-digit hex format (#RRGGBB)
- Real-time validation and preview
- Updates color picker state automatically
- Press Enter to confirm selection

## Color Formats

### RGB (Red, Green, Blue)

Colors are internally represented using GPUI's `Hsla` format but can be converted to RGB:

```rust
let color = cx.theme().blue;
// Access RGB components through Hsla methods
```

### HSL (Hue, Saturation, Lightness)

Native format used by the color picker:

```rust
use gpui_kit::Hsla;

// Create HSL color
let color = Hsla::hsl(240.0, 100.0, 50.0); // Blue color

// Access components
let hue = color.h;
let saturation = color.s;
let lightness = color.l;
```

### Hex Format

Standard web hex format with # prefix:

```rust
// Convert color to hex
let hex_string = color.to_hex(); // Returns "#3366FF"

// Parse hex string to color
if let Ok(color) = Hsla::parse_hex("#3366FF") {
    // Use parsed color
}
```

## Alpha Channel

Full alpha channel support for transparency:

```rust
use gpui_kit::hsla;

// Create color with alpha
let semi_transparent = hsla(0.5, 0.8, 0.6, 0.7); // 70% opacity

// Modify existing color opacity
let transparent_blue = cx.theme().blue.opacity(0.5);
```

The color picker preserves alpha values when selecting colors and allows modification through the alpha component of HSLA colors.

## API Reference

- [ColorPicker]
- [ColorPickerState]
- [ColorPickerEvent]

## Examples

### Color Theme Editor

```rust
struct ThemeEditor {
    primary_color: Entity<ColorPickerState>,
    secondary_color: Entity<ColorPickerState>,
    accent_color: Entity<ColorPickerState>,
}

impl ThemeEditor {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let primary_color = cx.new(|cx|
            ColorPickerState::new(window, cx)
                .default_value(cx.theme().primary)
        );

        let secondary_color = cx.new(|cx|
            ColorPickerState::new(window, cx)
                .default_value(cx.theme().secondary)
        );

        let accent_color = cx.new(|cx|
            ColorPickerState::new(window, cx)
                .default_value(cx.theme().accent)
        );

        Self {
            primary_color,
            secondary_color,
            accent_color,
        }
    }

    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child("Primary Color:")
                    .child(ColorPicker::new(&self.primary_color))
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child("Secondary Color:")
                    .child(ColorPicker::new(&self.secondary_color))
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child("Accent Color:")
                    .child(ColorPicker::new(&self.accent_color))
            )
    }
}
```

### Brand Color Selector

```rust
use gpui_kit::component::{Sizable as _};

let brand_colors = vec![
    Hsla::parse_hex("#FF6B6B").unwrap(), // Brand Red
    Hsla::parse_hex("#4ECDC4").unwrap(), // Brand Teal
    Hsla::parse_hex("#45B7D1").unwrap(), // Brand Blue
    Hsla::parse_hex("#96CEB4").unwrap(), // Brand Green
    Hsla::parse_hex("#FFEAA7").unwrap(), // Brand Yellow
];

ColorPicker::new(&color_picker)
    .featured_colors(brand_colors)
    .label("Brand Color")
    .large()
```

### Toolbar Color Picker

```rust
use gpui_kit::component::{Sizable as _, IconName);

ColorPicker::new(&text_color_picker)
    .icon(IconName::Type)
    .small()
    .anchor(Anchor::BottomLeft)
```

### Color Palette Builder

```rust
struct ColorPalette {
    colors: Vec<Entity<ColorPickerState>>,
}

impl ColorPalette {
    fn add_color(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let color_picker = cx.new(|cx| ColorPickerState::new(window, cx));

        // Subscribe to color changes
        cx.subscribe(&color_picker, |this, _, ev, _| match ev {
            ColorPickerEvent::Change(color) => {
                if let Some(color) = color {
                    this.update_palette_preview();
                }
            }
        });

        self.colors.push(color_picker);
        cx.notify();
    }

    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .children(
                self.colors.iter().map(|color_picker| {
                    ColorPicker::new(color_picker).small()
                })
            )
            .child(
                Button::new("add-color")
                    .icon(IconName::Plus)
                    .ghost()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.add_color(window, cx);
                    }))
            )
    }
}
```

### With Color Validation

```rust
let color_picker = cx.new(|cx| ColorPickerState::new(window, cx));

let _subscription = cx.subscribe(&color_picker, |this, _, ev, _| match ev {
    ColorPickerEvent::Change(color) => {
        if let Some(color) = color {
            // Validate color accessibility
            if this.validate_contrast(color) {
                this.apply_color(color);
            } else {
                this.show_contrast_warning();
            }
        }
    }
});
```

[ColorPicker]: https://docs.rs/gpui-component/latest/gpui_component/color_picker/struct.ColorPicker.html
[ColorPickerState]: https://docs.rs/gpui-component/latest/gpui_component/color_picker/struct.ColorPickerState.html
[ColorPickerEvent]: https://docs.rs/gpui-component/latest/gpui_component/color_picker/enum.ColorPickerEvent.html
