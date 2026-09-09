use std::{rc::Rc, time::Duration};

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, IconName, Sizable as _, StyledExt as _,
    bubble::{Bubble, BubbleVariant},
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    marker::{Marker, MarkerContent, MarkerVariant},
    message::{Message, MessageAlignment, MessageContent},
    message_scroller::{MessageScroller, MessageScrollerState},
    v_flex,
};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Div, Entity, FocusHandle, Focusable,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StyleRefinement, Styled as _, Task, Window, div, prelude::FluentBuilder as _, rems,
};

use crate::{Story, section};

const INITIAL_STREAM_MESSAGE_COUNT: usize = 7;

#[derive(Clone)]
struct DemoMessage {
    id: usize,
    body: SharedString,
    sent: bool,
}

impl DemoMessage {
    fn new(id: usize, sent: bool, body: impl Into<SharedString>) -> Self {
        Self {
            id,
            body: body.into(),
            sent,
        }
    }
}

pub struct MessageScrollerStory {
    focus_handle: FocusHandle,
    scroller: Entity<MessageScrollerState>,
    stream_scroller: Entity<MessageScrollerState>,
    history_scroller: Entity<MessageScrollerState>,
    navigation_scroller: Entity<MessageScrollerState>,
    custom_scroller: Entity<MessageScrollerState>,
    application_scroller: Entity<MessageScrollerState>,
    empty_scroller: Entity<MessageScrollerState>,
    composer: Entity<InputState>,
    messages: Vec<DemoMessage>,
    stream_messages: Vec<DemoMessage>,
    history_messages: Vec<DemoMessage>,
    preview_messages: Vec<DemoMessage>,
    empty_messages: Vec<DemoMessage>,
    unread_index: usize,
    next_id: usize,
    streaming: bool,
    stream_task: Option<Task<()>>,
}

impl MessageScrollerStory {
    fn create_scroller(count: usize, cx: &mut Context<Self>) -> Entity<MessageScrollerState> {
        let state = cx.new(|cx| MessageScrollerState::new(count, cx));
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        state
    }

    fn preview_messages(first_id: usize, count: usize) -> Vec<DemoMessage> {
        (0..count)
            .map(|index| {
                DemoMessage::new(
                    first_id + index,
                    index % 3 == 2,
                    if index % 4 == 0 {
                        format!(
                            "Message {} wraps across more than one line to keep virtual-row measurement realistic.",
                            index + 1
                        )
                    } else {
                        format!("Conversation message {}", index + 1)
                    },
                )
            })
            .collect()
    }

    /// The scripted conversation for the main demo, mirroring shadcn's
    /// message-scroller demo: an AI chat without avatars or author names.
    fn conversation_script() -> Vec<DemoMessage> {
        [
            (true, "I'm building a chat for our app and the scroll behavior is driving me nuts. Every time the AI streams a reply, the whole thread jumps around."),
            (false, "That's the classic streaming scroll problem. Render the rows with MessageScroller — it follows the tail while the reader sits at the live edge, so streamed tokens land in place instead of shoving the thread around."),
            (true, "Okay, but what happens when someone scrolls up to re-read an older answer? I don't want to yank them back down."),
            (false, "You won't. Scrolling up releases tail following, so the reading position is preserved while new rows keep arriving below.\n\nA jump-to-latest button appears once the reader leaves the tail; one click returns to the newest row and resumes following."),
            (true, "And loading older history when they reach the top?"),
            (false, "prepend inserts the earlier rows while the row the reader is on stays anchored in place — no jump to the top, no lost context."),
            (true, "Last one — does it handle rows that change height while streaming?"),
            (false, "Yes. Remeasure just the growing row and the list keeps its anchor, so streamed markdown, images, and expanding content stay stable."),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (sent, body))| DemoMessage::new(index, sent, body))
        .collect()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let messages = Self::conversation_script();
        let mut stream_messages = Self::preview_messages(1_000, INITIAL_STREAM_MESSAGE_COUNT - 1);
        stream_messages.push(DemoMessage::new(
            1_000 + INITIAL_STREAM_MESSAGE_COUNT - 1,
            true,
            "How does streaming preserve my position?",
        ));
        let history_messages = Self::preview_messages(2_000, 14);
        let preview_messages = Self::preview_messages(3_000, 18);

