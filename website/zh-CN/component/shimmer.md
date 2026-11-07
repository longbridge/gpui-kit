---
title: Shimmer
description: 可复用、遵循主题和 reduced motion 的文字 loading 高光。
---

# Shimmer

`ShimmerText` 为文字提供连续的 loading 高光，适合 thinking、上传中、处理中的短状态。`ShimmerStyle` 保存动画的时长、颜色、宽度、方向和是否只播放一次；它可以独立使用，也可以传给 `Marker` 或 `AttachmentTitle`。

Shimmer 只负责文字表现，不保存 loading 状态。是否显示 shimmer、什么时候改为完成文本，仍由应用状态决定。

## 适用场景

- AI 回复生成中的 “正在思考…” 或 “正在生成…”。
- 文件标题处于 `Uploading` / `Processing` 状态。
- 轻量的文本占位或后台任务状态。

如果需要骨架布局或占位矩形，使用 `Skeleton`；如果需要旋转进度指示，使用 `Spinner`；如果状态只需要静态文字，不要增加动画。

## 导入

```rust
use std::time::Duration;

use gpui_kit::Styled as _;
use gpui_kit::component::{
    ActiveTheme as _, StyledExt as _,
    attachment::{
        Attachment, AttachmentContent, AttachmentDescription, AttachmentStatus, AttachmentTitle,
    },
    marker::{Marker, MarkerContent, MarkerLoadingStyle},
    shimmer::{ShimmerStyle, ShimmerText},
};
```

## 基础用法

```rust
ShimmerText::new("正在思考…")
```

`ShimmerText` 默认使用当前文字上下文的字号、字体、颜色、换行和截断规则。它的默认配置为：

| 配置 | 默认值 | 说明 |
| --- | --- | --- |
| `duration` | 两秒 | 完成一次从左到右的高光扫过。 |
| `highlight_color` | 自动计算 | 根据文字颜色和当前主题生成明亮但可读的高光。 |
| `spread` | 相对 `0.3` | 高光半宽占文字宽度的比例，也可以传 `Pixels` 设置固定宽度。 |
| `reverse` | `false` | 从左向右移动。 |
| `once` | `false` | 默认循环播放。 |

需要给同一文本的多个 sibling 设置独立动画身份时，可以设置 `.id(...)`：

```rust
ShimmerText::new("正在生成…")
    .id("assistant-status")
```

## ShimmerStyle

可以创建一个可复用配置，再传给多个文字：

```rust
let processing_shimmer = ShimmerStyle::new()
    .duration(Duration::from_secs(3))
    .spread(0.4)
    .reverse(true);

ShimmerText::new("正在处理文件…")
    .with_shimmer_style(processing_shimmer);
```

`ShimmerText` 也提供同名的快捷 builder：

```rust
ShimmerText::new("正在处理文件…")
    .duration(Duration::from_secs(3))
    .spread(0.4)
    .reverse(true)
```

### 时长

`duration(...)` 设置一次完整 sweep 的时长：

```rust
ShimmerText::new("正在连接…")
    .duration(Duration::from_secs(4))
```

零时长会被限制为至少一毫秒，避免动画时钟失效。加载状态通常使用较慢的周期；短周期会增加注意力和运动感，应只用于确实需要强调的状态。

### 颜色

默认高光会跟随文字颜色和亮/暗主题。产品有明确强调色时，可以使用主题中的语义 token：

```rust
ShimmerText::new("正在同步…")
    .highlight_color(cx.theme().primary)
```

也可以在共享样式中设置：

```rust
let shimmer = ShimmerStyle::new()
    .highlight_color(cx.theme().info)
    .spread(0.35);
```

不要在组件调用点写固定 hex 颜色；自定义颜色应来自当前主题或应用自己的 token 层，并在亮色和暗色主题中检查对比度。

### Spread

