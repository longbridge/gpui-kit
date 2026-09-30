// GPUI's macros expand to `gpui::` paths and this crate depends on gpui-kit alone:
// alias the Kit as `gpui` so they resolve on any GPUI (see `gpui_kit`'s lib.rs).
extern crate gpui_kit as gpui;

fn main() {
    gpui_kit_recipes::bootstrap::run();
}
