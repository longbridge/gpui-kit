use gpui::{
    AnyElement, App, Bounds, ClickEvent, Element, ElementId, Entity, GlobalElementId,
    HighlightStyle, InspectorElementId, IntoElement, LayoutId, Pixels, Refineable as _, RenderOnce,
    SharedString, StyleRefinement, Styled, Window,
};

use std::{ops::Range, sync::Arc, time::Duration};

use super::{
    MarkdownExtensions, MarkdownNode, MarkdownParseContext, MarkdownPlugin, SelectionFormat,
    SharedCodeBlockHighlighter, TableData, TextViewMotion, TextViewState, TextViewStyle,
};
use gpui_base::{Easing, text::CodeBlock};

/// How long a word of streamed text takes to reach full color, and how much later each further
/// word of the same chunk starts. Measured frame by frame from claude.ai: a chunk lands as
/// ~6 words every ~100 ms and goes from transparent to solid in ~250-300 ms, its words lighting
/// up a few milliseconds apart rather than all at once.
///
/// The two pull in opposite directions and both matter.
///
/// `stagger × words` is how long a chunk takes to light up end to end, and it has to stay well
/// under the interval between chunks: let it reach that and chunks overlap into one continuous
/// drip of single words, which is what staggering was meant to avoid. At 10 ms a 6-word chunk
/// lights up in 60 ms -- one gesture, with a visible gradient inside it.
///
/// The fade has to outlast that interval instead. How many words are visibly mid-fade at any
/// moment is the reveal rate times the fade, so a short fade leaves a couple of grey glyphs on
/// the tail and no gradient to speak of.
const STREAM_FADE: Duration = Duration::from_millis(280);
const STREAM_FADE_STAGGER: Duration = Duration::from_millis(10);

/// The component-level rich text element.
///
/// The rendering, parsing and selection all live in [`gpui_base::TextView`];
/// this wrapper exists so that the component API -- `TextViewStyle`, the
/// component `HighlightTheme`, and the element's associated types -- keeps
/// working unchanged.
#[derive(Clone)]
pub struct TextView {
    id: ElementId,
    inner: gpui_base::TextView,
    text_style: Option<TextViewStyle>,
    motion: Option<TextViewMotion>,
    /// `None` leaves the state's own policy alone; `Some(false)` turns a
    /// fade off that an earlier frame turned on.
    stream_fade: Option<bool>,
    /// Takes precedence over the highlighter derived from `text_style` and
    /// the application default.
    code_block_highlighter: Option<Arc<SharedCodeBlockHighlighter>>,
}

impl Styled for TextView {
    fn style(&mut self) -> &mut StyleRefinement {
        gpui::Styled::style(&mut self.inner)
    }
}

