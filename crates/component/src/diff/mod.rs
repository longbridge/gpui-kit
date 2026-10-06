//! Readonly, virtualized patch display inspired by Diffs from Pierre.
//! Parse an externally supplied patch into [`DiffDocument`], retain [`DiffState`]
//! in the owner, and build [`Diff`] during rendering.
mod document;
mod parser;
#[cfg(test)]
mod parser_tests;
mod selection;
mod state;

pub use document::{DiffDocument, DiffFile, DiffLinePosition, DiffLineRange, DiffSide};
pub use parser::DiffParseError;
pub use state::{DiffEvent, DiffState};

use std::{collections::HashMap, rc::Rc};

use gpui::{
    AnyElement, App, ClipboardItem, Context, ElementId, Entity, HighlightStyle,
    InteractiveElement as _, IntoElement, KeyBinding, ListOffset, ParentElement as _, Pixels,
    RenderOnce, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, TextRun,
    Window, actions, div, list, point, prelude::FluentBuilder as _, px, rems,
};
use gpui_base::TestSupportExt as _;
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    scroll::Scrollbar,
    v_flex,
};
use selection::{CodeText, SelectionLayer};
use state::DisplayRow;

// Share Action identity with app menus and Root's standard text commands.
pub use crate::input::{Copy, SelectAll};
actions!(
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
        NextChange,
        PreviousChange,
        ExtendUp,
        ExtendDown
    ]
);

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
        KeyBinding::new("f7", NextChange, Some("Diff")),
        KeyBinding::new("shift-f7", PreviousChange, Some("Diff")),
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
/// the file side and original one-based source line, independent of folding.
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
/// The supplied state retains focus, scrolling, expanded context and selection.
/// The component owns both scrolling axes, so give it a bounded height. Long
/// lines scroll rather than wrap; file headers and review annotations are
/// explicit slots, and application commands remain owned by the application.
#[derive(IntoElement)]
pub struct Diff {
    state: Entity<DiffState>,
    style: StyleRefinement,
    line_numbers: bool,
    syntax_highlight: bool,
    header: bool,
    annotations: Rc<Vec<DiffLineAnnotation>>,
    annotation_index: Rc<HashMap<DiffLinePosition, Vec<usize>>>,
    layout_revision: u64,
    header_renderer: Option<HeaderRenderer>,
    header_prefix_renderer: Option<HeaderRenderer>,
    header_filename_suffix_renderer: Option<HeaderRenderer>,
    header_metadata_renderer: Option<HeaderRenderer>,
    annotation_renderer: Option<AnnotationRenderer>,
}

