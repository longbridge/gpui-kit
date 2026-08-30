use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, Styled, Subscription, Window, div, px,
};
use gpui_component::{
    ActiveTheme as _, Sizable, Size, StyledExt as _,
    button::{Button, ButtonVariants as _},
    dialog::{Dialog, DialogClose, DialogDescription, DialogFooter, DialogHeader, DialogTitle},
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex,
    input::InputState,
    progress::Progress,
    questionnaire::{
        Questionnaire, QuestionnaireActions, QuestionnaireChoice, QuestionnaireChoiceDefinition,
        QuestionnaireChoiceDescription, QuestionnaireChoices, QuestionnaireDescription,
        QuestionnaireError, QuestionnaireEvent, QuestionnaireInput, QuestionnaireInputDefinition,
        QuestionnaireItem, QuestionnaireItemDefinition, QuestionnaireNext, QuestionnairePrevious,
        QuestionnaireProgress, QuestionnaireShortcutMode, QuestionnaireSkip, QuestionnaireState,
        QuestionnaireSubmission, QuestionnaireSubmit, QuestionnaireTitle,
    },
    stepper::{Stepper, StepperItem},
    v_flex,
};

use crate::{ChangeStorySize, section, story_toolbar};

pub struct QuestionnaireStory {
    focus_handle: FocusHandle,
    size: Size,
    state: Entity<QuestionnaireState>,
    validation_state: Entity<QuestionnaireState>,
    external_state: Entity<QuestionnaireState>,
    control_state: Entity<QuestionnaireState>,
    letters_state: Entity<QuestionnaireState>,
    numbers_state: Entity<QuestionnaireState>,
    edge_state: Entity<QuestionnaireState>,
    dialog_state: Entity<QuestionnaireState>,
    size_states: Vec<(Size, Entity<QuestionnaireState>)>,
    event_log: Vec<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for QuestionnaireStory {
    fn title() -> &'static str {
        "Questionnaire"
    }

    fn description() -> &'static str {
        "Composable multi-step questions with answers, validation, progress, and navigation."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl QuestionnaireStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn input(
        window: &mut Window,
        cx: &mut Context<Self>,
        placeholder: &'static str,
        default_value: Option<&'static str>,
    ) -> Entity<InputState> {
        cx.new(|cx| {
            let input = InputState::new(window, cx).placeholder(placeholder);
            if let Some(value) = default_value {
                input.default_value(value)
            } else {
                input
            }
        })
    }

    fn state(
        items: Vec<QuestionnaireItemDefinition>,
        cx: &mut Context<Self>,
    ) -> Entity<QuestionnaireState> {
        cx.new(|cx| {
            QuestionnaireState::new(items, cx)
                .expect("Questionnaire Story definitions must be valid")
        })
    }

    fn shortcut_state(
        items: Vec<QuestionnaireItemDefinition>,
        mode: QuestionnaireShortcutMode,
        cx: &mut Context<Self>,
    ) -> Entity<QuestionnaireState> {
        cx.new(|cx| {
            QuestionnaireState::new(items, cx)
                .expect("Questionnaire Story definitions must be valid")
                .with_shortcuts(mode)
        })
    }

    fn single_item(name: &'static str, label: &'static str) -> Vec<QuestionnaireItemDefinition> {
        vec![QuestionnaireItemDefinition::new(name, label).with_choices([
            QuestionnaireChoiceDefinition::new("first", "First choice"),
            QuestionnaireChoiceDefinition::new("second", "Second choice"),
            QuestionnaireChoiceDefinition::new("third", "Third choice"),
        ])]
    }

