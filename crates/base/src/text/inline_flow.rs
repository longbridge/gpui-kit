use std::{
    ops::Range,
    sync::{Arc, Mutex},
};
use unicode_segmentation::UnicodeSegmentation as _;

use gpui::{
    AbsoluteLength, AnyElement, App, AvailableSpace, Bounds, DefiniteLength, Element, ElementId,
    GlobalElementId, InspectorElementId, InteractiveElement as _, IntoElement, LayoutId,
    LineFragment as WrapLineFragment, ObjectFit, ParentElement as _, Pixels, ShapedLine,
    SharedString, SharedUri, Size, StatefulInteractiveElement as _, Styled, StyledImage as _,
    TextRun, TextStyle, WhiteSpace, Window, div, img, point, prelude::FluentBuilder as _, px,
    relative, size,
};

use crate::text::text_view::{LinkClickHandlerFn, handle_link_click};

use super::{
    inline::{Inline, InlineHighlight, InlineState, text_runs, text_size_ranges},
    node::LinkMark,
    utils::image_source,
};

const IMAGE_LEN: usize = 1;
pub(super) const INLINE_CODE_PADDING: f32 = 2.;

pub(super) struct InlineFlow {
    id: ElementId,
    items: Vec<InlineFlowItem>,
    link_click_handler: Option<Arc<LinkClickHandlerFn>>,
}

pub(super) enum InlineFlowItem {
    Text {
        state: Arc<Mutex<InlineState>>,
        text: SharedString,
        links: Vec<(Range<usize>, LinkMark)>,
        highlights: Vec<(Range<usize>, InlineHighlight)>,
    },
    Image {
        url: SharedUri,
        link: Option<LinkMark>,
        title: String,
        width: Option<DefiniteLength>,
        height: Option<DefiniteLength>,
    },
}

#[derive(Default)]
pub(crate) struct InlineFlowLayoutState {
    layout: Arc<Mutex<Option<InlineFlowLayout>>>,
}

#[derive(Default)]
struct InlineFlowLayout {
    fragments: Vec<PositionedFragment>,
    size: Size<Pixels>,
}

#[derive(Clone)]
enum PositionedFragment {
    Text {
        item_ix: usize,
        origin: gpui::Point<Pixels>,
        size: Size<Pixels>,
        source_range: Range<usize>,
        font_size: Pixels,
        text: SharedString,
        links: Vec<(Range<usize>, LinkMark)>,
        highlights: Vec<(Range<usize>, InlineHighlight)>,
    },
    Image {
        item_ix: usize,
        origin: gpui::Point<Pixels>,
        size: Size<Pixels>,
    },
}

enum MeasureItem {
    Text {
        text: SharedString,
        links: Vec<(Range<usize>, LinkMark)>,
        highlights: Vec<(Range<usize>, InlineHighlight)>,
    },
    Image {
        url: SharedUri,
        width: Option<DefiniteLength>,
        height: Option<DefiniteLength>,
    },
}

struct LineFragmentLayout {
    item_ix: usize,
    kind: LineFragmentKind,
    size: Size<Pixels>,
    source_range: Range<usize>,
    baseline_adjustment: Pixels,
}

enum LineFragmentKind {
    Text {
        font_size: Pixels,
        text: SharedString,
        links: Vec<(Range<usize>, LinkMark)>,
        highlights: Vec<(Range<usize>, InlineHighlight)>,
    },
    Image,
}

impl InlineFlow {
    pub(super) fn new(
        id: impl Into<ElementId>,
        items: Vec<InlineFlowItem>,
        link_click_handler: Option<Arc<LinkClickHandlerFn>>,
    ) -> Self {
        Self {
            id: id.into(),
            items,
            link_click_handler,
        }
    }

