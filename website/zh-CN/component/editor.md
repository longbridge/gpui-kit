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

## 语言编辑规则

`LanguageConfig` 描述语言规则；`.auto_close(bool)` 和 `.smart_indent(bool)` 是独立的编辑器选项。
切换语言或替换规则不会重置这两个选项。自动补全、跳过结束符和成对 Backspace 使用
`auto_closing_pairs`；Enter 使用 `brackets` 和 `indentation_rules`，因此关闭自动补全后，
仍可在已有括号内换行并缩进。

```rust
use gpui_kit::component::input::{
    AutoClosingPair, BracketPair, language_config::LanguageConfig, SyntaxContext, set_language_config,
};

let rules = LanguageConfig::default()
    .brackets([BracketPair::new("{", "}"), BracketPair::new("(", ")")])
    .auto_closing_pairs([
        AutoClosingPair::new("{", "}")
            .not_in([SyntaxContext::String, SyntaxContext::Comment]),
        AutoClosingPair::new("(", ")")
            .not_in([SyntaxContext::String, SyntaxContext::Comment]),
    ]);

set_language_config("rust", rules, cx);

let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language("rust")
        .auto_close(true)
        .smart_indent(true)
});
```

`set_language_config` 替换当前应用中指定语言的配置，已有编辑器在下一次编辑时立即使用它，
即使配置修改和编辑发生在同一个事件处理函数内。语言别名共享配置，例如 `python`、`py`、
`pyi`，不受对应 grammar feature 是否启用影响。自定义配置在 Component 初始化后仍然保留。
精确注册的自定义 grammar 名称优先于内置别名，并保留原始大小写。
未知语言使用 `LanguageConfig::default()`。

Component 安装 `LanguageProvider`，统一提供语言名称、默认规则及每个编辑器的语法提供者。
首次编辑和切换语言后的语法选择都不依赖 render。直接使用 Base 时，可通过
`set_language_provider` 安装自己的语言服务；普通 Component 使用者只需调用
`set_language_config`。高亮的 grammar 资源可使用 `highlighter::GrammarConfig`，
原有 `highlighter::LanguageConfig` 名称保持兼容。

配对使用字符串，支持多字符定界符。`auto_closing_pairs = None` 表示使用 `brackets`；
`Some(vec![])` 表示禁用全部自动配对，其 builder 设置的是 `Some`。
`auto_close_before` 指定允许自动补全的后方字符；空白和文档末尾始终允许。
`not_in` 依赖语法上下文提供者；没有提供者时 Base 按 `Code` 处理。
启用对应 Tree-sitter grammar 后，Component 会安装语法上下文提供者。

`IndentationRules::new(increase, decrease)` 接受两个已编译的 `regex::Regex`。
Enter 时分别匹配光标前、后的文本；未配置增加缩进模式时，使用结构括号判断。
这些规则不会重新格式化已有行或粘贴内容。Python 的默认规则额外识别末尾冒号，
未知语言仅使用结构括号。

这是 Monaco 风格语言配置的已支持子集，不直接加载 Monaco JSON 或 `.scm`。
包围选区、自定义 `onEnterRules` 留待后续实现。


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

## 搜索

编辑器内置搜索面板。编辑器聚焦时按 `Ctrl-F`（Windows/Linux）或 `Cmd-F`（macOS）打开。`Enter` 跳到下一个匹配，`Shift+Enter` 跳到上一个，`Escape` 关闭面板。

```rust
// 以代码方式打开查找面板
editor.update(cx, |state, cx| {
    state.open_search(false, cx);
});

// 关闭它
editor.update(cx, |state, cx| {
    state.close_search(cx);
});
```

`Editor` 默认启用搜索。如需禁用：

```rust
editor.update(cx, |state, cx| {
    state.set_searchable(false, cx);
});
```

只读编辑器仍可搜索——替换界面会自动隐藏。

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
主题加载时会核对平台默认等宽字体（`Menlo`、`Consolas`、`DejaVu Sans Mono`）是否已安装，
缺失时换成已安装的等宽字体，再不行退到 `.SystemUIFont`；你自己指定的字体族则原样使用。

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
