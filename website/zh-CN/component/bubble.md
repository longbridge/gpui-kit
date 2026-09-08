---
title: Bubble
description: 可承载文本、富内容和 reaction 控件的聊天消息表面。
---

# Bubble

`Bubble` 是聊天内容的可组合表面。它负责对齐、内容宽度和 reaction 的定位；`BubbleContent` 负责背景、边框、圆角、padding 和文字样式。应用可以把 `Button`、`Link`、`Collapsible`、`Tooltip` 或 `Popover` 放在 bubble 内部，而不需要为每一种消息内容增加专用组件。

## 适用场景

- 用于一条消息中的文本、代码、文件摘要或其他富内容。
- 用 `BubbleGroup` 堆叠同一发送者的连续消息。
- 用 `BubbleReactions` 把可聚焦的 `Button` 放在 bubble 的上方或下方。
- 已经有 `Message` 时，让 `Message` 负责发送者、header、footer 和整体对齐；Bubble 只负责消息表面。

如果内容只是普通的图标和文本行，或需要完整的消息生命周期状态，请直接组合 `h_flex()`、`Marker`、`Message` 等更合适的组件。

## 导入

基础组合需要以下类型：

```rust
use gpui_kit::{ParentElement as _, Styled as _};
use gpui_kit::component::{
    bubble::{
        Bubble, BubbleContent, BubbleGroup, BubbleReactionSide,
        BubbleReactions, BubbleVariant,
    },
    button::{Button, ButtonVariants as _},
    collapsible::Collapsible,
    h_flex,
    link::Link,
    message::MessageAlignment,
    popover::Popover,
    ActiveTheme as _, Colorize as _, IconName, Sizable as _, StyledExt as _, v_flex,
};
```

这些 trait import 让文中的 `child(...)`、主题读取、语义尺寸和 Button variant builder 与实际 crate API 对齐。

## 结构

```text
Bubble
├── BubbleContent       # 可见的内容表面
└── BubbleReactions     # 可选，附着在表面边缘
    └── Button / 其他交互控件
```

`Bubble` 的直接 `.child(...)` 会把 child 添加到 `BubbleContent`。需要精确控制 surface 样式、或需要区分多个富内容区域时，使用 `.content(BubbleContent::new()...)`。

## 基础用法

最短的文本 bubble 可以直接添加 child：

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .child("可以帮我检查一下吗？")
```

显式创建 `BubbleContent` 适合需要调整 surface 的场景：

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .content(
        BubbleContent::new()
            .child("这里可以继续添加任意 GPUI element。"),
    )
```

`Bubble::new()` 默认使用 `BubbleVariant::Filled`。Bubble 默认不绑定对齐，适合由外层 `Message` 传播对齐；独立使用时应设置 `.alignment(...)`。

## 对齐

`Bubble` 与 `Message` 共用 `MessageAlignment`：

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .child("收到的消息");

Bubble::new()
    .alignment(MessageAlignment::End)
    .child("发出的消息")
