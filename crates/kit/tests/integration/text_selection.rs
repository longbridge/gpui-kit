use crate::common;
use gpui_kit::base::text::{TextView, TextViewState};
use gpui_kit::{
    AppContext as _, Bounds, Context, Entity, Modifiers, MouseButton, Pixels, Render,
    TestAppContext, VisualTestContext, Window,
    base::{SelectableText, TextSelection},
    component, div, point,
    prelude::*,
    px,
};

const COMPATIBLE_SELECTION_CODE: &str =
    "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu";
const COMPATIBLE_SELECTION_COLOR: u32 = 0x20f0b0;
const COPY_SENTINEL: &str = "NO_TEXT_VIEW_COPY";

struct CompatibleSelectionView {
    text_view: Entity<TextViewState>,
    width: Pixels,
    top_padding: Pixels,
    style: gpui_kit::base::TextViewStyle,
    heading_font_size: Pixels,
    extensions: component::text::MarkdownExtensions,
    show_plain_text: bool,
    highlighter_color: Option<u32>,
    font_size: Option<Pixels>,
}

impl CompatibleSelectionView {
    fn new(cx: &mut Context<Self>) -> Self {
        let source = format!("# `{COMPATIBLE_SELECTION_CODE}`");
        let style =
            gpui_kit::base::TextViewStyle::default().with_inline_code(gpui_kit::HighlightStyle {
                background_color: Some(gpui_kit::rgb(COMPATIBLE_SELECTION_COLOR).into()),
                ..Default::default()
            });
        Self {
            text_view: cx.new(|cx| TextViewState::markdown(&source, cx)),
            width: px(640.),
            top_padding: px(0.),
            style,
            heading_font_size: px(16.),
            extensions: Default::default(),
            show_plain_text: false,
            highlighter_color: None,
            font_size: None,
        }
    }
}

impl Render for CompatibleSelectionView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .pt(self.top_padding)
            .when_some(self.font_size, |this, font_size| this.text_size(font_size))
            .when(self.show_plain_text, |this| {
                this.child(
                    div()
                        .h(px(40.))
                        .child(SelectableText::new("plain-adapter", "Plain adapter")),
                )
            })
            .child(
                div().w(self.width).child(
                    TextView::new(&self.text_view)
                        .selectable(true)
                        .style(self.style.clone().with_heading({
                            let font_size = self.heading_font_size;
                            move |_| gpui_kit::StyleRefinement::default().text_size(font_size)
                        }))
                        .markdown_extensions(self.extensions.clone())
                        .when_some(self.highlighter_color, |this, color| {
                            this.code_block_highlighter(move |block| {
                                vec![(
                                    0..block.code().len(),
                                    gpui_kit::HighlightStyle {
                                        background_color: Some(gpui_kit::rgb(color).into()),
                                        ..Default::default()
                                    },
                                )]
                            })
                        }),
                ),
            )
    }
}

fn setup_compatible_selection(
    cx: &mut TestAppContext,
) -> (Entity<CompatibleSelectionView>, VisualTestContext) {
    cx.update(gpui_kit::init);
    let (window, view) =
        common::open_window(cx, None, |_, cx| cx.new(CompatibleSelectionView::new));
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (view, cx)
}

fn painted_background_bounds(color: u32, cx: &mut VisualTestContext) -> Vec<Bounds<Pixels>> {
    cx.update(|window, _| {
        let background: gpui_kit::Background = gpui_kit::rgb(color).into();
        let scale_factor = window.scale_factor();
        window
            .painted_quads()
            .into_iter()
            .filter(|quad| quad.background == background)
            .map(|quad| quad.bounds.map(|value| px(value.0 / scale_factor)))
            .collect()
    })
}

fn copy_selection(cx: &mut VisualTestContext) -> String {
    // TestAppContext's platform owns this clipboard. Poison it for every
    // dispatch so an unhandled Copy cannot pass using a previous payload.
    cx.update(|window, cx| {
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(COPY_SENTINEL.into()));
        window.dispatch_action(Box::new(component::input::Copy), cx);
    });
    cx.run_until_parked();
    cx.update(|_, cx| {
        cx.read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap_or_default()
    })
}

