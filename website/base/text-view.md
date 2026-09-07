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

Selection can copy rendered text or Markdown source through `SelectionFormat`. Link routing, code-block actions, table actions, images, and custom Markdown plugins use the same builders as the compatibility API documented on the [gpui-component TextView page](../docs/components/text-view.md).

## Runnable source

The live preview and native command use the same Base-only source:

<<< ../../crates/base/examples/showcase/components/text_view.rs{rust}

```bash
cargo run -p gpui-base-examples -- text-view
```
