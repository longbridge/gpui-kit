---
title: HoverCard
description: 鼠标悬停时显示富内容浮层的组件。
---

# HoverCard

HoverCard 用于在鼠标悬停到触发元素时显示富内容浮层，适合做用户资料预览、链接预览和上下文信息展示。它支持打开和关闭延迟，以减少鼠标快速移动时的闪烁问题。

它和 [Popover] 很像，但触发方式是 hover 而不是 click，并且提供了更细的时间控制。

## 导入

```rust
use gpui_kit::component::hover_card::HoverCard;
```

## 用法

### 基础 HoverCard

```rust
use gpui_kit::{ParentElement as _, Styled as _};
use gpui_kit::component::{hover_card::HoverCard, v_flex};

HoverCard::new("basic")
    .trigger(
        div()
            .child("Hover over me")
            .text_color(cx.theme().primary)
            .cursor_pointer()
            .text_sm()
    )
    .child(
        v_flex()
            .gap_2()
            .child(
                div()
                    .child("This is a hover card")
                    .font_semibold()
                    .text_sm()
            )
            .child(
                div()
                    .child("You can display rich content when hovering over a trigger element.")
                    .text_color(cx.theme().muted_foreground)
                    .text_sm()
            )
    )
```

### 用户资料预览

这是一个很常见的场景，类似 GitHub 或 Twitter 的用户悬停预览：

```rust
use gpui_kit::{px, relative, Styled as _};
use gpui_kit::component::{
    avatar::Avatar,
    hover_card::HoverCard,
    h_flex,
    v_flex,
};

h_flex()
    .child("Hover over ")
    .text_sm()
    .child(
        HoverCard::new("user-profile")
            .trigger(
                div()
                    .child("@huacnlee")
                    .cursor_pointer()
                    .text_color(cx.theme().link)
            )
            .child(
                h_flex()
                    .w(px(320.))
                    .gap_4()
                    .items_start()
                    .child(
                        Avatar::new()
                            .src("https://avatars.githubusercontent.com/u/5518?s=64")
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .line_height(relative(1.))
                            .child(div().child("Jason Lee").font_semibold())
                            .child(
                                div()
                                    .child("@huacnlee")
                                    .text_color(cx.theme().muted_foreground)
                                    .text_sm()
                            )
                            .child("The author of GPUI Kit.")
                    )
            )
    )
    .child(" to see their profile")
```

### 自定义时间控制

你可以按需调整打开和关闭延迟：

```rust
use std::time::Duration;
use gpui_kit::Styled as _;
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    h_flex,
};

h_flex()
    .gap_4()
    .child(
        HoverCard::new("fast-open")
            .open_delay(Duration::from_millis(200))
            .close_delay(Duration::from_millis(100))
            .trigger(Button::new("fast").label("Fast Open (200ms)").outline())
            .child(div().child("This hover card opens after 200ms").text_sm())
    )
    .child(
        HoverCard::new("slow-open")
            .open_delay(Duration::from_secs(1))
            .close_delay(Duration::from_secs_f32(0.5))
            .trigger(Button::new("slow").label("Slow Open (1000ms)").outline())
            .child(div().child("This hover card opens after 1000ms").text_sm())
    )
```

### 定位

HoverCard 支持通过 [Anchor] 设置 6 种定位：

- TopLeft
- TopCenter
- TopRight
- BottomLeft
- BottomCenter
- BottomRight

可以把卡片想象成有一个箭头尖角（像对话气泡的小三角）。anchor 指的就是这个尖角相对于触发器落在哪个点——`TopCenter` 把它放在触发器的顶部中间，`BottomRight` 放在右下角，以此类推。卡片就从这个点挂出来。

例如 `Anchor::TopLeft` 会让卡片出现在触发器正下方，并与其左对齐：

```text
[ Trigger ]
┌──────────────┐
│  Hover Card  │
└──────────────┘
```

### 使用动态内容构建器

对于较复杂的内容，可以使用 `content` builder，在 HoverCard 打开时再生成内容：

```rust
HoverCard::new("complex")
    .trigger(Button::new("btn").label("Hover me"))
    .content(|state, window, cx| {
        v_flex()
            .child("Dynamic content")
            .child(format!("Open: {}", state.is_open()))
    })
```

### 样式

HoverCard 继承了 `Styled` trait 的所有方法：

```rust
HoverCard::new("styled")
    .trigger(Button::new("btn").label("Styled"))
    .w(px(400.))
    .max_h(px(500.))
    .text_sm()
    .gap_2()
    .child("Styled content")
```

关闭默认外观并自定义样式：

```rust
HoverCard::new("custom-styled")
    .appearance(false)
    .trigger(Button::new("btn").label("Custom"))
    .bg(cx.theme().background)
    .border_2()
    .border_color(cx.theme().primary)
    .rounded(px(12.))
    .p_4()
    .child("Custom styled content")
```

## API 参考

### HoverCard 方法

- `new(id: impl Into<ElementId>)`：创建一个新的 HoverCard
- `trigger<T: IntoElement>(trigger: T)`：设置触发元素
- `content<F>(content: F)`：设置内容构建器
- `open_delay(duration: Duration)`：设置显示延迟，默认 600ms
- `close_delay(duration: Duration)`：设置隐藏延迟，默认 300ms
- `anchor(anchor: impl Into<Anchor>)`：设置定位，默认 `TopCenter`
- `on_open_change<F>(callback: F)`：打开状态变化回调
- `appearance(appearance: bool)`：是否启用默认样式，默认 `true`

### HoverCardState 方法

- `is_open() -> bool`：判断当前是否打开

## 行为细节

### Hover 时间控制

HoverCard 的时间控制主要解决悬停交互中的抖动问题：

1. **Open Delay**：防止鼠标快速扫过时意外打开
2. **Close Delay**：允许用户从 trigger 移动到内容区域时不立刻关闭
3. **Interactive Content**：只要鼠标在 trigger 或内容区域内，浮层就保持打开

### 已处理的边界场景

- **快速划过**：因为有打开延迟，不会误触发
- **从触发器移动到内容**：浮层不会马上关闭
- **频繁悬停**：通过基于 epoch 的定时器机制做了去抖
- **多个 HoverCard 同时存在**：每个 HoverCard 都维护独立状态，互不影响

## 最佳实践

1. 为不同场景设置合适的延迟。
2. HoverCard 适合预览信息，不适合承载完整流程。
3. 让触发器具备清晰的可悬停视觉反馈。
4. HoverCard 不支持键盘导航；如需键盘可达性，优先考虑 Popover。
5. 尽量避免嵌套 HoverCard，以免交互混乱。

## 与 [Popover] 的区别

| 特性 | HoverCard | Popover |
| ------------------------ | ---------------- | ------------------ |
| 触发方式 | 鼠标悬停 | 点击或右键 |
| 键盘导航 | 不支持 | 支持 |
| 点击外部关闭 | 不适用 | 支持，可配置 |
| 时间延迟 | 支持 | 不支持 |
| 主要用途 | 预览信息 | 操作和表单 |

[Popover]: ./popover.md
[Anchor]: https://docs.rs/gpui-component/latest/gpui_component/enum.Anchor.html
[Avatar]: ./avatar.md
