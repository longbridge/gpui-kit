//! Readonly, virtualized patch display inspired by Diffs from Pierre.
//! Parse an externally supplied patch into [`DiffDocument`]s, retain one
//! [`DiffState`] for all of them in the owner, and build [`Diff`] during rendering.
mod document;
mod parser;
#[cfg(test)]
mod parser_tests;
mod selection;
mod state;

pub use document::{DiffDocument, DiffLinePosition, DiffLineRange, DiffSide};
pub use parser::DiffParseError;
pub use state::{DiffEvent, DiffState};

use std::{collections::HashMap, rc::Rc};

use gpui::{
    AnyElement, App, ClipboardItem, Context, ElementId, Entity, Focusable as _, HighlightStyle,
    InteractiveElement as _, IntoElement, KeyBinding, ListOffset, ParentElement as _, Pixels,
    RenderOnce, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, TextRun,
    Window, div, list, point, prelude::FluentBuilder as _, px,
};
use gpui_base::TestSupportExt as _;
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Copy, SelectAll},
    scroll::Scrollbar,
    v_flex,
};
use actions::*;
use selection::{CodeText, SelectionLayer};
use state::{DisplayRow, LayoutKey};

// Private: bindings serve the focused code body only.
mod actions {
    gpui::actions!(
        diff,
        [
            ScrollUp,
            ScrollDown,
            ScrollLeft,
            ScrollRight,
            PageUp,
            PageDown,
            First,
            Last,
            ExtendUp,
            ExtendDown
        ]
    );
}

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", ScrollUp, Some("Diff")),
        KeyBinding::new("down", ScrollDown, Some("Diff")),
        KeyBinding::new("left", ScrollLeft, Some("Diff")),
        KeyBinding::new("right", ScrollRight, Some("Diff")),
        KeyBinding::new("pageup", PageUp, Some("Diff")),
        KeyBinding::new("pagedown", PageDown, Some("Diff")),
        KeyBinding::new("home", First, Some("Diff")),
        KeyBinding::new("end", Last, Some("Diff")),
        KeyBinding::new("shift-up", ExtendUp, Some("Diff")),
        KeyBinding::new("shift-down", ExtendDown, Some("Diff")),
        KeyBinding::new("secondary-c", Copy, Some("Diff")),
        KeyBinding::new("secondary-a", SelectAll, Some("Diff")),
    ]);
}

/// Presentation of the same immutable file comparison.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiffMode {
    Split,
    #[default]
    Unified,
}

/// Application-owned content attached beneath a source line.
///
/// The stable ID belongs to the annotation itself, while the position uses
/// the file, side and original one-based source line, independent of folding.
#[derive(Clone)]
pub struct DiffLineAnnotation {
    id: ElementId,
    position: DiffLinePosition,
}
impl DiffLineAnnotation {
    pub fn new(id: impl Into<ElementId>, position: DiffLinePosition) -> Self {
        Self {
            id: id.into(),
            position,
        }
    }
    pub fn id(&self) -> &ElementId {
        &self.id
    }
    pub fn position(&self) -> DiffLinePosition {
        self.position
    }
}

type HeaderRenderer = Rc<dyn Fn(&DiffDocument, &mut Window, &mut App) -> AnyElement>;
type AnnotationRenderer = Rc<dyn Fn(&DiffLineAnnotation, &mut Window, &mut App) -> AnyElement>;

/// A themed readonly code-review surface with split and unified layouts.
///
/// All files of the supplied state share one virtualized list, each introduced
/// by its header. The component owns both scrolling axes, so give it a bounded
/// height. Long lines scroll rather than wrap; file headers and review
/// annotations are explicit slots, and application commands remain owned by
/// the application.
#[derive(IntoElement)]
pub struct Diff {
    state: Entity<DiffState>,
    style: StyleRefinement,
    line_numbers: bool,
    syntax_highlight: bool,
    header_visible: bool,
    annotations: Rc<Vec<DiffLineAnnotation>>,
    annotation_index: Rc<HashMap<DiffLinePosition, Vec<usize>>>,
    header_renderer: Option<HeaderRenderer>,
    annotation_renderer: Option<AnnotationRenderer>,
}

