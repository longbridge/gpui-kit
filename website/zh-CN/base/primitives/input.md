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

`gpui-base` 不安装产品样式。向状态提供 `InputEditorStyle`，并把控件组合进自己的边框容器。若需要现成主题、尺寸、边框、前后缀槽位和清除按钮，请使用 [`gpui-component` Input](../../docs/components/input.md)。

## 可运行示例

```bash
cargo run -p gpui-base-examples -- input
```

实现位于 [`input.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/input.rs)。