    fn image_element(
        ix: usize,
        url: &SharedUri,
        link: &Option<LinkMark>,
        _title: &str,
        size: Size<Pixels>,
        link_click_handler: Option<Arc<LinkClickHandlerFn>>,
    ) -> AnyElement {
        img(image_source(url))
            .id(ix)
            .object_fit(ObjectFit::Contain)
            .max_w(relative(1.))
            .w(size.width)
            .h(size.height)
            .when_some(link.clone(), |this, link| {
                let aux_link = link.clone();
                let aux_link_click_handler = link_click_handler.clone();
                this.cursor_pointer()
                    .on_click(move |event, window, cx| {
                        crate::TextSelection::end(window, cx);
                        cx.stop_propagation();
                        handle_link_click(
                            &link_click_handler,
                            link.url.clone(),
                            event.clone(),
                            window,
                            cx,
                        );
                    })
                    .on_aux_click(move |event, window, cx| {
                        crate::TextSelection::end(window, cx);
                        cx.stop_propagation();
                        handle_link_click(
                            &aux_link_click_handler,
                            aux_link.url.clone(),
                            event.clone(),
                            window,
                            cx,
                        );
                    })
            })
            .into_any_element()
    }
}

impl IntoElement for InlineFlow {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for InlineFlow {
    type RequestLayoutState = InlineFlowLayoutState;
    type PrepaintState = Vec<(AnyElement, Option<(Bounds<Pixels>, gpui::Hsla)>)>;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let measure_items = self.items.iter().map(MeasureItem::from).collect::<Vec<_>>();
        let line_height = window.line_height();
        let rem_size = window.rem_size();
        let image_sizes = measure_items
            .iter()
            .enumerate()
            .map(|(ix, item)| match item {
                MeasureItem::Image { url, width, height } => Some(measure_image_size(
                    ix,
                    url,
                    *width,
                    *height,
                    line_height,
                    rem_size,
                    window,
                    cx,
                )),
                MeasureItem::Text { .. } => None,
            })
            .collect::<Vec<_>>();
        let layout_state = InlineFlowLayoutState::default();
        let layout_ref = layout_state.layout.clone();

        let layout_id = window.request_measured_layout(Default::default(), {
            move |known_dimensions, available_space, window, _cx| {
                let text_style = window.text_style();
                let wrap_width = if text_style.white_space == WhiteSpace::Normal {
                    known_dimensions.width.or(match available_space.width {
                        AvailableSpace::Definite(width) => Some(width),
                        _ => None,
                    })
                } else {
                    None
                };
                let layout = layout_flow(
                    &measure_items,
                    &image_sizes,
                    &text_style,
                    wrap_width,
                    window,
                );
                let size = layout.size;
                if let Ok(mut state) = layout_ref.lock() {
                    *state = Some(layout);
                }
                size
            }
        });

        (layout_id, layout_state)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let fragments = request_layout
            .layout
            .lock()
            .ok()
            .and_then(|layout| layout.as_ref().map(|layout| layout.fragments.clone()))
            .unwrap_or_default();
        let mut elements = Vec::with_capacity(fragments.len());

