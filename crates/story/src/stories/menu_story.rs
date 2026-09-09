use gpui_kit::component::{
    ActiveTheme as _, IconName, Side, StyledExt,
    button::Button,
    h_flex,
    menu::{ContextMenuExt, DropdownMenu as _, PopupMenuItem},
    v_flex,
};
use gpui_kit::*;
use serde::Deserialize;

use crate::section;

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = menu_story, no_json)]
struct Info(usize);

actions!(menu_story, [Copy, Paste, Cut, SearchAll, ToggleCheck]);

const CONTEXT: &str = "menu_story";
pub fn init(cx: &mut App) {
    cx.bind_keys([
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-v", Paste, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-v", Paste, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-x", Cut, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-x", Cut, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-f", SearchAll, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-f", SearchAll, Some(CONTEXT)),
        KeyBinding::new("ctrl-shift-alt-t", ToggleCheck, Some(CONTEXT)),
    ])
}

pub struct MenuStory {
    check_side: Option<Side>,
    message: String,
}

impl super::Story for MenuStory {
    fn title() -> &'static str {
        "Menu"
    }

    fn description() -> &'static str {
        "Popup menu and context menu"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl MenuStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self {
            check_side: None,
            message: "".to_string(),
        }
    }

    fn on_copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked copy".to_string();
        cx.notify()
    }

    fn on_cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked cut".to_string();
        cx.notify()
    }

    fn on_paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked paste".to_string();
        cx.notify()
    }

    fn on_search_all(&mut self, _: &SearchAll, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked search all".to_string();
        cx.notify()
    }

    fn on_action_info(&mut self, info: &Info, _: &mut Window, cx: &mut Context<Self>) {
        self.message = format!("You have clicked info: {}", info.0);
        cx.notify()
    }

    fn on_action_toggle_check(&mut self, _: &ToggleCheck, _: &mut Window, cx: &mut Context<Self>) {
        self.check_side = if self.check_side == Some(Side::Left) {
            Some(Side::Right)
        } else if self.check_side == Some(Side::Right) {
            None
        } else {
            Some(Side::Left)
        };

        self.message = format!("You have used check at side: {:?}", self.check_side);
        cx.notify()
    }
}