`spread(...)` 设置高光半宽。传 `f32` 表示相对文字宽度的比例，有限值会被限制在 `0.05..=1.0`；传 `Pixels` 表示固定的绝对半宽（最小一像素），适合让长短不一的对齐 label 共享同一条高光宽度：

```rust
ShimmerText::new("正在上传…")
    .spread(0.15); // 窄高光

ShimmerText::new("正在上传…")
    .spread(0.75); // 宽高光

ShimmerText::new("正在上传…")
    .spread(px(48.)); // 固定宽度高光
```

非有限值会保留原配置。较窄的 spread 更克制，较宽的 spread 更容易被注意到。

### 方向与单次播放

需要从右向左移动时使用 `reverse(true)`；`once(true)` 让高光完成一次 sweep 后停止：

```rust
ShimmerText::new("正在准备结果…")
    .reverse(true)
    .once(true)
```

反向只改变动画方向，不改变布局、文字颜色或可访问文本。`once` 适合把一次视觉提示与状态迁移联系起来，但它不会自动改变文字，也不会通知应用；状态完成后仍应由应用重新渲染普通文字。

## 与 Marker 组合

Marker 的 Shimmer loading 会把 `MarkerContent::text(...)` 中的文字替换为 `ShimmerText`：

```rust
Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Shimmer)
    .content(MarkerContent::new().text("正在思考…"))
```

可以把统一配置传给 Marker：

```rust
Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Shimmer)
    .with_shimmer_style(
        ShimmerStyle::new()
            .duration(Duration::from_secs(3))
            .highlight_color(cx.theme().primary)
            .spread(0.4),
    )
    .content(MarkerContent::new().text("正在生成回复…"))
```

显式组合的 `MarkerIcon` 和 Separator 装饰线保持静止；如果需要 Spinner，应选择 `MarkerLoadingStyle::Spinner`。

## 与 Attachment 组合

`AttachmentTitle` 在父级状态为 `Uploading` 或 `Processing` 时自动使用 shimmer：

```rust
Attachment::new()
    .status(AttachmentStatus::Processing)
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("report.pdf"))
            .description(AttachmentDescription::new("正在生成预览")),
    )
```

标题可以覆盖自己的动画配置，父级 status 仍负责决定是否处于 loading：

```rust
Attachment::new()
    .status(AttachmentStatus::Uploading)
    .content(
        AttachmentContent::new().title(
            AttachmentTitle::new("large-export.zip")
                .with_shimmer_style(
                    ShimmerStyle::new()
                        .duration(Duration::from_secs(3))
                        .spread(0.25)
                        .once(true),
                ),
        ),
    )
```

如果调用方给 `AttachmentTitle` 设置了显式 `AttachmentStatus::Complete`，标题不会播放 shimmer；`.with_shimmer_style(...)` 只配置动画，不会开启 loading。

## 在消息与气泡中使用

`ShimmerText` 是普通元素，任何接受文字 child 的位置都可以使用：

```rust
use gpui_kit::component::{
    bubble::{Bubble, BubbleContent, BubbleVariant},
    message::{Message, MessageContent},
};

Message::new()
    .content(
        MessageContent::new().bubble(
            Bubble::new()
                .with_variant(BubbleVariant::Ghost)
                .content(BubbleContent::new().child(
                    ShimmerText::new("助手正在思考…"),
                )),
        ),
    )
```

生成完成后应用应切换到最终消息内容，不要让动画在操作结束后继续运行。

## 样式、主题与 reduced motion

`ShimmerText` 实现 `Styled`，可以像普通文字一样调整字号、颜色、最大宽度和布局：

```rust
ShimmerText::new("正在生成…")
    .text_sm()
    .font_medium()
    .text_color(cx.theme().muted_foreground)
    .max_w_full()
```

动画文本保持 `StyledText` 的布局，因此会继承换行和截断规则。高光的背景与 foreground 由当前主题计算；应用应避免在父级同时设置难以读取的背景和文字颜色。

