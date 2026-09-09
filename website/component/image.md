---
title: Image
description: A flexible image display component with loading states, fallbacks, and responsive sizing options.
---

# Image

The Image component provides a robust way to display images with comprehensive fallback handling, loading states, and responsive sizing. Built on GPUI's native image support, it handles various image sources including URLs, local files, and SVG graphics with proper error handling and accessibility features.

## Import

```rust
use gpui_kit::{img, ImageSource, ObjectFit};
use gpui_kit::component::{v_flex, h_flex, div, Icon, IconName};
```

## Usage

### Basic Image

```rust
// Simple image from URL
img("https://example.com/image.jpg")

// Local image file
img("assets/logo.png")

// SVG image
img("icons/star.svg")
```

### Image with Sizing

```rust
// Fixed dimensions
img("https://example.com/photo.jpg")
    .w(px(300.))
    .h(px(200.))

// Responsive width with aspect ratio
img("https://example.com/banner.jpg")
    .w(relative(1.))  // Full width
    .max_w(px(800.))
    .h(px(400.))

// Square image
img("https://example.com/avatar.jpg")
    .size(px(100.))  // 100x100px
```

### Object Fit Options

Control how images are scaled and positioned within their containers:

```rust
// Cover - scales to fill container, may crop
img("https://example.com/photo.jpg")
    .w(px(300.))
    .h(px(200.))
    .object_fit(ObjectFit::Cover)

// Contain - scales to fit within container, preserves aspect ratio
img("https://example.com/photo.jpg")
    .w(px(300.))
    .h(px(200.))
    .object_fit(ObjectFit::Contain)

// Fill - stretches to fill container, may distort
img("https://example.com/photo.jpg")
    .w(px(300.))
    .h(px(200.))
    .object_fit(ObjectFit::Fill)

// Scale Down - acts like contain, but never scales up
img("https://example.com/photo.jpg")
    .w(px(300.))
    .h(px(200.))
    .object_fit(ObjectFit::ScaleDown)

// None - original size, may overflow or be smaller than container
img("https://example.com/photo.jpg")
    .w(px(300.))
    .h(px(200.))
    .object_fit(ObjectFit::None)
```

### Image with Fallback Handling

```rust
// Basic fallback with placeholder
fn image_with_fallback(src: &str, alt_text: &str) -> impl IntoElement {
    div()
        .w(px(300.))
        .h(px(200.))
        .bg(cx.theme().surface)
        .border_1()
        .border_color(cx.theme().border)
        .rounded(px(8.))
        .overflow_hidden()
        .child(
            img(src)
                .w_full()
                .h_full()
                .object_fit(ObjectFit::Cover)
                // Add error handling in practice
        )
}

// Fallback with icon placeholder
fn image_with_icon_fallback(src: &str) -> impl IntoElement {
    div()
        .size(px(200.))
        .bg(cx.theme().surface)
        .border_1()
        .border_color(cx.theme().border)
        .rounded(px(8.))
        .flex()
        .items_center()
        .justify_center()
        .child(
            img(src)
                .size_full()
                .object_fit(ObjectFit::Cover)
                // On error, show icon:
                // Icon::new(IconName::Image)
                //     .size(px(48.))
                //     .text_color(cx.theme().muted_foreground)
        )
}
```

### Loading States

```rust
// Image with loading skeleton
fn image_with_loading(src: &str, is_loading: bool) -> impl IntoElement {
    div()
        .w(px(400.))
        .h(px(300.))
        .rounded(px(8.))
        .overflow_hidden()
        .map(|this| {
            if is_loading {
                this.bg(cx.theme().muted)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child("Loading...")
            } else {
                this.child(
                    img(src)
                        .w_full()
                        .h_full()
                        .object_fit(ObjectFit::Cover)
                )
            }
        })
}

// Progressive loading with placeholder
fn progressive_image(src: &str, placeholder_src: &str) -> impl IntoElement {
    div()
        .relative()
        .w(px(400.))
        .h(px(300.))
        .rounded(px(8.))
        .overflow_hidden()
        .child(
            // Low-quality placeholder
            img(placeholder_src)
                .absolute()
                .inset_0()
                .w_full()
                .h_full()
                .object_fit(ObjectFit::Cover)
                .opacity(0.5)
        )
        .child(
            // High-quality image
            img(src)
                .absolute()
                .inset_0()
                .w_full()
                .h_full()
                .object_fit(ObjectFit::Cover)
        )
}
```

### Responsive Images

