use std::{
    collections::HashMap,
    ops::Range,
    path::PathBuf,
    process::Command,
    rc::Rc,
    sync::{Arc, Mutex, OnceLock},
};

use gpui_component_story::Open;
use gpui_kit::assets::Assets;
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _,
    avatar::Avatar,
    button::{Button, ButtonVariants as _},
    clipboard::Clipboard,
    h_flex,
    highlighter::Language,
    input::{
        DocumentRangeSemanticTokensProvider, Editor, EditorState, InputEvent, Rope, RopeExt,
        TabSize,
    },
    menu::{DropdownMenu as _, PopupMenuItem},
    resizable::{h_resizable, resizable_panel},
    status_bar::StatusBar,
    text::{
        InlineElement, InlineRenderContext, MarkdownNode, MarkdownParseContext, MarkdownPlugin,
        SelectionFormat, TextView, TextViewState, TextViewStyle, markdown_ast,
    },
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use lsp_types::{SemanticToken, SemanticTokenType, SemanticTokens, SemanticTokensLegend};
use regex::{Captures, Regex};

mod mention;

/// Markers, each mapped to a different `HighlightTheme` token-type name so
/// `TODO`, `FIXME`, … render in distinct colors.
const MARKERS: &[(&str, &str)] = &[
    ("TODO", "keyword"),
    ("FIXME", "string"),
    ("XXX", "number"),
    ("HACK", "function"),
    ("NOTE", "type"),
];

include!("../../fixtures/markdown_plugins.rs");

