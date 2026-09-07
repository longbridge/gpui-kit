---
title: Root View
description: 使用 Root 视图为窗口启用主题、通知、对话框及其他 GPUI Component 功能。
order: -7
---

# Root View

[Root] 组件是 GPUI Component 在窗口中的根提供者。要启用 GPUI Component 的功能，必须把 [Root] 作为窗口中的 **第一层子节点**。

这一点很重要。如果不把 [Root] 放在窗口的第一层，许多行为都会出现异常或不符合预期。

```rs
fn main() {
    gpui_kit::application().run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_kit::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| Example);
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
```

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

可以在你的第一层视图中这样放置这些图层（Root > YourFirstView）：

```rs
struct MyApp;

impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child("My App Content")
            .children(Root::render_dialog_layer(cx))
            .children(Root::render_sheet_layer(cx))
            .children(Root::render_notification_layer(cx))
    }
}
```

:::tip
这里使用的是 `children` 而不是 `child`，因为当没有打开的 dialog、sheet 或 notification 时，这些方法会返回 `None`，GPUI 就不会渲染任何内容。
:::

[Root]: https://docs.rs/gpui-component/latest/gpui_component/root/struct.Root.html
