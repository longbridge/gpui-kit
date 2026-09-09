use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    attachment::{
        Attachment, AttachmentActions, AttachmentContent, AttachmentDescription, AttachmentMedia,
        AttachmentTitle,
    },
    avatar::Avatar,
    bubble::{Bubble, BubbleReactions, BubbleVariant},
    button::{Button, ButtonVariants as _},
    message::{
        Message, MessageAlignment, MessageAvatar, MessageContent, MessageFooter, MessageGroup,
        MessageHeader,
    },
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Axis, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement as _, Render, StyleRefinement, Styled as _, Window, div, rems,
};

use crate::{Story, section};

pub struct MessageStory {
    focus_handle: FocusHandle,
}

impl MessageStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Story for MessageStory {
    fn title() -> &'static str {
        "Message"
    }

    fn description() -> &'static str {
        "Compose sender identity, metadata, rich content, and message actions."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for MessageStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MessageStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("Alignment")
                    .description("The message owns alignment for all of its named slots.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_5()
                    .child(
                        Message::new()
                            .avatar_slot(
                                MessageAvatar::new().child(Avatar::new().name("Alice").size_8()),
                            )
                            .header(MessageHeader::new().child("Alice").child("10:24 AM"))
                            .content(
                                MessageContent::new().bubble(
                                    Bubble::new()
                                        .with_variant(BubbleVariant::Secondary)
                                        .child("Can you review this?"),
                                ),
                            )
                            .footer(MessageFooter::new().child("Read")),
                    )
                    .child(
                        Message::new()
                            .alignment(MessageAlignment::End)
                            .avatar(Avatar::new().name("You").size_8())
                            .header(MessageHeader::new().child("You").child("10:25 AM"))
                            .content(
                                MessageContent::new().bubble(
                                    Bubble::new().child("Sure — I will send notes shortly."),
                                ),
                            )
                            .footer(MessageFooter::new().child("Delivered")),
                    ),
            )
            .child(
                section("Avatar")
                    .description("Use sender avatars, initials, or an empty slot to preserve alignment.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_5()
                    .child(
                        Message::new()
                            .avatar(
                                Avatar::new()
                                    .name("Alice Chen")
                                    .src("https://avatars.githubusercontent.com/u/5518?s=64")
                                    .size_8(),
                            )
                            .header(MessageHeader::new().child("Alice Chen"))
                            .content(
                                MessageContent::new().bubble(
                                    Bubble::new()
                                        .with_variant(BubbleVariant::Secondary)
                                        .child("The sender image falls back to initials when unavailable."),
                                ),
                            ),
                    )
                    .child(
                        Message::new()
                            .avatar(Avatar::new().name("Jordan Park").size_8())
                            .header(MessageHeader::new().child("Jordan Park"))
                            .content(
                                MessageContent::new().bubble(
                                    Bubble::new()
                                        .with_variant(BubbleVariant::Muted)
                                        .child("Initials remain available without an image."),
                                ),
                            ),
                    )
                    .child(
                        Message::new()
                            .avatar_slot(MessageAvatar::new().bg(cx.theme().transparent))
                            .content(
                                MessageContent::new().bubble(
                                    Bubble::new()
                                        .with_variant(BubbleVariant::Secondary)
                                        .child("An empty avatar slot keeps grouped responses aligned."),
                                ),
                            ),
                    ),
            )
            .child(
                section("Header and footer")
                    .description("Compose sender metadata, timestamps, and delivery status explicitly.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_5()
                    .child(
                        Message::new()
                            .avatar(Avatar::new().name("Support team").size_8())
                            .header(
                                MessageHeader::new()
                                    .child(div().text_color(cx.theme().foreground).child("Support"))
                                    .child("·")
                                    .child("10:42 AM"),
                            )
                            .content(
                                MessageContent::new().bubble(
                                    Bubble::new()
                                        .with_variant(BubbleVariant::Secondary)
                                        .child("Your issue has been assigned to the team."),
                                ),
                            )
                            .footer(MessageFooter::new().child("Read by 3 people")),
                    )
                    .child(
                        Message::new()
                            .alignment(MessageAlignment::End)
                            .header(MessageHeader::new().content_inset(false).child("You · Just now"))
                            .content(
                                MessageContent::new().bubble(
                                    Bubble::new().child("Thank you for the quick update."),
                                ),
                            )
                            .footer(
                                MessageFooter::new()
                                    .content_inset(false)
                                    .child("Delivered"),
                            ),
                    ),
            )
            .child(
                section("Actions")
                    .description("Keep copy, feedback, and retry actions keyboard-accessible.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_5()
                    .child(
                        Message::new()
                            .content(
                                MessageContent::new().bubble(
                                    Bubble::new()
                                        .with_variant(BubbleVariant::Muted)
                                        .child("The install failure is coming from the workspace package."),
                                ),
                            )
                            .footer(
                                MessageFooter::new()
                                    .gap_2()
                                    .child(
                                        Button::new("message-copy")
                                            .ghost()
                                            .xsmall()
                                            .icon(IconName::Copy)
                                            .tooltip("Copy message"),
                                    )
                                    .child(
                                        Button::new("message-like")
                                            .ghost()
                                            .xsmall()
                                            .icon(IconName::Heart)
                                            .tooltip("Like message"),
                                    )
                                    .child(
                                        Button::new("message-save")
                                            .ghost()
                                            .xsmall()
                                            .icon(IconName::Star)
                                            .tooltip("Save message"),
                                    ),
                            ),
                    )
                    .child(
                        Message::new()
                            .alignment(MessageAlignment::End)
                            .content(
                                MessageContent::new().bubble(
                                    Bubble::new()
                                        .with_variant(BubbleVariant::Destructive)
                                        .child("The response could not be sent."),
                                ),
                            )
                            .footer(
                                MessageFooter::new()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_color(cx.theme().danger)
                                            .child("Failed to send"),
                                    )
                                    .child(
                                        Button::new("message-retry")
                                            .ghost()
                                            .xsmall()
                                            .label("Retry"),
                                    ),
                            ),
                    ),
            )
            .child(
                section("Attachment")
                    .description("Mix image previews, file attachments, and text within one message.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_5()
                    .child(
                        Message::new()
                            .alignment(MessageAlignment::End)
                            .content(
                                MessageContent::new()
                                    .child(
                                        Attachment::new()
                                            .axis(Axis::Vertical)
                                            .media(
                                                AttachmentMedia::new().src(
                                                    "https://pub.lbkrs.com/files/202503/vEnnmgUM6bo362ya/sdk.svg",
                                                ),
                                            ),
                                    )
                                    .bubble(Bubble::new().child("Can you use this image on the cover?")),
                            ),
                    )
                    .child(
                        Message::new()
                            .avatar(Avatar::new().name("Alice").size_8())
                            .content(
                                MessageContent::new()
                                    .bubble(
                                        Bubble::new()
                                            .with_variant(BubbleVariant::Secondary)
                                            .child("Done. Here is the updated report."),
                                    )
                                    .child(
                                        Attachment::new()
                                            .media(
                                                AttachmentMedia::new()
                                                    .child(Icon::new(IconName::FileText)),
                                            )
                                            .content(
                                                AttachmentContent::new()
                                                    .title(AttachmentTitle::new("sales-dashboard.pdf"))
                                                    .description(AttachmentDescription::new("PDF · 2.4 MB")),
                                            )
                                            .actions(
                                                AttachmentActions::new().child(
                                                    Button::new("message-open-attachment")
                                                        .ghost()
                                                        .small()
                                                        .label("Open"),
                                                ),
                                            ),
                                    ),
                            ),
                    ),
            )
            .child(
                section("Multiple bubbles")
                    .description("A message can hold multiple surfaces, reactions, and long-form text.")
                    .max_w(rems(42.5))
                    .child(
                        Message::new()
                            .avatar(Avatar::new().name("Assistant").size_8())
                            .header(MessageHeader::new().child("Assistant").child("Just now"))
                            .content(
                                MessageContent::new()
                                    .gap_4()
                                    .bubble(
                                        Bubble::new()
                                            .with_variant(BubbleVariant::Secondary)
                                            .child("I reviewed the upload and message rendering paths."),
                                    )
                                    .bubble(
                                        Bubble::new()
                                            .with_variant(BubbleVariant::Muted)
                                            .child("Keep lifecycle state on the attachment, preserve the sender's alignment, and expose every action as an existing semantic Button.")
                                            .reactions(
                                                BubbleReactions::new().action(
                                                    Button::new("message-bubble-like")
                                                        .ghost()
                                                        .xsmall()
                                                        .label("👍 2")
                                                        .tooltip("Like this response"),
                                                ),
                                            ),
                                    ),
                            )
                            .footer(MessageFooter::new().child("Response complete")),
                    ),
            )
            .child(
                section("Group")
                    .description("Group consecutive messages while keeping each row composable.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .child(
                        MessageGroup::new()
                            .w_full()
                            .child(
                                Message::new()
                                    .avatar(Avatar::new().name("Alice").size_8())
                                    .header(MessageHeader::new().child("Alice"))
                                    .content(
                                        MessageContent::new().bubble(
                                            Bubble::new()
                                                .with_variant(BubbleVariant::Secondary)
                                                .child("I attached the draft."),
                                        ),
                                    ),
                            )
                            .child(
                                Message::new()
                                    .avatar_slot(MessageAvatar::new().bg(cx.theme().transparent))
                                    .content(
                                        MessageContent::new().bubble(
                                            Bubble::new()
                                                .with_variant(BubbleVariant::Secondary)
                                                .child("The second page needs attention."),
                                        ),
                                    ),
                            ),
                    ),
            )
            .child(
                section("Custom style")
                    .description("Every structural part accepts GPUI style refinements.")
                    .max_w(rems(42.5))
                    .child(
                        Message::new()
                            .p_3()
                            .rounded(cx.theme().radius_lg)
                            .bg(cx.theme().muted.opacity(0.35))
                            .header(MessageHeader::new().px_0().child("System"))
                            .content(
                                MessageContent::new().child("The conversation has been archived."),
                            )
                            .footer(MessageFooter::new().px_0().child("Just now")),
                    ),
            )
            .child(
                section("Ghost surface")
                    .description("Typed ghost bubbles automatically remove metadata insets.")
                    .max_w(rems(42.5))
                    .child(
                        Message::new()
                            .with_stack_style(StyleRefinement::default().gap_3())
                            .header(MessageHeader::new().child("System").child("Just now"))
                            .content(
                                MessageContent::new().bubble(
                                    Bubble::new()
                                        .with_variant(BubbleVariant::Ghost)
                                        .child("The conversation has been archived."),
                                ),
                            )
                            .footer(MessageFooter::new().child("No further action required")),
                    ),
            )
    }
}
