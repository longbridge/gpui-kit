---
title: Slider
description: A control that allows the user to select values from a range using a draggable thumb.
---

# Slider

A slider component for selecting numeric values within a specified range. Supports both single value and range selection modes, horizontal and vertical orientations, custom styling, and step intervals.

## Import

```rust
use gpui_kit::component::slider::{Slider, SliderState, SliderEvent, SliderValue};
```

## Usage

### Basic Slider

```rust
let slider_state = cx.new(|_| {
    SliderState::new()
        .min(0.0)
        .max(100.0)
        .default_value(50.0)
        .step(1.0)
});

Slider::new(&slider_state)
```

### Slider with Event Handling

```rust
struct MyView {
    slider_state: Entity<SliderState>,
    current_value: f32,
}

impl MyView {
    fn new(cx: &mut Context<Self>) -> Self {
        let slider_state = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .default_value(25.0)
                .step(5.0)
        });

        let subscription = cx.subscribe(&slider_state, |this, _, event: &SliderEvent, cx| {
            match event {
                SliderEvent::Change(value) => {
                    this.current_value = value.start();
                    cx.notify();
                }
            }
        });

        Self {
            slider_state,
            current_value: 25.0,
        }
    }
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(Slider::new(&self.slider_state))
            .child(format!("Value: {}", self.current_value))
    }
}
```

### Range Slider

```rust
let range_slider = cx.new(|_| {
    SliderState::new()
        .min(0.0)
        .max(100.0)
        .default_value(20.0..80.0)  // Range from 20 to 80
        .step(1.0)
});

Slider::new(&range_slider)
```

### Vertical Slider

```rust
Slider::new(&slider_state)
    .vertical()
    .h(px(200.))
```

### Custom Step Intervals

```rust
// Integer steps
let integer_slider = cx.new(|_| {
    SliderState::new()
        .min(0.0)
        .max(10.0)
        .step(1.0)
        .default_value(5.0)
});

// Decimal steps
let decimal_slider = cx.new(|_| {
    SliderState::new()
        .min(0.0)
        .max(1.0)
        .step(0.01)
        .default_value(0.5)
});
```

### Min/Max Configuration

```rust
// Temperature slider
let temp_slider = cx.new(|_| {
    SliderState::new()
        .min(-10.0)
        .max(40.0)
        .default_value(20.0)
        .step(0.5)
});

// Percentage slider
let percent_slider = cx.new(|_| {
    SliderState::new()
        .min(0.0)
        .max(100.0)
        .default_value(75.0)
        .step(5.0)
});
```

### Disabled State

```rust
Slider::new(&slider_state)
    .disabled(true)
```

### Custom Styling

```rust
Slider::new(&slider_state)
    .bg(cx.theme().success)
    .text_color(cx.theme().success_foreground)
    .rounded(px(4.))
```

### Scale

There have 2 types of scale for the slider:

- `Linear` (default)
- `Logarithmic`

The logarithmic scale is useful when the range of values is large and you want to give more precision to smaller values.

```rust
let log_slider = cx.new(|_| {
    SliderState::new()
        .min(1.0) // min must be greater than 0 for log scale
        .max(1000.0)
        .default_value(10.0)
        .step(1.0)
        .scale(SliderScale::Logarithmic)
});
```

In this case:

:::info
$$ v = min \times (max/min)^p $$

The value `v` is calculated using the formula above, where `p` is the slider percentage (0 to 1).
:::

- If slider at 25%, value will be approximately `5.62`.
- If slider at 50%, value will be approximately `31.62`.
- If slider at 75%, value will be approximately `177.83`.
- If slider at 100%, value will be `1000.0`.

#### Conversions

```rust
// From f32
let single_value: SliderValue = 42.0.into();

// From tuple
let range_value: SliderValue = (10.0, 90.0).into();

// From Range
let range_value: SliderValue = (10.0..90.0).into();
```

### SliderEvent

| Event                  | Description                                                     |
| ---------------------- | --------------------------------------------------------------- |
| `Change(SliderValue)`  | Emitted continuously while the slider value is being changed    |
| `Release(SliderValue)` | Emitted once when the user releases the slider after interaction |

### Styling

The slider component implements `Styled` trait and supports:

- Background color for track and thumb
- Text color for thumb
- Border radius
- Size customization

## Examples

### Color Picker

