---
title: Tooltip
description: 在悬停或聚焦时显示提示信息，支持快捷键和自定义内容。
---

# Tooltip

Tooltip 用于在鼠标悬停或元素获得焦点时显示补充信息。它支持纯文本、自定义内容、快捷键信息以及多种触发方式，适合做解释说明、状态提示和操作说明。

## 导入

```rust
use gpui_kit::component::tooltip::Tooltip;
```

## 用法

### 纯文本 Tooltip

```rust
div()
    .child("Hover me")
    .id("basic-tooltip")
    .tooltip(|window, cx| {
        Tooltip::new("This is a helpful tooltip").build(window, cx)
    })
```

### 按钮 Tooltip

```rust
Button::new("save-btn")
    .label("Save")
    .tooltip("Save the current document")
```

### 携带快捷键信息

```rust
actions!(my_actions, [SaveDocument]);

Button::new("save-btn")
    .label("Save")
    .tooltip_with_action(
        "Save the current document",
        &SaveDocument,
        Some("MyContext")
    )
```

### 自定义内容 Tooltip

```rust
div()
    .child("Hover for rich content")
    .id("rich-tooltip")
    .tooltip(|window, cx| {
        Tooltip::element(|_, cx| {
            h_flex()
                .gap_x_1()
                .child(IconName::Info)
                .child(
                    div()
                        .child("Muted Text")
                        .text_color(cx.theme().muted_foreground)
                )
                .child(
                    div()
                        .child("Danger Text")
                        .text_color(cx.theme().danger)
                )
                .child(IconName::ArrowUp)
        })
        .build(window, cx)
    })
```

### 手动指定快捷键

```rust
div()
    .child("Custom keybinding")
    .id("custom-kb")
    .tooltip(|window, cx| {
        Tooltip::new("Delete item")
            .key_binding(Some(Kbd::new("Delete")))
            .build(window, cx)
    })
```

## API 参考

### Tooltip

| 方法 | 说明 |
| --- | --- |
| `new(text)` | 创建文本型 Tooltip |
| `element(builder)` | 创建自定义内容 Tooltip |
| `action(action, context)` | 关联 action，显示对应快捷键信息 |
| `key_binding(kbd)` | 手动设置快捷键展示 |
| `build(window, cx)` | 构建并返回 Tooltip 视图 |

### 内置 Tooltip 方法

很多组件内置了 Tooltip 支持，常见形式包括：

| 方法 | 说明 |
| --- | --- |
| `tooltip(text)` | 添加简单文本提示 |
| `tooltip_with_action(text, action, context)` | 添加带快捷键的提示 |
| `tooltip(closure)` | 使用构建器生成自定义提示 |

## 样式

Tooltip 默认会自动应用与主题匹配的样式：

- 背景：`theme.popover`
- 文字：`theme.popover_foreground`
- 边框：`theme.border`
- 阴影：中等投影
- 圆角：约 `6px`

也可以继续通过 `Styled` trait 自定义：

```rust
Tooltip::new("Custom styled tooltip")
    .bg(cx.theme().accent)
    .text_color(cx.theme().accent_foreground)
    .build(window, cx)
```

## 示例

### 工具栏提示

```rust
h_flex()
    .gap_1()
    .child(
        Button::new("new")
            .icon(IconName::Plus)
            .tooltip_with_action("Create new file", &NewFile, Some("Editor"))
    )
    .child(
        Button::new("save")
            .icon(IconName::Save)
            .tooltip_with_action("Save file", &SaveFile, Some("Editor"))
    )
```

### 表单说明

```rust
v_flex()
    .gap_4()
    .child(
        Input::new("email")
            .placeholder("Enter your email")
            .tooltip("We'll never share your email address")
    )
    .child(
        Input::new("password")
            .input_type(InputType::Password)
            .placeholder("Password")
            .tooltip("Must be at least 8 characters with special characters")
    )
```

## 最佳实践

- 提示文案应简短直接，不重复界面上已经明显存在的信息。
- Tooltip 适合做补充说明，不应用来承载关键流程信息。
- 图标按钮、缩写和危险操作尤其适合配套 Tooltip。
- 需要高频触发的 Tooltip 应尽量避免复杂内容，以减少渲染开销。
