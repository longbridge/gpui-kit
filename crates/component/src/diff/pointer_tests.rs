use std::{cell::RefCell, rc::Rc};

use gpui::{
    AppContext as _, Context, Entity, Focusable as _, IntoElement, Modifiers, ParentElement as _,
    Pixels, Point, Render, Styled as _, TestAppContext, VisualTestContext, Window, div, point, px,
};

use super::{
    Diff, DiffLinePosition, DiffSide, DiffSpanHighlight, DiffState, DiffTextPosition,
    document::fixture, state::DisplayRow,
};

type Heard = Rc<RefCell<Vec<(Option<(usize, usize, String)>, bool)>>>;

struct Host {
    state: Entity<DiffState>,
    hovered: Heard,
    text_clicks: Rc<RefCell<Vec<(usize, usize)>>>,
    line_clicks: Rc<RefCell<Vec<usize>>>,
    /// Whether the text handler claims the clicks it hears.
    claim: bool,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let hovered = self.hovered.clone();
        let text_clicks = self.text_clicks.clone();
        let line_clicks = self.line_clicks.clone();
        let claim = self.claim;
        div().w(px(600.)).h(px(400.)).child(
            Diff::new(&self.state)
                .size_full()
                .span_highlights([DiffSpanHighlight::new(
                    DiffLinePosition::new("file.rs", DiffSide::Modified, 1),
                    3..8,
                )
                .with_background(gpui::transparent_black())
                .with_underline()])
                .on_text_hover(move |hit, modifiers, _, _| {
                    hovered.borrow_mut().push((
                        hit.map(|hit: &DiffTextPosition| {
                            (hit.position().line(), hit.offset(), hit.text().to_string())
                        }),
                        modifiers.platform,
                    ));
                })
                .on_text_click(move |hit, _, _, _| {
                    text_clicks
                        .borrow_mut()
                        .push((hit.position().line(), hit.offset()));
                    claim
                })
                .on_line_click(move |position, _, _, _| {
                    line_clicks.borrow_mut().push(position.line());
                }),
        )
    }
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        _ = window.draw(cx);
    });
}

struct Stage<'a> {
    state: Entity<DiffState>,
    hovered: Heard,
    text_clicks: Rc<RefCell<Vec<(usize, usize)>>>,
    line_clicks: Rc<RefCell<Vec<usize>>>,
    cx: &'a mut VisualTestContext,
}

fn stage(cx: &mut TestAppContext, claim: bool) -> Stage<'_> {
    cx.update(crate::init);
    let state = cx.new(|cx| {
        DiffState::new(
            [fixture::modified(
                "file.rs",
                "fn place() {}\n",
                "fn place() {}\nfn other() {}\n",
            )],
            cx,
        )
    });
    let hovered = Heard::default();
    let text_clicks = Rc::default();
    let line_clicks = Rc::default();
    let (_, cx) = cx.add_window_view({
        let (state, hovered) = (state.clone(), hovered.clone());
        let (text_clicks, line_clicks) = (Rc::clone(&text_clicks), Rc::clone(&line_clicks));
        move |_, _| Host {
            state,
            hovered,
            text_clicks,
            line_clicks,
            claim,
        }
    });
    for _ in 0..3 {
        draw(cx);
    }
    Stage {
        state,
        hovered,
        text_clicks,
        line_clicks,
        cx,
    }
}

/// A point inside the first painted run of the modified side's `line_ix`,
/// `inset` from its left edge, and one past the run's right edge.
fn points(stage: &mut Stage<'_>, line_ix: usize, inset: Pixels) -> (Point<Pixels>, Point<Pixels>) {
    let state = stage.state.clone();
    let bounds = stage.cx.update(|_, cx| {
        state
            .read(cx)
            .geometry()
            .borrow()
            .run_bounds(DiffSide::Modified, line_ix)
            .first()
            .copied()
            .expect("the line is painted")
    });
    let y = bounds.center().y;
    (
        point(bounds.left() + inset, y),
        point(bounds.right() + px(40.), y),
    )
}

#[gpui::test]
fn text_hover_reports_the_character_and_only_changes(cx: &mut TestAppContext) {
    let mut stage = stage(cx, true);
    let (on_text, past_end) = points(&mut stage, 1, px(1.));

    stage
        .cx
        .simulate_mouse_move(on_text, None, Modifiers::default());
    // The same character again says nothing new.
    stage
        .cx
        .simulate_mouse_move(on_text, None, Modifiers::default());
    stage
        .cx
        .simulate_mouse_move(past_end, None, Modifiers::default());

    assert_eq!(
        *stage.hovered.borrow(),
        [
            (Some((2, 0, "fn other() {}".to_string())), false),
            // Past the line's end is over no character, not the nearest one.
            (None, false),
        ]
    );
}

#[gpui::test]
fn a_modifier_alone_reports_the_character_again(cx: &mut TestAppContext) {
    let mut stage = stage(cx, true);
    let (on_text, _) = points(&mut stage, 0, px(1.));
    let state = stage.state.clone();
    stage
        .cx
        .update(|window, cx| state.read(cx).focus_handle(cx).focus(window, cx));
    draw(stage.cx);

    stage
        .cx
        .simulate_mouse_move(on_text, None, Modifiers::default());
    stage.cx.simulate_modifiers_change(Modifiers::command());

    assert_eq!(
        *stage.hovered.borrow(),
        [
            (Some((1, 0, "fn place() {}".to_string())), false),
            (Some((1, 0, "fn place() {}".to_string())), true),
        ]
    );
}

#[gpui::test]
fn a_claimed_text_click_keeps_the_line_handler_quiet(cx: &mut TestAppContext) {
    let mut stage = stage(cx, true);
    let (on_text, past_end) = points(&mut stage, 1, px(1.));

    stage.cx.simulate_click(on_text, Modifiers::command());
    assert_eq!(*stage.text_clicks.borrow(), [(2, 0)]);
    assert!(stage.line_clicks.borrow().is_empty());

    // Off the text there is no character to claim: the line still hears it.
    stage.cx.simulate_click(past_end, Modifiers::default());
    assert_eq!(stage.text_clicks.borrow().len(), 1);
}

#[gpui::test]
fn an_unclaimed_text_click_falls_through_to_the_line(cx: &mut TestAppContext) {
    let mut stage = stage(cx, false);
    let (on_text, _) = points(&mut stage, 1, px(1.));

    stage.cx.simulate_click(on_text, Modifiers::default());

    assert_eq!(*stage.text_clicks.borrow(), [(2, 0)]);
    assert_eq!(*stage.line_clicks.borrow(), [2]);
}

#[gpui::test]
fn has_line_counts_folded_lines_but_not_absent_ones(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let original: String = (0..200).map(|ix| format!("line {ix}\n")).collect();
    let modified = original.replacen("line 0\n", "first\n", 1);
    let state =
        cx.new(|cx| DiffState::new([fixture::modified("long.txt", &original, &modified)], cx));
    let far = DiffLinePosition::new("long.txt", DiffSide::Modified, 150);

    cx.update(|cx| {
        let state = state.read(cx);
        let shown = state
            .source_row(&far)
            .and_then(|row| state.rows().get(row).cloned())
            .is_some_and(|row| matches!(row, DisplayRow::Code { .. }));
        assert!(!shown, "far from the change, the line is folded");
        assert!(state.has_line(&far));
        assert!(!state.has_line(&DiffLinePosition::new("long.txt", DiffSide::Modified, 9000)));
        assert!(!state.has_line(&DiffLinePosition::new("other.txt", DiffSide::Modified, 1)));
    });
}
