---
title: Root View
description: 使用 Root 视图为窗口启用主题、通知、对话框及其他 GPUI Component 功能。
example: false
---

# Root View

[Root] 组件是 GPUI Component 在窗口中的根提供者。要启用 GPUI Component 的功能，必须把 [Root] 作为窗口中的 **第一层子节点**。

这一点很重要。如果不把 [Root] 放在窗口的第一层，许多行为都会出现异常或不符合预期。

下面这份完整的 **Tested consumer recipe** 在隔离的 `gpui-kit` 消费者工作区中编译。它会在创建窗口前初始化 GPUI Kit，将 `Root` 作为窗口的第一层视图，并渲染全部 Root 浮层。

<!-- recipe:bootstrap:start -->
```rust
use gpui_kit::component::Root;
use gpui_kit::{
    AppContext as _, Context, IntoElement, ParentElement as _, Render, Styled as _, Window,
    WindowOptions, div,
};

pub fn run() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            cx.spawn(async move |cx| {
                cx.open_window(WindowOptions::default(), |window, cx| {
                    let view = cx.new(|_| BootstrapView);
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("failed to open window");
            })
            .detach();
        });
}

struct BootstrapView;

impl Render for BootstrapView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child("My application")
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
```
<!-- recipe:bootstrap:end -->

## 窗口边框

默认情况下，[Root] 会渲染 GPUI Component 的客户端窗口边框包装层。`layer-shell` 全屏窗口等场景不应渲染这层边框，可以使用 `bordered(false)` 关闭：

```rs
cx.new(|cx| Root::new(view, window, cx).bordered(false))
```

## 浮层

对话框、抽屉、通知等 UI 都需要一个统一的展示层，[Root] 提供了这些浮层的渲染入口：

- [Root::render_dialog_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_dialog_layer) - 渲染当前打开的对话框
- [Root::render_sheet_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_sheet_layer) - 渲染当前打开的抽屉
- [Root::render_notification_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_notification_layer) - 渲染通知列表

在 `Root` 之下的第一层视图的 `render` 方法中放置这些图层；上方经过测试的 recipe 展示了所需的 `window, cx` 参数。

:::tip
这里使用的是 `children` 而不是 `child`，因为当没有打开的 dialog、sheet 或 notification 时，这些方法会返回 `None`，GPUI 就不会渲染任何内容。
:::

[Root]: https://docs.rs/gpui-component/latest/gpui_component/root/struct.Root.html