```rust
struct ColorPicker {
    hue_slider: Entity<SliderState>,
    saturation_slider: Entity<SliderState>,
    lightness_slider: Entity<SliderState>,
    alpha_slider: Entity<SliderState>,
    current_color: Hsla,
}

impl ColorPicker {
    fn new(cx: &mut Context<Self>) -> Self {
        let hue_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(1.0)
                .step(0.01)
                .default_value(0.5)
        });

        let saturation_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(1.0)
                .step(0.01)
                .default_value(1.0)
        });

        // Subscribe to all sliders to update color
        let subscriptions = [&hue_slider, &saturation_slider /* ... */]
            .iter()
            .map(|slider| {
                cx.subscribe(slider, |this, _, event: &SliderEvent, cx| {
                    match event {
                        SliderEvent::Change(_) => {
                            this.update_color(cx);
                        }
                    }
                })
            })
            .collect::<Vec<_>>();

        Self {
            hue_slider,
            saturation_slider,
            // ... other fields
        }
    }

    fn update_color(&mut self, cx: &mut Context<Self>) {
        let h = self.hue_slider.read(cx).value().start();
        let s = self.saturation_slider.read(cx).value().start();
        // ... calculate color
        self.current_color = hsla(h, s, l, a);
        cx.notify();
    }
}

impl Render for ColorPicker {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_4()
            .child(
                v_flex()
                    .gap_2()
                    .child("Hue")
                    .child(Slider::new(&self.hue_slider).vertical().h(px(120.)))
            )
            .child(
                v_flex()
                    .gap_2()
                    .child("Saturation")
                    .child(Slider::new(&self.saturation_slider).vertical().h(px(120.)))
            )
            // ... other sliders
    }
}
```

### Volume Control

```rust
struct VolumeControl {
    volume_slider: Entity<SliderState>,
    volume: f32,
}

impl VolumeControl {
    fn new(cx: &mut Context<Self>) -> Self {
        let volume_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value(50.0)
        });

        let subscription = cx.subscribe(&volume_slider, |this, _, event: &SliderEvent, cx| {
            match event {
                SliderEvent::Change(value) => {
                    this.volume = value.start();
                    this.apply_volume_change();
                    cx.notify();
                }
            }
        });

        Self {
            volume_slider,
            volume: 50.0,
        }
    }

    fn apply_volume_change(&self) {
        // Apply volume change to audio system
        println!("Volume changed to: {}%", self.volume);
    }
}

impl Render for VolumeControl {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .items_center()
            .gap_3()
            .child("🔊")
            .child(Slider::new(&self.volume_slider).flex_1())
            .child(format!("{}%", self.volume as i32))
    }
}
```

### Price Range Filter

```rust
struct PriceFilter {
    price_range: Entity<SliderState>,
    min_price: f32,
    max_price: f32,
}

impl PriceFilter {
    fn new(cx: &mut Context<Self>) -> Self {
        let price_range = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(1000.0)
                .step(10.0)
                .default_value(100.0..500.0)  // Range slider
        });

        let subscription = cx.subscribe(&price_range, |this, _, event: &SliderEvent, cx| {
            match event {
                SliderEvent::Change(value) => {
                    this.min_price = value.start();
                    this.max_price = value.end();
                    this.filter_products();
                    cx.notify();
                }
            }
        });

        Self {
            price_range,
            min_price: 100.0,
            max_price: 500.0,
        }
    }

    fn filter_products(&self) {
        println!("Filtering products: ${} - ${}", self.min_price, self.max_price);
    }
}

impl Render for PriceFilter {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child("Price Range")
            .child(Slider::new(&self.price_range))
            .child(format!("${} - ${}", self.min_price as i32, self.max_price as i32))
    }
}
```

### Temperature Slider with Custom Styling

```rust
struct TemperatureControl {
    temp_slider: Entity<SliderState>,
    temperature: f32,
}

impl Render for TemperatureControl {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let temp_color = if self.temperature < 10.0 {
            cx.theme().info  // Cold - blue
        } else if self.temperature > 25.0 {
            cx.theme().destructive  // Hot - red
        } else {
            cx.theme().success  // Comfortable - green
        };

        v_flex()
            .gap_3()
            .child("Temperature Control")
            .child(
                Slider::new(&self.temp_slider)
                    .bg(temp_color)
                    .text_color(cx.theme().background)
                    .rounded(px(8.))
            )
            .child(format!("{}°C", self.temperature as i32))
    }
}
```

## Keyboard Shortcuts

| Key           | Action                         |
| ------------- | ------------------------------ |
| `←` / `↓`     | Decrease value by step         |
| `→` / `↑`     | Increase value by step         |
| `Page Down`   | Decrease by larger amount      |
| `Page Up`     | Increase by larger amount      |
| `Home`        | Set to minimum value           |
| `End`         | Set to maximum value           |
| `Tab`         | Move focus to next element     |
| `Shift + Tab` | Move focus to previous element |
