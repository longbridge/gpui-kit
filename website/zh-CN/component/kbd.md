---
title: Kbd
description: 以符合平台习惯的方式显示键盘快捷键。
---

# Kbd

Kbd 用于展示键盘快捷键和组合键，并会自动根据平台采用合适的显示格式。macOS 会使用符号，Windows 和 Linux 则使用文本标签，便于文档、菜单和帮助面板保持一致的快捷键表达。

## 导入

```rust
use gpui_kit::component::kbd::Kbd;
use gpui_kit::Keystroke;
```

## 用法

### 基础快捷键

```rust
let kbd = Kbd::new(Keystroke::parse("cmd-shift-p").unwrap());
let kbd: Kbd = Keystroke::parse("escape").unwrap().into();
```

### 常见快捷键

```rust
Kbd::new(Keystroke::parse("cmd-shift-p").unwrap())
Kbd::new(Keystroke::parse("cmd-t").unwrap())
Kbd::new(Keystroke::parse("cmd--").unwrap())
Kbd::new(Keystroke::parse("cmd-+").unwrap())
Kbd::new(Keystroke::parse("escape").unwrap())
Kbd::new(Keystroke::parse("enter").unwrap())
Kbd::new(Keystroke::parse("backspace").unwrap())
```

### 多修饰键

```rust
Kbd::new(Keystroke::parse("cmd-ctrl-shift-a").unwrap())
Kbd::new(Keystroke::parse("cmd-alt-backspace").unwrap())
Kbd::new(Keystroke::parse("ctrl-alt-shift-a").unwrap())
```

### 方向键与功能键

```rust
Kbd::new(Keystroke::parse("left").unwrap())
Kbd::new(Keystroke::parse("right").unwrap())
Kbd::new(Keystroke::parse("up").unwrap())
Kbd::new(Keystroke::parse("down").unwrap())
Kbd::new(Keystroke::parse("f12").unwrap())
Kbd::new(Keystroke::parse("secondary-f12").unwrap())
Kbd::new(Keystroke::parse("pageup").unwrap())
Kbd::new(Keystroke::parse("pagedown").unwrap())
```

### 关闭默认外观

```rust
Kbd::new(Keystroke::parse("cmd-s").unwrap())
    .appearance(false)
```

### 从 Action 绑定读取

```rust
use gpui_kit::{Action, Window, FocusHandle};

if let Some(kbd) = Kbd::binding_for_action(&MyAction {}, None, window) {
    // 显示该 action 绑定的快捷键
}

if let Some(kbd) = Kbd::binding_for_action(&MyAction {}, Some("Editor"), window) {
    // 显示特定上下文中的快捷键
}

if let Some(kbd) = Kbd::binding_for_action_in(&MyAction {}, &focus_handle, window) {
    // 显示焦点元素上的快捷键
}
```

## 平台差异

### macOS

- 使用符号：⌃ ⌥ ⇧ ⌘
- 修饰键之间不加分隔符
- 顺序为 Control、Option、Shift、Command
- 特殊键使用 ⌫、⎋、⏎、← → ↑ ↓ 等符号

### Windows / Linux

- 使用文本标签：Ctrl、Alt、Shift、Win
- 修饰键之间使用 `+`
- 顺序为 Ctrl、Alt、Shift、Win
- 特殊键显示为 Backspace、Esc、Enter、Left、Right、Up、Down

### 平台示例

| 输入 | macOS | Windows / Linux |
| --- | --- | --- |
| `cmd-a` | ⌘A | Win+A |
| `ctrl-shift-a` | ⌃⇧A | Ctrl+Shift+A |
| `cmd-alt-backspace` | ⌥⌘⌫ | Win+Alt+Backspace |
| `escape` | ⎋ | Esc |
| `enter` | ⏎ | Enter |
| `left` | ← | Left |

## 示例

### 快捷键帮助面板

```rust
use gpui_kit::{div, h_flex, v_flex};

v_flex()
    .gap_2()
    .child(
        h_flex()
            .gap_2()
            .items_center()
            .child("Open command palette:")
            .child(Kbd::new(Keystroke::parse("cmd-shift-p").unwrap()))
    )
    .child(
        h_flex()
            .gap_2()
            .items_center()
            .child("Save file:")
            .child(Kbd::new(Keystroke::parse("cmd-s").unwrap()))
    )
    .child(
        h_flex()
            .gap_2()
            .items_center()
            .child("Find in files:")
            .child(Kbd::new(Keystroke::parse("cmd-shift-f").unwrap()))
    )
```

### 带快捷键的菜单项

```rust
h_flex()
    .justify_between()
    .items_center()
    .child("New File")
    .child(Kbd::new(Keystroke::parse("cmd-n").unwrap()))
```

### 行内说明

```rust
div()
    .child("Press ")
    .child(Kbd::new(Keystroke::parse("escape").unwrap()))
    .child(" to cancel or ")
    .child(Kbd::new(Keystroke::parse("enter").unwrap()))
    .child(" to confirm.")
```

### 自定义样式

```rust
Kbd::new(Keystroke::parse("cmd-k").unwrap())
    .text_color(cx.theme().accent)
    .border_color(cx.theme().accent)
    .bg(cx.theme().accent.opacity(0.1))
```

### 仅获取文本格式

```rust
let shortcut_text = Kbd::format(&Keystroke::parse("cmd-shift-p").unwrap());
div().child(format!("Shortcut: {}", shortcut_text))
```

## 样式

Kbd 默认包含以下样式：

- 使用主题边框颜色绘制边框
- 使用 muted 前景色显示文字
- 使用主题背景色作为底色
- 小圆角
- 文本居中
- 超小字号
- 极小的内边距
- 最小宽度为 5
- 禁止 flex shrink，避免压缩失真

所有样式都可以通过 `Styled` trait 的方法覆盖。
