use gpui::{
    AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, ScrollDelta,
    ScrollEvent, Styled as _, TestAppContext, VisualTestContext, Window, div, point, px,
};

use super::{Diff, DiffState, document::fixture};

/// Every line changed, so the unified patch is two rows per line.
const LINES: usize = 300;

struct Host {
    state: Entity<DiffState>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(400.))
            .h(px(200.))
            .child(Diff::new(&self.state).size_full())
    }
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        _ = window.draw(cx);
    });
}

fn long_patch(cx: &mut TestAppContext) -> (Entity<DiffState>, &mut VisualTestContext) {
    cx.update(crate::init);
    let original = (0..LINES)
        .map(|ix| format!("old {ix}\n"))
        .collect::<String>();
    let modified = (0..LINES)
        .map(|ix| format!("new {ix}\n"))
        .collect::<String>();
    let state =
        cx.new(|cx| DiffState::new([fixture::modified("long.txt", &original, &modified)], cx));
    let (_, cx) = cx.add_window_view({
        let state = state.clone();
        move |_, _| Host { state }
    });
    // The body records its size on the first paint and lays out again.
    for _ in 0..3 {
        draw(cx);
    }
    (state, cx)
}

#[gpui::test]
fn the_whole_patch_keeps_its_height_after_the_first_layout(cx: &mut TestAppContext) {
    let (state, cx) = long_patch(cx);
    let (max, viewport) = cx.update(|_, cx| {
        let list = state.read(cx).list();
        (
            list.max_offset_for_scrollbar().y,
            list.viewport_bounds().size.height,
        )
    });
    // Rows past the viewport have not been rendered; without their hinted
    // height the patch would read as barely taller than one screen.
    assert!(
        max > viewport * 10.,
        "max offset {max:?} for a {viewport:?} viewport"
    );
}

#[gpui::test]
fn the_wheel_scrolls_past_the_rows_rendered_so_far(cx: &mut TestAppContext) {
    let (state, cx) = long_patch(cx);
    for _ in 0..50 {
        cx.simulate_event(ScrollEvent {
            position: point(px(100.), px(120.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-40.))),
            ..Default::default()
        });
        draw(cx);
    }
    let top = cx.update(|_, cx| state.read(cx).list().logical_scroll_top().item_ix);
    // 2000 px of wheel travel crosses far more than the first screen of rows.
    assert!(top > 40, "scrolled to row {top}");
}

/// Wraps its lines, so rows are taller than the height the list estimates.
struct WrappedHost {
    state: Entity<DiffState>,
}

impl Render for WrappedHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(400.))
            .h(px(200.))
            .child(Diff::new(&self.state).soft_wrap(true).size_full())
    }
}

#[gpui::test]
fn a_mode_switch_at_the_top_keeps_the_file_header_in_view(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let line = |prefix: &str, ix: usize| {
        format!(
            "{prefix} {ix} {}\n",
            "a long line that wraps in the column ".repeat(3)
        )
    };
    let original = (0..40).map(|ix| line("old", ix)).collect::<String>();
    let modified = (0..40).map(|ix| line("new", ix)).collect::<String>();
    let state =
        cx.new(|cx| DiffState::new([fixture::modified("long.txt", &original, &modified)], cx));
    let (_, cx) = cx.add_window_view({
        let state = state.clone();
        move |_, _| WrappedHost { state }
    });
    for _ in 0..3 {
        draw(cx);
    }
    let top =
        |cx: &mut VisualTestContext| cx.update(|_, cx| state.read(cx).list().logical_scroll_top());
    assert_eq!(top(cx).item_ix, 0, "starts at the file header");

    state.update(cx, |state, cx| state.set_mode(super::DiffMode::Split, cx));
    for _ in 0..3 {
        draw(cx);
    }
    let after = top(cx);
    assert_eq!(
        (after.item_ix, after.offset_in_item),
        (0, px(0.)),
        "the file header stays in view"
    );
}

#[cfg(feature = "test-support")]
fn lefts(cx: &mut VisualTestContext, id: gpui::ElementId) -> Vec<gpui::Pixels> {
    cx.update(|window, _| {
        gpui_base::test_support::find_all(window, &[], &id)
            .into_iter()
            .map(|element| element.bounds().left())
            .collect()
    })
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn split_columns_keep_half_the_viewport_beside_a_long_line(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let original = format!("{}\nshared\n", "long ".repeat(200));
    let modified = "short\nshared\n";
    let state = cx.new(|cx| {
        let mut state = DiffState::new([fixture::modified("long.txt", &original, modified)], cx);
        state.set_mode(super::DiffMode::Split, cx);
        state
    });
    let (_, cx) = cx.add_window_view({
        let state = state.clone();
        move |_, _| Host { state }
    });
    for _ in 0..3 {
        draw(cx);
    }
    let gutters = || gpui::ElementId::from(("line", 0usize));
    let sources = || gpui::ElementId::from("source");
    let gutters_before = lefts(cx, gutters());
    let before = lefts(cx, sources());
    assert_eq!(gutters_before.len(), 2, "one gutter per side");
    assert_eq!(before.len(), 4, "two sides of two lines");
    assert!(
        before.iter().all(|left| *left < px(400.)),
        "both columns start inside the 400px viewport: {before:?}"
    );

    cx.simulate_event(ScrollEvent {
        position: point(px(100.), px(60.)),
        delta: ScrollDelta::Pixels(point(px(-200.), px(0.))),
        ..Default::default()
    });
    for _ in 0..2 {
        draw(cx);
    }
    let offset = cx.update(|_, cx| state.read(cx).horizontal_scroll().offset().x);
    assert!(offset < px(0.), "the long line scrolls: {offset:?}");
    assert_eq!(
        lefts(cx, gutters()),
        gutters_before,
        "the columns stay in place"
    );
    let after = lefts(cx, sources());
    assert!(
        after
            .iter()
            .zip(&before)
            .all(|(after, before)| after < before),
        "the code of both columns scrolls: {before:?} -> {after:?}"
    );
}
