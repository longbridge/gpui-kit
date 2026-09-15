#[derive(Clone)]
struct TickerNode {
    symbol: String,
}

#[derive(Clone)]
struct UserCardNode {
    id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MathNode {
    source: String,
    inline: bool,
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
    baseline: f32,
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
        if inline { "inline-math" } else { "math" },
        MathNode {
            source: source.clone(),
            inline,
        },
    )
    .text(prettify_math_source(&source))
    .accessibility_label(format!("Formula: {}", prettify_math_source(&source)))
    .markdown(markdown.into())
}

fn block_math_source(source: &str) -> Option<&str> {
    let source = source.trim();
    let body = source.strip_prefix("$$")?.strip_suffix("$$")?.trim();
    (!body.is_empty()).then_some(body)
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

        None
    }

    fn render(&self, node: &MarkdownNode, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let math = node.data::<MathNode>().expect("math markdown node data");
        let font_size = f32::from(window.text_style().font_size.to_pixels(window.rem_size()));

        div()
            .w_full()
            .flex()
            .justify_center()
            .py_1()
            .child(render_math_formula(&math.source, false, font_size, cx))
    }
}

/// Prepared resources belong to this example, not a process-global inline cache.
#[derive(Default)]
struct InlineMathCache {
    document: String,
    images: HashMap<String, Option<RenderedMathImage>>,
}

impl InlineMathCache {
    fn set_document(&mut self, source: &str) {
        if self.document != source {
            self.document = source.to_string();
            // Keep prepared and pending resources for formulas still in the document.
            self.images
                .retain(|key, _| source.contains(key.split('\0').next().unwrap_or_default()));
        }
    }
}

#[derive(Clone)]
struct InlineMathPlugin {
    cache: Arc<Mutex<InlineMathCache>>,
    invalidate: Arc<dyn Fn(&mut App) + Send + Sync>,
}

impl InlineMathPlugin {
    fn new(invalidate: impl Fn(&mut App) + Send + Sync + 'static) -> Self {
        Self {
            cache: Arc::default(),
            invalidate: Arc::new(invalidate),
        }
    }

    fn set_document(&self, source: &str) {
        self.cache.lock().unwrap().set_document(source);
    }
}

fn parse_inline_math(node: &markdown_ast::Node, source: Option<&str>) -> Option<MarkdownNode> {
    let markdown_ast::Node::InlineMath(math) = node else {
        return None;
    };
    Some(math_node(
        math.value.clone(),
        true,
        source
            .map(str::to_string)
            .unwrap_or_else(|| math_markdown(&math.value, true)),
    ))
}

impl MarkdownPlugin for InlineMathPlugin {
    fn name(&self) -> &str {
        "inline-math"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        context: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        parse_inline_math(node, context.node_source(node))
    }

    fn render_inline(
        &self,
        node: &MarkdownNode,
        context: &InlineRenderContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<InlineElement> {
        let source = node.data::<MathNode>()?.source.clone();
        let font_size = f32::from(context.font_size());
        let foreground = context.text_style().color;
        let background = cx.theme().background;
        let key = format!("{source}\0{font_size:?}\0{foreground:?}\0{background:?}");
        let mut cache = self.cache.lock().unwrap();
        if let Some(image) = cache.images.get(&key) {
            return image.as_ref().map(|image| {
                let fallback_source = source.clone();
                let fallback =
                    move || render_math_text(&fallback_source, true, font_size, foreground);
                InlineElement::new(
                    img(image.image.clone())
                        .object_fit(ObjectFit::Contain)
                        .w(px(image.width))
                        .h(px(image.height))
                        .with_loading(fallback.clone())
                        .with_fallback(fallback),
                )
                .with_baseline(px(image.baseline))
            });
        }
        // None represents pending or unavailable; both use TextView's atomic text fallback.
        cache.images.insert(key.clone(), None);
        drop(cache);
        let cache = Arc::downgrade(&self.cache);
        let invalidate = self.invalidate.clone();
        // The callback only reads prepared images and enqueues at most one request per
        // exact source/font/theme key. Deferring is necessary to capture the real
        // font size (including headings), without running MathJax during layout.
        // The pending entry above deduplicates repeated measurement callbacks.
        cx.defer(move |cx| {
            if cache
                .upgrade()
                .is_none_or(|cache| !cache.lock().unwrap().images.contains_key(&key))
            {
                return;
            }
            let task = cx.background_executor().spawn(async move {
                render_math_image(&source, true, font_size, foreground, background)
            });
            cx.spawn(async move |cx| {
                let image = task.await;
                let Some(cache) = cache.upgrade() else { return };
                let mut cache = cache.lock().unwrap();
                if !cache.images.contains_key(&key) {
                    return;
                }
                cache.images.insert(key, image);
                drop(cache);
                cx.update(|cx| invalidate(cx));
            })
            .detach();
        });
        None
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
    if !inline
        && let Ok(cache) = cache.lock()
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
            baseline: svg_baseline(&svg, height).unwrap_or(height),
            image: Arc::new(Image::from_bytes(ImageFormat::Svg, svg.into_bytes())),
        }
    });

    if !inline && let Ok(mut cache) = cache.lock() {
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
    if !inline {
        svg = inject_svg_background(&svg, &background_fill, background_opacity);
    }
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
    if let Some(root) = std::env::var_os("GPUI_MATHJAX_ROOT") {
        let root = PathBuf::from(root);
        return root.join("js/mathjax.js").is_file().then_some(root);
    }
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

// MathJax places the alphabetic baseline at y=0 in its SVG viewBox.
fn svg_baseline(svg: &str, pixel_height: f32) -> Option<f32> {
    let (_, y, _, height) = svg_view_box(svg)?;
    if !y.is_finite() || !height.is_finite() || height <= 0.0 {
        return None;
    }
    Some((-y / height * pixel_height).clamp(0.0, pixel_height))
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
