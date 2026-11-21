//! Two versions of a text side by side, both soft-wrapped, padded level with
//! spacer rows: every paragraph starts beside the paragraph it is paired with,
//! however much taller either one wraps, and a paragraph only one version has
//! stands beside blank rows on the other.

use std::{ops::Range, rc::Rc};

use gpui_kit::component::{
    ActiveTheme as _, Side, h_flex,
    input::{Editor, EditorState, Spacer, SpacerProvider},
    scroll::ScrollbarPlacement,
    switch::Switch,
    v_flex,
};
use gpui_kit::*;

/// The paragraphs of the two versions, one line each, in reading order.
/// `None` is a paragraph the version does not have.
const PARAGRAPHS: &[(Option<&str>, Option<&str>)] = &[
    (
        Some("The harbour was quiet in the early hours."),
        Some(
            "The harbour was quiet in the early hours, before the first ferry \
             left and before the fish market opened its doors to the trade.",
        ),
    ),
    (
        Some(
            "Fishing boats came in one by one, their engines low, and tied up \
             along the old stone wall where the water was deepest at low tide.",
        ),
        Some("Fishing boats came in one by one and tied up along the old wall."),
    ),
    (
        Some("Nobody spoke much. The work was known, and the light was coming."),
        Some("Nobody spoke much. The work was known, and the light was coming."),
    ),
    (
        None,
        Some(
            "A gull walked the length of the quay as if it owned it, stopping \
             at every crate to look inside, and nobody chased it away.",
        ),
    ),
    (
        Some(
            "By six the first crates were stacked on the quay, ice glittering \
             between them, and the auctioneer was clearing his throat for the \
             day's first lot while the buyers pretended not to be interested.",
        ),
        Some("By six the first crates were stacked on the quay."),
    ),
    (
        Some("Then the bell rang, and the harbour woke all at once."),
        None,
    ),
    (
        Some("It was, everyone agreed later, a good morning."),
        Some(
            "It was, everyone agreed later, a good morning — the kind that \
             makes a whole season feel possible.",
        ),
    ),
];

/// Pads one side's paragraphs with the rows by which the other side's partner
/// wraps taller, and puts blank rows where the other side has a paragraph this
/// one does not.
struct Level {
    this: WeakEntity<EditorState>,
    other: WeakEntity<EditorState>,
    /// Whether this side is the first of each pair.
    first: bool,
}

impl SpacerProvider for Level {
    fn spacers(&self, lines: Range<usize>, cx: &App) -> Vec<Spacer> {
        let (Some(this), Some(other)) = (self.this.upgrade(), self.other.upgrade()) else {
            return Vec::new();
        };
        let (this, other) = (this.read(cx), other.read(cx));
        let rows = |state: &EditorState, line| {
            state
                .display_map()
                .buffer_line_to_display_row_range(line)
                .map_or(0, |rows| rows.len())
        };
        let mut spacers = Vec::new();
        let (mut own_line, mut their_line) = (0, 0);
        let mut last: Option<usize> = None;
        for &(first, second) in PARAGRAPHS {
            let (own, theirs) = if self.first {
                (first, second)
            } else {
                (second, first)
            };
            let theirs = theirs.map(|_| {
                their_line += 1;
                rows(other, their_line - 1)
            });
            match (own, theirs) {
                (Some(_), theirs) => {
                    let own = rows(this, own_line);
                    let theirs = theirs.unwrap_or(0);
                    if theirs > own {
                        spacers.push(Spacer::below(own_line, theirs - own));
                    }
                    last = Some(own_line);
                    own_line += 1;
                }
                (None, Some(theirs)) => {
                    let spacer = match last {
                        Some(line) => Spacer::below(line, theirs),
                        None => Spacer::above(0, theirs),
                    };
                    spacers.push(spacer.with_background(cx.theme().muted));
                }
                (None, None) => {}
            }
        }
        spacers.retain(|spacer| lines.contains(&spacer.line()));
        spacers
    }
}