        let scroller = Self::create_scroller(messages.len(), cx);
        let stream_scroller = Self::create_scroller(stream_messages.len(), cx);
        let history_scroller = Self::create_scroller(history_messages.len(), cx);
        let navigation_scroller = Self::create_scroller(preview_messages.len(), cx);
        let custom_scroller = Self::create_scroller(preview_messages.len(), cx);
        let application_scroller = Self::create_scroller(preview_messages.len(), cx);
        let empty_scroller = Self::create_scroller(0, cx);
        let composer = cx.new(|cx| InputState::new(window, cx).placeholder("Write a message…"));

        Self {
            focus_handle: cx.focus_handle(),
            scroller,
            stream_scroller,
            history_scroller,
            navigation_scroller,
            custom_scroller,
            application_scroller,
            empty_scroller,
            composer,
            messages,
            stream_messages,
            history_messages,
            preview_messages,
            empty_messages: Vec::new(),
            unread_index: 4,
            next_id: 10_000,
            streaming: false,
            stream_task: None,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn append_message(&mut self, cx: &mut Context<Self>) {
        let id = self.next_id;
        self.next_id += 1;
        self.messages.push(DemoMessage::new(
            id,
            true,
            format!("New message {}", self.messages.len() + 1),
        ));
        self.scroller
            .update(cx, |state, cx| _ = state.append(1, cx));
        cx.notify();
    }

    fn prepend_history(&mut self, cx: &mut Context<Self>) {
        const COUNT: usize = 5;
        let first_id = self.next_id;
        self.next_id += COUNT;
        let earlier = (0..COUNT).map(|offset| {
            DemoMessage::new(
                first_id + offset,
                false,
                format!("Earlier history {}", offset + 1),
            )
        });

        self.messages.splice(0..0, earlier);
        self.unread_index += COUNT;
        self.scroller
            .update(cx, |state, cx| _ = state.prepend(COUNT, cx));
        cx.notify();
    }

    fn send_message(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let body = self.composer.read(cx).value();
        if body.trim().is_empty() {
            return;
        }

        let id = self.next_id;
        self.next_id += 1;
        self.messages.push(DemoMessage::new(id, true, body));
        self.composer
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.scroller
            .update(cx, |state, cx| _ = state.append(1, cx));
        cx.notify();
    }

    fn reset_conversation(&mut self, cx: &mut Context<Self>) {
        self.messages = Self::conversation_script();
        self.unread_index = 4;
        let count = self.messages.len();
        self.scroller.update(cx, |state, cx| state.reset(count, cx));
        cx.notify();
    }

    fn scroll_to_unread(&mut self, cx: &mut Context<Self>) {
        let unread_index = self.unread_index;
        self.scroller.update(cx, |state, cx| {
            _ = state.scroll_to_item(unread_index, cx);
        });
    }

    fn start_stream(&mut self, cx: &mut Context<Self>) {
        if self.streaming {
            return;
        }

        const RESPONSE: &str = "The virtual list follows new content while you remain at the live edge. Scroll upward during this response and your reading position stays in place until you choose to return to the latest message.";

        let message_ix = self.stream_messages.len();
        let id = self.next_id;
        self.next_id += 1;
        self.streaming = true;
        self.stream_messages
            .push(DemoMessage::new(id, false, "Preparing response…"));
        self.stream_scroller
            .update(cx, |state, cx| _ = state.append(1, cx));
        cx.notify();

        self.stream_task = Some(cx.spawn(async move |story, cx| {
            for (token_ix, token) in RESPONSE.split_whitespace().enumerate() {
                cx.background_executor()
                    .timer(Duration::from_millis(110))
                    .await;

                let should_continue = story
                    .update(cx, |story, cx| {
                        if !story.streaming {
                            return false;
                        }

                        let Some(message) = story.stream_messages.get_mut(message_ix) else {
                            return false;
                        };
                        message.body = if token_ix == 0 {
                            token.into()
                        } else {
                            format!("{} {token}", message.body).into()
                        };

                        story.stream_scroller.update(cx, |state, cx| {
                            _ = state.remeasure_items(message_ix..message_ix + 1, cx);
                        });
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);

                if !should_continue {
                    return;
                }
            }

            _ = story.update(cx, |story, cx| {
                story.streaming = false;
                story.stream_task = None;
                cx.notify();
            });
        }));
    }

    fn stop_stream(&mut self, cx: &mut Context<Self>) {
        self.streaming = false;
        self.stream_task = None;
        cx.notify();
    }

    fn reset_stream(&mut self, cx: &mut Context<Self>) {
        self.streaming = false;
        self.stream_task = None;
        self.stream_messages.truncate(INITIAL_STREAM_MESSAGE_COUNT);
        self.stream_scroller.update(cx, |state, cx| {
            state.reset(INITIAL_STREAM_MESSAGE_COUNT, cx)
        });
        cx.notify();
    }

    fn expand_stream_response(&mut self, cx: &mut Context<Self>) {
        let Some(message_ix) = self.stream_messages.len().checked_sub(1) else {
            return;
        };
        let message = &mut self.stream_messages[message_ix];
        message.body = format!(
            "{}\n\nExpanded details demonstrate how images, Markdown, and progressive content can change an existing row without replacing its identity.",
            message.body
        )
        .into();

        self.stream_scroller.update(cx, |state, cx| {
            _ = state.remeasure_items(message_ix..message_ix + 1, cx);
        });
        cx.notify();
    }

    fn load_earlier_preview(&mut self, cx: &mut Context<Self>) {
        const COUNT: usize = 5;
        let first_id = self.next_id;
        self.next_id += COUNT;
        let earlier = (0..COUNT).map(|offset| {
            DemoMessage::new(
                first_id + offset,
                false,
                format!("Earlier saved message {}", offset + 1),
            )
        });

        self.history_messages.splice(0..0, earlier);
        self.history_scroller
            .update(cx, |state, cx| _ = state.prepend(COUNT, cx));
        cx.notify();
    }

    fn toggle_empty_conversation(&mut self, cx: &mut Context<Self>) {
        if self.empty_messages.is_empty() {
            self.empty_messages.push(DemoMessage::new(
                20_000,
                true,
                "A first message replaces the application-owned empty state.",
            ));
            self.empty_scroller
                .update(cx, |state, cx| _ = state.append(1, cx));
        } else {
            self.empty_messages.clear();
            self.empty_scroller
                .update(cx, |state, cx| state.reset(0, cx));
        }

        cx.notify();
    }

    fn render_message_row(message: DemoMessage, unread: bool) -> AnyElement {
        // Mirror shadcn's message-scroller demo: no avatars or author names,
        // sent rows on a muted surface, received rows as ghost text.
        let alignment = if message.sent {
            MessageAlignment::End
        } else {
            MessageAlignment::Start
        };
        let bubble = Bubble::new()
            .with_variant(if message.sent {
                BubbleVariant::Muted
            } else {
                BubbleVariant::Ghost
            })
            .child(message.body);

        v_flex()
            .w_full()
            .min_w_0()
            .gap_3()
            .when(unread, |this| {
                this.child(
                    Marker::new()
                        .with_variant(MarkerVariant::Separator)
                        .content(MarkerContent::new().child("Unread")),
                )
            })
            .child(
                div()
                    .id(("message-scroller-row", message.id))
                    .w_full()
                    .child(
                        Message::new()
                            .alignment(alignment)
                            .content(MessageContent::new().bubble(bubble)),
                    ),
            )
            .into_any_element()
    }

    fn preview_frame(scroller: MessageScroller, cx: &Context<Self>) -> Div {
        div()
            .w(rems(26.))
            .max_w_full()
            .h(rems(19.))
            .overflow_hidden()
            .rounded(cx.theme().radius_2xl())
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(scroller.with_bottom_fade(cx.theme().background).size_full())
    }
}

impl Story for MessageScrollerStory {
    fn title() -> &'static str {
        "MessageScroller"
    }

    fn description() -> &'static str {
        "A virtualized message list with tail following, unread navigation, and anchor preservation."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for MessageScrollerStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MessageScrollerStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let messages = Rc::new(self.messages.clone());
        let stream_messages = Rc::new(self.stream_messages.clone());
        let history_messages = Rc::new(self.history_messages.clone());
        let navigation_messages = Rc::new(self.preview_messages.clone());
        let custom_messages = navigation_messages.clone();
        let application_messages = navigation_messages.clone();
        let empty_messages = Rc::new(self.empty_messages.clone());
        let unread_index = self.unread_index;
        let streaming = self.streaming;
        let stream_has_response = self.stream_messages.len() > INITIAL_STREAM_MESSAGE_COUNT;
        let custom_scrolled_up = self.custom_scroller.read(cx).is_scrolled_up();
        let application_scrolled_up = self.application_scroller.read(cx).is_scrolled_up();
        let empty = self.empty_messages.is_empty();
        let status = {
            let state = self.scroller.read(cx);
            format!(
                "Following tail: {} · Scrolled up: {}",
                state.is_following_tail(),
                state.is_scrolled_up()
            )
        };

        v_flex()
            .gap_4()
            .child(
            section("Conversation")
                .description(
                    "Scroll upward, append a row, jump to unread, or prepend history to exercise each behavior.",
                )
                .max_w(rems(45.))
                .v_flex()
                .gap_3()
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("message-scroller-append")
                                .label("Append")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.append_message(cx)
                                })),
                        )
                        .child(
                            Button::new("message-scroller-prepend")
                                .label("Prepend history")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.prepend_history(cx)
                                })),
                        )
                        .child(
                            Button::new("message-scroller-unread")
                                .label("Scroll to unread")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.scroll_to_unread(cx)
                                })),
                        ),
                )
                .child(
                    v_flex()
                        .w_96()
                        .max_w_full()
                        .h(rems(35.))
                        .overflow_hidden()
                        .rounded(cx.theme().radius_4xl())
                        .border_1()
                        .border_color(cx.theme().border)
                        .bg(cx.theme().background)
                        .text_color(cx.theme().foreground)
                        .child(
                            h_flex()
                                .w_full()
                                .items_start()
                                .justify_between()
                                .gap_2()
                                .p_5()
                                .border_b_1()
                                .border_color(cx.theme().border)
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(div().font_semibold().child("New Chat"))
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("How can I help you today?"),
                                        ),
                                )
                                .child(
                                    Button::new("message-scroller-reset")
                                        .outline()
                                        .icon(IconName::RotateCw)
                                        .rounded(cx.theme().radius_full())
                                        .tooltip("Reset conversation")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.reset_conversation(cx)
                                        })),
                                ),
                        )
                        .child(
                            div().flex_1().min_h_0().child(
                                MessageScroller::new(
                                    "message-scroller-demo",
                                    self.scroller.clone(),
                                    move |index, _, _| {
                                        let Some(message) = messages.get(index).cloned() else {
                                            return div().into_any_element();
                                        };

                                        Self::render_message_row(message, index == unread_index)
                                    },
                                )
                                .with_list_style(StyleRefinement::default().p_5())
                                .with_bottom_fade(cx.theme().background),
                            ),
                        )
                        .child(
                            div().w_full().px_5().pb_5().child(
                                v_flex()
                                    .w_full()
                                    .gap_1()
                                    .p_2()
                                    .rounded_xl()
                                    .bg(cx.theme().muted)
                                    .child(Input::new(&self.composer).appearance(false))
                                    .child(
                                        h_flex().w_full().justify_end().child(
                                            Button::new("message-scroller-send")
                                                .primary()
                                                .icon(IconName::ArrowUp)
                                                .rounded(cx.theme().radius_full())
                                                .tooltip("Send")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.send_message(window, cx)
                                                })),
                                        ),
                                    ),
                            ),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(status),
                ),
        )
            .child(
                section("Streaming responses")
                    .description(
                        "Append one assistant response, grow its text progressively, and remeasure only that row.",
                    )
                    .max_w(rems(45.))
                    .v_flex()
                    .gap_3()
                    .child(
                        h_flex()
                            .flex_wrap()
                            .gap_2()
                            .child(
                                Button::new("message-scroller-start-stream")
                                    .label("Stream response")
                                    .disabled(streaming)
                                    .on_click(cx.listener(|this, _, _, cx| this.start_stream(cx))),
                            )
                            .child(
                                Button::new("message-scroller-stop-stream")
                                    .outline()
                                    .label("Stop")
                                    .disabled(!streaming)
                                    .on_click(cx.listener(|this, _, _, cx| this.stop_stream(cx))),
                            )
                            .child(
                                Button::new("message-scroller-expand-response")
                                    .label("Expand response")
                                    .disabled(streaming || !stream_has_response)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.expand_stream_response(cx)
                                    })),
                            )
                            .child(
                                Button::new("message-scroller-reset-stream")
                                    .ghost()
                                    .label("Reset")
                                    .on_click(cx.listener(|this, _, _, cx| this.reset_stream(cx))),
                            ),
                    )
                    .child(Self::preview_frame(
                        MessageScroller::new(
                            "message-scroller-streaming",
                            self.stream_scroller.clone(),
                            move |index, _, _| {
                                stream_messages
                                    .get(index)
                                    .cloned()
                                    .map(|message| Self::render_message_row(message, false))
                                    .unwrap_or_else(|| div().into_any_element())
                            },
                        )
                        .with_list_style(StyleRefinement::default().p_4()),
                        cx,
                    ))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(if streaming {
                                "Streaming · existing row is remeasured as each token arrives"
                            } else {
                                "Idle · scroll upward before streaming to preserve reading position"
                            }),
                    ),
            )
            .child(
                section("Loading earlier messages")
                    .description(
                        "Prepend history without disturbing the currently visible message anchor.",
                    )
                    .max_w(rems(45.))
                    .v_flex()
                    .gap_3()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("message-scroller-load-earlier")
                                    .label("Load five earlier")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.load_earlier_preview(cx)
                                    })),
                            )
                            .child(
                                Button::new("message-scroller-history-start")
                                    .outline()
                                    .label("Scroll to oldest")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.history_scroller.update(cx, |state, cx| {
                                            _ = state.scroll_to_item(0, cx);
                                        });
                                    })),
                            ),
                    )
                    .child(Self::preview_frame(
                        MessageScroller::new(
                            "message-scroller-history",
                            self.history_scroller.clone(),
                            move |index, _, _| {
                                history_messages
                                    .get(index)
                                    .cloned()
                                    .map(|message| Self::render_message_row(message, false))
                                    .unwrap_or_else(|| div().into_any_element())
                            },
                        )
                        .with_list_style(StyleRefinement::default().p_4()),
                        cx,
                    )),
            )
            .child(
                section("Jumping to messages")
                    .description(
                        "Applications resolve stable message IDs to their current row indices.",
                    )
                    .max_w(rems(45.))
                    .v_flex()
                    .gap_3()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("message-scroller-first-message")
                                    .label("First message")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.navigation_scroller.update(cx, |state, cx| {
                                            _ = state.scroll_to_item(0, cx);
                                        });
                                    })),
                            )
                            .child(
                                Button::new("message-scroller-middle-message")
                                    .label("Message 9")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.navigation_scroller.update(cx, |state, cx| {
                                            _ = state.scroll_to_item(8, cx);
                                        });
                                    })),
                            )
                            .child(
                                Button::new("message-scroller-last-message")
                                    .outline()
                                    .label("Latest")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.navigation_scroller
                                            .update(cx, |state, cx| state.scroll_to_end(cx));
                                    })),
                            ),
                    )
                    .child(Self::preview_frame(
                        MessageScroller::new(
                            "message-scroller-navigation",
                            self.navigation_scroller.clone(),
                            move |index, _, _| {
                                navigation_messages
                                    .get(index)
                                    .cloned()
                                    .map(|message| Self::render_message_row(message, false))
                                    .unwrap_or_else(|| div().into_any_element())
                            },
                        )
                        .with_list_style(StyleRefinement::default().p_4()),
                        cx,
                    )),
            )
            .child(
                section("Empty conversation")
                    .description(
                        "The application owns empty states and switches to a virtual list when data arrives.",
                    )
                    .max_w(rems(45.))
                    .v_flex()
                    .gap_3()
                    .child(
                        Button::new("message-scroller-toggle-empty")
                            .label(if empty { "Add first message" } else { "Clear conversation" })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_empty_conversation(cx)
                            })),
                    )
                    .child(
                        div()
                            .w(rems(26.))
                            .max_w_full()
                            .h(rems(14.))
                            .overflow_hidden()
                            .rounded(cx.theme().radius_2xl())
                            .border_1()
                            .border_color(cx.theme().border)
                            .bg(cx.theme().background)
                            .when(empty, |this| {
                                this.child(
                                    v_flex()
                                        .size_full()
                                        .items_center()
                                        .justify_center()
                                        .gap_1()
                                        .child(div().font_semibold().child("No messages yet"))
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("Send a message to start the conversation"),
                                        ),
                                )
                            })
                            .when(!empty, |this| {
                                this.child(
                                    MessageScroller::new(
                                        "message-scroller-empty",
                                        self.empty_scroller.clone(),
                                        move |index, _, _| {
                                            empty_messages
                                                .get(index)
                                                .cloned()
                                                .map(|message| {
                                                    Self::render_message_row(message, false)
                                                })
                                                .unwrap_or_else(|| div().into_any_element())
                                        },
                                    )
                                    .with_list_style(StyleRefinement::default().p_4())
                                    .with_bottom_fade(cx.theme().background)
                                    .size_full(),
                                )
                            }),
                    ),
            )
            .child(
                section("Custom jump button")
                    .description(
                        "Change the built-in button, transition, tooltip, list spacing, and row styles.",
                    )
                    .max_w(rems(45.))
                    .v_flex()
                    .gap_3()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("message-scroller-show-custom-jump")
                                    .label("Reveal jump button")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.custom_scroller.update(cx, |state, cx| {
                                            _ = state.scroll_to_item(0, cx);
                                        });
                                    })),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(if custom_scrolled_up { "Visible" } else { "Hidden" }),
                            ),
                    )
                    .child(Self::preview_frame(
                        MessageScroller::new(
                            "message-scroller-custom-control",
                            self.custom_scroller.clone(),
                            move |index, _, _| {
                                custom_messages
                                    .get(index)
                                    .cloned()
                                    .map(|message| Self::render_message_row(message, false))
                                    .unwrap_or_else(|| div().into_any_element())
                            },
                        )
                        .with_content_style(StyleRefinement::default().bg(cx.theme().background))
                        .with_list_style(StyleRefinement::default().p_4())
                        .with_row_style(StyleRefinement::default().pb_4())
                        .with_jump_button_label("Return to the latest message")
                        .with_jump_button_renderer(|button| {
                            button.outline().small().label("Latest")
                        })
                        .with_jump_button_style(
                            StyleRefinement::default().rounded(cx.theme().radius_lg),
                        )
                        .with_jump_button_transition(Duration::from_millis(350)),
                        cx,
                    )),
            )
            .child(
                section("Application-owned controls")
                    .description(
                        "Disable built-in chrome and place the jump action wherever the product needs it.",
                    )
                    .max_w(rems(45.))
                    .v_flex()
                    .gap_3()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("message-scroller-application-scroll-up")
                                    .label("Scroll to first")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.application_scroller.update(cx, |state, cx| {
                                            _ = state.scroll_to_item(0, cx);
                                        });
                                    })),
                            )
                            .child(
                                Button::new("message-scroller-application-latest")
                                    .outline()
                                    .label("Jump to latest")
                                    .disabled(!application_scrolled_up)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.application_scroller
                                            .update(cx, |state, cx| state.scroll_to_end(cx));
                                    })),
                            ),
                    )
                    .child(Self::preview_frame(
                        MessageScroller::new(
                            "message-scroller-application-controls",
                            self.application_scroller.clone(),
                            move |index, _, _| {
                                application_messages
                                    .get(index)
                                    .cloned()
                                    .map(|message| Self::render_message_row(message, false))
                                    .unwrap_or_else(|| div().into_any_element())
                            },
                        )
                        .jump_button(false)
                        .scrollbar(false)
                        .with_list_style(StyleRefinement::default().p_4()),
                        cx,
                    )),
            )
    }
}
