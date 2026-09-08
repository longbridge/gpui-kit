---
title: AlertDialog
description: 使用于重要确认场景的模态对话框组件。
---

# AlertDialog

AlertDialog 是一个用于中断用户并请求明确响应的模态对话框组件。它构建在 [Dialog] 之上，提供更明确的默认行为和更精简的 API，适合确认、警告和危险操作提示。

## 与 Dialog 的区别

AlertDialog 基于 Dialog 提供了以下默认值：

- 默认不允许点击遮罩关闭，可通过 `overlay_closable(true)` 修改
- 默认不显示关闭按钮，可通过 `close_button(true)` 修改
- 底部按钮居中对齐，而 Dialog 默认为右对齐
- API 更聚焦在确认和提示场景

## 导入

```rust
use gpui_kit::component::dialog::{AlertDialog, DialogAction, DialogClose};
use gpui_kit::component::WindowExt;
```

## 用法

### 配置应用根视图

与 Dialog 一样，你需要在应用根视图中渲染 dialog layer。具体可参考 [Dialog 文档](./dialog.md#setup-application-root-view)。

### 基础 AlertDialog：声明式 API

通过 `trigger` 和 `content` 创建声明式 AlertDialog：

```rust
use gpui_kit::component::dialog::{AlertDialog, DialogHeader, DialogTitle, DialogDescription, DialogFooter};

AlertDialog::new(cx)
    .trigger(
        Button::new("show-alert")
            .outline()
            .label("Show Alert")
    )
    .content(|content, _, cx| {
        content
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Are you absolutely sure?"))
                    .child(DialogDescription::new().child(
                        "This action cannot be undone. \
                        This will permanently delete your account from our servers."
                    ))
            )
            .child(
                DialogFooter::new()
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
                            .label("Continue")
                            .on_click(|_, window, cx| {
                                window.push_notification("Confirmed", cx);
                                window.close_dialog(cx);
                            })
                    )
            )
    })
```

### 使用 DialogAction 和 DialogClose

`DialogAction` 与 `DialogClose` 是包装组件，可自动触发对应按钮行为：

- `DialogClose`：触发取消操作，并调用 `on_cancel`
- `DialogAction`：触发确认操作，并调用 `on_ok`

这样就不需要手动调用 `window.close_dialog(cx)`：

```rust
AlertDialog::new(cx)
    .trigger(Button::new("show-alert").outline().label("Show Alert"))
    .on_ok(|_, window, cx| {
        window.push_notification("You confirmed!", cx);
        true
    })
    .on_cancel(|_, window, cx| {
        window.push_notification("You cancelled!", cx);
        true
    })
    .content(|content, _, cx| {
        content
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Confirm Action"))
                    .child(DialogDescription::new().child("Do you want to proceed?"))
            )
            .child(
                DialogFooter::new()
                    .child(
                        DialogClose::new().child(
                            Button::new("cancel").outline().label("Cancel")
                        )
                    )
                    .child(
                        DialogAction::new().child(
                            Button::new("ok").primary().label("Confirm")
                        )
                    )
            )
    })
```

优点：

- 不需要手动关闭对话框
- 自动连接 `on_ok` 和 `on_cancel`
- 代码更简洁
- 支持在回调返回 `false` 时阻止关闭

### 基础 AlertDialog：命令式 API

通过 `WindowExt::open_alert_dialog` 直接打开：

```rust
window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .title("Delete File")
        .description("Are you sure you want to delete this file? This action cannot be undone.")
        .show_cancel(true)
        .on_ok(|_, window, cx| {
            window.push_notification("File deleted", cx);
            true
        })
})
```

### 自定义按钮属性

```rust
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::button::ButtonVariant;

window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .title("Delete Account")
        .description("This will permanently delete your account and all associated data.")
        .button_props(
            DialogButtonProps::default()
                .ok_text("Delete")
                .ok_variant(ButtonVariant::Danger)
                .cancel_text("Keep")
                .show_cancel(true)
        )
        .on_ok(|_, window, cx| {
            window.push_notification("Account deleted", cx);
            true
        })
})
```

### 带图标的 AlertDialog

声明式写法：

```rust
use gpui_kit::component::{Icon, IconName, ActiveTheme};

AlertDialog::new(cx)
    .w(px(320.))
    .trigger(Button::new("permission").outline().label("Request Permission"))
    .on_ok(|_, window, cx| {
        window.push_notification("Permission granted", cx);
        true
    })
    .content(|content, _, cx| {
        content
            .child(
                DialogHeader::new()
                    .items_center()
                    .child(
                        Icon::new(IconName::TriangleAlert)
                            .size_10()
                            .text_color(cx.theme().warning)
                    )
                    .child(DialogTitle::new().child("Network Permission Required"))
                    .child(DialogDescription::new().child(
                        "We need your permission to access the network to provide better services."
                    ))
            )
            .child(
                DialogFooter::new()
                    .v_flex()
                    .child(
                        DialogAction::new().child(
                            Button::new("allow").w_full().primary().label("Allow")
                        )
                    )
                    .child(
                        DialogClose::new().child(
                            Button::new("deny").w_full().outline().label("Don't Allow")
                        )
                    )
            )
    })
```

命令式写法：

```rust
window.open_alert_dialog(cx, |alert, _, cx| {
    alert
        .title("Warning")
        .description("This action requires confirmation.")
        .icon(
            Icon::new(IconName::AlertTriangle)
                .size_8()
                .text_color(cx.theme().warning)
        )
})
```

### 危险操作确认

```rust
AlertDialog::new(cx)
    .trigger(
        Button::new("delete-account")
            .outline()
            .danger()
            .label("Delete Account")
    )
    .on_ok(|_, window, cx| {
        window.push_notification("Account deletion initiated", cx);
        true
    })
    .content(|content, _, _| {
        content
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Delete Account"))
                    .child(DialogDescription::new().child(
                        "This will permanently delete your account \
                        and all associated data. This action cannot be undone."
                    ))
            )
            .child(
                DialogFooter::new()
                    .child(
                        DialogClose::new().child(
                            Button::new("cancel").flex_1().outline().label("Cancel")
                        )
                    )
                    .child(
                        DialogAction::new().child(
                            Button::new("delete")
                                .flex_1()
                                .outline()
                                .danger()
                                .label("Delete Forever")
                        )
                    )
            )
    })
```

### 自定义宽度

```rust
AlertDialog::new(cx)
    .width(px(500.))
    .trigger(Button::new("custom-width").label("Custom Width"))
    .content(|content, _, _| {
        // ... dialog content
    })
```

### 控制关闭行为

#### 允许点击遮罩关闭

```rust
window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .title("Notice")
        .description("Click outside this dialog or press ESC to close it.")
        .overlay_closable(true)
})
```

#### 禁用 ESC 关闭

```rust
window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .title("Important Notice")
        .description("Please read this carefully before proceeding.")
        .keyboard(false)
})
```

#### 显示关闭按钮

```rust
window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .title("Information")
        .description("Some information...")
        .close_button(true)
})
```

### 阻止对话框关闭

如果 `on_ok` 或 `on_cancel` 返回 `false`，对话框不会关闭：

```rust
use gpui_kit::component::dialog::DialogButtonProps;

window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .title("Processing")
        .description("A process is running. Click Continue to stop it or Cancel to keep waiting.")
        .button_props(
            DialogButtonProps::default()
                .ok_text("Continue")
                .show_cancel(true)
        )
        .on_ok(|_, window, cx| {
            window.push_notification("Cannot close: Process still running", cx);
            false
        })
        .on_cancel(|_, window, cx| {
            window.push_notification("Waiting...", cx);
            false
        })
})
```

### Dialog 关闭回调

```rust
window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .title("Confirm")
        .description("Are you sure?")
        .on_close(|_, window, cx| {
            window.push_notification("Dialog closed", cx);
        })
})
```

## API 参考

### AlertDialog

| 方法 | 说明 |
| ------------------------ | ------------------------------------------------------------- |
| `new(cx)` | 创建新的 AlertDialog |
| `trigger(element)` | 设置触发对话框的元素 |
| `content(builder)` | 通过 builder 函数设置内容 |
| `title(title)` | 设置标题，命令式 API |
| `description(desc)` | 设置描述，命令式 API |
| `icon(icon)` | 设置图标，命令式 API |
| `button_props(props)` | 设置按钮文本、样式和可见性 |
| `show_cancel(bool)` | 显示或隐藏取消按钮，默认 `false` |
| `width(px)` | 设置宽度，默认 `420px` |
| `overlay_closable(bool)` | 是否允许点击遮罩关闭，默认 `false` |
| `close_button(bool)` | 是否显示关闭按钮，默认 `false` |
| `keyboard(bool)` | 是否支持 ESC 关闭，默认 `true` |
| `on_ok(callback)` | 设置确认回调，返回 `true` 时关闭 |
| `on_cancel(callback)` | 设置取消回调，返回 `true` 时关闭 |
| `on_close(callback)` | 设置关闭后的回调 |

### DialogButtonProps

| 方法 | 说明 |
| ------------------------- | ---------------------------------------- |
| `ok_text(text)` | 设置确认按钮文案，默认 `"OK"` |
| `cancel_text(text)` | 设置取消按钮文案，默认 `"Cancel"` |
| `ok_variant(variant)` | 设置确认按钮变体 |
| `cancel_variant(variant)` | 设置取消按钮变体 |
| `show_cancel(bool)` | 显示或隐藏取消按钮 |
| `on_ok(callback)` | 设置确认回调 |
| `on_cancel(callback)` | 设置取消回调 |

### DialogAction

点击其子元素时自动触发 `Confirm`，调用 AlertDialog 的 `on_ok`。

```rust
DialogAction::new().child(
    Button::new("ok").primary().label("Confirm")
)
```

行为：

- 派发 `Confirm`
- 调用 `on_ok`
- 回调返回 `true` 时关闭
- 返回 `false` 时保持打开

### DialogClose

点击其子元素时自动触发 `Cancel`，调用 AlertDialog 的 `on_cancel`。

```rust
DialogClose::new().child(
    Button::new("cancel").outline().label("Cancel")
)
```

行为：

- 派发 `Cancel`
- 调用 `on_cancel`
- 回调返回 `true` 时关闭；如果没有设置回调也会关闭
- 返回 `false` 时保持打开

## 示例

### 删除确认

命令式 API：

```rust
Button::new("delete")
    .danger()
    .label("Delete")
    .on_click(|_, window, cx| {
        window.open_alert_dialog(cx, |alert, _, _| {
            alert
                .title("Delete File?")
                .description("This action cannot be undone.")
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("Delete")
                        .ok_variant(ButtonVariant::Danger)
                        .show_cancel(true)
                )
                .on_ok(|_, window, cx| {
                    window.push_notification("File deleted", cx);
                    true
                })
        });
    })
```

声明式 API：

```rust
AlertDialog::new(cx)
    .trigger(Button::new("delete").danger().label("Delete"))
    .on_ok(|_, window, cx| {
        window.push_notification("File deleted", cx);
        true
    })
    .content(|content, _, cx| {
        content
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Delete File?"))
                    .child(DialogDescription::new().child("This action cannot be undone."))
            )
            .child(
                DialogFooter::new()
                    .child(
                        DialogClose::new().child(
                            Button::new("cancel").outline().label("Cancel")
                        )
                    )
                    .child(
                        DialogAction::new().child(
                            Button::new("delete-confirm").danger().label("Delete")
                        )
                    )
            )
    })
```

### 会话超时

```rust
window.open_alert_dialog(cx, |alert, _, _| {
    alert
        .content(|content, _, _| {
            content
                .child(
                    DialogHeader::new()
                        .items_center()
                        .child(DialogTitle::new().child("Session Expired"))
                        .child(DialogDescription::new().child(
                            "Your session has expired due to inactivity. \
                            Please log in again to continue."
                        ))
                )
                .child(
                    DialogFooter::new()
                        .child(
                            Button::new("sign-in")
                                .label("Sign in")
                                .primary()
                                .flex_1()
                                .on_click(|_, window, cx| {
                                    window.push_notification("Redirecting to login...", cx);
                                    window.close_dialog(cx);
                                })
                        )
                )
        })
})
```

### 有更新可用

```rust
AlertDialog::new(cx)
    .trigger(Button::new("update").outline().label("Update Available"))
    .on_cancel(|_, window, cx| {
        window.push_notification("Update postponed", cx);
        true
    })
    .on_ok(|_, window, cx| {
        window.push_notification("Starting update...", cx);
        true
    })
    .content(|content, _, _| {
        content
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Update Available"))
                    .child(DialogDescription::new().child(
                        "A new version (v2.0.0) is available. \
                        This update includes new features and bug fixes."
                    ))
            )
            .child(
                DialogFooter::new()
                    .child(
                        DialogClose::new().child(
                            Button::new("later").flex_1().outline().label("Later")
                        )
                    )
                    .child(
                        DialogAction::new().child(
                            Button::new("update-now").flex_1().primary().label("Update Now")
                        )
                    )
            )
    })
```

## 最佳实践

1. 简单确认场景优先使用命令式 `open_alert_dialog`。
2. 复杂布局或需要和其它组件联动时使用声明式 `trigger` + `content`。
3. 优先使用 `DialogAction` 和 `DialogClose`，而不是手动关闭对话框。
4. 对危险操作使用明确的按钮样式和文案。
5. 对结果不可逆的操作提供清晰说明。
6. 只有在确有必要时才阻止对话框关闭。
7. 保持整个应用中的按钮顺序和交互一致。

## 相关组件

- [Dialog]
- [DialogHeader]
- [DialogTitle]
- [DialogDescription]
- [DialogFooter]
- [DialogAction]
- [DialogClose]

[AlertDialog]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.AlertDialog.html
[Dialog]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.Dialog.html
[DialogHeader]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.DialogHeader.html
[DialogTitle]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.DialogTitle.html
[DialogDescription]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.DialogDescription.html
[DialogFooter]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.DialogFooter.html
[DialogAction]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.DialogAction.html
[DialogClose]: https://docs.rs/gpui-component/latest/gpui_component/dialog/struct.DialogClose.html
