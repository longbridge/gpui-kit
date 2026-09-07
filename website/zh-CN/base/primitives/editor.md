---
title: Editor
description: 支持语言、行号槽、折叠和装饰的无样式源代码编辑器。
order: 16
---

# Editor

`Editor` 是源代码编辑控件。它建立在共享文本引擎之上，增加语言、行号槽、折叠、空白字符显示、文本装饰、高亮、搜索基础、诊断与 LSP 扩展。单行值使用 [Input](./input.md)，普通多行文本使用 [Textarea](./textarea.md)。

## 快捷键

Base 与样式组件共享键盘和鼠标行为。各平台快捷键、多光标编辑和矩形列选的细节请参阅
[快捷键与矩形列选](../../docs/components/editor.md#快捷键与矩形列选)。

## 导入

```rust
use gpui_kit::base::input::{Editor, EditorState, TabSize};
```

## 基本用法

```rust
let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language("rust")
        .line_number(true)
        .folding(true)
        .tab_size(TabSize { tab_size: 4, hard_tabs: false })
        .default_value("fn main() {\n    println!(\"Hello\");\n}")
});
Editor::new(&editor)
```

## 空白字符与装饰

通过 `show_whitespaces(true)` 显示空白字符，通过 `create_decorations_collection` 创建随文本编辑自动跟踪范围的装饰集合。只要装饰仍需生效，就应保留返回的 collection。

## 高亮与语言功能

`InputHighlighterFactory`、`InputHighlighter`、诊断类型和 LSP provider trait 是提供给设计系统作者的底层扩展点，作用于共享的 `InputBaseState`。样式组件的应用通常应通过编辑器集成配置它们，而不是普通文本框。

可运行展示使用 `syntect`，在 WASM 中选择兼容的 `fancy-regex` 后端。Syntect 只识别语法 scope；适配器把它们映射为语义名称，再由 `HighlightStyleResolver` 从应用主题解析颜色和字体样式。示例会在每次编辑后重新解析短代码；生产集成可以在 `InputHighlighter` 中保留增量解析状态。

## 字体与表现

Editor 没有独立字体设置，而是使用环境文本样式。可在外层元素设置 `font_family`、`text_size`、字重和行高。应用负责编辑器颜色、行号槽、折叠图标和覆盖层；使用 `InputEditorStyle`、`FoldIconRenderer` 与 provider trait 接入。现成视觉方案参见 [`gpui-component` Editor](../../docs/components/editor.md)。

## 可运行示例

```bash
cargo run -p gpui-base-examples -- editor
```
