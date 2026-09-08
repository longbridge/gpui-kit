---
title: Rating
description: A simple interactive star rating component.
---

# Rating

A star rating component that allows users to select a rating value. Supports different sizes, custom colors, disabled state, and click handlers.

## Import

```rust
use gpui_kit::component::rating::Rating;
```

## Usage

### Basic Rating

```rust
Rating::new("my-rating")
    .value(3)
    .max(5)
    .on_click(|value, _, _| {
        println!("Rating changed to: {}", value);
    })
```

### Controlled Rating

```rust
struct MyView {
    rating: usize,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Rating::new("rating")
            .value(self.rating)
            .max(5)
            .on_click(cx.listener(|view, value: &usize, _, cx| {
                view.rating = *value;
                cx.notify();
            }))
    }
}
```

### Different Sizes

The Rating component supports the [Sizable] trait for different sizes.

```rust
Rating::new("rating").xsmall().value(3).max(5)
Rating::new("rating").small().value(3).max(5)
Rating::new("rating").value(3).max(5) // default (Medium)
Rating::new("rating").large().value(3).max(5)
```

### Custom Color

By default, the rating uses the theme's `yellow` color. You can customize it with the `color` method.

```rust
Rating::new("rating")
    .value(4)
    .max(5)
    .color(cx.theme().green)
```

### Disabled State

```rust
Rating::new("rating")
    .value(2)
    .max(5)
    .disabled(true)
```

### Custom Maximum

The default maximum is 5 stars, but you can set a different maximum value.

```rust
Rating::new("rating")
    .value(7)
    .max(10)
```

### Click Behavior

The rating component has special click behavior:

- Clicking on a star that's already filled will reduce the rating by 1
- Clicking on an unfilled star will set the rating to that star's value

The `on_click` callback receives the new rating value as `&usize`.

```rust
Rating::new("rating")
    .value(3)
    .max(5)
    .on_click(|new_value, _, _| {
        println!("New rating: {}", new_value);
    })
```

## API Reference

- [Rating]

### Methods

- `new(id: impl Into<ElementId>)` - Create a new Rating component
- `with_size(size: impl Into<Size>)` - Set the star size (implements [Sizable])
- `value(value: usize)` - Set the initial rating value (0..=max)
- `max(max: usize)` - Set the maximum number of stars (default: 5)
- `color(color: impl Into<Hsla>)` - Set the active color (default: theme yellow)
- `disabled(disabled: bool)` - Disable interaction (implements [Disableable])
- `on_click(handler: Fn(&usize, &mut Window, &mut App))` - Set click handler

## Examples

### Read-only Display

```rust
Rating::new("rating")
    .value(4)
    .max(5)
    .disabled(true)
```

### Interactive Rating with State

```rust
struct ProductView {
    user_rating: usize,
}

impl Render for ProductView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                Rating::new("product-rating")
                    .value(self.user_rating)
                    .max(5)
                    .on_click(cx.listener(|view, value: &usize, _, cx| {
                        view.user_rating = *value;
                        // Save rating to backend, etc.
                        cx.notify();
                    }))
            )
            .child(format!("Your rating: {}/5", self.user_rating))
    }
}
```

### Large Rating with Custom Color

```rust
Rating::new("rating")
    .large()
    .value(5)
    .max(5)
    .color(cx.theme().orange)
```

[Rating]: https://docs.rs/gpui-component/latest/gpui_component/rating/struct.Rating.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html
[Disableable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Disableable.html
