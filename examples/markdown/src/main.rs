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
        MarkdownNode, MarkdownParseContext, MarkdownPlugin, SelectionFormat, TextViewStyle,
        markdown, markdown_ast,
    },
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use lsp_types::{SemanticToken, SemanticTokenType, SemanticTokens, SemanticTokensLegend};
use regex::{Captures, Regex};

/// Markers, each mapped to a different `HighlightTheme` token-type name so
/// `TODO`, `FIXME`, … render in distinct colors.
const MARKERS: &[(&str, &str)] = &[
    ("TODO", "keyword"),
    ("FIXME", "string"),
    ("XXX", "number"),
    ("HACK", "function"),
    ("NOTE", "type"),
];

#[derive(Clone)]
struct TickerNode {
    symbol: String,
}

#[derive(Clone)]
struct UserCardNode {
    id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MathNode {
    Formula { source: String, inline: bool },
    Paragraph { segments: Vec<MathSegment> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MathSegment {
    source: String,
    math: bool,
}

#[derive(Clone, Copy)]
struct TickerQuote {
    name: &'static str,
    price: f64,
    change: f64,
}

#[derive(Clone)]
struct TickerPlugin {
    apple_quote: TickerQuote,
    tesla_quote: TickerQuote,
}

impl TickerPlugin {
    fn new(apple_quote: TickerQuote, tesla_quote: TickerQuote) -> Self {
        Self {
            apple_quote,
            tesla_quote,
        }
    }

    fn quote(&self, symbol: &str) -> TickerQuote {
        match symbol {
            "AAPL.US" => self.apple_quote,
            "TSLA.US" => self.tesla_quote,
            _ => TickerQuote {
                name: "Unknown",
                price: 0.0,
                change: 0.0,
            },
        }
    }
}

#[derive(Clone)]
struct UserCardPlugin;

#[derive(Clone)]
struct MathPlugin;

#[derive(Clone)]
struct RenderedMathImage {
    image: Arc<Image>,
    width: f32,
    height: f32,
}

impl MathPlugin {
    fn new() -> Self {
        Self
    }
}

impl UserCardPlugin {
    fn new() -> Self {
        Self
    }
}

fn mdx_attr(attrs: &[markdown_ast::AttributeContent], name: &str) -> Option<String> {
    attrs.iter().find_map(|attr| match attr {
        markdown_ast::AttributeContent::Property(prop) if prop.name == name => {
            match prop.value.as_ref() {
                Some(markdown_ast::AttributeValue::Literal(value)) => Some(value.clone()),
                _ => None,
            }
        }
        _ => None,
    })
}

fn html_tag_name(value: &str) -> Option<&str> {
    value
        .trim()
        .strip_prefix('<')?
        .split([' ', '/', '>'])
        .next()
}

fn html_attr(value: &str, name: &str) -> Option<String> {
    let pattern = format!("{name}=\"");
    let start = value.find(&pattern)? + pattern.len();
    let end = value[start..].find('"')?;
    Some(value[start..start + end].to_string())
}

fn ticker_symbol(value: &str) -> Option<&str> {
    let symbol = value.strip_prefix('$')?;
    if symbol.is_empty()
        || !symbol.contains('.')
        || !symbol
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.')
    {
        return None;
    }
    Some(symbol)
}

fn math_markdown(source: &str, inline: bool) -> String {
    if inline {
        format!("${source}$")
    } else {
        format!("$$\n{source}\n$$")
    }
}

fn math_node(source: String, inline: bool, markdown: impl Into<String>) -> MarkdownNode {
    MarkdownNode::new(
        "math",
        MathNode::Formula {
            source: source.clone(),
            inline,
        },
    )
    .text(math_markdown(&source, inline))
    .markdown(markdown.into())
}

fn math_paragraph_node(markdown: &str, segments: Vec<MathSegment>) -> MarkdownNode {
    MarkdownNode::new("math", MathNode::Paragraph { segments })
        .text(markdown.to_string())
        .markdown(markdown.to_string())
}

fn block_math_source(source: &str) -> Option<&str> {
    let source = source.trim();
    let body = source.strip_prefix("$$")?.strip_suffix("$$")?.trim();
    (!body.is_empty()).then_some(body)
}

fn inline_math_segments(source: &str) -> Option<Vec<MathSegment>> {
    let mut segments = Vec::new();
    let mut text_start = 0;
    let mut ix = 0;
    let mut code_ticks = None;

    while ix < source.len() {
        if let Some(ticks) = count_run(source, ix, b'`') {
            if code_ticks == Some(ticks) {
                code_ticks = None;
            } else if code_ticks.is_none() {
                code_ticks = Some(ticks);
            }
            ix += ticks;
            continue;
        }

        if code_ticks.is_none()
            && source.as_bytes()[ix] == b'$'
            && !is_escaped(source, ix)
            && source.as_bytes().get(ix + 1) != Some(&b'$')
            && let Some(end_ix) = find_inline_math_end(source, ix + 1)
        {
            let math = source[ix + 1..end_ix].trim();
            if !math.is_empty() {
                if text_start < ix {
                    segments.push(MathSegment {
                        source: source[text_start..ix].to_string(),
                        math: false,
                    });
                }
                segments.push(MathSegment {
                    source: math.to_string(),
                    math: true,
                });
                ix = end_ix + 1;
                text_start = ix;
                continue;
            }
        }

        ix += source[ix..].chars().next().map_or(1, char::len_utf8);
    }

    if segments.iter().all(|segment| !segment.math) {
        return None;
    }

    if text_start < source.len() {
        segments.push(MathSegment {
            source: source[text_start..].to_string(),
            math: false,
        });
    }

    Some(segments)
}

fn find_inline_math_end(source: &str, mut ix: usize) -> Option<usize> {
    while ix < source.len() {
        if source.as_bytes()[ix] == b'$'
            && !is_escaped(source, ix)
            && source.as_bytes().get(ix + 1) != Some(&b'$')
        {
            return Some(ix);
        }
        ix += source[ix..].chars().next().map_or(1, char::len_utf8);
    }
    None
}

fn count_run(source: &str, ix: usize, needle: u8) -> Option<usize> {
    if source.as_bytes().get(ix) != Some(&needle) {
        return None;
    }

    let mut end = ix + 1;
    while source.as_bytes().get(end) == Some(&needle) {
        end += 1;
    }
    Some(end - ix)
}

fn is_escaped(source: &str, ix: usize) -> bool {
    let mut backslashes = 0;
    let mut cursor = ix;
    while cursor > 0 && source.as_bytes()[cursor - 1] == b'\\' {
        backslashes += 1;
        cursor -= 1;
    }
    backslashes % 2 == 1
}

impl MarkdownPlugin for MathPlugin {
    fn is_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "math"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        if let markdown_ast::Node::Math(math) = node {
            return Some(math_node(
                math.value.clone(),
                false,
                cx.node_source(node)
                    .map(str::to_string)
                    .unwrap_or_else(|| math_markdown(&math.value, false)),
            ));
        }

        let markdown_ast::Node::Paragraph(_) = node else {
            return None;
        };
        let source = cx.node_source(node)?;

        if let Some(math) = block_math_source(source) {
            return Some(math_node(math.to_string(), false, source));
        }

        inline_math_segments(source).map(|segments| math_paragraph_node(source, segments))
    }

    fn render(&self, node: &MarkdownNode, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let math = node.data::<MathNode>().expect("math markdown node data");
        let font_size = f32::from(window.text_style().font_size.to_pixels(window.rem_size()));

        match math {
            MathNode::Formula { source, inline } => {
                let content = render_math_formula(source, *inline, font_size, cx);
                if *inline {
                    content
                } else {
                    div()
                        .w_full()
                        .flex()
                        .justify_center()
                        .py_1()
                        .child(content)
                        .into_any_element()
                }
            }
            MathNode::Paragraph { segments } => h_flex()
                .w_full()
                .flex_wrap()
                .items_center()
                .children(segments.iter().map(|segment| {
                    if segment.math {
                        render_math_formula(&segment.source, true, font_size, cx)
                    } else {
                        div()
                            .line_height(relative(1.5))
                            .child(segment.source.clone())
                            .into_any_element()
                    }
                }))
                .into_any_element(),
        }
    }
}

fn render_math_formula(source: &str, inline: bool, font_size: f32, cx: &mut App) -> AnyElement {
    if let Some(image) = render_math_image(
        source,
        inline,
        font_size,
        cx.theme().foreground,
        cx.theme().background,
    ) {
        img(image.image)
            .object_fit(ObjectFit::Contain)
            .flex_shrink_0()
            .w(px(image.width))
            .h(px(image.height))
            .into_any_element()
    } else {
        render_math_text(source, inline, font_size, cx.theme().foreground)
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
        let symbol = ticker_symbol(&text.value)?;
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

    fn render(&self, node: &MarkdownNode, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ticker = node
            .data::<TickerNode>()
            .expect("ticker markdown node data");
        let symbol = ticker.symbol.as_str();
        let quote = self.quote(symbol);
        let up = quote.change >= 0.0;
        let trend = if up { cx.theme().green } else { cx.theme().red };

        v_flex()
            .w(px(240.))
            .gap_1p5()
            .px_3()
            .py_2()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .line_height(relative(1.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(format!("${symbol}")),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .line_height(relative(1.))
                                    .text_color(cx.theme().muted_foreground)
                                    .child(quote.name),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_0p5()
                            .px_1()
                            .py_0p5()
                            .rounded(cx.theme().radius)
                            .bg(trend.opacity(0.12))
                            .text_xs()
                            .line_height(relative(1.))
                            .text_color(trend)
                            .child(
                                Icon::new(if up {
                                    IconName::ArrowUp
                                } else {
                                    IconName::ArrowDown
                                })
                                .xsmall(),
                            )
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(format!("{:+.1}%", quote.change)),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_lg()
                            .line_height(relative(1.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("{:.2}", quote.price)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .line_height(relative(1.))
                            .text_color(cx.theme().muted_foreground)
                            .child("Last"),
                    ),
            )
    }
}

impl MarkdownPlugin for UserCardPlugin {
    fn is_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "user-card"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        match node {
            markdown_ast::Node::MdxJsxFlowElement(element)
                if element.name.as_deref() == Some("UserCard") =>
            {
                let id = mdx_attr(&element.attributes, "id")?;
                Some(
                    MarkdownNode::new("user-card", UserCardNode { id: id.clone() })
                        .text(id)
                        .markdown(cx.node_source(node).unwrap_or_default()),
                )
            }
            markdown_ast::Node::Html(raw) if html_tag_name(&raw.value) == Some("UserCard") => {
                let id = html_attr(&raw.value, "id")?;
                Some(
                    MarkdownNode::new("user-card", UserCardNode { id: id.clone() })
                        .text(id)
                        .markdown(cx.node_source(node).unwrap_or(raw.value.as_str())),
                )
            }
            _ => None,
        }
    }

    fn render(&self, node: &MarkdownNode, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let user = node
            .data::<UserCardNode>()
            .expect("user-card markdown node data");
        let id = user.id.as_str();
        let (name, avatar) = match id {
            "huacnlee" => (
                "Jason Lee",
                "https://avatars.githubusercontent.com/u/5518?v=4",
            ),
            "madcodelife" => (
                "Floyd Wang",
                "https://avatars.githubusercontent.com/u/28998859?v=4",
            ),
            _ => ("Unknown", ""),
        };

        let following = window.use_keyed_state(
            SharedString::from(format!("user-card-follow-{id}")),
            cx,
            |_, _| false,
        );
        let is_following = *following.read(cx);

        h_flex()
            .w(px(300.))
            .items_center()
            .gap_3()
            .px_3()
            .py_2()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(
                Avatar::new()
                    .name(name)
                    .with_size(px(24.))
                    .when(!avatar.is_empty(), |this| this.src(avatar)),
            )
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(name),
            )
            .child(
                Button::new(SharedString::from(format!("follow-{id}")))
                    .outline()
                    .small()
                    .label(if is_following { "Following" } else { "Follow" })
                    .on_click(move |_, _, cx| {
                        following.update(cx, |v, cx| {
                            *v = !*v;
                            cx.notify();
                        });
                    }),
            )
    }
}

const MATHJAX_NODE_SCRIPT: &str = r#"
const path = require("path");
const root = process.env.GPUI_MATHJAX_ROOT;
const source = process.env.GPUI_MATH_SOURCE || "";
const display = process.env.GPUI_MATH_DISPLAY === "1";
const req = (file) => require(path.join(root, file));

const {mathjax} = req("js/mathjax.js");
const {TeX} = req("js/input/tex.js");
const {SVG} = req("js/output/svg.js");
const {liteAdaptor} = req("js/adaptors/liteAdaptor.js");
const {RegisterHTMLHandler} = req("js/handlers/html.js");

const adaptor = liteAdaptor();
RegisterHTMLHandler(adaptor);

const tex = new TeX({packages: ["base", "ams"]});
const svg = new SVG({fontCache: "none"});
const html = mathjax.document("", {InputJax: tex, OutputJax: svg});
const node = html.convert(source, {display});
const outer = adaptor.outerHTML(node);
const match = outer.match(/<svg[\s\S]*<\/svg>/);

if (!match) {
  process.exit(2);
}

process.stdout.write(match[0]);
"#;

fn render_math_image(
    source: &str,
    inline: bool,
    font_size: f32,
    foreground: Hsla,
    background: Hsla,
) -> Option<RenderedMathImage> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<RenderedMathImage>>>> = OnceLock::new();

    let (foreground_fill, foreground_opacity) = svg_color(foreground);
    let (background_fill, background_opacity) = svg_color(background);
    let cache_key = format!(
        "{inline}\0{font_size:.2}\0{foreground_fill}\0{foreground_opacity:.3}\0{background_fill}\0{background_opacity:.3}\0{source}"
    );
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(cache) = cache.lock()
        && let Some(image) = cache.get(&cache_key)
    {
        return image.clone();
    }

    let image = render_math_svg(source, inline, font_size, foreground, background).map(|svg| {
        let width = svg_attr(&svg, "width")
            .and_then(|width| width.parse().ok())
            .unwrap_or(1.0);
        let height = svg_attr(&svg, "height")
            .and_then(|height| height.parse().ok())
            .unwrap_or(1.0);

        RenderedMathImage {
            width,
            height,
            image: Arc::new(Image::from_bytes(ImageFormat::Svg, svg.into_bytes())),
        }
    });

    if let Ok(mut cache) = cache.lock() {
        cache.insert(cache_key, image.clone());
    }

    image
}

fn render_math_svg(
    source: &str,
    inline: bool,
    font_size: f32,
    foreground: Hsla,
    background: Hsla,
) -> Option<String> {
    let root = mathjax_root()?;
    let output = Command::new("node")
        .arg("-e")
        .arg(MATHJAX_NODE_SCRIPT)
        .env("GPUI_MATHJAX_ROOT", root)
        .env("GPUI_MATH_SOURCE", source)
        .env("GPUI_MATH_DISPLAY", if inline { "0" } else { "1" })
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let mut svg = String::from_utf8(output.stdout).ok()?;
    let width = svg_dimension(&svg, "width")?;
    let height = svg_dimension(&svg, "height")?;
    let font_size = if inline {
        font_size.max(10.0)
    } else {
        (font_size * 1.18).max(12.0)
    };
    let ex = font_size * 0.5;
    let width = (width * ex).ceil().max(1.0);
    let height = (height * ex).ceil().max(1.0);
    let (foreground_fill, foreground_opacity) = svg_color(foreground);
    let (background_fill, background_opacity) = svg_color(background);

    svg = replace_svg_attr(&svg, "width", &format!("{width:.1}"));
    svg = replace_svg_attr(&svg, "height", &format!("{height:.1}"));
    svg = remove_svg_attr(&svg, "style");
    svg = rewrite_rects_as_paths(&svg);
    svg = svg.replace("currentColor", &foreground_fill);
    svg = inject_svg_background(&svg, &background_fill, background_opacity);
    if foreground_opacity < 0.999 {
        svg = svg.replacen(
            "<g ",
            &format!(r#"<g opacity="{foreground_opacity:.3}" "#),
            1,
        );
    }

    Some(svg)
}

fn render_math_text(source: &str, inline: bool, font_size: f32, color: Hsla) -> AnyElement {
    let font_size = if inline {
        font_size.max(10.0)
    } else {
        (font_size * 1.18).max(12.0)
    };

    div()
        .flex_none()
        .line_height(relative(if inline { 1.0 } else { 1.2 }))
        .text_size(px(font_size))
        .text_color(color)
        .italic()
        .child(prettify_math_source(source))
        .into_any_element()
}

fn mathjax_root() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        manifest_dir.join("../../docs/node_modules/mathjax-full"),
        PathBuf::from("docs/node_modules/mathjax-full"),
    ]
    .into_iter()
    .find(|path| path.join("js/mathjax.js").is_file())
}

fn svg_dimension(svg: &str, name: &str) -> Option<f32> {
    svg_attr(svg, name)?.strip_suffix("ex")?.parse().ok()
}

fn svg_attr<'a>(svg: &'a str, name: &str) -> Option<&'a str> {
    let pattern = format!(r#"{name}=""#);
    let start = svg.find(&pattern)? + pattern.len();
    let end = svg[start..].find('"')?;
    Some(&svg[start..start + end])
}

