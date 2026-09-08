//! `#[gpui_kit::test]` is GPUI's test attribute reached through the umbrella
//! crate; it needs the `test-support` feature.
//!
//! Test modules import what they use rather than `use gpui_kit::*;`: with
//! `test-support` on, the glob also brings in GPUI's `test` attribute, which
//! would shadow the built-in `#[test]` the expansion relies on.

use gpui_kit::{AppContext as _, TestAppContext};

struct Counter(u32);

#[gpui_kit::test]
fn entity_round_trip(cx: &mut TestAppContext) {
    let counter = cx.new(|_| Counter(1));
    counter.update(cx, |counter, _| counter.0 += 1);
    assert_eq!(counter.read_with(cx, |counter, _| counter.0), 2);
}

#[gpui_kit::test]
async fn async_test_runs(cx: &mut TestAppContext) {
    let counter = cx.new(|_| Counter(0));
    cx.background_executor.run_until_parked();
    assert_eq!(counter.read_with(cx, |counter, _| counter.0), 0);
}

#[test]
fn ordinary_rust_test_keeps_the_builtin_attribute() {
    assert_eq!(
        gpui_kit::size(gpui_kit::px(12.), gpui_kit::px(24.)).width,
        gpui_kit::px(12.)
    );
}
