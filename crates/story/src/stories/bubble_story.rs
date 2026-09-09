use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _, StyledExt as _,
    bubble::{
        Bubble, BubbleContent, BubbleGroup, BubbleReactionSide, BubbleReactions, BubbleVariant,
    },
    button::{Button, ButtonVariants as _},
    collapsible::Collapsible,
    h_flex,
    link::Link,
    message::MessageAlignment,
    popover::Popover,
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Window, div, rems,
};

use crate::{Story, section};

pub struct BubbleStory {
    focus_handle: FocusHandle,
    expanded: bool,
}

fn start_bubble() -> Bubble {
    Bubble::new().alignment(MessageAlignment::Start)
}

impl BubbleStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            expanded: false,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Story for BubbleStory {
    fn title() -> &'static str {
        "Bubble"
    }

    fn description() -> &'static str {
        "A styleable chat surface for text, rich content, and reactions."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for BubbleStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for BubbleStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("Variants")
                    .description("Semantic variants match the Base UI bubble treatments.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_4()
                    .child(start_bubble().child("A strong primary bubble."))
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Secondary)
                            .child("The neutral secondary bubble."),
                    )
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Muted)
                            .child("A lower-emphasis muted bubble."),
                    )
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Tinted)
                            .child("A softly tinted primary bubble."),
                    )
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Outline)
                            .child("A bordered bubble for rich content."),
                    )
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Destructive)
                            .child("A failed action with its reason in text."),
                    )
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Ghost)
                            .child("Ghost content is unframed and can use the full row width."),
                    ),
            )
            .child(
                section("Alignment")
                    .description("Use the same alignment value as Message.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Secondary)
                            .child("Incoming message"),
                    )
                    .child(
                        start_bubble()
                            .alignment(MessageAlignment::End)
                            .child("Outgoing message"),
                    ),
            )
            .child(
                section("Reactions")
                    .description("Use action for integrated Button controls; child keeps emoji and arbitrary reactions composable.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_8()
                    .py_6()
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Outline)
                            .child("This bubble has reaction feedback.")
                            .reactions(
                                BubbleReactions::new().action(
                                    Button::new("bubble-like")
                                        .ghost()
                                        .small()
                                        .label("👍 2")
                                        .tooltip("Like this message"),
                                ),
                            ),
                    )
                    .child(
                        Bubble::new()
                            .alignment(MessageAlignment::End)
                            .child("Reactions can attach to any edge.")
                            .reactions(
                                BubbleReactions::new()
                                    .side(BubbleReactionSide::Top)
                                    .alignment(MessageAlignment::Start)
                                    .child("✨ 1"),
                            ),
                    ),
            )
            .child(
                section("Group")
                    .description("Group consecutive bubbles using the shared spacing scale.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_5()
                    .child(
                        BubbleGroup::new()
                            .w_full()
                            .child(
                                start_bubble()
                                    .with_variant(BubbleVariant::Secondary)
                                    .child("Can you tell me what changed?"),
                            )
                            .child(
                                start_bubble()
                                    .with_variant(BubbleVariant::Secondary)
                                    .child("The registry route was stale."),
                            ),
                    )
                    .child(
                        BubbleGroup::new()
                            .w_full()
                            .child(
                                Bubble::new()
                                    .alignment(MessageAlignment::End)
                                    .child("I removed the stale route."),
                            )
                            .child(
                                Bubble::new()
                                    .alignment(MessageAlignment::End)
                                    .child("The updated registry is ready to review."),
                            ),
                    ),
            )
            .child(
                section("Links and buttons")
                    .description("Compose external links and application actions inside a bubble.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        start_bubble().with_variant(BubbleVariant::Outline).content(
                            BubbleContent::new().child(
                                v_flex()
                                    .gap_2()
                                    .child("The implementation guide is available online.")
                                    .child(
                                        Link::new("bubble-documentation-link")
                                            .href("https://gpui-kit.com/")
                                            .child("Open the component documentation"),
                                    ),
                            ),
                        ),
                    )
                    .child(
                        start_bubble().with_variant(BubbleVariant::Muted).content(
                            BubbleContent::new().child(
                                h_flex()
                                    .gap_3()
                                    .child("The generated report is ready.")
                                    .child(
                                        Button::new("bubble-open-report")
                                            .ghost()
                                            .small()
                                            .label("Open report"),
                                    ),
                            ),
                        ),
                    ),
            )
            .child(
                section("Collapsible content")
                    .description("Keep long responses readable with an explicit disclosure action.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .child(
                        start_bubble().with_variant(BubbleVariant::Muted).content(
                            BubbleContent::new().child(
                                Collapsible::new()
                                    .gap_2()
                                    .open(self.expanded)
                                    .child(
                                        "The accessibility review found two focus states that need more contrast.",
                                    )
                                    .content(
                                        v_flex()
                                            .gap_2()
                                            .child("Dialog and sheet controls already keep their focus rings visible.")
                                            .child("Update the menu focus token separately from its pointer hover state."),
                                    )
                                    .child(
                                        Button::new("bubble-toggle-details")
                                            .ghost()
                                            .small()
                                            .icon(IconName::ChevronsUpDown)
                                            .label(if self.expanded { "Show less" } else { "Show more" })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.expanded = !this.expanded;
                                                cx.notify();
                                            })),
                                    ),
                            ),
                        ),
                    ),
            )
            .child(
                section("Tooltip")
                    .description("Label icon-only reaction controls with their concrete meaning.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .py_4()
                    .child(
                        Bubble::new()
                            .alignment(MessageAlignment::End)
                            .child("The updated registry route is live.")
                            .reactions(
                                BubbleReactions::new().action(
                                    Button::new("bubble-delivery-details")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::CircleCheck)
                                        .tooltip("Read today at 4:32 PM"),
                                ),
                            ),
                    ),
            )
            .child(
                section("Popover")
                    .description("Use a semantic popover for contextual failure details.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .py_4()
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Destructive)
                            .child("The build command could not finish.")
                            .reactions(
                                BubbleReactions::new().p_0().child(
                                    Popover::new("bubble-error-popover")
                                        .trigger(
                                            Button::new("bubble-show-error")
                                                .ghost()
                                                .xsmall()
                                                .icon(IconName::Info)
                                                .rounded(cx.theme().radius_full())
                                                .tooltip("Show error details"),
                                        )
                                        .w_64()
                                        .gap_2()
                                        .child("Build command failed")
                                        .child("The workspace lockfile could not be found."),
                                ),
                            ),
                    ),
            )
            .child(
                section("Rich content")
                    .description("Any GPUI element can be placed directly in the surface.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        start_bubble().content(
                            BubbleContent::new().child(
                                h_flex()
                                    .gap_3()
                                    .child(
                                        div()
                                            .size_10()
                                            .rounded(cx.theme().radius)
                                            .bg(cx.theme().primary_foreground.opacity(0.18)),
                                    )
                                    .child(v_flex().child("design-notes.pdf").child(
                                        div().text_xs().opacity(0.75).child("2.4 MB · PDF"),
                                    )),
                            ),
                        ),
                    )
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Secondary)
                            .child(
                                "A longer message wraps naturally within the bubble's available width, preserving the shared text scale, comfortable reading rhythm, and leading alignment.",
                            ),
                    ),
            )
            .child(
                section("Custom style")
                    .description("Caller refinements override the surface defaults.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .child(
                        start_bubble().content(
                            BubbleContent::new()
                                .rounded(cx.theme().radius)
                                .bg(cx.theme().success.opacity(0.15))
                                .text_color(cx.theme().success)
                                .border_color(cx.theme().success.opacity(0.35))
                                .child("Custom semantic color"),
                        ),
                    ),
            )
    }
}
