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

双击选中单词，三击选中段落。双击或三击后不松开继续拖动，或之后按住 Shift 点击，选区会按整词或整段扩展，并保留最初选中的单词或段落。

从表格单元格内开始的拖动只会选中该单元格的内容；指针离开单元格后，选区停在单元格边缘。从表格外开始的拖动仍会跨过整个表格。如需复制整个表格，请使用 `table_actions` 提供复制操作。

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

没有显式设置 `.style()` 的 TextView 可以跟随容器设置的文字颜色，这样放在聊天气泡这类有填充底色的容器里也能看清。在应用默认值里开启：

```rust
use gpui_kit::base::{TextViewDefaults, TextViewStyle};

TextViewDefaults::new()
    .with_style(TextViewStyle::default())
    .with_inherit_text_color(true)
    .install(cx);
```

正文使用继承来的颜色。如果这个颜色和样式的正文颜色在明度上相差很大，说明底色与页面明暗相反，此时链接、次要文字、代码和表格背景、边框、选择色也都从它推算，并且不再使用已安装的语法高亮。只有在每个窗口的根部都设置了文字颜色时才开启；GPUI Component 的 Root 会设置，Component 也默认开启了这一项。

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

`TextViewMotion` 是视图的动效策略。Base 负责播放，但不带任何时长：所有时长默认为零，未加样式的视图会直接显示流式到达的文字。给 `stream_fade` 一个时长，更新追加的文字就会在落点处淡入；可选的 `stream_fade_stagger` 让同一次更新里后面的词比前一个词稍晚开始：

```rust
use std::time::Duration;

use gpui_kit::base::{Easing, TextView, TextViewMotion};

TextView::new(&document).motion(
    TextViewMotion::default()
        .with_stream_fade(Duration::from_millis(350))
        .with_stream_fade_stagger(Duration::from_millis(30))
        .with_stream_fade_easing(Easing::EaseOut),
)
```

不设错位时每次更新整块一起淡入。设了错位时，追加的文字按词拆分（词带上其后的空白），中日韩文字按字拆分；一次追加很长时会压缩错位，保证最后一个词在一个淡入时长内开始。追踪器比较的是渲染后的文字而不是源码字节，因此 `set_text` 传入以当前文本为前缀的更长文本会被视为追加；流式过程中被补齐的 Markdown 标记（`**bo` 变成粗体 `bold`）只让发生变化的字形重新淡入，不会整段闪烁。每次只比较更新触及的块，并且只在还有文字在淡入时才请求下一帧。系统开启减少动态效果时跳过淡入。

