use gpui_kit::component::{ActiveTheme as _, GlobalState, Theme, ThemeMode, menu::AppMenuBar};
use gpui_kit::{App, Entity, Menu, MenuItem, SharedString};

use crate::{
    About, Open, OpenCommandPalette, Quit, SelectLocale,
    themes::{SelectTheme, SwitchThemeMode},
};

pub fn init(title: impl Into<SharedString>, cx: &mut App) -> Entity<AppMenuBar> {
    let app_menu_bar = AppMenuBar::new(cx);
    let title: SharedString = title.into();
    update_app_menu(title.clone(), app_menu_bar.clone(), cx);

    cx.on_action({
        let title = title.clone();
        let app_menu_bar = app_menu_bar.clone();
        move |s: &SelectLocale, cx: &mut App| {
            rust_i18n::set_locale(&s.0.as_str());
            update_app_menu(title.clone(), app_menu_bar.clone(), cx);
        }
    });

    // Observe theme changes to update the menu to refresh the checked state
    cx.observe_global::<Theme>({
        let title = title.clone();
        let app_menu_bar = app_menu_bar.clone();
        move |cx| {
            update_app_menu(title.clone(), app_menu_bar.clone(), cx);
        }
    })
    .detach();

    app_menu_bar
}

fn update_app_menu(title: impl Into<SharedString>, app_menu_bar: Entity<AppMenuBar>, cx: &mut App) {
    let title: SharedString = title.into();

    cx.set_menus(build_menus(title.clone(), cx));
    let menus = build_menus(title, cx)
        .into_iter()
        .map(|menu| menu.owned())
        .collect();
    GlobalState::global_mut(cx).set_app_menus(menus);

    app_menu_bar.update(cx, |menu_bar, cx| {
        menu_bar.reload(cx);
    })
}

fn build_menus(title: impl Into<SharedString>, cx: &App) -> Vec<Menu> {
    vec![
        Menu {
            name: title.into(),
            items: vec![
                MenuItem::action("About", About),
                MenuItem::Separator,
                MenuItem::action("Open...", Open),
                MenuItem::Separator,
                MenuItem::Submenu(Menu {
                    name: "Appearance".into(),
                    items: vec![
                        MenuItem::action("Light", SwitchThemeMode(ThemeMode::Light))
                            .checked(!cx.theme().mode.is_dark()),
                        MenuItem::action("Dark", SwitchThemeMode(ThemeMode::Dark))
                            .checked(cx.theme().mode.is_dark()),
                    ],
                    disabled: false,
                }),
                language_menu(cx),
                MenuItem::Separator,
                MenuItem::action("Quit", Quit),
            ],
            disabled: false,
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("Undo", gpui_kit::component::input::Undo),
                MenuItem::action("Redo", gpui_kit::component::input::Redo),
                MenuItem::separator(),
                MenuItem::action("Cut", gpui_kit::component::input::Cut),
                MenuItem::action("Copy", gpui_kit::component::input::Copy),
                MenuItem::action("Paste", gpui_kit::component::input::Paste),
                MenuItem::separator(),
                MenuItem::action("Delete", gpui_kit::component::input::Delete),
                MenuItem::action(
                    "Delete Previous Word",
                    gpui_kit::component::input::DeleteToPreviousWordStart,
                ),
                MenuItem::action(
                    "Delete Next Word",
                    gpui_kit::component::input::DeleteToNextWordEnd,
                ),
                MenuItem::separator(),
                MenuItem::action("Find", gpui_kit::component::input::Search),
                MenuItem::separator(),
                MenuItem::action("Select All", gpui_kit::component::input::SelectAll),
            ],
            disabled: false,
        },
        Menu {
            name: "Go".into(),
            items: vec![
                MenuItem::action("Go to...", OpenCommandPalette),
                MenuItem::action("Themes...", SelectTheme),
            ],
            disabled: false,
        },
        Menu {
            name: "Help".into(),
            items: vec![
                MenuItem::action("Documentation", Open).disabled(true),
                MenuItem::separator(),
                MenuItem::action("Open Website", Open),
            ],
            disabled: false,
        },
    ]
}

fn language_menu(_: &App) -> MenuItem {
    let locale = rust_i18n::locale().to_string();
    MenuItem::Submenu(Menu {
        name: "Language".into(),
        items: vec![
            MenuItem::action("English", SelectLocale("en".into())).checked(locale == "en"),
            MenuItem::action("简体中文", SelectLocale("zh-CN".into())).checked(locale == "zh-CN"),
            MenuItem::action("Français", SelectLocale("fr".into())).checked(locale == "fr"),
        ],
        disabled: false,
    })
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    // The glob also carries GPUI's `test` attribute; keep the built-in one for `#[test]`.
    use core::prelude::v1::test;
    use gpui_kit::*;

    use super::*;

    #[gpui_kit::test]
    fn build_menus_puts_navigation_actions_in_go_menu(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);

        cx.read(|cx| {
            let menus = build_menus("Story", cx);
            let names: Vec<_> = menus.iter().map(|menu| menu.name.as_ref()).collect();
            assert_eq!(names, ["Story", "Edit", "Go", "Help"]);

            let go_menu = menus.iter().find(|menu| menu.name == "Go").unwrap();
            assert_action(&go_menu.items[0], "Go to...", |action| {
                action.as_any().is::<crate::OpenCommandPalette>()
            });
            assert_action(&go_menu.items[1], "Themes...", |action| {
                action.as_any().is::<SelectTheme>()
            });

            let app_menu = &menus[0];
            assert!(app_menu.items.iter().all(|item| !matches!(
                item,
                MenuItem::Action { action, .. } if action.as_any().is::<SelectTheme>()
            )));
            assert!(menus.iter().all(|menu| menu.name != "Window"));
            assert!(menus.iter().flat_map(|menu| &menu.items).all(|item| !matches!(
                item,
                MenuItem::Action { action, .. } if action.as_any().is::<crate::ToggleSearch>()
            )));
        });
    }

    #[gpui_kit::test]
    fn build_menus_reflects_active_theme_and_locale_checked_states(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| Theme::change(ThemeMode::Dark, None, cx));
        rust_i18n::set_locale("fr");

        cx.read(|cx| {
            let menus = build_menus("Story", cx);
            let app_menu = &menus[0];
            let appearance = submenu(&app_menu.items, "Appearance");
            assert_eq!(appearance.items.len(), 2);
            assert!(!appearance.items[0].is_checked());
            assert!(appearance.items[1].is_checked());

            let language = submenu(&app_menu.items, "Language");
            assert!(!language.items[0].is_checked());
            assert!(!language.items[1].is_checked());
            assert!(language.items[2].is_checked());
        });

        rust_i18n::set_locale("en");
    }

    fn submenu<'a>(items: &'a [MenuItem], name: &str) -> &'a Menu {
        items
            .iter()
            .find_map(|item| match item {
                MenuItem::Submenu(menu) if menu.name == name => Some(menu),
                _ => None,
            })
            .unwrap()
    }

    fn assert_action(
        item: &MenuItem,
        expected_name: &str,
        matches_action: impl FnOnce(&dyn gpui_kit::Action) -> bool,
    ) {
        match item {
            MenuItem::Action { name, action, .. } => {
                assert_eq!(name, expected_name);
                assert!(matches_action(action.as_ref()));
            }
            _ => panic!("expected an action menu item"),
        }
    }
}
