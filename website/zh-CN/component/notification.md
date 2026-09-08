---
title: Notification
description: 在窗口右上角显示支持自动消失的 toast 通知。
---

# Notification

Notification 是一个 toast 通知系统，用于向用户显示短暂消息。通知会出现在窗口右上角，并可在超时后自动消失。它支持多种类型、标题、自定义内容和操作按钮，适合状态反馈、确认信息和异步操作提示。

## 导入

```rust
use gpui_kit::component::{
    notification::{Notification, NotificationType},
    WindowExt
};
```

## 用法

### 在根视图中渲染通知层

如果你想显示通知，需要在应用根视图中渲染 notification layer。

[Root::render_notification_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_notification_layer) 会将当前激活的通知渲染在应用内容之上。

```rust
use gpui_kit::component::{TitleBar, Root};

struct Example {}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(TitleBar::new())
                    .child(div().flex_1().child("Hello world!")),
            )
            .children(notification_layer)
    }
}
```

### 基础通知

```rust
window.push_notification("This is a notification.", cx);

Notification::new()
    .message("Your changes have been saved.")
```

### 通知类型

```rust
window.push_notification(
    (NotificationType::Info, "File saved successfully."),
    cx,
);

window.push_notification(
    (NotificationType::Success, "Payment processed successfully."),
    cx,
);

window.push_notification(
    (NotificationType::Warning, "Network connection is unstable."),
    cx,
);

window.push_notification(
    (NotificationType::Error, "Failed to save file. Please try again."),
    cx,
);
```

### 带标题

```rust
Notification::new()
    .title("Update Available")
    .message("A new version of the application is ready to install.")
    .with_type(NotificationType::Info)
```

### 自动隐藏

```rust
Notification::new()
    .message("This notification stays until manually closed.")
    .autohide(false)

Notification::new()
    .message("This will disappear automatically.")
    .autohide(true)
```

指针悬停在通知上或某条通知获得键盘焦点时倒计时暂停，指针移开或焦点离开后继续。窗口未激活时倒计时照常进行，不能错过的消息应关闭自动隐藏或使用系统通知投递。

### 操作按钮

```rust
Notification::new()
    .title("Connection Lost")
    .message("Unable to connect to server.")
    .with_type(NotificationType::Error)
    .autohide(false)
    .action(|_, cx| {
        Button::new("retry")
            .primary()
            .label("Retry")
            .on_click(cx.listener(|this, _, window, cx| {
                println!("Retrying connection...");
                this.dismiss(window, cx);
            }))
    })
```

### 可点击通知

```rust
Notification::new()
    .message("Click to view details")
    .on_click(cx.listener(|_, _, _, cx| {
        println!("Notification clicked");
        cx.notify();
    }))
```

### 自定义内容

```rust
use gpui_kit::component::text::markdown;

let markdown_content = r#"
## Custom Notification
- **Feature**: New dashboard available
- **Status**: Ready to use
- [Learn more](https://example.com)
"#;

Notification::new()
    .content(|_, window, cx| {
        markdown(markdown_content).into_any_element()
    })
```

### 唯一通知 ID

如果你要手动管理通知，例如长任务状态或持久警告，可以为通知分配唯一 ID。

```rust
struct UpdateNotification;

Notification::new()
    .id::<UpdateNotification>()
    .message("System update available")
    .autohide(false)

struct TaskNotification;

Notification::warning("Task failed to complete")
    .id1::<TaskNotification>("task-123")
    .title("Task Failed")
```

后续可以通过：

```rust
window.remove_notification::<UpdateNotification>(cx);
```

来移除对应通知。

### 系统通知

通知也可以投递到操作系统的通知中心。使用 `NotificationDelivery` 选择通知的去向：应用内 toast（`InApp`，默认）、系统通知中心（`System`）、或两者都发（`InAppAndSystem`）。

```rust
use gpui_kit::component::notification::{Notification, NotificationDelivery};

// 单条通知覆盖；`.system()` 和 `.in_app_and_system()` 是
// `.delivery(NotificationDelivery::...)` 的简写。
Notification::info("Your download is ready.")
    .title("Download complete")
    .system()

// 或为所有通知设置全局默认值
Theme::global_mut(cx).notification.delivery = NotificationDelivery::InAppAndSystem;
```

通知的标题和消息分别成为系统通知的标题和正文；两者都缺失时不会投递。用相同的 `.id::<T>()` 再次推送会替换之前的系统通知，`window.remove_notification::<T>(cx)` / `window.clear_notifications(cx)` 会将其撤回。toast 自动隐藏时，系统通知会保留在通知中心。

点击系统通知会激活应用及其窗口、关闭对应的应用内 toast（如有）、并以默认的 `ClickEvent` 触发 `on_click`。`NotificationDelivery::System` 模式下没有 toast，因此 `on_close` 不会被调用。

`gpui_kit::component::init` 会注册应用级的 `on_system_notification_response` 处理器，之后请勿再自行注册——gpui 只保留一个。应用通过 `cx.show_system_notification` 直接发送的系统通知不受影响。

平台要求：

| 平台 | 要求 | 撤回 |
| --- | --- | --- |
| macOS | 必须从可信位置（如 `/Applications`）的打包 `.app` 运行；`cargo run` 裸跑时静默禁用。首次投递会触发系统授权弹窗，拒绝后系统会记住该选择，后续投递静默失败 | 支持 |
| Windows | 启动早期调用 `cx.set_app_identity(identifier, name)` | 支持 |
| Linux | 需要 XDG 通知守护进程 | 不支持（自然过期） |

## 示例

### 表单校验失败

```rust
Notification::error("Please correct the following errors before submitting.")
    .title("Validation Failed")
    .autohide(false)
```

### 文件上传进度

```rust
struct UploadNotification;

window.push_notification(
    Notification::info("Uploading file...")
        .id::<UploadNotification>()
        .title("File Upload")
        .autohide(false),
    cx,
);
```
