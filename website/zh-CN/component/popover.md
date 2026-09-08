---
title: Popover
description: 相对于触发元素显示富内容的浮动层组件。
---

# Popover

Popover 用于在触发元素附近展示浮动内容。它支持多种定位方式、自定义内容、不同触发方式以及自动关闭行为，适合用来实现提示卡片、上下文菜单、小表单和局部操作面板。

## 导入

```rust
use gpui_kit::component::popover::{Popover};
```

## 用法

### 基础 Popover

:::info
任何实现了 [Selectable] 的元素都可以作为触发器，例如 [Button]。

任何实现了 [RenderOnce] 或 [Render] 的元素都可以作为 Popover 内容，可以直接通过 `.child(...)` 添加。
:::

```rust
use gpui_kit::ParentElement as _;
use gpui_kit::component::{button::Button, popover::Popover};

Popover::new("basic-popover")
    .trigger(Button::new("trigger").label("Click me").outline())
    .child("Hello, this is a popover!")
    .child("It appears when you click the button.")
```

### 自定义定位

`anchor` 方法用于控制 Popover 如何贴合触发器，使用 [`Anchor`] 类型。

可以把 Popover 想象成有一个箭头尖角（像对话气泡的小三角）。anchor 指的就是这个尖角相对于触发器落在哪个点——`Anchor::TopLeft` 把它放在触发器的左上角，`Anchor::BottomRight` 放在右下角，以此类推。Popover 就从这个点挂出来。

例如 `Anchor::TopLeft` 会让 Popover 出现在触发器正下方，并与其左对齐：

```text
[ Trigger ]
┌──────────────┐
│   Popover    │
└──────────────┘
```

```rust
use gpui_kit::component::Anchor;

Popover::new("top-center")
    .anchor(Anchor::TopCenter)
    .trigger(Button::new("btn").label("Top Center").outline())
    .child("Anchored to the trigger's top-center")
```

### 在 Popover 中渲染 View

你也可以把实现了 [Render] 的 `Entity<T>` 作为 Popover 内容：

```rust
let view = cx.new(|_| MyView::new());

Popover::new("form-popover")
    .anchor(Anchor::BottomLeft)
    .trigger(Button::new("show-form").label("Open Form").outline())
    .child(view.clone())
```

### 使用 `content` 构造动态内容

如果你需要根据状态动态构造内容，或者希望在闭包中拿到 Popover 的上下文，可以使用 `content`：

```rust
use gpui_kit::ParentElement as _;
use gpui_kit::component::popover::Popover;

Popover::new("complex-popover")
    .anchor(Anchor::BottomLeft)
    .trigger(Button::new("complex").label("Complex Content").outline())
    .content(|_, _, _| {
        div()
            .child("This popover has complex content.")
            .child(
                Button::new("action-btn")
                    .label("Perform Action")
                    .outline()
            )
    })
```

:::warning
`content` 回调会在每次渲染 Popover 时执行，因此不要在闭包里频繁创建重量级对象或进行高成本计算。
:::

### 右键触发

如果你想把 Popover 当作自定义上下文菜单来用，可以指定鼠标按键：

```rust
use gpui_kit::MouseButton;

Popover::new("context-menu")
    .anchor(Anchor::BottomRight)
    .mouse_button(MouseButton::Right)
    .trigger(Button::new("right-click").label("Right Click Me").outline())
    .child("Context Menu")
    .child(Separator::horizontal())
    .child("This is a custom context menu.")
```

### 手动关闭

如果你希望在内容内部主动关闭 Popover，可以发出 `DismissEvent`：

```rust
use gpui_kit::component::{DismissEvent, popover::Popover};

Popover::new("dismiss-popover")
    .trigger(Button::new("dismiss").label("Dismiss Popover").outline())
    .content(|_, cx| {
        div()
            .child("Click the button below to dismiss this popover.")
            .child(
                Button::new("close-btn")
                    .label("Close Popover")
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.emit(DismissEvent);
                    }))
            )
    })
```

### 自定义样式

Popover 同样支持 `appearance(false)` 来关闭默认样式，并通过 [Styled] trait 完整自定义外观：

```rust
Popover::new("custom-popover")
    .appearance(false)
    .trigger(Button::new("custom").label("Custom Style"))
    .bg(cx.theme().accent)
    .text_color(cx.theme().accent_foreground)
    .p_6()
    .rounded_xl()
    .shadow_2xl()
    .child("Fully custom styled popover")
```

### 受控打开状态

通过 `open` 和 `on_open_change`，你可以把 Popover 的开关状态交给外部状态管理：

```rust
use gpui_kit::component::popover::Popover;

struct MyView {
    popover_open: bool,
}

Popover::new("controlled-popover")
    .open(self.open)
    .on_open_change(cx.listener(|this, open: &bool, _, cx| {
        this.popover_open = *open;
        cx.notify();
    }))
    .trigger(Button::new("control-btn").label("Control Popover").outline())
    .child("This popover's open state is controlled programmatically.")
```

### 默认打开

如果只想设置首次渲染时默认打开，可以使用 `default_open(true)`：

```rust
use gpui_kit::component::popover::Popover;

Popover::new("default-open-popover")
    .default_open(true)
    .trigger(Button::new("default-open-btn").label("Default Open").outline())
    .child("This popover is open by default when first rendered.")
```

[Button]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.Button.html
[Selectable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Selectable.html
[Render]: https://docs.rs/gpui/latest/gpui/trait.Render.html
[RenderOnce]: https://docs.rs/gpui/latest/gpui/trait.RenderOnce.html
[Styled]: https://docs.rs/gpui/latest/gpui/trait.Styled.html
[`Anchor`]: https://docs.rs/gpui-component/latest/gpui_component/enum.Anchor.html
