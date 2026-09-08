---
title: Marker
description: 用于会话状态、通知边界和分隔标记的紧凑组合行。
---

# Marker

`Marker` 是一种轻量的全宽会话行，适合状态文字、时间线边界、未读提示和系统消息。它只提供通用布局与视觉变体，不定义 `Online`、`Typing`、`Read` 等业务状态；图标、内容、颜色和交互行为由应用组合。

## 适用场景

- 一行简短的系统状态或会话提示。
- 带有左右装饰线的日期、未读边界或阶段标题。
- 需要 Spinner 或文字 shimmer 的 loading 状态。
- 需要在行中放置 `Button` 或 `Link`，但不希望把整行变成一个按钮。

只有一个数量或状态标签时，`Badge` 或 `Tag` 更直接；只有一条带文字的分隔线时，使用 `Separator`。

## 导入

```rust
use gpui_kit::{ParentElement as _, StyleRefinement, Styled as _};
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    marker::{Marker, MarkerContent, MarkerIcon, MarkerLoadingStyle, MarkerVariant},
    shimmer::{ShimmerStyle, ShimmerText},
    spinner::Spinner,
    ActiveTheme as _, Colorize as _, Icon, IconName, Sizable as _, StyledExt as _,
};
```

## 结构

```text
Marker
├── MarkerIcon       # 可选，紧凑图标 slot
├── MarkerContent    # 文本或富内容 slot
└── 任意 child        # Button、Link 或其他 GPUI element
```

`MarkerIcon` 和 `MarkerContent` 是有默认尺寸与文字布局的具名 slot；`.child(...)` 仍可用于应用自己的组合。`Marker` 本身保持为语义容器，不会替应用创建状态 enum。

## Plain、Separator 与 Border

`MarkerVariant` 提供三个有明确用途的表面：

| Variant | 用途 | 默认布局 |
| --- | --- | --- |
| `Plain` | 普通状态或系统消息，默认值。 | 全宽紧凑行。 |
| `Separator` | 日期、阶段或未读边界。 | 内容两侧显示主题边框线。 |
| `Border` | 需要底部边界的状态行。 | 内容下方显示语义边框。 |

```rust
Marker::new()
    .content(MarkerContent::new().text("会话已归档"));

Marker::new()
    .with_variant(MarkerVariant::Separator)
    .content(MarkerContent::new().text("今天"));

Marker::new()
    .with_variant(MarkerVariant::Border)
    .content(MarkerContent::new().text("3 条未读消息"))
```

Separator 的装饰线是内部实现，不携带语义内容。文本本身应说明它代表的日期、边界或状态。

## 状态内容与图标

应用可以组合 Icon、Spinner、Badge 或自己的富内容：

```rust
Marker::new()
    .text_color(cx.theme().primary)
    .icon(
        MarkerIcon::new()
            .child(Icon::new(IconName::CircleCheck)),
    )
    .content(MarkerContent::new().text("在线"));

Marker::new()
    .icon(MarkerIcon::new().child(Spinner::new().xsmall()))
    .content(MarkerContent::new().text("Alice 正在输入…"))
```

`MarkerIcon` 会保留紧凑的图标槽尺寸。图标和文字的间距、文字字号与行高跟随共享设计系统；自定义 child 不会被 Marker 改写其内部语义。

## Loading 样式

通过 `.loading(true)` 启用 loading；loading 不会改变 Marker 的 variant 或普通布局。默认样式是 `Spinner`：

```rust
Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Spinner)
    .content(MarkerContent::new().text("正在加载消息…"))
```

没有显式 `MarkerIcon` 时，Spinner loading 会自动添加紧凑 Spinner；如果已经组合 `MarkerIcon`，显式图标优先：

```rust
Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Spinner)
    .icon(MarkerIcon::new().child(Icon::new(IconName::LoaderCircle)))
    .content(MarkerContent::new().text("同步中…"))
```

需要文字高光时使用 `Shimmer`：

```rust
Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Shimmer)
    .content(MarkerContent::new().text("正在思考…"))
```