impl Diff {
    pub fn new(state: &Entity<DiffState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            line_numbers: true,
            syntax_highlight: true,
            header_visible: true,
            annotations: Rc::new(Vec::new()),
            annotation_index: Rc::new(HashMap::new()),
            header_renderer: None,
            annotation_renderer: None,
        }
    }
    /// Shows the original and modified line-number lanes. Default is true.
    pub fn line_numbers(mut self, value: bool) -> Self {
        self.line_numbers = value;
        self
    }
    /// Emphasizes syntax once the state has prepared it. Default is true.
    pub fn syntax_highlight(mut self, value: bool) -> Self {
        self.syntax_highlight = value;
        self
    }
    /// Shows a header above each file. Default is true.
    pub fn header_visible(mut self, visible: bool) -> Self {
        self.header_visible = visible;
        self
    }
    /// Supplies annotations with stable identities and source positions.
    ///
    /// Rows re-measure when annotations are added, removed or replaced, and
    /// whenever they are visible, so annotation content may change height freely.
    pub fn annotations(
        mut self,
        annotations: impl IntoIterator<Item = DiffLineAnnotation>,
    ) -> Self {
        let annotations = annotations.into_iter().collect::<Vec<_>>();
        let mut index: HashMap<DiffLinePosition, Vec<usize>> = HashMap::new();
        for (ix, annotation) in annotations.iter().enumerate() {
            index.entry(annotation.position).or_default().push(ix);
        }
        self.annotations = Rc::new(annotations);
        self.annotation_index = Rc::new(index);
        self
    }
    /// Replaces the content of each file header, keeping its shell and separator.
    pub fn header<F, E>(mut self, render: F) -> Self
    where
        F: Fn(&DiffDocument, &mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.header_renderer = Some(Rc::new(move |document, window, cx| {
            render(document, window, cx).into_any_element()
        }));
        self
    }
    /// Renders each supplied annotation. Application state and commands stay with the owner.
    pub fn annotation<F, E>(mut self, render: F) -> Self
    where
        F: Fn(&DiffLineAnnotation, &mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.annotation_renderer = Some(Rc::new(move |annotation, window, cx| {
            render(annotation, window, cx).into_any_element()
        }));
        self
    }
}
impl Styled for Diff {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Diff {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.clone();
        state.update(cx, |state, cx| render_diff(self, state, window, cx))
    }
}