        for fragment in fragments {
            match fragment {
                PositionedFragment::Text {
                    item_ix,
                    origin,
                    size: fragment_size,
                    source_range,
                    font_size,
                    text,
                    links,
                    mut highlights,
                    ..
                } => {
                    let InlineFlowItem::Text {
                        state: source_state,
                        ..
                    } = &self.items[item_ix]
                    else {
                        continue;
                    };
                    let state = Arc::new(Mutex::new(InlineState::default()));
                    if let Ok(mut state) = state.lock() {
                        state.set_text(text.clone());
                    }

                    let is_code = highlights.iter().any(|(_, h)| h.font_size_scale.is_some());
                    let padding = if is_code {
                        px(INLINE_CODE_PADDING)
                    } else {
                        Pixels::ZERO
                    };
                    let background = if is_code {
                        let style = window.text_style();
                        let runs = text_runs(text.len(), &style, &highlights);
                        let line = shape_line(text.clone(), font_size, &runs, window);
                        let baseline =
                            (fragment_size.height - line.ascent - line.descent) / 2. + line.ascent;
                        let cap_height = runs
                            .iter()
                            .map(|run| {
                                let font = window.text_system().resolve_font(&run.font);
                                window.text_system().cap_height(font, font_size)
                            })
                            .fold(Pixels::ZERO, Pixels::max);
                        // Center the background on the capital-height body of the text.
                        // Share descender room between both sides instead of adding it only below.
                        let vertical_padding = font_size * 0.125 + line.descent / 2.;
                        let color = highlights
                            .iter()
                            .find_map(|(_, h)| h.style.background_color);
                        for (_, highlight) in &mut highlights {
                            highlight.style.background_color = None;
                        }
                        color.map(|color| {
                            (
                                Bounds::new(
                                    bounds.origin
                                        + origin
                                        + point(
                                            Pixels::ZERO,
                                            baseline - cap_height - vertical_padding,
                                        ),
                                    size(fragment_size.width, cap_height + vertical_padding * 2.),
                                ),
                                color,
                            )
                        })
                    } else {
                        None
                    };
                    let inline = Inline::new(
                        elements.len(),
                        state,
                        links,
                        highlights,
                        self.link_click_handler.clone(),
                    )
                    .selection_source(source_state.clone(), source_range)
                    .paint_origin(bounds.origin + origin + point(padding, Pixels::ZERO));
                    let mut element = div()
                        .text_size(font_size)
                        .line_height(fragment_size.height)
                        .child(inline)
                        .into_any_element();
                    element.prepaint_as_root(
                        bounds.origin + origin + point(padding, Pixels::ZERO),
                        size(
                            AvailableSpace::Definite(fragment_size.width - padding * 2.),
                            AvailableSpace::Definite(fragment_size.height),
                        ),
                        window,
                        cx,
                    );
                    elements.push((element, background));
                }
                PositionedFragment::Image {
                    item_ix,
                    origin,
                    size: fragment_size,
                } => {
                    let InlineFlowItem::Image {
                        url, link, title, ..
                    } = &self.items[item_ix]
                    else {
                        continue;
                    };
                    let mut element = Self::image_element(
                        elements.len(),
                        url,
                        link,
                        title.as_str(),
                        fragment_size,
                        self.link_click_handler.clone(),
                    );
                    element.prepaint_as_root(
                        bounds.origin + origin,
                        size(
                            AvailableSpace::Definite(fragment_size.width),
                            AvailableSpace::Definite(fragment_size.height),
                        ),
                        window,
                        cx,
                    );
                    elements.push((element, None));
                }
            }
        }

        elements
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        for item in &self.items {
            if let InlineFlowItem::Text { state, .. } = item
                && let Ok(mut state) = state.lock()
            {
                state.selection = None;
            }
        }
        let radius = crate::Theme::global(cx).tokens.radius.sm;
        for (element, background) in prepaint {
            if let Some((bounds, color)) = background {
                window.paint_quad(gpui::fill(*bounds, *color).corner_radii(radius));
            }
            element.paint(window, cx);
        }
    }
}

impl From<&InlineFlowItem> for MeasureItem {
    fn from(item: &InlineFlowItem) -> Self {
        match item {
            InlineFlowItem::Text {
                state: _,
                text,
                links,
                highlights,
                ..
            } => MeasureItem::Text {
                text: text.clone(),
                links: links.clone(),
                highlights: highlights.clone(),
            },
            InlineFlowItem::Image {
                url, width, height, ..
            } => MeasureItem::Image {
                url: url.clone(),
                width: *width,
                height: *height,
            },
        }
    }
}

impl MeasureItem {
    fn len(&self) -> usize {
        match self {
            MeasureItem::Text { text, .. } => text.len(),
            MeasureItem::Image { .. } => IMAGE_LEN,
        }
    }
}

