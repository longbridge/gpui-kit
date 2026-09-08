//! The normal Kit glob import supports upstream GPUI test syntax alongside
//! ordinary Rust tests, without importing or shadowing a test attribute.
use gpui_kit::*;

struct Counter(u32);

#[gpui::test]
fn entity_round_trip(cx: &mut TestAppContext) {
    let counter = cx.new(|_| Counter(1));
    counter.update(cx, |counter, _| counter.0 += 1);
    assert_eq!(counter.read_with(cx, |counter, _| counter.0), 2);
}

#[gpui::test]
async fn async_test_runs(cx: &mut TestAppContext) {
    let counter = cx.new(|_| Counter(0));
    cx.background_executor.run_until_parked();
    assert_eq!(counter.read_with(cx, |counter, _| counter.0), 0);
}

#[test]
fn ordinary_rust_test_keeps_the_builtin_attribute() {
    assert_eq!(size(px(12.), px(24.)).width, px(12.));
}