只有通过 `MarkerContent::text(...)` 添加的文本会使用平滑移动的 shimmer。普通 child 仍然可用，但 loading 时会使用轻微透明度变化；图标和 Separator 装饰线保持静止。

## 配置 Shimmer

Marker 可以接受一个可复用的 `ShimmerStyle`。默认动画周期为两秒、spread 为 `0.3`、方向为从左到右并循环播放：

```rust
use std::time::Duration;

Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Shimmer)
    .with_shimmer_style(
        ShimmerStyle::new()
            .duration(Duration::from_secs(3))
            .highlight_color(cx.theme().primary)
            .spread(0.45)
            .reverse(true)
            .once(true),
    )
    .content(MarkerContent::new().text("正在处理…"))
```

可以分别调整单项配置：

```rust
// 更慢的循环
ShimmerStyle::new().duration(Duration::from_secs(4));

// 更窄的高光带；值会限制在 0.05..=1.0
ShimmerStyle::new().spread(0.15);

// 使用当前主题中的语义色
ShimmerStyle::new().highlight_color(cx.theme().primary);

// 从右向左移动，并只完成一次
ShimmerStyle::new().reverse(true).once(true)
```

`duration(Duration::ZERO)` 会被限制为至少一毫秒；`spread` 可传相对比例 `f32`（限制在 `0.05..=1.0`）或绝对宽度 `Pixels`，非有限值会保留当前值。显式 highlight color 适合产品有明确强调色的场景，默认值会根据文本色和当前主题计算。

## 独立使用 ShimmerText

需要在 Marker 之外显示 thinking 或上传文案时，直接使用 `ShimmerText`：

```rust
ShimmerText::new("正在上传 report.pdf…")
    .with_shimmer_style(ShimmerStyle::new().spread(0.4))
    .text_sm()
    .text_color(cx.theme().muted_foreground)
```

也可以使用 `duration(...)`、`highlight_color(...)`、`spread(...)`、`reverse(...)` 和 `once(...)` 的快捷 builder：

```rust
ShimmerText::new("Generating…")
    .duration(std::time::Duration::from_secs(3))
    .highlight_color(cx.theme().primary)
    .spread(0.35)
    .reverse(true)
```

`ShimmerText` 实现 `Styled`，会继承周围的字号、字体、文字颜色、换行和截断样式。完整参数与 reduced motion 行为见 [Shimmer] 文档。

## 链接和按钮

Marker 可以包含交互 child，但交互应保持局部、明确：

```rust
Marker::new()
    .with_variant(MarkerVariant::Border)
    .content(
        MarkerContent::new()
            .text("同步失败")
            .child(
                Button::new("retry-sync")
                    .ghost()
                    .small()
                    .label("重试"),
            ),
    )
```

URL 使用 `Link`，应用操作使用 `Button`。Marker 不会自动给任意 child 添加 loading、selected 或 focus 语义。

## 分隔线样式

`separator_style(...)` 只作用于 Separator 两侧的内部装饰线；内容本身的颜色和布局通过 `Marker`、`MarkerContent` 的 `Styled` refinement 调整：

```rust
Marker::new()
    .with_variant(MarkerVariant::Separator)
    .separator_style(
        StyleRefinement::default()
            .bg(cx.theme().muted_foreground.opacity(0.35)),
    )
    .content(
        MarkerContent::new()
            .text_color(cx.theme().muted_foreground)
            .text("2026 年 8 月 25 日"),
    )
```

如果需要完全不同的分隔布局，可以使用 `h_flex()` 与 `Separator`；不要把内部装饰线当作应用状态的独立节点。

## 自定义样式与主题 token

`Marker`、`MarkerIcon` 和 `MarkerContent` 都实现 `Styled`。默认样式之后应用调用方 refinement，因此只覆盖必要部分：

```rust
Marker::new()
    .px_3()
    .py_2()
    .rounded(cx.theme().radius_lg)
    .bg(cx.theme().muted)
    .text_color(cx.theme().foreground)
    .icon(
        MarkerIcon::new()
            .text_color(cx.theme().primary)
            .child(Icon::new(IconName::Star)),
    )
    .content(MarkerContent::new().text("已置顶消息"))
```

