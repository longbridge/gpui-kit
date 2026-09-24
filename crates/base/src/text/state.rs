use futures::Stream as _;
#[cfg(not(target_family = "wasm"))]
use std::time::Instant;
use std::{
    ops::{Range, RangeInclusive},
    pin::Pin,
    sync::{Arc, Mutex, OnceLock},
    task::Poll,
};
#[cfg(target_family = "wasm")]
use web_time::Instant;

use gpui::{
    App, AppContext as _, Bounds, Context, EntityId, FocusHandle, IntoElement, KeyBinding,
    ListState, ParentElement as _, Pixels, Point, Render, SharedString, Styled as _, Task, Window,
    prelude::FluentBuilder as _, px,
};

use crate::{
    AutoScroll, ElementExt, TextSelection,
    async_util::{Receiver, Sender, unbounded},
    input::{self, SelectAll},
    text::{
        CodeBlockActionsFn, CodeBlockHighlighterFn, LinkClickHandlerFn, MarkdownExtensions,
        RangeHighlight, RangeHighlightError, RenderedText, TableActionsFn, TextViewStyle,
        document::ParsedDocument,
        format,
        node::{self, NodeContext},
        range_highlight::{RangeHighlightFrame, RenderedIndex},
        selection_adapter::TextViewSelectionAdapter,
        stream_fade::{StreamFadeTracker, TextViewMotion},
    },
    v_flex,
};

const CONTEXT: &'static str = "TextView";
// Keep coalescing bounded so sustained streams still render intermediate updates.
const MAX_COALESCED_UPDATES_PER_PARSE: usize = 64;
// Preserve exact first-layout height for small documents while bounding the
// amount of source parsed synchronously on the UI thread.
const MAX_SYNC_FULL_REPLACE_BYTES: usize = 4 * 1024;

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys(vec![
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", input::Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", input::Copy, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-a", input::SelectAll, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-a", input::SelectAll, Some(CONTEXT)),
    ]);
}

/// The content format of the text view.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TextViewFormat {
    /// Markdown view
    Markdown,
    /// HTML view
    Html,
}

/// The format of the text returned by
/// [`TextViewState::selected_text`], which is also what copy writes to the
/// clipboard.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SelectionFormat {
    /// The rendered text, without any markup.
    #[default]
    Plain,
    /// The source of the selection.
    ///
    /// Select-all returns the original source verbatim, a partial selection is
    /// reconstructed as Markdown from the parsed nodes (e.g. selecting inside
    /// a `**bold**` run yields `**bold**`).
    Source,
}

/// One text element's laid-out vertical extent, reported by `Inline` during
/// prepaint so `TextView` can snap its `max_lines` clip to a whole-line
/// boundary.
#[derive(Clone, Copy)]
pub(super) struct LineSpan {
    pub(super) top: Pixels,
    pub(super) bottom: Pixels,
    pub(super) line_height: Pixels,
}

/// The state of a TextView.
pub struct TextViewState {
    entity_id: EntityId,
    pub(super) focus_handle: FocusHandle,
    pub(super) list_state: ListState,

    /// The bounds of the text view
    bounds: Bounds<Pixels>,

    pub(super) selectable: bool,
    pub(super) selection_format: SelectionFormat,
    pub(super) scrollable: bool,
    pub(super) max_lines: Option<usize>,
    /// Line spans reported by `Inline` during prepaint (collected only while
    /// [`Self::max_lines`] is set); cleared by `TextView` at each frame start.
    pub(super) line_spans: Arc<Mutex<Vec<LineSpan>>>,
    /// Whether the last painted frame clipped content due to `max_lines`.
    pub(super) clamped: bool,
    pub(super) text_view_style: Arc<TextViewStyle>,
    pub(super) code_block_actions: Option<std::sync::Arc<CodeBlockActionsFn>>,
    pub(super) code_block_highlighter: Option<std::sync::Arc<CodeBlockHighlighterFn>>,
    pub(super) table_actions: Option<std::sync::Arc<TableActionsFn>>,
    pub(super) link_click_handler: Option<std::sync::Arc<LinkClickHandlerFn>>,
    pub(super) markdown_extensions: Arc<MarkdownExtensions>,

    pub(super) is_selecting: bool,
    /// Logical ranges retained across an explicitly requested resource reflow.
    pub(super) preserve_inline_selection: bool,
    multi_click_selection: Option<TextViewMultiClickSelection>,
    selected_text_override: Option<String>,
    select_all: bool,
    pub(super) auto_scroll: AutoScroll,
    pub(super) selection_adapter: TextViewSelectionAdapter,

    pub(super) parsed_content: ParsedContent,
    pub(super) stream_fade: StreamFadeTracker,
    /// Content format (markdown / html), used for bounded synchronous parsing
    /// of small full-replace updates.
    format: TextViewFormat,
    text: String,
    /// The text a `TextView` element last handed over, to recognize the same
    /// string next frame without comparing its bytes.
    element_text: Option<SharedString>,
    revision: usize,
    /// The revision of the update `parsed_content` was committed from.
    committed_revision: usize,
    /// The revision of the last update that replaced the text rather than
    /// appending to it.
    full_update_revision: usize,
    /// The rendered text of `parsed_content`, built when first read.
    rendered_index: Arc<OnceLock<RenderedIndex>>,
    range_highlights: Option<Arc<RangeHighlightFrame>>,
    pub(super) selection_revision: usize,
    compatible_layout_update: bool,
    layout_text_style: Option<(gpui::TextStyle, Pixels)>,
    parsed_error: Option<SharedString>,
    tx: Sender<UpdateOptions>,
    _parse_task: Task<()>,
    _receive_task: Task<()>,
}

impl TextViewState {
    /// Create a Markdown TextViewState.
    pub fn markdown(text: &str, cx: &mut Context<Self>) -> Self {
        Self::new(TextViewFormat::Markdown, text, cx)
    }

    /// Create a HTML TextViewState.
    pub fn html(text: &str, cx: &mut Context<Self>) -> Self {
        Self::new(TextViewFormat::Html, text, cx)
    }

    /// Create a new TextViewState.
    fn new(format: TextViewFormat, text: &str, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let selection_adapter = TextViewSelectionAdapter::new(cx.entity().downgrade(), cx);

        let (tx, rx) = unbounded::<UpdateOptions>();
        let (tx_result, rx_result) = unbounded::<ParsedUpdate>();
        let _receive_task = cx.spawn({
            async move |weak_self, cx| {
                while let Ok(parsed_update) = rx_result.recv().await {
                    _ = weak_self.update(cx, |state, cx| {
                        if parsed_update.revision != state.revision {
                            return;
                        }
                        if parsed_update.baseline_ack {
                            debug_assert!(parsed_update.full_parse);
                            return;
                        }

                        match parsed_update.result {
                            Ok(content) => {
                                state.reconcile_range_highlights(
                                    &content.document,
                                    parsed_update.revision,
                                    parsed_update.selection_compatible && !parsed_update.full_parse,
                                );
                                state.stream_fade.record(
                                    &state.parsed_content.document,
                                    &content.document,
                                    Instant::now(),
                                );
                                state.parsed_content = content;
                                state.parsed_error = None;
                                state.compatible_layout_update = parsed_update.selection_compatible;
                                if parsed_update.full_parse {
                                    state.invalidate_measured_heights();
                                }
                            }
                            Err(err) => {
                                state.stream_fade.discard_pending();
                                state.parsed_error = Some(err);
                            }
                        }
                        // Don't interrupt an active drag-selection; the stored
                        // positions remain valid for append-only updates and will
                        // self-correct on the next mouse-move event.
                        if !parsed_update.selection_compatible && !state.is_selecting {
                            state.reset_selection_and_adapter(cx);
                        }
                        cx.notify();
                    });
                }
            }
        });

        let _parse_task = cx.background_spawn(UpdateFuture::new(format, rx, tx_result));

        let mut this = Self {
            entity_id: cx.entity_id(),
            focus_handle,
            bounds: Bounds::default(),
            multi_click_selection: None,
            selected_text_override: None,
            select_all: false,
            selectable: false,
            selection_format: SelectionFormat::default(),
            scrollable: false,
            max_lines: None,
            line_spans: Arc::default(),
            clamped: false,
            // Measure all blocks (not just visible ones) so the scrollbar
            // thumb size stays stable. Without this, off-screen blocks count
            // as zero height until scrolled into view, which makes the
            // scrollbar jitter as more blocks get measured during scrolling.
            list_state: ListState::new(0, gpui::ListAlignment::Top, px(1000.)).measure_all(),
            text_view_style: Arc::default(),
            code_block_actions: None,
            code_block_highlighter: None,
            table_actions: None,
            link_click_handler: None,
            markdown_extensions: Arc::default(),
            is_selecting: false,
            preserve_inline_selection: false,
            auto_scroll: AutoScroll::default(),
            selection_adapter,
            parsed_content: Default::default(),
            stream_fade: StreamFadeTracker::default(),
            format,
            parsed_error: None,
            text: text.to_string(),
            element_text: None,
            revision: 0,
            committed_revision: 0,
            full_update_revision: 0,
            rendered_index: Arc::default(),
            range_highlights: None,
            selection_revision: 0,
            compatible_layout_update: false,
            layout_text_style: None,
            tx,
            _parse_task,
            _receive_task,
        };
        this.increment_update(&text, false, cx);
        this
    }

    /// Get the text content.
    pub(crate) fn source(&self) -> SharedString {
        self.parsed_content.document.source.clone()
    }

    /// Set whether the text is selectable, default false.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Set whether the text is selectable, default false.
    pub fn set_selectable(&mut self, selectable: bool, cx: &mut Context<Self>) {
        self.selectable = selectable;
        cx.notify();
    }