`TextViewState::set_range_highlights` 在 `rendered_text()`（与纯文本复制得到的文字一致）的指定范围后面绘制背景，应用可以借此显示搜索结果或引用位置，无需重新解析或修改文档样式。这些范围只参与绘制、不参与排版，因此不会改变布局。Markdown 源码里的范围（例如 `selected_source_range()` 返回的范围）可以用 `rendered_text().range_for_source(range)` 转换过来。`reveal_range` 通过视图自身的列表、外层 `gpui::list`，或者其他容器上的 `TextView::on_reveal`，把范围起点所在的行滚动到可见区域内；规则详见[高亮文本范围](../component/text-view.md#高亮文本范围)和[滚动到范围](../component/text-view.md#滚动到范围)。

通过 `SelectionFormat` 可以选择复制渲染文本或 Markdown 源码；`SelectionFormat::SourceAcrossStyles` 只在选区跨越多种样式时复制源码，例如跨过 `**bold**` 的边界或跨越两个块。链接路由、代码块操作、表格操作、图片和 Markdown 插件继续使用与兼容 API 相同的 builder，详见 [gpui-component TextView 文档](../component/text-view.md)。

## 可运行源码

网页预览和本地命令使用同一份 Base-only 源码：

<<< ../../../crates/base/examples/showcase/components/text_view.rs{rust}

```bash
cargo run -p gpui-base-examples -- text-view
```

## 显示变化后的选区保持

样式、语法高亮、继承字体或宽度变化时，已完成的选区默认保持相同的逻辑文本范围和复制内容，同时应用新的显示样式和换行布局。移动已挂载的视图不会用旧的鼠标矩形重新计算选区。新的选择操作或显式清除会正常更新选区；替换原文或 Markdown 解析配置仍会清除选区。拥有视图的代码必须保持状态和选择参与者挂载；这个行为不会固定外层虚拟列表中已卸载的行。


## 构建外部富文本组件

`gpui_kit::base::text::editor_host` 对外提供 TextView 的渲染能力，外部组件可以保留自己的编辑模型。将文档投影为 `BlockNode`，由组件管理命令、历史记录、选区、焦点和原生输入。

- 通过 `Paragraph::with_leaf_key` 或 `CodeBlock::with_leaf_key` 标记叶节点。同一收集范围内的 key 必须唯一。克隆最初共享运行时状态；为克隆设置 key 时，会将其叶节点状态与原节点分离。
- 使用 `RenderContext::new(style)` 设置局部样式，或使用 `from_app(cx)` 读取 TextView 默认值。通过 `with_code_block_highlighter`、`with_image_source`、`on_link_click` 和 `with_link_reference` 局部配置高亮、图片来源、链接操作及引用链接。
- 使用 `render_with_id` 渲染块，并传入稳定的业务 ID。叶节点的几何 key 与 GPUI element ID 分别承担不同职责。
- 在自定义 element 的 prepaint 阶段调用 `collect_leaf_geometry(decorations, || child.prepaint(window, cx))`。它只返回当前范围内执行过 prepaint 的叶节点，嵌套范围相互隔离。
- 内容、字体、宽度或滚动位置变化后，应替换几何结果。`LeafGeometry` 是不可变快照，坐标使用窗口逻辑像素；索引使用 UTF-8 字节，每个行内原子节点以 U+FFFC 表示，占三个字节。
- Decoration 使用有序的半开区间。文本背景绘制在字形下方；原子节点的 decoration 覆盖整个节点并绘制在内容上方。原子节点的选区颜色应使用半透明色。

这个 API 复用 TextView 的水平块流渲染器。宿主仍需管理原生输入的 UTF-16 转换、无障碍集成和虚拟化。当前的字节索引几何无法表达双向文本中的光标 affinity，也无法区分软换行边界的两侧，因此还不是完整的视觉光标导航 API。

## 行内代码样式

通过 `TextView::inline_code_style(StyleRefinement)` 覆盖行内代码样式。覆盖在主题、显式样式和表面颜色适配之后应用；未指定的属性保留默认值，指定的颜色在反色表面上也保持不变。

Base 样式提供对应的 `TextViewStyle::with_inline_code_style` 构建方法和 `inline_code_style()` 读取方法。现有 `with_inline_code(HighlightStyle)` 保持兼容，新的覆盖样式具有更高优先级。

```rust
use gpui_kit::{StyleRefinement, Styled as _, rems};

let border = gpui_kit::base::Theme::global(cx).tokens.colors.border;

TextView::markdown("notes", "Run `cargo build` first.")
    .inline_code_style(
        StyleRefinement::default()
            .text_size(rems(1.))
            .px(rems(0.25))
            .border_1()
            .border_color(border),
    )
```

支持字体、字号、字体特性、后备字体、字重、斜体、文字颜色、下划线、删除线、背景、边框宽度／颜色／线型、圆角和内边距。字号使用像素或相对于窗口的 rem；非有限值和非正值会被忽略。默认字号仍为周围文字的 0.875 倍，默认不绘制边框。内边距和边框宽度参与换行与表格列宽计算。外边距、定位、flex、显式容器尺寸、overflow 和交互属性不参与渲染；行高由段落控制。

## 列表与引用块样式

通过 `TextView::list_style(StyleRefinement)` 和 `blockquote_style(StyleRefinement)` 覆盖列表标记和引用块样式。覆盖在主题、显式样式和表面颜色适配之后应用；未指定的属性保留默认值，指定的颜色在反色表面上也保持不变。

Base 样式提供对应的 `TextViewStyle::with_list_style`、`with_blockquote_style` 构建方法，以及 `list_style()`、`blockquote_style()` 读取方法。

```rust
use gpui_kit::{StyleRefinement, Styled as _};

let primary = gpui_kit::base::Theme::global(cx).tokens.colors.primary;

TextView::markdown("notes", "1. First\n2. Second\n\n> Quoted")
    .list_style(StyleRefinement::default().text_color(primary))
    .blockquote_style(StyleRefinement::default().border_color(primary))
```

列表标记样式作用于每个列表项的项目符号或序号列，叠加在默认样式之上：次要文字颜色、以列表缩进为最小宽度、文字右对齐、右内边距为缩进的 0.3 倍。颜色、字体、字号、字重等文字属性用于标记本身；宽度、内边距等盒模型属性会改变标记列。其文字颜色也用于未勾选任务复选框的边框；已勾选的复选框仍使用链接颜色。

引用块样式作用于引用容器，叠加在默认样式之上：次要文字颜色、边框颜色的 2px 左边框、左内边距为正文字号的 0.9 倍。边框宽度和颜色用于引用竖线；文字属性、背景、内边距和圆角作用于引用及其内部内容。子块显式设置的样式仍优先，例如标题和表格的字号。其他元素属性与普通 `div` 一样作用于容器。代码块、表格、分隔线和已完成任务仍使用共享的次要文字颜色和边框颜色。
