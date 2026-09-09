use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    link::Link,
    marker::{Marker, MarkerContent, MarkerIcon, MarkerLoadingStyle, MarkerVariant},
    shimmer::{ShimmerStyle, ShimmerText},
    spinner::Spinner,
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, StyleRefinement, Styled as _, Window, rems,
};
use std::time::Duration;

use crate::{Story, section};

pub struct MarkerStory {
    focus_handle: FocusHandle,
}

impl MarkerStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Story for MarkerStory {
    fn title() -> &'static str {
        "Marker"
    }

    fn description() -> &'static str {
        "A compact row for conversation status, notifications, and separators."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for MarkerStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MarkerStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("Variants")
                    .description(
                        "Choose a plain row, a centered separator, or a bordered boundary.",
                    )
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_4()
                    .child(Marker::new().content(MarkerContent::new().child("Plain status update")))
                    .child(
                        Marker::new()
                            .with_variant(MarkerVariant::Separator)
                            .content(MarkerContent::new().child("Earlier messages")),
                    )
                    .child(
                        Marker::new()
                            .with_variant(MarkerVariant::Border)
                            .content(MarkerContent::new().child("Unread messages")),
                    ),
            )
            .child(
                section("Status")
                    .description(
                        "Compose icons, spinners, and labels without a fixed status model.",
                    )
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        Marker::new()
                            .text_color(cx.theme().success)
                            .icon(MarkerIcon::new().child(Icon::new(IconName::CircleCheck)))
                            .content(MarkerContent::new().child("Online")),
                    )
                    .child(
                        Marker::new()
                            .icon(MarkerIcon::new().child(Spinner::new().xsmall()))
                            .content(MarkerContent::new().child("Alice is typing…")),
                    )
                    .child(
                        Marker::new()
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Bell)))
                            .content(MarkerContent::new().child("Unread notifications")),
                    )
                    .child(
                        Marker::new()
                            .text_color(cx.theme().danger)
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
                            .content(MarkerContent::new().child("Message could not be delivered")),
                    ),
            )
            .child(
                section("With icon")
                    .description("Icons can communicate sender activity, notices, and saved items.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        Marker::new()
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
                            .content(MarkerContent::new().child("Conversation details updated")),
                    )
                    .child(
                        Marker::new()
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Star)))
                            .content(MarkerContent::new().child("Pinned for your team")),
                    )
                    .child(
                        Marker::new().content(MarkerContent::new().child("No icon is required")),
                    ),
            )
            .child(
                section("Loading styles")
                    .description("Choose a spinner or a sweeping, ChatGPT-style text shimmer.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_4()
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Spinner)
                            .content(MarkerContent::new().text("shadcn/ui · Loading messages…")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .content(MarkerContent::new().text("ChatGPT · Thinking")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
                            .content(MarkerContent::new().text("正在探索 4 个文件…")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .with_shimmer_style(
                                ShimmerStyle::new()
                                    .duration(Duration::from_secs(3))
                                    .highlight_color(cx.theme().primary)
                                    .spread(0.45)
                                    .reverse(true),
                            )
                            .content(
                                MarkerContent::new().text("Custom color, width, and direction"),
                            ),
                    )
                    .child(
                        ShimmerText::new("Reusable shimmer without a Marker")
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            .child(
                section("Shimmer settings")
                    .description(
                        "Customize timing, highlight width, direction, and playback independently.",
                    )
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .with_shimmer_style(
                                ShimmerStyle::new().duration(Duration::from_millis(900)),
                            )
                            .content(MarkerContent::new().text("Faster highlight sweep")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .with_shimmer_style(ShimmerStyle::new().spread(0.55))
                            .content(MarkerContent::new().text("Wider highlight band")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .with_shimmer_style(ShimmerStyle::new().reverse(true))
                            .content(MarkerContent::new().text("Right-to-left sweep")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .with_shimmer_style(
                                ShimmerStyle::new().highlight_color(cx.theme().primary),
                            )
                            .content(MarkerContent::new().text("Semantic primary highlight")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .with_shimmer_style(ShimmerStyle::new().once(true))
                            .content(MarkerContent::new().text("Play the highlight once")),
                    )
                    .child(
                        Marker::new()
                            .loading(false)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .content(MarkerContent::new().text("Loading is disabled")),
                    ),
            )
            .child(
                section("Separator")
                    .description("Place a conversation boundary between two semantic lines.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_4()
                    .child(
                        Marker::new()
                            .with_variant(MarkerVariant::Separator)
                            .content(MarkerContent::new().child("Today")),
                    )
                    .child(
                        Marker::new()
                            .with_variant(MarkerVariant::Separator)
                            .separator_style(
                                StyleRefinement::default().bg(cx.theme().primary.opacity(0.35)),
                            )
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Star)))
                            .content(MarkerContent::new().child("Pinned messages")),
                    ),
            )
            .child(
                section("Border")
                    .description("Use a bottom edge for an unread or section boundary.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        Marker::new()
                            .with_variant(MarkerVariant::Border)
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
                            .content(MarkerContent::new().child("3 unread messages")),
                    )
                    .child(
                        Marker::new()
                            .with_variant(MarkerVariant::Border)
                            .border_color(cx.theme().primary.opacity(0.4))
                            .content(
                                MarkerContent::new().child("New replies since your last visit"),
                            ),
                    ),
            )
            .child(
                section("Links and buttons")
                    .description(
                        "Keep external destinations and in-app commands semantically distinct.",
                    )
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        Marker::new()
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
                            .content(
                                MarkerContent::new().child(
                                    Link::new("marker-documentation-link")
                                        .href("https://gpui-kit.com/")
                                        .child("Open the component documentation"),
                                ),
                            ),
                    )
                    .child(
                        Marker::new()
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Star)))
                            .content(MarkerContent::new().child("A saved draft is ready"))
                            .child(
                                Button::new("marker-open-draft")
                                    .ghost()
                                    .small()
                                    .label("Open draft"),
                            ),
                    ),
            )
            .child(
                section("Custom style")
                    .description("Caller refinements can replace spacing, color, and surface.")
                    .max_w(rems(42.5))
                    .child(
                        Marker::new()
                            .px_3()
                            .py_2()
                            .rounded(cx.theme().radius)
                            .bg(cx.theme().accent)
                            .text_color(cx.theme().accent_foreground)
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Star)))
                            .content(MarkerContent::new().child("Pinned message")),
                    ),
            )
    }
}
