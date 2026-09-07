---
title: TextView
description: Renders Markdown and HTML text with optional custom Markdown plugins.
---

# TextView

`TextView` renders formatted text in GPUI. It supports Markdown and simple HTML, text selection, code block actions, and custom Markdown plugins for project-specific syntax.

The canonical implementation now lives in `gpui-base`; this module remains a compatibility re-export and provides component-theme adaptation. Base-only setup, complete default styling, and opt-in syntax highlighting are documented on [GPUI Base TextView](/base/text-view).

TextView is selectable by default and uses the shared window selection engine from `gpui-base`. Use `.selectable(false)` only when selection must be disabled. See [GPUI Base Text Selection](/base/text-selection) when integrating plain text or a custom renderer with the same selection.

## Import

```rust
use gpui_kit::component::text::{markdown, TextView};
```

## Usage

### Markdown

Use the `markdown` helper when you only need to render Markdown text:

```rust
use gpui_kit::component::text::markdown;

markdown("# Hello\n\nThis is **Markdown**.")
    .scrollable(true)
```

You can also construct a `TextView` directly when you need a stable id:

```rust
use gpui_kit::component::text::TextView;

TextView::markdown("preview", markdown_source)
```

### HTML

```rust
TextView::html("html-preview", "<strong>Hello</strong>")
```

### Clamp to a number of lines

Use `max_lines` to render a bounded preview of rich content — for example a
collapsed "show more" section. The view's height is capped at `n` × the base
line height, and a line of glyphs is never cut in half: a line that would
straddle the bottom of the box is left out whole, across paragraphs, lists,
headings, code blocks and tables:

```rust
TextView::markdown("preview", markdown_source).max_lines(5)
```

Nothing is shown with less than a line of itself to show, so the border and
padding a table row leads with never strands at the bottom. Whatever has more
than that is cut on the box edge and keeps the part that fits, so an image
crossing the edge shows instead of disappearing and leaving blank space
behind.

`TextViewState::is_clamped()` reports whether the previous painted frame
actually clipped content, so the caller can decide whether to render an
"expand" affordance. `n` counts lines of body text, so paragraph spacing and
taller lines mean fewer of them fit inside the capped height, and a line taller
than the whole budget keeps the part that fits rather than emptying the box.
`max_lines` only applies to the fit-content mode and is ignored when
`scrollable` is set.

## Link Click Handling

Use `on_link_click` when links should be routed by the application instead of
being opened directly by `App::open_url`. The callback receives the resolved
URL and the original GPUI `ClickEvent`, so it can distinguish mouse buttons,
keyboard activation, touch, and modifier keys:

```rust
use gpui_kit::ClickEvent;
use gpui_kit::component::text::markdown;

markdown("[Open the project](https://github.com/longbridge/gpui-kit)")
    .on_link_click(|url, event, _window, cx| {
        if event.is_right_click() {
            println!("Show a context menu for {url}");
            return;
        }

        match event {
            ClickEvent::Mouse(click) if click.up.modifiers.control => {
                println!("Open {url} in an internal view");
            }
            _ => cx.open_url(url),
        }
    })
```

Installing a handler consumes the link event and disables the default URL
opening behavior. If no handler is installed, links continue to use
`App::open_url` as usual. The callback is used for both text links and linked
images.

## Markdown Plugins

Use `.plugin(...)` to support custom Markdown formats. A plugin owns both parsing and rendering, so callers only need to attach it to the `TextView`:

```rust
markdown(source)
    .plugin(TickerPlugin::new())
```

A Markdown plugin implements `MarkdownPlugin`:

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

Then attach it to a Markdown `TextView`:

```rust
markdown("$AAPL.US")
    .plugin(TickerPlugin::new())
```

## MarkdownNode

`MarkdownNode` is the neutral data passed between `parse` and `render`.

```rust
MarkdownNode::new("ticker", TickerNode { symbol })
    .text("$AAPL.US")
    .markdown("$AAPL.US")
```

- `name` is the stable node name used to match the renderer.
- `data` is typed parser output read with `node.data::<T>()`.
- `text` is the plain text representation used by selection and fallback rendering.
- `markdown` is the Markdown representation used when the document is serialized back to Markdown.

## Block Plugins

Custom Markdown rendering currently supports block plugins. Return `true` from `is_block()` for plugins that should be registered today:

```rust
fn is_block(&self) -> bool {
    true
}
```

Inline plugins are reserved for future `TextView` support.

## Code Block Actions

You can render controls for Markdown code blocks:

```rust
markdown(source)
    .code_block_actions(|code_block, _window, _cx| {
        gpui_kit::div().child(format!("Run {}", code_block.lang().unwrap_or_default()))
    })
```