优先使用 `cx.theme()` 的语义颜色与主题圆角。Marker 的外层布局、图标 slot、内容 slot 和 Separator 装饰线各自可以定制；这些 refinement 不会改变 loading 状态的业务所有权。

## 可访问性与 reduced motion

- 状态、进度、日期和未读边界必须有可读文本；颜色和装饰线只能提供辅助层次。
- Marker 默认是纯展示元素。表示流式或加载进度的行可以设置 `.id(...)` 加 `.role(Role::Status)`，让辅助技术播报其更新；role 依赖 id 提供的稳定标识。
- 只有图标的交互 child 使用 `Button` 并提供可见的 `.label(...)` 或其他可读名称；tooltip 只作为补充提示。
- Separator 线是装饰性的，语义应来自 `MarkerContent` 中的文字。
- `MarkerContent::text(...)` 的 Shimmer 在系统启用 reduced motion 时会显示静态文字，不请求动画帧。
- Marker 的普通 child 可能是自定义动画；应用应在 reduced motion 下提供清晰的静态状态。
- 不要让整个 Marker 成为 hover 才能发现的唯一操作入口；将操作放入明确可聚焦的 Button 或 Link。

## 何时不需要 Marker

- 只有数量、圆点或短状态时使用 `Badge`。
- 独立的标签状态使用 `Tag`。
- 只有一条带文字的分隔线时使用 `Separator::horizontal().label(...)`。
- 应用专属的 icon + text 行没有共享 Marker 语义时，使用 `h_flex()`。
- 需要头像、发送者、消息内容和 footer 时，使用 `Message`。

当这些内容需要统一的会话行表面，或要在 plain、separator、border 之间切换时，再使用 Marker。

## API 参考

### `Marker`

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建默认的 plain marker。 |
| `with_variant(MarkerVariant)` | 设置 `Plain`、`Separator` 或 `Border`。 |
| `loading(bool)` | 开启或关闭 loading。默认关闭。 |
| `with_loading_style(MarkerLoadingStyle)` | 选择 `Spinner` 或 `Shimmer`。 |
| `with_shimmer_style(ShimmerStyle)` | 配置文字 shimmer。 |
| `separator_style(StyleRefinement)` | 调整 Separator 的内部装饰线。 |
| `id(ElementId)` | 设置稳定标识，让 marker 进入无障碍树。 |
| `role(Role)` | 设置辅助技术播报的 role；流式更新用 `Role::Status`，需要配合 `id(...)`，默认纯展示。 |
| `icon(MarkerIcon)` | 添加配置过的图标 slot。 |
| `content(MarkerContent)` | 添加配置过的内容 slot。 |
| `child(element)` | 添加任意 child。 |
| `Styled` | 调整 marker 的布局、颜色、间距和 surface。 |

### `MarkerIcon` / `MarkerContent`

| 类型 | 方法 | 说明 |
| --- | --- | --- |
| `MarkerIcon` | `new()` / `child(element)` | 创建紧凑图标 slot。 |
| `MarkerContent` | `new()` / `text(text)` | 创建内容 slot，`text` 允许 Shimmer 直接替换文字渲染。 |
| 两者 | `Styled` | 调整各自的尺寸、颜色、文字和布局。 |

### 类型链接

- [Marker]
- [MarkerVariant]
- [MarkerLoadingStyle]
- [MarkerIcon]
- [MarkerContent]
- [ShimmerStyle]
- [ShimmerText]

[Marker]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.Marker.html
[MarkerVariant]: https://docs.rs/gpui-component/latest/gpui_component/marker/enum.MarkerVariant.html
[MarkerLoadingStyle]: https://docs.rs/gpui-component/latest/gpui_component/marker/enum.MarkerLoadingStyle.html
[MarkerIcon]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.MarkerIcon.html
[MarkerContent]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.MarkerContent.html
[ShimmerStyle]: https://docs.rs/gpui-component/latest/gpui_component/shimmer/struct.ShimmerStyle.html
[ShimmerText]: https://docs.rs/gpui-component/latest/gpui_component/shimmer/struct.ShimmerText.html
