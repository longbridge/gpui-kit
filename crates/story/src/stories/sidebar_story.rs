use std::collections::HashMap;

use gpui_kit::component::red_500;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use gpui_kit::component::{
    ActiveTheme, Icon, IconName, Side, Sizable, StyledExt, ThemeStyled as _,
    badge::Badge,
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    button::Button,
    h_flex,
    menu::DropdownMenu,
    separator::Separator,
    sidebar::{
        Sidebar, SidebarCollapsible, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu,
        SidebarMenuItem, SidebarToggleButton,
    },
    switch::Switch,
    v_flex,
};
use serde::Deserialize;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sidebar_story, no_json)]
pub struct SelectCompany(SharedString);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sidebar_story, no_json)]
enum SidebarOption {
    Icon,
    Offcanvas,
    None,
    Right,
    ClickToOpen,
    DynamicChildren,
}

pub struct SidebarStory {
    active_items: HashMap<Item, bool>,
    last_active_item: Item,
    active_subitem: Option<SubItem>,
    collapsed: bool,
    collapsible: SidebarCollapsible,
    side: Side,
    click_to_open_submenu: bool,
    show_dynamic_children: bool,
    focus_handle: gpui_kit::FocusHandle,
    checked: bool,
}

impl SidebarStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut active_items = HashMap::new();
        active_items.insert(Item::Playground, true);

        Self {
            active_items,
            last_active_item: Item::Playground,
            active_subitem: None,
            collapsed: false,
            collapsible: SidebarCollapsible::Icon,
            side: Side::Left,
            focus_handle: cx.focus_handle(),
            checked: false,
            click_to_open_submenu: false,
            show_dynamic_children: false,
        }
    }

    fn render_content(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .min_h_0()
            .flex_1()
            .gap_4()
            .child(
                h_flex()
                    .w_full()
                    .items_start()
                    .justify_between()
                    .gap_4()
                    .child(
                        v_flex()
                            .min_w_0()
                            .gap_1()
                            .child(
                                div()
                                    .text_2xl()
                                    .font_semibold()
                                    .child(self.last_active_item.label()),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("A quick view of your workspace activity."),
                            ),
                    )
                    .child(
                        crate::story_toolbar_group()
                            .w_auto()
                            .flex_shrink_0()
                            .dropdown_child(Button::new("sidebar-options").label("Options"), {
                                let collapsible = self.collapsible;
                                let right = self.side.is_right();
                                let click_to_open = self.click_to_open_submenu;
                                let dynamic_children = self.show_dynamic_children;
                                move |menu, _, _| {
                                    menu.menu_with_check(
                                        "Icon mode",
                                        collapsible == SidebarCollapsible::Icon,
                                        Box::new(SidebarOption::Icon),
                                    )
                                    .menu_with_check(
                                        "Offcanvas mode",
                                        collapsible == SidebarCollapsible::Offcanvas,
                                        Box::new(SidebarOption::Offcanvas),
                                    )
                                    .menu_with_check(
                                        "Fixed mode",
                                        collapsible == SidebarCollapsible::None,
                                        Box::new(SidebarOption::None),
                                    )
                                    .separator()
                                    .menu_with_check(
                                        "Right Side",
                                        right,
                                        Box::new(SidebarOption::Right),
                                    )
                                    .menu_with_check(
                                        "Click to Open",
                                        click_to_open,
                                        Box::new(SidebarOption::ClickToOpen),
                                    )
                                    .menu_with_check(
                                        "Dynamic Children",
                                        dynamic_children,
                                        Box::new(SidebarOption::DynamicChildren),
                                    )
                                }
                            }),
                    ),
            )
            .child(
                h_flex().w_full().gap_3().children(
                    [
                        ("Active projects", "12", "+2 this week"),
                        ("Team members", "28", "4 online"),
                        ("Tasks completed", "84%", "+6% this month"),
                    ]
                    .into_iter()
                    .map(|(label, value, detail)| {
                        v_flex()
                            .min_w_0()
                            .flex_1()
                            .gap_2()
                            .p_4()
                            .rounded(cx.theme().radius_lg)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(label),
                            )
                            .child(div().text_2xl().font_semibold().child(value))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(detail),
                            )
                    }),
                ),
            )
            .child(
                v_flex()
                    .w_full()
                    .min_h_0()
                    .flex_1()
                    .mt_2()
                    .rounded(cx.theme().radius_lg)
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .px_4()
                            .py_2()
                            .child(div().font_medium().child("Recent activity"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Today"),
                            ),
                    )
                    .child(Separator::horizontal())
                    .children(
                        [
                            (
                                IconName::CircleCheck,
                                "Design review completed",
                                "12 minutes ago",
                            ),
                            (IconName::File, "Project brief updated", "1 hour ago"),
                            (
                                IconName::CircleUser,
                                "Maya joined the workspace",
                                "3 hours ago",
                            ),
                        ]
                        .into_iter()
                        .map(|(icon, title, time)| {
                            h_flex()
                                .items_center()
                                .gap_3()
                                .px_4()
                                .py_3()
                                .child(
                                    div()
                                        .size_8()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_full_style(cx)
                                        .bg(cx.theme().muted)
                                        .child(Icon::new(icon).small()),
                                )
                                .child(
                                    v_flex()
                                        .min_w_0()
                                        .flex_1()
                                        .gap_0p5()
                                        .child(div().text_sm().child(title))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(time),
                                        ),
                                )
                        }),
                    ),
            )
    }

    fn switch_checked_handler(
        &mut self,
        checked: &bool,
        _: &mut Window,
        _: &mut Context<SidebarStory>,
    ) {
        self.checked = *checked;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Item {
    Playground,
    Models,
    Documentation,
    Settings,
    DesignEngineering,
    SalesAndMarketing,
    Travel,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubItem {
    History,
    Starred,
    General,
    Team,
    Billing,
    Limits,
    Settings,
    Genesis,
    Explorer,
    Quantum,
    Introduction,
    GetStarted,
    Tutorial,
    Changelog,
}

impl Item {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Playground => "Playground",
            Self::Models => "Models",
            Self::Documentation => "Documentation",
            Self::Settings => "Settings",
            Self::DesignEngineering => "Design Engineering",
            Self::SalesAndMarketing => "Sales and Marketing",
            Self::Travel => "Travel",
        }
    }

    pub fn is_disabled(&self) -> bool {
        match self {
            Self::Travel => true,
            _ => false,
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            Self::Playground => IconName::SquareTerminal,
            Self::Models => IconName::Bot,
            Self::Documentation => IconName::BookOpen,
            Self::Settings => IconName::Settings2,
            Self::DesignEngineering => IconName::Frame,
            Self::SalesAndMarketing => IconName::ChartPie,
            Self::Travel => IconName::Map,
        }
    }

    pub fn handler(
        &self,
    ) -> impl Fn(&mut SidebarStory, &ClickEvent, &mut Window, &mut Context<SidebarStory>) + 'static
    {
        let item = *self;
        move |this, _, _, cx| {
            if this.active_items.contains_key(&item) {
                this.active_items.remove(&item);
            } else {
                this.active_items.insert(item, true);
            }

            this.last_active_item = item;
            this.active_subitem = None;
            cx.notify();
        }
    }

    pub fn items(&self) -> Vec<SubItem> {
        match self {
            Self::Playground => vec![SubItem::History, SubItem::Starred, SubItem::Settings],
            Self::Models => vec![SubItem::Genesis, SubItem::Explorer, SubItem::Quantum],
            Self::Documentation => vec![
                SubItem::Introduction,
                SubItem::GetStarted,
                SubItem::Tutorial,
                SubItem::Changelog,
            ],
            Self::Settings => vec![
                SubItem::General,
                SubItem::Team,
                SubItem::Billing,
                SubItem::Limits,
            ],
            _ => Vec::new(),
        }
    }
}

