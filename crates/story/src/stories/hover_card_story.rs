use gpui_kit::component::{
    ActiveTheme, StyledExt, avatar::Avatar, button::Button, h_flex, hover_card::HoverCard,
    link::Link, v_flex,
};
use gpui_kit::{
    Anchor, App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render,
    Styled as _, Window, div, px, relative,
};
use std::time::Duration;

use crate::{Story, section};

pub struct HoverCardStory {}

impl HoverCardStory {
    fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self {}
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for HoverCardStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .child(self.render_basic_example(cx))
            .child(self.render_user_profile_example(cx))
            .child(self.render_custom_timing_example(cx))
            .child(self.render_positioning_examples(cx))
    }
}

impl HoverCardStory {
    /// Basic hover card example
    fn render_basic_example(&self, cx: &mut Context<Self>) -> impl IntoElement {
        section("Default")
            .description("Shows supporting information without changing the current view.")
            .w(px(520.))
            .child(
            HoverCard::new("basic")
                .trigger(
                    div()
                        .child("Hover over me")
                        .text_color(cx.theme().primary)
                        .cursor_pointer()
                        .text_sm(),
                )
                .child(
                    v_flex()
                        .gap_1()
                        .w(px(450.))
                        .child(
                            div()
                                .child("This is a hover card")
                                .font_semibold()
                                .text_sm(),
                        )
                        .child(
                            div()
                                .child("You can display rich content when hovering over a trigger element.")
                                .text_color(cx.theme().muted_foreground)
                                .text_sm(),
                        ),
                ),
        )
    }

    fn render_user_profile_example(&self, _: &mut Context<Self>) -> impl IntoElement {
        section("Rich Content")
            .description("Cards can contain avatars, typography, and structured details.")
            .w(px(520.))
            .child(
                h_flex()
                    .child("Hover over ")
                    .child(
                        HoverCard::new("user-profile")
                            .trigger(Link::new("user-profile-link").child("@huacnlee"))
                            .content(|_, _, cx| {
                                h_flex()
                                    .w(px(320.))
                                    .gap_3()
                                    .items_start()
                                    .child(
                                        Avatar::new().src(
                                            "https://avatars.githubusercontent.com/u/5518?s=64",
                                        ),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .line_height(relative(1.))
                                            .child(div().child("Jason Lee").font_semibold())
                                            .child(
                                                div()
                                                    .child("@huacnlee")
                                                    .text_color(cx.theme().link)
                                                    .text_sm(),
                                            )
                                            .child(
                                                div().mt_1().child("The author of GPUI Component."),
                                            ),
                                    )
                            }),
                    )
                    .child(" to see their profile"),
            )
    }

    /// Custom timing configuration example
    fn render_custom_timing_example(&self, _: &mut Context<Self>) -> impl IntoElement {
        section("Timing")
            .description("Open and close delays can match the interaction context.")
            .w(px(520.))
            .child(
                h_flex()
                    .gap_4()
                    .child(
                        HoverCard::new("fast-open")
                            .open_delay(Duration::from_millis(200))
                            .close_delay(Duration::from_millis(100))
                            .trigger(Button::new("fast").label("Fast Open (200ms)").outline())
                            .child(div().child("This hover card opens after 200ms").text_sm()),
                    )
                    .child(
                        HoverCard::new("slow-open")
                            .open_delay(Duration::from_secs(1))
                            .close_delay(Duration::from_secs_f32(0.5))
                            .trigger(Button::new("slow").label("Slow Open (1000ms)").outline())
                            .child(div().child("This hover card opens after 1000ms").text_sm()),
                    ),
            )
    }

    /// All positioning options
    fn render_positioning_examples(&self, _: &mut Context<Self>) -> impl IntoElement {
        section("Position")
            .description("Content can anchor to each side of its trigger.")
            .w(px(640.))
            .child(
                v_flex()
                    .gap_4()
                    .items_center()
                    .justify_center()
                    .child(
                        h_flex()
                            .gap_4()
                            .child(
                                HoverCard::new("anchor-top-left")
                                    .anchor(Anchor::TopLeft)
                                    .trigger(Button::new("tl").label("Top Left").outline())
                                    .child(div().child("Positioned at Top Left").text_sm()),
                            )
                            .child(
                                HoverCard::new("anchor-top-center")
                                    .anchor(Anchor::TopCenter)
                                    .trigger(Button::new("tc").label("Top Center").outline())
                                    .child(div().child("Positioned at Top Center").text_sm()),
                            )
                            .child(
                                HoverCard::new("anchor-top-right")
                                    .anchor(Anchor::TopRight)
                                    .trigger(Button::new("tr").label("Top Right").outline())
                                    .child(div().child("Positioned at Top Right").text_sm()),
                            ),
                    )
                    // Bottom row
                    .child(
                        h_flex()
                            .gap_4()
                            .child(
                                HoverCard::new("anchor-bottom-left")
                                    .anchor(Anchor::BottomLeft)
                                    .trigger(Button::new("bl").label("Bottom Left").outline())
                                    .child(div().child("Positioned at Bottom Left").text_sm()),
                            )
                            .child(
                                HoverCard::new("anchor-bottom-center")
                                    .anchor(Anchor::BottomCenter)
                                    .trigger(Button::new("bc").label("Bottom Center").outline())
                                    .child(div().child("Positioned at Bottom Center").text_sm()),
                            )
                            .child(
                                HoverCard::new("anchor-bottom-right")
                                    .anchor(Anchor::BottomRight)
                                    .trigger(Button::new("br").label("Bottom Right").outline())
                                    .child(div().child("Positioned at Bottom Right").text_sm()),
                            ),
                    ),
            )
    }
}

impl Story for HoverCardStory {
    fn title() -> &'static str {
        "HoverCard"
    }

    fn description() -> &'static str {
        "A hover card displays content when hovering over a trigger element, with configurable delays."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}
