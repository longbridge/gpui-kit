use gpui::AssetSource;
fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "icons/search.svg".into());
    // Keep the bytes observable so release size measurements include the SVG payload.
    println!(
        "{}",
        std::hint::black_box(gpui_kit_assets::AllAssets.load(&path).unwrap().unwrap()).len()
    );
}