impl SubItem {
    pub fn label(&self) -> &'static str {
        match self {
            Self::History => "History",
            Self::Starred => "Starred",
            Self::Settings => "Settings",
            Self::Genesis => "Genesis",
            Self::Explorer => "Explorer",
            Self::Quantum => "Quantum",
            Self::Introduction => "Introduction",
            Self::GetStarted => "Get Started",
            Self::Tutorial => "Tutorial",
            Self::Changelog => "Changelog",
            Self::Team => "Team",
            Self::Billing => "Billing",
            Self::Limits => "Limits",
            Self::General => "General",
        }
    }

    pub fn is_disabled(&self) -> bool {
        match self {
            Self::Quantum => true,
            _ => false,
        }
    }

    pub fn handler(
        &self,
        item: &Item,
    ) -> impl Fn(&mut SidebarStory, &ClickEvent, &mut Window, &mut Context<SidebarStory>) + 'static
    {
        let item = *item;
        let subitem = *self;
        move |this, _, _, cx| {
            println!(
                "Clicked on item: {}, child: {}",
                item.label(),
                subitem.label()
            );
            this.active_items.insert(item, true);
            this.last_active_item = item;
            this.active_subitem = Some(subitem);
            cx.notify();
        }
    }
}

impl super::Story for SidebarStory {
    fn title() -> &'static str {
        "Sidebar"
    }

    fn description() -> &'static str {
        "A composable, themeable and customizable sidebar component."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for SidebarStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SidebarStory {
    fn render(
        &mut self,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::Context<Self>,
    ) -> impl gpui_kit::IntoElement {
        let groups: [Vec<Item>; 2] = [
            vec![
                Item::Playground,
                Item::Models,
                Item::Documentation,
                Item::Settings,
            ],
            vec![
                Item::DesignEngineering,
                Item::SalesAndMarketing,
                Item::Travel,
            ],
        ];
        let collapsible = self.collapsible;
        let icon_collapsed = self.collapsed && collapsible == SidebarCollapsible::Icon;
        let toggle_collapsed = self.collapsed && collapsible != SidebarCollapsible::None;

        h_flex()
            .on_action(cx.listener(|this, action: &SidebarOption, _, cx| {
                match action {
                    SidebarOption::Icon => this.collapsible = SidebarCollapsible::Icon,
                    SidebarOption::Offcanvas => this.collapsible = SidebarCollapsible::Offcanvas,
                    SidebarOption::None => this.collapsible = SidebarCollapsible::None,
                    SidebarOption::Right => {
                        this.side = if this.side.is_right() {
                            Side::Left
                        } else {
                            Side::Right
                        }
                    }
                    SidebarOption::ClickToOpen => {
                        this.click_to_open_submenu = !this.click_to_open_submenu
                    }
                    SidebarOption::DynamicChildren => {
                        this.show_dynamic_children = !this.show_dynamic_children
                    }
                }
                cx.notify();
            }))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .h_full()
            .when(self.side.is_right(), |this| this.flex_row_reverse())
            .child(
                Sidebar::new("sidebar-story")
                    .side(self.side)
                    .collapsible(collapsible)
                    .collapsed(self.collapsed)
                    .w(px(220.))
                    .gap_0()
                    .header(
                        SidebarHeader::new()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(cx.theme().radius)
                                    .bg(cx.theme().success)
                                    .text_color(cx.theme().success_foreground)
                                    .size_8()
                                    .flex_shrink_0()
                                    .when(!icon_collapsed, |this| {
                                        this.child(Icon::new(IconName::GalleryVerticalEnd))
                                    })
                                    .when(icon_collapsed, |this| {
                                        this.size_4()
                                            .bg(cx.theme().transparent)
                                            .text_color(cx.theme().foreground)
                                            .child(Icon::new(IconName::GalleryVerticalEnd))
                                    }),
                            )
                            .when(!icon_collapsed, |this| {
                                this.child(
                                    v_flex()
                                        .gap_0()
                                        .text_sm()
                                        .flex_1()
                                        .line_height(relative(1.25))
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child("Company Name")
                                        .child(div().child("Enterprise").text_xs()),
                                )
                            })
                            .when(!icon_collapsed, |this| {
                                this.child(
                                    Icon::new(IconName::ChevronsUpDown).size_4().flex_shrink_0(),
                                )
                            })
                            .dropdown_menu(|menu, _, _| {
                                menu.menu(
                                    "Twitter Inc.",
                                    Box::new(SelectCompany(SharedString::from("twitter"))),
                                )
                                .menu(
                                    "Meta Platforms",
                                    Box::new(SelectCompany(SharedString::from("meta"))),
                                )
                                .menu(
                                    "Google Inc.",
                                    Box::new(SelectCompany(SharedString::from("google"))),
                                )
                            }),
                    )
                    .child(
                        SidebarGroup::new("Platform").child(SidebarMenu::new().children(
                            groups[0].iter().enumerate().map(|(ix, item)| {
                                let is_active =
                                    self.last_active_item == *item && self.active_subitem == None;
                                SidebarMenuItem::new(item.label())
                                    .icon(item.icon())
                                    .active(is_active)
                                    .default_open(ix == 0)
                                    .click_to_open(self.click_to_open_submenu)
                                    .when(is_active, |this| {
                                        this.border_1().border_color(Hsla::white())
                                    })
                                    .when(ix == 0, |this| {
                                        this.context_menu({
                                            move |this, _, _| {
                                                this.link(
                                                    "About",
                                                    "https://github.com/longbridge/gpui-kit",
                                                )
                                            }
                                        })
                                    })
                                    .children(item.items().into_iter().enumerate().map(
                                        |(ix, sub_item)| {
                                            SidebarMenuItem::new(sub_item.label())
                                                .active(self.active_subitem == Some(sub_item))
                                                .disable(sub_item.is_disabled())
                                                .when(ix == 0, |this| {
                                                    this.suffix({
                                                        let checked = self.checked;
                                                        let view = cx.entity();
                                                        move |window, _| {
                                                            Switch::new("switch")
                                                                .xsmall()
                                                                .checked(checked)
                                                                .on_click(window.listener_for(
                                                                    &view,
                                                                    Self::switch_checked_handler,
                                                                ))
                                                        }
                                                    })
                                                    .label_style(
                                                        StyleRefinement::default()
                                                            .text_color(red_500()),
                                                    )
                                                    .context_menu({
                                                        move |this, _, _| {
                                                            this.label("This is a label")
                                                        }
                                                    })
                                                })
                                                .on_click(cx.listener(sub_item.handler(&item)))
                                        },
                                    ))
                                    .on_click(cx.listener(item.handler()))
                            }),
                        )),
                    )
                    .child(
                        SidebarGroup::new("Projects").child(SidebarMenu::new().children(
                            groups[1].iter().enumerate().map(|(ix, item)| {
                                let is_active =
                                    self.last_active_item == *item && self.active_subitem == None;
                                SidebarMenuItem::new(item.label())
                                    .icon(item.icon())
                                    .active(is_active)
                                    .disable(item.is_disabled())
                                    .click_to_open(self.click_to_open_submenu)
                                    .when(ix == 0 && self.show_dynamic_children, |this| {
                                        this.default_open(true).children(vec![
                                            SidebarMenuItem::new("Child A")
                                                .on_click(cx.listener(|_, _, _, _| {})),
                                            SidebarMenuItem::new("Child B")
                                                .on_click(cx.listener(|_, _, _, _| {})),
                                        ])
                                    })
                                    .when(ix == 0, |this| {
                                        this.suffix(|_, _| {
                                            Badge::new().dot().count(1).child(
                                                div().p_0p5().child(Icon::new(IconName::Bell)),
                                            )
                                        })
                                    })
                                    .when(ix == 1, |this| {
                                        this.suffix(|_, _| Icon::new(IconName::Settings2))
                                    })
                                    .on_click(cx.listener(item.handler()))
                            }),
                        )),
                    )
                    .footer(
                        SidebarFooter::new()
                            .justify_between()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(IconName::CircleUser)
                                    .when(!icon_collapsed, |this| this.child("Jason Lee")),
                            )
                            .when(!icon_collapsed, |this| {
                                this.child(Icon::new(IconName::ChevronsUpDown).size_4())
                            }),
                    ),
            )
            .child(
                v_flex()
                    .h_full()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .gap_4()
                    .p_4()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .when(
                                self.side.is_right() && collapsible != SidebarCollapsible::None,
                                |this| this.flex_row_reverse().justify_between(),
                            )
                            .when(collapsible != SidebarCollapsible::None, |this| {
                                this.child(
                                    SidebarToggleButton::new()
                                        .side(self.side)
                                        .collapsed(toggle_collapsed)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.collapsed = !this.collapsed;
                                            cx.notify();
                                        })),
                                )
                                .child(Separator::vertical().h_4())
                            })
                            .child(
                                Breadcrumb::new()
                                    .child("Breadcrumb")
                                    .child(BreadcrumbItem::new("Home").on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.last_active_item = Item::Playground;
                                            cx.notify();
                                        },
                                    )))
                                    .child(
                                        BreadcrumbItem::new(self.last_active_item.label())
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.active_subitem = None;
                                                cx.notify();
                                            })),
                                    )
                                    .when_some(self.active_subitem, |this, subitem| {
                                        this.child(BreadcrumbItem::new(subitem.label()))
                                    }),
                            ),
                    )
                    .child(self.render_content(window, cx)),
            )
    }
}
