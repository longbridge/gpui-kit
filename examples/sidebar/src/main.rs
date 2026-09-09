use gpui_kit::assets::Assets;
use gpui_kit::component::{
    ActiveTheme, Icon, IconName, Selectable,
    button::Button,
    sidebar::{
        Sidebar, SidebarCollapsible, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu,
        SidebarMenuItem, SidebarToggleButton,
    },
    *,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

pub struct Example {
    collapsible: SidebarCollapsible,
    collapsed: bool,
}

impl Example {
    fn new() -> Self {
        Self {
            collapsible: SidebarCollapsible::Icon,
            collapsed: false,
        }
    }

    fn menu() -> SidebarMenu {
        SidebarMenu::new().children([
            SidebarMenuItem::new("Dashboard")
                .icon(IconName::LayoutDashboard)
                .active(true),
            SidebarMenuItem::new("Inbox").icon(IconName::Inbox),
            SidebarMenuItem::new("Calendar").icon(IconName::Calendar),
            SidebarMenuItem::new("Projects")
                .icon(IconName::Folder)
                .default_open(true)
                .click_to_toggle(true)
                .children([
                    SidebarMenuItem::new("Design"),
                    SidebarMenuItem::new("Engineering"),
                    SidebarMenuItem::new("Marketing"),
                ]),
            SidebarMenuItem::new("Settings").icon(IconName::Settings),
        ])
    }

    fn description(&self) -> &'static str {
        match self.collapsible {
            SidebarCollapsible::Icon => {
                "The sidebar collapses to icon width, matching shadcn's collapsible=\"icon\" behavior."
            }
            SidebarCollapsible::Offcanvas => {
                "The sidebar releases its layout width when collapsed and keeps hidden controls out of keyboard navigation, matching shadcn's collapsible=\"offcanvas\" behavior."
            }
            SidebarCollapsible::None => {
                "The sidebar ignores the collapsed state and remains expanded, matching shadcn's collapsible=\"none\" behavior."
            }
        }
    }

    fn mode_button(
        &self,
        id: &'static str,
        label: &'static str,
        mode: SidebarCollapsible,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(id)
            .label(label)
            .small()
            .selected(self.collapsible == mode)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.collapsible = mode;
                cx.notify();
            }))
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let icon_collapsed = self.collapsed && self.collapsible == SidebarCollapsible::Icon;
        let show_toggle = self.collapsible != SidebarCollapsible::None;

        h_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                Sidebar::new("sidebar-example")
                    .collapsible(self.collapsible)
                    .collapsed(self.collapsed)
                    .w(px(240.))
                    .header(
                        SidebarHeader::new()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .size_8()
                                    .flex_shrink_0()
                                    .rounded(cx.theme().radius)
                                    .bg(cx.theme().sidebar_primary)
                                    .text_color(cx.theme().sidebar_primary_foreground)
                                    .when(icon_collapsed, |this| {
                                        this.size_4()
                                            .bg(cx.theme().transparent)
                                            .text_color(cx.theme().foreground)
                                    })
                                    .child(Icon::new(IconName::GalleryVerticalEnd)),
                            )
                            .when(!icon_collapsed, |this| {
                                this.child(
                                    v_flex()
                                        .flex_1()
                                        .overflow_hidden()
                                        .child("Acme Inc")
                                        .child(div().text_xs().child("Enterprise")),
                                )
                            }),
                    )
                    .child(SidebarGroup::new("Application").child(Self::menu()))
                    .footer(
                        SidebarFooter::new().child(
                            h_flex()
                                .gap_2()
                                .child(IconName::CircleUser)
                                .when(!icon_collapsed, |this| this.child("Jason Lee")),
                        ),
                    ),
            )
            .child(
                v_flex()
                    .h_full()
                    .flex_1()
                    .min_w_0()
                    .gap_4()
                    .p_4()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .when(show_toggle, |this| {
                                this.child(
                                    SidebarToggleButton::new()
                                        .collapsed(icon_collapsed)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.collapsed = !this.collapsed;
                                            cx.notify();
                                        })),
                                )
                            })
                            .child(div().font_bold().child("Sidebar collapsible modes")),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_sm().child("Mode:"))
                            .child(self.mode_button(
                                "mode-icon",
                                "Icon",
                                SidebarCollapsible::Icon,
                                cx,
                            ))
                            .child(self.mode_button(
                                "mode-offcanvas",
                                "Offcanvas",
                                SidebarCollapsible::Offcanvas,
                                cx,
                            ))
                            .child(self.mode_button(
                                "mode-none",
                                "None",
                                SidebarCollapsible::None,
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().border)
                            .p_5()
                            .child(self.description()),
                    ),
            )
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        gpui_kit::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(900.), px(620.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|_| Example::new());
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
