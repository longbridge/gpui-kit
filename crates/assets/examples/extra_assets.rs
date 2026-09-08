use gpui::AssetSource;
gpui_kit_assets::icon_assets!(
    ExtraIcons,
    [
        Accessibility,
        AlarmClock,
        Archive,
        Award,
        Backpack,
        Bike,
        Bird,
        Camera,
        Coffee,
        Compass
    ]
);
struct AppAssets;
impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        if let Some(bytes) = ExtraIcons.load(path)? {
            return Ok(Some(bytes));
        }
        gpui_kit_assets::Assets.load(path)
    }
    fn list(&self, path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        let mut paths = gpui_kit_assets::Assets.list(path)?;
        paths.extend(ExtraIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| gpui_kit_assets::IconName::Accessibility.path().to_string());
    let source: &dyn AssetSource = std::hint::black_box(&AppAssets);
    let bytes = std::hint::black_box(source.load(&path).unwrap());
    println!("{:?}", bytes.map(|bytes| bytes.len()));
}