fn layout_flow(
    items: &[MeasureItem],
    image_sizes: &[Option<Size<Pixels>>],
    text_style: &TextStyle,
    wrap_width: Option<Pixels>,
    window: &mut Window,
) -> InlineFlowLayout {
    let line_height = window.pixel_snap(window.line_height());
    let rem_size = window.rem_size();
    let total_len = items.iter().map(MeasureItem::len).sum::<usize>();
    if total_len == 0 {
        return InlineFlowLayout::default();
    }

    let line_ranges = line_ranges(items, image_sizes, text_style, wrap_width, window);
    let font_size = text_style.font_size.to_pixels(rem_size);
    let mut fragments = Vec::new();
    let mut max_width = Pixels::ZERO;
    let mut y = Pixels::ZERO;

    for line_range in line_ranges {
        let mut line_fragments = Vec::new();
        let mut line_width = Pixels::ZERO;
        let mut actual_line_height = line_height;
        let mut item_start = 0;

        for (item_ix, item) in items.iter().enumerate() {
            let item_end = item_start + item.len();
            if item_end <= line_range.start {
                item_start = item_end;
                continue;
            }
            if item_start >= line_range.end {
                break;
            }

            match item {
                MeasureItem::Text {
                    text,
                    links,
                    highlights,
                } => {
                    let local_start = line_range.start.max(item_start) - item_start;
                    let local_end = line_range.end.min(item_end) - item_start;
                    for (segment, scale) in text_size_ranges(text.len(), highlights) {
                        let start = local_start.max(segment.start);
                        let end = local_end.min(segment.end);
                        if start >= end {
                            continue;
                        }
                        let subtext = SharedString::from(text[start..end].to_string());
                        let highlights = slice_ranges(highlights, start, end, |range, style| {
                            (range, style.clone())
                        });
                        let links =
                            slice_ranges(links, start, end, |range, link| (range, link.clone()));
                        let runs = text_runs(subtext.len(), text_style, &highlights);
                        let segment_font_size = font_size * scale;
                        let shaped_line =
                            shape_line(subtext.clone(), segment_font_size, &runs, window);
                        let is_code = highlights.iter().any(|(_, h)| h.font_size_scale.is_some());
                        let padding = if is_code {
                            px(INLINE_CODE_PADDING * 2.)
                        } else {
                            Pixels::ZERO
                        };
                        let width = shaped_line.width() + padding;
                        // Keep the glyph paint layer large enough for ascenders and descenders.
                        // The compact code background is painted independently.
                        let segment_line_height = window
                            .pixel_snap(line_height.max(shaped_line.ascent + shaped_line.descent));
                        let baseline =
                            (segment_line_height - shaped_line.ascent - shaped_line.descent) / 2.
                                + shaped_line.ascent;
                        actual_line_height = actual_line_height.max(segment_line_height);
                        let body_font = window.text_system().resolve_font(&text_style.font());
                        let body_baseline =
                            window
                                .text_system()
                                .baseline_offset(body_font, font_size, line_height);
                        line_width += width;
                        line_fragments.push(LineFragmentLayout {
                            item_ix,
                            kind: LineFragmentKind::Text {
                                font_size: segment_font_size,
                                text: subtext,
                                links,
                                highlights,
                            },
                            size: size(width, segment_line_height),
                            source_range: start..end,
                            baseline_adjustment: body_baseline
                                - baseline
                                - (line_height - segment_line_height) / 2.,
                        });
                    }
                }
                MeasureItem::Image { .. } => {
                    if line_range.start <= item_start && item_end <= line_range.end {
                        let size = image_sizes[item_ix]
                            .expect("image size should be measured before layout");
                        line_width += size.width;
                        actual_line_height = actual_line_height.max(size.height);
                        line_fragments.push(LineFragmentLayout {
                            item_ix,
                            kind: LineFragmentKind::Image,
                            size,
                            source_range: 0..IMAGE_LEN,
                            baseline_adjustment: Pixels::ZERO,
                        });
                    }
                }
            }

            item_start = item_end;
        }

        let mut x = Pixels::ZERO;
        for fragment in line_fragments {
            let origin = point(
                x,
                y + (actual_line_height - fragment.size.height) / 2. + fragment.baseline_adjustment,
            );
            let positioned = match fragment.kind {
                LineFragmentKind::Text {
                    font_size,
                    text,
                    links,
                    highlights,
                } => PositionedFragment::Text {
                    item_ix: fragment.item_ix,
                    origin,
                    size: fragment.size,
                    source_range: fragment.source_range,
                    font_size,
                    text,
                    links,
                    highlights,
                },
                LineFragmentKind::Image => PositionedFragment::Image {
                    item_ix: fragment.item_ix,
                    origin,
                    size: fragment.size,
                },
            };
            x += fragment.size.width;
            fragments.push(positioned);
        }

        max_width = max_width.max(line_width);
        y += actual_line_height;
    }

    InlineFlowLayout {
        fragments,
        size: size(max_width, y),
    }
}

