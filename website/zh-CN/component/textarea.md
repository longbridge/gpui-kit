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

## 按内容定高

宿主自行控制高度时，可以读取 `notes.read(cx).display_map().wrap_row_count()`。
使用与 [Editor 相同的 prewrap 模式](./editor.md#按内容定高)，可以在首次布局前得到行数；
请传入 Textarea 实际使用的字体、字号和文本区宽度，而不是 Editor 默认的等宽字体设置。

## 滚动

只要文本框中的内容可以滚动，在文本框上的滚动就只作用于文本框，滚到文本末尾也不会带动外层页面；内容未超出文本框时，滚动交给外层页面。

## 滚动回弹

```rust
let notes = cx.new(|cx| {
    TextareaState::new(window, cx)
        .rows(10)
        .scroll_bounce(true)
});
```

开启 `scroll_bounce(true)` 后，内容高于文本框时，触控板或 Magic Mouse 的精细像素滚动及其惯性到达顶部或底部时，会让文本越过边缘，再弹回原位。普通鼠标滚轮产生按行滚动，不触发回弹；外框保持不动。iOS、Android 和 macOS 默认开启（与系统自带的滚动视图一致），其他平台默认关闭；系统要求减弱动态效果时，文本停在边缘，不会越过。

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

## 尺寸

```rust
Textarea::new(&notes).large()
Textarea::new(&notes) // medium（默认）
Textarea::new(&notes).small()
```

尺寸会同时改变文字大小和文本四周的内边距，但不决定高度：固定高度用 `h` 设置，随内容增高的 Textarea 由 `rows` 或 `auto_grow` 决定高度。

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

从 `gpui_kit::component::input` 导入 `InputToken`，从 `gpui_kit::component` 导入 `IconName`。token 会整块换到下一行，自动增高会相应调整输入框高度。换行符应放在 token 之间的普通文本中。使用 `on_token_hover` 可在不选中、不编辑的情况下显示提示或预览；禁用 token 不会上报悬停进入事件；禁用已悬停的 token 时会发送离开事件。编辑、激活、草稿保存、模式限制和 JavaScript API 见 [Input：原子行内 token](./input.md#原子行内-token)。
