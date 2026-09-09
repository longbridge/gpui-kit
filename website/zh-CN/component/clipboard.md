---
title: Clipboard
description: 用于复制文本或其它内容到剪贴板的按钮组件。
---

# Clipboard

Clipboard 组件提供了一个简单的复制按钮，可将文本或其它数据复制到用户剪贴板。它默认显示复制图标，在复制成功后会切换为勾选图标。组件既支持静态值，也支持通过回调动态生成复制内容。

## 导入

```rust
use gpui_kit::component::clipboard::Clipboard;
```

## 用法

### 基础 Clipboard

```rust
Clipboard::new("my-clipboard")
    .value("Text to copy")
    .on_copied(|value, window, cx| {
        window.push_notification(format!("Copied: {}", value), cx)
    })
```

### 动态值

`value_fn` 允许你在用户点击复制按钮时再动态生成内容：

- 适合依赖当前应用状态的值
- 也适合那些计算开销较大、不想在每次渲染时都计算的内容

```rust
let state = some_state.clone();
Clipboard::new("dynamic-clipboard")
    .value_fn(move |_, cx| {
        state.read(cx).get_current_value()
    })
    .on_copied(|value, window, cx| {
        window.push_notification(format!("Copied: {}", value), cx)
    })
```

### 自定义组合内容

```rust
use gpui_kit::component::label::Label;

h_flex()
    .gap_2()
    .child(Label::new("Share URL"))
    .child(Icon::new(IconName::Share))
    .child(
        Clipboard::new("custom-clipboard")
            .value("https://example.com")
    )
```

### 用在输入框中

Clipboard 很适合作为输入框后缀：

```rust
use gpui_kit::component::input::{InputState, Input};

let url_state = cx.new(|cx| InputState::new(window, cx).default_value("https://github.com"));

Input::new(&url_state)
    .suffix(
        Clipboard::new("url-clipboard")
            .value_fn({
                let state = url_state.clone();
                move |_, cx| state.read(cx).value()
            })
            .on_copied(|value, window, cx| {
                window.push_notification(format!("URL copied: {}", value), cx)
            })
    )
```

## API 参考

- [Clipboard]

## 示例

### 复制简单文本

```rust
Clipboard::new("simple")
    .value("Hello, World!")
```

### 带反馈提示

```rust
h_flex()
    .gap_2()
    .child(Label::new("Your API Key:"))
    .child(
        Clipboard::new("feedback")
            .value("sk-1234567890abcdef")
            .on_copied(|_, window, cx| {
                window.push_notification("API key copied to clipboard", cx)
            })
    )
```

### 表单字段集成

```rust
use gpui_kit::component::{
    input::{InputState, Input},
    h_flex, label::Label
};

let api_key = "sk-1234567890abcdef";

h_flex()
    .gap_2()
    .items_center()
    .child(Label::new("API Key:"))
    .child(
        Input::new(&input_state)
            .value(api_key)
            .readonly(true)
            .suffix(
                Clipboard::new("api-key-copy")
                    .value(api_key)
                    .on_copied(|_, window, cx| {
                        window.push_notification("API key copied!", cx)
                    })
            )
    )
```

### 复制动态内容

```rust
struct AppState {
    current_url: String,
}

let app_state = cx.new(|_| AppState {
    current_url: "https://example.com".to_string()
});

Clipboard::new("current-url")
    .value_fn({
        let state = app_state.clone();
        move |_, cx| {
            SharedString::from(state.read(cx).current_url.clone())
        }
    })
    .on_copied(|url, window, cx| {
        window.push_notification(format!("Shared: {}", url), cx)
    })
```

## 数据类型

Clipboard 当前主要支持复制文本字符串，内部使用 GPUI 的 `ClipboardItem::new_string()`，可处理：

- 纯文本
- UTF-8 编码内容
- 跨平台剪贴板写入

[Clipboard]: https://docs.rs/gpui-component/latest/gpui_component/clipboard/struct.Clipboard.html
