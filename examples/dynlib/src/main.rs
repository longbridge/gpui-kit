//! Headless loader smoke test: run with `--features dynlib`.
fn main() {
    let text = gpui_kit::SharedString::from(String::from("GPUI Kit dynamic linking"));
    assert_eq!(text.as_ref(), "GPUI Kit dynamic linking");
    println!("{text}");
}