fn line_ranges(
    items: &[MeasureItem],
    image_sizes: &[Option<Size<Pixels>>],
    text_style: &TextStyle,
    wrap_width: Option<Pixels>,
    window: &mut Window,
) -> Vec<Range<usize>> {
    let total_len = items.iter().map(MeasureItem::len).sum::<usize>();
    let mut hard_lines = Vec::new();
    let mut line_start = 0;
    let mut item_start = 0;

    for item in items {
        if let MeasureItem::Text { text, .. } = item {
            for (newline, _) in text.match_indices('\n') {
                let newline = item_start + newline;
                hard_lines.push(line_start..newline);
                line_start = newline + 1;
            }
        }
        item_start += item.len();
    }
    hard_lines.push(line_start..total_len);

    let Some(wrap_width) = wrap_width else {
        return hard_lines;
    };
    let rem_size = window.rem_size();
    let font_size = text_style.font_size.to_pixels(rem_size);
    let mut wrapper = window
        .text_system()
        .line_wrapper(text_style.font(), font_size);
    let mut ranges = Vec::new();

    for hard_line in hard_lines {
        let mut item_start = 0;
        let mut wrap_fragments = Vec::new();
        for (ix, item) in items.iter().enumerate() {
            let item_end = item_start + item.len();
            if item_end > hard_line.start && item_start < hard_line.end {
                match item {
                    MeasureItem::Text {
                        text, highlights, ..
                    } => {
                        let start = hard_line.start.max(item_start) - item_start;
                        let end = hard_line.end.min(item_end) - item_start;
                        if start < end {
                            push_text_wrap_fragments(
                                &mut wrap_fragments,
                                text,
                                highlights,
                                start..end,
                                text_style,
                                wrap_width,
                                window,
                            );
                        }
                    }
                    MeasureItem::Image { .. } => {
                        if hard_line.start <= item_start && item_end <= hard_line.end {
                            wrap_fragments.push(WrapLineFragment::element(
                                image_sizes[ix]
                                    .expect("image size should be measured before wrapping")
                                    .width,
                                IMAGE_LEN,
                            ));
                        }
                    }
                }
            }
            item_start = item_end;
        }

        let boundaries = wrapper
            .wrap_line(&wrap_fragments, wrap_width)
            .map(|boundary| hard_line.start + boundary.ix.min(hard_line.len()))
            .collect::<Vec<_>>();
        let mut start = hard_line.start;

        for end in boundaries {
            if start < end {
                ranges.push(start..end);
            }
            start = end;
        }

        if start < hard_line.end || hard_line.is_empty() {
            ranges.push(start..hard_line.end);
        }
    }

    ranges
}

/// Appends the wrap fragments for `range` of `text`. The line wrapper
/// measures text fragments in the body font, so a span whose highlight sets
/// another family is shaped with the same run the renderer uses and enters
/// the wrapper as measured elements. Oversized spans retain word boundaries;
/// oversized words can break at grapheme boundaries without splitting Unicode.
fn push_text_wrap_fragments<'a>(
    fragments: &mut Vec<WrapLineFragment<'a>>,
    text: &'a str,
    highlights: &[(Range<usize>, InlineHighlight)],
    range: Range<usize>,
    text_style: &TextStyle,
    wrap_width: Pixels,
    window: &mut Window,
) {
    let font_size = text_style.font_size.to_pixels(window.rem_size());
    let mut cursor = range.start;
    for (highlight_range, highlight) in highlights {
        if highlight.font_family.is_none() {
            continue;
        }
        let start = highlight_range.start.max(cursor);
        let end = highlight_range.end.min(range.end);
        if start >= end {
            continue;
        }
        if cursor < start {
            fragments.push(WrapLineFragment::text(&text[cursor..start]));
        }
        let span = &text[start..end];
        let measure = |text: &str| {
            let runs = text_runs(
                text.len(),
                text_style,
                &[(0..text.len(), highlight.clone())],
            );
            window
                .text_system()
                .layout_line(
                    text,
                    font_size * highlight.font_size_scale.unwrap_or(1.),
                    &runs,
                    None,
                )
                .width
        };
        let padding = if highlight.font_size_scale.is_some() {
            px(INLINE_CODE_PADDING * 2.)
        } else {
            Pixels::ZERO
        };
        let width = measure(span) + padding;
        if width <= wrap_width {
            fragments.push(WrapLineFragment::element(width, span.len()));
        } else {
            for word in span.split_word_bounds() {
                let width = measure(word) + padding;
                if width <= wrap_width {
                    fragments.push(WrapLineFragment::element(width, word.len()));
                } else {
                    for grapheme in word.graphemes(true) {
                        fragments.push(WrapLineFragment::element(
                            measure(grapheme) + padding,
                            grapheme.len(),
                        ));
                    }
                }
            }
        }
        cursor = end;
    }
    if cursor < range.end {
        fragments.push(WrapLineFragment::text(&text[cursor..range.end]));
    }
}

