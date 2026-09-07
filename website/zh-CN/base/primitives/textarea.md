---
title: Textarea
description: 支持固定行数或自动增高的无样式多行文本框。
order: 15
---

# Textarea

`Textarea` 用于普通多行文本，接口聚焦于行数、换行、自动增高、值更新、插入、替换和光标位置。代码编辑器概念由 [`Editor`](./editor.md) 提供。

## 导入

```rust
use gpui_kit::base::input::{InputEvent, Textarea, TextareaState};
```

## 固定行数

```rust
let notes = cx.new(|cx| {
    TextareaState::new(window, cx)
        .rows(5)
        .placeholder("Notes")
        .default_value("First line\nSecond line")
});
Textarea::new(&notes)
```

## 自动增高

文本框会在指定的最小与最大行数之间增长；达到最大值后内容改为滚动。

```rust
let message = cx.new(|cx| {
    TextareaState::new(window, cx)
        .auto_grow(2, 8)
        .placeholder("Write a message")
});
Textarea::new(&message)
```

## 编辑值

```rust
notes.update(cx, |state, cx| state.insert("Appended text", window, cx));
let cursor = notes.read(cx).cursor_position(cx);
let value = notes.read(cx).value();
```

不希望视觉换行时使用 `soft_wrap(false)`。只有 Enter 应提交而非换行时才设置 `submit_on_enter(true)`。`TextareaState` 发出与 `InputState` 相同的 `InputEvent`。

## 表现

该控件没有产品样式；边框、高度、颜色、内边距和 `InputEditorStyle` 由设计系统提供。现成样式控件参见 [`gpui-component` Textarea](../../docs/components/textarea.md)。

## 可运行示例

```bash
cargo run -p gpui-base-examples -- textarea
```