impl Render for MenuStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let check_side = self.check_side;
        let view = cx.entity();

        v_flex()
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_search_all))
            .on_action(cx.listener(Self::on_action_info))
            .on_action(cx.listener(Self::on_action_toggle_check))
            .size_full()
            .min_h(px(400.))
            .items_center()
            .gap_6()
            .child(
                section("Popup Menu")
                    .description(
                        "Supports actions, links, checks, icons, custom rows, and nested menus.",
                    )
                    .w(px(640.))
                    .child(
                        Button::new("popup-menu-1")
                            .outline()
                            .label("Edit")
                            .dropdown_menu(move |this, window, cx| {
                                this.min_w(250.)
                                    .link("About", "https://github.com/longbridge/gpui-kit")
                                    .check_side(check_side.unwrap_or(Side::Left))
                                    .separator()
                                    .item(PopupMenuItem::new("Handle Click").on_click(
                                        window.listener_for(&view, |this, _, _, cx| {
                                            this.message =
                                                "You have clicked Handle Click".to_string();
                                            cx.notify();
                                        }),
                                    ))
                                    .separator()
                                    .menu("Copy", Box::new(Copy))
                                    .menu("Cut", Box::new(Cut))
                                    .menu("Paste", Box::new(Paste))
                                    .separator()
                                    .menu_with_check(
                                        format!("Check Side {:?}", check_side),
                                        check_side.is_some(),
                                        Box::new(ToggleCheck),
                                    )
                                    .separator()
                                    .menu_with_icon("Search", IconName::Search, Box::new(SearchAll))
                                    .separator()
                                    .item(
                                        PopupMenuItem::element(|_, cx| {
                                            v_flex().child("Custom Element").child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("This is sub-title"),
                                            )
                                        })
                                        .on_click(
                                            window.listener_for(&view, |this, _, _, cx| {
                                                this.message = "You have clicked on custom element"
                                                    .to_string();
                                                cx.notify();
                                            }),
                                        ),
                                    )
                                    .menu_element_with_check(
                                        check_side.is_some(),
                                        Box::new(ToggleCheck),
                                        |_, cx| {
                                            h_flex().gap_1().child("Custom Element").child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("checked"),
                                            )
                                        },
                                    )
                                    .menu_element_with_icon(
                                        IconName::Info,
                                        Box::new(Info(0)),
                                        |_, cx| {
                                            h_flex().gap_1().child("Custom").child(
                                                div()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("element"),
                                            )
                                        },
                                    )
                                    .separator()
                                    .menu_with_disabled("Disabled Item", Box::new(Info(0)), true)
                                    .separator()
                                    .submenu("Links", window, cx, |menu, _, _| {
                                        menu.link_with_icon(
                                            "GPUI Component",
                                            IconName::Github,
                                            "https://github.com/longbridge/gpui-kit",
                                        )
                                        .separator()
                                        .link("GPUI Kit", "https://gpui-kit.com")
                                        .link("Zed", "https://zed.dev")
                                    })
                                    .separator()
                                    .submenu("Other Links", window, cx, |menu, window, cx| {
                                        menu.link("Crates", "https://crates.io")
                                            .link("Rust Docs", "https://docs.rs")
                                            .separator()
                                            .submenu("Nested", window, cx, |menu, window, cx| {
                                                menu.link("Docs.rs", "https://docs.rs")
                                                    .separator()
                                                    .submenu("Deeper", window, cx, |menu, _, _| {
                                                        menu.link(
                                                            "GPUI Kit",
                                                            "https://gpui-kit.com",
                                                        )
                                                    })
                                            })
                                    })
                            }),
                    )
                    .child(self.message.clone()),
            )
            .child(
                section("Context Menu")
                    .description("Different regions can provide their own right-click actions.")
                    .w(px(640.))
                    .v_flex()
                    .gap_4()
                    .child(
                        v_flex()
                            .w_full()
                            .p_4()
                            .items_center()
                            .justify_center()
                            .min_h_20()
                            .rounded(cx.theme().radius_lg)
                            .border_2()
                            .border_dashed()
                            .border_color(cx.theme().border)
                            .child("Right click to open ContextMenu")
                            .context_menu({
                                move |this, window, cx| {
                                    this.check_side(check_side.unwrap_or(Side::Left))
                                        .external_link_icon(false)
                                        .link("About", "https://github.com/longbridge/gpui-kit")
                                        .separator()
                                        .menu("Cut", Box::new(Cut))
                                        .menu("Copy", Box::new(Copy))
                                        .menu("Paste", Box::new(Paste))
                                        .separator()
                                        .label("This is a label")
                                        .menu_with_check(
                                            format!("Check Side {:?}", check_side),
                                            check_side.is_some(),
                                            Box::new(ToggleCheck),
                                        )
                                        .separator()
                                        // Deeply nested submenus to verify each
                                        // level paints above the shallower ones and
                                        // the background content behind them.
                                        .submenu("Settings", window, cx, move |menu, window, cx| {
                                            menu.menu("Info 0", Box::new(Info(0)))
                                                .separator()
                                                .menu("Item 1", Box::new(Info(1)))
                                                .menu("Item 2", Box::new(Info(2)))
                                                .separator()
                                                .submenu(
                                                    "More",
                                                    window,
                                                    cx,
                                                    move |menu, window, cx| {
                                                        menu.menu("More Item 1", Box::new(Info(1)))
                                                            .menu("More Item 2", Box::new(Info(2)))
                                                            .separator()
                                                            .submenu(
                                                                "Even More",
                                                                window,
                                                                cx,
                                                                move |menu, window, cx| {
                                                                    menu.menu(
                                                                        "Deep Item 1",
                                                                        Box::new(Info(1)),
                                                                    )
                                                                    .menu(
                                                                        "Deep Item 2",
                                                                        Box::new(Info(2)),
                                                                    )
                                                                    .separator()
                                                                    .submenu(
                                                                        "Deepest",
                                                                        window,
                                                                        cx,
                                                                        move |menu, _, _| {
                                                                            menu.menu(
                                                                                "Leaf 1",
                                                                                Box::new(Info(1)),
                                                                            )
                                                                            .menu(
                                                                                "Leaf 2",
                                                                                Box::new(Info(2)),
                                                                            )
                                                                        },
                                                                    )
                                                                },
                                                            )
                                                    },
                                                )
                                        })
                                        .separator()
                                        .menu("Search All", Box::new(SearchAll))
                                        .separator()
                                }
                            })
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(
                                        "You can right click anywhere in \
                                         this area to open the context menu.",
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("other")
                            .flex()
                            .w_full()
                            .p_4()
                            .items_center()
                            .justify_center()
                            .min_h_20()
                            .rounded(cx.theme().radius_lg)
                            .border_2()
                            .border_dashed()
                            .border_color(cx.theme().border)
                            .child("Here is another area with context menu.")
                            .context_menu({
                                move |this, _, _| {
                                    this.link("About", "https://github.com/longbridge/gpui-kit")
                                        .separator()
                                        .menu("Item 1", Box::new(Info(1)))
                                }
                            }),
                    )
                    .child(
                        div()
                            .id("other1")
                            .flex()
                            .w_full()
                            .p_4()
                            .items_center()
                            .justify_center()
                            .min_h_20()
                            .rounded(cx.theme().radius_lg)
                            .border_2()
                            .border_dashed()
                            .border_color(cx.theme().border)
                            .child("ContextMenu area 1")
                            .context_menu({
                                move |this, _, _| {
                                    this.link("About", "https://github.com/longbridge/gpui-kit")
                                        .separator()
                                        .menu("Item 1", Box::new(Info(1)))
                                }
                            }),
                    ),
            )
            .child(
                section("Scrollable")
                    .description(
                        "Long menus constrain their height while short menus stay compact.",
                    )
                    .w(px(640.))
                    .child(
                        Button::new("dropdown-menu-scrollable-1")
                            .outline()
                            .label("Scrollable Menu (100 items)")
                            .dropdown_menu_with_anchor(Anchor::TopRight, move |this, _, _| {
                                let mut this = this
                                    .scrollable(true)
                                    .max_h(px(300.))
                                    .label(format!("Total {} items", 100));
                                for i in 0..100 {
                                    if i % 5 == 0 {
                                        this = this.separator();
                                    }

                                    this = this.menu(
                                        SharedString::from(format!("Item {}", i)),
                                        Box::new(Info(i)),
                                    )
                                }
                                this.min_w(px(100.))
                            }),
                    )
                    .child(
                        Button::new("dropdown-menu-scrollable-2")
                            .outline()
                            .label("Scrollable Menu (5 items)")
                            .dropdown_menu_with_anchor(Anchor::TopRight, move |this, _, _| {
                                let mut this = this
                                    .scrollable(true)
                                    .max_h(px(300.))
                                    .label(format!("Total {} items", 100));
                                for i in 0..5 {
                                    this = this.menu(
                                        SharedString::from(format!("Item {}", i)),
                                        Box::new(Info(i)),
                                    )
                                }
                                this.min_w(px(100.))
                            }),
                    ),
            )
    }
}
