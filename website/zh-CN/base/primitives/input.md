---
title: Input
description: 支持掩码、验证和数字步进的无样式单行文本输入。
order: 14
---

# Input

`Input` 是 `gpui-base` 的单行文本控件。它负责编辑、焦点、选择、键盘输入、IME、掩码、验证与事件，表现由应用提供。普通多行文本使用 [Textarea](./textarea.md)，源代码使用 [Editor](./editor.md)。

## 导入

```rust
use gpui_kit::base::input::{Input, InputEvent, InputState};
```

## 基本用法

持久状态只创建一次，再用对应 entity 渲染 `Input`：

```rust
let input = cx.new(|cx| {
    InputState::new(window, cx)
        .placeholder("Account name")
        .default_value("Ada")
});

Input::new(&input)
```

通过状态读取和更新值：

```rust
let value = input.read(cx).value();
input.update(cx, |state, cx| state.set_value("Grace", window, cx));
```

## 掩码与验证

```rust
let password = cx.new(|cx| {
    InputState::new(window, cx)
        .placeholder("Password")
        .masked(true)
        .validate(|value, _| value.chars().count() >= 8)
});
```

格式化值可按需组合 `mask_pattern`、`pattern`、`min`、`max`、`step` 或 `step_by`。`unmask_value()` 返回掩码输入的底层值。

## 事件

`InputState` 会发出 `InputEvent::Change`、`PressEnter`、`Focus` 和 `Blur`。订阅事件后读取新值，并在更新宿主状态后调用 `cx.notify()`。

## 表现

`gpui-base` 不安装产品样式。向状态提供 `InputEditorStyle`，并把控件组合进自己的边框容器。若需要现成主题、尺寸、边框、前后缀槽位和清除按钮，请使用 [`gpui-component` Input](../../component/input.md)。

## 光标闪烁

获得焦点的输入框中，文本光标每 500 ms 闪烁一次，用户输入或移动光标时保持显示。安装 `CursorBlinkDefaults` 可为应用中所有 Input、Textarea、Editor 和 OTP 输入框修改闪烁间隔、用 `None` 关闭闪烁，或在超时后停止闪烁：

```rust
use std::time::Duration;
use gpui_kit::base::input::CursorBlinkDefaults;

CursorBlinkDefaults::new()
    .with_interval(Some(Duration::from_millis(530)))
    .with_timeout(Some(Duration::from_secs(5)))
    .install(cx);
```

每次闪烁都会重绘输入框所在的视图；设置超时后，空闲的输入框不再引起重绘，直到下一次获得焦点、编辑或移动光标。

## 可运行示例

```bash
cargo run -p gpui-base-examples -- input
```

实现位于 [`input.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/input.rs)。

## 原子行内 token

需要让用户整块选中、删除人员提及或资源引用时，可以使用 token。通过已有的 InputState 插入：

```rust
use gpui_kit::base::input::InlineToken;

input.update(cx, |state, cx| {
    state.replace_with_token(
        InlineToken::new("person-1", "@alice").with_label("Alice"),
        window,
        cx,
    ).expect("有效的引用");
});
```

token 默认显示为无装饰标签。通过 `token` 槽位提供自己的单行元素，通过 `on_token_click` 打开引用。通过 `on_token_hover` 上报指针进入与离开（`InlineTokenHoverEvent::is_hovered`），不选中、不编辑，可用于提示或预览；禁用 token 从不上报悬停进入，但禁用正在悬停的 token 会发送其退出事件。复制和 `value()` 返回 `@alice` 这样的真实文本。用 `content()` 保存草稿、`set_value(content)` 恢复草稿，可以保留其中的引用。

自定义展示、草稿恢复和范围单位见 [Input 的 token 示例](../../component/input.md#原子行内-token)。数据类型从 `gpui_kit::base::input` 导入。JavaScript 使用 `gpui-base` 中的 `InputState.new()`，它提供相同的 token 方法。