pub struct Example {
    original: Entity<EditorState>,
    rewritten: Entity<EditorState>,
    wrap: bool,
    /// The rows each side's spacers were last asked against.
    rows_versions: [usize; 2],
    scroll_y: [Pixels; 2],
    _subscriptions: Vec<Subscription>,
}

impl Example {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let text = |first: bool| {
            PARAGRAPHS
                .iter()
                .filter_map(|&(a, b)| if first { a } else { b })
                .collect::<Vec<_>>()
                .join("\n")
        };
        let original = cx.new(|cx| {
            EditorState::new(window, cx)
                .line_number(true)
                .soft_wrap(true)
                .gutter_side(Side::Right)
                .scrollbar_placement(ScrollbarPlacement::BottomLeft)
                .default_value(text(true))
        });
        let rewritten = cx.new(|cx| {
            EditorState::new(window, cx)
                .line_number(true)
                .soft_wrap(true)
                .default_value(text(false))
        });
        for (this, other, first) in [
            (&original, &rewritten, true),
            (&rewritten, &original, false),
        ] {
            let level = Rc::new(Level {
                this: this.downgrade(),
                other: other.downgrade(),
                first,
            });
            this.update(cx, |state, cx| state.set_spacer_provider(Some(level), cx));
        }
        let subscriptions = [&original, &rewritten]
            .map(|editor| cx.observe(editor, |_, _, cx| cx.notify()))
            .into();
        Self {
            original,
            rewritten,
            wrap: true,
            rows_versions: [0; 2],
            scroll_y: [px(0.); 2],
            _subscriptions: subscriptions,
        }
    }

    /// Each side is padded against the other, so a change in either side's
    /// rows — a resize, the wrap turned off — asks both again.
    fn keep_level(&mut self, cx: &mut Context<Self>) {
        let versions = [
            self.original.read(cx).display_map().version(),
            self.rewritten.read(cx).display_map().version(),
        ];
        if versions != self.rows_versions {
            self.rows_versions = versions;
            for editor in [&self.original, &self.rewritten] {
                editor.update(cx, |state, cx| state.refresh_spacers(cx));
            }
        }
    }

    /// Padded, both sides have the same rows, so they scroll at one offset.
    fn scroll_together(&mut self, cx: &mut Context<Self>) {
        let current = [
            self.original.read(cx).scroll_offset().y,
            self.rewritten.read(cx).scroll_offset().y,
        ];
        let moved = if current[0] != self.scroll_y[0] {
            Some(0)
        } else if current[1] != self.scroll_y[1] {
            Some(1)
        } else {
            None
        };
        if let Some(source) = moved {
            let y = current[source];
            let target = if source == 0 {
                &self.rewritten
            } else {
                &self.original
            };
            target.update(cx, |state, cx| {
                let mut offset = state.scroll_offset();
                offset.y = y;
                state.set_scroll_offset(offset, cx);
            });
            self.scroll_y = [y; 2];
        }
    }

    fn set_wrap(&mut self, wrap: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.wrap = wrap;
        for editor in [&self.original, &self.rewritten] {
            editor.update(cx, |state, cx| state.set_soft_wrap(wrap, window, cx));
        }
        cx.notify();
    }

    fn pane(label: &'static str, state: &Entity<EditorState>, cx: &App) -> impl IntoElement {
        v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(label),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(Editor::new(state).bordered(false).size_full()),
            )
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.keep_level(cx);
        self.scroll_together(cx);
        let view = cx.entity();
        v_flex()
            .size_full()
            .p_3()
            .gap_3()
            .child(
                Switch::new("wrap")
                    .label("Wrap long lines")
                    .checked(self.wrap)
                    .on_click(move |wrap, window, cx| {
                        view.update(cx, |this, cx| this.set_wrap(*wrap, window, cx));
                    }),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(Self::pane("Original", &self.original, cx))
                    .child(div().w_px().h_full().bg(cx.theme().border))
                    .child(Self::pane("Rewritten", &self.rewritten, cx)),
            )
    }
}

fn main() {
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(900.), px(560.)), cx)),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| Example::new(window, cx)),
        )
        .expect("Failed to open window");
    });
}