    /// Set the [`SelectionFormat`], default is [`SelectionFormat::Plain`].
    pub fn selection_format(mut self, selection_format: SelectionFormat) -> Self {
        self.selection_format = selection_format;
        self
    }

    /// Set the [`SelectionFormat`], default is [`SelectionFormat::Plain`].
    pub fn set_selection_format(
        &mut self,
        selection_format: SelectionFormat,
        cx: &mut Context<Self>,
    ) {
        self.selection_format = selection_format;
        cx.notify();
    }

    /// Set whether the text view scrolls internally, default false.
    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = scrollable;
        self
    }

    /// Set whether the text view scrolls internally, default false.
    pub fn set_scrollable(&mut self, scrollable: bool, cx: &mut Context<Self>) {
        if !scrollable {
            self.reset_selection_and_adapter(cx);
        }
        self.scrollable = scrollable;
        cx.notify();
    }

    /// Whether the last painted frame clipped content because of
    /// [`TextView::max_lines`](crate::text::TextView::max_lines).
    pub fn is_clamped(&self) -> bool {
        self.clamped
    }

    /// Set the text content.
    ///
    /// With a streamed fade-in enabled, text that extends the current text
    /// fades in like [`Self::push_str`] would; any other replacement shows at
    /// once.
    pub fn set_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if self.text.as_str() == text {
            return;
        }
        if self.stream_fade.is_enabled() {
            if text.starts_with(self.text.as_str()) {
                self.stream_fade.note_extend(self.text.len());
            } else {
                self.stream_fade.note_replace();
            }
        }

        self.text.clear();
        self.text.push_str(text);
        self.parsed_error = None;
        self.increment_update(text, false, cx);
    }

    /// [`Self::set_text`] for the text a `TextView` element hands over every
    /// frame: the string it handed over last time (the same allocation, not
    /// merely equal bytes) is recognized without a comparison, as long as the
    /// state still holds a text of its length.
    pub(super) fn set_element_text(&mut self, text: &SharedString, cx: &mut Context<Self>) {
        let same_string = self.element_text.as_ref().is_some_and(|current| {
            current.as_ptr() == text.as_ptr() && current.len() == text.len()
        });
        if same_string && self.text.len() == text.len() {
            return;
        }
        self.element_text = Some(text.clone());
        self.set_text(text, cx);
    }

    /// Append partial text content to the existing text.
    pub fn push_str(&mut self, new_text: &str, cx: &mut Context<Self>) {
        if new_text.is_empty() {
            return;
        }
        self.stream_fade.note_extend(self.text.len());
        self.text.push_str(new_text);
        self.increment_update(new_text, true, cx);
    }

    /// Set the motion policy; see [`TextViewMotion`].
    pub fn motion(mut self, motion: TextViewMotion) -> Self {
        self.set_motion(motion);
        self
    }

    /// Set the motion policy; see [`TextViewMotion`].
    pub fn set_motion(&mut self, motion: TextViewMotion) {
        self.stream_fade.set_motion(motion);
    }

    pub(crate) fn set_markdown_extensions(
        &mut self,
        markdown_extensions: Arc<MarkdownExtensions>,
        cx: &mut Context<Self>,
    ) {
        if self.markdown_extensions.revision() == markdown_extensions.revision() {
            return;
        }

        let parser_configuration_changed = !self
            .markdown_extensions
            .has_same_parser_configuration(&markdown_extensions);
        self.markdown_extensions = markdown_extensions;
        if parser_configuration_changed && self.format == TextViewFormat::Markdown {
            let text = self.text.clone();
            self.increment_update(&text, false, cx);
        }
    }

    /// Return the selected text, in the view's [`SelectionFormat`].
    pub fn selected_text(&self) -> String {
        self.selected_text_in(None)
    }

    /// Return the original Markdown source byte range corresponding to the
    /// rendered selection.
    ///
    /// The range addresses the source passed to this Markdown TextView. It is
    /// derived from parser positions retained by the rendered inline nodes, so
    /// identical rendered text maps to the occurrence that was actually
    /// selected. The result is one contiguous source range, so it includes any
    /// Markdown delimiters between the selected rendered endpoints. HTML views
    /// and selections without an exact source mapping return `None`. Select-all
    /// in a Markdown view returns the full source range.
    pub fn selected_source_range(&self) -> Option<Range<usize>> {
        if self.format != TextViewFormat::Markdown {
            return None;
        }
        if self.select_all {
            return Some(0..self.source().len());
        }
        self.parsed_content.document.selected_source_range()
    }

    /// The format to copy in, which is [`SelectionFormat::Plain`] whenever the
    /// requested one cannot be produced.
    ///
    /// Only a Markdown view can return source. Reconstructing HTML would mean
    /// spelling every attribute back out — a mark's color, an image's
    /// dimensions, a cell's alignment — with a new way to lose one at each
    /// step, and html5ever records no source offsets to fall back on (it
    /// reports only line numbers), so there is no original text to copy from
    /// either.
    fn effective_format(&self) -> SelectionFormat {
        match self.format {
            TextViewFormat::Markdown => self.selection_format,
            TextViewFormat::Html => SelectionFormat::Plain,
        }
    }

    /// Return the selected text, with `blocks` bounding which top-level blocks
    /// the selection covers.
    ///
    /// The range comes from the selection endpoints, which know their block
    /// even after it scrolls out of view; see
    /// [`ParsedDocument::selected_text`](crate::text::document::ParsedDocument).
    pub(super) fn selected_text_in(&self, blocks: Option<RangeInclusive<usize>>) -> String {
        let format = self.effective_format();

        if self.select_all {
            if format == SelectionFormat::Source {
                return self.source().to_string();
            }

            return self.parsed_content.document.text();
        }

        // A multi-click stores the plain text it selected, which is a shortcut
        // past the block walk. Source mode cannot take it: the word it stored
        // has lost its markup. The click also set the inline selection it came
        // from, so the walk reconstructs the same range with the markup intact.
        if format != SelectionFormat::Source
            && let Some(text) = &self.selected_text_override
        {
            return text.clone();
        }

        self.parsed_content.document.selected_text(format, blocks)
    }

    /// Force a full re-measure of the block list after the document has been
    /// replaced.
    ///
    /// `Document::render_root` only calls `ListState::reset` when the block
    /// *count* changes, and the full-measure pass enabled by `measure_all` is
    /// a one-shot latch that only `reset`, `remeasure_items`, or a width
    /// change re-arms. Replacing a document with one that happens to have the
    /// same number of blocks therefore leaves every cached height belonging to
    /// the *previous* document. The list summary height stays wrong, and since
    /// the wheel clamps against that summary, the blocks past the false bottom
    /// can never scroll into view to be re-measured -- the clamp seals itself.
    ///
    /// `remeasure_items` re-arms the latch and marks the items unmeasured
    /// while keeping their old sizes as hints, so the scrollbar does not
    /// collapse in the frame before the next layout measures the real heights.
    fn invalidate_measured_heights(&self) {
        let count = self.list_state.item_count();
        if count > 0 {
            self.list_state.remeasure_items(0..count);
        }
    }

    /// Remeasure inline resources after their prepared content or metrics change.
    ///
    /// Call from the resource owner's completion handler, through a weak entity
    /// when the task can outlive this view. This does not reparse the document.
    /// Existing logical selection is retained until the next selection gesture.
    pub fn invalidate_inline_layout(&mut self, cx: &mut Context<Self>) {
        self.preserve_inline_selection = true;
        self.compatible_layout_update = true;
        self.invalidate_measured_heights();
        cx.notify();
    }

    fn increment_update(&mut self, text: &str, append: bool, cx: &mut Context<Self>) {
        self.revision += 1;
        if !append {
            self.full_update_revision = self.revision;
            self.selection_revision = self.selection_revision.wrapping_add(1);
        }
        let parse_synchronously = !append && text.len() <= MAX_SYNC_FULL_REPLACE_BYTES;
        let update_options = UpdateOptions {
            revision: self.revision,
            append,
            mode: if append {
                ParseMode::Compatible
            } else if parse_synchronously {
                ParseMode::BaselineAck
            } else {
                ParseMode::Replace
            },
            pending_text: text.to_string(),
            markdown_extensions: self.markdown_extensions.clone(),
        };

        // Keep small full replacements synchronous so their first layout has
        // the exact content height. Larger replacements use the existing
        // background parser, bounding synchronous parser input on the UI thread.
        if parse_synchronously {
            match parse_content(self.format, ParsedContent::default(), &update_options) {
                Ok(content) => {
                    self.reconcile_range_highlights(&content.document, self.revision, false);
                    self.stream_fade.record(
                        &self.parsed_content.document,
                        &content.document,
                        Instant::now(),
                    );
                    self.parsed_content = content;
                    self.parsed_error = None;
                    self.invalidate_measured_heights();
                    if !self.is_selecting {
                        self.reset_selection_and_adapter(cx);
                    }
                }
                Err(err) => {
                    self.stream_fade.discard_pending();
                    self.parsed_error = Some(err);
                }
            }
            // Keep the background parser's accumulated document in sync so a
            // later append extends this baseline instead of parsing the delta
            // as a standalone document.
            _ = self.tx.try_send(update_options);
            cx.notify();
            return;
        }

        _ = self.tx.try_send(update_options);
    }

    /// The text this view renders, which [`RangeHighlight`] ranges index.
    ///
    /// This is the string plain copy produces, as of the last parse that
    /// landed; text set since then is not in it until its parse lands.
    pub fn rendered_text(&self) -> RenderedText {
        RenderedText::new(
            self.entity_id,
            self.committed_revision,
            self.parsed_content.document.clone(),
            self.rendered_index.clone(),
        )
    }

    /// Replace the range highlights, whose ranges index the current rendered text.
    ///
    /// Compute ranges from the current [`Self::rendered_text`] and set them in
    /// the same update. Search the new text again after its content changes.
    /// A range crossing blocks paints in both, skipping their separator.
    /// Text outside every block (separators, custom blocks, HTML blocks, and
    /// inline objects) is left unpainted. Any invalid range rejects the set.
    ///
    /// Highlights follow unchanged text through updates. Text appended while
    /// streaming keeps earlier highlights; after a table edit, cells in and
    /// after the edited row lose theirs because cells are known by position.
    /// Text backgrounds such as `<mark>` paint over a highlight; inline code
    /// backgrounds paint under it.
    pub fn set_range_highlights(
        &mut self,
        highlights: impl IntoIterator<Item = RangeHighlight>,
        cx: &mut Context<Self>,
    ) -> Result<(), RangeHighlightError> {
        let text = self.rendered_text();
        if self.format != TextViewFormat::Markdown {
            return Err(RangeHighlightError::Unsupported);
        }
        self.range_highlights = RangeHighlightFrame::new(&text, highlights)?.map(Arc::new);
        cx.notify();
        Ok(())
    }

    /// Remove all range highlights.
    pub fn clear_range_highlights(&mut self, cx: &mut Context<Self>) {
        if self.range_highlights.take().is_some() {
            cx.notify();
        }
    }

    /// Carry the range highlights over to `new`, the document the update of
    /// `revision` parsed, before it replaces the current one.
    ///
    /// `append` is whether that update parsed only appended text onto the
    /// document before it. Only when no update has replaced the text since
    /// the current document was committed was that document the current one,
    /// so only then do the blocks before its last stay unchanged.
    fn reconcile_range_highlights(&mut self, new: &ParsedDocument, revision: usize, append: bool) {
        let tail_only = append && self.full_update_revision <= self.committed_revision;
        self.committed_revision = revision;
        self.rendered_index = Arc::default();
        if let Some(highlights) = self.range_highlights.take() {
            self.range_highlights = highlights
                .remap(&self.parsed_content.document, new, tail_only)
                .map(Arc::new);
        }
    }

    /// Save bounds and unselect if bounds changed.
    pub(super) fn update_bounds(&mut self, bounds: Bounds<Pixels>, _cx: &mut App) {
        self.bounds = bounds;
    }

    /// The index of the top-level block at `content_y`, in this view's content
    /// coordinates (the same space the base selection endpoint stores its point in).
    ///
    /// Only laid-out blocks can be located, which is enough for a selection
    /// endpoint: the user can only put one where they can see it. Returns
    /// `None` for a view that is not virtualized, where every block paints and
    /// the range is not needed.
    pub(super) fn block_ix_at(&self, content_y: Pixels) -> Option<usize> {
        if !self.scrollable {
            return None;
        }

        let origin = self.bounds.origin.y + self.scroll_offset().y;
        let count = self.list_state.item_count();
        let mut ix = self.list_state.logical_scroll_top().item_ix;
        while ix < count {
            let bounds = self.list_state.bounds_for_item(ix)?;
            if content_y < bounds.bottom() - origin {
                return Some(ix);
            }
            ix += 1;
        }

        count.checked_sub(1)
    }

    #[doc(hidden)]
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    #[doc(hidden)]
    pub fn list_state(&self) -> &ListState {
        &self.list_state
    }

    #[doc(hidden)]
    pub fn is_selecting(&self) -> bool {
        self.is_selecting
    }

    #[doc(hidden)]
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    /// Whether this view has a view-local selection (select-all, multi-click, or override),
    /// independent of the window-level selection.
    pub(super) fn has_view_selection(&self) -> bool {
        self.select_all
            || self.multi_click_selection.is_some()
            || self.selected_text_override.is_some()
    }

    pub(super) fn stop_auto_scroll(&mut self) {
        self.auto_scroll.stop();
    }

    pub(super) fn reset_selection(&mut self) {
        self.preserve_inline_selection = false;
        self.multi_click_selection = None;
        self.selected_text_override = None;
        self.select_all = false;
        self.is_selecting = false;
        self.auto_scroll.stop();
        // Clear the inline selection state synchronously, so offscreen
        // (virtualized) views that won't repaint don't leak stale selection
        // text into a new cross-view copy.
        self.parsed_content.document.clear_selection();
    }

    fn reset_selection_and_adapter(&mut self, cx: &mut App) {
        self.reset_selection();
        self.selection_adapter.set_local_selection(false, cx);
    }

    /// Clear the current text selection.
    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        self.reset_selection_and_adapter(cx);
        cx.notify();
    }

    pub(super) fn scroll_offset(&self) -> Point<Pixels> {
        if self.scrollable {
            self.list_state.scroll_px_offset_for_scrollbar()
        } else {
            Point::default()
        }
    }

    /// Select all rendered text in this view.
    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        self.multi_click_selection = None;
        self.selected_text_override = None;
        self.select_all = true;
        self.is_selecting = false;
        self.auto_scroll.stop();
        self.selection_adapter.set_local_selection(true, cx);
        cx.notify();
    }

    pub(crate) fn set_multi_click_selection(
        &mut self,
        pos: Point<Pixels>,
        kind: TextViewMultiClickKind,
        selected_text: String,
        cx: &mut App,
    ) {
        self.preserve_inline_selection = false;
        let scroll_offset = self.scroll_offset();
        let pos = pos - self.bounds.origin - scroll_offset;
        self.multi_click_selection = Some(TextViewMultiClickSelection {
            pos,
            kind,
            line_bounds: None,
        });
        self.selected_text_override = Some(selected_text);
        self.select_all = false;
        self.is_selecting = false;
        self.auto_scroll.stop();
        self.selection_adapter.set_local_selection(true, cx);
    }

    pub(crate) fn set_multi_click_line(&mut self, bounds: Bounds<Pixels>, cx: &mut App) {
        self.set_multi_click_selection(
            bounds.center(),
            TextViewMultiClickKind::Line,
            String::new(),
            cx,
        );
        self.selected_text_override = None;
        let offset = self.bounds.origin + self.scroll_offset();
        if let Some(selection) = self.multi_click_selection.as_mut() {
            selection.line_bounds = Some(Bounds::new(bounds.origin - offset, bounds.size));
        }
    }

    pub(super) fn set_auto_scroll(&mut self, delta: Option<Pixels>, cx: &mut Context<Self>) {
        self.auto_scroll.set(delta, cx, |delta, state, cx| {
            state.list_state.scroll_by(delta);
            cx.notify();
        });
    }

    /// Return the window selection (anchor, cursor) in window coordinates if
    /// this view participates in it.
    ///
    /// Single-view fast path: when both endpoints are anchored inside one
    /// TextView, only that view participates (identical to the previous
    /// per-view behavior).
    pub(crate) fn selection_points(&self, cx: &App) -> Option<(Point<Pixels>, Point<Pixels>)> {
        if !self.selectable {
            return None;
        }
        self.selection_adapter.selection_points(cx)
    }

    pub(crate) fn has_selection(&self, cx: &App) -> bool {
        self.has_view_selection() || self.selection_points(cx).is_some()
    }

    pub(super) fn on_action_select_all(
        &mut self,
        _: &SelectAll,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.selectable {
            cx.propagate();
            return;
        }

        self.select_all(cx);
    }

    pub(crate) fn is_selectable(&self) -> bool {
        self.selectable
    }

    pub(crate) fn is_all_selected(&self) -> bool {
        self.select_all
    }

    pub(crate) fn multi_click_selection(&self) -> Option<TextViewMultiClickSelection> {
        let scroll_offset = self.scroll_offset();
        self.multi_click_selection.map(|selection| {
            let pos = selection.pos + scroll_offset + self.bounds.origin;
            let line_bounds = selection.line_bounds.map(|bounds| {
                Bounds::new(
                    bounds.origin + scroll_offset + self.bounds.origin,
                    bounds.size,
                )
            });
            TextViewMultiClickSelection {
                pos,
                line_bounds,
                ..selection
            }
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextViewMultiClickSelection {
    pub(crate) pos: Point<Pixels>,
    pub(crate) kind: TextViewMultiClickKind,
    pub(crate) line_bounds: Option<Bounds<Pixels>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextViewMultiClickKind {
    Word,
    Paragraph,
    Line,
}

impl Render for TextViewState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let typography = (window.text_style(), window.rem_size());
        if self
            .layout_text_style
            .as_ref()
            .is_some_and(|previous| previous != &typography)
        {
            // ListState invalidates its cached rows for width changes, but does
            // not know that an inherited font or rem change affects offscreen
            // inline metrics. Retain logical selections just as for resources.
            self.preserve_inline_selection = true;
            self.compatible_layout_update = true;
            self.invalidate_measured_heights();
        }
        self.layout_text_style = Some(typography);
        let state = cx.entity();
        let stream_fade = self.stream_fade.frame(Instant::now(), cx.reduce_motion());
        if stream_fade.is_some() {
            window.request_animation_frame();
        }
        // Built every frame, so everything in it is shared, not copied.
        let node_cx = NodeContext {
            offset: self.parsed_content.node_cx.offset,
            link_refs: self.parsed_content.node_cx.link_refs.clone(),
            style: self.text_view_style.clone(),
            code_block_actions: self.code_block_actions.clone(),
            code_block_highlighter: self.code_block_highlighter.clone(),
            table_actions: self.table_actions.clone(),
            link_click_handler: self.link_click_handler.clone(),
            markdown_extensions: self.markdown_extensions.clone(),
            stream_fade,
            range_highlights: self.range_highlights.clone(),
        };

        v_flex()
            .w_full()
            // Clamped content must keep its natural height: stretching it to
            // the capped box would hide the overflow the clamp has to measure.
            .when(self.max_lines.is_none(), |this| this.h_full())
            .map(|this| match &self.parsed_error {
                None => this.child(self.parsed_content.document.render_root(
                    if self.scrollable {
                        Some(self.list_state.clone())
                    } else {
                        None
                    },
                    &node_cx,
                    window,
                    cx,
                )),
                Some(err) => this.child(
                    v_flex()
                        .gap_1()
                        .child("Failed to parse content")
                        .child(err.to_string()),
                ),
            })
            .on_prepaint(move |bounds, window, cx| {
                let (
                    size_changed,
                    selection_involves_view,
                    has_selection_snapshot,
                    is_selecting,
                    compatible_layout_update,
                ) = {
                    let state = state.read(cx);
                    (
                        state.bounds().size != bounds.size,
                        state.selection_adapter.is_part_of_window_selection(cx),
                        state.selection_adapter.has_selection_snapshot(cx),
                        state.is_selecting,
                        state.compatible_layout_update,
                    )
                };
                let mut revision_changed = false;
                state.update(cx, |state, cx| {
                    revision_changed = state
                        .selection_adapter
                        .update_layout_revision(state.selection_revision, state.is_selecting);
                    state.update_bounds(bounds, cx);
                    state.compatible_layout_update = false;
                });
                if !is_selecting
                    && ((size_changed && selection_involves_view && !compatible_layout_update)
                        || (revision_changed && has_selection_snapshot))
                {
                    TextSelection::clear(window, cx);
                }
            })
    }
}

#[derive(Clone, PartialEq, Default)]
pub(crate) struct ParsedContent {
    pub(crate) document: ParsedDocument,
    pub(crate) node_cx: node::NodeContext,
}

struct UpdateFuture {
    format: TextViewFormat,
    content: ParsedContent,
    rx: Pin<Box<Receiver<UpdateOptions>>>,
    tx_result: Sender<ParsedUpdate>,
}

impl UpdateFuture {
    fn new(
        format: TextViewFormat,
        rx: Receiver<UpdateOptions>,
        tx_result: Sender<ParsedUpdate>,
    ) -> Self {
        Self {
            format,
            content: Default::default(),
            rx: Box::pin(rx),
            tx_result,
        }
    }
}

impl Future for UpdateFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.rx.as_mut().poll_next(cx) {
                Poll::Ready(Some(mut options)) => {
                    let hit_coalesce_budget =
                        merge_pending_options(&mut options, self.rx.as_ref().get_ref());

                    let res = parse_content(self.format, self.content.clone(), &options);
                    if let Ok(content) = &res {
                        self.content = content.clone();
                    }
                    _ = self.tx_result.try_send(ParsedUpdate {
                        revision: options.revision,
                        full_parse: !options.append,
                        selection_compatible: options.mode == ParseMode::Compatible,
                        baseline_ack: options.mode == ParseMode::BaselineAck,
                        result: res,
                    });
                    if hit_coalesce_budget {
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                    continue;
                }
                Poll::Ready(None) => return Poll::Ready(()),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[derive(Clone)]
struct UpdateOptions {
    revision: usize,
    pending_text: String,
    append: bool,
    mode: ParseMode,
    markdown_extensions: Arc<MarkdownExtensions>,
}

impl UpdateOptions {
    fn merge(&mut self, next: UpdateOptions) {
        if next.append {
            self.pending_text.push_str(&next.pending_text);
            self.revision = next.revision;
            if self.mode != ParseMode::Replace {
                self.mode = ParseMode::Compatible;
            }
        } else {
            *self = next;
        }
    }
}

struct ParsedUpdate {
    revision: usize,
    full_parse: bool,
    selection_compatible: bool,
    baseline_ack: bool,
    result: Result<ParsedContent, SharedString>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParseMode {
    BaselineAck,
    Replace,
    Compatible,
}

fn merge_pending_options(options: &mut UpdateOptions, rx: &Receiver<UpdateOptions>) -> bool {
    let mut update_count = 1;

    while update_count < MAX_COALESCED_UPDATES_PER_PARSE {
        match rx.try_recv() {
            Ok(next_options) => {
                options.merge(next_options);
                update_count += 1;
            }
            Err(_) => return false,
        }
    }

    true
}

fn parse_content(
    format: TextViewFormat,
    mut content: ParsedContent,
    options: &UpdateOptions,
) -> Result<ParsedContent, SharedString> {
    let mut node_cx = NodeContext {
        markdown_extensions: options.markdown_extensions.clone(),
        ..NodeContext::default()
    };

    // Re-parse the last block together with the appended text, so a block the
    // new text continues (an unclosed list, a fenced code block) is not split
    // in two. A block without a span cannot be located in `source` — the HTML
    // parser never records spans — so it is left in place and only the
    // appended text is parsed, positioned at the end of the current source.
    let last_span = options
        .append
        .then(|| {
            content
                .document
                .blocks
                .last()
                .and_then(|block| block.span())
        })
        .flatten();

    let mut source = String::new();
    if let Some(span) = last_span {
        Arc::make_mut(&mut content.document.blocks).pop();
        node_cx.offset = span.start;
        source.push_str(&content.document.source[span.start..]);
        source.push_str(&options.pending_text);
    } else {
        if options.append {
            node_cx.offset = content.document.source.len();
        }
        source.push_str(&options.pending_text);
    }

    let new_document = match format {
        TextViewFormat::Markdown => format::markdown::parse(&source, &mut node_cx),
        TextViewFormat::Html => format::html::parse(&source, &mut node_cx),
    }?;

    if options.append {
        content.document.source =
            format!("{}{}", content.document.source, options.pending_text).into();
        Arc::make_mut(&mut content.document.blocks)
            .extend(Arc::unwrap_or_clone(new_document.blocks));
    } else {
        content.document = new_document;
    }

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{MarkdownNode, node::BlockNode};
    use gpui::TestAppContext;

    mod stream_fade {
        use std::{ops::Range, time::Duration};

        use gpui::{Entity, TestAppContext};

        use super::super::*;
        use crate::{motion::Easing, text::stream_fade::TextLeafKey};

        /// Long enough that a debug test cannot outrun the fade before it
        /// samples the first frame.
        const FADE: Duration = Duration::from_secs(10);

        fn fading_state(markdown: &str, cx: &mut TestAppContext) -> Entity<TextViewState> {
            cx.update(crate::init);
            let state = cx.update(|cx| {
                cx.new(|cx| {
                    TextViewState::markdown(markdown, cx).motion(
                        TextViewMotion::default()
                            .with_stream_fade(FADE)
                            .with_stream_fade_easing(Easing::Linear),
                    )
                })
            });
            cx.run_until_parked();
            state
        }

        /// The fade ranges of `key` sampled right now, or `None` when the
        /// state has nothing fading.
        fn fades(
            state: &Entity<TextViewState>,
            key: TextLeafKey,
            cx: &mut TestAppContext,
        ) -> Option<Vec<Range<usize>>> {
            state.update(cx, |state, _| {
                let frame = state.stream_fade.frame(Instant::now(), false)?;
                let fades = frame.fades(key)?;
                assert!(
                    fades.iter().all(|(_, fade_out)| *fade_out > 0.9),
                    "a fade sampled right after it starts is still transparent: {fades:?}"
                );
                Some(fades.iter().map(|(range, _)| range.clone()).collect())
            })
        }

        #[gpui::test]
        fn push_str_fades_only_the_appended_text(cx: &mut TestAppContext) {
            let state = fading_state("hello", cx);
            state.update(cx, |state, cx| state.push_str(" world", cx));
            cx.run_until_parked();

            assert_eq!(fades(&state, TextLeafKey::block(0), cx), Some(vec![5..11]));

            state.update(cx, |state, _| {
                let later = Instant::now() + FADE + Duration::from_secs(1);
                assert!(state.stream_fade.frame(later, false).is_none());
                assert!(state.stream_fade.frame(Instant::now(), false).is_none());
            });
        }

        #[gpui::test]
        fn set_text_extending_the_text_fades_like_push_str(cx: &mut TestAppContext) {
            let state = fading_state("hello", cx);
            state.update(cx, |state, cx| state.set_text("hello world", cx));
            cx.run_until_parked();

            assert_eq!(fades(&state, TextLeafKey::block(0), cx), Some(vec![5..11]));
        }

        #[gpui::test]
        fn set_text_replacing_the_text_shows_it_at_once(cx: &mut TestAppContext) {
            let state = fading_state("hello", cx);
            state.update(cx, |state, cx| state.push_str(" world", cx));
            cx.run_until_parked();
            assert!(fades(&state, TextLeafKey::block(0), cx).is_some());

            state.update(cx, |state, cx| state.set_text("other", cx));
            cx.run_until_parked();

            assert_eq!(fades(&state, TextLeafKey::block(0), cx), None);
        }

        #[gpui::test]
        fn completed_markup_refades_from_the_divergence(cx: &mut TestAppContext) {
            // The paragraph renders `text` (trailing space trimmed), then
            // `text **bo` literally.
            let state = fading_state("text ", cx);
            state.update(cx, |state, cx| state.push_str("**bo", cx));
            cx.run_until_parked();
            assert_eq!(fades(&state, TextLeafKey::block(0), cx), Some(vec![4..9]));

            // `text **bold**` renders `text bold`: the space keeps its fade,
            // the glyphs from byte 5 on changed and fade again as one run.
            state.update(cx, |state, cx| state.push_str("ld**", cx));
            cx.run_until_parked();
            assert_eq!(
                fades(&state, TextLeafKey::block(0), cx),
                Some(vec![4..5, 5..9])
            );
        }

        #[gpui::test]
        fn without_stagger_an_update_fades_as_one_chunk(cx: &mut TestAppContext) {
            let state = fading_state("hello", cx);
            state.update(cx, |state, cx| state.push_str(" one two three", cx));
            cx.run_until_parked();

            assert_eq!(fades(&state, TextLeafKey::block(0), cx), Some(vec![5..19]));
        }

        #[gpui::test]
        fn words_of_one_update_start_one_after_another(cx: &mut TestAppContext) {
            let stagger = Duration::from_millis(100);
            let state = fading_state("hello", cx);
            state.update(cx, |state, _| {
                state.set_motion(
                    TextViewMotion::default()
                        .with_stream_fade(FADE)
                        .with_stream_fade_stagger(stagger)
                        .with_stream_fade_easing(Easing::Linear),
                )
            });
            state.update(cx, |state, cx| state.push_str(" one two three", cx));
            cx.run_until_parked();

            state.update(cx, |state, _| {
                let frame = state
                    .stream_fade
                    .frame(Instant::now() + stagger * 3, false)
                    .expect("words still fading");
                let fades = frame.fades(TextLeafKey::block(0)).expect("paragraph fades");
                let ranges: Vec<_> = fades.iter().map(|(range, _)| range.clone()).collect();
                assert_eq!(ranges, vec![5..10, 10..14, 14..19]);
                // A later word has faded less, so it is still more transparent.
                assert!(
                    fades[0].1 < fades[1].1 && fades[1].1 < fades[2].1,
                    "{fades:?}"
                );
            });
        }

        #[gpui::test]
        fn a_new_paragraph_fades_as_a_whole(cx: &mut TestAppContext) {
            let state = fading_state("first", cx);
            state.update(cx, |state, cx| state.push_str("\n\nsecond", cx));
            cx.run_until_parked();

            assert_eq!(fades(&state, TextLeafKey::block(0), cx), None);
            assert_eq!(fades(&state, TextLeafKey::block(7), cx), Some(vec![0..6]));
        }

        #[gpui::test]
        fn code_block_text_fades_by_block(cx: &mut TestAppContext) {
            let state = fading_state("```rs\nlet", cx);
            state.update(cx, |state, cx| state.push_str(" x", cx));
            cx.run_until_parked();

            assert_eq!(fades(&state, TextLeafKey::block(0), cx), Some(vec![3..5]));
        }

        #[gpui::test]
        fn table_cells_fade_by_ordinal(cx: &mut TestAppContext) {
            let state = fading_state("| a | b |\n|---|---|\n| c | d", cx);
            state.update(cx, |state, cx| state.push_str("e |", cx));
            cx.run_until_parked();

            assert_eq!(fades(&state, TextLeafKey::table_cell(0, 2), cx), None);
            assert_eq!(
                fades(&state, TextLeafKey::table_cell(0, 3), cx),
                Some(vec![1..2])
            );
        }

        #[gpui::test]
        fn zero_duration_records_nothing(cx: &mut TestAppContext) {
            cx.update(crate::init);
            let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("hello", cx)));
            cx.run_until_parked();
            state.update(cx, |state, cx| state.push_str(" world", cx));
            cx.run_until_parked();

            state.update(cx, |state, _| {
                assert!(state.stream_fade.frame(Instant::now(), false).is_none());
            });
        }

        #[gpui::test]
        fn reduced_motion_drops_the_fade(cx: &mut TestAppContext) {
            let state = fading_state("hello", cx);
            state.update(cx, |state, cx| state.push_str(" world", cx));
            cx.run_until_parked();

            state.update(cx, |state, _| {
                assert!(state.stream_fade.frame(Instant::now(), true).is_none());
                assert!(state.stream_fade.frame(Instant::now(), false).is_none());
            });
        }
    }

    #[gpui::test]
    fn small_full_replace_parses_before_background_executor_runs(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let markdown = "# ready";
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown(markdown, cx)));

        state.read_with(cx, |state, _| {
            assert_eq!(state.source().as_str(), markdown);
            assert_eq!(state.parsed_content.document.blocks.len(), 1);
        });
    }

    #[gpui::test]
    fn large_markdown_and_html_full_replacements_wait_for_background_executor(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let markdown = "# x\n\n".repeat(MAX_SYNC_FULL_REPLACE_BYTES / 5 + 1);
        let html = format!("<p>{}</p>", "x".repeat(MAX_SYNC_FULL_REPLACE_BYTES + 1));
        assert!(markdown.len() > MAX_SYNC_FULL_REPLACE_BYTES);
        assert!(html.len() > MAX_SYNC_FULL_REPLACE_BYTES);

        let (markdown_state, html_state) = cx.update(|cx| {
            (
                cx.new(|cx| TextViewState::markdown(&markdown, cx)),
                cx.new(|cx| TextViewState::html(&html, cx)),
            )
        });

        markdown_state.read_with(cx, |state, _| {
            assert_eq!(state.text.as_str(), markdown.as_str());
            assert!(state.source().as_str().is_empty());
            assert!(state.parsed_content.document.blocks.is_empty());
        });
        html_state.read_with(cx, |state, _| {
            assert_eq!(state.text.as_str(), html.as_str());
            assert!(state.source().as_str().is_empty());
            assert!(state.parsed_content.document.blocks.is_empty());
        });

        cx.run_until_parked();

        markdown_state.read_with(cx, |state, _| {
            assert_eq!(state.source().as_str(), markdown.as_str());
            assert!(!state.parsed_content.document.blocks.is_empty());
        });
        html_state.read_with(cx, |state, _| {
            assert_eq!(state.source().as_str(), html.as_str());
            assert!(!state.parsed_content.document.blocks.is_empty());
        });
    }

    #[gpui::test]
    fn inline_source_ranges_follow_streamed_tail_reparsing(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("中文 $a$\n\n尾 $x", cx)));
        state.update(cx, |state, cx| {
            let extensions = MarkdownExtensions::default().plugin(
                crate::text::markdown_ext::TestInlinePlugin::new("test").parse_with(|node, _| {
                    let markdown::mdast::Node::InlineMath(math) = node else {
                        return None;
                    };
                    Some(super::super::MarkdownNode::new("formula", ()).text(math.value.clone()))
                }),
            );
            state.set_markdown_extensions(Arc::new(extensions), cx);
        });
        cx.run_until_parked();
        state.update(cx, |state, cx| state.push_str("^2$ 后", cx));
        cx.run_until_parked();
        state.read_with(cx, |state, _| {
            let source = "中文 $a$\n\n尾 $x^2$ 后";
            assert_eq!(state.source().as_str(), source);
            let objects: Vec<_> = state
                .parsed_content
                .document
                .blocks
                .iter()
                .flat_map(|block| {
                    let super::super::node::BlockNode::Paragraph(paragraph) = block else {
                        panic!()
                    };
                    paragraph
                        .children
                        .iter()
                        .filter_map(|node| node.custom.as_ref())
                })
                .collect();
            assert_eq!(objects.len(), 2);
            for (object, markdown) in objects.iter().zip(["$a$", "$x^2$"]) {
                let start = source.find(markdown).unwrap();
                assert_eq!(object.source_range(), Some(start..start + markdown.len()));
                assert_eq!(object.as_markdown(), markdown);
            }
        });
    }

    #[gpui::test]
    fn async_full_replace_then_push_str_preserves_complete_source(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("old", cx)));
        cx.run_until_parked();

        let replacement = "x".repeat(MAX_SYNC_FULL_REPLACE_BYTES + 1);
        let expected = format!("{replacement} tail");
        state.update(cx, |state, cx| {
            state.set_text(&replacement, cx);
            state.push_str(" tail", cx);
        });
        cx.run_until_parked();

        state.read_with(cx, |state, _| {
            assert_eq!(state.text.as_str(), expected.as_str());
            assert_eq!(state.source().as_str(), expected.as_str());
        });
    }

    #[gpui::test]
    fn html_push_str_keeps_earlier_blocks(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::html("<p>first</p>", cx)));
        cx.run_until_parked();

        state.update(cx, |state, cx| {
            state.push_str("<p>second</p>", cx);
        });
        cx.run_until_parked();

        state.read_with(cx, |state, _| {
            assert_eq!(state.source().as_str(), "<p>first</p><p>second</p>");
            let text = state
                .parsed_content
                .document
                .blocks
                .iter()
                .map(|block| block.text())
                .collect::<String>();
            assert!(text.contains("first"), "lost the first block: {text:?}");
            assert!(text.contains("second"), "lost the appended block: {text:?}");
        });
    }

    #[gpui::test]
    fn element_text_of_the_same_string_is_not_compared_again(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("", cx)));
        let revision = |cx: &mut TestAppContext| state.read_with(cx, |state, _| state.revision);
        let text = SharedString::from("hello");

        state.update(cx, |state, cx| state.set_element_text(&text, cx));
        let parsed = revision(cx);
        assert!(parsed > 0);

        // The same allocation again, and equal bytes in another allocation,
        // both leave the parsed text alone.
        state.update(cx, |state, cx| state.set_element_text(&text, cx));
        assert_eq!(revision(cx), parsed);
        let equal = SharedString::from("hello".to_string());
        state.update(cx, |state, cx| state.set_element_text(&equal, cx));
        assert_eq!(revision(cx), parsed);

        // Once the state's text moved on, the element's string is set again.
        state.update(cx, |state, cx| {
            state.push_str(" world", cx);
            state.set_element_text(&equal, cx);
        });
        state.read_with(cx, |state, _| assert_eq!(state.text.as_str(), "hello"));
    }

    #[gpui::test]
    fn set_text_then_push_str_appends_to_replaced_content(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("old", cx)));
        cx.run_until_parked();

        state.update(cx, |state, cx| {
            state.set_text("", cx);
            state.push_str("new", cx);
            state.push_str(" text", cx);
        });
        cx.run_until_parked();

        state.read_with(cx, |state, _| {
            assert_eq!(state.text.as_str(), "new text");
            assert_eq!(state.source().as_str(), "new text");
        });

        state.update(cx, |state, cx| {
            state.set_text("", cx);
        });
        cx.run_until_parked();

        state.read_with(cx, |state, _| {
            assert_eq!(state.text.as_str(), "");
            assert_eq!(state.source().as_str(), "");
        });
    }

    #[gpui::test]
    fn full_parse_coalesced_with_append_preserves_new_select_all(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("old", cx)));
        cx.run_until_parked();

        state.update(cx, |state, cx| {
            state.set_text("new", cx);
            state.push_str(" text", cx);
            state.select_all(cx);
        });
        cx.run_until_parked();

        state.read_with(cx, |state, _| {
            assert!(state.select_all);
            assert_eq!(state.selected_text().trim(), "new text");
        });
    }

    #[test]
    fn update_options_merge_keeps_latest_full_text() {
        let mut options = UpdateOptions {
            revision: 1,
            pending_text: "old".to_string(),
            append: true,
            mode: ParseMode::Compatible,
            markdown_extensions: Arc::default(),
        };

        options.merge(UpdateOptions {
            revision: 2,
            pending_text: "new".to_string(),
            append: false,
            mode: ParseMode::BaselineAck,
            markdown_extensions: Arc::default(),
        });
        options.merge(UpdateOptions {
            revision: 3,
            pending_text: " text".to_string(),
            append: true,
            mode: ParseMode::Compatible,
            markdown_extensions: Arc::default(),
        });

        assert_eq!(options.revision, 3);
        assert_eq!(options.pending_text, "new text");
        assert!(!options.append);
    }

    #[test]
    fn append_merged_into_async_replace_remains_a_replacement() {
        let mut options = UpdateOptions {
            revision: 1,
            pending_text: "new".to_string(),
            append: false,
            mode: ParseMode::Replace,
            markdown_extensions: Arc::default(),
        };

        options.merge(UpdateOptions {
            revision: 2,
            pending_text: " text".to_string(),
            append: true,
            mode: ParseMode::Compatible,
            markdown_extensions: Arc::default(),
        });

        assert_eq!(options.revision, 2);
        assert_eq!(options.pending_text, "new text");
        assert!(!options.append);
        assert_eq!(options.mode, ParseMode::Replace);
    }

    #[test]
    fn update_future_yields_before_coalescing_all_queued_updates() {
        let (tx, rx) = unbounded::<UpdateOptions>();
        let (tx_result, rx_result) = unbounded::<ParsedUpdate>();
        let total_updates = 128;

        for revision in 1..=total_updates {
            tx.try_send(UpdateOptions {
                revision,
                pending_text: format!("{revision}\n"),
                append: revision != 1,
                mode: if revision == 1 {
                    ParseMode::BaselineAck
                } else {
                    ParseMode::Compatible
                },
                markdown_extensions: Arc::default(),
            })
            .unwrap();
        }

        let mut future = Box::pin(UpdateFuture::new(TextViewFormat::Markdown, rx, tx_result));
        let waker = futures::task::noop_waker();
        let mut task_cx = std::task::Context::from_waker(&waker);

        assert!(matches!(
            std::future::Future::poll(future.as_mut(), &mut task_cx),
            Poll::Pending
        ));
        let parsed_update = rx_result.try_recv().expect("parse result");

        assert!(
            parsed_update.revision < total_updates,
            "single poll coalesced every queued update through revision {}",
            parsed_update.revision
        );

        assert!(matches!(
            std::future::Future::poll(future.as_mut(), &mut task_cx),
            Poll::Pending
        ));
        let parsed_update = rx_result.try_recv().expect("next parse result");
        assert_eq!(parsed_update.revision, total_updates);
    }

    #[gpui::test]
    fn select_all_returns_rendered_text(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("**quick** value", cx)));
        cx.run_until_parked();

        state.update(cx, |state, cx| {
            state.select_all(cx);
        });

        state.read_with(cx, |state, _| {
            assert!(state.has_view_selection());
            assert_eq!(state.selected_text().trim(), "quick value");
        });

        state.update(cx, |state, cx| {
            state.clear_selection(cx);
        });

        state.read_with(cx, |state, _| {
            assert!(!state.has_view_selection());
            assert_eq!(state.selected_text(), "");
        });
    }

    #[gpui::test]
    fn select_all_in_source_format_returns_source(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let markdown = "**quick** value";
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown(markdown, cx)));
        cx.run_until_parked();

        state.update(cx, |state, cx| state.select_all(cx));

        // The default (plain) mode strips the markup.
        state.read_with(cx, |state, _| {
            assert_eq!(state.selected_text().trim(), "quick value");
        });

        state.update(cx, |state, cx| {
            state.set_selection_format(SelectionFormat::Source, cx)
        });

        // Source mode yields the whole source verbatim.
        state.read_with(cx, |state, _| {
            assert_eq!(state.selected_text().trim(), markdown);
        });
    }

    #[gpui::test]
    fn selected_source_range_returns_full_markdown_source_for_select_all(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let markdown = "**quick** value";
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown(markdown, cx)));
        cx.run_until_parked();

        state.update(cx, |state, cx| state.select_all(cx));

        state.read_with(cx, |state, _| {
            assert_eq!(state.selected_source_range(), Some(0..markdown.len()));
        });
    }

    #[gpui::test]
    fn selected_source_range_returns_none_for_html(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::html("<b>quick</b>", cx)));
        cx.run_until_parked();

        state.update(cx, |state, cx| state.select_all(cx));

        state.read_with(cx, |state, _| {
            assert_eq!(state.selected_source_range(), None);
        });
    }

    #[test]
    fn selected_source_range_keeps_global_offsets_after_incremental_tail_parse() {
        let options = UpdateOptions {
            revision: 1,
            pending_text: "first\n\nsecond".to_string(),
            append: false,
            mode: ParseMode::Replace,
            markdown_extensions: Arc::default(),
        };
        let content = parse_content(TextViewFormat::Markdown, ParsedContent::default(), &options)
            .expect("initial parse");
        let content = parse_content(
            TextViewFormat::Markdown,
            content,
            &UpdateOptions {
                revision: 2,
                pending_text: "\n\n**écho**".to_string(),
                append: true,
                mode: ParseMode::Compatible,
                markdown_extensions: Arc::default(),
            },
        )
        .expect("incremental parse");
        let BlockNode::Paragraph(paragraph) = &content.document.blocks[2] else {
            panic!("expected appended paragraph");
        };
        let mut state = paragraph.state.lock().unwrap();
        state.set_text(paragraph.text().into());
        state.selection = Some((0.."écho".len()).into());
        drop(state);

        assert_eq!(content.document.selected_source_range(), Some(17..22));
    }

    #[test]
    fn streamed_ordered_list_continuation_preserves_start() {
        let initial = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            &UpdateOptions {
                revision: 1,
                pending_text: "3. three".to_string(),
                append: false,
                mode: ParseMode::Replace,
                markdown_extensions: Arc::default(),
            },
        )
        .expect("initial list parse");
        let continued = parse_content(
            TextViewFormat::Markdown,
            initial,
            &UpdateOptions {
                revision: 2,
                pending_text: "\n4. four".to_string(),
                append: true,
                mode: ParseMode::Compatible,
                markdown_extensions: Arc::default(),
            },
        )
        .expect("streamed list continuation parse");

        let BlockNode::List {
            start, children, ..
        } = &continued.document.blocks[0]
        else {
            panic!("expected streamed ordered list");
        };
        assert_eq!(*start, Some(3));
        assert_eq!(children.len(), 2);
    }

    #[gpui::test]
    fn parser_revision_reparses_same_name_inline_configuration(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("@member", cx)));
        for (revision, label) in [(1, "Alice"), (2, "Bob")] {
            let extensions = MarkdownExtensions::default()
                .parser_revision(revision)
                .plugin(
                    crate::text::markdown_ext::TestInlinePlugin::new("test").parse_with(
                        move |node, _| {
                            matches!(node, markdown::mdast::Node::Text(_))
                                .then(|| MarkdownNode::new("mention", label).text(label))
                        },
                    ),
                );
            state.update(cx, |state, cx| {
                state.set_markdown_extensions(Arc::new(extensions), cx)
            });
            cx.run_until_parked();
            state.read_with(cx, |state, _| {
                let node::BlockNode::Paragraph(paragraph) =
                    &state.parsed_content.document.blocks[0]
                else {
                    panic!()
                };
                assert_eq!(
                    paragraph.children[0].custom.as_ref().unwrap().as_text(),
                    label
                );
            });
        }
    }

    #[gpui::test]
    fn set_markdown_extensions_reparses_existing_text(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("$TSLA.US", cx)));
        cx.run_until_parked();

        let extensions = MarkdownExtensions::default().block_parser(|node, cx| {
            let markdown::mdast::Node::Paragraph(paragraph) = node else {
                return None;
            };
            let [markdown::mdast::Node::Text(text)] = paragraph.children.as_slice() else {
                return None;
            };
            let symbol = text.value.strip_prefix('$')?.to_string();
            let node_text = format!("${symbol}");

            Some(
                MarkdownNode::new("ticker", symbol)
                    .text(node_text)
                    .markdown(cx.node_source(node).unwrap_or_default()),
            )
        });

        state.update(cx, |state, cx| {
            state.set_markdown_extensions(Arc::new(extensions), cx);
        });
        cx.run_until_parked();

        state.read_with(cx, |state, _| {
            let node::BlockNode::Custom(node) = &state.parsed_content.document.blocks[0] else {
                panic!("expected custom markdown node");
            };
            assert_eq!(node.name(), "ticker");
            assert_eq!(node.data::<String>().map(String::as_str), Some("TSLA.US"));
        });
    }

    mod range_highlights {
        use std::ops::Range;

        use gpui::{Entity, TestAppContext};

        use super::super::*;
        use crate::text::stream_fade::TextLeafKey;

        fn state(markdown: &str, cx: &mut TestAppContext) -> Entity<TextViewState> {
            cx.update(crate::init);
            let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown(markdown, cx)));
            cx.run_until_parked();
            state
        }

        fn highlight(range: Range<usize>) -> RangeHighlight {
            RangeHighlight::new(range, gpui::hsla(0.15, 1., 0.5, 0.4))
        }

        /// Highlights `ranges` of the current rendered text.
        fn set(
            state: &Entity<TextViewState>,
            ranges: impl IntoIterator<Item = Range<usize>>,
            cx: &mut TestAppContext,
        ) -> Result<(), RangeHighlightError> {
            state.update(cx, |state, cx| {
                state.set_range_highlights(ranges.into_iter().map(highlight), cx)
            })
        }

        /// The ranges leaf `key` paints, in its own byte space.
        fn painted(
            state: &Entity<TextViewState>,
            key: TextLeafKey,
            cx: &mut TestAppContext,
        ) -> Vec<Range<usize>> {
            state.read_with(cx, |state, _| {
                state.range_highlights.as_ref().map_or(Vec::new(), |frame| {
                    frame
                        .backgrounds(key)
                        .iter()
                        .map(|(range, _)| range.clone())
                        .collect()
                })
            })
        }

        fn has_highlights(state: &Entity<TextViewState>, cx: &mut TestAppContext) -> bool {
            state.read_with(cx, |state, _| state.range_highlights.is_some())
        }

        #[gpui::test]
        fn rendered_text_is_the_plain_copy_text(cx: &mut TestAppContext) {
            let state = state(
                "# Title\n\nhello **world** \\*\n\n- item\n\n| a | b |\n|---|---|\n| c | d |\n\n```\nlet x\n```",
                cx,
            );
            state.read_with(cx, |state, _| {
                assert_eq!(
                    state.rendered_text().as_str(),
                    "Title\nhello world *\nitem\na b\nc d\n\nlet x\n"
                );
            });
        }

        #[gpui::test]
        fn a_match_across_marks_paints_in_its_paragraph(cx: &mut TestAppContext) {
            let state = state("hello **world**", cx);
            set(&state, [0.."hello world".len()], cx).unwrap();
            assert_eq!(painted(&state, TextLeafKey::block(0), cx), [0..11]);
        }

        #[gpui::test]
        fn repeated_text_maps_to_the_occurrence_addressed(cx: &mut TestAppContext) {
            let state = state("foo\n\nfoo", cx);
            let second = state.read_with(cx, |state, _| {
                state.rendered_text().as_str().rfind("foo").unwrap()
            });
            set(&state, [second..second + 3], cx).unwrap();
            assert!(painted(&state, TextLeafKey::block(0), cx).is_empty());
            assert_eq!(painted(&state, TextLeafKey::block(5), cx), [0..3]);
        }

        #[gpui::test]
        fn a_range_across_blocks_skips_the_separator(cx: &mut TestAppContext) {
            let state = state("ab\n\ncd", cx);
            // "ab\ncd\n": 1..4 is "b\nc".
            set(&state, [1..4], cx).unwrap();
            assert_eq!(painted(&state, TextLeafKey::block(0), cx), [1..2]);
            assert_eq!(painted(&state, TextLeafKey::block(4), cx), [0..1]);
        }

        #[gpui::test]
        fn table_cells_and_code_blocks_are_leaves(cx: &mut TestAppContext) {
            let state = state("| a | b |\n|---|---|\n| c | d |\n\n```\nlet x\n```", cx);
            // "a b\nc d\n\nlet x\n"
            set(&state, [0..3, 6..7, 9..12], cx).unwrap();
            assert_eq!(painted(&state, TextLeafKey::table_cell(0, 0), cx), [0..1]);
            assert_eq!(painted(&state, TextLeafKey::table_cell(0, 1), cx), [0..1]);
            assert_eq!(painted(&state, TextLeafKey::table_cell(0, 3), cx), [0..1]);
            let code_start = "| a | b |\n|---|---|\n| c | d |\n\n".len();
            assert_eq!(painted(&state, TextLeafKey::block(code_start), cx), [0..3]);
        }

        #[gpui::test]
        fn invalid_ranges_reject_the_whole_set(cx: &mut TestAppContext) {
            let state = state("中文\n\nab", cx);
            // "中文\nab\n"
            set(&state, [0.."中".len()], cx).unwrap();
            let invalid = [Range { start: 6, end: 3 }, 0..1, 0..100];
            for range in invalid {
                assert_eq!(
                    set(&state, [0..3, range.clone()], cx),
                    Err(RangeHighlightError::InvalidRange(1)),
                    "{range:?}"
                );
            }
            // The rejected sets left the earlier highlight in place.
            assert_eq!(painted(&state, TextLeafKey::block(0), cx), [0..3]);
        }

        #[gpui::test]
        fn text_outside_every_block_is_left_unpainted(cx: &mut TestAppContext) {
            let source = "foo one\n\n<div>foo two</div>\n\n| foo | x |\n|---|---|\n\nfoo three";
            let state = state(source, cx);
            let text = state.read_with(cx, |state, _| state.rendered_text().as_str().to_string());
            // Every "foo" and " ", an empty range, and the separator after the
            // first block: the HTML block's text and the separators are not
            // any block's text, so they paint nothing, but nothing fails.
            let ranges = text
                .match_indices("foo")
                .chain(text.match_indices(' '))
                .map(|(start, found)| start..start + found.len())
                .chain([3..3, 7..8])
                .collect::<Vec<_>>();
            set(&state, ranges, cx).unwrap();
            assert_eq!(painted(&state, TextLeafKey::block(0), cx), [0..3, 3..4]);
            let table = source.find("| foo").unwrap();
            assert_eq!(
                painted(&state, TextLeafKey::table_cell(table, 0), cx),
                [0..3]
            );
            let last = source.find("foo three").unwrap();
            assert_eq!(painted(&state, TextLeafKey::block(last), cx), [0..3, 3..4]);
        }

        #[gpui::test]
        fn a_later_highlight_paints_over_an_earlier_one(cx: &mut TestAppContext) {
            let state = state("abcdef", cx);
            set(&state, [0..6, 2..3], cx).unwrap();
            assert_eq!(painted(&state, TextLeafKey::block(0), cx), [0..6, 2..3]);
        }

        #[gpui::test]
        fn html_text_is_unsupported(cx: &mut TestAppContext) {
            let html = cx.update(|cx| cx.new(|cx| TextViewState::html("<p>one</p>", cx)));
            cx.run_until_parked();
            html.update(cx, |html, cx| {
                assert_eq!(
                    html.set_range_highlights([highlight(0..3)], cx),
                    Err(RangeHighlightError::Unsupported)
                );
            });
        }

        #[gpui::test]
        fn inline_objects_are_skipped(cx: &mut TestAppContext) {
            let state = state("x $a$ y", cx);
            state.update(cx, |state, cx| {
                let extensions = MarkdownExtensions::default().plugin(
                    crate::text::markdown_ext::TestInlinePlugin::new("test").parse_with(
                        |node, _| {
                            let markdown::mdast::Node::InlineMath(math) = node else {
                                return None;
                            };
                            Some(
                                crate::text::MarkdownNode::new("formula", ())
                                    .text(math.value.clone()),
                            )
                        },
                    ),
                );
                state.set_markdown_extensions(Arc::new(extensions), cx);
            });
            cx.run_until_parked();
            // "x a y\n", where "a" is the formula.
            set(&state, [2..3], cx).unwrap();
            assert!(painted(&state, TextLeafKey::block(0), cx).is_empty());
            set(&state, [0..5], cx).unwrap();
            assert_eq!(painted(&state, TextLeafKey::block(0), cx), [0..2, 3..5]);
        }

        #[gpui::test]
        fn push_str_keeps_earlier_blocks_and_clips_the_changed_tail(cx: &mut TestAppContext) {
            let state = state("first\n\na **b", cx);
            // "first\na **b\n"
            set(&state, [0..5, 6..11], cx).unwrap();

            // Closing the emphasis renders the tail as "a bc".
            state.update(cx, |state, cx| state.push_str("c**", cx));
            cx.run_until_parked();

            assert_eq!(painted(&state, TextLeafKey::block(0), cx), [0..5]);
            assert_eq!(painted(&state, TextLeafKey::block(7), cx), [0..2]);
        }

        #[gpui::test]
        fn push_str_keeps_a_block_whose_text_is_unchanged(cx: &mut TestAppContext) {
            let state = state("first\n\nsecond", cx);
            set(&state, [6..12], cx).unwrap();
            // A setext underline turns the paragraph into a heading at the
            // same source start, rendering the same text.
            state.update(cx, |state, cx| state.push_str("\n===", cx));
            cx.run_until_parked();
            assert_eq!(painted(&state, TextLeafKey::block(7), cx), [0..6]);
        }

        #[gpui::test]
        fn push_str_drops_highlights_of_a_leaf_that_is_gone(cx: &mut TestAppContext) {
            let state = state("first\n\n| a |", cx);
            set(&state, [0..5, 6..11], cx).unwrap();
            // A delimiter row turns the paragraph into a table, whose text
            // lives in cells instead.
            state.update(cx, |state, cx| state.push_str("\n|---|", cx));
            cx.run_until_parked();
            assert_eq!(painted(&state, TextLeafKey::block(0), cx), [0..5]);
            assert!(painted(&state, TextLeafKey::block(7), cx).is_empty());
            assert!(painted(&state, TextLeafKey::table_cell(7, 0), cx).is_empty());
        }

        #[gpui::test]
        fn replacing_text_drops_highlights_only_where_it_changed(cx: &mut TestAppContext) {
            let state = state("first\n\nsecond", cx);
            set(&state, [0..5, 6..12], cx).unwrap();
            state.update(cx, |state, cx| state.set_text("first\n\nchanged", cx));
            cx.run_until_parked();
            assert_eq!(painted(&state, TextLeafKey::block(0), cx), [0..5]);
            assert!(painted(&state, TextLeafKey::block(7), cx).is_empty());

            let large = "x".repeat(MAX_SYNC_FULL_REPLACE_BYTES + 1);
            state.update(cx, |state, cx| state.set_text(&large, cx));
            cx.run_until_parked();
            assert!(!has_highlights(&state, cx));
        }

        #[gpui::test]
        fn a_highlight_follows_its_block_past_an_earlier_edit(cx: &mut TestAppContext) {
            let state = state("foo\n\nbar\n\nbar", cx);
            // "foo\nbar\nbar\n": the first "bar", in the block at source 5.
            set(&state, [4..7], cx).unwrap();
            // Deleting "foo" moves that "bar" to 0, and the second one to 5.
            state.update(cx, |state, cx| state.set_text("bar\n\nbar", cx));
            cx.run_until_parked();
            assert_eq!(painted(&state, TextLeafKey::block(0), cx), [0..3]);
            assert!(painted(&state, TextLeafKey::block(5), cx).is_empty());

            let state = self::state("ERROR a\n\nERROR b\n\nERROR c", cx);
            set(&state, [8..15], cx).unwrap();
            state.update(cx, |state, cx| state.set_text("ERROR b\n\nERROR c", cx));
            cx.run_until_parked();
            assert_eq!(painted(&state, TextLeafKey::block(0), cx), [0..7]);
            assert!(painted(&state, TextLeafKey::block(9), cx).is_empty());
        }

        #[gpui::test]
        fn appending_a_copy_of_the_last_block_keeps_the_highlight_on_it(cx: &mut TestAppContext) {
            let state = state("a\n\nfoo", cx);
            // "a\nfoo\n"
            set(&state, [2..5], cx).unwrap();
            state.update(cx, |state, cx| state.set_text("a\n\nfoo\n\nfoo", cx));
            cx.run_until_parked();
            assert_eq!(painted(&state, TextLeafKey::block(3), cx), [0..3]);
            assert!(painted(&state, TextLeafKey::block(8), cx).is_empty());
        }

        #[gpui::test]
        fn deleting_a_table_row_drops_highlights_in_the_rows_it_moves(cx: &mut TestAppContext) {
            let table = "| a | b |\n|---|---|\n| x | 1 |\n| x | 2 |";
            let without_middle_row = "| a | b |\n|---|---|\n| x | 2 |";
            // "a b\nx 1\nx 2\n\n": the last row's cells, then the middle
            // row's "x".
            for ranges in [vec![8..9, 10..11], vec![4..5]] {
                let state = state(table, cx);
                set(&state, [0..1].into_iter().chain(ranges), cx).unwrap();
                state.update(cx, |state, cx| state.set_text(without_middle_row, cx));
                cx.run_until_parked();
                assert_eq!(painted(&state, TextLeafKey::table_cell(0, 0), cx), [0..1]);
                for cell in 1..4 {
                    assert!(
                        painted(&state, TextLeafKey::table_cell(0, cell), cx).is_empty(),
                        "cell {cell}"
                    );
                }
            }
        }

        #[gpui::test]
        fn streaming_table_rows_keeps_the_highlights_of_earlier_rows(cx: &mut TestAppContext) {
            let state = state("| a | b |\n|---|---|\n| x | 1 |", cx);
            // "a b\nx 1\n\n"
            set(&state, [0..1, 4..5], cx).unwrap();
            state.update(cx, |state, cx| {
                state.set_text("| a | b |\n|---|---|\n| x | 1 |\n| y | 2 |", cx)
            });
            cx.run_until_parked();
            assert_eq!(painted(&state, TextLeafKey::table_cell(0, 0), cx), [0..1]);
            assert_eq!(painted(&state, TextLeafKey::table_cell(0, 2), cx), [0..1]);
        }

        #[gpui::test]
        fn an_edit_in_an_earlier_block_keeps_later_highlights(cx: &mut TestAppContext) {
            let state = state("one\n\ntwo", cx);
            set(&state, [0..3, 4..7], cx).unwrap();
            state.update(cx, |state, cx| state.set_text("one!\n\ntwo", cx));
            cx.run_until_parked();
            assert_eq!(painted(&state, TextLeafKey::block(0), cx), [0..3]);
            assert_eq!(painted(&state, TextLeafKey::block(6), cx), [0..3]);
        }

        #[gpui::test]
        fn an_append_after_a_full_update_compares_every_block(cx: &mut TestAppContext) {
            let state = state("first\n\nsecond", cx);
            set(&state, [0..5], cx).unwrap();
            // An append result whose baseline was a full update that has not
            // been committed: its earlier blocks are not the current ones.
            let options = UpdateOptions {
                revision: 0,
                pending_text: "FIRST\n\nsecond".to_string(),
                append: false,
                mode: ParseMode::Replace,
                markdown_extensions: Arc::default(),
            };
            let new = parse_content(TextViewFormat::Markdown, ParsedContent::default(), &options)
                .unwrap()
                .document;
            state.update(cx, |state, _| {
                state.full_update_revision = state.committed_revision + 1;
                let revision = state.committed_revision + 2;
                state.reconcile_range_highlights(&new, revision, true);
            });
            assert!(!has_highlights(&state, cx));
        }

        #[gpui::test]
        fn streaming_through_set_text_keeps_highlights(cx: &mut TestAppContext) {
            let state = state("first\n\nsec", cx);
            // "first\nsec\n"
            set(&state, [0..5, 6..9], cx).unwrap();
            state.update(cx, |state, cx| state.set_text("first\n\nsecond", cx));
            cx.run_until_parked();
            assert_eq!(painted(&state, TextLeafKey::block(0), cx), [0..5]);
            assert_eq!(painted(&state, TextLeafKey::block(7), cx), [0..3]);
        }

        #[gpui::test]
        fn a_full_parse_merged_with_an_append_compares_every_block(cx: &mut TestAppContext) {
            let state = state("", cx);
            // The small text is committed at once, and its background parse
            // merges with the append into one full parse, in which the
            // definition turns the earlier `[foo]` into the link text `foo`.
            state.update(cx, |state, cx| {
                state.set_text("[foo] and some text\n\nmore", cx);
                let text = state.rendered_text();
                let some = text.as_str().find("some").unwrap();
                state
                    .set_range_highlights([highlight(some..some + 4)], cx)
                    .unwrap();
                state.push_str("\n\n[foo]: https://example.com", cx);
            });
            cx.run_until_parked();
            state.read_with(cx, |state, _| {
                assert!(state.rendered_text().as_str().starts_with("foo and some"));
            });
            assert!(painted(&state, TextLeafKey::block(0), cx).is_empty());
        }

        #[gpui::test]
        fn a_full_update_before_an_append_drops_replaced_highlights(cx: &mut TestAppContext) {
            let state = state("first", cx);
            set(&state, [0..5], cx).unwrap();
            let large = "x".repeat(MAX_SYNC_FULL_REPLACE_BYTES + 1);
            state.update(cx, |state, cx| {
                state.set_text(&large, cx);
                state.push_str(" tail", cx);
            });
            cx.run_until_parked();
            assert!(!has_highlights(&state, cx));
        }

        #[gpui::test]
        fn reparsing_unchanged_text_keeps_highlights(cx: &mut TestAppContext) {
            let state = state("first", cx);
            set(&state, [0..5], cx).unwrap();
            state.update(cx, |state, cx| {
                state.set_markdown_extensions(
                    Arc::new(MarkdownExtensions::default().parser_revision(1)),
                    cx,
                )
            });
            cx.run_until_parked();
            assert_eq!(painted(&state, TextLeafKey::block(0), cx), [0..5]);
        }

        struct Root {
            state: Entity<TextViewState>,
        }

        impl Render for Root {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                gpui::div()
                    .w(px(120.))
                    .child(crate::text::TextView::new(&self.state).selectable(true))
            }
        }

        #[gpui::test]
        fn highlights_paint_across_inline_code_tables_and_code_blocks(cx: &mut TestAppContext) {
            cx.update(crate::init);
            let markdown = "wrapping text with `inline code` and a [link](https://x.y) \
                            that wraps\n\n| a | b |\n|---|---|\n| c | d |\n\n```\nlet x = 1;\n```";
            let (root, cx) = cx.add_window_view(|_, cx| Root {
                state: cx.new(|cx| TextViewState::markdown(markdown, cx)),
            });
            cx.run_until_parked();
            let state = root.read_with(cx, |root, _| root.state.clone());
            let text = state.read_with(cx, |state, _| state.rendered_text().as_str().to_string());
            let code = text.find("code").unwrap();
            let len = text.len();
            // Part of the inline code, every character, and the whole text.
            let ranges = [code..code + 2]
                .into_iter()
                .chain(text.char_indices().map(|(ix, c)| ix..ix + c.len_utf8()))
                .chain([0..len])
                .filter(|range| !text[range.clone()].trim().is_empty());
            state.update(cx, |state, cx| {
                state
                    .set_range_highlights(ranges.map(highlight), cx)
                    .unwrap();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            assert_eq!(
                painted(&state, TextLeafKey::block(0), cx)[0],
                code..code + 2
            );
        }

        #[gpui::test]
        fn clear_range_highlights_removes_them(cx: &mut TestAppContext) {
            let state = state("first", cx);
            set(&state, [0..5], cx).unwrap();
            state.update(cx, |state, cx| state.clear_range_highlights(cx));
            assert!(!has_highlights(&state, cx));
        }
    }
}