```rust
// Responsive grid images
fn responsive_image_grid() -> impl IntoElement {
    div()
        .grid()
        .grid_cols(3)
        .gap_4()
        .child(
            img("https://example.com/photo1.jpg")
                .w_full()
                .aspect_ratio(1.0)  // Square aspect ratio
                .object_fit(ObjectFit::Cover)
                .rounded(px(8.))
        )
        .child(
            img("https://example.com/photo2.jpg")
                .w_full()
                .aspect_ratio(1.0)
                .object_fit(ObjectFit::Cover)
                .rounded(px(8.))
        )
        .child(
            img("https://example.com/photo3.jpg")
                .w_full()
                .aspect_ratio(1.0)
                .object_fit(ObjectFit::Cover)
                .rounded(px(8.))
        )
}

// Hero image with text overlay
fn hero_image() -> impl IntoElement {
    div()
        .relative()
        .w_full()
        .h(px(500.))
        .rounded(px(12.))
        .overflow_hidden()
        .child(
            img("https://example.com/hero-image.jpg")
                .absolute()
                .inset_0()
                .w_full()
                .h_full()
                .object_fit(ObjectFit::Cover)
        )
        .child(
            div()
                .absolute()
                .inset_0()
                .bg(rgba(0, 0, 0, 0.4))  // Dark overlay
                .flex()
                .items_center()
                .justify_center()
                .child(
                    v_flex()
                        .items_center()
                        .gap_4()
                        .child("Hero Title")
                        .child("Subtitle text here")
                )
        )
}
```

### Image Gallery

```rust
// Simple image gallery
fn image_gallery(images: Vec<&str>) -> impl IntoElement {
    v_flex()
        .gap_6()
        .child(
            // Main image
            div()
                .w_full()
                .h(px(400.))
                .rounded(px(12.))
                .overflow_hidden()
                .child(
                    img(images[0])
                        .w_full()
                        .h_full()
                        .object_fit(ObjectFit::Cover)
                )
        )
        .child(
            // Thumbnail row
            h_flex()
                .gap_3()
                .children(
                    images.iter().map(|src| {
                        div()
                            .size(px(80.))
                            .rounded(px(6.))
                            .overflow_hidden()
                            .border_2()
                            .border_color(cx.theme().border)
                            .cursor_pointer()
                            .hover(|this| this.border_color(cx.theme().primary))
                            .child(
                                img(*src)
                                    .size_full()
                                    .object_fit(ObjectFit::Cover)
                            )
                    })
                )
        )
}
```

### SVG Images

```rust
// SVG icon with custom styling
img("assets/icons/logo.svg")
    .size(px(64.))
    .text_color(cx.theme().primary)  // SVG color

// Inline SVG handling
img("data:image/svg+xml;base64,...")
    .w(px(32.))
    .h(px(32.))

// SVG with animation-friendly setup
img("assets/spinner.svg")
    .size(px(24.))
    .text_color(cx.theme().primary)
    // Add rotation animation in practice
```

## API Reference

### Core Image Function

| Function      | Description                           |
| ------------- | ------------------------------------- |
| `img(source)` | Create image element from ImageSource |

### Image Sources (ImageSource)

| Type        | Description            | Example                           |
| ----------- | ---------------------- | --------------------------------- |
| String/&str | URL or file path       | `"https://example.com/image.jpg"` |
| SharedUri   | Shared URI reference   | `SharedUri::from("file://path")`  |
| Local Path  | Local file system path | `"assets/logo.png"`               |
| Data URI    | Base64 encoded image   | `"data:image/png;base64,..."`     |

### Sizing Methods

| Method          | Description               |
| --------------- | ------------------------- |
| `w(length)`     | Set width                 |
| `h(length)`     | Set height                |
| `size(length)`  | Set both width and height |
| `w_full()`      | Full width of container   |
| `h_full()`      | Full height of container  |
| `size_full()`   | Full size of container    |
| `max_w(length)` | Maximum width             |
| `max_h(length)` | Maximum height            |
| `min_w(length)` | Minimum width             |
| `min_h(length)` | Minimum height            |

### Object Fit Options

| Value                  | Description                       |
| ---------------------- | --------------------------------- |
| `ObjectFit::Cover`     | Scale to fill container, may crop |
| `ObjectFit::Contain`   | Scale to fit within container     |
| `ObjectFit::Fill`      | Stretch to fill container         |
| `ObjectFit::ScaleDown` | Like contain, but never scale up  |
| `ObjectFit::None`      | Original size                     |

### Styling Methods

| Method                | Description             |
| --------------------- | ----------------------- |
| `rounded(radius)`     | Border radius           |
| `border_1()`          | 1px border              |
| `border_color(color)` | Border color            |
| `opacity(value)`      | Image opacity (0.0-1.0) |
| `shadow_sm()`         | Small shadow            |
| `shadow_lg()`         | Large shadow            |

## Examples

### Product Image Card

