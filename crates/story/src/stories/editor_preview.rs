use gpui_kit::{Context, IntoElement, ParentElement, Render, Styled, Window, div};
use gpui_kit::component::{ActiveTheme, Icon, IconName, StyledExt, h_flex, progress::Progress, v_flex};

pub struct ProjectOverview {
    progress: f32,
}

impl ProjectOverview {
    pub fn new() -> Self {
        Self { progress: 72. }
    }

    fn metric(&self, label: &'static str, value: &'static str) -> impl IntoElement {
        v_flex()
            .gap_1()
            .p_4()
            .rounded(cx.theme().radius_lg)
            .border_1()
            .child(div().text_sm().child(label))
            .child(div().text_2xl().font_semibold().child(value))
    }
}

impl Render for ProjectOverview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_5()
            .p_6()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().text_2xl().font_semibold().child("Project overview"))
                            .child(div().text_sm().child("Everything is moving on schedule.")),
                    )
                    .child(Icon::new(IconName::ChartNoAxesCombined)),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(self.metric("Open tasks", "24"))
                    .child(self.metric("Completed", "86%"))
                    .child(self.metric("Contributors", "12")),
            )
            .child(
                v_flex()
                    .gap_3()
                    .p_4()
                    .rounded(cx.theme().radius_lg)
                    .bg(cx.theme().muted)
                    .child(
                        h_flex()
                            .justify_between()
                            .child("Release progress")
                            .child(format!("{}%", self.progress)),
                    )
                    .child(Progress::new("release").value(self.progress)),
            )
    }
}