impl Diff {
    pub fn new(state: &Entity<DiffState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            line_numbers: true,
            syntax_highlight: true,
            header: true,
            annotations: Rc::new(Vec::new()),
            annotation_index: Rc::new(HashMap::new()),
            layout_revision: 0,
            header_renderer: None,
            header_prefix_renderer: None,
            header_filename_suffix_renderer: None,
            header_metadata_renderer: None,
            annotation_renderer: None,
        }
    }
    /// Shows the original and modified line-number lanes. Default is true.
    pub fn line_numbers(mut self, value: bool) -> Self {
        self.line_numbers = value;
        self
    }
    /// Uses the document's prepared syntax grammar and current theme. Default is true.
    pub fn syntax_highlight(mut self, value: bool) -> Self {
        self.syntax_highlight = value;
        self
    }
    /// Shows the file header. Default is true.
    pub fn header(mut self, value: bool) -> Self {
        self.header = value;
        self
    }
    /// Supplies annotations with stable identities and source positions.
    pub fn with_annotations(
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
    /// Invalidates cached row heights when annotation content or layout changes.
    /// Change this revision when an application-owned annotation changes height.
    pub fn with_layout_revision(mut self, revision: u64) -> Self {
        self.layout_revision = revision;
        self
    }
    /// Replaces the file header content, keeping its shell and separator.
    pub fn render_header<F, E>(mut self, render: F) -> Self
    where
        F: Fn(&DiffDocument, &mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.header_renderer = Some(Rc::new(move |doc, window, cx| {
            render(doc, window, cx).into_any_element()
        }));
        self
    }
    /// Inserts content before the filename in the default header.
    pub fn render_header_prefix<F, E>(mut self, render: F) -> Self
    where
        F: Fn(&DiffDocument, &mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.header_prefix_renderer = Some(Rc::new(move |doc, window, cx| {
            render(doc, window, cx).into_any_element()
        }));
        self
    }
    /// Inserts compact content immediately after the filename.
    pub fn render_header_filename_suffix<F, E>(mut self, render: F) -> Self
    where
        F: Fn(&DiffDocument, &mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.header_filename_suffix_renderer = Some(Rc::new(move |doc, window, cx| {
            render(doc, window, cx).into_any_element()
        }));
        self
    }
    /// Inserts trailing content after the addition/deletion statistics.
    pub fn render_header_metadata<F, E>(mut self, render: F) -> Self
    where
        F: Fn(&DiffDocument, &mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.header_metadata_renderer = Some(Rc::new(move |doc, window, cx| {
            render(doc, window, cx).into_any_element()
        }));
        self
    }
    /// Renders each supplied annotation. Application state and commands stay with the owner.
    pub fn render_annotation<F, E>(mut self, render: F) -> Self
    where
        F: Fn(&DiffLineAnnotation, &mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.annotation_renderer = Some(Rc::new(move |annotation, window, cx| {
            render(annotation, window, cx).into_any_element()
        }));
        self
    }

    fn file_header(
        &self,
        document: &DiffDocument,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let theme = cx.theme();
        let content = if let Some(render) = &self.header_renderer {
            render(document, window, cx)
        } else {
            let name = match (document.original(), document.modified()) {
                (Some(old), Some(new)) if old.name() != new.name() => {
                    format!("{} → {}", old.name(), new.name()).into()
                }
                (_, Some(new)) => new.name().clone(),
                (Some(old), _) => old.name().clone(),
                _ => SharedString::default(),
            };
            let foreground = theme.muted_foreground;
            let prefix = self
                .header_prefix_renderer
                .as_ref()
                .map(|render| render(document, window, cx));
            let suffix = self
                .header_filename_suffix_renderer
                .as_ref()
                .map(|render| render(document, window, cx));
            let metadata = self
                .header_metadata_renderer
                .as_ref()
                .map(|render| render(document, window, cx));
            h_flex()
                .w_full()
                .min_w_0()
                .gap_2()
                .when_some(prefix, |this, prefix| this.child(prefix))
                .child(
                    h_flex()
                        .min_w_0()
                        .flex_1()
                        .gap_2()
                        .child(div().min_w_0().truncate().font_medium().child(name))
                        .when_some(suffix, |this, suffix| {
                            this.child(div().flex_shrink_0().child(suffix))
                        }),
                )
                .child(div().text_xs().text_color(foreground).child(format!(
                    "+{} −{}",
                    document.additions(),
                    document.deletions()
                )))
                .when(document.original().is_none(), |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(foreground)
                            .child(t!("Diff.AddedFile").to_string()),
                    )
                })
                .when(document.modified().is_none(), |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(foreground)
                            .child(t!("Diff.DeletedFile").to_string()),
                    )
                })
                .when_some(metadata, |this, metadata| this.child(metadata))
                .into_any_element()
        };
        div()
            .w_full()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(content)
            .into_any_element()
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
    let rem = window.rem_size();
    let font_size = rem * (f32::from(cx.theme().mono_font_size) / 16.);
    let font_family = cx.theme().mono_font_family.clone();
    let measurement = (
        rem,
        font_size,
        font_family.clone(),
        state.mode,
        props.layout_revision,
        props.line_numbers,
    );
    if state.measurement.as_ref() != Some(&measurement) {
        let scroll_top = state.list.logical_scroll_top();
        state.list.reset(state.rows.len());
        state.list.scroll_to(scroll_top);
        state.measurement = Some(measurement);
    }
    let width_measurement = (
        rem,
        font_size,
        font_family.clone(),
        state.mode,
        props.line_numbers,
    );
    if state.width_measurement.as_ref() != Some(&width_measurement) {
        state.measured_width = measure_code_width(
            &state.document,
            font_size,
            &font_family,
            props.line_numbers,
            state.mode,
            window,
            cx,
        );
        state.width_measurement = Some(width_measurement);
    }
    let content_width = state.measured_width.max(state.viewport_width);
    let column_width = if state.mode == DiffMode::Split {
        content_width / 2.
    } else {
        content_width
    };
    let code = Rc::new(CodePresentation {
        document: state.document.clone(),
        mode: state.mode,
        column_width,
        font_size,
        line_numbers: props.line_numbers,
        syntax_highlight: props.syntax_highlight,
        annotations: props.annotations.clone(),
        annotation_index: props.annotation_index.clone(),
        annotation_renderer: props.annotation_renderer.clone(),
        selection: state.selection.clone(),
        selected_lines: state.selected_lines,
        geometry: state.geometry.clone(),
        state: cx.entity(),
    });
    let rows = state.rows.clone();
    let header = props
        .header
        .then(|| props.file_header(&state.document, window, cx));
    let scrollbar = state.list.clone();
    let horizontal = state.horizontal_scroll.clone();
    let is_identical = !state.document.has_changes();
    let has_rows = !state.rows.is_empty();
    let digits = state
        .document
        .lines(DiffSide::Original)
        .last()
        .map_or(1, |line| line.line_number)
        .max(
            state
                .document
                .lines(DiffSide::Modified)
                .last()
                .map_or(1, |line| line.line_number),
        )
        .max(1)
        .to_string()
        .len();
    let gutter_width = if props.line_numbers {
        font_size * 0.7 * digits as f32 + rem
    } else {
        px(0.)
    };
    let mode = state.mode;
    let body = div()
        .id("diff-body")
        .relative()
        .min_h_0()
        .min_w_0()
        .flex_1()
        .child(SelectionLayer {
            selection: state.selection.clone(),
            geometry: state.geometry.clone(),
            scroll: state.list.clone(),
            horizontal: state.horizontal_scroll.clone(),
            child: div()
                .id("diff-horizontal")
                .size_full()
                .overflow_x_scroll()
                .track_scroll(&horizontal)
                .child(
                    list(state.list.clone(), move |ix, window, cx| {
                        render_row(&rows[ix], code.clone(), window, cx)
                    })
                    .w(content_width)
                    .h_full(),
                )
                .into_any_element(),
        })
        .when(has_rows, |this| {
            this.child(Scrollbar::vertical(&scrollbar))
                .child(Scrollbar::horizontal(&horizontal))
        })
        .on_action(cx.listener(|this, _: &ScrollUp, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            this.list.scroll_by(-window.rem_size() * 1.5);
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &ScrollDown, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            this.list.scroll_by(window.rem_size() * 1.5);
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &ScrollLeft, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            let offset = this.horizontal_scroll.offset();
            this.horizontal_scroll
                .set_offset(point(offset.x + window.rem_size() * 3., offset.y));
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &ScrollRight, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            let offset = this.horizontal_scroll.offset();
            this.horizontal_scroll
                .set_offset(point(offset.x - window.rem_size() * 3., offset.y));
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &PageUp, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            this.list.scroll_by(-this.viewport_height * 0.9);
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &PageDown, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            this.list.scroll_by(this.viewport_height * 0.9);
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &First, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            this.list.scroll_to(ListOffset::default());
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &Last, window, cx| {
            if !body_focused(this, window, cx) {
                return;
            }
            this.list.scroll_to_end();
            cx.notify();
        }))
        .on_action(cx.listener(|this, _: &NextChange, window, cx| {
            if body_focused(this, window, cx) {
                this.next_change(cx);
            }
        }))
        .on_action(cx.listener(|this, _: &PreviousChange, window, cx| {
            if body_focused(this, window, cx) {
                this.previous_change(cx);
            }
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
                            let actual_width = state
                                .geometry
                                .borrow()
                                .runs
                                .iter()
                                .fold((None, px(0.), px(0.)), |(previous, left, widest), run| {
                                    let key = (run.side, run.line_ix);
                                    let left = if previous == Some(key) {
                                        left
                                    } else {
                                        run.bounds.left()
                                    };
                                    (Some(key), left, widest.max(run.bounds.right() - left))
                                })
                                .2;
                            let actual_width = if mode == DiffMode::Split {
                                (actual_width + gutter_width + rem * 2.) * 2.
                            } else {
                                actual_width + gutter_width * 2. + rem * 2.
                            };
                            let wider = actual_width > state.measured_width;
                            if wider {
                                state.measured_width = actual_width;
                            }
                            if wider
                                || state.viewport_width != bounds.size.width
                                || state.viewport_height != bounds.size.height
                            {
                                state.viewport_width = bounds.size.width;
                                state.viewport_height = bounds.size.height;
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
        .track_focus(&state.focus)
        .key_context("Diff")
        .role(gpui::accesskit::Role::Document)
        .aria_label(
            t!(
                "Diff.Viewer",
                name = state
                    .document
                    .modified()
                    .or_else(|| state.document.original())
                    .map_or("", |f| f.name().as_str())
            )
            .to_string(),
        )
        .when(!has_rows, |this| {
            this.child(div().p_4().text_color(cx.theme().muted_foreground).child(
                if state.document.is_binary() {
                    t!("Diff.BinaryChanges").to_string()
                } else if !state.document.metadata().is_empty() {
                    t!("Diff.NoTextChanges").to_string()
                } else {
                    t!("Diff.EmptyFile").to_string()
                },
            ))
        });
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
        .when_some(header, |this, header| this.child(header))
        .when(!has_rows && !state.document.metadata().is_empty(), |this| {
            this.child(
                v_flex()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .children(
                        state
                            .document
                            .metadata()
                            .iter()
                            .cloned()
                            .map(|line| div().child(line)),
                    ),
            )
        })
        .when(is_identical, |this| {
            this.child(
                div()
                    .px_3()
                    .py_2()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("Diff.NoChanges").to_string()),
            )
        })
        .child(body)
        .refine_style(&props.style)
        .into_any_element()
}

fn body_focused(state: &DiffState, window: &Window, cx: &mut Context<DiffState>) -> bool {
    if state.focus.is_focused(window) {
        true
    } else {
        cx.propagate();
        false
    }
}

struct CodePresentation {
    document: DiffDocument,
    mode: DiffMode,
    column_width: Pixels,
    font_size: Pixels,
    line_numbers: bool,
    syntax_highlight: bool,
    annotations: Rc<Vec<DiffLineAnnotation>>,
    annotation_index: Rc<HashMap<DiffLinePosition, Vec<usize>>>,
    annotation_renderer: Option<AnnotationRenderer>,
    selection: gpui_base::TextSelectionHandle,
    selected_lines: Option<DiffLineRange>,
    geometry: Rc<std::cell::RefCell<selection::SelectionGeometry>>,
    state: Entity<DiffState>,
}

fn measure_code_width(
    document: &DiffDocument,
    font_size: Pixels,
    family: &SharedString,
    line_numbers: bool,
    mode: DiffMode,
    window: &mut Window,
    cx: &App,
) -> Pixels {
    let font = gpui::Font {
        family: family.clone(),
        ..Default::default()
    };
    let mut widths = Vec::new();
    for side in [DiffSide::Original, DiffSide::Modified] {
        // Choose by Unicode display cells rather than UTF-8 byte length, then
        // shape the candidate with the actual code font for resolved geometry.
        let longest = document
            .lines(side)
            .iter()
            .max_by_key(|line| unicode_width::UnicodeWidthStr::width(line.display.as_str()));
        let width = longest.map_or(px(0.), |line| {
            window
                .text_system()
                .shape_line(
                    line.display.clone(),
                    font_size,
                    &[TextRun {
                        len: line.display.len(),
                        font: font.clone(),
                        color: cx.theme().foreground,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    None,
                )
                .width
        });
        widths.push(width);
    }
    let digits = document
        .lines(DiffSide::Original)
        .last()
        .map_or(1, |line| line.line_number)
        .max(
            document
                .lines(DiffSide::Modified)
                .last()
                .map_or(1, |line| line.line_number),
        )
        .max(1)
        .to_string()
        .len();
    let gutter = if line_numbers {
        font_size * 0.7 * digits as f32 + window.rem_size() * 1.
    } else {
        px(0.)
    };
    let code_inset = window.rem_size() * 2.;
    if mode == DiffMode::Split {
        (widths[0].max(widths[1]) + gutter + code_inset) * 2.
    } else {
        widths[0].max(widths[1]) + gutter * 2. + code_inset
    }
}

fn render_row(
    row: &DisplayRow,
    code: Rc<CodePresentation>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    match row {
        DisplayRow::Hunk(label) => h_flex()
            .w_full()
            .h_6()
            .px_3()
            .bg(cx.theme().muted.opacity(0.35))
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(label.clone())
            .into_any_element(),
        DisplayRow::Fold(range) => {
            let range = range.clone();
            let state = code.state.clone();
            h_flex()
                .w_full()
                .h_6()
                .px_2()
                .bg(cx.theme().muted.opacity(0.35))
                .child(
                    Button::new(("expand", range.start))
                        .ghost()
                        .xsmall()
                        .w_auto()
                        .icon(crate::IconName::ChevronDown)
                        .text_color(cx.theme().muted_foreground)
                        .label(t!("Diff.UnchangedLines", count = range.len()).to_string())
                        .accessibility_label(
                            t!("Diff.ExpandLines", count = range.len()).to_string(),
                        )
                        .on_click(move |_, _, cx| {
                            state.update(cx, |state, cx| state.expand(range.clone(), cx))
                        }),
                )
                .into_any_element()
        }
        DisplayRow::Code {
            original,
            modified,
            changed,
        } => {
            if code.mode == DiffMode::Split {
                h_flex()
                    .items_stretch()
                    .w_full()
                    .child(render_cell(
                        DiffSide::Original,
                        *original,
                        *modified,
                        *changed,
                        code.clone(),
                        window,
                        cx,
                    ))
                    .child(render_cell(
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
                let side = if modified.is_some() {
                    DiffSide::Modified
                } else {
                    DiffSide::Original
                };
                let ix = if side == DiffSide::Modified {
                    *modified
                } else {
                    *original
                };
                render_unified_cell(
                    side,
                    ix.unwrap(),
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

fn render_gutter(
    side: DiffSide,
    ix: Option<usize>,
    code: &CodePresentation,
    cx: &App,
) -> AnyElement {
    let digits = code
        .document
        .lines(DiffSide::Original)
        .last()
        .map_or(1, |line| line.line_number)
        .max(
            code.document
                .lines(DiffSide::Modified)
                .last()
                .map_or(1, |line| line.line_number),
        )
        .max(1)
        .to_string()
        .len();
    let width = code.font_size * 0.7 * digits as f32
        + code.font_size * (16. / f32::from(cx.theme().mono_font_size));
    div()
        .id(if side == DiffSide::Original {
            "old-gutter"
        } else {
            "new-gutter"
        })
        .flex_shrink_0()
        .w(width)
        .pr_1()
        .text_color(cx.theme().muted_foreground)
        .when_some(ix, |this, ix| {
            let position = code.document.position(side, ix);
            let state = code.state.clone();
            this.child(
                Button::new(("line", ix))
                    .ghost()
                    .xsmall()
                    .tab_stop(false)
                    .w_full()
                    .h_full()
                    .px_1()
                    .justify_end()
                    .content_style(
                        StyleRefinement::default().text_size(code.font_size),
                        crate::Size::XSmall,
                    )
                    .text_color(cx.theme().muted_foreground)
                    .font_family(cx.theme().mono_font_family.clone())
                    .label(position.line().to_string())
                    .accessibility_label(
                        t!(
                            "Diff.SelectLine",
                            side = if side == DiffSide::Original {
                                t!("Diff.Original")
                            } else {
                                t!("Diff.Modified")
                            },
                            line = position.line()
                        )
                        .to_string(),
                    )
                    .on_click(move |event, window, cx| {
                        state.update(cx, |state, cx| {
                            state.click_line(position, event.modifiers().shift, window, cx)
                        })
                    }),
            )
        })
        .into_any_element()
}

fn render_cell(
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
        side,
        ix,
        changed,
        code.clone(),
        [Some((side, Some(ix))), None],
        window,
        cx,
    ))
    .children(line_extras(side, ix, other_ix, changed, code, window, cx))
    .into_any_element()
}

fn render_unified_cell(
    side: DiffSide,
    ix: usize,
    pair: (Option<usize>, Option<usize>),
    changed: bool,
    code: Rc<CodePresentation>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let (original, modified) = pair;
    let counterpart = code.document.lines(side)[ix].counterpart;
    let other_side = if side == DiffSide::Original {
        DiffSide::Modified
    } else {
        DiffSide::Original
    };
    let other_ix = if side == DiffSide::Original {
        modified
    } else {
        original
    };
    let mut extras = line_extras(side, ix, counterpart, changed, code.clone(), window, cx);
    if !changed && let Some(other_ix) = other_ix {
        extras.extend(annotation_extras(other_side, other_ix, &code, window, cx));
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
            side,
            ix,
            changed,
            code.clone(),
            [
                Some((DiffSide::Original, original)),
                Some((DiffSide::Modified, modified)),
            ],
            window,
            cx,
        ))
        .children(extras)
        .into_any_element()
}

fn code_line(
    side: DiffSide,
    ix: usize,
    changed: bool,
    code: Rc<CodePresentation>,
    gutters: [Option<(DiffSide, Option<usize>)>; 2],
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let line = &code.document.lines(side)[ix];
    let status = if side == DiffSide::Original {
        cx.theme().danger
    } else {
        cx.theme().success
    };
    let selected = code.selected_lines.is_some_and(|range| {
        range.contains(code.document.position(side, ix))
            || (code.mode == DiffMode::Unified
                && !changed
                && gutters.iter().flatten().any(|(side, ix)| {
                    ix.is_some_and(|ix| range.contains(code.document.position(*side, ix)))
                }))
    });
    let mut highlights = if code.syntax_highlight && line.text.len() <= 1000 {
        code.document
            .highlighter(side)
            .map_or_else(Vec::new, |highlighter| {
                highlighter.styles(
                    &(line.source.start..line.content_end),
                    cx.theme().highlight_theme.as_ref(),
                )
            })
            .into_iter()
            .map(|(range, style)| {
                (
                    line.display_range(
                        range.start.saturating_sub(line.source.start)
                            ..range.end.saturating_sub(line.source.start),
                    ),
                    style,
                )
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    if selected {
        highlights.push((
            0..line.display.len(),
            HighlightStyle {
                background_color: Some(cx.theme().selection),
                ..Default::default()
            },
        ));
    }
    let (text_side, text_ix) = if code.mode == DiffMode::Unified
        && !changed
        && let Some((selected_side, _)) = selection::selected_source_range(&code.selection, cx)
        && selected_side != side
        && let Some(other_ix) = line.counterpart
    {
        (selected_side, other_ix)
    } else {
        (side, ix)
    };
    let position = code.document.position(text_side, text_ix);
    let source_line = &code.document.lines(text_side)[text_ix];
    let text =
        h_flex()
            .gap_0()
            .flex_shrink_0()
            .children(
                source_line
                    .chunks
                    .iter()
                    .enumerate()
                    .map(|(chunk_ix, chunk)| {
                        CodeText::new(
                            ("code", chunk_ix).into(),
                            code.document.clone(),
                            position,
                            chunk.range.clone(),
                            highlights.clone(),
                            code.selection.clone(),
                            code.geometry.clone(),
                        )
                    }),
            );
    h_flex()
        .w_full()
        .h(rems(
            f32::from(code.font_size) / f32::from(window.rem_size()) * 1.6,
        ))
        .font_family(cx.theme().mono_font_family.clone())
        .text_size(code.font_size)
        .line_height(rems(
            f32::from(code.font_size) / f32::from(window.rem_size()) * 1.6,
        ))
        .whitespace_nowrap()
        .when(changed, |this| this.bg(status.opacity(0.12)))
        .when(selected, |this| this.bg(cx.theme().selection))
        .when(code.line_numbers, |this| {
            this.when_some(gutters[0], |this, (side, ix)| {
                this.child(render_gutter(side, ix, &code, cx))
            })
            .when_some(gutters[1], |this, (side, ix)| {
                this.child(render_gutter(side, ix, &code, cx))
            })
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
                .aria_label(source_line.text.clone())
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
                        side = if text_side == DiffSide::Original {
                            t!("Diff.Original")
                        } else {
                            t!("Diff.Modified")
                        },
                        line = position.line()
                    )
                    .to_string(),
                )
                .child(text),
        )
        .into_any_element()
}

fn line_extras(
    side: DiffSide,
    ix: usize,
    other_ix: Option<usize>,
    changed: bool,
    code: Rc<CodePresentation>,
    window: &mut Window,
    cx: &mut App,
) -> Vec<AnyElement> {
    let mut extras = Vec::new();
    let line = &code.document.lines(side)[ix];
    let source = code.document.source(side);
    let ending = &source[line.content_end..line.source.end];
    let ending_only = changed && has_line_ending_change(&code.document, side, ix, other_ix);
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
    extras.extend(annotation_extras(side, ix, &code, window, cx));
    extras
}

fn annotation_extras(
    side: DiffSide,
    ix: usize,
    code: &CodePresentation,
    window: &mut Window,
    cx: &mut App,
) -> Vec<AnyElement> {
    let mut extras = Vec::new();
    if let Some(render) = &code.annotation_renderer {
        let position = code.document.position(side, ix);
        for ix in code.annotation_index.get(&position).into_iter().flatten() {
            let annotation = &code.annotations[*ix];
            extras.push(
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
                    .into_any_element(),
            );
        }
    }
    extras
}

fn has_line_ending_change(
    document: &DiffDocument,
    side: DiffSide,
    ix: usize,
    counterpart: Option<usize>,
) -> bool {
    counterpart.is_some_and(|counterpart| {
        let other_side = if side == DiffSide::Original {
            DiffSide::Modified
        } else {
            DiffSide::Original
        };
        let line = &document.lines(side)[ix];
        let other = &document.lines(other_side)[counterpart];
        line.text == other.text
            && document.source(side)[line.content_end..line.source.end]
                != document.source(other_side)[other.content_end..other.source.end]
    })
}

#[cfg(test)]
mod copy_tests;
#[cfg(test)]
mod header_tests;
#[cfg(test)]
mod selection_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod newline_tests;