    fn main_items(window: &mut Window, cx: &mut Context<Self>) -> Vec<QuestionnaireItemDefinition> {
        let direction_input = Self::input(window, cx, "Type another direction…", None);
        let tools_input = Self::input(window, cx, "Add another tool…", Some("Terminal"));

        vec![
            QuestionnaireItemDefinition::new("direction", "What should we prototype next?")
                .with_required(true)
                .with_description("Choose one direction or write your own.")
                .with_choices([
                    QuestionnaireChoiceDefinition::new("delegation", "Delegation")
                        .with_description("Show how work moves to a specialist.")
                        .with_default_selected(true),
                    QuestionnaireChoiceDefinition::new("questions", "Question prompts")
                        .with_description("Show choices while the interface waits."),
                    QuestionnaireChoiceDefinition::new("both", "Both together"),
                ])
                .with_input(QuestionnaireInputDefinition::new(
                    direction_input,
                    "Another direction",
                )),
            QuestionnaireItemDefinition::new("tools", "Which tools do you use?")
                .with_multiple(true)
                .with_description("Choose any that belong in the prototype.")
                .with_choices([
                    QuestionnaireChoiceDefinition::new("editor", "Editor"),
                    QuestionnaireChoiceDefinition::new("terminal", "Terminal"),
                    QuestionnaireChoiceDefinition::new("browser", "Browser").with_disabled(true),
                ])
                .with_input(QuestionnaireInputDefinition::new(
                    tools_input,
                    "Another tool",
                )),
            QuestionnaireItemDefinition::new("tone", "What tone should the interface use?")
                .with_description("This optional question can be intentionally skipped.")
                .with_choices([
                    QuestionnaireChoiceDefinition::new("direct", "Direct"),
                    QuestionnaireChoiceDefinition::new("warm", "Warm"),
                ]),
            QuestionnaireItemDefinition::new("advanced", "Advanced preferences")
                .with_disabled(true)
                .with_choices([QuestionnaireChoiceDefinition::new(
                    "enabled",
                    "Enable advanced options",
                )]),
        ]
    }