```

| 值 | 含义 |
| --- | --- |
| `MessageAlignment::Start` | 放在消息行的起始侧。 |
| `MessageAlignment::End` | 放在消息行的结束侧。 |
| 未设置 | 保留给父级布局决定；放进 `MessageContent` 时通常使用这一方式。 |

Bubble 的普通 variant 最大宽度为可用宽度的 80%，`Ghost` variant 会占满父级宽度。宽度仍可以在 `Bubble` 或 `BubbleContent` 上通过 `Styled` refinement 调整。

## 样式变体

| Variant | 适合表达的语义 | 默认 surface |
| --- | --- | --- |
| `Filled` | 主要消息内容，默认值。 | `primary` 与 `primary_foreground`。 |
| `Secondary` | 次级强调的消息。 | `muted` 底与 `secondary_foreground`（主题的 `secondary` 是按钮角色，比 shadcn 的会话 secondary 深一档）。 |
| `Muted` | 低强调度的普通内容。 | `muted` 与普通前景色。 |
| `Tinted` | 轻微使用 primary 色调的内容。 | 由主题背景与 primary 混合。 |
| `Outline` | 需要清晰边界但不需要填充色的内容。 | 背景色与 `border`。 |
| `Ghost` | 作为消息布局中的无表面富内容。 | 无 padding、边框和背景。 |
| `Destructive` | 失败、拒绝或无效结果。 | 语义 destructive 色。 |

```rust
for variant in [
    BubbleVariant::Filled,
    BubbleVariant::Secondary,
    BubbleVariant::Muted,
    BubbleVariant::Tinted,
    BubbleVariant::Outline,
    BubbleVariant::Ghost,
    BubbleVariant::Destructive,
] {
    let bubble = Bubble::new()
        .alignment(MessageAlignment::Start)
        .with_variant(variant)
        .child("同一内容可以切换不同语义表面");
    // 在应用的 view 中渲染 bubble。
}
```

在实际界面中应根据内容语义选择 variant，不要仅为增加色彩而并列使用所有 variant。`Destructive` 也应配合文字说明，不能只依靠颜色表达失败。

## 富内容

`BubbleContent` 接受任意 GPUI element，因此代码块、文件摘要和多段内容都可以由应用组合：

```rust
use gpui_kit::{div, ParentElement as _, Styled as _};
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Sizable as _, StyledExt as _};