#[allow(clippy::too_many_arguments)]
fn measure_image_size(
    ix: usize,
    url: &SharedUri,
    width: Option<DefiniteLength>,
    height: Option<DefiniteLength>,
    line_height: Pixels,
    rem_size: Pixels,
    window: &mut Window,
    cx: &mut App,
) -> Size<Pixels> {
    let intrinsic_size = if width.is_some() && height.is_some() {
        None
    } else {
        intrinsic_image_size(ix, url, width, height, window, cx)
    };
    image_size(width, height, intrinsic_size, line_height, rem_size)
}

fn intrinsic_image_size(
    ix: usize,
    url: &SharedUri,
    width: Option<DefiniteLength>,
    height: Option<DefiniteLength>,
    window: &mut Window,
    cx: &mut App,
) -> Option<Size<Pixels>> {
    let mut element = img(image_source(url))
        .id(ix)
        .object_fit(ObjectFit::Contain)
        .max_w(relative(1.))
        .when_some(width, |this, width| this.w(width))
        .when_some(height, |this, height| this.h(height))
        .into_any_element();
    let measured_size = element.layout_as_root(AvailableSpace::min_size(), window, cx);

    if measured_size.width <= Pixels::ZERO || measured_size.height <= Pixels::ZERO {
        None
    } else {
        Some(measured_size)
    }
}

fn image_size(
    width: Option<DefiniteLength>,
    height: Option<DefiniteLength>,
    intrinsic_size: Option<Size<Pixels>>,
    line_height: Pixels,
    rem_size: Pixels,
) -> Size<Pixels> {
    let base_size = AbsoluteLength::Pixels(line_height);
    match (width, height) {
        (Some(width), Some(height)) => size(
            width.to_pixels(base_size, rem_size),
            height.to_pixels(base_size, rem_size),
        ),
        (Some(width), None) => {
            let width = width.to_pixels(base_size, rem_size);
            let height = intrinsic_size
                .and_then(|intrinsic_size| {
                    (intrinsic_size.width > Pixels::ZERO && intrinsic_size.height > Pixels::ZERO)
                        .then(|| width * (intrinsic_size.height / intrinsic_size.width))
                })
                .unwrap_or(line_height);
            size(width, height)
        }
        (None, Some(height)) => {
            let height = height.to_pixels(base_size, rem_size);
            let width = intrinsic_size
                .and_then(|intrinsic_size| {
                    (intrinsic_size.width > Pixels::ZERO && intrinsic_size.height > Pixels::ZERO)
                        .then(|| height * (intrinsic_size.width / intrinsic_size.height))
                })
                .unwrap_or(height);
            size(width, height)
        }
        (None, None) => inline_image_size_for_line(intrinsic_size, line_height),
    }
}

fn inline_image_size_for_line(
    intrinsic_size: Option<Size<Pixels>>,
    line_height: Pixels,
) -> Size<Pixels> {
    let height = line_height * 0.75;
    let aspect_ratio = intrinsic_size
        .and_then(|intrinsic_size| {
            (intrinsic_size.width > Pixels::ZERO && intrinsic_size.height > Pixels::ZERO)
                .then(|| intrinsic_size.width / intrinsic_size.height)
        })
        .unwrap_or(1.);

    size((height * aspect_ratio).max(px(1.)), height.max(px(1.)))
}

fn shape_line(
    text: SharedString,
    font_size: Pixels,
    runs: &[TextRun],
    window: &mut Window,
) -> ShapedLine {
    window.text_system().shape_line(text, font_size, runs, None)
}