fn render_diff(
    props: Diff,
    state: &mut DiffState,
    window: &mut Window,
    cx: &mut Context<DiffState>,
) -> AnyElement {
    state.set_window(window.window_handle().window_id());
    let rem = window.rem_size();
    let font_size = rem * (f32::from(cx.theme().mono_font_size) / 16.);
    let font_family = cx.theme().mono_font_family.clone();
    let row_height = font_size * 1.6;
    let digits = state
        .documents()
        .iter()
        .map(|document| document.line_number_digits())
        .max()
        .unwrap_or(1);
    let gutter_width = if props.line_numbers {
        font_size * 0.7 * digits as f32 + rem
    } else {
        px(0.)
    };
    let mode = state.mode();
    let layout = LayoutKey::new(
        rem,
        font_size,
        font_family.clone(),
        mode,
        props.line_numbers,
    );
    if state.update_layout(layout, row_height) {
        let width = measure_code_width(
            state.documents(),
            font_size,
            &font_family,
            gutter_width,
            mode,
            window,
            cx,
        );
        state.set_content_width(width);
    }
    state.sync_annotations(&props.annotations);
    let content_width = state.content_width().max(state.viewport().width);
    let column_width = if mode == DiffMode::Split {
        content_width / 2.
    } else {
        content_width
    };
    let code = Rc::new(CodePresentation {
        documents: state.documents().to_vec(),
        mode,
        column_width,
        font_size,
        row_height,
        gutter_width,
        line_numbers: props.line_numbers,
        syntax_highlight: props.syntax_highlight,
        header_visible: props.header_visible,
        header_renderer: props.header_renderer.clone(),
        annotations: props.annotations.clone(),
        annotation_index: props.annotation_index.clone(),
        annotation_renderer: props.annotation_renderer.clone(),
        selection: state.selection().clone(),
        selected_lines: state.selected_lines(),
        geometry: state.geometry().clone(),
        state: cx.entity(),
    });
    let rows = state.rows().clone();
    let scrollbar = state.list().clone();
    let horizontal = state.horizontal_scroll().clone();
    let has_rows = !rows.is_empty();
    let label = match state.documents() {
        [document] => t!("Diff.Viewer", name = document.path().as_str()).to_string(),
        documents => t!("Diff.ViewerFiles", count = documents.len()).to_string(),
    };
    let body = div()
        .id("diff-body")
        .relative()
        .min_h_0()
        .min_w_0()
        .flex_1()
        .child(SelectionLayer::new(
            div()
                .id("diff-horizontal")
                .size_full()
                .overflow_x_scroll()
                .track_scroll(&horizontal)
                .child(
                    list(scrollbar.clone(), move |ix, window, cx| {
                        let row = &rows[ix];
                        div()
                            .id(("diff-file", row.file()))
                            .w_full()
                            .child(render_row(row, code.clone(), window, cx))
                            .into_any_element()
                    })
                    .w(content_width)
                    .h_full(),
                )
                .into_any_element(),
            scrollbar.clone(),
            horizontal.clone(),
            state.selection().clone(),
            state.geometry().clone(),
        ))
        .when(has_rows, |this| {
            this.child(Scrollbar::vertical(&scrollbar))
                .child(Scrollbar::horizontal(&horizontal))
        })
        .on_action(cx.listener(|this, _: &ScrollUp, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            this.list().scroll_by(-window.rem_size() * 1.5);
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &ScrollDown, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            this.list().scroll_by(window.rem_size() * 1.5);
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &ScrollLeft, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            let offset = this.horizontal_scroll().offset();
            this.horizontal_scroll()
                .set_offset(point(offset.x + window.rem_size() * 3., offset.y));
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &ScrollRight, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            let offset = this.horizontal_scroll().offset();
            this.horizontal_scroll()
                .set_offset(point(offset.x - window.rem_size() * 3., offset.y));
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &PageUp, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            this.list().scroll_by(-this.viewport().height * 0.9);
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &PageDown, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            this.list().scroll_by(this.viewport().height * 0.9);
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &First, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            this.list().scroll_to(ListOffset::default());
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &Last, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            this.list().scroll_to_end();
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &ExtendUp, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            this.keyboard_select(-1, true, window, cx)
        }))
        .on_action(cx.listener(|this, _: &ExtendDown, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            this.keyboard_select(1, true, window, cx)
        }))
        .on_action(cx.listener(|this, _: &SelectAll, window, cx| {
            if body_focused(this, window, cx) {
                this.select_all(window, cx);
            }
        }))
        .on_action(cx.listener(|this, _: &Copy, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            let text = this.selected_text(cx);
            if text.is_empty() {
                cx.propagate();
            } else {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
        }))
        .child(
            gpui::canvas(
                {
                    let state = cx.entity();
                    move |bounds, _, cx| {
                        state.update(cx, |state, cx| {
                            // The estimate shapes the widest candidates only;
                            // grow to fit any wider line once it is painted.
                            let painted = state.geometry().borrow().painted_width();
                            let painted = code_width(painted, gutter_width, rem, mode);
                            if state.record_paint(bounds.size, painted) {
                                cx.notify();
                            }
                        });
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        )
        .test_support()
        .track_focus(&state.focus_handle(cx))
        .key_context("Diff")
        .role(gpui::accesskit::Role::Document)
        .aria_label(label);
    v_flex()
        .id(("diff", cx.entity_id()))
        .size_full()
        .min_w_0()
        .min_h_0()
        .bg(cx.theme().background)
        .text_color(cx.theme().foreground)
        .text_sm()
        .border_1()
        .border_color(cx.theme().border)
        .child(body)
        .refine_style(&props.style)
        .into_any_element()
}

fn body_focused(state: &DiffState, window: &Window, cx: &mut Context<DiffState>) -> bool {
    if state.focus_handle(cx).is_focused(window) {
        true
    } else {
        cx.propagate();
        false
    }
}

struct CodePresentation {
    documents: Vec<DiffDocument>,
    mode: DiffMode,
    column_width: Pixels,
    font_size: Pixels,
    row_height: Pixels,
    gutter_width: Pixels,
    line_numbers: bool,
    syntax_highlight: bool,
    header_visible: bool,
    header_renderer: Option<HeaderRenderer>,
    annotations: Rc<Vec<DiffLineAnnotation>>,
    annotation_index: Rc<HashMap<DiffLinePosition, Vec<usize>>>,
    annotation_renderer: Option<AnnotationRenderer>,
    selection: gpui_base::TextSelectionHandle,
    selected_lines: Option<DiffLineRange>,
    geometry: Rc<std::cell::RefCell<selection::SelectionGeometry>>,
    state: Entity<DiffState>,
}

impl CodePresentation {
    fn position(&self, file: usize, side: DiffSide, ix: usize) -> DiffLinePosition {
        DiffLinePosition::new(file, side, self.documents[file].line_number(side, ix))
    }
}

/// Total row width for code of `width` in the current layout.
fn code_width(width: Pixels, gutter_width: Pixels, rem: Pixels, mode: DiffMode) -> Pixels {
    if mode == DiffMode::Split {
        (width + gutter_width + rem * 2.) * 2.
    } else {
        width + gutter_width * 2. + rem * 2.
    }
}

/// Estimates content width from the lines widest in display cells, then
/// shapes those candidates with the code font for resolved geometry.
fn measure_code_width(
    documents: &[DiffDocument],
    font_size: Pixels,
    family: &SharedString,
    gutter_width: Pixels,
    mode: DiffMode,
    window: &mut Window,
    cx: &App,
) -> Pixels {
    // Glyph widths vary within a font, so shape several candidates rather
    // than trusting the single widest by cell count.
    const CANDIDATES: usize = 8;
    let mut candidates: Vec<(usize, &SharedString)> = Vec::with_capacity(CANDIDATES + 1);
    for document in documents {
        for side in [DiffSide::Original, DiffSide::Modified] {
            for line in document.lines(side) {
                let cells = unicode_width::UnicodeWidthStr::width(line.display().as_str());
                if candidates.len() == CANDIDATES
                    && candidates
                        .last()
                        .is_some_and(|(widest, _)| *widest >= cells)
                {
                    continue;
                }
                let ix = candidates.partition_point(|(widest, _)| *widest >= cells);
                candidates.insert(ix, (cells, line.display()));
                candidates.truncate(CANDIDATES);
            }
        }
    }
    let font = gpui::Font {
        family: family.clone(),
        ..Default::default()
    };
    let width = candidates
        .into_iter()
        .map(|(_, text)| {
            window
                .text_system()
                .shape_line(
                    text.clone(),
                    font_size,
                    &[TextRun {
                        len: text.len(),
                        font: font.clone(),
                        color: cx.theme().foreground,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    None,
                )
                .width
        })
        .fold(px(0.), Pixels::max);
    code_width(width, gutter_width, window.rem_size(), mode)
}

fn render_row(
    row: &DisplayRow,
    code: Rc<CodePresentation>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    match row {
        DisplayRow::File(file) => render_file_header(*file, &code, window, cx),
        DisplayRow::Notice(file) => render_notice(&code.documents[*file], cx),
        DisplayRow::Hunk { file, hunk } => h_flex()
            .w_full()
            .h_6()
            .px_3()
            .bg(cx.theme().muted.opacity(0.35))
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(code.documents[*file].hunks()[*hunk].label().clone())
            .into_any_element(),
        DisplayRow::Fold { file, pairs } => {
            let (file, pairs) = (*file, pairs.clone());
            let state = code.state.clone();
            h_flex()
                .w_full()
                .h_6()
                .px_2()
                .bg(cx.theme().muted.opacity(0.35))
                .child(
                    Button::new(("expand", pairs.start))
                        .ghost()
                        .xsmall()
                        .w_auto()
                        .icon(crate::IconName::ChevronDown)
                        .text_color(cx.theme().muted_foreground)
                        .label(t!("Diff.UnchangedLines", count = pairs.len()).to_string())
                        .accessibility_label(
                            t!("Diff.ExpandLines", count = pairs.len()).to_string(),
                        )
                        .on_click(move |_, _, cx| {
                            state.update(cx, |state, cx| state.expand(file, pairs.clone(), cx))
                        }),
                )
                .into_any_element()
        }
        DisplayRow::Code {
            file,
            original,
            modified,
            changed,
        } => {
            if code.mode == DiffMode::Split {
                h_flex()
                    .items_stretch()
                    .w_full()
                    .child(render_cell(
                        *file,
                        DiffSide::Original,
                        *original,
                        *modified,
                        *changed,
                        code.clone(),
                        window,
                        cx,
                    ))
                    .child(render_cell(
                        *file,
                        DiffSide::Modified,
                        *modified,
                        *original,
                        *changed,
                        code,
                        window,
                        cx,
                    ))
                    .into_any_element()
            } else {
                let (side, ix) = match (original, modified) {
                    (_, Some(modified)) => (DiffSide::Modified, *modified),
                    (Some(original), None) => (DiffSide::Original, *original),
                    (None, None) => unreachable!("a code row shows at least one side"),
                };
                render_unified_cell(
                    *file,
                    side,
                    ix,
                    (*original, *modified),
                    *changed,
                    code,
                    window,
                    cx,
                )
            }
        }
    }
}

fn render_file_header(
    file: usize,
    code: &CodePresentation,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    if !code.header_visible {
        return div().into_any_element();
    }
    let document = &code.documents[file];
    let theme = cx.theme();
    let foreground = theme.muted_foreground;
    let border = theme.border;
    let content = if let Some(render) = &code.header_renderer {
        render(document, window, cx)
    } else {
        let name: SharedString = match (document.original_path(), document.modified_path()) {
            (Some(old), Some(new)) if old != new => format!("{old} → {new}").into(),
            _ => document.path().clone(),
        };
        h_flex()
            .w_full()
            .min_w_0()
            .gap_2()
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .truncate()
                    .font_medium()
                    .child(name),
            )
            .child(div().text_xs().text_color(foreground).child(format!(
                "+{} −{}",
                document.additions(),
                document.deletions()
            )))
            .when(document.original_path().is_none(), |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(foreground)
                        .child(t!("Diff.AddedFile").to_string()),
                )
            })
            .when(document.modified_path().is_none(), |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(foreground)
                        .child(t!("Diff.DeletedFile").to_string()),
                )
            })
            .into_any_element()
    };
    div()
        .id("file-header")
        .w_full()
        .px_3()
        .py_2()
        .when(file > 0, |this| this.border_t_1())
        .border_b_1()
        .border_color(border)
        .child(content)
        .into_any_element()
}