fn inject_svg_background(svg: &str, fill: &str, opacity: f32) -> String {
    let Some(open_end) = svg.find('>') else {
        return svg.to_string();
    };
    let opacity_attr = if opacity < 0.999 {
        format!(r#" opacity="{opacity:.3}""#)
    } else {
        String::new()
    };
    let background = if let Some((x, y, width, height)) = svg_view_box(svg) {
        format!(
            r#"<rect data-gpui-math-background="true" x="{x:.3}" y="{y:.3}" width="{width:.3}" height="{height:.3}" fill="{fill}"{opacity_attr}></rect>"#
        )
    } else {
        format!(
            r#"<rect data-gpui-math-background="true" width="100%" height="100%" fill="{fill}"{opacity_attr}></rect>"#
        )
    };

    let mut out = String::with_capacity(svg.len() + background.len());
    out.push_str(&svg[..open_end + 1]);
    out.push_str(&background);
    out.push_str(&svg[open_end + 1..]);
    out
}

fn svg_view_box(svg: &str) -> Option<(f32, f32, f32, f32)> {
    let values = svg_attr(svg, "viewBox")?
        .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .map(str::parse::<f32>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let [x, y, width, height] = values.as_slice() else {
        return None;
    };
    Some((*x, *y, *width, *height))
}

fn replace_svg_attr(svg: &str, name: &str, value: &str) -> String {
    let pattern = format!(r#"{name}=""#);
    let Some(start) = svg.find(&pattern).map(|start| start + pattern.len()) else {
        return svg.to_string();
    };
    let Some(end) = svg[start..].find('"') else {
        return svg.to_string();
    };

    let mut out = String::with_capacity(svg.len() + value.len());
    out.push_str(&svg[..start]);
    out.push_str(value);
    out.push_str(&svg[start + end..]);
    out
}

fn remove_svg_attr(svg: &str, name: &str) -> String {
    let pattern = format!(r#" {name}=""#);
    let Some(start) = svg.find(&pattern) else {
        return svg.to_string();
    };
    let Some(end) = svg[start + pattern.len()..].find('"') else {
        return svg.to_string();
    };

    let mut out = String::with_capacity(svg.len());
    out.push_str(&svg[..start]);
    out.push_str(&svg[start + pattern.len() + end + 1..]);
    out
}

fn rewrite_rects_as_paths(svg: &str) -> String {
    static RECT_RE: OnceLock<Regex> = OnceLock::new();
    let rect_re =
        RECT_RE.get_or_init(|| Regex::new(r#"<rect\b([^>]*)>(?:</rect>)?"#).expect("rect regex"));

    rect_re
        .replace_all(svg, |captures: &Captures<'_>| {
            let attrs = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
            rect_path(attrs).unwrap_or_else(|| captures[0].to_string())
        })
        .into_owned()
}

fn rect_path(attrs: &str) -> Option<String> {
    let width = svg_attr(attrs, "width")?.parse::<f32>().ok()?;
    let height = svg_attr(attrs, "height")?.parse::<f32>().ok()?;
    let x = svg_attr(attrs, "x")
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.0);
    let y = svg_attr(attrs, "y")
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.0);
    let right = x + width;
    let bottom = y + height;

    Some(format!(
        r#"<path d="M {x:.3} {y:.3} L {right:.3} {y:.3} L {right:.3} {bottom:.3} L {x:.3} {bottom:.3} Z"></path>"#
    ))
}

fn prettify_math_source(source: &str) -> String {
    let mut out = source.split_whitespace().collect::<Vec<_>>().join(" ");
    let replacements = [
        (r"\alpha", "\u{03b1}"),
        (r"\beta", "\u{03b2}"),
        (r"\gamma", "\u{03b3}"),
        (r"\delta", "\u{03b4}"),
        (r"\pi", "\u{03c0}"),
        (r"\sum", "\u{2211}"),
        (r"\sqrt", "\u{221a}"),
        (r"\times", "\u{00d7}"),
        (r"\cdot", "\u{22c5}"),
        (r"\leq", "\u{2264}"),
        (r"\geq", "\u{2265}"),
        (r"\neq", "\u{2260}"),
        (r"\infty", "\u{221e}"),
        (r"\left", ""),
        (r"\right", ""),
    ];

    for (from, to) in replacements {
        out = out.replace(from, to);
    }

    compact_math_scripts(&out)
}

fn compact_math_scripts(source: &str) -> String {
    let chars = source.chars().collect::<Vec<_>>();
    let mut out = String::new();
    let mut ix = 0;

    while ix < chars.len() {
        match chars[ix] {
            '^' | '_' => {
                let superscript = chars[ix] == '^';
                let (script, next_ix) = take_script(&chars, ix + 1);
                if script.is_empty() {
                    out.push(chars[ix]);
                    ix += 1;
                    continue;
                }

                for ch in script.chars() {
                    out.push(script_char(ch, superscript));
                }
                ix = next_ix;
            }
            ch => {
                out.push(ch);
                ix += 1;
            }
        }
    }

    out
}

fn take_script(chars: &[char], start_ix: usize) -> (String, usize) {
    let Some(first) = chars.get(start_ix).copied() else {
        return (String::new(), start_ix);
    };

    if first != '{' {
        return (first.to_string(), start_ix + 1);
    }

    let mut depth = 1;
    let mut ix = start_ix + 1;
    let mut script = String::new();
    while let Some(ch) = chars.get(ix).copied() {
        match ch {
            '{' => {
                depth += 1;
                script.push(ch);
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return (script, ix + 1);
                }
                script.push(ch);
            }
            _ => script.push(ch),
        }
        ix += 1;
    }

    (script, ix)
}

fn script_char(ch: char, superscript: bool) -> char {
    if superscript {
        match ch {
            '0' => '\u{2070}',
            '1' => '\u{00b9}',
            '2' => '\u{00b2}',
            '3' => '\u{00b3}',
            '4' => '\u{2074}',
            '5' => '\u{2075}',
            '6' => '\u{2076}',
            '7' => '\u{2077}',
            '8' => '\u{2078}',
            '9' => '\u{2079}',
            '+' => '\u{207a}',
            '-' => '\u{207b}',
            '=' => '\u{207c}',
            '(' => '\u{207d}',
            ')' => '\u{207e}',
            'i' => '\u{2071}',
            'n' => '\u{207f}',
            _ => ch,
        }
    } else {
        match ch {
            '0' => '\u{2080}',
            '1' => '\u{2081}',
            '2' => '\u{2082}',
            '3' => '\u{2083}',
            '4' => '\u{2084}',
            '5' => '\u{2085}',
            '6' => '\u{2086}',
            '7' => '\u{2087}',
            '8' => '\u{2088}',
            '9' => '\u{2089}',
            '+' => '\u{208a}',
            '-' => '\u{208b}',
            '=' => '\u{208c}',
            '(' => '\u{208d}',
            ')' => '\u{208e}',
            'a' => '\u{2090}',
            'e' => '\u{2091}',
            'h' => '\u{2095}',
            'i' => '\u{1d62}',
            'j' => '\u{2c7c}',
            'k' => '\u{2096}',
            'l' => '\u{2097}',
            'm' => '\u{2098}',
            'n' => '\u{2099}',
            'o' => '\u{2092}',
            'p' => '\u{209a}',
            'r' => '\u{1d63}',
            's' => '\u{209b}',
            't' => '\u{209c}',
            'u' => '\u{1d64}',
            'v' => '\u{1d65}',
            'x' => '\u{2093}',
            _ => ch,
        }
    }
}

fn svg_color(color: Hsla) -> (String, f32) {
    let rgba: Rgba = color.into();
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    (
        format!(
            "#{:02x}{:02x}{:02x}",
            channel(rgba.r),
            channel(rgba.g),
            channel(rgba.b)
        ),
        rgba.a.clamp(0.0, 1.0),
    )
}

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

        let _subscriptions = vec![cx.subscribe(&input_state, |_, _, _: &InputEvent, _| {})];

        Self {
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
                                        markdown(self.input_state.read(cx).value())
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
    fn math_fallback_text_prettifies_formula_text() {
        assert_eq!(
            prettify_math_source(r"\alpha + \beta"),
            "\u{03b1} + \u{03b2}"
        );
    }

    #[::core::prelude::v1::test]
    fn math_svg_uses_path_based_renderer() {
        let Some(svg) = render_math_svg(
            r"e^{i\pi} + 1 = 0",
            true,
            20.0,
            Hsla::black(),
            Hsla::white(),
        ) else {
            eprintln!("skipping MathJax SVG test because mathjax-full is not available");
            return;
        };

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
    fn inline_math_svg_uses_pixel_dimensions_without_glyph_scaling() {
        let Some(svg) = render_math_svg(
            r"e^{i\pi} + 1 = 0",
            true,
            20.0,
            Hsla::black(),
            Hsla::white(),
        ) else {
            eprintln!("skipping MathJax SVG test because mathjax-full is not available");
            return;
        };

        assert!(svg_width(&svg) > 0.0);
        assert!(!svg_attr(&svg, "width").unwrap().ends_with("ex"));
        assert!(!svg_attr(&svg, "height").unwrap().ends_with("ex"));
        assert!(!svg.contains("vertical-align"));
        assert!(!svg.contains("currentColor"));
        assert!(!svg.contains("lengthAdjust="));
        assert!(!svg.contains("textLength="));
    }

    #[::core::prelude::v1::test]
    fn block_math_svg_rewrites_mathjax_rects_to_paths() {
        let Some(svg) = render_math_svg(
            r"\frac{\alpha + \beta}{\sqrt{\gamma}} = \sum_{i=1}^{n} i^2",
            false,
            20.0,
            Hsla::black(),
            Hsla::white(),
        ) else {
            eprintln!("skipping MathJax SVG test because mathjax-full is not available");
            return;
        };

        assert!(!svg.contains(r#"<rect width="543""#));
        assert!(!svg.contains(r#"<rect width="2628.4""#));
        assert!(svg.contains("<path"));
    }

    #[::core::prelude::v1::test]
    fn block_math_svg_includes_theme_background() {
        let Some(svg) = render_math_svg(
            r"\frac{\alpha + \beta}{\sqrt{\gamma}} = \sum_{i=1}^{n} i^2",
            false,
            20.0,
            Hsla::black(),
            Hsla::white(),
        ) else {
            eprintln!("skipping MathJax SVG test because mathjax-full is not available");
            return;
        };

        assert!(svg.contains(r#"data-gpui-math-background="true""#));
        assert!(svg.contains("fill=\"#ffffff\""));
    }

    #[::core::prelude::v1::test]
    fn block_math_image_exposes_intrinsic_svg_size() {
        let Some(image) = render_math_image(
            r"\frac{\alpha + \beta}{\sqrt{\gamma}} = \sum_{i=1}^{n} i^2",
            false,
            20.0,
            Hsla::black(),
            Hsla::white(),
        ) else {
            eprintln!("skipping MathJax image test because mathjax-full is not available");
            return;
        };

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
    fn inline_math_segments_skip_inline_code() {
        assert_eq!(
            inline_math_segments("This is $x^2$ and `$ignored$`.").unwrap(),
            vec![
                MathSegment {
                    source: "This is ".to_string(),
                    math: false,
                },
                MathSegment {
                    source: "x^2".to_string(),
                    math: true,
                },
                MathSegment {
                    source: " and `$ignored$`.".to_string(),
                    math: false,
                },
            ]
        );
    }

    fn svg_width(svg: &str) -> f32 {
        let start = svg.find("width=\"").unwrap() + "width=\"".len();
        let end = svg[start..].find('"').unwrap();
        svg[start..start + end].parse().unwrap()
    }
}
