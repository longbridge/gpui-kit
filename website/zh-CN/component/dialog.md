---
title: Dialog
description: 在应用内容上方显示浮层内容的对话框组件。
---

# Dialog

Dialog 用于创建普通对话框、确认框和提示弹窗。它支持遮罩层、键盘快捷键以及多种自定义能力。

## 导入

```rust
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::WindowExt;
```

## 用法

### 在应用根视图中渲染 Dialog 图层

如果你要展示对话框，需要在应用根视图中渲染 dialog layer。通常这会放在主应用结构体的 `render` 方法里。

[Root::render_dialog_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_dialog_layer) 会把当前激活的对话框渲染在应用内容之上。

```rust
use gpui_kit::component::TitleBar;

struct MyApp {
    view: AnyView,
}

impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(window, cx);

        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(TitleBar::new())
                    .child(div().flex_1().overflow_hidden().child(self.view.clone())),
            )
            .children(dialog_layer)
    }
}
```

### 基础对话框

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("Welcome")
        .child("This is a dialog dialog.")
})
```

### 表单对话框

```rust
let input = cx.new(|cx| InputState::new(window, cx));

window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("User Information")
        .child(
            v_flex()
                .gap_3()
                .child("Please enter your details:")
                .child(Input::new(&input))
        )
        .footer(|_, _, _, _| {
            vec![
                Button::new("ok")
                    .primary()
                    .label("Submit")
                    .on_click(|_, window, cx| {
                        window.close_dialog(cx);
                    }),
                Button::new("cancel")
                    .label("Cancel")
                    .on_click(|_, window, cx| {
                        window.close_dialog(cx);
                    }),
            ]
        })
})
```

### 带图标的对话框

```rust
window.open_dialog(cx, |dialog, _, cx| {
    dialog
        .child(
            h_flex()
                .gap_3()
                .child(Icon::new(IconName::TriangleAlert)
                    .size_6()
                    .text_color(cx.theme().warning))
                .child("This action cannot be undone.")
        )
})
```

### 可滚动内容

```rust
use gpui_kit::component::text::markdown;

window.open_dialog(cx, |dialog, window, cx| {
    dialog
        .h(px(450.))
        .title("Long Content")
        .child(markdown(long_markdown_text))
})
```

### 常用选项

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("Custom Dialog")
        .overlay(true)
        .overlay_closable(true)
        .keyboard(true)
        .close_button(false)
        .child("Dialog content")
})
```

### 嵌套对话框

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("First Dialog")
        .child("This is the first dialog")
        .footer(|_, _, _, _| {
            vec![
                Button::new("open-another")
                    .label("Open Another Dialog")
                    .on_click(|_, window, cx| {
                        window.open_dialog(cx, |dialog, _, _| {
                            dialog
                                .title("Second Dialog")
                                .child("This is nested")
                        });
                    }),
            ]
        })
})
```

### 自定义样式

```rust
window.open_dialog(cx, |dialog, _, cx| {
    dialog
        .rounded(cx.theme().radius_lg)
        .bg(cx.theme().cyan)
        .text_color(cx.theme().info_foreground)
        .title("Custom Style")
        .child("Styled dialog content")
})
```

### 自定义内边距

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .p_3()
        .title("Custom Padding")
        .child("Dialog with custom spacing")
})
```

### 代码中主动关闭

```rust
window.close_dialog(cx);

Button::new("submit")
    .primary()
    .label("Submit")
    .on_click(|_, window, cx| {
        window.close_dialog(cx);
    })
```

## 声明式 API

现在 Dialog 也支持声明式写法，可以通过 header、title、description、footer 等组件来组织内容。

### 导入

```rust
use gpui_kit::component::dialog::{
    Dialog, DialogHeader, DialogTitle, DialogDescription, DialogFooter,
};
```

### 触发器模式

```rust
Dialog::new(cx)
    .trigger(
        Button::new("open-dialog")
            .outline()
            .label("Open Dialog")
    )
    .content(|content, _, cx| {
        content
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Account Created"))
                    .child(DialogDescription::new().child(
                        "Your account has been created successfully!",
                    ))
            )
            .child(
                DialogFooter::new()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().muted)
                    .child(
                        Button::new("cancel")
                            .outline()
                            .label("Cancel")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            })
                    )
                    .child(
                        Button::new("ok")
                            .primary()
                            .label("Save Changes")
                    )
            )
    })
```

### 内容构建器模式

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .w(px(400.))
        .content(|content, _, _| {
            content
                .child(
                    DialogHeader::new()
                        .child(DialogTitle::new().child("Custom Width"))
                        .child(DialogDescription::new().child(
                            "This dialog has a custom width of 400px.",
                        ))
                )
                .child(div().child(
                    "Content area with custom width configuration."
                ))
                .child(
                    DialogFooter::new()
                        .justify_center()
                        .child(
                            Button::new("cancel")
                                .flex_1()
                                .outline()
                                .label("Cancel")
                                .on_click(|_, window, cx| {
                                    window.close_dialog(cx);
                                })
                        )
                        .child(
                            Button::new("done")
                                .flex_1()
                                .primary()
                                .label("Done")
                                .on_click(|_, window, cx| {
                                    window.close_dialog(cx);
                                })
                        )
                )
        })
})
```

### 相关子组件

#### DialogHeader

用于容纳标题和描述区域：

```rust
DialogHeader::new()
    .child(DialogTitle::new().child("Title"))
    .child(DialogDescription::new().child("Description"))
```

#### DialogTitle

用于显示主标题：

```rust
DialogTitle::new()
    .child("Account Settings")
```

#### DialogDescription

用于显示标题下方的说明文字：

```rust
DialogDescription::new()
    .child("Update your account settings and preferences here.")
```

#### DialogFooter

用于放置底部操作按钮和页脚内容：

```rust
DialogFooter::new()
    .bg(cx.theme().muted)
    .border_t_1()
    .border_color(cx.theme().border)
    .child(Button::new("cancel").outline().label("Cancel"))
    .child(Button::new("save").primary().label("Save"))
```

## API 变化

### Dialog::new() 签名变化

`Dialog::new()` 现在不再需要 `window` 参数：

```rust
// Old API (deprecated)
Dialog::new(window, cx)

// New API
Dialog::new(cx)
```

### content 构建方式变化

`.content()` 现在接收 builder function，而不是预先构造好的 `DialogContent`：

```rust
// Old approach (still works)
dialog.child(DialogHeader::new()...)

// New declarative API
dialog.content(|content, window, cx| {
    content
        .child(DialogHeader::new()...)
        .child(DialogFooter::new()...)
})
```

## 最佳实践

1. 优先使用 `DialogHeader`、`DialogTitle`、`DialogDescription` 和 `DialogFooter`
2. 简单场景优先用 trigger 模式
3. 复杂逻辑或复杂状态更适合 `window.open_dialog` + builder
4. 尽量保持语义结构完整，标题和说明建议成对出现
5. 所有操作按钮尽量放在 `DialogFooter` 中保持一致性
6. 对内容尺寸敏感的弹窗，建议显式设置宽度
