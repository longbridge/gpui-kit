---
title: Skeleton
description: Use to show a placeholder while content is loading.
---

# Skeleton

The Skeleton component displays animated placeholder content while actual content is loading. It provides visual feedback to users that content is being loaded and helps maintain layout structure during loading states.

## Import

```rust
use gpui_kit::component::skeleton::Skeleton;
```

## Usage

### Basic Skeleton

```rust
Skeleton::new()
```

### Text Line Skeleton

```rust
// Single line of text
Skeleton::new()
    .w(px(250.))
    .h_4()
    .rounded_md()

// Multiple text lines
v_flex()
    .gap_2()
    .child(Skeleton::new().w(px(250.)).h_4().rounded_md())
    .child(Skeleton::new().w(px(200.)).h_4().rounded_md())
    .child(Skeleton::new().w(px(180.)).h_4().rounded_md())
```

### Circle Skeleton

```rust
// Avatar placeholder
Skeleton::new()
    .size_12()
    .rounded_full()

// Profile picture placeholder
Skeleton::new()
    .w(px(64.))
    .h(px(64.))
    .rounded_full()
```

### Rectangle Skeleton

```rust
// Card image placeholder
Skeleton::new()
    .w(px(250.))
    .h(px(125.))
    .rounded_md()

// Button placeholder
Skeleton::new()
    .w(px(120.))
    .h(px(40.))
    .rounded_md()
```

### Different Shapes

```rust
// Text content
Skeleton::new().w(px(200.)).h_4().rounded_sm()

// Square image
Skeleton::new().size_20().rounded_md()

// Wide banner
Skeleton::new().w_full().h(px(200.)).rounded_lg()

// Small icon
Skeleton::new().size_6().rounded_md()
```

### Secondary Variant

```rust
// Use secondary color (more subtle)
Skeleton::new()
    .secondary()
    .w(px(200.))
    .h_4()
    .rounded_md()
```

## Animation

The Skeleton component includes a built-in pulse animation that:

- Runs continuously with a 2-second duration
- Uses a bounce easing function with ease-in-out
- Animates opacity from 100% to 50% and back
- Automatically repeats to indicate loading state

The animation cannot be disabled as it's essential for indicating loading state.

## Sizes

The Skeleton component doesn't have predefined size variants. Instead, use gpui's sizing utilities:

```rust
// Height utilities
Skeleton::new().h_3()    // 12px height
Skeleton::new().h_4()    // 16px height
Skeleton::new().h_5()    // 20px height
Skeleton::new().h_6()    // 24px height

// Width utilities
Skeleton::new().w(px(100.))   // 100px width
Skeleton::new().w(px(200.))   // 200px width
Skeleton::new().w_full()      // Full width
Skeleton::new().w_1_2()       // 50% width

// Square sizes
Skeleton::new().size_4()      // 16x16px
Skeleton::new().size_8()      // 32x32px
Skeleton::new().size_12()     // 48x48px
Skeleton::new().size_16()     // 64x64px
```

## Examples

### Loading Profile Card

```rust
v_flex()
    .gap_4()
    .p_4()
    .border_1()
    .border_color(cx.theme().border)
    .rounded(cx.theme().radius_lg)
    .child(
        h_flex()
            .gap_3()
            .items_center()
            .child(Skeleton::new().size_12().rounded_full()) // Avatar
            .child(
                v_flex()
                    .gap_2()
                    .child(Skeleton::new().w(px(120.)).h_4().rounded_md()) // Name
                    .child(Skeleton::new().w(px(100.)).h_3().rounded_md()) // Email
            )
    )
    .child(
        v_flex()
            .gap_2()
            .child(Skeleton::new().w_full().h_4().rounded_md()) // Bio line 1
            .child(Skeleton::new().w(px(200.)).h_4().rounded_md()) // Bio line 2
    )
```

### Loading Article List

```rust
v_flex()
    .gap_6()
    .children((0..3).map(|_| {
        h_flex()
            .gap_4()
            .child(Skeleton::new().w(px(120.)).h(px(80.)).rounded_md()) // Thumbnail
            .child(
                v_flex()
                    .gap_2()
                    .flex_1()
                    .child(Skeleton::new().w_full().h_5().rounded_md()) // Title
                    .child(Skeleton::new().w(px(300.)).h_4().rounded_md()) // Excerpt line 1
                    .child(Skeleton::new().w(px(250.)).h_4().rounded_md()) // Excerpt line 2
                    .child(Skeleton::new().w(px(100.)).h_3().rounded_md()) // Date
            )
    }))
```

### Loading Table Rows

```rust
v_flex()
    .gap_2()
    .children((0..5).map(|_| {
        h_flex()
            .gap_4()
            .p_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(Skeleton::new().size_8().rounded_full()) // Status indicator
            .child(Skeleton::new().w(px(150.)).h_4().rounded_md()) // Name
            .child(Skeleton::new().w(px(200.)).h_4().rounded_md()) // Email
            .child(Skeleton::new().w(px(80.)).h_4().rounded_md()) // Role
            .child(Skeleton::new().w(px(60.)).h_4().rounded_md()) // Actions
    }))
```

### Loading Button States

```rust
h_flex()
    .gap_3()
    .child(Skeleton::new().w(px(80.)).h(px(36.)).rounded_md()) // Primary button
    .child(Skeleton::new().w(px(70.)).h(px(36.)).rounded_md()) // Secondary button
    .child(Skeleton::new().size_9().rounded_md()) // Icon button
```

### Loading Form Fields

```rust
v_flex()
    .gap_4()
    .child(
        v_flex()
            .gap_1()
            .child(Skeleton::new().w(px(60.)).h_4().rounded_md()) // Label
            .child(Skeleton::new().w_full().h(px(40.)).rounded_md()) // Input
    )
    .child(
        v_flex()
            .gap_1()
            .child(Skeleton::new().w(px(80.)).h_4().rounded_md()) // Label
            .child(Skeleton::new().w_full().h(px(120.)).rounded_md()) // Textarea
    )
```

### Conditional Loading

```rust
if loading {
    Skeleton::new().w(px(200.)).h_4().rounded_md()
} else {
    div().child("Actual content here")
}
```

## Theming

The Skeleton component uses the theme's `skeleton` color, which defaults to the `secondary` color if not specified. You can customize it in your theme:

```json
{
  "skeleton.background": "#e2e8f0"
}
```

The `secondary(true)` variant applies 50% opacity to the skeleton color for more subtle loading indicators.
