use gpui_kit::{AssetSource, IntoElement, ParentElement, assets::IconName};

gpui_kit::assets::icon_assets!(AppAssets, [Search, Check]);

#[test]
fn assets_are_usable_through_kit_without_component() {
    let _ = gpui_kit::div().child(IconName::Search).into_any_element();
    assert!(AppAssets.load(&IconName::Search.path()).unwrap().is_some());
    assert!(
        AppAssets
            .load(&IconName::Accessibility.path())
            .unwrap()
            .is_none()
    );
}

#[cfg(feature = "component")]
#[test]
fn component_reexports_the_shared_type_and_adapts_it() {
    use gpui_kit::component::{Icon, IconNameExt};
    let name: gpui_kit::component::IconName = IconName::Search;
    let _ = Icon::new(name);
    let _: fn(IconName, &mut gpui_kit::App) -> gpui_kit::Entity<Icon> = IconNameExt::view;
}