impl TextView {
    /// Creates a text view rendering an existing [`TextViewState`].
    pub fn new(state: &Entity<TextViewState>) -> Self {
        Self {
            id: ElementId::Name(state.entity_id().to_string().into()),
            inner: gpui_base::TextView::new(state),
            text_style: None,
            motion: None,
            stream_fade: None,
            code_block_highlighter: None,
        }
    }
    /// Creates a text view that parses `text` as Markdown.
    pub fn markdown(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            inner: gpui_base::TextView::markdown(id, text),
            text_style: None,
            motion: None,
            stream_fade: None,
            code_block_highlighter: None,
        }
    }
    /// Creates a text view that parses `text` as HTML.
    pub fn html(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            inner: gpui_base::TextView::html(id, text),
            text_style: None,
            motion: None,
            stream_fade: None,
            code_block_highlighter: None,
        }
    }
    /// Sets the style, folded onto the one derived from the active theme.
    pub fn style(mut self, style: TextViewStyle) -> Self {
        self.text_style = Some(style);
        self
    }
    /// Sets whether the text can be selected with the mouse.
    pub fn selectable(mut self, value: bool) -> Self {
        self.inner = self.inner.selectable(value);
        self
    }
    /// Sets whether a copied selection carries Markdown source or plain text.
    pub fn selection_format(mut self, value: SelectionFormat) -> Self {
        self.inner = self.inner.selection_format(value);
        self
    }
    /// Sets whether the view scrolls its own content.
    pub fn scrollable(mut self, value: bool) -> Self {
        self.inner = self.inner.scrollable(value);
        self
    }
    /// Fades streamed text in the way Claude reveals a reply: the words a `set_text` or
    /// `push_str` adds start transparent and light up one after another, each reaching full
    /// color over 280 ms. A chunk far larger than one keystroke burst -- a backfill, a replay --
    /// fades as a whole instead, since nobody typed it. Text that replaces rather than extends
    /// the current content shows at once, and reduced motion disables the fade. Use
    /// [`Self::motion`] for other timing.
    pub fn stream_fade(mut self, value: bool) -> Self {
        self.stream_fade = Some(value);
        self
    }
    /// Sets the motion policy explicitly, overriding [`Self::stream_fade`]'s
    /// theme timing.
    pub fn motion(mut self, motion: TextViewMotion) -> Self {
        self.motion = Some(motion);
        self
    }
    /// Clamps the rendered content to `value` lines.
    pub fn max_lines(mut self, value: usize) -> Self {
        self.inner = self.inner.max_lines(value);
        self
    }
    /// Renders an element in the corner of every fenced code block.
    pub fn code_block_actions<F, E>(mut self, f: F) -> Self
    where
        F: Fn(&CodeBlock, &mut Window, &mut App) -> E + Send + Sync + 'static,
        E: IntoElement,
    {
        self.inner = self.inner.code_block_actions(f);
        self
    }
    /// Highlights fenced code blocks with `highlighter` instead of the
    /// application default or the highlight theme of [`Self::style`].
    ///
    /// The highlighter returns UTF-8 byte ranges relative to
    /// [`CodeBlock::code`]; invalid ranges are discarded. Return no ranges to
    /// render code blocks as plain text in this view while others keep the
    /// default highlighting:
    ///
    /// ```ignore
    /// TextView::markdown("notes", source).code_block_highlighter(|_| Vec::new())
    /// ```
    ///
    /// A code block reuses its highlights only while the highlighter is the
    /// same one, so a view built in `render` runs its highlighter for every
    /// code block each time it is built.
    pub fn code_block_highlighter<F>(mut self, highlighter: F) -> Self
    where
        F: Fn(&CodeBlock) -> Vec<(Range<usize>, HighlightStyle)> + Send + Sync + 'static,
    {
        self.code_block_highlighter = Some(Arc::new(highlighter));
        self
    }
    /// Renders an element in the corner of every table.
    pub fn table_actions<F, E>(mut self, f: F) -> Self
    where
        F: Fn(&TableData, &mut Window, &mut App) -> E + Send + Sync + 'static,
        E: IntoElement,
    {
        self.inner = self.inner.table_actions(f);
        self
    }
    /// Handles link clicks instead of opening the URL.
    pub fn on_link_click<F>(mut self, f: F) -> Self
    where
        F: Fn(&SharedString, &ClickEvent, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.inner = self.inner.on_link_click(f);
        self
    }
    /// Scrolls a container that ignores scroll requests to the line of
    /// `TextViewState::reveal_range`, with the line's window bounds.
    pub fn on_reveal<F>(mut self, f: F) -> Self
    where
        F: Fn(Bounds<Pixels>, &mut Window, &mut App) + 'static,
    {
        self.inner = self.inner.on_reveal(f);
        self
    }
    /// Sets which Markdown extensions the parser accepts.
    pub fn markdown_extensions(mut self, value: MarkdownExtensions) -> Self {
        self.inner = self.inner.markdown_extensions(value);
        self
    }
    /// Enables the MDX Markdown extensions.
    pub fn markdown_mdx(mut self) -> Self {
        self.inner = self.inner.markdown_mdx();
        self
    }

    /// Parses custom block nodes out of the Markdown AST.
    pub fn markdown_block_parser<F>(mut self, parser: F) -> Self
    where
        F: for<'a> Fn(&markdown::mdast::Node, &MarkdownParseContext<'a>) -> Option<MarkdownNode>
            + Send
            + Sync
            + 'static,
    {
        self.inner = self.inner.markdown_block_parser(parser);
        self
    }
    /// Renders the custom block nodes named `name`.
    pub fn markdown_block_renderer<F, E>(
        mut self,
        name: impl Into<SharedString>,
        renderer: F,
    ) -> Self
    where
        F: Fn(&MarkdownNode, &mut Window, &mut App) -> E + Send + Sync + 'static,
        E: IntoElement,
    {
        self.inner = self.inner.markdown_block_renderer(name, renderer);
        self
    }
    /// Applies a plugin, which may install any of the hooks above.
    pub fn plugin<P>(self, plugin: P) -> Self
    where
        P: TextViewPlugin,
    {
        plugin.setup(self)
    }
}

