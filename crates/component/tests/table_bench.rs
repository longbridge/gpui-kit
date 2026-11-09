//! Headless frame-cost benchmark for `DataTable`.
//!
//! Measures `Window::draw` time and heap allocations per frame for a
//! `DataTable` and, as a baseline, a plain `uniform_list` table with the same
//! cells. The tests are ignored by default; run them in release mode:
//!
//! ```sh
//! cargo test --release -p gpui-component --test table_bench -- --ignored --nocapture --test-threads=1
//! cargo test --release -p gpui-component --features gpui-fast --test table_bench -- --ignored --nocapture --test-threads=1
//! ```
//!
//! This binary installs a counting global allocator, so it is kept apart from
//! the integration test binary.

#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;

use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use gpui::{
    AnyWindowHandle, App, AppContext as _, Context, Entity, IntoElement, ParentElement as _,
    Render, SharedString, Styled as _, TestAppContext, Window, div, point, px, size, uniform_list,
};
use gpui_component::table::{Column, DataTable, TableDelegate, TableState};

struct CountingAlloc;

static ALLOCS: AtomicU64 = AtomicU64::new(0);
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

const COLUMNS: usize = 12;
const COL_WIDTH: f32 = 120.;
const ROW_HEIGHT: f32 = 32.;
const WARMUP: usize = 20;
const FRAMES: usize = 300;

fn cell_text(row_ix: usize, col_ix: usize) -> SharedString {
    format!("r{row_ix} c{col_ix}").into()
}

struct BenchDelegate {
    rows: usize,
}

impl TableDelegate for BenchDelegate {
    fn columns_count(&self, _: &App) -> usize {
        COLUMNS
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        Column::new(format!("c{col_ix}"), format!("Column {col_ix}")).width(px(COL_WIDTH))
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        cell_text(row_ix, col_ix)
    }
}

struct DataTableView {
    table: Entity<TableState<BenchDelegate>>,
}

impl Render for DataTableView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(DataTable::new(&self.table))
    }
}

/// The baseline: a hand-written table with the same cells and no interaction.
struct PlainTableView {
    rows: usize,
    scroll_handle: gpui::UniformListScrollHandle,
}

impl Render for PlainTableView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            uniform_list("plain-table", self.rows, |range, _, _| {
                range
                    .map(|row_ix| {
                        div()
                            .flex()
                            .flex_row()
                            .h(px(ROW_HEIGHT))
                            .children((0..COLUMNS).map(|col_ix| {
                                div()
                                    .w(px(COL_WIDTH))
                                    .flex_shrink_0()
                                    .px_2()
                                    .child(cell_text(row_ix, col_ix))
                            }))
                    })
                    .collect()
            })
            .size_full()
            .track_scroll(&self.scroll_handle),
        )
    }
}

#[derive(Clone, Copy)]
enum Scenario {
    /// Redraw without any change.
    Stationary,
    /// Move one row down each frame.
    ScrollVertical,
    /// Move 37px right each frame.
    ScrollHorizontal,
}

struct Stats {
    median: Duration,
    p95: Duration,
    allocs: u64,
    bytes: u64,
}

fn measure(
    cx: &mut TestAppContext,
    window: AnyWindowHandle,
    mut before_frame: impl FnMut(usize, &mut Window, &mut App),
) -> Stats {
    let mut times = Vec::with_capacity(FRAMES);
    let mut allocs = 0;
    let mut bytes = 0;
    for frame in 0..WARMUP + FRAMES {
        cx.update_window(window, |_, window, cx| {
            before_frame(frame, window, cx);
            window.refresh();
            let a0 = ALLOCS.load(Ordering::Relaxed);
            let b0 = ALLOC_BYTES.load(Ordering::Relaxed);
            let start = Instant::now();
            window.draw(cx).clear(cx);
            let elapsed = start.elapsed();
            if frame >= WARMUP {
                times.push(elapsed);
                allocs += ALLOCS.load(Ordering::Relaxed) - a0;
                bytes += ALLOC_BYTES.load(Ordering::Relaxed) - b0;
            }
        })
        .unwrap();
    }
    times.sort();
    Stats {
        median: times[times.len() / 2],
        p95: times[times.len() * 95 / 100],
        allocs: allocs / FRAMES as u64,
        bytes: bytes / FRAMES as u64,
    }
}

