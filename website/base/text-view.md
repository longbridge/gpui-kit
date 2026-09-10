---
title: TextView
description: Render selectable Markdown and HTML directly with gpui-base.
order: 4
example: text-view
exampleKind: base
---

# TextView

`gpui-base` owns the complete `TextView` implementation for rendering Markdown and common HTML. It includes document parsing, links, images, lists, tables, code blocks, scrolling, line clamping, plugins, selection, and copying without depending on `gpui-component`.

The live example above uses only `gpui-base`. Its fenced Rust block is intentionally unhighlighted: syntax highlighting is opt-in.

## Set up the window

Call `gpui_kit::base::init` once during application startup and render one `TextSelectionLayer` per window. The layer coordinates selection across `TextView`, [`SelectableText`](./text-selection.md), and custom text renderers.

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
                "# Hello\n\nSelect and copy this **Markdown**.",
            ))
    }
}
```

If the application already calls `gpui_kit::component::init`, Base initialization is included. `gpui-component::Root` also installs the window selection layer.

TextView is selectable by default. While dragging a selection near a viewport edge, the shared selection layer scrolls the related `overflow_*_scroll` region automatically; no TextView scroll or selection parameter is required. Use `.selectable(false)` only to disable selection explicitly.

## Markdown and HTML

Use the helpers for call-site-derived IDs, or constructors when an explicit stable ID is useful:

```rust
use gpui_kit::base::{html, markdown, TextView};

let short_markdown = markdown("A **short** message.");
let short_html = html("<p>A <strong>short</strong> message.</p>");

let preview = TextView::markdown("document-preview", markdown_source).scrollable(true);

let article = TextView::html("article", html_source);
```

`scrollable(true)` makes the view fill its container and scroll vertically. Without it, the view grows to fit its content. `max_lines(n)` clamps a non-scrollable preview to at most `n` body-text lines.

## Complete default styling

Every constructor starts with `TextViewStyle::default()`. The default contains readable neutral foreground, muted, link, selection, code-background, border, heading, paragraph, inline-code, and table styles. A Base-only application does not need to construct a style before rendering text.

Override only the values owned by your design system:

```rust
use gpui_kit::base::TextViewStyle;

let style = TextViewStyle::default()
    .with_foreground(app_colors.foreground)
    .with_muted_foreground(app_colors.muted_foreground)
    .with_link(app_colors.link)
    .with_selection(app_colors.selection);

TextView::markdown("themed", source).style(style)
```

`TextViewStyle::from_theme(&theme)` maps the semantic colors from a `gpui_kit::base::Theme`. Applications using the higher-level component theme can use `gpui_kit::component::text::text_view_style(cx.theme())`.

## Syntax highlighting is opt-in

`gpui-base` does not enable syntax highlighting and has no tree-sitter language dependency. Fenced code blocks use the neutral code surface and plain foreground until the application supplies `code_block_highlighter`.

The callback receives a `CodeBlock` and returns byte ranges paired with GPUI `HighlightStyle` values:

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

Ranges are UTF-8 byte ranges relative to `CodeBlock::code()`. Invalid ranges are discarded. The highlighter implementation and its language registrations remain entirely application-owned.

## Markdown extensions

`MarkdownExtensions` starts with CommonMark/GFM-compatible parsing. YAML
frontmatter is disabled by default because it is not part of either standard.
Enable the construct explicitly when a block parser or plugin handles
`markdown_ast::Node::Yaml`:

```rust
use gpui_base::{MarkdownExtensions, TextView};

let extensions = MarkdownExtensions::default().frontmatter();

TextView::markdown("metadata", source)
    .markdown_extensions(extensions)
```

Without a matching plugin, enabled YAML frontmatter uses the existing YAML
code-block fallback. A custom plugin can be attached with `.plugin(...)`;
`gpui-component` provides a themed
`FrontmatterPlugin`; Base remains independent of that presentation.

## Inline extensions

Inline extensions follow the block extension mechanism: register parsers in order and renderers by `MarkdownNode::name()`. A `MarkdownPlugin` with the default `is_block() == false` uses `render_inline`; block plugins keep `render`.

```rust
use gpui_base::{MarkdownExtensions, MarkdownNode, TextView, markdown_ast};