pub(super) fn slice_ranges<T, U>(
    ranges: &[(Range<usize>, T)],
    start: usize,
    end: usize,
    map: impl Fn(Range<usize>, &T) -> U,
) -> Vec<U> {
    ranges
        .iter()
        .filter_map(|(range, value)| {
            let clipped_start = range.start.max(start);
            let clipped_end = range.end.min(end);
            (clipped_start < clipped_end)
                .then(|| map((clipped_start - start)..(clipped_end - start), value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_image_without_explicit_size_scales_intrinsic_ratio_to_line_height() {
        let line_height = px(20.);
        let intrinsic_size = size(px(160.), px(40.));

        let measured = inline_image_size_for_line(Some(intrinsic_size), line_height);

        assert_eq!(measured, size(px(60.), px(15.)));
    }

    #[test]
    fn inline_image_without_intrinsic_size_uses_compact_square_fallback() {
        let measured = inline_image_size_for_line(None, px(20.));

        assert_eq!(measured, size(px(15.), px(15.)));
    }

    /// Line breaking must see the width of an inline code span in its own
    /// family. With a body-font-only wrapper the span below is measured at
    /// half its shaped width, the line is kept whole, and the flow reports a
    /// width past `wrap_width`.
    #[test]
    fn inline_code_near_the_wrap_width_does_not_overflow_the_flow() {
        use super::super::inline::test_fonts::{BODY, MONO, WideMonoTextSystem};
        use gpui::{AbsoluteLength, Empty, HighlightStyle, TestApp};

        let mut app = TestApp::with_text_system(Arc::new(WideMonoTextSystem));
        let mut window = app.open_window(|_, _| Empty);

        let font_size = px(10.);
        let text_style = TextStyle {
            font_family: SharedString::from(BODY),
            font_size: AbsoluteLength::Pixels(font_size),
            ..Default::default()
        };
        let lead = SharedString::from("See ");
        let tail_text = " with code_span_here end";
        let code = tail_text.find("code_span_here").unwrap();
        let code_range = code..code + "code_span_here".len();
        let code_highlight = InlineHighlight {
            style: HighlightStyle::default(),
            font_family: Some(SharedString::from(MONO)),
            font_size_scale: None,
        };
        let items = vec![
            MeasureItem::Text {
                text: lead.clone(),
                links: vec![],
                highlights: vec![],
            },
            MeasureItem::Image {
                url: SharedUri::from("https://example.com/badge.png"),
                width: None,
                height: None,
            },
            MeasureItem::Text {
                text: SharedString::from(tail_text),
                links: vec![],
                highlights: vec![(code_range.clone(), code_highlight)],
            },
        ];
        let image_size = size(px(10.), px(10.));
        let image_sizes = vec![None, Some(image_size), None];
        // Body text, the image and the mono span fill the wrap width exactly;
        // the trailing "end" only fits if the span is under-measured.
        let wrap_width = WideMonoTextSystem::width_of("See  with  ", BODY, font_size)
            + image_size.width
            + WideMonoTextSystem::width_of("code_span_here", MONO, font_size);

        let layout = window.update(|_, window, _| {
            layout_flow(&items, &image_sizes, &text_style, Some(wrap_width), window)
        });

        assert!(
            layout.size.width <= wrap_width,
            "flow width {:?} exceeds wrap width {:?}",
            layout.size.width,
            wrap_width
        );
        let mono_fragment_width = layout
            .fragments
            .iter()
            .find_map(|fragment| match fragment {
                PositionedFragment::Text { text, size, .. } if text.contains("code_span_here") => {
                    Some(size.width)
                }
                _ => None,
            })
            .expect("the code span is laid out as a text fragment");
        assert!(
            mono_fragment_width >= WideMonoTextSystem::width_of("code_span_here", MONO, font_size),
            "the code span fragment is shaped in the mono family"
        );
        // The image sits vertically centred in its line, so line membership is
        // read off the text fragments only.
        let text_lines = layout
            .fragments
            .iter()
            .filter_map(|fragment| match fragment {
                PositionedFragment::Text { text, origin, .. } => Some((text.trim(), origin.y)),
                PositionedFragment::Image { .. } => None,
            })
            .collect::<Vec<_>>();
        let first_y = text_lines[0].1;
        assert!(
            text_lines
                .iter()
                .any(|(text, y)| text.contains("code_span_here") && *y == first_y),
            "the span stays on the first line: {text_lines:?}"
        );
        assert!(
            text_lines
                .iter()
                .any(|(text, y)| *text == "end" && *y > first_y),
            "the trailing word wraps to a second line: {text_lines:?}"
        );
    }
    #[test]
    fn long_inline_code_wraps_in_mixed_flow() {
        use super::super::inline::test_fonts::{BODY, MONO, WideMonoTextSystem};
        use gpui::{Empty, TestApp};
        let mut app = TestApp::with_text_system(Arc::new(WideMonoTextSystem));
        let mut window = app.open_window(|_, _| Empty);
        let style = TextStyle {
            font_family: BODY.into(),
            font_size: AbsoluteLength::Pixels(px(10.)),
            ..Default::default()
        };
        for text in [
            "one two three four five six",
            "very_long_unbroken_identifier",
            "你好世界你好世界你好世界",
            "e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}",
        ] {
            let items = vec![
                MeasureItem::Image {
                    url: "https://example.com/icon.png".into(),
                    width: None,
                    height: None,
                },
                MeasureItem::Text {
                    text: text.into(),
                    links: vec![],
                    highlights: vec![(
                        0..text.len(),
                        InlineHighlight {
                            font_family: Some(MONO.into()),
                            ..Default::default()
                        },
                    )],
                },
            ];
            let image_sizes = vec![Some(size(px(10.), px(10.))), None];
            let layout = window.update(|_, window, _| {
                layout_flow(&items, &image_sizes, &style, Some(px(100.)), window)
            });
            assert!(layout.size.width <= px(100.), "{text:?}: {:?}", layout.size);
            let reconstructed: String = layout
                .fragments
                .iter()
                .filter_map(|fragment| match fragment {
                    PositionedFragment::Text { text, .. } => Some(text.as_ref()),
                    _ => None,
                })
                .collect();
            assert_eq!(reconstructed, text);
        }
    }
    #[test]
    fn inline_code_size_is_relative_and_shares_the_body_baseline() {
        use super::super::inline::test_fonts::{BODY, MONO, WideMonoTextSystem};
        use gpui::{Empty, TestApp};
        let mut app = TestApp::with_text_system(Arc::new(WideMonoTextSystem));
        let mut window = app.open_window(|_, _| Empty);
        for body_size in [16., 24.] {
            let style = TextStyle {
                font_family: BODY.into(),
                font_size: AbsoluteLength::Pixels(px(body_size)),
                ..Default::default()
            };
            let items = vec![MeasureItem::Text {
                text: "a code z".into(),
                links: vec![],
                highlights: vec![(
                    2..6,
                    InlineHighlight {
                        font_family: Some(MONO.into()),
                        font_size_scale: Some(0.875),
                        ..Default::default()
                    },
                )],
            }];
            window.update(|_, window, _| {
                let layout = layout_flow(&items, &[None], &style, None, window);
                let text_fragments = layout
                    .fragments
                    .iter()
                    .filter_map(|fragment| match fragment {
                        PositionedFragment::Text {
                            text,
                            font_size,
                            origin,
                            size,
                            ..
                        } => Some((text.as_ref(), *font_size, origin.y, *size)),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(text_fragments.len(), 3);
                assert_eq!(text_fragments[0].1, px(body_size));
                assert_eq!(text_fragments[1].0, "code");
                assert_eq!(text_fragments[1].1, px(body_size * 0.875));
                assert!(text_fragments[1].3.height >= text_fragments[0].3.height);
                assert_eq!(
                    text_fragments[1].3.width,
                    WideMonoTextSystem::width_of("code", MONO, px(body_size * 0.875))
                        + px(INLINE_CODE_PADDING * 2.)
                );
                let baseline = |family, fragment: &(&str, Pixels, Pixels, Size<Pixels>)| {
                    let font = window.text_system().resolve_font(&gpui::font(family));
                    fragment.2
                        + window
                            .text_system()
                            .baseline_offset(font, fragment.1, fragment.3.height)
                };
                assert!(
                    (baseline(BODY, &text_fragments[0]) - baseline(MONO, &text_fragments[1])).abs()
                        < px(0.01)
                );
            });
        }
    }
}