/// Serialize a table to CSV: `,` separated, quoting only cells that contain
/// `"`, `,` or a newline, with `"` doubled inside quotes.
fn table_to_csv(headers: &[String], rows: &[Vec<String>]) -> String {
    let field = |cell: &str| {
        if cell.contains(['"', ',', '\n']) {
            format!("\"{}\"", cell.replace('"', "\"\""))
        } else {
            cell.to_string()
        }
    };
    let line = |cells: &[String]| cells.iter().map(|c| field(c)).collect::<Vec<_>>().join(",");

    std::iter::once(line(headers))
        .chain(rows.iter().map(|row| line(row)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Serialize a table to TSV: tab separated, never quoted; tabs and newlines
/// inside a cell become literal `\t` / `\n` so rows stay intact.
fn table_to_tsv(headers: &[String], rows: &[Vec<String>]) -> String {
    let field = |cell: &str| {
        cell.replace('\t', "\\t")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    };
    let line = |cells: &[String]| {
        cells
            .iter()
            .map(|c| field(c))
            .collect::<Vec<_>>()
            .join("\t")
    };

    std::iter::once(line(headers))
        .chain(rows.iter().map(|row| line(row)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Example [`DocumentRangeSemanticTokensProvider`]: tags `TODO` / `FIXME` /
/// `XXX` / `HACK` / `NOTE` markers anywhere in the document, each with its
/// own semantic token type so they render in distinct theme colors.
///
/// Installed on `input_state.lsp.semantic_tokens_provider`, exactly like the
/// other LSP providers (`document_color_provider`, `hover_provider`, …). The
/// editor fetches it (debounced) on document change, caches the result, and
/// composes it into the render pipeline on top of the tree-sitter syntax
/// highlighting. This example scans synchronously and returns a ready task;
/// a real language server would return tokens from an async request, and a
/// heavy local parser (syntect, …) would offload to a background task.
struct MarkerHighlighter;

impl DocumentRangeSemanticTokensProvider for MarkerHighlighter {
    fn legend(&self) -> SemanticTokensLegend {
        SemanticTokensLegend {
            token_types: MARKERS
                .iter()
                .map(|(_, name)| SemanticTokenType::from(name.to_string()))
                .collect(),
            token_modifiers: vec![],
        }
    }

    fn semantic_tokens(
        &self,
        text: &Rope,
        range: Range<usize>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<SemanticTokens>> {
        // Scan the requested range and collect absolute
        // (line, character, length, token_type) hits. `token_type` indexes
        // the legend, so each marker gets its own color.
        let slice = text.slice(range.clone()).to_string();
        let mut hits: Vec<(u32, u32, u32, u32)> = Vec::new();
        for (token_type, (marker, _)) in MARKERS.iter().enumerate() {
            let mut from = 0;
            while let Some(rel) = slice[from..].find(marker) {
                let abs = range.start + from + rel;
                let pos = text.offset_to_position(abs);
                hits.push((
                    pos.line,
                    pos.character,
                    marker.chars().count() as u32,
                    token_type as u32,
                ));
                from += rel + marker.len();
            }
        }
        hits.sort_unstable();

        // Delta-encode into LSP semantic tokens — the exact format a real
        // language server returns from `textDocument/semanticTokens/range`.
        let mut data = Vec::with_capacity(hits.len());
        let (mut prev_line, mut prev_char) = (0u32, 0u32);
        for (line, character, length, token_type) in hits {
            let delta_line = line - prev_line;
            let delta_start = if delta_line == 0 {
                character - prev_char
            } else {
                character
            };
            data.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type,
                token_modifiers_bitset: 0,
            });
            prev_line = line;
            prev_char = character;
        }

        Task::ready(Ok(SemanticTokens {
            result_id: None,
            data,
        }))
    }
}

pub struct Example {
    input_state: Entity<EditorState>,
    text_view: Entity<TextViewState>,
    inline_math: InlineMathPlugin,
    preview_zoom: f32,
    /// When `true`, tables wrap cell content to fit the width; when `false`
    /// (the default), tables keep cells on one line and scroll horizontally.
    table_wrap: bool,
    /// Whether copying a selection yields the rendered text or its Markdown
    /// source.
    selection_format: SelectionFormat,
    _subscriptions: Vec<Subscription>,
}

const EXAMPLE: &str = include_str!("../../fixtures/test.md");

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            EditorState::new(window, cx)
                .language(Language::Markdown)
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 2,
                    ..Default::default()
                })
                .searchable(true)
                .placeholder("Enter your Markdown here...")
                .default_value(EXAMPLE)
        });

        // Install the example range semantic tokens provider, alongside the
        // other LSP providers. It highlights TODO/FIXME/… markers.
        input_state.update(cx, |state, cx| {
            state.lsp_mut().semantic_tokens_provider = Some(Rc::new(MarkerHighlighter));
            cx.notify();
        });

        // Focus the input on startup so that actions (e.g. Open) can bubble
        // up through this view's element tree and reach their handlers.
        let focus_handle = input_state.focus_handle(cx);
        window.defer(cx, move |window, cx| {
            focus_handle.focus(window, cx);
        });

        let _subscriptions =
            vec![cx.subscribe(&input_state, |_, _, _: &InputEvent, cx| cx.notify())];

        let text_view = cx.new(|cx| TextViewState::markdown(EXAMPLE, cx));
        let weak_view = text_view.downgrade();
        let inline_math = InlineMathPlugin::new(move |cx| {
            let _ = weak_view.update(cx, |view, cx| view.invalidate_inline_layout(cx));
        });
        Self {
            text_view,
            inline_math,
            preview_zoom: 1.0,
            input_state,
            // Default to horizontal scrolling for tables.
            table_wrap: false,
            selection_format: SelectionFormat::Plain,
            _subscriptions,
        }
    }

    /// Build the markdown style: tables scroll horizontally unless `table_wrap`
    /// is on, in which case the default wrapping layout is used.
    fn text_view_style(&self) -> TextViewStyle {
        if self.table_wrap {
            return TextViewStyle::default();
        }
        let mut table = StyleRefinement::default();
        table.overflow.x = Some(Overflow::Scroll);
        TextViewStyle::default().table(table)
    }

    fn on_action_open(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        let path = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: false,
            prompt: Some("Select a Markdown file".into()),
        });

        let input_state = self.input_state.clone();
        cx.spawn_in(window, async move |_, window| {
            let path = path.await.ok()?.ok()??.iter().next()?.clone();

            let content = std::fs::read_to_string(&path).ok()?;

            window
                .update(|window, cx| {
                    _ = input_state.update(cx, |this, cx| {
                        this.set_value(content, window, cx);
                    });
                })
                .ok();

            Some(())
        })
        .detach();
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let source = self.input_state.read(cx).value();
        self.inline_math.set_document(&source);
        self.text_view
            .update(cx, |state, cx| state.set_text(&source, cx));
        div()
            .id("editor")
            .size_full()
            .on_action(cx.listener(Self::on_action_open))
            .child(
                v_flex()
                    .size_full()
                    .child(
                        div().flex_1().overflow_hidden().child(
                            h_resizable("container")
                                .child(
                                    resizable_panel().child(
                                        div()
                                            .id("source")
                                            .size_full()
                                            .font_family(cx.theme().mono_font_family.clone())
                                            .text_size(cx.theme().mono_font_size)
                                            .child(
                                                Editor::new(&self.input_state)
                                                    .h(relative(1.))
                                                    .p_0()
                                                    .border_0(),
                                            ),
                                    ),
                                )
                                .child(
                                    resizable_panel().child(
                                        TextView::new(&self.text_view)
                                            .plugin(self.inline_math.clone())
                                            .plugin(mention::MentionPlugin)
                                            .code_block_actions(|code_block, _window, _cx| {
                                                let code = code_block.code();
                                                let lang = code_block.lang();

                                                h_flex()
                                                    .gap_1()
                                                    .child(
                                                        Clipboard::new("copy").value(code.clone()),
                                                    )
                                                    .when_some(lang, |this, lang| {
                                                        // Only show run terminal button for certain languages
                                                        if lang.as_ref() == "rust"
                                                            || lang.as_ref() == "python"
                                                        {
                                                            this.child(
                                                                Button::new("run-terminal")
                                                                    .icon(IconName::SquareTerminal)
                                                                    .ghost()
                                                                    .xsmall()
                                                                    .on_click(move |_, _, _cx| {
                                                                        println!(
                                                                            "Running {} code: {}",
                                                                            lang, code
                                                                        );
                                                                    }),
                                                            )
                                                        } else {
                                                            this
                                                        }
                                                    })
                                            })
                                            .table_actions(|table, _window, cx| {
                                                // The hook hands over the table as plain data:
                                                // header cells, body rows, and the table
                                                // re-serialized to GFM Markdown.
                                                //
                                                // This runs on every render, so only cheap
                                                // clones belong here — CSV / TSV are built
                                                // inside the click handlers instead.
                                                let markdown = table.markdown.clone();
                                                let headers = table.headers.clone();
                                                let rows = table.rows.clone();
                                                // Plain ids are fine: the actions row is scoped
                                                // per table by the component.
                                                let shape = format!(
                                                    "{} × {}",
                                                    table.rows.len(),
                                                    table.headers.len()
                                                );

                                                h_flex()
                                                    .w_full()
                                                    .justify_end()
                                                    .items_center()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(shape),
                                                    )
                                                    .child(
                                                        Clipboard::new("copy-table")
                                                            .value(markdown.clone())
                                                            .tooltip("Copy as Markdown"),
                                                    )
                                                    .child(
                                                        Button::new("export-table")
                                                            .icon(IconName::Ellipsis)
                                                            .ghost()
                                                            .xsmall()
                                                            .dropdown_menu_with_anchor(
                                                                Anchor::TopRight,
                                                                move |menu, _window, _cx| {
                                                                    // The builder is a `Fn`, so
                                                                    // captured values are cloned
                                                                    // on every rebuild.
                                                                    let (csv_headers, csv_rows) =
                                                                        (headers.clone(), rows.clone());
                                                                    let (tsv_headers, tsv_rows) =
                                                                        (headers.clone(), rows.clone());

                                                                    menu.item(
                                                                        PopupMenuItem::new(
                                                                            "Copy as CSV",
                                                                        )
                                                                        .on_click(
                                                                            move |_, _, cx| {
                                                                                let csv = table_to_csv(&csv_headers, &csv_rows);
                                                                                cx.write_to_clipboard(ClipboardItem::new_string(csv));
                                                                            },
                                                                        ),
                                                                    )
                                                                    .item(
                                                                        PopupMenuItem::new(
                                                                            "Copy as TSV",
                                                                        )
                                                                        .on_click(
                                                                            move |_, _, cx| {
                                                                                let tsv = table_to_tsv(&tsv_headers, &tsv_rows);
                                                                                cx.write_to_clipboard(ClipboardItem::new_string(tsv));
                                                                            },
                                                                        ),
                                                                    )
                                                                },
                                                            ),
                                                    )
                                            })
                                            .plugin(TickerPlugin::new(
                                                TickerQuote {
                                                    name: "Apple Inc.",
                                                    price: 300.21,
                                                    change: 5.2,
                                                },
                                                TickerQuote {
                                                    name: "Tesla, Inc.",
                                                    price: 412.05,
                                                    change: -2.13,
                                                },
                                            ))
                                            .plugin(UserCardPlugin::new())
                                            .plugin(MathPlugin::new())
                                            .on_link_click(|url, event, _window, cx| {
                                                println!(
                                                    "Markdown link clicked: {url} ({event:?})"
                                                );
                                                if !event.is_right_click() {
                                                    cx.open_url(url);
                                                }
                                            })
                                            // Tables scroll horizontally by default; the
                                            // status bar toggle switches to wrapping.
                                            .style(self.text_view_style())
                                            .text_size(rems(self.preview_zoom))
                                            .flex_none()
                                            .px_5()
                                            .scrollable(true)
                                            .selectable(true)
                                            .selection_format(self.selection_format),
                                    ),
                                ),
                        ),
                    )
                    .child(
                        StatusBar::new()
                            .right(
                                Button::new("preview-zoom")
                                    .ghost()
                                    .xsmall()
                                    .label(format!("Zoom: {:.0}%", self.preview_zoom * 100.0))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.preview_zoom = if this.preview_zoom >= 2.0 { 0.75 } else { this.preview_zoom + 0.25 };
                                        this.text_view.update(cx, |view, cx| view.invalidate_inline_layout(cx));
                                        cx.notify();
                                    })),
                            )
                            .right(
                                Button::new("selection-format")
                                    .ghost()
                                    .xsmall()
                                    .label(match self.selection_format {
                                        SelectionFormat::Plain => "Selection: Plain",
                                        SelectionFormat::Source => "Selection: Source",
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.selection_format = match this.selection_format {
                                            SelectionFormat::Plain => SelectionFormat::Source,
                                            SelectionFormat::Source => SelectionFormat::Plain,
                                        };
                                        cx.notify();
                                    })),
                            )
                            .right(
                                Button::new("table-wrap")
                                    .ghost()
                                    .xsmall()
                                    .label(if self.table_wrap {
                                        "Table: Wrap"
                                    } else {
                                        "Table: Scroll"
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.table_wrap = !this.table_wrap;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        gpui_component_story::init(cx);
        cx.activate(true);

        gpui_component_story::create_new_window("Markdown Editor", Example::view, cx);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn prose_edits_retain_prepared_and_pending_formula_resources() {
        let image = Arc::new(Image::from_bytes(ImageFormat::Svg, b"<svg/>".to_vec()));
        let mut cache = InlineMathCache::default();
        cache.images.insert(
            "x^2\0theme".into(),
            Some(RenderedMathImage {
                image: image.clone(),
                width: 20.,
                height: 20.,
                baseline: 15.,
            }),
        );
        cache.images.insert("y^2\0theme".into(), None);
        cache.images.insert("z^2\0theme".into(), None);
        cache.set_document("Prose edited, still $x^2$ and $y^2$.");
        assert!(Arc::ptr_eq(
            &cache.images["x^2\0theme"].as_ref().unwrap().image,
            &image
        ));
        assert!(cache.images.contains_key("y^2\0theme"));
        assert!(!cache.images.contains_key("z^2\0theme"));
        cache.set_document("No formulas remain.");
        assert!(cache.images.is_empty());
    }

    #[::core::prelude::v1::test]
    fn math_fallback_text_prettifies_formula_text() {
        assert_eq!(
            prettify_math_source(r"\alpha + \beta"),
            "\u{03b1} + \u{03b2}"
        );
    }

    #[::core::prelude::v1::test]
    #[ignore = "requires Node.js and docs/node_modules/mathjax-full"]
    fn math_svg_uses_path_based_renderer() {
        let svg = render_math_svg(
            r"e^{i\pi} + 1 = 0",
            true,
            20.0,
            Hsla::black(),
            Hsla::white(),
        )
        .expect("install mathjax-full in docs/node_modules before running ignored MathJax tests");

        assert!(
            !svg.contains("<text"),
            "math SVG should not estimate font text bounds: {svg}"
        );
        assert!(
            svg.contains("<path"),
            "math SVG should contain renderer-generated glyph paths: {svg}"
        );
    }

    #[::core::prelude::v1::test]
    fn inline_math_fallback_text_compacts_script_source() {
        assert_eq!(
            prettify_math_source(r"e^{i\pi} + 1 = 0"),
            "e\u{2071}\u{03c0} + 1 = 0"
        );
    }

    #[::core::prelude::v1::test]
    #[ignore = "requires Node.js and docs/node_modules/mathjax-full"]
    fn inline_math_svg_uses_pixel_dimensions_without_glyph_scaling() {
        let svg = render_math_svg(
            r"e^{i\pi} + 1 = 0",
            true,
            20.0,
            Hsla::black(),
            Hsla::white(),
        )
        .expect("install mathjax-full in docs/node_modules before running ignored MathJax tests");

        assert!(svg_width(&svg) > 0.0);
        assert!(!svg.contains("data-gpui-math-background"));
        let height = svg_attr(&svg, "height").unwrap().parse().unwrap();
        assert!(svg_baseline(&svg, height).unwrap() > 0.0);
        assert!(!svg_attr(&svg, "width").unwrap().ends_with("ex"));
        assert!(!svg_attr(&svg, "height").unwrap().ends_with("ex"));
        assert!(!svg.contains("vertical-align"));
        assert!(!svg.contains("currentColor"));
        assert!(!svg.contains("lengthAdjust="));
        assert!(!svg.contains("textLength="));
    }

    #[::core::prelude::v1::test]
    #[ignore = "requires Node.js and docs/node_modules/mathjax-full"]
    fn block_math_svg_rewrites_mathjax_rects_to_paths() {
        let svg = render_math_svg(
            r"\frac{\alpha + \beta}{\sqrt{\gamma}} = \sum_{i=1}^{n} i^2",
            false,
            20.0,
            Hsla::black(),
            Hsla::white(),
        )
        .expect("install mathjax-full in docs/node_modules before running ignored MathJax tests");

        assert!(!svg.contains(r#"<rect width="543""#));
        assert!(!svg.contains(r#"<rect width="2628.4""#));
        assert!(svg.contains("<path"));
    }

    #[::core::prelude::v1::test]
    #[ignore = "requires Node.js and docs/node_modules/mathjax-full"]
    fn block_math_svg_includes_theme_background() {
        let svg = render_math_svg(
            r"\frac{\alpha + \beta}{\sqrt{\gamma}} = \sum_{i=1}^{n} i^2",
            false,
            20.0,
            Hsla::black(),
            Hsla::white(),
        )
        .expect("install mathjax-full in docs/node_modules before running ignored MathJax tests");

        assert!(svg.contains(r#"data-gpui-math-background="true""#));
        assert!(svg.contains("fill=\"#ffffff\""));
    }

    #[::core::prelude::v1::test]
    #[ignore = "requires Node.js and docs/node_modules/mathjax-full"]
    fn block_math_image_exposes_intrinsic_svg_size() {
        let image = render_math_image(
            r"\frac{\alpha + \beta}{\sqrt{\gamma}} = \sum_{i=1}^{n} i^2",
            false,
            20.0,
            Hsla::black(),
            Hsla::white(),
        )
        .expect("install mathjax-full in docs/node_modules before running ignored MathJax tests");

        assert_eq!(image.image.format, ImageFormat::Svg);
        assert!(image.image.bytes.starts_with(b"<svg"));
        assert!(image.width > 0.0);
        assert!(image.height > 0.0);
        assert!(image.width > image.height);
    }

    #[::core::prelude::v1::test]
    fn math_markdown_preserves_inline_delimiters() {
        assert_eq!(math_markdown("x^2", true), "$x^2$");
    }

    #[::core::prelude::v1::test]
    fn block_math_source_extracts_dollar_fence_body() {
        assert_eq!(
            block_math_source(
                "$$\n\\frac{\\alpha + \\beta}{\\sqrt{\\gamma}} = \\sum_{i=1}^{n} i^2\n$$",
            ),
            Some(r"\frac{\alpha + \beta}{\sqrt{\gamma}} = \sum_{i=1}^{n} i^2")
        );
    }

    #[::core::prelude::v1::test]
    fn inline_math_baseline_uses_view_box_origin() {
        let svg = r#"<svg viewBox="0 -800 1200 1000"></svg>"#;
        assert_eq!(svg_baseline(svg, 20.0), Some(16.0));
        assert_eq!(svg_baseline(svg, 40.0), Some(32.0));
        assert_eq!(svg_baseline(r#"<svg viewBox="0 0 0 0">"#, 20.0), None);
    }

    #[::core::prelude::v1::test]
    fn math_node_distinguishes_plain_markdown_and_accessibility() {
        let node = math_node("x^2".to_string(), true, "$x^2$");
        assert_eq!(node.name(), "inline-math");
        assert_eq!(node.as_text(), "x²");
        assert_eq!(node.as_markdown(), "$x^2$");
        assert_eq!(node.accessibility_name(), "Formula: x²");
    }

    #[::core::prelude::v1::test]
    fn inline_plugin_accepts_math_and_leaves_code_and_paragraphs_to_markdown() {
        let node = markdown_ast::Node::InlineMath(markdown_ast::InlineMath {
            value: "x^2".into(),
            position: None,
        });
        let parsed = parse_inline_math(&node, Some("$ x^2 $")).unwrap();
        assert_eq!(parsed.as_markdown(), "$ x^2 $");
        assert_eq!(parsed.as_text(), "x²");
        let code = markdown_ast::Node::InlineCode(markdown_ast::InlineCode {
            value: "$x^2$".into(),
            position: None,
        });
        assert!(parse_inline_math(&code, None).is_none());
        let paragraph = markdown_ast::Node::Paragraph(markdown_ast::Paragraph {
            children: vec![node],
            position: None,
        });
        assert!(parse_inline_math(&paragraph, None).is_none());
    }

    fn svg_width(svg: &str) -> f32 {
        let start = svg.find("width=\"").unwrap() + "width=\"".len();
        let end = svg[start..].find('"').unwrap();
        svg[start..start + end].parse().unwrap()
    }
}
