use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    dock::PanelControl,
    h_flex,
    progress::ProgressCircle,
    separator::Separator,
    status_bar::StatusBar,
    v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window, px,
};

use crate::section;

pub struct StatusBarStory {
    focus_handle: gpui_kit::FocusHandle,
}

impl StatusBarStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for StatusBarStory {
    fn title() -> &'static str {
        "StatusBar"
    }

    fn description() -> &'static str {
        "A horizontal bar with left/center/right regions, usually placed at the bottom."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for StatusBarStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for StatusBarStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .child(
                section("Editor")
                    .description(
                        "Places repository state on the left and document state on the right.",
                    )
                    .w(px(760.))
                    .child(
                        v_flex().w_full().child(
                            StatusBar::new()
                                .left(
                                    Button::new("branch")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Github)
                                        .label("main")
                                        .tooltip("Git branch")
                                        .on_click(|_, window, cx| {
                                            window.push_notification("Switch branch", cx);
                                        }),
                                )
                                .left(Separator::vertical().h_3())
                                .left(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            h_flex()
                                                .items_center()
                                                .gap_1()
                                                .child(
                                                    Icon::new(IconName::CircleCheck)
                                                        .xsmall()
                                                        .text_color(cx.theme().green),
                                                )
                                                .child("0"),
                                        )
                                        .child(
                                            h_flex()
                                                .items_center()
                                                .gap_1()
                                                .child(
                                                    Icon::new(IconName::Info)
                                                        .xsmall()
                                                        .text_color(cx.theme().blue),
                                                )
                                                .child("2"),
                                        ),
                                )
                                .right(
                                    Button::new("position")
                                        .ghost()
                                        .xsmall()
                                        .label("Ln 12, Col 34")
                                        .tooltip("Go to Line/Column")
                                        .on_click(|_, window, cx| {
                                            window.push_notification("Go to Line/Column", cx);
                                        }),
                                )
                                .right(Separator::vertical().h_3())
                                .right(
                                    Button::new("encoding")
                                        .ghost()
                                        .xsmall()
                                        .label("UTF-8")
                                        .on_click(|_, window, cx| {
                                            window.push_notification("Select encoding", cx);
                                        }),
                                )
                                .right(
                                    Button::new("language")
                                        .ghost()
                                        .xsmall()
                                        .label("Rust")
                                        .on_click(|_, window, cx| {
                                            window.push_notification("Select language", cx);
                                        }),
                                ),
                        ),
                    ),
            )
            .child(
                section("Application")
                    .description("Combines connectivity, progress, save state, and notifications.")
                    .w(px(760.))
                    .child(
                        v_flex().w_full().child(
                            StatusBar::new()
                                .left(
                                    h_flex()
                                        .items_center()
                                        .gap_1()
                                        .child(Icon::new(IconName::Check).xsmall())
                                        .child("Connected"),
                                )
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(ProgressCircle::new("syncing").value(45.))
                                        .child("Syncing…"),
                                )
                                .right("All changes saved")
                                .right(
                                    Button::new("notifications")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Bell)
                                        .label("3")
                                        .tooltip("3 notifications")
                                        .on_click(|_, window, cx| {
                                            window.push_notification("3 notifications", cx);
                                        }),
                                ),
                        ),
                    ),
            )
            // Layout cases for verifying the dynamic centering behavior.
            .child(
                section("Alignment")
                    .description("Center content adapts when either side is empty or populated.")
                    .w(px(760.))
                    .child(
                        v_flex()
                            .w_full()
                            .gap_6()
                            .child(StatusBar::new().child("Center only → start-aligned"))
                            .child(
                                StatusBar::new()
                                    .left("Left")
                                    .child("Center → end (only left)"),
                            )
                            .child(
                                StatusBar::new()
                                    .child("Center → start (only right)")
                                    .right("Right"),
                            )
                            .child(
                                StatusBar::new()
                                    .left("Left")
                                    .child("Center → centered (left + right)")
                                    .right("Right"),
                            )
                            .child(StatusBar::new().left("Left").right("Right")),
                    ),
            )
    }
}
