use gpui_kit::component::{
    ActiveTheme as _, Sizable, Size, StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    dialog::{Dialog, DialogClose, DialogDescription, DialogFooter, DialogHeader, DialogTitle},
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex,
    input::InputState,
    kbd::Kbd,
    questionnaire::{
        Questionnaire, QuestionnaireActions, QuestionnaireAnswer, QuestionnaireChoice,
        QuestionnaireChoiceDefinition, QuestionnaireChoiceDescription, QuestionnaireChoices,
        QuestionnaireDescription, QuestionnaireError, QuestionnaireEvent, QuestionnaireInput,
        QuestionnaireInputDefinition, QuestionnaireItem, QuestionnaireItemDefinition,
        QuestionnaireNext, QuestionnairePrevious, QuestionnaireProgress, QuestionnaireShortcutMode,
        QuestionnaireSkip, QuestionnaireState, QuestionnaireSubmission, QuestionnaireSubmit,
        QuestionnaireTitle,
    },
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

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
    card_state: Entity<QuestionnaireState>,
    custom_choice_state: Entity<QuestionnaireState>,
    dialog_state: Entity<QuestionnaireState>,
    event_log: Vec<SharedString>,
    keyboard_event_log: Vec<SharedString>,
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

    fn keyboard_items(
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<QuestionnaireItemDefinition> {
        let input = Self::input(window, cx, "Type without triggering A, B, or C…", None);
        vec![
            QuestionnaireItemDefinition::new("shortcut", "Choose an answer or type your own")
                .with_description(
                    "Use Up/Down to include the freeform input in the answer focus order.",
                )
                .with_choices([
                    QuestionnaireChoiceDefinition::new("first", "First choice"),
                    QuestionnaireChoiceDefinition::new("second", "Second choice"),
                    QuestionnaireChoiceDefinition::new("third", "Third choice"),
                ])
                .with_input(QuestionnaireInputDefinition::new(input, "Custom answer")),
            QuestionnaireItemDefinition::new("keyboard_review", "Confirm the keyboard result")
                .with_choices([
                    QuestionnaireChoiceDefinition::new("keep", "Keep it"),
                    QuestionnaireChoiceDefinition::new("change", "Change it"),
                ]),
        ]
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
    ) -> QuestionnaireItem {
        let result = QuestionnaireItem::new(state, item)
            .child(QuestionnaireTitle::new(state, item))
            .child(QuestionnaireDescription::new(state, item));

        let mut choice_parts = QuestionnaireChoices::new(state, item);
        for value in choices {
            let choice = QuestionnaireChoice::new(state, item, value);
            choice_parts = choice_parts.child(choice);
        }

        // Keep the freeform answer in the same answer group as fixed choices,
        // matching the ReUI questionnaire composition and spacing.
        choice_parts = choice_parts.child(QuestionnaireInput::new(state, item));

        result
            .child(choice_parts)
            .child(QuestionnaireError::new(state, item))
    }

    fn questionnaire_view(
        state: &Entity<QuestionnaireState>,
        size: Size,
        items: &[(&'static str, &'static [&'static str])],
    ) -> Questionnaire {
        // The root is the only place the scale is named; every part follows it.
        let mut questionnaire = Questionnaire::new(state)
            .with_size(size)
            .child(QuestionnaireProgress::new(state));
        for (name, choices) in items {
            questionnaire =
                questionnaire.child(Self::item_view(state, name, choices.iter().copied()));
        }
        questionnaire.child(
            QuestionnaireActions::new(state)
                .child(QuestionnairePrevious::new(state))
                .child(QuestionnaireSkip::new(state))
                .child(QuestionnaireNext::new(state))
                .child(QuestionnaireSubmit::new(state)),
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

    fn on_control_event(
        &mut self,
        state: &Entity<QuestionnaireState>,
        event: &QuestionnaireEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let QuestionnaireEvent::AnswerChanged(change) = event else {
            return;
        };
        if change.item() != "runtime" {
            return;
        }

        let cloud = state
            .read(cx)
            .answer("runtime")
            .is_some_and(|answer| answer.choices().iter().any(|value| value == "cloud"));
        let environment_disabled = state
            .read(cx)
            .item_state("environment")
            .is_some_and(|item| item.is_disabled());
        let should_disable_environment = !cloud;
        if environment_disabled != should_disable_environment {
            state.update(cx, |state, cx| {
                state
                    .set_item_disabled("environment", should_disable_environment, window, cx)
                    .expect("conditional Story item exists");
            });
        }
    }

    fn on_keyboard_event(
        &mut self,
        state: &Entity<QuestionnaireState>,
        event: &QuestionnaireEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mode = if state == &self.letters_state {
            "Letters"
        } else {
            "Numbers"
        };
        let message = match event {
            QuestionnaireEvent::CurrentItemChanged { current, .. } => {
                format!("{mode}: current={}", current.as_deref().unwrap_or("none"))
            }
            QuestionnaireEvent::AnswerChanged(change) => {
                format!("{mode}: answer={:?}", change.answer())
            }
            QuestionnaireEvent::Completed(_) => format!("{mode}: completed"),
            QuestionnaireEvent::Submit(_) => format!("{mode}: submitted"),
            _ => return,
        };
        self.keyboard_event_log.push(message.into());
        if self.keyboard_event_log.len() > 4 {
            self.keyboard_event_log.remove(0);
        }
        cx.notify();
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
            QuestionnaireItemDefinition::new("runtime", "Where will this workflow run?")
                .with_required(true)
                .with_description("Selecting Cloud enables the Environment question.")
                .with_choices([
                    QuestionnaireChoiceDefinition::new("local", "Local")
                        .with_default_selected(true),
                    QuestionnaireChoiceDefinition::new("cloud", "Cloud"),
                ]),
            QuestionnaireItemDefinition::new("delivery", "How should updates be delivered?")
                .with_choices([
                    QuestionnaireChoiceDefinition::new("guided", "Guided"),
                    QuestionnaireChoiceDefinition::new("automatic", "Automatic"),
                ]),
            QuestionnaireItemDefinition::new("environment", "Which cloud environment?")
                .with_disabled(true)
                .with_choices([
                    QuestionnaireChoiceDefinition::new("staging", "Staging"),
                    QuestionnaireChoiceDefinition::new("production", "Production"),
                ]),
        ];
        let control_state = Self::state(control_items, cx);

        let letters_state = Self::shortcut_state(
            Self::keyboard_items(window, cx),
            QuestionnaireShortcutMode::Letters,
            cx,
        );
        let numbers_state = Self::shortcut_state(
            Self::single_item("shortcut", "Choose an answer with a number"),
            QuestionnaireShortcutMode::Numbers,
            cx,
        );

        let card_state = Self::state(
            vec![
                QuestionnaireItemDefinition::new("card_scope", "Who can use this workspace?")
                    .with_required(true)
                    .with_choices([
                        QuestionnaireChoiceDefinition::new("team", "Team members"),
                        QuestionnaireChoiceDefinition::new("everyone", "Everyone"),
                    ]),
                QuestionnaireItemDefinition::new("card_updates", "Send setup updates?")
                    .with_choices([
                        QuestionnaireChoiceDefinition::new("yes", "Yes"),
                        QuestionnaireChoiceDefinition::new("no", "No"),
                    ]),
            ],
            cx,
        );
        let custom_choice_state = Self::shortcut_state(
            vec![
                QuestionnaireItemDefinition::new("custom", "Choose a presentation").with_choices([
                    QuestionnaireChoiceDefinition::new("compact", "Compact")
                        .with_description("Use a custom indicator and composed description."),
                    QuestionnaireChoiceDefinition::new("comfortable", "Comfortable"),
                ]),
            ],
            QuestionnaireShortcutMode::Letters,
            cx,
        );

        let dialog_state = Self::state(
            vec![
                QuestionnaireItemDefinition::new("dialog", "Which workspace should we open?")
                    .with_required(true)
                    .with_choices([
                        QuestionnaireChoiceDefinition::new("first", "Personal"),
                        QuestionnaireChoiceDefinition::new("second", "Team"),
                    ]),
                QuestionnaireItemDefinition::new(
                    "dialog_verification",
                    "How should we verify the setup?",
                )
                .with_required(true)
                .with_choices([
                    QuestionnaireChoiceDefinition::new("targeted", "Targeted checks"),
                    QuestionnaireChoiceDefinition::new("full", "Full verification"),
                ]),
            ],
            cx,
        );

        let subscriptions = vec![
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
            cx.subscribe_in(&control_state, window, Self::on_control_event),
            cx.subscribe_in(&letters_state, window, Self::on_keyboard_event),
            cx.subscribe_in(&numbers_state, window, Self::on_keyboard_event),
            cx.subscribe_in(
                &dialog_state,
                window,
                |_, _, event: &QuestionnaireEvent, window, cx| {
                    if matches!(event, QuestionnaireEvent::Submit(_)) {
                        window.close_dialog(cx);
                    }
                },
            ),
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
            card_state,
            custom_choice_state,
            dialog_state,
            event_log: Vec::new(),
            keyboard_event_log: Vec::new(),
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        let letters = Self::questionnaire_view(
            &self.letters_state,
            self.size,
            &[
                ("shortcut", &["first", "second", "third"]),
                ("keyboard_review", &["keep", "change"]),
            ],
        );
        let numbers = Self::questionnaire_view(
            &self.numbers_state,
            self.size,
            &[("shortcut", &["first", "second", "third"])],
        );
        let letters_snapshot = self.letters_state.read(cx);
        let keyboard_focus = if letters_snapshot.is_current_input_focused(window) {
            "freeform input".to_string()
        } else if let Some(choice) = letters_snapshot.focused_current_choice(window) {
            format!("choice {choice}")
        } else {
            "item group or none".to_string()
        };
        let letters_answer = letters_snapshot.answer("shortcut").unwrap_or_default();
        let letters_draft = letters_snapshot
            .input_state("shortcut")
            .map(|input| input.read(cx).value())
            .unwrap_or_default();
        let keyboard_event_log = self.keyboard_event_log.clone();

        let external_error = self
            .external_state
            .read(cx)
            .error("server")
            .and_then(|error| error.message().map(ToString::to_string))
            .unwrap_or_else(|| "none".to_string());

        let control_state = self.control_state.clone();
        let control_navigation = control_state.read(cx).navigation_state();
        let control_previous_visible = control_navigation.is_previous_visible();
        let control_skip_visible = control_navigation.is_skip_visible();
        let control_next_visible = control_navigation.is_next_visible();
        let control_submit_visible = control_navigation.is_submit_visible();
        let environment_enabled = control_state
            .read(cx)
            .item_state("environment")
            .is_some_and(|item| !item.is_disabled());
        let custom_control = Questionnaire::new(&control_state)
            .with_size(self.size)
            .child(QuestionnaireProgress::new(&control_state))
            .child(Self::item_view(
                &control_state,
                "runtime",
                ["local", "cloud"],
            ))
            .child(Self::item_view(
                &control_state,
                "delivery",
                ["guided", "automatic"],
            ))
            .child(Self::item_view(
                &control_state,
                "environment",
                ["staging", "production"],
            ))
            .child(
                QuestionnaireActions::new(&control_state)
                    .when(control_previous_visible, |actions| {
                        actions.child(
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
                    })
                    .when(control_skip_visible, |actions| {
                        actions.child(
                            Button::new("questionnaire-custom-skip")
                                .outline()
                                .ml_auto()
                                .label("Not now")
                                .on_click({
                                    let state = control_state.clone();
                                    move |_, window, cx| {
                                        state.update(cx, |state, cx| {
                                            state.skip_current(window, cx);
                                        });
                                    }
                                }),
                        )
                    })
                    .when(control_next_visible, |actions| {
                        actions.child(
                            Button::new("questionnaire-custom-next")
                                .primary()
                                .when(!control_skip_visible, |button| button.ml_auto())
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
                    })
                    .when(control_submit_visible, |actions| {
                        actions.child(
                            Button::new("questionnaire-custom-submit")
                                .primary()
                                .when(!control_skip_visible, |button| button.ml_auto())
                                .label("Finish")
                                .on_click({
                                    let state = control_state.clone();
                                    move |_, window, cx| {
                                        state.update(cx, |state, cx| {
                                            state.submit(window, cx);
                                        });
                                    }
                                }),
                        )
                    }),
            );

        let card =
            GroupBox::new()
                .outline()
                .title("Workspace access")
                .child(Self::questionnaire_view(
                    &self.card_state,
                    self.size,
                    &[
                        ("card_scope", &["team", "everyone"]),
                        ("card_updates", &["yes", "no"]),
                    ],
                ));

        let custom_choice_state = self.custom_choice_state.clone();
        let custom_choice = Questionnaire::new(&custom_choice_state)
            .with_size(self.size)
            .child(
                QuestionnaireItem::new(&custom_choice_state, "custom")
                    .child(QuestionnaireTitle::new(&custom_choice_state, "custom"))
                    .child(
                        QuestionnaireChoices::new(&custom_choice_state, "custom")
                            .child(
                                QuestionnaireChoice::new(&custom_choice_state, "custom", "compact")
                                    .content_style(StyleRefinement::default().gap_2())
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
                                    .render_shortcut(|choice, _, _| {
                                        let Some(shortcut) = choice.shortcut() else {
                                            return div().into_any_element();
                                        };
                                        let Ok(keystroke) =
                                            Keystroke::parse(&shortcut.to_lowercase())
                                        else {
                                            return div().into_any_element();
                                        };
                                        Kbd::new(keystroke).outline().into_any_element()
                                    })
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(div().font_medium().child("Compact"))
                                            .child(QuestionnaireChoiceDescription::new().child(
                                                "A custom indicator and composed description.",
                                            )),
                                    ),
                            )
                            .child(
                                QuestionnaireChoice::new(
                                    &custom_choice_state,
                                    "custom",
                                    "comfortable",
                                )
                                .indicator_style(StyleRefinement::default().opacity(0.65))
                                .shortcut_style(StyleRefinement::default().opacity(0.65)),
                            ),
                    )
                    .child(QuestionnaireError::new(&custom_choice_state, "custom")),
            )
            .child(
                QuestionnaireActions::new(&custom_choice_state)
                    .child(QuestionnaireSubmit::new(&custom_choice_state)),
            );

        let dialog_content_state = self.dialog_state.clone();
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
                                "Questionnaire validates the answer; the Dialog host owns close and cancel.",
                            )),
                    )
                    .child(
                        Questionnaire::new(&dialog_content_state)
                            .with_size(Size::Small)
                            .child(
                                QuestionnaireProgress::new(&dialog_content_state),
                            )
                            .child(Self::item_view(
                                &dialog_content_state,
                                "dialog",
                                ["first", "second"],))
                            .child(
                                Self::item_view(
                                    &dialog_content_state,
                                    "dialog_verification",
                                    ["targeted", "full"],),
                            )
                            .child(
                                DialogFooter::new()
                                    .child(
                                        DialogClose::new().child(
                                            Button::new("questionnaire-dialog-cancel")
                                                .outline()
                                                .label("Cancel"),
                                        ),
                                    )
                                    .child(
                                        QuestionnaireActions::new(&dialog_content_state)
                                            .child(
                                                QuestionnairePrevious::new(&dialog_content_state)
                                            )
                                            .child(
                                                QuestionnaireNext::new(&dialog_content_state)
                                            )
                                            .child(
                                                QuestionnaireSubmit::new(&dialog_content_state)
                                            ),
                                    ),
                            )
                            .px_4()
                            .pb_4(),
                    )
            });
        let control_state_for_jump = self.control_state.clone();
        let external_state_for_error = self.external_state.clone();
        let external_state_for_clear = self.external_state.clone();
        let external_state_for_fix = self.external_state.clone();

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
                    .description("Next validates the active item; external errors remain host-owned until cleared or fixed.")
                    .w(px(600.))
                    .child(div().font_medium().child("Internal validation"))
                    .child(validation)
                    .child(div().font_medium().child("External error lifecycle"))
                    .child(external)
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("questionnaire-external-reapply")
                                    .outline()
                                    .label("Apply server error")
                                    .on_click(move |_, _, cx| {
                                        external_state_for_error.update(cx, |state, cx| {
                                            state
                                                .set_external_error(
                                                    "server",
                                                    "This workspace is not available.",
                                                    cx,
                                                )
                                                .expect("external Story item exists");
                                        });
                                    }),
                            )
                            .child(
                                Button::new("questionnaire-external-clear")
                                    .outline()
                                    .label("Clear error")
                                    .on_click(move |_, _, cx| {
                                        external_state_for_clear.update(cx, |state, cx| {
                                            state
                                                .clear_external_error("server", cx)
                                                .expect("external Story item exists");
                                        });
                                    }),
                            )
                            .child(
                                Button::new("questionnaire-external-fix-submit")
                                    .primary()
                                    .label("Fix and submit again")
                                    .on_click(move |_, window, cx| {
                                        external_state_for_fix.update(cx, |state, cx| {
                                            state
                                                .set_answer(
                                                    "server",
                                                    QuestionnaireAnswer::new()
                                                        .with_choices(["team"]),
                                                    window,
                                                    cx,
                                                )
                                                .expect("external Story answer is valid");
                                            state
                                                .clear_external_error("server", cx)
                                                .expect("external Story item exists");
                                            state.submit(window, cx);
                                        });
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("External error: {external_error}")),
                    ),
            )
            .child(
                section("Navigation state")
                    .description("The host can inspect current item, answers, status, available actions, and events.")
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
                    .children(event_log.into_iter().map(|event| {
                        div().text_xs().text_color(cx.theme().muted_foreground).child(event)
                    })),
            )
            .child(
                section("Shortcuts and keyboard")
                    .description("The fixture exposes focus, answers, drafts, and events while testing the full keyboard contract.")
                    .w(px(600.))
                    .child(
                        v_flex()
                            .gap_1()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Up/Down: move through choices and the freeform input; text editing keeps native arrow behavior.")
                            .child("Left/Right: switch items outside text input and radio focus; Right requires an answer.")
                            .child("Enter: confirm a filled answer. Command/Ctrl+Enter: confirm the current item.")
                            .child("A–Z or 1–9: activate enabled choices. While input is focused, typed characters edit the draft and are not intercepted."),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .gap_4()
                            .child(v_flex().flex_1().gap_2().child(div().font_medium().child("Letters")).child(letters))
                            .child(v_flex().flex_1().gap_2().child(div().font_medium().child("Numbers")).child(numbers)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "Letters focus: {keyboard_focus} · answer={letters_answer:?} · draft={letters_draft:?}"
                            )),
                    )
                    .children(keyboard_event_log.into_iter().map(|event| {
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(event)
                    })),
            )
            .child(
                section("Controlled current, conditional items, and custom actions")
                    .description("The host synchronizes Environment from the Runtime answer; custom actions use NavigationState visibility.")
                    .w(px(600.))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("questionnaire-controlled-current")
                                    .outline()
                                    .label("Jump to runtime question")
                                    .on_click(move |_, window, cx| {
                                        control_state_for_jump.update(cx, |state, cx| {
                                            state
                                                .set_current_item("runtime", window, cx)
                                                .expect("controlled Story item exists");
                                        });
                                    }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(if environment_enabled {
                                        "Environment enabled: Runtime is Cloud"
                                    } else {
                                        "Environment disabled: choose Cloud on Runtime"
                                    }),
                            ),
                    )
                    .child(custom_control)
            )
            .child(
                section("Card and Dialog composition")
                    .description("GroupBox or Dialog owns the surface and its close behavior; the questionnaire keeps progress, items, and actions together.")
                    .w(px(600.))
                    .child(card)
                    .child(dialog),
            )
            .child(
                section("Custom choice composition")
                    .description("Customize indicator, content, shortcut renderers, and style seams while preserving Questionnaire state and behavior.")
                    .w(px(600.))
                    .child(custom_choice),
            )
    }
}
