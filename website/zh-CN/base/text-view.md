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

## Markdown 扩展

`MarkdownExtensions` 默认使用兼容 CommonMark/GFM 的解析方式。YAML
frontmatter 不属于这两项标准，因此默认关闭。当 block parser 或插件处理
`markdown_ast::Node::Yaml` 时，需要明确启用该 construct：

```rust
use gpui_base::{MarkdownExtensions, TextView};

let extensions = MarkdownExtensions::default().frontmatter();

TextView::markdown("metadata", source)
    .markdown_extensions(extensions)
```

如果没有匹配的插件，已启用的 YAML frontmatter 会使用现有的 YAML
code-block fallback。可以通过 `.plugin(...)` 挂载自定义插件；
`gpui-component` 提供带主题样式的
`FrontmatterPlugin`；Base 不依赖该 presentation。

## Inline plugin

与 Block plugin 一样，Inline plugin 实现 `MarkdownPlugin`，通过 `.plugin(...)` 注册。`MarkdownPlugin` 默认 `is_block() == false`，使用 `render_inline`；Block plugin 继续使用 `render`。

```rust
use gpui::{App, Styled, Window, div};
use gpui_base::{
    InlineElement, InlineRenderContext, MarkdownNode,
    MarkdownParseContext, MarkdownPlugin, TextView, markdown_ast,
};

struct FormulaPlugin;

impl MarkdownPlugin for FormulaPlugin {
    fn name(&self) -> &str {
        "formula"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        _: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        let markdown_ast::Node::InlineMath(math) = node else {
            return None;
        };
        Some(
            MarkdownNode::new("formula", math.value.clone())
                .text(math.value.clone())
                .accessibility_label(format!("Formula: {}", math.value)),
        )
    }

    fn render_inline(
        &self,
        node: &MarkdownNode,
        _: &InlineRenderContext,
        _: &mut Window,
        _: &mut App,
    ) -> Option<InlineElement> {
        Some(InlineElement::new(div().italic().child(node.as_text().to_string())))
    }
}

TextView::markdown("inline-formulas", "Formulas $x^2$ and $y^2$")
    .plugin(FormulaPlugin)
```

`render_inline` 返回 `Some(InlineElement::new(element))`，支持任意 GPUI `IntoElement`，包括带样式的文本、图片和组合元素。样式、hover 和子元素事件直接使用原生 GPUI API。renderer 收到的 `InlineRenderContext` 包含实际文本样式、字号、行高和 rem 大小。这些渲染类型不依赖 Markdown；本例的解析和注册仍属于 Markdown API。

TextView 测量元素的固有尺寸，将整个元素作为一个原子对象排版。需要指定基线时，在 `InlineElement` 上调用 `.with_baseline(px(...))`，数值为从顶部到基线的逻辑像素距离。只能在对象前后换行。固定尺寸的元素即使超过行宽，也保留真实尺寸；需要限宽时使用 GPUI 样式约束。TextView 不会整体缩放元素子树。

当 parser 捕获值或插件配置改变，但注册名称不变时，使用 `MarkdownExtensions::parser_revision(config_version)` 触发重新解析。每次 render 重建相同配置时应保持该 revision 不变。

可以直接用原生 `HoverCard` 包裹 trigger 来显示资料卡。Markdown 示例使用 `StyledText` 设置淡色 `@` 和用户名下划线，通过 `Anchor::TopCenter` 居中定位 `HoverCard`。`[@huacnlee](mention:huacnlee)` 的纯文本复制输出账号，Markdown 复制保留原始链接语法。

选择以整个渲染元素为单位。双击选中对象，三击选中所在混排行；拖选可以双向跨越文字与连续对象。子元素事件保留原生 GPUI 行为，插件中的交互控件应与 TextView 的选择手势协调。

`source_range()` 返回包括分隔符在内的全局 UTF-8 字节范围。`.text(...)` 提供纯文本复制和降级内容；`.markdown(...)` 提供 Markdown 复制内容，默认使用节点原始源码。未提供纯文本时使用源码。`.accessibility_label(...)` 提供无障碍名称，默认使用纯文本。`render_inline` 返回 `None` 时使用原子文本降级。图片的加载中和失败内容由插件通过 `img(...).with_loading(...).with_fallback(...)` 提供。

异步资源应由应用缓存：保留 `TextViewState`，准备完成后通过弱 entity 更新缓存并调用 `state.invalidate_inline_layout(cx)`。这会重新测量行内内容和虚拟列表高度，不重解析文档，也不丢弃已有逻辑选区。缓存键应区分源码、字号和主题，过期结果应丢弃。渲染回调应读取已准备的资源，不应在布局期间同步调用公式排版引擎。`examples/markdown` 提供公式实现和预览缩放控件。

默认解析 inline math 语法，通过 Plugin 自定义渲染，无需额外开关。行内代码里的美元符号仍保留为代码。没有 Plugin 认领某个 math 节点时，TextView 按原始 `$...$` 源码渲染为普通文本，因此正文中单纯出现美元符号的句子（`spent $5 and $10`）显示和复制都保持原样。块级公式同样会被解析：`$$` 围栏产生块级节点，由 Block plugin（`is_block() == true`）渲染；无人认领时降级为代码块。

## 保留状态与动态更新

内容需要持续更新时使用 `TextViewState`：

```rust
use gpui_kit::base::{TextView, TextViewState};

let document = cx.new(|cx| TextViewState::markdown(initial_source, cx));

TextView::new(&document)

document.update(cx, |state, cx| state.set_text(updated_source, cx));
```

通过 `SelectionFormat` 可以选择复制渲染文本或 Markdown 源码。链接路由、代码块操作、表格操作、图片和 Markdown 插件继续使用与兼容 API 相同的 builder，详见 [gpui-component TextView 文档](../component/text-view.md)。

## 可运行源码

网页预览和本地命令使用同一份 Base-only 源码：

<<< ../../../crates/base/examples/showcase/components/text_view.rs{rust}

```bash
cargo run -p gpui-base-examples -- text-view
```
