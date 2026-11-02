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
fn component_accepts_shared_names_and_preserves_legacy_views() {
    use gpui_kit::component::{Icon, IconNameExt};
    use gpui_kit::component::{IconName as LegacyIconName, IconNamed};
    let _: fn(LegacyIconName, &mut gpui_kit::App) -> gpui_kit::Entity<Icon> = LegacyIconName::view;
    assert_eq!(LegacyIconName::Search.path(), "icons/search.svg");
    let _: gpui_kit::AnyElement = LegacyIconName::Search.into();
    let shared: IconName = LegacyIconName::Search.into();
    assert_eq!(shared, IconName::Search);
    let _ = Icon::new(shared);
    let _ = Icon::new(IconName::Accessibility);
    let _ = Icon::new(LegacyIconName::Search);
    let _: fn(IconName, &mut gpui_kit::App) -> gpui_kit::Entity<Icon> = IconNameExt::view;
}
