---
title: Collapsible
description: 可展开和收起内容的交互式组件。
---

# Collapsible

Collapsible 是一个用于展开和收起内容的交互式组件。

## 导入

```rust
use gpui_kit::component::collapsible::Collapsible;
```

## 用法

### 基础用法

```rust
Collapsible::new()
    .max_w_128()
    .gap_1()
    .open(self.open)
    .child(
        "This is a collapsible component. \
        Click the header to expand or collapse the content.",
    )
    .content(
        "This is the full content of the Collapsible component. \
        It is only visible when the component is expanded. \n\
        You can put any content you like here, including text, images, \
        or other UI elements.",
    )
    .child(
        h_flex().justify_center().child(
            Button::new("toggle1")
                .icon(IconName::ChevronDown)
                .label("Show more")
                .when(open, |this| {
                    this.icon(IconName::ChevronUp).label("Show less")
                })
                .xsmall()
                .link()
                .on_click({
                    cx.listener(move |this, _, _, cx| {
                        this.open = !this.open;
                        cx.notify();
                    })
                }),
        ),
    )
```

可以通过 `open` 方法控制当前是否展开。若值为 `false`，则通过 `content` 添加的子内容会被隐藏。

### 展开动画

使用稳定 motion ID 可选择启用支持中途反向的测量式高度展开：

```rust
Collapsible::new()
    .motion_id("advanced-options")
    .open(self.open)
    .content(options)
```

启用后，内容在关闭时仍保持挂载，以便测量自然高度，并可在动画途中切换时立即反向。不调用 `motion_id` 时仍使用即时挂载/卸载行为。timing、reduced motion 和性能细节见 [GPUI Base 动画与动效](/zh-CN/base/motion)。

[Collapsible]: https://docs.rs/gpui-component/latest/gpui_component/collapsible/struct.Collapsible.html
