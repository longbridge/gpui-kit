---
title: GroupBox
description: 带可选标题的分组容器组件，用于组织相关内容。
---

# GroupBox

GroupBox 是一个用于组织相关内容的容器组件，支持标题、边框、背景和自定义样式，适合表单分区、设置面板和语义分组场景。

## 导入

```rust
use gpui_kit::component::group_box::{GroupBox, GroupBoxVariant, GroupBoxVariants as _};
```

## 用法

### 基础 GroupBox

```rust
GroupBox::new()
    .child("Subscriptions")
    .child(Checkbox::new("all").label("All"))
    .child(Checkbox::new("newsletter").label("Newsletter"))
    .child(Button::new("save").primary().label("Save"))
```

### 不同变体

```rust
GroupBox::new()
    .child("Content without visual container")

GroupBox::new()
    .fill()
    .title("Settings")
    .child("Content with background")

GroupBox::new()
    .outline()
    .title("Preferences")
    .child("Content with border")
```

### 带标题

```rust
GroupBox::new()
    .fill()
    .title("Account Settings")
    .child(
        h_flex()
            .justify_between()
            .child("Make profile private")
            .child(Switch::new("privacy").checked(false))
    )
    .child(Button::new("save").primary().label("Save Changes"))
```

### 自定义 ID

```rust
GroupBox::new()
    .id("user-preferences")
    .outline()
    .title("User Preferences")
    .child("Preference controls...")
```

### 自定义标题样式

```rust
use gpui_kit::{StyleRefinement, relative};

GroupBox::new()
    .outline()
    .title("Custom Title")
    .title_style(
        StyleRefinement::default()
            .font_semibold()
            .line_height(relative(1.0))
            .px_3()
            .text_color(cx.theme().accent)
    )
    .child("Content with custom title styling")
```

### 自定义内容区域样式

```rust
GroupBox::new()
    .fill()
    .title("Custom Content Area")
    .content_style(
        StyleRefinement::default()
            .rounded_xl()
            .py_3()
            .px_4()
            .border_2()
            .border_color(cx.theme().accent)
    )
    .child("Content with custom styling")
```

### 复杂示例

```rust
GroupBox::new()
    .id("notification-settings")
    .outline()
    .bg(cx.theme().group_box)
    .rounded_xl()
    .p_5()
    .title("Notification Preferences")
    .title_style(
        StyleRefinement::default()
            .font_semibold()
            .line_height(relative(1.0))
            .px_3()
    )
    .content_style(
        StyleRefinement::default()
            .rounded_xl()
            .py_3()
            .px_4()
            .border_2()
    )
    .child(
        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .child("Email notifications")
                    .child(Switch::new("email").checked(true))
            )
            .child(
                h_flex()
                    .justify_between()
                    .child("Push notifications")
                    .child(Switch::new("push").checked(false))
            )
            .child(
                h_flex()
                    .justify_between()
                    .child("SMS notifications")
                    .child(Switch::new("sms").checked(false))
            )
    )
    .child(
        h_flex()
            .justify_end()
            .gap_2()
            .child(Button::new("cancel").label("Cancel"))
            .child(Button::new("save").primary().label("Save Settings"))
    )
```

## 示例

### 表单分区

```rust
GroupBox::new()
    .fill()
    .title("Personal Information")
    .child(
        v_flex()
            .gap_4()
            .child(
                h_flex()
                    .gap_2()
                    .child(Input::new("first-name").placeholder("First Name"))
                    .child(Input::new("last-name").placeholder("Last Name"))
            )
            .child(Input::new("email").placeholder("Email Address"))
            .child(
                h_flex()
                    .justify_end()
                    .child(Button::new("update").primary().label("Update Profile"))
            )
    )
```

### 设置面板

```rust
GroupBox::new()
    .outline()
    .title("Display Settings")
    .child(
        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .child(Label::new("Theme"))
                    .child(
                        RadioGroup::horizontal("theme")
                            .child(Radio::new("light").label("Light"))
                            .child(Radio::new("dark").label("Dark"))
                            .child(Radio::new("auto").label("Auto"))
                    )
            )
            .child(
                h_flex()
                    .justify_between()
                    .child(Label::new("Font Size"))
                    .child(
                        Select::new("font-size")
                            .option("small", "Small")
                            .option("medium", "Medium")
                            .option("large", "Large")
                    )
            )
    )
```

### 邮件订阅管理

```rust
GroupBox::new()
    .title("Email Subscriptions")
    .child(
        v_flex()
            .gap_2()
            .child(Checkbox::new("newsletter").label("Weekly Newsletter"))
            .child(Checkbox::new("updates").label("Product Updates"))
            .child(Checkbox::new("security").label("Security Alerts"))
            .child(Checkbox::new("marketing").label("Marketing Communications"))
    )
    .child(
        h_flex()
            .justify_between()
            .mt_4()
            .child(Button::new("unsubscribe-all").link().label("Unsubscribe All"))
            .child(Button::new("save").primary().label("Update Preferences"))
    )
```

### 无标题分组

```rust
GroupBox::new()
    .outline()
    .child(
        h_flex()
            .justify_between()
            .items_center()
            .child("Enable two-factor authentication")
            .child(Switch::new("2fa").checked(false))
    )
```

## 样式

GroupBox 既支持内置变体，也支持自定义样式。

### 主题集成

```rust
GroupBox::new()
    .fill()
    .bg(cx.theme().group_box)
    .title("Themed Group Box")
```

### 自定义外观

```rust
GroupBox::new()
    .outline()
    .border_2()
    .border_color(cx.theme().accent)
    .rounded(cx.theme().radius_lg)
    .title("Custom Styled Group Box")
    .title_style(
        StyleRefinement::default()
            .text_color(cx.theme().accent)
            .font_bold()
    )
```

## 最佳实践

1. 对明确分组的表单项使用标题。
2. 主要内容分区可用 `fill()`，次级分区可用 `outline()`。
3. 用 GroupBox 建立清晰层级，但避免视觉过载。
4. 只把逻辑相关的内容放进同一个分组。
5. 组件会处理内部间距，但外部间距仍需要按页面布局控制。
6. GroupBox 能较好适配不同容器宽度和响应式布局。

## 相关组件

- **Form**：可在表单中用 GroupBox 做分区
- **Dialog**：适合在对话框中组织内容
- **Accordion**：需要可折叠分组时可考虑使用
- **Card**：需要更强视觉容器感时可考虑 Card