fn select_compatible_partial(
    view: &Entity<CompatibleSelectionView>,
    cx: &mut VisualTestContext,
) -> (
    gpui_kit::Point<Pixels>,
    gpui_kit::Point<Pixels>,
    String,
    std::ops::Range<usize>,
) {
    let backgrounds = painted_background_bounds(COMPATIBLE_SELECTION_COLOR, cx);
    assert_eq!(backgrounds.len(), 1, "initial inline code must fit one row");
    let bounds = backgrounds[0];
    let start = point(bounds.left() + px(20.), bounds.center().y);
    let end = point(bounds.left() + px(130.), bounds.center().y);
    drag(cx, start, end);
    let selected = window_selected_text(cx);
    assert!(!selected.trim().is_empty(), "real drag must select text");
    assert_ne!(selected.trim(), COMPATIBLE_SELECTION_CODE);
    assert!(COMPATIBLE_SELECTION_CODE.contains(selected.trim()));
    assert_eq!(copy_selection(cx), selected.trim());
    let range = view.read_with(cx, |view, cx| {
        let state = view.text_view.read(cx);
        assert!(!state.is_selecting(), "selection must be ended");
        state.selected_source_range().expect("partial source range")
    });
    assert!(range.start > 3, "selection must start inside the code span");
    assert!(range.end < COMPATIBLE_SELECTION_CODE.len() + 3);
    (start, end, selected, range)
}

