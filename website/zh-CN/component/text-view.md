---
title: TextView
description: 渲染 Markdown 与 HTML 文本，并支持自定义 Markdown 插件。
---

# TextView

`TextView` 用于在 GPUI 中渲染格式化文本。它支持 Markdown、简单 HTML、文本选择、代码块操作，以及通过 Markdown 插件解析和渲染项目自定义语法。

标准实现现在位于 `gpui-base`；本模块保留兼容重导出和组件主题适配。仅使用 Base 时的设置、完整默认样式及可选语法高亮请参阅 [GPUI Base TextView](/zh-CN/base/text-view)。

`TextView::selectable(true)` 使用 `gpui-base` 提供的窗口级文本选择引擎。如果要让普通文本或自定义 renderer 参与同一选择，请参阅 [GPUI Base Text Selection](/base/text-selection)（英文）。

## 导入

```rust
use gpui_kit::component::text::{markdown, TextView};
```

## 用法

### Markdown

只需要渲染 Markdown 时，可以使用 `markdown` helper：

```rust
use gpui_kit::component::text::markdown;

markdown("# Hello\n\nThis is **Markdown**.")
    .selectable(true)
    .scrollable(true)
```

如果需要稳定 id，也可以直接构造 `TextView`：

```rust
use gpui_kit::component::text::TextView;

TextView::markdown("preview", markdown_source)
    .selectable(true)
```

### HTML

```rust
TextView::html("html-preview", "<strong>Hello</strong>")
```

### 流式文字淡入

聊天回复是分块到达的。`stream_fade(true)` 会让每一块新文字在落点处淡入，而不是直接蹦出来，观感与 Claude 展示回复的方式一致：

```rust
TextView::new(&self.reply).stream_fade(true)
```

淡入以渲染后的文字为准：`push_str`，或者 `set_text` 传入以当前文本为前缀的更长文本，新增的部分从透明渐变到正常颜色，用时 350ms、ease-out 曲线，这是从 Claude 实测得到的节奏：比模型每块 50–300ms 的到达间隔更长，于是相邻几块的淡入互相重叠，尾部呈现为一段渐变，而不是最新一块突然变实。代码块里的代码和表格单元格里的文字同样参与。流式过程中被补齐的 Markdown 标记（`**bo` 变成粗体 `bold`）只让发生变化的字形重新淡入，不会整段闪烁。替换当前内容的文本直接显示；系统开启减少动态效果时也直接显示。不开启就没有任何动画。

