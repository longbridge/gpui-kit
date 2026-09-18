---
title: Textarea
description: 支持固定行数、软换行和自动增高的多行文本输入组件。
---

# Textarea

`Textarea` 用于普通多行文本。单行输入请使用 [Input](./input.md)，源代码编辑请使用 [Editor](./editor.md)。

## 导入

```rust
use gpui_kit::component::input::{Textarea, TextareaState};
```

## 基础用法

```rust
let notes = cx.new(|cx| {
    TextareaState::new(window, cx)
        .rows(5)
        .placeholder("备注")
});

Textarea::new(&notes)
```

## 自动增高

```rust
let message = cx.new(|cx| {
    TextareaState::new(window, cx)
        .auto_grow(2, 8)
        .placeholder("输入消息")
});

Textarea::new(&message)
```

组件最多增长到 `max_rows`，之后内容在内部滚动。

## 值与事件

```rust
let value = notes.read(cx).value();

notes.update(cx, |state, cx| {
    state.set_value("更新后的备注", window, cx);
});

cx.subscribe(&notes, |this, state, event: &InputEvent, cx| {
    if matches!(event, InputEvent::Change) {
        this.notes = state.read(cx).value();
        cx.notify();
    }
});
```

`TextareaState` 还提供 `insert`、`replace`、`cursor_position`、
`soft_wrap`、`searchable` 和 `submit_on_enter`。

## 外观

```rust
Textarea::new(&notes)
    .h(px(160.))
    .bordered(true)
    .disabled(false)
    .readonly(false)
    .aria_label("备注")
```

与 `disabled` 不同，只读 Textarea 保持正常外观，仍然可以聚焦、选中和复制，只是拒绝用户对内容的修改。

`Textarea` 不提供只适用于单行 Input 的前后缀、密码显示切换和清除按钮；相关操作应组合在 Textarea 外部。

## 原子行内 token

在多行消息中加入人员提及、文件引用或命令时，可以使用 token。通过 `TextareaState::replace_with_token` 插入，或用 `set_value` 恢复已有草稿。默认标签可以直接使用，需要图标等自定义内容时再设置 renderer：

```rust
Textarea::new(&state)
    .token(|token, _, _| InputToken::new(token).icon(IconName::File))
```

从 `gpui_kit::component::input` 导入 `InputToken`，从 `gpui_kit::component` 导入 `IconName`。token 会整块换到下一行，自动增高会相应调整输入框高度。换行符应放在 token 之间的普通文本中。编辑、激活、草稿保存、模式限制和 JavaScript API 见 [Input：原子行内 token](./input.md#原子行内-token)。
