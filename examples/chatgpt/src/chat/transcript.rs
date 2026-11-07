//! Rendering of the conversation's messages and the empty state.

use std::sync::Arc;

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    ai::{Reasoning, Sources, Suggestion, Suggestions, Tool, TraceStatus},
    bubble::Bubble,
    button::{Button, ButtonVariants as _},
    clipboard::Clipboard,
    h_flex,
    message::{Message, MessageAlignment, MessageContent, MessageFooter},
    shimmer::ShimmerText,
    text::TextView,
    v_flex,
};
use gpui_kit::{assets::IconName as LucideIcon, prelude::FluentBuilder as _, *};

use super::{ChatImage, ChatMessage, ChatView, PromptStatus};
use crate::responses::Role;

const SUGGESTIONS: [&str; 4] = [
    "Explain Rust lifetimes with a small example",
    "What changed in the news today?",
    "Create an image of a lighthouse at dusk, watercolor",
    "Draft a polite reply declining a meeting",
];

impl ChatView {
    pub(super) fn render_row(message: ChatMessage, view: WeakEntity<Self>, cx: &App) -> AnyElement {
        if message.role == Role::User {
            return Message::new()
                .id(("message", message.id))
                .alignment(MessageAlignment::End)
                .content(
                    MessageContent::new()
                        .when(!message.images.is_empty(), |this| {
                            this.child(
                                h_flex().flex_wrap().justify_end().gap_2().children(
                                    message
                                        .images
                                        .iter()
                                        .filter_map(|image| image.image.clone())
                                        .map(|image| {
                                            img(image)
                                                .size_32()
                                                .object_fit(ObjectFit::Cover)
                                                .rounded(cx.theme().radius_lg)
                                        }),
                                ),
                            )
                        })
                        .when(!message.text.is_empty(), |this| {
                            this.bubble(Bubble::new().child(message.text))
                        }),
                )
                .into_any_element();
        }

        let id = message.id;
        let has_reasoning = !message.reasoning.is_empty();
        let thinking = message.streaming && message.text.is_empty() && message.images.is_empty();
        let text = message.text.clone();
        let finished = !message.streaming;

        Message::new()
            .id(("message", id))
            .content(
                MessageContent::new()
                    .w_full()
                    .when(has_reasoning || thinking, |this| {
                        let view = view.clone();
                        this.child(
                            Reasoning::new(("reasoning", id))
                                .streaming(thinking)
                                .when_some(message.thought_for, |this, duration| {
                                    this.duration(duration)
                                })
                                .open(message.reasoning_open && has_reasoning)
                                .on_toggle(move |open, _, cx| {
                                    let open = *open;
                                    _ = view.update(cx, |this, cx| {
                                        this.set_message_flag(id, cx, |m| m.reasoning_open = open)
                                    });
                                })
                                .child(message.reasoning.clone()),
                        )
                    })
                    .children(message.searches.into_iter().map(|search| {
                        Tool::new(
                            SharedString::from(format!("search-{id}-{}", search.id)),
                            "web_search",
                        )
                        .status(if search.done {
                            TraceStatus::Completed
                        } else {
                            TraceStatus::Running
                        })
                        .when_some(search.query, |this, query| this.input(query))
                    }))
                    .children(
                        message
                            .images
                            .into_iter()
                            .map(|image| Self::render_generated_image(id, image, view.clone(), cx)),
                    )
                    .when_some(message.markdown, |this, markdown| {
                        this.child(
                            TextView::new(&markdown)
                                .w_full()
                                .selectable(true)
                                .stream_fade(true),
                        )
                    })
                    .when(finished && !message.sources.is_empty(), |this| {
                        let view = view.clone();
                        this.child(
                            Sources::new(("sources", id))
                                .sources(message.sources)
                                .open(message.sources_open)
                                .on_toggle(move |open, _, cx| {
                                    let open = *open;
                                    _ = view.update(cx, |this, cx| {
                                        this.set_message_flag(id, cx, |m| m.sources_open = open)
                                    });
                                }),
                        )
                    })
                    .when_some(message.error, |this, error| {
                        this.child(div().text_sm().text_color(cx.theme().danger).child(error))
                    }),
            )
            .when(finished, |this| {
                let view = view.clone();
                this.footer(
                    MessageFooter::new()
                        .gap_1()
                        .when(!text.is_empty(), |this| {
                            this.child(
                                Clipboard::new(("copy", id))
                                    .accessibility_label("Copy response")
                                    .value(text),
                            )
                        })
                        .child(
                            Button::new(("regenerate", id))
                                .ghost()
                                .xsmall()
                                .icon(IconName::RefreshCw)
                                .accessibility_label("Regenerate response")
                                .on_click(move |_, _, cx| {
                                    _ = view.update(cx, |this, cx| this.regenerate(cx));
                                }),
                        ),
                )
            })
            .into_any_element()
    }