    fn validation_items(
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<QuestionnaireItemDefinition> {
        let handle_input = Self::input(window, cx, "At least three characters", None);
        vec![
            QuestionnaireItemDefinition::new("handle", "Choose a public handle")
                .with_required(true)
                .with_description("The validator rejects short handles.")
                .with_input(QuestionnaireInputDefinition::new(
                    handle_input,
                    "Public handle",
                ))
                .with_validator(|context| {
                    if context
                        .answer()
                        .freeform()
                        .is_some_and(|value| value.as_ref().len() >= 3)
                    {
                        Ok(())
                    } else {
                        Err("Use at least three characters.".into())
                    }
                }),
            QuestionnaireItemDefinition::new("summary", "How should we summarize it?")
                .with_choices([
                    QuestionnaireChoiceDefinition::new("short", "Short"),
                    QuestionnaireChoiceDefinition::new("detailed", "Detailed"),
                ]),
        ]
    }

    fn item_view(
        state: &Entity<QuestionnaireState>,
        item: &'static str,
        choices: impl IntoIterator<Item = &'static str>,
        size: Size,
    ) -> QuestionnaireItem {
        let result = QuestionnaireItem::new(state, item)
            .with_size(size)
            .child(QuestionnaireTitle::new(state, item).with_size(size))
            .child(QuestionnaireDescription::new(state, item).with_size(size));

        let mut choice_parts = QuestionnaireChoices::new(state, item).with_size(size);
        for value in choices {
            let choice = QuestionnaireChoice::new(state, item, value).with_size(size);
            choice_parts = choice_parts.child(choice);
        }

        // Keep the freeform answer in the same answer group as fixed choices,
        // matching shadcn/ui's Questionnaire composition and spacing.
        choice_parts = choice_parts.child(QuestionnaireInput::new(state, item).with_size(size));

        result
            .child(choice_parts)
            .child(QuestionnaireError::new(state, item).with_size(size))
    }

    fn questionnaire_view(
        state: &Entity<QuestionnaireState>,
        size: Size,
        items: &[(&'static str, &'static [&'static str])],
    ) -> Questionnaire {
        let mut questionnaire = Questionnaire::new(state)
            .with_size(size)
            .child(QuestionnaireProgress::new(state).with_size(size));
        for (name, choices) in items {
            questionnaire =
                questionnaire.child(Self::item_view(state, name, choices.iter().copied(), size));
        }
        questionnaire.child(
            QuestionnaireActions::new(state)
                .with_size(size)
                .child(QuestionnairePrevious::new(state).with_size(size))
                .child(QuestionnaireSkip::new(state).with_size(size))
                .child(QuestionnaireNext::new(state).with_size(size))
                .child(QuestionnaireSubmit::new(state).with_size(size)),
        )
    }

    fn submission_summary(submission: &QuestionnaireSubmission) -> String {
        submission
            .items()
            .iter()
            .map(|item| format!("{}:{:?}={:?}", item.name(), item.status(), item.answer()))
            .collect::<Vec<_>>()
            .join(" · ")
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let main_state = Self::shortcut_state(
            Self::main_items(window, cx),
            QuestionnaireShortcutMode::Letters,
            cx,
        );
        let validation_state = Self::state(Self::validation_items(window, cx), cx);
        let external_state = Self::state(
            vec![
                QuestionnaireItemDefinition::new("server", "Which workspace should we connect?")
                    .with_required(true)
                    .with_choices([
                        QuestionnaireChoiceDefinition::new("personal", "Personal"),
                        QuestionnaireChoiceDefinition::new("team", "Team"),
                    ]),
            ],
            cx,
        );
        external_state.update(cx, |state, cx| {
            state
                .set_external_error("server", "This workspace is not available.", cx)
                .expect("external Story item exists");
        });

        let control_items = vec![
            QuestionnaireItemDefinition::new("first", "Which screen comes first?").with_choices([
                QuestionnaireChoiceDefinition::new("first", "First choice"),
                QuestionnaireChoiceDefinition::new("second", "Second choice"),
            ]),
            QuestionnaireItemDefinition::new("second", "Which screen comes next?").with_choices([
                QuestionnaireChoiceDefinition::new("first", "First choice"),
                QuestionnaireChoiceDefinition::new("second", "Second choice"),
            ]),
            QuestionnaireItemDefinition::new("conditional", "Conditional preferences")
                .with_choices([QuestionnaireChoiceDefinition::new("third", "Third choice")]),
        ];
        let control_state = cx.new(|cx| {
            QuestionnaireState::new(control_items, cx)
                .expect("Questionnaire Story definitions must be valid")
                .with_current_item("second")
                .expect("controlled Story item exists")
        });

        let shortcut_items = Self::single_item("shortcut", "Choose an answer with a shortcut");
        let letters_state = Self::shortcut_state(
            shortcut_items.clone(),
            QuestionnaireShortcutMode::Letters,
            cx,
        );
        let numbers_state =
            Self::shortcut_state(shortcut_items, QuestionnaireShortcutMode::Numbers, cx);

        let edge_state = Self::state(
            vec![
                QuestionnaireItemDefinition::new("edge", "No description and disabled choice")
                    .with_required(true)
                    .with_choices([
                        QuestionnaireChoiceDefinition::new("first", "Available choice"),
                        QuestionnaireChoiceDefinition::new("second", "Disabled choice")
                            .with_disabled(true),
                    ]),
            ],
            cx,
        );
        let dialog_state = Self::state(
            vec![
                QuestionnaireItemDefinition::new("dialog", "Which workspace should we open?")
                    .with_choices([
                        QuestionnaireChoiceDefinition::new("first", "Personal"),
                        QuestionnaireChoiceDefinition::new("second", "Team"),
                    ]),
            ],
            cx,
        );

        let mut size_states = Vec::new();
        for size in [Size::XSmall, Size::Small, Size::Medium, Size::Large] {
            let state = Self::state(
                vec![
                    QuestionnaireItemDefinition::new("size", "Choose a size").with_choices([
                        QuestionnaireChoiceDefinition::new("first", "Example choice"),
                    ]),
                ],
                cx,
            );
            size_states.push((size, state));
        }

        let subscriptions =
            vec![
                cx.subscribe(&main_state, |this, _, event: &QuestionnaireEvent, cx| {
                    let message = match event {
                        QuestionnaireEvent::CurrentItemChanged { current, .. } => {
                            format!("Current item: {}", current.as_deref().unwrap_or("none"))
                        }
                        QuestionnaireEvent::AnswerChanged(change) => {
                            format!("Answer changed: {} ({:?})", change.item(), change.status())
                        }
                        QuestionnaireEvent::Completed(submission) => {
                            format!("Completed: {}", Self::submission_summary(submission))
                        }
                        QuestionnaireEvent::Submit(submission) => {
                            format!("Submitted: {}", Self::submission_summary(submission))
                        }
                        _ => "Questionnaire event".to_string(),
                    };
                    this.event_log.push(message.into());
                    if this.event_log.len() > 4 {
                        this.event_log.remove(0);
                    }
                    cx.notify();
                }),
            ];

        Self {
            focus_handle: cx.focus_handle(),
            size: Size::Medium,
            state: main_state,
            validation_state,
            external_state,
            control_state,
            letters_state,
            numbers_state,
            edge_state,
            dialog_state,
            size_states,
            event_log: Vec::new(),
            _subscriptions: subscriptions,
        }
    }
}

impl Focusable for QuestionnaireStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for QuestionnaireStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let main = Self::questionnaire_view(
            &self.state,
            self.size,
            &[
                ("direction", &["delegation", "questions", "both"]),
                ("tools", &["editor", "terminal", "browser"]),
                ("tone", &["direct", "warm"]),
                ("advanced", &["enabled"]),
            ],
        );
        let validation = Self::questionnaire_view(
            &self.validation_state,
            self.size,
            &[("handle", &[]), ("summary", &["short", "detailed"])],
        );
        let external = Self::questionnaire_view(
            &self.external_state,
            self.size,
            &[("server", &["personal", "team"])],
        );
        let progress = self.state.read(cx).progress();
        let progress_value = if progress.total() == 0 {
            0.
        } else {
            progress.current() as f32 / progress.total() as f32 * 100.
        };
        let current_step = progress.current().saturating_sub(1);
        let event_log = self.event_log.clone();
        let state_snapshot = self.state.read(cx);
        let navigation = state_snapshot.navigation_state();
        let navigation_summary = format!(
            "Navigation: previous={} · next={} · skip={} · submit={} · can_confirm={}",
            navigation.is_previous_visible(),
            navigation.is_next_visible(),
            navigation.is_skip_visible(),
            navigation.is_submit_visible(),
            navigation.is_confirmable(),
        );
        let status_summary = ["direction", "tools", "tone", "advanced"]
            .into_iter()
            .filter_map(|name| {
                state_snapshot
                    .item_state(name)
                    .map(|item| format!("{}: {:?}", name, item.status()))
            })
            .collect::<Vec<_>>()
            .join(" · ");
        let answer_summary = state_snapshot
            .answers()
            .iter()
            .map(|(name, answer)| format!("{}={:?}", name, answer))
            .collect::<Vec<_>>()
            .join(" · ");
        let advanced_disabled = state_snapshot
            .item_state("advanced")
            .is_some_and(|item| item.is_disabled());
        let letters = Self::questionnaire_view(
            &self.letters_state,
            self.size,
            &[("shortcut", &["first", "second", "third"])],
        );
        let numbers = Self::questionnaire_view(
            &self.numbers_state,
            self.size,
            &[("shortcut", &["first", "second", "third"])],
        );
        let edge = Self::questionnaire_view(
            &self.edge_state,
            self.size,
            &[("edge", &["first", "second"])],
        );

        let control_state = self.control_state.clone();
        let custom_control = Questionnaire::new(&control_state)
            .with_size(self.size)
            .child(QuestionnaireProgress::new(&control_state).with_size(self.size))
            .child(Self::item_view(
                &control_state,
                "first",
                ["first", "second"],
                self.size,
            ))
            .child(Self::item_view(
                &control_state,
                "second",
                ["first", "second"],
                self.size,
            ))
            .child(Self::item_view(
                &control_state,
                "conditional",
                ["third"],
                self.size,
            ))
            .child(
                QuestionnaireActions::new(&control_state)
                    .with_size(self.size)
                    .child(
                        Button::new("questionnaire-custom-previous")
                            .outline()
                            .label("Back")
                            .on_click({
                                let state = control_state.clone();
                                move |_, window, cx| {
                                    state.update(cx, |state, cx| {
                                        state.go_previous(window, cx);
                                    });
                                }
                            }),
                    )
                    .child(
                        Button::new("questionnaire-custom-next")
                            .primary()
                            .label("Continue")
                            .on_click({
                                let state = control_state.clone();
                                move |_, window, cx| {
                                    state.update(cx, |state, cx| {
                                        state.go_next(window, cx);
                                    });
                                }
                            }),
                    )
                    .child(
                        Button::new("questionnaire-custom-submit")
                            .primary()
                            .label("Finish")
                            .on_click(move |_, window, cx| {
                                control_state.update(cx, |state, cx| {
                                    state.submit(window, cx);
                                });
                            }),
                    ),
            );
        let dialog_state = self.dialog_state.clone();
        let dialog = Dialog::new(cx)
            .trigger(
                Button::new("questionnaire-dialog-trigger")
                    .outline()
                    .label("Open Questionnaire Dialog"),
            )
            .p_0()
            .content(move |content, _, _| {
                content
                    .child(
                        DialogHeader::new()
                            .p_4()
                            .child(DialogTitle::new().child("Workspace setup"))
                            .child(DialogDescription::new().child(
                                "The host owns dismissal while Questionnaire owns the flow.",
                            )),
                    )
                    .child(
                        Self::questionnaire_view(
                            &dialog_state,
                            Size::Small,
                            &[("dialog", &["first", "second"])],
                        )
                        .px_4(),
                    )
                    .child(
                        DialogFooter::new().p_4().child(
                            DialogClose::new().child(
                                Button::new("questionnaire-dialog-close")
                                    .outline()
                                    .label("Cancel"),
                            ),
                        ),
                    )
            });
        let main_state = self.state.clone();
        let control_state_for_jump = self.control_state.clone();

        v_flex()
            .id("questionnaire-story")
            .track_focus(&self.focus_handle)
            .w_full()
            .gap_4()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .child(story_toolbar(self.size))
            .child(
                section("Complete flow")
                    .description("Required single choice, multiple choice, freeform input, skip, disabled item, and submit events.")
                    .w(px(448.))
                    .child(main),
            )
            .child(
                section("Validation and external errors")
                    .description("Next validates the active item; Submit returns to the first invalid item.")
                    .w(px(600.))
                    .child(validation)
                    .child(external),
            )
            .child(
                section("Navigation state and reset")
                    .description("The host can inspect status, restore answers, disable items, and reset to defaults.")
                    .w(px(600.))
                    .child(
                        Button::new("questionnaire-reset")
                            .outline()
                            .label("Reset complete flow")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.state.update(cx, |state, cx| state.reset(window, cx));
                            })),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "Current: {} · enabled items: {} · complete: {}",
                                self.state
                                    .read(cx)
                                    .current_item()
                                    .map(SharedString::as_ref)
                                    .unwrap_or("none"),
                                self.state.read(cx).total(),
                                self.state.read(cx).is_complete()
                            )),
                    )
                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child(status_summary))
                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child(navigation_summary))
                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child(if answer_summary.is_empty() {
                        "Answers: none".to_string()
                    } else {
                        format!("Answers: {answer_summary}")
                    }))
                    .child(
                        Button::new("questionnaire-controlled-current")
                            .outline()
                            .label("Set controlled current to first")
                            .on_click(move |_, window, cx| {
                                control_state_for_jump.update(cx, |state, cx| {
                                    let _ = state.set_current_item("first", window, cx);
                                });
                            }),
                    )
                    .child(
                        Button::new("questionnaire-conditional")
                            .outline()
                            .label(if advanced_disabled {
                                "Enable conditional item"
                            } else {
                                "Disable conditional item"
                            })
                            .on_click(move |_, window, cx| {
                                main_state.update(cx, |state, cx| {
                                    let disabled = state
                                        .item_state("advanced")
                                        .is_some_and(|item| item.is_disabled());
                                    let _ = state.set_item_disabled("advanced", !disabled, window, cx);
                                });
                            }),
                    )
                    .children(event_log.into_iter().map(|event| {
                        div().text_xs().text_color(cx.theme().muted_foreground).child(event)
                    })),
            )
            .child(
                section("Shortcuts")
                    .description("Letters and numbers are assigned only to enabled choices; the Kbd hints are part of the choice card.")
                    .w(px(600.))
                    .child(
                        h_flex()
                            .w_full()
                            .gap_4()
                            .child(v_flex().flex_1().gap_2().child(div().font_medium().child("Letters")).child(letters))
                            .child(v_flex().flex_1().gap_2().child(div().font_medium().child("Numbers")).child(numbers)),
                    ),
            )
            .child(
                section("Custom Progress and Stepper")
                    .description("Compose the state snapshot with existing progress components.")
                    .w(px(600.))
                    .child(
                        v_flex()
                            .w_full()
                            .gap_2()
                            .child(Progress::new("questionnaire-progress-custom").value(progress_value))
                            .child(
                                Stepper::new("questionnaire-stepper")
                                    .w_full()
                                    .selected_index(current_step)
                                    .items({
                                        let mut items = vec![
                                            StepperItem::new().child("Direction"),
                                            StepperItem::new().child("Tools"),
                                            StepperItem::new().child("Tone"),
                                        ];
                                        if !advanced_disabled {
                                            items.push(StepperItem::new().child("Advanced"));
                                        }
                                        items
                                    }),
                            ),
                    ),
            )
            .child(
                section("Controlled current, custom actions, and card composition")
                    .description("Container layout and content presentation remain host-owned.")
                    .w(px(600.))
                    .child(custom_control)
                    .child(
                        GroupBox::new()
                            .outline()
                            .title("Workspace setup")
                            .child(
                                QuestionnaireChoice::new(&self.edge_state, "edge", "first")
                                    .render_indicator(|choice, _, cx| {
                                        div()
                                            .size_4()
                                            .rounded_full()
                                            .bg(if choice.is_selected() {
                                                cx.theme().primary
                                            } else {
                                                cx.theme().muted
                                            })
                                            .into_any_element()
                                    })
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(div().font_medium().child("Available choice"))
                                            .child(QuestionnaireChoiceDescription::new().child(
                                                "A custom choice body using the same state.",
                                            )),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Place Questionnaire inside Dialog content when the host owns dismissal and cancellation."),
                    ),
            )
            .child(
                section("No description, disabled, invalid, and Dialog")
                    .description("The edge states are rendered with the same semantic parts; Dialog owns its close action.")
                    .w(px(600.))
                    .child(edge)
                    .child(dialog),
            )
            .child(
                section("All sizes")
                    .description("Medium is the base-nova default; the same composition scales through all four Size values.")
                    .w(px(600.))
                    .child(
                        h_flex()
                            .flex_wrap()
                            .gap_3()
                            .children(self.size_states.iter().map(|(size, state)| {
                                let label = match size {
                                    Size::XSmall => "XSmall",
                                    Size::Small => "Small",
                                    Size::Medium => "Medium",
                                    Size::Large => "Large",
                                    Size::Size(_) => "Custom",
                                };
                                v_flex()
                                    .w(px(135.))
                                    .gap_2()
                                    .child(div().font_medium().child(label))
                                    .child(Self::questionnaire_view(state, *size, &[("size", &["first"])]))
                            })),
                    ),
            )
    }
}
