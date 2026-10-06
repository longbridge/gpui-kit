#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;
#[cfg(feature = "gpui-fast")]
extern crate gpui_platform_fast as gpui_platform;

#[path = "../../../motion/mod.rs"]
mod app;

fn main() {
    app::run();
}