impl IntoElement for TextView {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// Layout state retained for source compatibility with the original component TextView.
pub struct TextViewLayoutState {
    element: AnyElement,
}

/// Prepaint state retained for source compatibility with the original component TextView.
pub struct TextViewPrepaintState;

impl Element for TextView {
    type RequestLayoutState = TextViewLayoutState;
    type PrepaintState = TextViewPrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut inner = self.inner.clone();
        if let Some(style) = self.text_style.clone() {
            // `request_layout` runs every frame, so this asks whether the
            // caller ever replaced the theme -- a pointer comparison against
            // the shared default -- rather than comparing two whole themes
            // field by field.
            #[cfg(feature = "tree-sitter")]
            if !std::sync::Arc::ptr_eq(
                &style.highlight_theme,
                &crate::highlighter::HighlightTheme::default_light(),
            ) {
                inner = inner.shared_code_block_highlighter(super::shared_code_block_highlighter(
                    &style.highlight_theme,
                ));
            }
            inner = inner.style(resolve_component_style(
                crate::ActiveTheme::theme(cx),
                style,
            ));
        }
        // After the style, whose highlight theme it overrides.
        if let Some(highlighter) = self.code_block_highlighter.clone() {
            inner = inner.shared_code_block_highlighter(highlighter);
        }
        let motion = self.motion.clone().or_else(|| {
            self.stream_fade.map(|fade| {
                if !fade {
                    return TextViewMotion::default();
                }
                TextViewMotion::default()
                    .with_stream_fade(STREAM_FADE)
                    .with_stream_fade_stagger(STREAM_FADE_STAGGER)
                    .with_stream_fade_easing(Easing::EaseOut)
            })
        });
        if let Some(motion) = motion {
            inner = inner.motion(motion);
        }
        let mut element = inner.into_any_element();
        let layout_id = element.request_layout(window, cx);
        (layout_id, TextViewLayoutState { element })
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        element: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        element.element.prepaint(window, cx);
        TextViewPrepaintState
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        element: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.element.paint(window, cx);
    }
}

