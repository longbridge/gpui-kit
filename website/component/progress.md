---
title: Progress
description: Displays an indicator showing the completion progress of a task, typically displayed as a progress bar or circular indicator.
---

# Progress

Progress components visually represent the completion percentage of a task. The library provides two variants:

- **[Progress](#progress)** - A linear horizontal progress bar
- **[ProgressCircle](#progresscircle)** - A circular progress indicator

Both components feature smooth transition animations when the value changes, a loading (indeterminate) animation mode, customizable colors, and automatic styling that adapts to the current theme.

## Progress

```rust
use gpui_kit::component::progress::Progress;
```

### Usage

```rust
Progress::new("my-progress")
    .value(75.0) // 75% complete
```

### Different Progress Values

```rust
Progress::new("progress-0").value(0.0)
Progress::new("progress-25").value(25.0)
Progress::new("progress-75").value(75.0)
Progress::new("progress-100").value(100.0)
```

### Loading State

Use `.loading(true)` to show an indeterminate animation when the actual progress is unknown. The `value` is ignored while loading is active.

```rust
// Indeterminate loading animation
Progress::new("loading").loading(true)

// Toggle between loading and determinate
Progress::new("my-progress")
    .loading(self.is_loading)
    .value(self.progress)
```

### Sizes

`Progress` implements the `Sizable` trait:

```rust
Progress::new("xs").value(50.0).xsmall()  // 4px height
Progress::new("sm").value(50.0).small()   // 6px height
Progress::new("md").value(50.0)           // 8px height (default)
Progress::new("lg").value(50.0).large()   // 10px height
```

### Custom Style

The component implements the `Styled` trait, allowing custom height, border radius, color, and border:

```rust
Progress::new("custom")
    .value(32.0)
    .h(px(16.))
    .rounded(px(2.))
    .color(cx.theme().green_light)
    .border_2()
    .border_color(cx.theme().green)
```

### Dynamic Progress Updates

```rust
struct MyView {
    value: f32,
    is_loading: bool,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("toggle-loading")
                            .label("Loading")
                            .selected(self.is_loading)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.is_loading = !this.is_loading;
                                cx.notify();
                            })),
                    )
                    .child(Button::new("inc").icon(IconName::Plus).on_click(
                        cx.listener(|this, _, _, _| {
                            this.value = (this.value + 10.).min(100.);
                        }),
                    )),
            )
            .child(
                Progress::new("progress")
                    .value(self.value)
                    .loading(self.is_loading),
            )
    }
}
```

### API Reference

| Method | Type | Description |
|---|---|---|
| `new(id)` | `ElementId` | Create a new progress bar |
| `value(v)` | `f32` | Set progress value (0–100), clamped automatically |
| `loading(v)` | `bool` | Enable indeterminate loading animation; ignores `value` when `true` |
| `color(c)` | `impl Into<Hsla>` | Override the fill color (defaults to `theme.progress_bar`) |
| `xsmall()` / `small()` / `large()` | — | Set predefined height via `Sizable` |
| `Styled` trait methods | — | Custom height, border radius, border, etc. |

## ProgressCircle

A circular progress indicator that displays progress as an arc. Ideal for compact spaces, inline labels, or as a download/upload indicator.

```rust
use gpui_kit::component::progress::ProgressCircle;
```

### Usage

```rust
ProgressCircle::new("circle").value(50.0)
```

### Loading State

```rust
// Indeterminate rotating arc animation
ProgressCircle::new("loading").loading(true)

// Toggle between loading and determinate
ProgressCircle::new("circle")
    .loading(self.is_loading)
    .value(self.progress)
```

### Sizes

`ProgressCircle` implements the `Sizable` trait. Named sizes map to fixed pixel dimensions; use `.size(px(n))` for custom sizes:

```rust
ProgressCircle::new("xs").value(50.0).xsmall()    // size_2
ProgressCircle::new("sm").value(50.0).small()     // size_3
ProgressCircle::new("md").value(50.0)             // size_4 (default)
ProgressCircle::new("lg").value(50.0).large()     // size_5
ProgressCircle::new("xl").value(50.0).size_20()   // 80px
```

### Custom Color

```rust
ProgressCircle::new("green").value(75.0).color(cx.theme().green)
ProgressCircle::new("yellow").value(40.0).color(cx.theme().yellow)
ProgressCircle::new("primary").value(60.0).color(cx.theme().primary)
```

### With Inner Content

`ProgressCircle` implements `ParentElement`, so you can place content inside the circle:

```rust
ProgressCircle::new("circle-with-label")
    .value(self.value)
    .size_20()
    .child(
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_1()
            .child(
                div()
                    .child(format!("{}%", self.value as i32))
                    .text_color(cx.theme().progress_bar),
            )
            .child(div().child("Loading").text_xs()),
    )
```

### Inline with Label

```rust
h_flex()
    .gap_2()
    .items_center()
    .child(
        ProgressCircle::new("download")
            .color(cx.theme().primary)
            .value(self.progress)
            .size_4(),
    )
    .child("Downloading...")
```

### API Reference

| Method | Type | Description |
|---|---|---|
| `new(id)` | `ElementId` | Create a new circular progress indicator |
| `value(v)` | `f32` | Set progress value (0–100), clamped automatically |
| `loading(v)` | `bool` | Enable indeterminate loading animation; ignores `value` when `true` |
| `color(c)` | `impl Into<Hsla>` | Override the arc color (defaults to `theme.progress_bar`) |
| `xsmall()` / `small()` / `large()` | — | Set predefined size via `Sizable` |
| `size(px(n))` | `Pixels` | Set custom size |
| `ParentElement` | — | Place content inside the circle |

## Examples

### File Upload

```rust
struct FileUpload {
    uploaded: u64,
    total: u64,
}

impl FileUpload {
    fn progress(&self) -> f32 {
        if self.total == 0 { return 0.0; }
        (self.uploaded as f32 / self.total as f32) * 100.0
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .justify_between()
                    .child("Uploading...")
                    .child(format!("{:.0}%", self.progress())),
            )
            .child(Progress::new("upload").value(self.progress()))
    }
}
```

### Initialization with Loading State

```rust
struct AppInit {
    loading: bool,
    progress: f32,
}

impl Render for AppInit {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        ProgressCircle::new("init-circle")
                            .loading(self.loading)
                            .value(self.progress)
                            .size_4(),
                    )
                    .child(if self.loading { "Initializing..." } else { "Ready" }),
            )
            .child(
                Progress::new("init-bar")
                    .loading(self.loading)
                    .value(self.progress),
            )
    }
}
```

### Multi-Step Process

```rust
struct Install {
    step: usize,       // current package index
    total: usize,      // total packages
    step_progress: f32,
}

impl Install {
    fn overall(&self) -> f32 {
        if self.total == 0 { return 0.0; }
        (self.step as f32 + self.step_progress / 100.0) / self.total as f32 * 100.0
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .justify_between()
                    .child(format!("Package {}/{}", self.step + 1, self.total))
                    .child(format!("{:.0}%", self.overall())),
            )
            .child(Progress::new("overall").value(self.overall()))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Progress::new("package").value(self.step_progress).small())
                    .child("Current package"),
            )
    }
}
```
