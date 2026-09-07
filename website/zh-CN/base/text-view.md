---
title: TextView
description: 直接使用 gpui-base 渲染可选择的 Markdown 与 HTML。
order: 4
example: text-view
exampleKind: base
---

# TextView

`gpui-base` 现在拥有完整的 `TextView` 实现，可渲染 Markdown 和常用 HTML。解析、链接、图片、列表、表格、代码块、滚动、行数限制、插件、文本选择和复制都不依赖 `gpui-component`。

上方可运行示例只依赖 `gpui-base`。其中 Rust 代码块特意没有着色，因为语法高亮默认不开启。

## 设置窗口

应用启动时调用一次 `gpui_kit::base::init`，并在每个窗口渲染一个 `TextSelectionLayer`。它统一协调 `TextView`、[`SelectableText`](./text-selection.md) 和自定义文本 renderer 的选择行为。

```rust
use gpui_kit::prelude::*;
use gpui_kit::{Context, Render, Window};
use gpui_kit::base::{TextSelectionLayer, TextView};

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(TextSelectionLayer)
            .child(TextView::markdown(
                "readme",
                "# Hello\n\n选择并复制这段 **Markdown**。",
            ))
    }
}
```

如果应用已经调用 `gpui_kit::component::init`，其中已包含 Base 初始化；`gpui-component::Root` 也会安装窗口选择层。

TextView 默认支持选择。拖动选区靠近视口边缘时，共享选择层会自动滚动相关的 `overflow_*_scroll` 区域，不需要额外设置 TextView 的滚动或选择参数。只有明确需要禁用选择时才使用 `.selectable(false)`。

## Markdown 与 HTML

短内容可以使用自动生成调用点 ID 的 helper，需要明确稳定 ID 时使用构造器：

```rust
use gpui_kit::base::{html, markdown, TextView};

let short_markdown = markdown("一段 **Markdown**。");
let short_html = html("<p>一段 <strong>HTML</strong>。</p>");

let preview = TextView::markdown("document-preview", markdown_source).scrollable(true);

let article = TextView::html("article", html_source);
```

`scrollable(true)` 让视图填满容器并垂直滚动；未设置时视图随内容增长。`max_lines(n)` 可把非滚动预览限制在最多 `n` 行正文高度。

## 可直接使用的默认样式

所有构造方式都会使用 `TextViewStyle::default()`。默认值已经包含可读的正文、次要文字、链接、选择色、代码背景、边框、标题、段落、行内代码和表格样式。只使用 Base 的项目不需要先定义一套样式才能显示文本。

应用可以只覆盖自己设计系统负责的颜色：

```rust
use gpui_kit::base::TextViewStyle;

let style = TextViewStyle::default()
    .with_foreground(app_colors.foreground)
    .with_muted_foreground(app_colors.muted_foreground)
    .with_link(app_colors.link)
    .with_selection(app_colors.selection);

TextView::markdown("themed", source).style(style)
```

`TextViewStyle::from_theme(&theme)` 可读取 `gpui_kit::base::Theme` 的语义颜色。使用上层组件主题时，可调用 `gpui_kit::component::text::text_view_style(cx.theme())`。

## 语法高亮由使用者开启

`gpui-base` 默认不启用语法高亮，也不包含 tree-sitter 语言依赖。应用未提供 `code_block_highlighter` 时，围栏代码块只使用中性的代码背景和普通前景色。

回调接收 `CodeBlock`，并返回 UTF-8 字节范围及对应的 GPUI `HighlightStyle`：

```rust
use gpui_kit::HighlightStyle;
use gpui_kit::base::TextView;

TextView::markdown("highlighted", source).code_block_highlighter(|block| {
    my_highlighter(block.lang(), block.code())
        .into_iter()
        .map(|(range, color)| {
            (
                range,
                HighlightStyle {
                    color: Some(color),
                    ..Default::default()
                },
            )
        })
        .collect()
})
```

范围相对于 `CodeBlock::code()`；无效范围会被丢弃。高亮器实现和语言注册完全由应用管理。

## 保留状态与动态更新

内容需要持续更新时使用 `TextViewState`：

```rust
use gpui_kit::base::{TextView, TextViewState};

let document = cx.new(|cx| TextViewState::markdown(initial_source, cx));

TextView::new(&document)

document.update(cx, |state, cx| state.set_text(updated_source, cx));
```

通过 `SelectionFormat` 可以选择复制渲染文本或 Markdown 源码。链接路由、代码块操作、表格操作、图片和 Markdown 插件继续使用与兼容 API 相同的 builder，详见 [gpui-component TextView 文档](../docs/components/text-view.md)。

## 可运行源码

网页预览和本地命令使用同一份 Base-only 源码：

<<< ../../../crates/base/examples/showcase/components/text_view.rs{rust}

```bash
cargo run -p gpui-base-examples -- text-view
```