需要自定义时长、缓动，或者让每块按词逐个浮现时，通过 `.motion(...)` 传入 `TextViewMotion`，详见 [GPUI Base TextView](/zh-CN/base/text-view#保留状态与动态更新)。

### 高亮文本范围

应用在文档里搜索，或者指向文档里的某处引用时，用 `set_range_highlights` 把这些范围标出来。搜索由应用负责：在视图显示的文本 `rendered_text()` 里找到范围，连同绘制用的颜色一起交回，当前结果用更醒目的颜色：

```rust
use gpui_kit::component::{
    ActiveTheme as _,
    text::{RangeHighlight, RangeHighlightError, TextViewState},
};

fn highlight_matches(
    state: &mut TextViewState,
    query: &str,
    current_match: usize,
    cx: &mut Context<TextViewState>,
) -> Result<(), RangeHighlightError> {
    let (color, current_color) = (cx.theme().warning.opacity(0.3), cx.theme().warning);
    let text = state.rendered_text();
    let matches = if query.is_empty() {
        Vec::new()
    } else {
        text.as_str().match_indices(query).collect()
    };
    let highlights = matches.into_iter().enumerate().map(|(ix, (start, found))| {
        RangeHighlight::new(
            start..start + found.len(),
            if ix == current_match { current_color } else { color },
        )
    });
    state.set_range_highlights(highlights, cx)
}
```

`rendered_text()` 与纯文本复制得到的文字一致：`hello **world**` 读作 `hello world`，转义字符已经还原，标题和列表的标记不在其中。偏移量是 UTF-8 字节偏移，`str` 搜索返回的范围可以直接传入；重复出现的词组按各自的位置区分。文本在第一次读取时才会生成。

高亮绘制在文字背后、选区之下，换行、对齐、语法颜色、链接、选择和复制都保持不变。多个高亮重叠时，后面的覆盖前面的。跨越两个块的范围在两个块里分别绘制。不属于任何块的文字不会绘制：块之间的换行、表格单元格之间的空格、自定义块、HTML 块和 inline plugin 对象。只有起点在终点之后、超出范围或不在字符边界上的范围会被拒绝，同一批的整组高亮也一并拒绝。

内容变化时，高亮会跟随它所在的块，保留到这个块里文字开始变化的位置为止。流式追加的文字（无论通过 `push_str` 还是 `set_text`）不影响前面的高亮；修改某处时，修改前后的高亮都会保留。表格单元格只按位置区分，因此修改表格内部时，被修改的那一行及其后各行单元格的高亮都会失效。视图会发出通知：观察这个 state，重新在新的 `rendered_text()` 里搜索即可。请在同一次 state 更新中计算范围并调用 `set_range_highlights`，确保范围对应当前文本。属于文字本身的背景（例如 `<mark>` 和语法高亮）会覆盖在范围高亮之上（行内代码的背景在高亮之下），高亮也不会随流式文字一起淡入。HTML 视图不支持范围高亮。

## 触摸选择

在触摸屏上，长按会选中手指下的单词，手指按住不放时选区跟随手指移动。抬起手指后，选区上方会出现包含 `复制` 和 `全选` 的编辑菜单，并在选区两端各显示一个拖动 handle。拖动 handle 会移动对应的一端，另一端保持不动；`全选` 选中被按下的那个视图，其 handle 仍可继续调整结果。

handle 和菜单由 [`Root`](/zh-CN/component/root) 为整个窗口选区绘制，因此跨多个视图的选区也能覆盖到。点击其他位置会清除它们；手指滚动内容时菜单会暂时让开。

## 图片

Markdown 的 `![alt](src)` 和 HTML 的 `<img src>` 都通过 GPUI 的 `img` 元素渲染，
`src` 决定字节从哪里来：

- `http://`、`https://` URL 使用应用的 HTTP client 拉取。
- `data:` URL 就地解码，文档可以内嵌自己的图片（`data:image/png;base64,…`，
  或者百分号编码的 `data:image/svg+xml,…`）。GPUI 能解码的图片格式都可以；
  media type 不是图片的 `data:` URL 会交给默认加载器，像其他加载失败的图片一样报错。
- 其余的值——相对路径、`file://`、自定义 scheme——原样作为 URI 传下去。
  `TextView` 不会替文档读文件系统或 asset bundle。

要解析这些来源，或者改变任意图片的加载方式，把 `TextView` 包在一个安装了 GPUI
`ImageCache` 的元素里。它内部的每个 `img`（包括文档生成的）都会先向这个 cache
请求自己的 `Resource`，再回退到默认加载器：

```rust
use gpui_kit::{ImageCache, ImageCacheProvider};

div()
    .image_cache(app_image_cache.clone())
    .child(markdown("![diagram](app://diagrams/pipeline.svg)"))
```

`ImageCache::load` 拿到 `Resource::Uri` 后自行决定怎样得到 `RenderImage`，
加载策略归应用所有，文档本身仍是普通 Markdown。

## Markdown 插件

使用 `.plugin(...)` 支持自定义 Markdown 格式。插件同时拥有解析和渲染逻辑，调用方只需要把它挂到 `TextView` 上：

```rust
markdown(source)
    .plugin(TickerPlugin::new())
```

Markdown 插件实现 `MarkdownPlugin`：

```rust
use gpui_kit::{App, IntoElement, ParentElement as _, Window};
use gpui_kit::component::text::{
    markdown_ast, MarkdownNode, MarkdownParseContext, MarkdownPlugin,
};

struct TickerNode {
    symbol: String,
}

struct TickerPlugin;

impl TickerPlugin {
    fn new() -> Self {
        Self
    }
}

impl MarkdownPlugin for TickerPlugin {
    fn is_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "ticker"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        let markdown_ast::Node::Paragraph(paragraph) = node else {
            return None;
        };
        let [markdown_ast::Node::Text(text)] = paragraph.children.as_slice() else {
            return None;
        };
        let symbol = text.value.strip_prefix('$')?;

        Some(
            MarkdownNode::new(
                "ticker",
                TickerNode {
                    symbol: symbol.to_string(),
                },
            )
            .text(format!("${symbol}"))
            .markdown(cx.node_source(node).unwrap_or(text.value.as_str())),
        )
    }

    fn render(
        &self,
        node: &MarkdownNode,
        _window: &mut Window,
        _cx: &mut App,
    ) -> impl IntoElement {
        let ticker = node.data::<TickerNode>().expect("ticker node data");

        gpui_kit::div().child(format!("${}", ticker.symbol))
    }
}
```

然后挂到 Markdown `TextView`：

```rust
markdown("$AAPL.US")
    .plugin(TickerPlugin::new())
```

## MarkdownNode

`MarkdownNode` 是 `parse` 和 `render` 之间传递的中性数据结构。

```rust
MarkdownNode::new("ticker", TickerNode { symbol })
    .text("$AAPL.US")
    .markdown("$AAPL.US")
```

- `name` 是稳定的节点名称，用于匹配 renderer。
- `data` 是 parser 产生的类型化数据，通过 `node.data::<T>()` 读取。
- `text` 是纯文本表示，用于选择和未注册 renderer 时的回退渲染。
- `markdown` 是 Markdown 表示，用于将文档重新序列化为 Markdown。

## Block plugin

Block plugin 在 `is_block()` 中返回 `true`，使用 block parser 和 renderer：

```rust
fn is_block(&self) -> bool {
    true
}
```

Inline plugin 保留默认的 `is_block() == false`，`render_inline` 返回 `Option<InlineElement>`。通过 `InlineElement::new(...)` 包裹任意 GPUI 元素，使用原生样式与事件，并按需指定基线。TextView 将整个元素作为原子对象测量和选择，支持纯文本与 Markdown 复制、文本降级和异步布局失效。契约与 `.plugin(...)` 注册示例详见[Inline plugin](../base/text-view.md#inline-plugin)。Component 层导出相同的 `InlineElement` 和 `InlineRenderContext` 类型。

## YAML Frontmatter

YAML frontmatter 不属于 CommonMark 或 GFM，因此默认不启用。启用 parser
construct 并挂载 `FrontmatterPlugin` 后，顶层 mapping 会渲染为
`DescriptionList`：

```rust
use gpui_component::text::{markdown, FrontmatterPlugin, MarkdownExtensions};

let extensions = MarkdownExtensions::default().frontmatter();

markdown("---\nname: example\ndescription: Example metadata.\n---")
    .markdown_extensions(extensions)
    .plugin(FrontmatterPlugin::new())
```

值以纯文本渲染。支持简单的无引号值，以及使用 `|-` 或 `>-` 的 block scalar；
literal scalar 会保留内容缩进。带引号的值、行尾注释、集合、别名、其他 block
header，以及包含额外缩进行的 folded scalar 会回退为 YAML code block，
保留原始内容，避免显示错误解析的值。

## 代码块操作

可以为 Markdown 代码块渲染操作控件：

```rust
markdown(source)
    .code_block_actions(|code_block, _window, _cx| {
        gpui_kit::div().child(format!("Run {}", code_block.lang().unwrap_or_default()))
    })
```
