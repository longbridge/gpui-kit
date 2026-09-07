use gpui_kit::assets::Assets;
use gpui_kit::component::{
    ActiveTheme, Icon, IconName, Side, TitleBar, WindowExt,
    scroll::ScrollableElement,
    button::{Button, ButtonVariants as _},
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    dialog::DialogButtonProps,
    sidebar::{
        Sidebar, SidebarCollapsible, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu,
        SidebarMenuItem, SidebarToggleButton,
    },
    text::TextView,
    *,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

const DOC: &str = r#"
# Quarterly Report

This document is rendered by `TextView::markdown` in the **center view**,
between the two sidebars.

## Layout

- **Left sidebar** — navigation with collapsible groups.
- **Center view** — the working area: breadcrumb, document, toolbar actions.
- **Right sidebar** — an inspector panel showing document properties.

> Try the toolbar buttons: *Share* posts a notification, *Delete* opens an
> alert dialog that must be answered.

```rust
Sidebar::new("right")
    .side(Side::Right)
    .w(px(230.))
    .child(SidebarGroup::new("Properties"))
```
"#;

pub struct Example {
    left_collapsed: bool,
    right_collapsed: bool,
}

impl Example {
    fn new() -> Self {
        Self {
            left_collapsed: false,
            right_collapsed: false,
        }
    }

    /// Left sidebar: app navigation.
    fn nav_menu() -> SidebarMenu {
        SidebarMenu::new().children([
            SidebarMenuItem::new("Dashboard")
                .icon(IconName::LayoutDashboard)
                .active(true),
            SidebarMenuItem::new("Search").icon(IconName::Search),
            SidebarMenuItem::new("Projects")
                .icon(IconName::Folder)
                .default_open(true)
                .click_to_toggle(true)
                .children([
                    SidebarMenuItem::new("Website Redesign"),
                    SidebarMenuItem::new("Mobile App"),
                    SidebarMenuItem::new("Design System"),
                ]),
            SidebarMenuItem::new("Inbox").icon(IconName::Inbox),
            SidebarMenuItem::new("Calendar").icon(IconName::Calendar),
        ])
    }

    /// Center toolbar: sidebar toggle, breadcrumb, actions.
    fn toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .px_3()
            .py_2()
            .child(
                SidebarToggleButton::new()
                    .collapsed(self.left_collapsed)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.left_collapsed = !this.left_collapsed;
                        cx.notify();
                    })),
            )
            .child(
                Breadcrumb::new().children([
                    BreadcrumbItem::new("Projects"),
                    BreadcrumbItem::new("Website Redesign"),
                    BreadcrumbItem::new("Quarterly Report"),
                ]),
            )
            .child(
                h_flex().ml_auto().gap_1().child(
                    Button::new("share")
                        .ghost()
                        .icon(IconName::ExternalLink)
                        .label("Share")
                        .small()
                        .on_click(|_, window, cx| {
                            window.push_notification("Report shared with the team", cx);
                        }),
                ),
            )
            .child(
                Button::new("delete")
                    .ghost()
                    .danger()
                    .icon(IconName::Delete)
                    .small()
                    .on_click(|_, window, cx| {
                        window.open_alert_dialog(cx, |alert, _, _| {
                            alert
                                .title("Delete “Quarterly Report”?")
                                .description("The file stays in the recycle bin for 30 days.")
                                .button_props(
                                    DialogButtonProps::default()
                                        .ok_text("Delete")
                                        .on_ok(|_, window, cx| {
                                            window.push_notification("Report deleted", cx);
                                            true
                                        }),
                                )
                        });
                    }),
            )
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            // Semi-transparent surface over the blurred window background.
            .bg(cx.theme().background.opacity(0.72))
            .child(
                TitleBar::new()
                    .bg(hsla(0., 0., 0., 0.))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Quarterly Report — Acme Inc"),
                    ),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
            // Left sidebar: navigation
            .child(
                Sidebar::new("left")
                    .collapsible(SidebarCollapsible::Icon)
                    .collapsed(self.left_collapsed)
                    .w(px(220.))
                    .header(
                        SidebarHeader::new().child(
                            h_flex().gap_2().items_center().child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .size_7()
                                    .flex_shrink_0()
                                    .rounded(cx.theme().radius)
                                    .bg(cx.theme().sidebar_primary)
                                    .text_color(cx.theme().sidebar_primary_foreground)
                                    .child(Icon::new(IconName::GalleryVerticalEnd)),
                            )
                            .when(!self.left_collapsed, |this| {
                                this.child(
                                    v_flex()
                                        .overflow_hidden()
                                        .child(div().text_sm().font_bold().child("Acme Inc"))
                                        .child(div().text_xs().child("Workspace")),
                                )
                            }),
                        ),
                    )
                    .child(SidebarGroup::new("Application").child(Self::nav_menu()))
                    .footer(
                        SidebarFooter::new().child(
                            h_flex().gap_2().items_center().child(IconName::CircleUser).when(
                                !self.left_collapsed,
                                |this| this.child("Jason Lee"),
                            ),
                        ),
                    ),
            )
            // Center view
            .child(
                v_flex()
                    .h_full()
                    .flex_1()
                    .min_w_0()
                    .child(self.toolbar(cx))
                    .child(
                        div()
                            .id("doc")
                            .flex_1()
                            .overflow_y_scrollbar()
                            .p_6()
                            .child(TextView::markdown("doc", DOC)),
                    )
                    .child(
                        h_flex()
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .px_3()
                            .py_1()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Markdown · 128 lines")
                            .child(div().ml_auto().child("UTF-8")),
                    ),
            )
            // Right sidebar: inspector
            .child(
                Sidebar::new("right")
                    .side(Side::Right)
                    .collapsible(SidebarCollapsible::Icon)
                    .collapsed(self.right_collapsed)
                    .w(px(230.))
                    .header(
                        SidebarHeader::new().child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .when(!self.right_collapsed, |this| {
                                    this.child(div().text_sm().font_bold().child("Inspector"))
                                })
                                .child(
                                    Button::new("toggle-right")
                                        .ghost()
                                        .icon(if self.right_collapsed {
                                            IconName::PanelRightOpen
                                        } else {
                                            IconName::PanelRightClose
                                        })
                                        .xsmall()
                                        .ml_auto()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.right_collapsed = !this.right_collapsed;
                                            cx.notify();
                                        })),
                                ),
                        ),
                    )
                    .child(SidebarGroup::new("Properties").children([
                        SidebarMenuItem::new("Owner: Jason Lee"),
                        SidebarMenuItem::new("Modified: 2h ago"),
                        SidebarMenuItem::new("Size: 24 KB"),
                    ]))
                    .child(SidebarGroup::new("Sharing").children([
                        SidebarMenuItem::new("3 collaborators"),
                        SidebarMenuItem::new("Link sharing off"),
                    ])),
            ),
            )
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        gpui_kit::init(cx);

        let window_options = WindowOptions {
            titlebar: None,
            window_decorations: Some(WindowDecorations::Client),
            window_background: WindowBackgroundAppearance::Blurred,
            window_bounds: Some(WindowBounds::centered(size(px(1180.), px(720.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|_| Example::new());
                // Root paints an opaque theme background by default; clear it so
                // the window's Blurred background shows through the 0.72 layer.
                cx.new(|cx| {
                    Root::new(view, window, cx)
                        .bordered(false)
                        .bg(hsla(0., 0., 0., 0.))
                })
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