/// Summarizes a file without source rows: binary, metadata-only or empty.
fn render_notice(document: &DiffDocument, cx: &App) -> AnyElement {
    let message = if !document.has_changes() {
        t!("Diff.NoChanges")
    } else if document.is_binary() {
        t!("Diff.BinaryChanges")
    } else if !document.metadata().is_empty() {
        t!("Diff.NoTextChanges")
    } else {
        t!("Diff.EmptyFile")
    };
    v_flex()
        .w_full()
        .px_3()
        .py_2()
        .gap_1()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .children(
            document
                .metadata()
                .iter()
                .cloned()
                .map(|line| div().child(line)),
        )
        .child(div().text_sm().child(message.to_string()))
        .into_any_element()
}

fn render_gutter(
    file: usize,
    side: DiffSide,
    ix: Option<usize>,
    code: &CodePresentation,
    cx: &App,
) -> AnyElement {
    div()
        .id(if side == DiffSide::Original {
            "old-gutter"
        } else {
            "new-gutter"
        })
        .flex_shrink_0()
        .w(code.gutter_width)
        .pr_1()
        .when_some(ix, |this, ix| {
            let position = code.position(file, side, ix);
            let state = code.state.clone();
            // A plain hit target: line numbers need no button chrome or state.
            this.child(
                div()
                    .id(("line", ix))
                    .test_support()
                    .size_full()
                    .px_1()
                    .flex()
                    .items_center()
                    .justify_end()
                    .text_size(code.font_size)
                    .text_color(cx.theme().muted_foreground)
                    .hover(|this| this.text_color(cx.theme().foreground))
                    .role(gpui::accesskit::Role::Button)
                    .aria_label(
                        t!(
                            "Diff.SelectLine",
                            side = side_label(side),
                            line = position.line()
                        )
                        .to_string(),
                    )
                    .child(position.line().to_string())
                    .on_click(move |event, window, cx| {
                        state.update(cx, |state, cx| {
                            state.click_line(position, event.modifiers().shift, window, cx)
                        })
                    }),
            )
        })
        .into_any_element()
}

