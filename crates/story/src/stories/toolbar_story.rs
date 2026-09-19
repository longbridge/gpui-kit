use gpui_kit::component::{
    IconName, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    dock::PanelControl,
    separator::Separator,
    toolbar::{Toolbar, ToolbarGroup},
    v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window, px,
};

use crate::section;

pub struct ToolbarStory {
    focus_handle: FocusHandle,
}

impl ToolbarStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

fn icon_button(id: &'static str, icon: IconName, tooltip: &'static str) -> Button {
    Button::new(id)
        .ghost()
        .icon(icon)
        .tooltip(tooltip)
        .on_click(move |_, window, cx| window.push_notification(tooltip, cx))
}

impl super::Story for ToolbarStory {
    fn title() -> &'static str {
        "Toolbar"
    }

    fn description() -> &'static str {
        "A themed bar of commands, usually placed at the top of a window or pane."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for ToolbarStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ToolbarStory {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .child(
                section("Document")
                    .description(
                        "Frequent commands on the left, view and overflow actions on the right.",
                    )
                    .w(px(760.))
                    .child(
                        v_flex().w_full().child(
                            Toolbar::new("document-toolbar")
                                .left(
                                    Button::new("new")
                                        .ghost()
                                        .icon(IconName::Plus)
                                        .label("New")
                                        .on_click(|_, window, cx| {
                                            window.push_notification("New", cx);
                                        }),
                                )
                                .left(icon_button("open", IconName::FolderOpen, "Open"))
                                .left(Separator::vertical().h_5())
                                .left(
                                    ToolbarGroup::new("history-group")
                                        .label("History")
                                        .gap_2()
                                        .child(icon_button("undo", IconName::Undo2, "Undo"))
                                        .child(icon_button("redo", IconName::Redo2, "Redo")),
                                )
                                .right(icon_button("find", IconName::Search, "Find"))
                                .right(Separator::vertical().h_5())
                                .right(icon_button("settings", IconName::Settings2, "Settings"))
                                .right(icon_button("more", IconName::Ellipsis, "More options")),
                        ),
                    ),
            )
            .child(
                section("Sizes")
                    .description(
                        "Bar height, spacing, and text scale with the size. Buttons follow the \
                        Button size scale, whose frames run 20/24/32 px — a large icon button \
                        keeps the 32 px frame and grows its icon to 24 px.",
                    )
                    .w(px(760.))
                    .child(
                        v_flex()
                            .w_full()
                            .gap_6()
                            .child(
                                Toolbar::new("xs-toolbar")
                                    .xsmall()
                                    .left(
                                        Button::new("xs-new")
                                            .ghost()
                                            .xsmall()
                                            .icon(IconName::Plus)
                                            .label("New"),
                                    )
                                    .right(
                                        icon_button("xs-find", IconName::Search, "Find").xsmall(),
                                    ),
                            )
                            .child(
                                Toolbar::new("sm-toolbar")
                                    .small()
                                    .left(
                                        Button::new("sm-new")
                                            .ghost()
                                            .small()
                                            .icon(IconName::Plus)
                                            .label("New"),
                                    )
                                    .right(
                                        icon_button("sm-find", IconName::Search, "Find").small(),
                                    ),
                            )
                            .child(
                                Toolbar::new("md-toolbar")
                                    .left(
                                        Button::new("md-new")
                                            .ghost()
                                            .icon(IconName::Plus)
                                            .label("New"),
                                    )
                                    .right(icon_button("md-find", IconName::Search, "Find")),
                            )
                            .child(
                                Toolbar::new("lg-toolbar")
                                    .large()
                                    .left(
                                        Button::new("lg-new")
                                            .ghost()
                                            .large()
                                            .icon(IconName::Plus)
                                            .label("New"),
                                    )
                                    .right(
                                        icon_button("lg-find", IconName::Search, "Find").large(),
                                    ),
                            ),
                    ),
            )
            // Layout cases for verifying the dynamic centering behavior.
            .child(
                section("Regions")
                    .description("Middle content adapts when either end is empty or populated.")
                    .w(px(760.))
                    .child(
                        v_flex()
                            .w_full()
                            .gap_4()
                            .child(
                                Toolbar::new("regions-center").child("Center only → start-aligned"),
                            )
                            .child(
                                Toolbar::new("regions-left")
                                    .left("Left")
                                    .child("Center → end (only left)"),
                            )
                            .child(
                                Toolbar::new("regions-right")
                                    .child("Center → start (only right)")
                                    .right("Right"),
                            )
                            .child(
                                Toolbar::new("regions-both")
                                    .left("Left")
                                    .child("Center → centered (left + right)")
                                    .right("Right"),
                            )
                            .child(Toolbar::new("regions-ends").left("Left").right("Right")),
                    ),
            )
    }
}