fn report(name: &str, rows: usize, scenario: Scenario, stats: Stats) {
    let scenario = match scenario {
        Scenario::Stationary => "stationary",
        Scenario::ScrollVertical => "scroll-y",
        Scenario::ScrollHorizontal => "scroll-x",
    };
    println!(
        "| {name} | {rows} | {scenario} | {:.1} | {:.1} | {} | {:.1} |",
        stats.median.as_secs_f64() * 1e6,
        stats.p95.as_secs_f64() * 1e6,
        stats.allocs,
        stats.bytes as f64 / 1024.,
    );
}

fn new_window<V: Render>(
    cx: &mut TestAppContext,
    build: impl FnOnce(&mut Window, &mut Context<V>) -> V + 'static,
) -> gpui::WindowHandle<V> {
    let window = cx.add_window(build);
    cx.simulate_window_resize(window.into(), size(px(1280.), px(800.)));
    cx.run_until_parked();
    window
}

fn bench_data_table(cx: &mut TestAppContext, rows: usize, scenario: Scenario) {
    let window = new_window(cx, move |window, cx| {
        let table = cx.new(|cx| TableState::new(BenchDelegate { rows }, window, cx));
        DataTableView { table }
    });
    let table = window.update(cx, |view, _, _| view.table.clone()).unwrap();
    let max_row = rows.saturating_sub(30).max(1);
    let stats = measure(cx, window.into(), |frame, _, cx| {
        table.update(cx, |table, cx| match scenario {
            Scenario::Stationary => cx.notify(),
            Scenario::ScrollVertical => {
                let y = (frame % max_row) as f32 * ROW_HEIGHT;
                table
                    .vertical_scroll_handle
                    .0
                    .borrow()
                    .base_handle
                    .set_offset(point(px(0.), px(-y)));
                cx.notify();
            }
            Scenario::ScrollHorizontal => {
                table.scroll_to_col(frame % COLUMNS, cx);
                cx.notify();
            }
        });
    });
    report("DataTable", rows, scenario, stats);
}

fn bench_plain_table(cx: &mut TestAppContext, rows: usize, scenario: Scenario) {
    let window = new_window(cx, move |_, _| PlainTableView {
        rows,
        scroll_handle: gpui::UniformListScrollHandle::new(),
    });
    let view = window.update(cx, |_, _, cx| cx.entity()).unwrap();
    let max_row = rows.saturating_sub(30).max(1);
    let stats = measure(cx, window.into(), |frame, _, cx| {
        view.update(cx, |view, cx| {
            if let Scenario::ScrollVertical = scenario {
                let y = (frame % max_row) as f32 * ROW_HEIGHT;
                view.scroll_handle
                    .0
                    .borrow()
                    .base_handle
                    .set_offset(point(px(0.), px(-y)));
            }
            cx.notify();
        });
    });
    report("uniform_list", rows, scenario, stats);
}

fn header() {
    println!("| table | rows | scenario | median us | p95 us | allocs/frame | KiB/frame |");
    println!("|---|---|---|---|---|---|---|");
}

#[gpui::test]
#[ignore = "benchmark; run with --release --ignored --nocapture"]
fn bench_table_frames(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    header();
    for rows in [20, 2000] {
        for scenario in [
            Scenario::Stationary,
            Scenario::ScrollVertical,
            Scenario::ScrollHorizontal,
        ] {
            bench_data_table(cx, rows, scenario);
        }
        for scenario in [Scenario::Stationary, Scenario::ScrollVertical] {
            bench_plain_table(cx, rows, scenario);
        }
    }
}
