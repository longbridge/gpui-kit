---
title: Progress
description: 用于显示任务完成进度的线性或环形指示器。
---

# Progress

Progress 组件用于直观展示任务完成百分比。库中提供两种形式：

- **[Progress](#progress)**：线性水平进度条
- **[ProgressCircle](#progresscircle)**：环形进度指示器

这两个组件都支持数值变化动画、加载中（不确定进度）状态、自定义颜色以及自动适配当前主题。

## Progress

```rust
use gpui_kit::component::progress::Progress;
```

### 用法

```rust
Progress::new("my-progress")
    .value(75.0)
```

### 不同进度值

```rust
Progress::new("progress-0").value(0.0)
Progress::new("progress-25").value(25.0)
Progress::new("progress-75").value(75.0)
Progress::new("progress-100").value(100.0)
```

### 加载状态

当实际进度未知时，可通过 `.loading(true)` 显示不确定进度动画。启用后会忽略 `value`。

```rust
Progress::new("loading").loading(true)

Progress::new("my-progress")
    .loading(self.is_loading)
    .value(self.progress)
```

### 尺寸

`Progress` 实现了 `Sizable` trait：

```rust
Progress::new("xs").value(50.0).xsmall()
Progress::new("sm").value(50.0).small()
Progress::new("md").value(50.0)
Progress::new("lg").value(50.0).large()
```

### 自定义样式

组件实现了 `Styled` trait，可自定义高度、圆角、颜色和边框：

```rust
Progress::new("custom")
    .value(32.0)
    .h(px(16.))
    .rounded(px(2.))
    .color(cx.theme().green_light)
    .border_2()
    .border_color(cx.theme().green)
```

### 动态更新进度

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

### API 参考

| 方法 | 类型 | 说明 |
|---|---|---|
| `new(id)` | `ElementId` | 创建新的进度条 |
| `value(v)` | `f32` | 设置进度值，范围 0 到 100，超出时会自动裁剪 |
| `loading(v)` | `bool` | 开启不确定进度动画；为 `true` 时忽略 `value` |
| `color(c)` | `impl Into<Hsla>` | 覆盖填充颜色，默认使用 `theme.progress_bar` |
| `xsmall()` / `small()` / `large()` | — | 通过 `Sizable` 设置预定义高度 |
| `Styled` trait methods | — | 自定义高度、圆角、边框等 |

## ProgressCircle

环形进度指示器适合用于紧凑场景、内联状态或下载上传进度。

```rust
use gpui_kit::component::progress::ProgressCircle;
```

### 用法

```rust
ProgressCircle::new("circle").value(50.0)
```

### 加载状态

```rust
ProgressCircle::new("loading").loading(true)

ProgressCircle::new("circle")
    .loading(self.is_loading)
    .value(self.progress)
```

### 尺寸

`ProgressCircle` 也实现了 `Sizable` trait。命名尺寸映射到固定像素值，也可以通过 `.size(px(n))` 设置自定义尺寸：

```rust
ProgressCircle::new("xs").value(50.0).xsmall()
ProgressCircle::new("sm").value(50.0).small()
ProgressCircle::new("md").value(50.0)
ProgressCircle::new("lg").value(50.0).large()
ProgressCircle::new("xl").value(50.0).size_20()
```

### 自定义颜色

```rust
ProgressCircle::new("green").value(75.0).color(cx.theme().green)
ProgressCircle::new("yellow").value(40.0).color(cx.theme().yellow)
ProgressCircle::new("primary").value(60.0).color(cx.theme().primary)
```

### 内部内容

`ProgressCircle` 实现了 `ParentElement`，所以你可以在环形内部放置内容：

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

### 和文本一起内联显示

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

### API 参考

| 方法 | 类型 | 说明 |
|---|---|---|
| `new(id)` | `ElementId` | 创建新的环形进度组件 |
| `value(v)` | `f32` | 设置进度值，范围 0 到 100，超出时会自动裁剪 |
| `loading(v)` | `bool` | 开启不确定进度动画；为 `true` 时忽略 `value` |
| `color(c)` | `impl Into<Hsla>` | 覆盖弧线颜色，默认使用 `theme.progress_bar` |
| `xsmall()` / `small()` / `large()` | — | 通过 `Sizable` 设置预定义尺寸 |
| `size(px(n))` | `Pixels` | 设置自定义尺寸 |
| `ParentElement` | — | 允许在环形内部放置内容 |

## 示例

### 文件上传

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

### 初始化加载状态

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

### 多步骤流程

```rust
struct Install {
    step: usize,
    total: usize,
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
