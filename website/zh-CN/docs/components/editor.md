---
title: Editor
description: 支持语法高亮、行号、折叠和文本装饰的源代码编辑器。
---

# Editor

`Editor` 用于编辑源代码。单行输入请使用 [Input](./input.md)，普通多行文本请使用 [Textarea](./textarea.md)。

## 导入

```rust
use gpui_kit::component::input::{Editor, EditorState, TabSize};
```

## 基础用法

```rust
let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language("rust")
        .line_number(true)
        .folding(true)
        .tab_size(TabSize {
            tab_size: 4,
            hard_tabs: false,
        })
        .default_value("fn main() {\n    println!(\"Hello\");\n}")
});

Editor::new(&editor).h(px(320.))
```

使用 `.language()` 指定语法高亮语言。应用需要启用对应的 Cargo feature，例如 `tree-sitter-rust` 或 `tree-sitter-markdown`；也可以使用 `tree-sitter-languages` 包含全部内置语法。

## 编辑器选项

```rust
let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language("json")
        .line_number(true)
        .folding(true)
        .show_whitespaces(true)
        .default_value(source)
});
```

## 快捷键与矩形列选

以下默认快捷键在编辑器聚焦时生效。macOS 的 Option 对应 Alt 修饰键；Linux 的这些操作不使用 Super/Win。

| 操作 | macOS | Linux | Windows |
| --- | --- | --- | --- |
| 在上方／下方添加光标 | Cmd+Option+↑ / ↓ | Alt+Shift+↑ / ↓ | Ctrl+Alt+↑ / ↓ |
| 逐字符扩展所有选区 | Shift+← / → | Shift+← / → | Shift+← / → |
| 按词扩展所有选区 | Option+Shift+← / → | Ctrl+Shift+← / → | Ctrl+Shift+← / → |
| 鼠标添加光标 | Option+左键点击 | Alt+左键点击 | Alt+左键点击 |
| 矩形列选 | Option+Shift+左键拖动 | Alt+Shift+左键拖动 | Alt+Shift+左键拖动 |
| 只保留活动光标 | Escape | Escape | Escape |

Linux 额外支持与 Ghostty 一致的 Ctrl+Alt+左键拖动列选，以及 Alt+Shift+← / → 按词选择。Windows 额外支持 Alt+Shift+← / → 逐字符选择。三个平台都兼容 Alt/Option+左键拖动列选：单击添加光标，继续拖动则以鼠标按下位置为起点建立新的矩形选区。

在编辑区按住 Alt/Option 时，鼠标指针显示为 `+`。带 Alt 的选择手势优先于 Ctrl/Cmd+点击跳转定义。矩形选区按显示行生成，每行一个选区，短行会截断到已有文本边界。输入和删除同时作用于所有选区。松开鼠标结束拖动，Escape 只保留活动光标（若上下文菜单已打开，则先处理菜单的 Escape）。

使用 ↑ / ↓ 添加光标是累加操作，反向按键不会收缩矩形高度。因此这是多光标编辑与鼠标列选，并非持续的 Vim Visual Block 模式。键盘输入期间光标保持可见，空闲 300ms 后恢复闪烁。

Linux 桌面可能在编辑器收到事件之前拦截快捷键。部分桌面使用 Ctrl+Alt+↑ / ↓ 切换工作区，因此 Linux 默认不绑定这一组合。以上快捷键指键盘重映射后的逻辑修饰键。

## 文本装饰

```rust
let decorations = editor.update(cx, |state, cx| {
    state.create_decorations_collection(initial_decorations, cx)
});
```

需要装饰存在多久，就应将返回的 `TextDecorationCollection` 保留多久；文本修改后，其 range 会自动跟随内容变化。

## 值与事件

```rust
let source = editor.read(cx).value();

editor.update(cx, |state, cx| {
    state.set_value(new_source, window, cx);
});
```

`EditorState` 会发出 `InputEvent::Change`、`Focus` 和 `Blur` 等事件。

## 字体

Editor 默认使用主题中的等宽字体 —— `mono_font_family` 和 `mono_font_size`，行高为字号的
1.5 倍。这只是默认值：在 Editor 上设置的文本样式会覆盖它，gutter 和行高都跟随字号变化。

```rust
Editor::new(&editor).text_sm()

Editor::new(&editor)
    .font_family("JetBrains Mono")
    .text_size(px(15.))
```

这些就是所有元素都有的 [`Styled`](https://docs.rs/gpui/latest/gpui/trait.Styled.html)
方法，`font_weight`、`line_height` 用法相同。

## 外观

```rust
Editor::new(&editor)
    .h(px(480.))
    .bordered(true)
    .disabled(false)
    .readonly(false)
    .aria_label("Rust 源代码")
```

预览文件但不允许修改时使用 `readonly`。与 `disabled` 不同，只读编辑器保持正常外观，仍然可以聚焦、选中、复制和搜索，只是拒绝用户对内容的修改。`set_value` 等程序调用不受影响。

```rust
Editor::new(&editor).readonly(true)
```

Editor 聚焦时不会应用单行 Input 的焦点边框效果。gutter、当前行背景和滚动条会作为同一个编辑器表面对齐绘制。

前后缀、密码显示切换和清除按钮只属于单行 Input。Editor 的工具栏和操作按钮应组合在组件外部。
