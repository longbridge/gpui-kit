use gpui_kit::*;

use gpui_kit::component::{
    ActiveTheme, IconName, StyledExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    label::{HighlightsMatch, Label},
    v_flex,
};
use serde::Deserialize;

use crate::{section, story_toolbar_group};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = label_story, no_json)]
struct TogglePrefix;

pub struct LabelStory {
    focus_handle: gpui_kit::FocusHandle,
    masked: bool,
    highlights_text: SharedString,
    highlights_input: Entity<InputState>,
    prefix: bool,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for LabelStory {
    fn title() -> &'static str {
        "Label"
    }

    fn description() -> &'static str {
        "Display concise text with hierarchy, highlighting, and masking."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl LabelStory {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let highlights_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search labels")
                .clean_on_escape()
        });

        let _subscriptions =
            vec![
                cx.subscribe(&highlights_input, |this, state, e: &InputEvent, cx| {
                    if let InputEvent::Change = e {
                        this.highlights_text = state.read(cx).value();
                        cx.notify();
                    }
                }),
            ];

        Self {
            focus_handle: cx.focus_handle(),
            masked: false,
            highlights_text: Default::default(),
            highlights_input,
            prefix: false,
            _subscriptions,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    #[allow(unused)]
    fn on_click(checked: &bool, window: &mut Window, cx: &mut App) {
        println!("Check value changed: {}", checked);
    }

    fn highlights_text(&self) -> HighlightsMatch {
        if self.prefix {
            HighlightsMatch::Prefix(self.highlights_text.clone())
        } else {
            HighlightsMatch::Full(self.highlights_text.clone())
        }
    }
}
impl Focusable for LabelStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for LabelStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ht = self.highlights_text();

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .on_action(cx.listener(|this, _: &TogglePrefix, _, cx| {
                this.prefix = !this.prefix;
                cx.notify();
            }))
            .child(story_toolbar_group().dropdown_child(
                Button::new("label-options").label("Options"),
                {
                    let prefix = self.prefix;
                    move |menu, _, _| {
                        menu.menu_with_check("Prefix Match", prefix, Box::new(TogglePrefix))
                    }
                },
            ))
            .child(
                section("Default")
                    .description("Present primary text with optional supporting context.")
                    .w(px(560.))
                    .items_center()
                    .child(
                        v_flex()
                            .w(px(320.))
                            .gap_4()
                            .child(Label::new("Account details"))
                            .child(Label::new("Company address").secondary("Optional"))
                            .child(
                                Label::new("Workspace owner")
                                    .font_semibold()
                                    .secondary("Administrator"),
                            ),
                    ),
            )
            .child(
                section("Highlighting")
                    .description("Find matching text across Latin and CJK content.")
                    .w(px(560.))
                    .items_center()
                    .child(
                        v_flex()
                            .w(px(320.))
                            .gap_4()
                            .child(Input::new(&self.highlights_input))
                            .child(
                                v_flex()
                                    .w_full()
                                    .gap_3()
                                    .p_4()
                                    .rounded(cx.theme().radius_lg)
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .child(
                                        Label::new("Design system documentation")
                                            .highlights(ht.clone()),
                                    )
                                    // Keeps the mixed ASCII/CJK matching regression visible.
                                    .child(Label::new("AAA中文BB").highlights(ht.clone())),
                            ),
                    ),
            )
            .child(
                section("Layout")
                    .description("Labels support alignment and natural wrapping.")
                    .w(px(560.))
                    .items_center()
                    .child(
                        v_flex()
                            .w(px(320.))
                            .gap_4()
                            .child(
                                v_flex()
                                    .w_full()
                                    .gap_2()
                                    .p_4()
                                    .rounded(cx.theme().radius_lg)
                                    .bg(cx.theme().muted.opacity(0.4))
                                    .child(Label::new("Start aligned"))
                                    .child(Label::new("Center aligned").text_center())
                                    .child(Label::new("End aligned").text_right()),
                            )
                            .child(
                                div().w(px(220.)).child(
                                    Label::new(
                                        "Long labels wrap cleanly inside constrained layouts.",
                                    )
                                    .line_height(rems(1.5)),
                                ),
                            ),
                    ),
            )
            .child(
                section("Masked")
                    .description("Reveal or conceal sensitive values in place.")
                    .w(px(560.))
                    .items_center()
                    .child(
                        h_flex()
                            .w(px(320.))
                            .items_center()
                            .justify_between()
                            .p_4()
                            .rounded(cx.theme().radius_lg)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Available balance"),
                                    )
                                    .child(
                                        Label::new("$9,182.10")
                                            .text_2xl()
                                            .font_semibold()
                                            .masked(self.masked),
                                    ),
                            )
                            .child(
                                Button::new("btn-mask")
                                    .ghost()
                                    .icon(if self.masked {
                                        IconName::EyeOff
                                    } else {
                                        IconName::Eye
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.masked = !this.masked;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
    }
}