/// Folds a component [`TextViewStyle`] onto the one the theme already derived.
///
/// The legacy type carries `StyleRefinement`s that callers filled in
/// partially, so each one is refined onto the themed value rather than
/// replacing it -- a caller who set only `white_space` keeps the themed
/// padding and colors.
pub(super) fn resolve_component_style(
    theme: &crate::Theme,
    legacy: TextViewStyle,
) -> gpui_base::TextViewStyle {
    let themed = super::base_text_view_style(theme);

    let refined = |mut base: gpui::StyleRefinement, overlay: &StyleRefinement| {
        base.refine(overlay);
        base
    };
    let code_block = refined(themed.code_block().clone(), &legacy.code_block);
    let table = refined(themed.table().clone(), &legacy.table);
    let table_head = refined(themed.table_head().clone(), &legacy.table_head);
    let table_cell = refined(themed.table_cell().clone(), &legacy.table_cell);

    let mut inline_code = themed.inline_code();
    refine_highlight_style(&mut inline_code, legacy.inline_code);

    // `is_dark` only ever turns on: the component theme already answered the
    // question, and a legacy style left at its `false` default must not undo
    // a dark theme.
    let is_dark = themed.is_dark() || legacy.is_dark;

    let heading_base_font_size = legacy.heading_base_font_size;
    let heading_font_size = legacy.heading_font_size;
    let style = themed
        .with_paragraph_gap(legacy.paragraph_gap)
        .with_heading(move |level| {
            let default_size = match level {
                1 => gpui::rems(2.),
                2 => gpui::rems(1.5),
                3 => gpui::rems(1.25),
                4 => gpui::rems(1.125),
                _ => gpui::rems(1.),
            }
            .to_pixels(heading_base_font_size);
            let text_size = heading_font_size.as_ref().map_or(default_size, |resolve| {
                resolve(level, heading_base_font_size)
            });
            StyleRefinement::default().text_size(text_size)
        })
        .with_code_block(code_block)
        .with_table(table)
        .with_table_head(table_head)
        .with_table_cell(table_cell)
        .with_inline_code(inline_code)
        .with_dark(is_dark);
    style
}

fn refine_highlight_style(style: &mut HighlightStyle, refinement: HighlightStyle) {
    if refinement.color.is_some() {
        style.color = refinement.color;
    }
    if refinement.font_weight.is_some() {
        style.font_weight = refinement.font_weight;
    }
    if refinement.font_style.is_some() {
        style.font_style = refinement.font_style;
    }
    if refinement.background_color.is_some() {
        style.background_color = refinement.background_color;
    }
    if refinement.underline.is_some() {
        style.underline = refinement.underline;
    }
    if refinement.strikethrough.is_some() {
        style.strikethrough = refinement.strikethrough;
    }
    if refinement.fade_out.is_some() {
        style.fade_out = refinement.fade_out;
    }
}

/// A bundle of [`TextView`] configuration that can be applied in one call.
pub trait TextViewPlugin {
    /// Applies this plugin's configuration to `text_view`.
    fn setup(self, text_view: TextView) -> TextView;
}
impl<P> TextViewPlugin for P
where
    P: MarkdownPlugin,
{
    fn setup(self, mut text_view: TextView) -> TextView {
        text_view.inner = text_view.inner.plugin(self);
        text_view
    }
}

/// Either a plain string or a rich [`TextView`].
#[derive(IntoElement, Clone)]
pub enum Text {
    String(SharedString),
    TextView(Box<TextView>),
}
impl From<SharedString> for Text {
    fn from(value: SharedString) -> Self {
        Self::String(value)
    }
}
impl From<String> for Text {
    fn from(value: String) -> Self {
        Self::String(value.into())
    }
}
impl From<&str> for Text {
    fn from(value: &str) -> Self {
        Self::String(value.to_string().into())
    }
}
impl From<TextView> for Text {
    fn from(value: TextView) -> Self {
        Self::TextView(Box::new(value))
    }
}
impl Text {
    /// Sets the style for the [`TextView`]. Does nothing for a plain string.
    pub fn style(self, style: TextViewStyle) -> Self {
        match self {
            Self::String(value) => Self::String(value),
            Self::TextView(view) => Self::TextView(Box::new(view.style(style))),
        }
    }
    pub(crate) fn get_text(&self, cx: &App) -> SharedString {
        match self {
            Self::String(value) => value.clone(),
            Self::TextView(view) => gpui_base::Text::from(view.inner.clone()).get_text(cx),
        }
    }
}
impl RenderOnce for Text {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        match self {
            Self::String(value) => value.into_any_element(),
            Self::TextView(view) => view.into_any_element(),
        }
    }
}

