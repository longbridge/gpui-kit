---
title: Alert
description: 用于吸引用户注意的重要提示组件。
---

# Alert

Alert 是一个通用提示组件，用于展示重要消息。它支持多种变体、可选标题、自定义图标、可关闭行为以及横幅模式，适合通知、状态提示和操作反馈场景。

## 导入

```rust
use gpui_kit::component::alert::Alert;
```

## 用法

### 基础 Alert

```rust
Alert::new("alert-id", "This is a basic alert message.")
```

### 带标题

```rust
Alert::new("alert-with-title", "Your changes have been saved successfully.")
    .title("Success!")
```

### 不同变体

```rust
Alert::info("info-alert", "This is an informational message.")
    .title("Information")

Alert::success("success-alert", "Your operation completed successfully.")
    .title("Success!")

Alert::warning("warning-alert", "Please review your settings before proceeding.")
    .title("Warning")

Alert::error("error-alert", "An error occurred while processing your request.")
    .title("Error")
```

### Alert 尺寸

```rust
use gpui_kit::component::{alert::Alert, Sizable as _};

Alert::info("alert", "Message content")
    .xsmall()
    .title("XSmall Alert")
Alert::info("alert", "Message content")
    .small()
    .title("Small Alert")

Alert::info("alert", "Message content")
    .title("Medium Alert")

Alert::info("alert", "Message content")
    .large()
    .title("Large Alert")
```

### 可关闭提示

只要设置 `on_close`，Alert 就会显示关闭按钮：

```rust
Alert::info("closable-alert", "This alert can be dismissed.")
    .title("Dismissible")
    .on_close(|_event, _window, _cx| {
        println!("Alert was closed");
    })
```

### 横幅模式

横幅模式会占满可用宽度，并且不显示标题：

```rust
Alert::info("banner-alert", "This is a banner alert that spans the full width.")
    .banner()

Alert::success("banner-success", "Operation completed successfully!")
    .banner()

Alert::warning("banner-warning", "System maintenance scheduled for tonight.")
    .banner()

Alert::error("banner-error", "Service temporarily unavailable.")
    .banner()
```

### 自定义图标

```rust
use gpui_kit::component::IconName;

Alert::new("custom-icon", "Meeting scheduled for tomorrow at 3 PM.")
    .title("Calendar Reminder")
    .icon(IconName::Calendar)
```

### 使用 Markdown 内容

可以配合 `TextView` 渲染 Markdown 或 HTML 内容：

```rust
use gpui_kit::component::text::markdown;

Alert::error(
    "error-with-markdown",
    markdown(
        "Please verify your billing information and try again.\n\
        - Check your card details\n\
        - Ensure sufficient funds\n\
        - Verify billing address"
    ),
)
.title("Payment Failed")
```

### 条件显示

```rust
Alert::info("conditional-alert", "This alert may be hidden.")
    .title("Conditional")
    .visible(should_show_alert)
```

## API 参考

- [Alert]

## 示例

### 表单校验错误

```rust
Alert::error(
    "validation-error",
    "Please correct the following errors before submitting:\n\
    - Email address is required\n\
    - Password must be at least 8 characters\n\
    - Terms of service must be accepted"
)
.title("Validation Failed")
```

### 成功提示

```rust
Alert::success("save-success", "Your profile has been updated successfully.")
    .title("Changes Saved")
    .on_close(|_, _, _| {
        // Auto-dismiss after showing
    })
```

### 系统状态横幅

```rust
Alert::warning(
    "maintenance-banner",
    "Scheduled maintenance will occur tonight from 2:00 AM to 4:00 AM EST. \
    Some services may be temporarily unavailable."
)
.banner()
.large()
```

### 交互式提示

```rust
Alert::info("update-available", "A new version of the application is available.")
    .title("Update Available")
    .icon(IconName::Download)
    .on_close(cx.listener(|this, _, _, cx| {
        this.handle_update_notification(cx);
    }))
```

### 多行格式化内容

```rust
use gpui_kit::component::text::markdown;

Alert::warning(
    "security-alert",
    markdown(
        "**Security Notice**: Unusual activity detected on your account.\n\n\
        Recent activity:\n\
        - Login from new device (Chrome on Windows)\n\
        - Location: San Francisco, CA\n\
        - Time: Today at 2:30 PM\n\n\
        If this wasn't you, please [change your password](/) immediately."
    )
)
.title("Security Alert")
.icon(IconName::Shield)
```

[Alert]: https://docs.rs/gpui-component/latest/gpui_component/alert/struct.Alert.html
