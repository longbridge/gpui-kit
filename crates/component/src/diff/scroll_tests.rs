use gpui::{
    AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, ScrollDelta,
    ScrollEvent, Styled as _, TestAppContext, VisualTestContext, Window, div, point, px,
};

use super::{Diff, DiffLinePosition, DiffSide, DiffState, document::fixture};

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

#[gpui::test]
fn a_revealed_line_lands_below_the_pinned_header(cx: &mut TestAppContext) {
    let (state, cx) = long_patch(cx);
    let position = DiffLinePosition::new("long.txt", DiffSide::Modified, 150);
    state.update(cx, |state, cx| state.scroll_to_line(position.clone(), cx));
    for _ in 0..3 {
        draw(cx);
    }
    let (gap, row_height) = cx.update(|_, cx| {
        let state = state.read(cx);
        let row = state.source_row(&position).expect("line has a row");
        let list = state.list();
        let bounds = list.bounds_for_item(row).expect("target row is rendered");
        (
            bounds.top() - list.viewport_bounds().top(),
            bounds.size.height,
        )
    });
    // Flush to the top the line would sit under the pinned header overlay.
    assert!(
        gap >= row_height,
        "line {gap:?} below the viewport top, rows {row_height:?} tall"
    );
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

#[cfg(feature = "test-support")]
#[gpui::test]
fn file_headers_stay_in_the_viewport_under_horizontal_scrolling(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let original = format!("{}\nshared\n", "long ".repeat(200));
    let modified = "short\nshared\n";
    for mode in [super::DiffMode::Unified, super::DiffMode::Split] {
        let state = cx.new(|cx| {
            let mut state =
                DiffState::new([fixture::modified("long.txt", &original, modified)], cx);
            state.set_mode(mode, cx);
            state
        });
        let (_, cx) = cx.add_window_view({
            let state = state.clone();
            move |_, _| Host { state }
        });
        for _ in 0..3 {
            draw(cx);
        }
        let header = |cx: &mut VisualTestContext| {
            cx.update(|window, _| {
                gpui_base::test_support::find(window, &[], &"file-header".into())
                    .expect("the file header is rendered")
                    .bounds()
            })
        };
        let before = header(cx);
        assert!(
            before.left() >= px(0.) && before.right() <= px(400.),
            "{mode:?}: the header fits the 400px viewport: {before:?}"
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
        assert!(
            offset < px(0.),
            "{mode:?}: the long line scrolls: {offset:?}"
        );
        assert_eq!(header(cx), before, "{mode:?}: the header stays in place");
    }
}

#[test]
fn pinned_file_follows_the_viewport_top() {
    use gpui::ListOffset;

    use super::state::DisplayRow;
    let code = |file| DisplayRow::Code {
        file,
        original: Some(0),
        modified: Some(0),
        changed: false,
    };
    let rows = [
        DisplayRow::File(0),
        code(0),
        code(0),
        DisplayRow::File(1),
        code(1),
    ];
    let headers = [0, 3];
    let at = |item_ix, offset: f32| ListOffset {
        item_ix,
        offset_in_item: px(offset),
    };
    // The list at rest shows file 0's own header row; nothing is pinned.
    assert_eq!(super::pinned_file(&rows, &headers, at(0, 0.)), None);
    // Any scroll into the file carries its header out; the pin takes over.
    assert_eq!(super::pinned_file(&rows, &headers, at(0, 4.)), Some(0));
    assert_eq!(super::pinned_file(&rows, &headers, at(2, 0.)), Some(0));
    // The moment the next header reaches the top it stands inline, unpinned.
    assert_eq!(super::pinned_file(&rows, &headers, at(3, 0.)), None);
    assert_eq!(super::pinned_file(&rows, &headers, at(3, 2.)), Some(1));
    // A stale offset past the rows pins nothing.
    assert_eq!(super::pinned_file(&rows, &headers, at(9, 0.)), None);
}

#[gpui::test]
fn collapsing_the_scrolled_file_shows_its_header(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let text = |tag: &str| {
        (0..LINES)
            .map(|ix| format!("{tag} {ix}\n"))
            .collect::<String>()
    };
    let state = cx.new(|cx| {
        DiffState::new(
            [
                fixture::modified("first.txt", &text("old"), &text("new")),
                fixture::modified("second.txt", &text("older"), &text("newer")),
            ],
            cx,
        )
    });
    let (_, cx) = cx.add_window_view({
        let state = state.clone();
        move |_, _| Host { state }
    });
    for _ in 0..3 {
        draw(cx);
    }
    let position = DiffLinePosition::new("first.txt", DiffSide::Modified, 150);
    state.update(cx, |state, cx| state.scroll_to_line(position, cx));
    draw(cx);
    // Collapsing from deep inside the file, where only the pinned bar shows
    // its header, must not strand the viewport on the next file.
    state.update(cx, |state, cx| state.toggle_file_collapsed(0, cx));
    draw(cx);
    let (top, header) = cx.update(|_, cx| {
        let state = state.read(cx);
        (
            state.list().logical_scroll_top().item_ix,
            state.header_rows()[0],
        )
    });
    assert_eq!(top, header, "the collapsed file's header tops the viewport");
}