/// Creates a Markdown text view identified by the caller's code location.
#[track_caller]
pub fn markdown(source: impl Into<SharedString>) -> TextView {
    TextView::markdown(
        ElementId::CodeLocation(*std::panic::Location::caller()),
        source,
    )
}
/// Creates an HTML text view identified by the caller's code location.
#[track_caller]
pub fn html(source: impl Into<SharedString>) -> TextView {
    TextView::html(
        ElementId::CodeLocation(*std::panic::Location::caller()),
        source,
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use gpui::{
        Context, IntoElement, ParentElement as _, Render, SharedString, TestAppContext,
        VisualTestContext, div,
    };

    use super::{TextView, TextViewStyle};

    /// The code of every block a highlighter was asked to highlight.
    type SeenBlocks = Arc<Mutex<Vec<SharedString>>>;

    fn seen(blocks: &SeenBlocks) -> Vec<String> {
        blocks
            .lock()
            .unwrap()
            .iter()
            .map(|code| code.trim().to_string())
            .collect()
    }

    struct StatelessMarkdown {
        renders: Arc<AtomicUsize>,
    }

    impl Render for StatelessMarkdown {
        fn render(&mut self, _: &mut gpui::Window, _: &mut Context<Self>) -> impl IntoElement {
            self.renders.fetch_add(1, Ordering::Relaxed);
            div().child(
                super::markdown(include_str!("../../../../examples/fixtures/test.md"))
                    .markdown_block_parser(|_, _| None),
            )
        }
    }

    #[gpui::test]
    fn stateless_markdown_facade_settles_after_parsing(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let renders = Arc::new(AtomicUsize::new(0));
        let (_, cx) = cx.add_window_view({
            let renders = renders.clone();
            move |_, _| StatelessMarkdown { renders }
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let renders_after_redraw = renders.load(Ordering::Relaxed);
        cx.run_until_parked();
        assert_eq!(
            renders.load(Ordering::Relaxed),
            renders_after_redraw,
            "an unchanged compatibility TextView must not schedule another render after its parse",
        );
    }

    struct DefaultAndOverride {
        overridden: SeenBlocks,
    }

    impl Render for DefaultAndOverride {
        fn render(&mut self, _: &mut gpui::Window, _: &mut Context<Self>) -> impl IntoElement {
            let overridden = self.overridden.clone();
            // A custom highlight theme would install its own highlighter;
            // the explicit one must still win.
            let mut style = TextViewStyle::default();
            style.highlight_theme = crate::highlighter::HighlightTheme::default_dark();
            div()
                .child(TextView::markdown("default", "```rust\nby_default()\n```"))
                .child(
                    TextView::markdown("override", "```rust\nby_override()\n```")
                        .style(style)
                        .code_block_highlighter(move |block| {
                            overridden.lock().unwrap().push(block.code());
                            Vec::new()
                        }),
                )
        }
    }

    #[gpui::test]
    fn code_block_highlighter_replaces_the_default_for_one_view(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let defaulted = SeenBlocks::default();
        let overridden = SeenBlocks::default();
        cx.update(|cx| {
            let defaulted = defaulted.clone();
            gpui_base::TextViewDefaults::global(cx)
                .with_code_block_highlighter(move |block| {
                    defaulted.lock().unwrap().push(block.code());
                    Vec::new()
                })
                .install(cx);
        });
        let (_, cx) = cx.add_window_view({
            let overridden = overridden.clone();
            move |_, _| DefaultAndOverride { overridden }
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let defaulted = seen(&defaulted);
        let overridden = seen(&overridden);
        assert!(defaulted.iter().all(|code| code == "by_default()"));
        assert!(
            defaulted.iter().any(|code| code == "by_default()"),
            "a view without an override keeps the application default",
        );
        assert!(overridden.iter().all(|code| code == "by_override()"));
        assert!(
            overridden.iter().any(|code| code == "by_override()"),
            "the override must reach the view's code blocks, ahead of its style's theme",
        );
    }
}
