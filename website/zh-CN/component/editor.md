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

保留返回的 `TextDecorationCollection`，以便更新或清空该调用方的文本样式。
范围会跟随文本编辑；丢弃句柄不会移除装饰。

### 几何范围装饰

使用独立的 `RangeDecorationCollection` 绘制连续填充或一个逻辑像素宽的边框。
它们只参与绘制，不预留行内空间、不添加 widget、不拦截鼠标事件，也不改变键盘焦点。

```rust
use gpui_kit::component::input::{RangeDecoration, RangeDecorationStyle};

let review_ranges = editor.update(cx, |state, cx| {
    state.create_range_decorations_collection(
        vec![
            RangeDecoration::new(0..8).with_style(RangeDecorationStyle::Fill),
            RangeDecoration::new(12..24), // 默认为 Frame。
        ],
        cx,
    )
});

review_ranges.set(vec![RangeDecoration::new(4..16)], cx);
review_ranges.append(vec![RangeDecoration::new(20..28)], cx);
let tracked_ranges = review_ranges.get_ranges(cx);
review_ranges.clear(cx);   // 清空，但保留集合以便复用。
review_ranges.dispose(cx); // 释放集合，使该句柄及其克隆全部失效。
```

每个集合只管理自己的条目，不同扩展不会互相覆盖。丢弃句柄后，集合仍保存在编辑器中；
调用 `dispose` 才会永久释放它。对已释放集合或已销毁编辑器的调用不会产生效果。
单个装饰不需要提供 ID。

范围使用左闭右开的 UTF-8 **字节偏移量**，不是字符下标或行号。起止偏移量向外裁剪到有效
字符边界；空范围、反向范围和完全超出文档的范围会被丢弃。文本装饰和几何装饰共享跟踪规则：

- 在两端插入文字不会扩展范围，在内部插入则会扩展。
- 替换文字时，重叠锚点裁剪到替换区域；删除整个范围会移除该装饰。
- 撤销、重做、`set_value`、`replace_all`（包括格式化）都应用相同的编辑变换。
  装饰**不是**撤销历史中的快照：撤销删除不会恢复已移除的装饰，撤销替换也不会恢复原先的
  内部锚点。如果业务需要这种语义，请依据自己的语义数据重新设置集合。
- 折叠只改变显示投影，不修改存储的范围。完全隐藏的范围不绘制；可见部分按视口裁剪，
  并跟随软换行后的实际字形位置。

填充绘制在边框下方，两者均位于选区和字形下方。同一种样式内，后创建的集合和后加入的
条目覆盖先前的条目。不调用 `with_color` 时，边框使用编辑器前景色，填充使用其 12% 透明度
版本，因此默认颜色会跟随主题变化；显式颜色由应用自行维护。

可见范围查询使用区间索引并跳过折叠的缓冲区范围，不会逐帧扫描全部装饰。设置或追加条目
会重建对应集合的索引；编辑文本时线性更新受影响的集合，不重新排序。
Editor 展示页的 **Decorations** 标签演示了两种集合的组合使用。

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