系统启用 reduced motion 时，`ShimmerText` 会直接渲染静态 `StyledText`，不会请求动画帧。应用无需额外写一个重复的动画分支，但仍应让文字本身完整表达状态。

## 可访问性指引

- shimmer 是视觉提示，状态文字必须本身有意义，不能只显示无文本的高光。“正在思考…”或“正在上传 report.pdf…”比无标签的动画条更有用。
- 颜色和移动方向不能承担唯一语义；完成、失败、暂停等状态应由应用文字或控件表达。
- 操作完成、失败或取消时，停止或替换 shimmer。
- 高光不提供角度、RTL 自动适配或单独的 disable builder。需要关闭时，不渲染 `ShimmerText`，直接渲染普通文字；需要 RTL 语义时由应用布局和文本方向处理，也可以用 `.reverse(true)` 手动控制移动方向。
- 如果应用在 ShimmerText 周围增加 Button、Link 或 overlay，交互控件仍需要自己的可读 label 和键盘路径。
- 显式高光颜色应在亮色和暗色主题中都验证过，避免低对比组合。

## 何时不使用 Shimmer

- 需要表示确定百分比时使用 `Progress`。
- 需要持续旋转指示器时使用 `Spinner`。
- 需要多行占位布局时使用 `Skeleton`。
- 已经有稳定结果时直接渲染普通 `StyledText`，不要让动画继续运行。

## API 参考

### `ShimmerStyle`

| 方法 | 默认值 | 说明 |
| --- | --- | --- |
| `new()` | 同 `Default` | 创建主题高光、两秒周期的循环配置。 |
| `duration(Duration)` | 两秒 | 设置完整 sweep 时长；最小为一毫秒。 |
| `highlight_color(Hsla)` | 主题计算 | 设置显式高光颜色，覆盖主题计算。 |
| `spread(f32 \| Pixels)` | 相对 `0.3` | 设置高光半宽：`f32` 为相对比例（限制在 `0.05..=1.0`），`Pixels` 为绝对宽度（最小 1px）。 |
| `reverse(bool)` | `false` | 设置是否从右向左移动。 |
| `once(bool)` | `false` | 设置是否只完成一次 sweep。 |

### `ShimmerText`

| 方法 | 默认值 | 说明 |
| --- | --- | --- |
| `new(text)` | 默认样式、按文字生成身份 | 创建 loading 文字。 |
| `id(ElementId)` | 基于文字的身份 | 区分文字相同的 sibling。 |
| `with_shimmer_style(ShimmerStyle)` | 默认样式 | 应用完整的可复用配置。 |
| `duration(Duration)` | 两秒 | 直接设置时长。 |
| `highlight_color(Hsla)` | 主题计算 | 直接设置颜色。 |
| `spread(f32 \| Pixels)` | 相对 `0.3` | 直接设置半宽。 |
| `reverse(bool)` | `false` | 直接设置方向。 |
| `once(bool)` | `false` | 直接设置播放次数。 |
| `Styled` 方法 | 继承文字样式 | 调整字号、颜色、宽度、字体和布局。 |

### 相关组件

- [`Marker`] — 支持 spinner 或 shimmer loading 的状态行。
- [`AttachmentTitle`] — 感知状态、可自定义 shimmer 的文件标题。
- [`Progress`] — 确定进度。
- [`Spinner`] — 紧凑的不确定进度指示。

[ShimmerStyle]: https://docs.rs/gpui-component/latest/gpui_component/shimmer/struct.ShimmerStyle.html
[ShimmerText]: https://docs.rs/gpui-component/latest/gpui_component/shimmer/struct.ShimmerText.html
[Marker]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.Marker.html
[AttachmentTitle]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentTitle.html
[Progress]: https://docs.rs/gpui-component/latest/gpui_component/progress/struct.Progress.html
[Spinner]: https://docs.rs/gpui-component/latest/gpui_component/spinner/struct.Spinner.html