#[gpui_kit::test]
fn color_style_change_keeps_finished_partial_selection_and_copy(cx: &mut TestAppContext) {
    const NEW_COLOR: u32 = 0xf020b0;
    const REPLACEMENT: &str = "omega zulu sigma tango upsilon kilo rho bravo echo victor yankee pi";
    let (view, mut test_cx) = setup_compatible_selection(cx);
    let cx = &mut test_cx;
    let (start, end, selected, range) = select_compatible_partial(&view, cx);
    let old_backgrounds = painted_background_bounds(COMPATIBLE_SELECTION_COLOR, cx);
    let state = view.read_with(cx, |view, _| view.text_view.clone());

    view.update(cx, |view, cx| {
        view.style = view
            .style
            .clone()
            .with_inline_code(gpui_kit::HighlightStyle {
                background_color: Some(gpui_kit::rgb(NEW_COLOR).into()),
                ..view.style.inline_code()
            });
        cx.notify();
    });
    for _ in 0..3 {
        view.update(cx, |_, cx| cx.notify());
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        assert_eq!(window_selected_text(cx), selected);
        assert_eq!(
            state.read_with(cx, |state, _| state.selected_source_range()),
            Some(range.clone())
        );
        assert_eq!(copy_selection(cx), selected.trim());
        assert_eq!(painted_background_bounds(NEW_COLOR, cx), old_backgrounds);
        assert!(painted_background_bounds(COMPATIBLE_SELECTION_COLOR, cx).is_empty());
    }

    // The policy is presentation-only: a new committed document must not
    // keep copying the old range, even when the same Entity stays mounted.
    state.update(cx, |state, cx| {
        state.set_text(&format!("# `{REPLACEMENT}`"), cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        let _ = window.draw(cx);
    });
    assert_eq!(window_selected_text(cx), "");
    assert!(
        state
            .read_with(cx, |state, _| state.selected_source_range())
            .is_none()
    );
    assert!(!cx.update(|window, cx| TextSelection::has_selection(window, cx)));
    assert_eq!(copy_selection(cx), COPY_SENTINEL);

    drag(cx, start, end);
    let replacement_selection = window_selected_text(cx);
    assert!(!replacement_selection.trim().is_empty());
    assert!(REPLACEMENT.contains(replacement_selection.trim()));
    assert_ne!(replacement_selection, selected);
    assert_eq!(copy_selection(cx), replacement_selection.trim());
}

#[gpui_kit::test]
fn width_only_narrowing_keeps_finished_partial_selection_and_copy(cx: &mut TestAppContext) {
    let (view, mut test_cx) = setup_compatible_selection(cx);
    let cx = &mut test_cx;
    let (_, _, selected, range) = select_compatible_partial(&view, cx);
    let state = view.read_with(cx, |view, _| view.text_view.clone());
    let before = state.read_with(cx, |state, _| state.bounds());
    let old_backgrounds = painted_background_bounds(COMPATIBLE_SELECTION_COLOR, cx);

    // No style/font/parser/source update accompanies this narrower layout.
    view.update(cx, |view, cx| {
        view.width = px(100.);
        cx.notify();
    });
    for _ in 0..3 {
        view.update(cx, |_, cx| cx.notify());
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        assert_eq!(window_selected_text(cx), selected);
        assert_eq!(
            state.read_with(cx, |state, _| state.selected_source_range()),
            Some(range.clone())
        );
        assert_eq!(copy_selection(cx), selected.trim());
    }
    let after = state.read_with(cx, |state, _| state.bounds());
    assert!(after.size.width < before.size.width);
    assert!(after.size.height > before.size.height);
    let backgrounds = painted_background_bounds(COMPATIBLE_SELECTION_COLOR, cx);
    assert!(
        backgrounds.len() > 1,
        "width alone must actually wrap the code"
    );
    assert_eq!(
        backgrounds[0].size.height, old_backgrounds[0].size.height,
        "the painted font size must remain unchanged"
    );

    // A fresh drag in the wrapped layout replaces the held logical range.
    let first_row = backgrounds[0];
    drag(
        cx,
        point(first_row.left() + px(1.), first_row.center().y),
        point(first_row.left() + px(40.), first_row.center().y),
    );
    let new_selection = window_selected_text(cx);
    assert!(!new_selection.trim().is_empty());
    assert!(COMPATIBLE_SELECTION_CODE.contains(new_selection.trim()));
    assert_ne!(new_selection, selected);
    assert_ne!(
        state.read_with(cx, |state, _| state.selected_source_range()),
        Some(range)
    );
    assert_eq!(copy_selection(cx), new_selection.trim());
}

#[gpui_kit::test]
fn font_and_width_reflow_then_outer_origin_shift_keeps_logical_partial_selection(
    cx: &mut TestAppContext,
) {
    let (view, mut test_cx) = setup_compatible_selection(cx);
    let cx = &mut test_cx;
    let (start, end, selected, range) = select_compatible_partial(&view, cx);
    let state = view.read_with(cx, |view, _| view.text_view.clone());
    let before = state.read_with(cx, |state, _| state.bounds());
    let old_backgrounds = painted_background_bounds(COMPATIBLE_SELECTION_COLOR, cx);

    view.update(cx, |view, cx| {
        view.width = px(220.);
        view.heading_font_size = px(32.);
        cx.notify();
    });
    for _ in 0..3 {
        view.update(cx, |_, cx| cx.notify());
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        assert_eq!(window_selected_text(cx), selected);
        assert_eq!(
            state.read_with(cx, |state, _| state.selected_source_range()),
            Some(range.clone())
        );
        assert_eq!(copy_selection(cx), selected.trim());
    }
    let after = state.read_with(cx, |state, _| state.bounds());
    assert!(after.size.width < before.size.width);
    assert!(after.size.height > before.size.height);
    let backgrounds = painted_background_bounds(COMPATIBLE_SELECTION_COLOR, cx);
    assert!(backgrounds.len() > 1, "the code must actually wrap");
    assert!(
        backgrounds[0].size.height > old_backgrounds[0].size.height * 1.5,
        "the new font must reach the painted fragments"
    );

    // Move the mounted consumer from outside TextView only after the font
    // reflow has settled. Geometry-only snapshot publication must not turn
    // its held logical range back into the pre-reflow pointer rectangle.
    let origin_shift = px(48.);
    view.update(cx, |view, cx| {
        view.top_padding = origin_shift;
        cx.notify();
    });
    for _ in 0..3 {
        view.update(cx, |_, cx| cx.notify());
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        assert_eq!(window_selected_text(cx), selected);
        assert_eq!(
            state.read_with(cx, |state, _| state.selected_source_range()),
            Some(range.clone())
        );
        assert_eq!(copy_selection(cx), selected.trim());
    }
    let moved = state.read_with(cx, |state, _| state.bounds());
    assert_eq!(moved.origin, after.origin + point(px(0.), origin_shift));
    assert_eq!(moved.size, after.size, "only the outer origin must move");
    let moved_backgrounds = painted_background_bounds(COMPATIBLE_SELECTION_COLOR, cx);
    assert_eq!(moved_backgrounds.len(), backgrounds.len());
    for (before, moved) in backgrounds.iter().zip(&moved_backgrounds) {
        assert_eq!(
            moved.origin,
            before.origin + point(px(0.), origin_shift),
            "the painted fragments must follow the outer layout"
        );
        assert_eq!(moved.size, before.size);
    }

    // Translate the original pointer rectangle into the moved consumer.
    // As a *new* real gesture in the larger font it selects different bytes.
    let translation = point(px(0.), origin_shift);
    drag(cx, start + translation, end + translation);
    let reprojected = window_selected_text(cx);
    assert!(!reprojected.trim().is_empty());
    assert_ne!(
        reprojected, selected,
        "the fixture must distinguish old geometry"
    );
    assert_ne!(
        state.read_with(cx, |state, _| state.selected_source_range()),
        Some(range)
    );
    assert_eq!(copy_selection(cx), reprojected.trim());
}

#[gpui_kit::test]
fn inherited_font_and_zoom_changes_keep_completed_selection(cx: &mut TestAppContext) {
    let (view, mut test_cx) = setup_compatible_selection(cx);
    let cx = &mut test_cx;
    let state = view.read_with(cx, |view, _| view.text_view.clone());
    state.update(cx, |state, cx| {
        state.set_text(&format!("`{COMPATIBLE_SELECTION_CODE}`"), cx)
    });
    view.update(cx, |view, cx| {
        view.font_size = Some(px(16.));
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let before = painted_background_bounds(COMPATIBLE_SELECTION_COLOR, cx);
    assert_eq!(before.len(), 1);
    drag(
        cx,
        point(before[0].left() + px(20.), before[0].center().y),
        point(before[0].left() + px(130.), before[0].center().y),
    );
    let selected = window_selected_text(cx);
    assert!(!selected.trim().is_empty());
    let range = state
        .read_with(cx, |state, _| state.selected_source_range())
        .unwrap();
    view.update(cx, |view, cx| {
        view.font_size = Some(px(32.));
        view.width = px(220.);
        cx.notify();
    });
    for _ in 0..3 {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(window_selected_text(cx), selected);
        assert_eq!(copy_selection(cx), selected.trim());
        assert_eq!(
            state.read_with(cx, |state, _| state.selected_source_range()),
            Some(range.clone())
        );
    }
    let after = painted_background_bounds(COMPATIBLE_SELECTION_COLOR, cx);
    assert!(after.len() > 1);
    assert!(after[0].size.height > before[0].size.height);
    cx.update(|window, cx| {
        component::Theme::update(cx, |theme| theme.font_size = px(24.));
        window.set_rem_size(px(24.));
        cx.notify(view.entity_id());
    });
    for _ in 0..3 {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(window_selected_text(cx), selected);
        assert_eq!(copy_selection(cx), selected.trim());
    }
}

#[gpui_kit::test]
fn completed_cross_block_selection_survives_reflow_and_clear(cx: &mut TestAppContext) {
    let (view, mut test_cx) = setup_compatible_selection(cx);
    let cx = &mut test_cx;
    let state = view.read_with(cx, |view, _| view.text_view.clone());
    state.update(cx, |state, cx| {
        state.set_text(
            &format!("# `{COMPATIBLE_SELECTION_CODE}`\n\n# `{COMPATIBLE_SELECTION_CODE}`"),
            cx,
        )
    });
    cx.run_until_parked();
    for _ in 0..3 {
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
    let rows = painted_background_bounds(COMPATIBLE_SELECTION_COLOR, cx);
    assert_eq!(rows.len(), 2);
    drag(
        cx,
        point(rows[0].left() + px(20.), rows[0].center().y),
        point(rows[1].left() + px(130.), rows[1].center().y),
    );
    let selected = window_selected_text(cx);
    assert!(selected.contains('\n'));
    let copied = copy_selection(cx);
    view.update(cx, |view, cx| {
        view.width = px(180.);
        view.heading_font_size = px(24.);
        cx.notify();
    });
    for _ in 0..3 {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(window_selected_text(cx), selected);
        assert_eq!(copy_selection(cx), copied);
    }
    assert!(painted_background_bounds(COMPATIBLE_SELECTION_COLOR, cx).len() > 2);
    cx.update(|window, cx| TextSelection::clear(window, cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(window_selected_text(cx), "");
    assert_eq!(copy_selection(cx), COPY_SENTINEL);
}

#[gpui_kit::test]
fn completed_cross_renderer_selection_survives_rich_text_reflow(cx: &mut TestAppContext) {
    let (view, mut test_cx) = setup_compatible_selection(cx);
    let cx = &mut test_cx;
    view.update(cx, |view, cx| {
        view.show_plain_text = true;
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let row = painted_background_bounds(COMPATIBLE_SELECTION_COLOR, cx)[0];
    drag(
        cx,
        point(px(1.), px(15.)),
        point(row.left() + px(130.), row.center().y),
    );
    let selected = window_selected_text(cx);
    assert!(selected.contains("Plain adapter"));
    assert!(selected.contains("alpha"));
    let copied = copy_selection(cx);
    view.update(cx, |view, cx| {
        view.width = px(100.);
        view.top_padding = px(48.);
        view.heading_font_size = px(24.);
        cx.notify();
    });
    for _ in 0..3 {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(window_selected_text(cx), selected);
        assert_eq!(copy_selection(cx), copied);
    }
    assert!(painted_background_bounds(COMPATIBLE_SELECTION_COLOR, cx).len() > 1);
}

#[gpui_kit::test]
fn highlighter_change_preserves_selection_but_parser_change_clears_it(cx: &mut TestAppContext) {
    const HIGHLIGHT: u32 = 0x12abcd;
    let (view, mut test_cx) = setup_compatible_selection(cx);
    let cx = &mut test_cx;
    let state = view.read_with(cx, |view, _| view.text_view.clone());
    state.update(cx, |state, cx| {
        state.set_text(
            &format!("# `{COMPATIBLE_SELECTION_CODE}`\n\n```rust\nfn main() {{}}\n```"),
            cx,
        )
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let (_, _, selected, range) = select_compatible_partial(&view, cx);
    view.update(cx, |view, cx| {
        view.highlighter_color = Some(HIGHLIGHT);
        cx.notify();
    });
    for _ in 0..3 {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(window_selected_text(cx), selected);
        assert_eq!(copy_selection(cx), selected.trim());
        assert_eq!(
            state.read_with(cx, |state, _| state.selected_source_range()),
            Some(range.clone())
        );
    }
    assert!(!painted_background_bounds(HIGHLIGHT, cx).is_empty());
    view.update(cx, |view, cx| {
        view.extensions = component::text::MarkdownExtensions::default().parser_revision(1);
        cx.notify();
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(window_selected_text(cx), "");
    assert_eq!(copy_selection(cx), COPY_SENTINEL);
    assert!(
        state
            .read_with(cx, |state, _| state.selected_source_range())
            .is_none()
    );
}

fn window_selected_text(cx: &mut VisualTestContext) -> String {
    cx.update(|window, cx| TextSelection::selected_text(window, cx))
}

fn drag(cx: &mut VisualTestContext, from: gpui_kit::Point<Pixels>, to: gpui_kit::Point<Pixels>) {
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.simulate_mouse_move(to, Some(MouseButton::Left), Modifiers::default());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
    cx.update(|window, cx| window.draw(cx).clear(cx));
}