Bubble::new()
    .alignment(MessageAlignment::Start)
    .content(
        BubbleContent::new().child(
            v_flex()
                .gap_2()
                .child("请查看下面的文件：")
                .child(
                    h_flex()
                        .gap_2()
                        .child("📄")
                        .child("quarterly-report.pdf"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("PDF · 2.4 MB"),
                ),
        ),
    )
```

较长文本仍由调用方决定换行、截断和最小宽度。可将 `min_w_0()`、`max_w_full()` 等 refinement 放在富内容的内部容器上，使内容不会把消息行撑开。

### 链接和按钮

链接和应用命令应该保留自己的语义：URL 使用 `Link`，应用操作使用 `Button`。它们可以直接放进 `BubbleContent`：

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .content(
        BubbleContent::new()
            .child("文档已更新：")
            .child(
                Link::new("release-notes")
                    .href("https://example.com/release-notes")
                    .child("查看 release notes"),
            )
            .child(
                Button::new("retry")
                    .ghost()
                    .small()
                    .label("重试"),
            ),
    )
```

不要把整个 bubble 变成一个不可区分的点击区域。每个操作都应有明确的焦点目标、标签和结果。

### 折叠内容

Bubble 没有专用的 `ShowMore` API。需要折叠长文本时，直接组合 `Collapsible`，并由应用状态控制 `open`：

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .content(
        BubbleContent::new().child(
            Collapsible::new()
                .open(show_details)
                .child(
                    Button::new("toggle-details")
                        .ghost()
                        .small()
                        .label(if show_details { "收起详情" } else { "显示详情" }),
                )
                .content(
                    div()
                        .text_sm()
                        .child("这里放置较长的诊断信息或工具输出。"),
                ),
        ),
    )
```

`Collapsible::open(...)` 是静态配置；切换状态、键盘 action 和状态持有仍属于应用 view。

### Tooltip 和 Popover

Tooltip 应附着在具体的 icon button 或 link 上。Button 支持通过 `tooltip(...)` 添加提示：

```rust
Bubble::new()
    .content(
        BubbleContent::new().child(
            Button::new("copy-message")
                .ghost()
                .icon(IconName::Copy)
                .label("复制")
                .tooltip("复制消息"),
        ),
    )
```

需要显示更多操作或上下文内容时，可以组合 `Popover`。Popover 的触发器与内容由应用提供：

```rust
Popover::new("message-options")
    .trigger(
        Button::new("message-options-trigger")
            .ghost()
            .icon(IconName::Ellipsis)
            .label("更多操作"),
    )
    .child(Button::new("copy").label("复制"))
    .child(Button::new("report").label("报告问题"))
```

Popover 需要在包含 `Root` 的窗口中使用，触发器应保持可聚焦；菜单项的权限、关闭后续行为和业务状态由应用负责。

## 分组

`BubbleGroup` 只负责以统一间距堆叠连续 bubble，不保存发送者或时间信息：

```rust
BubbleGroup::new()
    .child(
        Bubble::new()
            .alignment(MessageAlignment::Start)
            .with_variant(BubbleVariant::Muted)
            .child("第一条消息"),
    )
    .child(
        Bubble::new()
            .alignment(MessageAlignment::Start)
            .with_variant(BubbleVariant::Muted)
            .child("同一发送者的第二条消息"),
    )
```

跨发送者、头像、header 和 footer 的组合应使用 `MessageGroup` 或应用自己的消息列表。

## Reactions

`BubbleReactions` 负责 reaction 区域的边缘定位与基础表面。对于需要和
reaction 表面连成一体的按钮，使用类型明确的 `action(Button)`：

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .child("看起来没问题。")
    .reactions(
        BubbleReactions::new()
            .side(BubbleReactionSide::Bottom)
            .alignment(MessageAlignment::End)
            .action(
                Button::new("like")
                    .ghost()
                    .small()
                    .label("👍 2"),
            )
            .action(
                Button::new("reply")
                    .ghost()
                    .small()
                    .label("回复"),
            ),
    )
```

reaction 可以附着到上边缘：

```rust
Bubble::new()
    .child("需要在上方显示状态的消息")
    .reactions(
        BubbleReactions::new()
            .side(BubbleReactionSide::Top)
            .alignment(MessageAlignment::Start)
            .action(Button::new("status").ghost().label("处理中")),
    )
```

`action(Button)` 会让 `BubbleReactions` 识别出这是语义操作。当 reaction 区域
包含任意一个类型化操作时，组件会移除装饰性内边距，并将每个类型化
Button 的圆角设为当前主题的最大圆角（`radius_full()`），使按钮和外层表面连成一个
整体。传入的 `Button` 仍可以自定义变体、尺寸、图标、`.on_click(...)` 点击回调和
`.tooltip(...)`；类型化操作会保留这个胶囊圆角以维持整体外观，需要不同圆角时
使用下面的通用路径。多个按钮可以重复调用 `.action(...)`。

需要放入 emoji、文本、自定义子元素或直接 `Button` 之外的包装组件时，继续使用
`.child(...)`。这个通用路径保持向后兼容，也不会自动启用操作按钮的紧凑样式。
即使同一区域混合了 `.child(...)`，只要存在一个 `.action(...)`，整个 reaction 区域
仍会采用紧凑表面布局；普通子元素继续按通用路径渲染：

```rust
BubbleReactions::new()
    .child("👍 2")
    .action(
        Button::new("reply")
            .ghost()
            .small()
            .label("回复"),
    )
```

像 `Popover` 这样的嵌套交互包装组件也应继续使用 `.child(...)`，因为
`action(...)` 接收的是直接的 `Button`。如果包装组件的触发按钮需要和 reaction
表面共用几何样式，可以在 `BubbleReactions` 上显式使用 `p_0()`，并在触发按钮上
使用主题的最大圆角；需要给普通 Button 保留其他圆角或表面样式时也使用这个通用路径：

```rust
BubbleReactions::new().p_0().child(
    gpui_kit::component::popover::Popover::new("bubble-more")
        .trigger(
            Button::new("bubble-more-trigger")
                .ghost()
                .small()
                .label("更多")
                .rounded(cx.theme().radius_full()),
        )
        .child(Button::new("bubble-copy").label("复制")),
)
```

reaction 表面的默认样式仍由组件提供；调用方的 `Styled` 样式调整会在默认值之后
应用，因此可以继续调整 `BubbleReactions` 或 `Button` 的其他样式。`.action(...)`
会统一管理 Button 的胶囊圆角；需要恢复额外内边距或使用不同圆角时，使用
`.child(...)`，或在 reaction 区域上明确调用 `px_2()`、`p_0()` 等样式方法。
`BubbleReactions` 不额外提供 `BubbleAction` 或 reaction 数据模型；计数、选中状态、
提交动作和错误提示由 Button 外层的应用状态负责。

## 自定义样式与主题 token

所有公开 part 都实现 `Styled`。默认样式先应用，调用方 refinement 后应用，因此可以只调整需要改变的部分：

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .px_2()
    .content(
        BubbleContent::new()
            .rounded(cx.theme().radius_lg)
            .bg(cx.theme().muted)
            .text_color(cx.theme().foreground)
            .border_color(cx.theme().border)
            .child("遵循当前主题的自定义消息表面"),
    )
```

优先从 `cx.theme()` 读取语义颜色、圆角和间距。Bubble 的外层布局、可见 surface 和 reactions 各自有样式入口，应用可以在不复制组件内部布局的情况下调整背景、文字、边框、间距、最大宽度和 reaction 位置。

## 可访问性

- Reaction 使用 `Button`；可见 `.label(...)` 是控件的可读名称，tooltip 只作为补充提示。`👍 2` 这类文本应同时说明它是“点赞”动作。
- 链接使用 `Link`，应用操作使用 `Button`，避免用普通 `div` 模拟控件。
- `Destructive`、`Tinted` 等颜色只提供视觉层次；失败、状态或结果必须在文本中说明。
- 折叠区域的触发器应是可聚焦的 `Button`，并在状态改变时更新“展开/收起”等可读标签。
- 需要键盘操作的 Popover 触发器和内容应遵循 `Root` 提供的焦点与 overlay 生命周期。
- 系统启用 reduced motion 时，Bubble 本身没有额外动画；应用为 child 增加动画时也应提供静态结果。

## 何时不需要 Bubble

- 只有状态文本和分隔线：使用 `Marker`。
- 需要头像、发送者、时间和送达状态：使用 `Message`，将 Bubble 放进 `MessageContent`。
- 只有一个独立操作：使用 `Button` 或 `Link`。
- 只需要一组普通横向内容：使用 `h_flex()` 或 `v_flex()`，避免为简单布局增加 Bubble 表面。

## API 参考

### `Bubble`

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建默认的 filled bubble；默认不设置对齐。 |
| `alignment(MessageAlignment)` | 设置起始侧或结束侧对齐。 |
| `with_variant(BubbleVariant)` | 设置 bubble surface variant。 |
| `content(BubbleContent)` | 替换可见内容 surface；已添加的直接 children 会并入其中。 |
| `reactions(BubbleReactions)` | 添加可选 reaction 区域。 |
| `child(element)` | 通过 `ParentElement` 将 child 添加到 `BubbleContent`。 |
| `Styled` | 调整外层布局、宽度、间距和其他 GPUI 样式。 |

### `BubbleContent`

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建空的内容 surface。 |
| `child(element)` | 添加任意 GPUI element。 |
| `Styled` | 调整 padding、背景、文字、边框和圆角。 |

### `BubbleGroup`

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建连续 bubble 的垂直 stack。 |
| `child(element)` | 按顺序添加 bubble 或其他 element。 |
| `Styled` | 调整 stack 间距、宽度和布局。 |

### `BubbleReactions`

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建默认在底部、结束侧对齐的 reaction 区域。 |
| `side(BubbleReactionSide)` | 选择 `Top` 或 `Bottom`。 |
| `alignment(MessageAlignment)` | 选择 reaction 区域的起始侧或结束侧对齐。 |
| `action(Button)` | 添加类型化操作；与 reaction 表面共用主题最大圆角，并自动使用紧凑布局。 |
| `child(element)` | 添加 emoji、文本或任意 GPUI 子元素；保留通用组合能力。 |
| `Styled` | 调整 reaction 表面与定位 refinement。 |

### 类型链接

- [Bubble]
- [BubbleContent]
- [BubbleGroup]
- [BubbleReactions]
- [BubbleVariant]
- [BubbleReactionSide]

[Bubble]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.Bubble.html
[BubbleContent]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.BubbleContent.html
[BubbleGroup]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.BubbleGroup.html
[BubbleReactions]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.BubbleReactions.html
[BubbleVariant]: https://docs.rs/gpui-component/latest/gpui_component/bubble/enum.BubbleVariant.html
[BubbleReactionSide]: https://docs.rs/gpui-component/latest/gpui_component/bubble/enum.BubbleReactionSide.html
