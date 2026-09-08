#![cfg(not(target_family = "wasm"))]

use gpui::{AssetSource, IntoElement};
use gpui_kit_assets::{Assets, IconName};
use std::collections::BTreeSet;

#[test]
fn every_named_icon_is_available_in_the_asset_source() {
    let assets = Assets;
    let named: BTreeSet<_> = IconName::ALL.iter().map(|icon| icon.path()).collect();
    let bundled: BTreeSet<_> = assets.list("icons/").unwrap().into_iter().collect();
    assert_eq!(named, bundled);
    for path in named {
        let bytes = assets.load(&path).unwrap().unwrap();
        assert!(
            std::str::from_utf8(&bytes).unwrap().contains("<svg"),
            "{path}"
        );
    }
}

#[test]
fn shared_names_work_without_component() {
    assert_eq!(IconName::Accessibility.path(), "icons/accessibility.svg");
    assert_eq!(IconName::ALargeSmall.path(), "icons/a-large-small.svg");
    let _ = IconName::Search.into_any_element();
    let _: gpui::AnyElement = IconName::Search.into();
}

gpui_kit_assets::icon_assets!(SelectedAssets, [Search, Check]);

#[test]
fn selected_assets_only_expose_requested_icons() {
    let assets = SelectedAssets;
    assert_eq!(
        assets.list("icons/").unwrap(),
        vec!["icons/search.svg", "icons/check.svg"]
    );
    assert_eq!(assets.list("icons/se").unwrap(), vec!["icons/search.svg"]);
    for name in [IconName::Search, IconName::Check] {
        let selected = assets.load(&name.path()).unwrap().unwrap();
        assert!(matches!(selected, std::borrow::Cow::Borrowed(_)));
        assert_eq!(Some(selected), Assets.load(&name.path()).unwrap());
    }
    assert!(assets.load("").unwrap().is_none());
    assert!(assets.load("icons/accessibility.svg").unwrap().is_none());
}

gpui_kit_assets::icon_assets!(pub EmptyAssets, []);

#[test]
fn empty_selection_is_a_valid_asset_source() {
    assert!(EmptyAssets.list("").unwrap().is_empty());
    assert!(EmptyAssets.load("icons/search.svg").unwrap().is_none());
}