    /// A generated image: a shimmering placeholder until the first partial
    /// arrives, then the picture refining in place.
    fn render_generated_image(
        message_id: usize,
        image: ChatImage,
        view: WeakEntity<Self>,
        cx: &App,
    ) -> AnyElement {
        let theme = cx.theme();
        let frame = div()
            .relative()
            .size_96()
            .max_w_full()
            .overflow_hidden()
            .rounded(theme.radius_lg)
            .bg(theme.muted);

        let Some(picture) = image.image.clone() else {
            return frame
                .flex()
                .items_center()
                .justify_center()
                .child(
                    ShimmerText::new("Creating image…")
                        .id(SharedString::from(format!(
                            "creating-{message_id}-{}",
                            image.key
                        )))
                        .text_sm()
                        .text_color(theme.muted_foreground),
                )
                .into_any_element();
        };

        v_flex()
            .gap_1()
            .child(
                frame
                    .child(
                        img(picture.clone())
                            .size_full()
                            .object_fit(ObjectFit::Contain),
                    )
                    .when(!image.done, |this| {
                        this.child(
                            div()
                                .absolute()
                                .bottom_2()
                                .left_2()
                                .px_2()
                                .rounded(theme.radius)
                                .bg(theme.background.opacity(0.8))
                                .child(
                                    ShimmerText::new("Refining…")
                                        .id(SharedString::from(format!(
                                            "refining-{message_id}-{}",
                                            image.key
                                        )))
                                        .text_xs(),
                                ),
                        )
                    }),
            )
            .when(image.done, |this| {
                this.child(
                    h_flex().child(
                        Button::new(SharedString::from(format!(
                            "save-{message_id}-{}",
                            image.key
                        )))
                        .ghost()
                        .xsmall()
                        .icon(Icon::new(LucideIcon::Download))
                        .label("Save image…")
                        .on_click(move |_, _, cx| {
                            let picture = Arc::clone(&picture);
                            _ = view.update(cx, |this, cx| this.save_image_as(picture, cx));
                        }),
                    ),
                )
            })
            .into_any_element()
    }

    pub(super) fn render_empty(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let ready = self.status() == PromptStatus::Ready;
        let heading: SharedString = match self.current_project() {
            Some(project) => format!("New chat in {}", project.name).into(),
            None => "What can I help with?".into(),
        };

        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_6()
            .px_6()
            .child(div().text_2xl().font_semibold().child(heading))
            .child(
                Suggestions::new()
                    .max_w(rems(40.))
                    .justify_center()
                    .children(SUGGESTIONS.iter().enumerate().map(|(ix, text)| {
                        Suggestion::new(("suggestion", ix), *text)
                            .disabled(!ready)
                            .on_click(cx.listener(|this, text: &SharedString, window, cx| {
                                this.send_text(text.to_string(), window, cx)
                            }))
                    })),
            )
    }
}