fn side_label(side: DiffSide) -> std::borrow::Cow<'static, str> {
    if side == DiffSide::Original {
        t!("Diff.Original")
    } else {
        t!("Diff.Modified")
    }
}

#[allow(clippy::too_many_arguments)]
fn render_cell(
    file: usize,
    side: DiffSide,
    ix: Option<usize>,
    other_ix: Option<usize>,
    changed: bool,
    code: Rc<CodePresentation>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let cell = v_flex()
        .w(code.column_width)
        .min_w_0()
        .flex_shrink_0()
        .when(side == DiffSide::Original, |this| {
            this.border_r_1().border_color(cx.theme().border)
        })
        .when(ix.is_none(), |this| {
            this.bg(cx.theme().muted.opacity(0.3)).child(div().h_6())
        });
    let Some(ix) = ix else {
        return cell.into_any_element();
    };
    cell.id((
        if side == DiffSide::Original {
            "old"
        } else {
            "new"
        },
        ix,
    ))
    .child(code_line(
        file,
        side,
        ix,
        changed,
        code.clone(),
        [Some((side, Some(ix))), None],
        cx,
    ))
    .children(line_extras(
        file, side, ix, other_ix, changed, &code, window, cx,
    ))
    .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn render_unified_cell(
    file: usize,
    side: DiffSide,
    ix: usize,
    pair: (Option<usize>, Option<usize>),
    changed: bool,
    code: Rc<CodePresentation>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let (original, modified) = pair;
    let counterpart = code.documents[file].lines(side)[ix].counterpart();
    let other_ix = if side == DiffSide::Original {
        modified
    } else {
        original
    };
    let mut extras = line_extras(file, side, ix, counterpart, changed, &code, window, cx);
    if !changed && let Some(other_ix) = other_ix {
        extras.extend(annotation_extras(
            file,
            side.other(),
            other_ix,
            &code,
            window,
            cx,
        ));
    }
    v_flex()
        .w(code.column_width)
        .id((
            if side == DiffSide::Original {
                "old"
            } else {
                "new"
            },
            ix,
        ))
        .child(code_line(
            file,
            side,
            ix,
            changed,
            code.clone(),
            [
                Some((DiffSide::Original, original)),
                Some((DiffSide::Modified, modified)),
            ],
            cx,
        ))
        .children(extras)
        .into_any_element()
}

fn code_line(
    file: usize,
    side: DiffSide,
    ix: usize,
    changed: bool,
    code: Rc<CodePresentation>,
    gutters: [Option<(DiffSide, Option<usize>)>; 2],
    cx: &mut App,
) -> AnyElement {
    let document = &code.documents[file];
    let line = &document.lines(side)[ix];
    let status = if side == DiffSide::Original {
        cx.theme().danger
    } else {
        cx.theme().success
    };
    let selected = code.selected_lines.is_some_and(|range| {
        range.contains(code.position(file, side, ix))
            || (code.mode == DiffMode::Unified
                && !changed
                && gutters.iter().flatten().any(|(side, ix)| {
                    ix.is_some_and(|ix| range.contains(code.position(file, *side, ix)))
                }))
    });
    let mut highlights = if code.syntax_highlight {
        document.line_highlights(side, ix, cx.theme().highlight_theme.as_ref())
    } else {
        Vec::new()
    };
    if selected {
        highlights.push((
            0..line.display().len(),
            HighlightStyle {
                background_color: Some(cx.theme().selection),
                ..Default::default()
            },
        ));
    }
    // An unchanged Unified row shows one source line for both sides; follow
    // the side a text selection started on so its offsets stay consistent.
    let (text_side, text_ix) = if code.mode == DiffMode::Unified
        && !changed
        && let Some((selected_file, selected_side, _)) =
            selection::selected_source_range(&code.selection, cx)
        && selected_file == file
        && selected_side != side
        && let Some(other_ix) = line.counterpart()
    {
        (selected_side, other_ix)
    } else {
        (side, ix)
    };
    let position = code.position(file, text_side, text_ix);
    let source_line = &document.lines(text_side)[text_ix];
    let text =
        h_flex()
            .gap_0()
            .flex_shrink_0()
            .children(
                source_line
                    .chunk_ranges()
                    .enumerate()
                    .map(|(chunk_ix, range)| {
                        CodeText::new(
                            ("code", chunk_ix).into(),
                            document.clone(),
                            file,
                            text_side,
                            text_ix,
                            range,
                            highlights.clone(),
                            code.selection.clone(),
                            code.geometry.clone(),
                        )
                    }),
            );
    h_flex()
        .w_full()
        .h(code.row_height)
        .font_family(cx.theme().mono_font_family.clone())
        .text_size(code.font_size)
        .line_height(code.row_height)
        .whitespace_nowrap()
        .when(changed, |this| this.bg(status.opacity(0.12)))
        .when(selected, |this| this.bg(cx.theme().selection))
        .when(code.line_numbers, |this| {
            this.children(
                gutters
                    .into_iter()
                    .flatten()
                    .map(|(side, ix)| render_gutter(file, side, ix, &code, cx)),
            )
        })
        .child(
            div()
                .w_4()
                .flex_shrink_0()
                .text_color(if changed {
                    status
                } else {
                    cx.theme().muted_foreground
                })
                .child(if changed {
                    if side == DiffSide::Original {
                        "−"
                    } else {
                        "+"
                    }
                } else {
                    " "
                }),
        )
        .child(
            div()
                .id("source")
                .test_support()
                .flex_shrink_0()
                .role(gpui::accesskit::Role::Label)
                .aria_label(source_line.text().clone())
                .aria_description(
                    t!(
                        if changed {
                            if side == DiffSide::Original {
                                "Diff.RemovedLine"
                            } else {
                                "Diff.AddedLine"
                            }
                        } else {
                            "Diff.SourceLine"
                        },
                        side = side_label(text_side),
                        line = position.line()
                    )
                    .to_string(),
                )
                .child(text),
        )
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn line_extras(
    file: usize,
    side: DiffSide,
    ix: usize,
    other_ix: Option<usize>,
    changed: bool,
    code: &CodePresentation,
    window: &mut Window,
    cx: &mut App,
) -> Vec<AnyElement> {
    let mut extras = Vec::new();
    let document = &code.documents[file];
    let line = &document.lines(side)[ix];
    let ending = &document.source(side)[line.content_end()..line.source().end];
    let ending_only = changed && has_line_ending_change(document, side, ix, other_ix);
    if ending.is_empty() || ending_only {
        let label = if ending.is_empty() {
            t!("Diff.NoNewline").to_string()
        } else {
            format!("\\ {}", if ending == "\r\n" { "CRLF" } else { "LF" })
        };
        extras.push(
            div()
                .h_6()
                .px_3()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(label)
                .into_any_element(),
        );
    }
    extras.extend(annotation_extras(file, side, ix, code, window, cx));
    extras
}

fn annotation_extras(
    file: usize,
    side: DiffSide,
    ix: usize,
    code: &CodePresentation,
    window: &mut Window,
    cx: &mut App,
) -> Vec<AnyElement> {
    let Some(render) = &code.annotation_renderer else {
        return Vec::new();
    };
    let position = code.position(file, side, ix);
    code.annotation_index
        .get(&position)
        .into_iter()
        .flatten()
        .map(|ix| {
            let annotation = &code.annotations[*ix];
            div()
                .id(annotation.id.clone())
                .w_full()
                .px_3()
                .py_3()
                .border_t_1()
                .border_b_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().muted.opacity(0.18))
                .child(render(annotation, window, cx))
                .into_any_element()
        })
        .collect()
}

fn has_line_ending_change(
    document: &DiffDocument,
    side: DiffSide,
    ix: usize,
    counterpart: Option<usize>,
) -> bool {
    counterpart.is_some_and(|counterpart| {
        let other_side = side.other();
        let line = &document.lines(side)[ix];
        let other = &document.lines(other_side)[counterpart];
        line.text() == other.text()
            && document.source(side)[line.content_end()..line.source().end]
                != document.source(other_side)[other.content_end()..other.source().end]
    })
}

#[cfg(test)]
mod copy_tests;
#[cfg(test)]
mod header_tests;
#[cfg(test)]
mod newline_tests;
#[cfg(test)]
mod selection_tests;
#[cfg(test)]
mod tests;