let extensions = MarkdownExtensions::default()
    .math()
    .inline_parser(|node, _| {
        let markdown_ast::Node::InlineMath(math) = node else { return None };
        Some(MarkdownNode::new("formula", math.value.clone())
            .text(math.value.clone())
            .accessibility_label(format!("Formula: {}", math.value)))
    });

TextView::markdown("inline-formulas", "Formulas $x^2$ and $y^2$")
    .markdown_extensions(extensions)
```

This renders an atomic text fallback. Register `inline_renderer("formula", ...)` to return `Some(MarkdownInlinePresentation::image(image, metrics))` for a prepared `Arc<gpui::Image>`. `MarkdownInlineMetrics::new(size(width, height), baseline)` uses logical pixels at the current font size; the baseline is measured from the top. The renderer receives `MarkdownInlineRenderContext` with the current text style, font size, line height, rem size, and available width. Returning `None` or invalid metrics uses the text fallback. Render callbacks should read prepared resources; do not run an equation engine synchronously during layout.

`MarkdownInlinePresentation::text()` keeps the node's plain text in the inline flow. Add `.hover_card(|window, cx| ...)` to build a read-only `AnyView` in a HoverCard centered horizontally below the inline object. The card is created on hover, outside inline layout, and must not contain focusable controls. Use `text_color`, `font_weight`, `background`, `hover_background`, `padding_x`, and `rounded` to style the object. Horizontal padding participates in measurement, wrapping, selection, and proportional shrinking; background colors remain beneath selection highlighting. The Markdown example uses this for `[@huacnlee](mention:huacnlee)` profile cards; plain copy emits the handle and Markdown copy retains the original link syntax.

Objects align to the text baseline and wrap only before or after the whole object. An object wider than the available line shrinks proportionally, including its baseline. Objects are read-only, have no focus stop or internal controls, and are selected as a whole. Double-click selects an object; triple-click selects its mixed text line. Drag selection can cross text and consecutive objects in either direction.

`source_range()` exposes full-document UTF-8 byte offsets including delimiters. `.text(...)` supplies plain copy and fallback text; `.markdown(...)` supplies Markdown copy, defaulting to the original node source. Missing plain text falls back to source. `.accessibility_label(...)` supplies the accessible name, defaulting to the plain text. Image loading/failure retains a visible text fallback and the same atomic copying contract.

For asynchronous resources, retain a `TextViewState`, update the application-owned cache, then call `state.invalidate_inline_layout(cx)` through the view's weak entity. This remeasures inline content and virtual-list heights without reparsing or dropping the current logical selection. Associate results with source/font/theme keys and discard obsolete completions. `examples/markdown` contains the formula implementation and a preview zoom control.

`TextView` also offers `markdown_math`, `markdown_inline_parser`, and `markdown_inline_renderer` convenience builders. Math parsing is opt-in; inline code continues to protect dollar signs from math parsing.

## Retained state and streaming updates

Use `TextViewState` when content changes without replacing the view:

```rust
use gpui_kit::base::{TextView, TextViewState};

let document = cx.new(|cx| TextViewState::markdown(initial_source, cx));

// Render
TextView::new(&document)

// Later
document.update(cx, |state, cx| state.set_text(updated_source, cx));
```

Selection can copy rendered text or Markdown source through `SelectionFormat`. Link routing, code-block actions, table actions, images, and custom Markdown plugins use the same builders as the compatibility API documented on the [gpui-component TextView page](../component/text-view.md).

## Runnable source

The live preview and native command use the same Base-only source:

<<< ../../crates/base/examples/showcase/components/text_view.rs{rust}

```bash
cargo run -p gpui-base-examples -- text-view
```