```rust
use gpui_kit::component::{v_flex, div, Icon, IconName};

fn product_card(image_src: &str, title: &str, price: &str) -> impl IntoElement {
    v_flex()
        .gap_3()
        .p_4()
        .bg(cx.theme().card)
        .rounded(px(12.))
        .shadow_sm()
        .child(
            div()
                .relative()
                .w_full()
                .h(px(200.))
                .rounded(px(8.))
                .overflow_hidden()
                .bg(cx.theme().muted)
                .child(
                    img(image_src)
                        .w_full()
                        .h_full()
                        .object_fit(ObjectFit::Cover)
                )
                .child(
                    // Wishlist button
                    div()
                        .absolute()
                        .top_2()
                        .right_2()
                        .size(px(32.))
                        .bg(rgba(255, 255, 255, 0.9))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .child(Icon::new(IconName::Heart).size(px(16.)))
                )
        )
        .child(title)
        .child(price)
}
```

### Avatar with Image

```rust
fn custom_avatar(src: &str, name: &str, size: f32) -> impl IntoElement {
    div()
        .size(px(size))
        .rounded_full()
        .overflow_hidden()
        .border_2()
        .border_color(cx.theme().background)
        .shadow_sm()
        .child(
            img(src)
                .size_full()
                .object_fit(ObjectFit::Cover)
        )
}
```

### Image Comparison Slider

```rust
fn image_comparison(before_src: &str, after_src: &str) -> impl IntoElement {
    div()
        .relative()
        .w_full()
        .h(px(400.))
        .rounded(px(12.))
        .overflow_hidden()
        .child(
            // Before image
            img(before_src)
                .absolute()
                .inset_0()
                .w_full()
                .h_full()
                .object_fit(ObjectFit::Cover)
        )
        .child(
            // After image with clip
            div()
                .absolute()
                .top_0()
                .left_0()
                .w(relative(0.5))  // Show 50% initially
                .h_full()
                .overflow_hidden()
                .child(
                    img(after_src)
                        .w(px(800.))  // Full width of container
                        .h_full()
                        .object_fit(ObjectFit::Cover)
                )
        )
        .child(
            // Separator line
            div()
                .absolute()
                .top_0()
                .left(relative(0.5))
                .w(px(2.))
                .h_full()
                .bg(cx.theme().primary)
        )
}
```

### Error Handling Pattern

```rust
enum ImageState {
    Loading,
    Loaded,
    Error,
}

fn robust_image(src: &str, state: ImageState) -> impl IntoElement {
    div()
        .w(px(300.))
        .h(px(200.))
        .bg(cx.theme().muted)
        .rounded(px(8.))
        .border_1()
        .border_color(cx.theme().border)
        .flex()
        .items_center()
        .justify_center()
        .map(|this| {
            match state {
                ImageState::Loading => {
                    this.child(
                        v_flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(IconName::Loader2).size(px(24.)))
                            .child("Loading...")
                    )
                }
                ImageState::Loaded => {
                    this.p_0()
                        .overflow_hidden()
                        .child(
                            img(src)
                                .w_full()
                                .h_full()
                                .object_fit(ObjectFit::Cover)
                        )
                }
                ImageState::Error => {
                    this.child(
                        v_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(IconName::ImageOff)
                                    .size(px(32.))
                                    .text_color(cx.theme().muted_foreground)
                            )
                            .child("Failed to load image")
                    )
                }
            }
        })
}
```

## Best Practices

### Image Optimization

- Use appropriate image dimensions for display size
- Compress images without sacrificing quality
- Consider using modern image formats (WebP, AVIF)
- Implement responsive images for different screen sizes

### Error Handling

- Always provide meaningful fallbacks for failed image loads
- Use skeleton loading states to maintain layout stability
- Implement retry mechanisms for temporary network failures
- Provide user feedback for permanent load failures

### Performance

- Use lazy loading for images not immediately visible
- Implement proper caching strategies
- Consider using placeholder images during loading
- Optimize image sizes for their display context

### User Experience

- Maintain consistent aspect ratios in image grids
- Provide smooth loading transitions
- Use appropriate object-fit values for content type
- Consider providing zoom functionality for detailed images

## Implementation Notes

### GPUI Integration

- Built on GPUI's native image rendering capabilities
- Supports all GPUI ImageSource types automatically
- Inherits GPUI's styling and layout system
- Compatible with GPUI's animation and interaction systems

### SVG Support

- Full support for SVG graphics with proper scaling
- SVG images can be styled with text colors for theming
- Vector graphics maintain sharpness at all sizes
- Supports both external SVG files and inline data URIs

### Memory Management

- GPUI handles image caching and memory management automatically
- Large images are efficiently managed by the graphics backend
- No manual memory cleanup required for image components

### Cross-Platform Compatibility

- Consistent behavior across Windows, macOS, and Linux
- Native image format support varies by platform
- Uses platform-optimized rendering where available
