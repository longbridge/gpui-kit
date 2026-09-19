//! Questionnaire: typed question and choice data plus the retained flow.
//!
//! The schema a `QuestionnaireState` is built from is a tree of plain Rust
//! values, and a state factory cannot take elements, so the questions arrive as
//! typed children the way `Tree`'s items do. The root builds the state, keeps
//! it across frames, and renders the default composition: progress, the active
//! question with its title, description, answers and error, then the
//! navigation actions.

use std::sync::Arc;

use gpui_component::questionnaire::{
    Questionnaire, QuestionnaireActions, QuestionnaireChoice, QuestionnaireChoiceDefinition,
    QuestionnaireChoices, QuestionnaireDescription, QuestionnaireError, QuestionnaireInput,
    QuestionnaireInputDefinition, QuestionnaireItem, QuestionnaireItemDefinition,
    QuestionnaireNext, QuestionnairePrevious, QuestionnaireProgress, QuestionnaireShortcutMode,
    QuestionnaireSkip, QuestionnaireState, QuestionnaireSubmit, QuestionnaireTitle,
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{
        self, AppContext as _, IntoElement as _, ParentElement as _, Refineable as _, Styled as _,
    },
};

use super::support::{bool_method, require_child, string_method};
use super::typed_child::{Carrier, take};

#[derive(Clone)]
struct ChoicePayload {
    value: String,
    label: String,
}

#[derive(Clone)]
enum ChoiceOp {
    Description(String),
    Disabled(bool),
    DefaultSelected(bool),
}

#[derive(Clone)]
struct ItemPayload {
    name: String,
    label: String,
}

#[derive(Clone)]
enum ItemOp {
    Description(String),
    Required(bool),
    Multiple(bool),
    Disabled(bool),
}

#[derive(Clone)]
struct InputPayload {
    state: ComponentArgument,
    label: String,
}

#[derive(Clone)]
struct RootPayload(String);

#[derive(Clone, Copy)]
enum RootOp {
    Shortcuts(QuestionnaireShortcutMode),
}

/// What the retained state was built from, as one hash. A question or choice
/// that changed means a different questionnaire, and `QuestionnaireState` fixes
/// its schema at construction, so the state is rebuilt rather than patched.
///
/// This runs on every frame of every questionnaire on screen, so it hashes the
/// schema in place instead of copying it into comparable values.
type Fingerprint = u64;

struct RetainedQuestionnaire {
    native: gpui::Entity<QuestionnaireState>,
    fingerprint: Fingerprint,
    /// A schema the state refused. The flow renders nothing and the script
    /// hears why, instead of a questionnaire that silently lost a question.
    error: Option<String>,
}

struct ChoiceMaterializer;

impl ComponentMaterializer for ChoiceMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<ChoicePayload>()
            .ok_or_else(|| anyhow::anyhow!("QuestionnaireChoice received an incompatible payload"))?
            .clone();
        let mut choice = QuestionnaireChoiceDefinition::new(payload.value, payload.label);
        for operation in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<ChoiceOp>().cloned())
            .collect::<Vec<_>>()
        {
            choice = match operation {
                ChoiceOp::Description(text) => choice.with_description(text),
                ChoiceOp::Disabled(value) => choice.with_disabled(value),
                ChoiceOp::DefaultSelected(value) => choice.with_default_selected(value),
            };
        }
        super::support::reject_style(request.take_style(), "QuestionnaireChoice")?;
        Ok(Carrier::new(choice).into_any_element())
    }
}

struct InputMaterializer;

impl ComponentMaterializer for InputMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<InputPayload>()
            .ok_or_else(|| anyhow::anyhow!("QuestionnaireInput received an incompatible payload"))?
            .clone();
        let state = request.with_state::<gpui::Entity<gpui_component::input::InputState>, _>(
            &payload.state,
            Clone::clone,
        )?;
        let mut input = QuestionnaireInputDefinition::new(state, payload.label);
        for operation in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<bool>().copied())
            .collect::<Vec<_>>()
        {
            input = input.with_disabled(operation);
        }
        super::support::reject_style(request.take_style(), "QuestionnaireInput")?;
        Ok(Carrier::new(input).into_any_element())
    }
}

struct ItemMaterializer;

impl ComponentMaterializer for ItemMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<ItemPayload>()
            .ok_or_else(|| anyhow::anyhow!("QuestionnaireItem received an incompatible payload"))?
            .clone();
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<ItemOp>().cloned())
            .collect::<Vec<_>>();
        let mut item = QuestionnaireItemDefinition::new(payload.name, payload.label);
        for operation in operations {
            item = match operation {
                ItemOp::Description(text) => item.with_description(text),
                ItemOp::Required(value) => item.with_required(value),
                ItemOp::Multiple(value) => item.with_multiple(value),
                ItemOp::Disabled(value) => item.with_disabled(value),
            };
        }
        super::support::reject_style(request.take_style(), "QuestionnaireItem")?;
        for mut child in request.take_typed_children()? {
            let name = child.component_name();
            require_child(
                "QuestionnaireItem",
                name,
                &["QuestionnaireChoice", "QuestionnaireInput"],
            )?;
            let freeform = name == Some("QuestionnaireInput");
            let mut element = request.materialize_child(&mut child)?;
            item = if freeform {
                item.with_input(take::<QuestionnaireInputDefinition>(
                    &mut element,
                    "QuestionnaireInput",
                )?)
            } else {
                item.with_choice(take::<QuestionnaireChoiceDefinition>(
                    &mut element,
                    "QuestionnaireChoice",
                )?)
            };
        }
        Ok(Carrier::new(item).into_any_element())
    }
}

struct RootMaterializer;

impl ComponentMaterializer for RootMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = request
            .payload()
            .downcast_ref::<RootPayload>()
            .ok_or_else(|| anyhow::anyhow!("Questionnaire received an incompatible payload"))?
            .0
            .clone();
        let shortcuts = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<RootOp>().copied())
            .fold(None, |_, operation| match operation {
                RootOp::Shortcuts(mode) => Some(mode),
            });

        let mut definitions = Vec::new();
        for mut child in request.take_typed_children()? {
            require_child(
                "Questionnaire",
                child.component_name(),
                &["QuestionnaireItem"],
            )?;
            let mut element = request.materialize_child(&mut child)?;
            definitions.push(take::<QuestionnaireItemDefinition>(
                &mut element,
                "QuestionnaireItem",
            )?);
        }

        let names: Vec<(gpui::SharedString, Vec<gpui::SharedString>)> = definitions
            .iter()
            .map(|item| {
                (
                    item.name().clone(),
                    item.choices()
                        .iter()
                        .map(|choice| choice.value().clone())
                        .collect(),
                )
            })
            .collect();
        let fingerprint = fingerprint(&definitions, shortcuts);
        let (state, error) = request.with_window_app(|window, cx| {
            let retained =
                window.use_keyed_state(format!("shell-questionnaire:{id}"), cx, |_, cx| {
                    build_retained(definitions.clone(), shortcuts, fingerprint, cx)
                });
            retained.update(cx, |retained, cx| {
                if retained.fingerprint != fingerprint {
                    *retained = build_retained(definitions.clone(), shortcuts, fingerprint, cx);
                }
            });
            let retained = retained.read(cx);
            Ok((retained.native.clone(), retained.error.clone()))
        })?;
        if let Some(error) = error {
            anyhow::bail!(error);
        }

        let mut questionnaire =
            Questionnaire::new(&state).child(QuestionnaireProgress::new(&state));
        for (name, values) in names {
            let mut answers = QuestionnaireChoices::new(&state, name.clone());
            for value in values {
                answers = answers.child(QuestionnaireChoice::new(&state, name.clone(), value));
            }
            questionnaire = questionnaire.child(
                QuestionnaireItem::new(&state, name.clone())
                    .child(QuestionnaireTitle::new(&state, name.clone()))
                    .child(QuestionnaireDescription::new(&state, name.clone()))
                    .child(answers.child(QuestionnaireInput::new(&state, name.clone())))
                    .child(QuestionnaireError::new(&state, name)),
            );
        }
        let mut element = questionnaire.child(
            QuestionnaireActions::new(&state)
                .child(QuestionnairePrevious::new(&state))
                .child(QuestionnaireSkip::new(&state))
                .child(QuestionnaireNext::new(&state))
                .child(QuestionnaireSubmit::new(&state)),
        );
        element.style().refine(&request.take_style());
        Ok(element.into_any_element())
    }
}

fn build_retained(
    definitions: Vec<QuestionnaireItemDefinition>,
    shortcuts: Option<QuestionnaireShortcutMode>,
    fingerprint: Fingerprint,
    cx: &mut gpui::App,
) -> RetainedQuestionnaire {
    let mut error = None;
    let native = cx.new(|cx| match QuestionnaireState::new(definitions, cx) {
        Ok(state) => match shortcuts {
            Some(mode) => state.with_shortcuts(mode),
            None => state,
        },
        Err(schema_error) => {
            error = Some(format!("Questionnaire schema is invalid: {schema_error}"));
            QuestionnaireState::new(Vec::new(), cx).expect("an empty questionnaire is valid")
        }
    });
    RetainedQuestionnaire {
        native,
        fingerprint,
        error,
    }
}

fn fingerprint(
    definitions: &[QuestionnaireItemDefinition],
    shortcuts: Option<QuestionnaireShortcutMode>,
) -> Fingerprint {
    use std::hash::{Hash as _, Hasher as _};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    shortcuts
        .map(|mode| match mode {
            QuestionnaireShortcutMode::Letters => 1u8,
            QuestionnaireShortcutMode::Numbers => 2,
        })
        .hash(&mut hasher);
    for item in definitions {
        item.name().as_ref().hash(&mut hasher);
        item.accessibility_label().as_ref().hash(&mut hasher);
        item.description().map(AsRef::as_ref).hash(&mut hasher);
        (
            item.is_required(),
            item.is_multiple(),
            item.is_disabled(),
            item.choices().len(),
        )
            .hash(&mut hasher);
        if let Some(input) = item.input() {
            input.state().entity_id().as_u64().hash(&mut hasher);
            input.accessibility_label().as_ref().hash(&mut hasher);
            input.is_disabled().hash(&mut hasher);
        }
        for choice in item.choices() {
            choice.value().as_ref().hash(&mut hasher);
            choice.accessibility_label().as_ref().hash(&mut hasher);
            choice.description().map(AsRef::as_ref).hash(&mut hasher);
            (choice.is_disabled(), choice.is_default_selected()).hash(&mut hasher);
        }
    }
    hasher.finish()
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("QuestionnaireChoice", Arc::new(ChoiceMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "QuestionnaireChoice",
                vec![
                    ArgumentDescriptor::new("value", ArgumentSchema::String),
                    ArgumentDescriptor::new("label", ArgumentSchema::String),
                ],
                |arguments| match arguments {
                    [
                        ComponentArgument::String(value),
                        ComponentArgument::String(label),
                    ] if !value.trim().is_empty() && !label.trim().is_empty() => {
                        Ok(ComponentPayload::new(ChoicePayload {
                            value: value.clone(),
                            label: label.clone(),
                        }))
                    }
                    _ => Err("QuestionnaireChoice expects a non-empty value and label".into()),
                },
            )])
            .with_methods(vec![
                string_method(
                    "QuestionnaireChoice",
                    "description",
                    "Adds secondary text under the choice label.",
                    ChoiceOp::Description,
                ),
                bool_method(
                    "QuestionnaireChoice",
                    "disabled",
                    "Keeps the choice visible but unselectable.",
                    ChoiceOp::Disabled,
                ),
                bool_method(
                    "QuestionnaireChoice",
                    "default_selected",
                    "Selects the choice in the questionnaire's initial snapshot.",
                    ChoiceOp::DefaultSelected,
                ),
            ])
            .with_documentation(
                "Typed answer data for one questionnaire choice; style is rejected.",
            ),
    )?;
    registry.register(
        ComponentDescriptor::new("QuestionnaireItem", Arc::new(ItemMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "QuestionnaireItem",
                vec![
                    ArgumentDescriptor::new("name", ArgumentSchema::String),
                    ArgumentDescriptor::new("label", ArgumentSchema::String),
                ],
                |arguments| match arguments {
                    [
                        ComponentArgument::String(name),
                        ComponentArgument::String(label),
                    ] if !name.trim().is_empty() && !label.trim().is_empty() => {
                        Ok(ComponentPayload::new(ItemPayload {
                            name: name.clone(),
                            label: label.clone(),
                        }))
                    }
                    _ => Err("QuestionnaireItem expects a non-empty name and label".into()),
                },
            )])
            .with_methods(vec![
                string_method(
                    "QuestionnaireItem",
                    "description",
                    "Adds supporting text under the question title.",
                    ItemOp::Description,
                ),
                bool_method(
                    "QuestionnaireItem",
                    "required",
                    "Requires an answer and hides Skip for this question.",
                    ItemOp::Required,
                ),
                bool_method(
                    "QuestionnaireItem",
                    "multiple",
                    "Accepts more than one choice for this question.",
                    ItemOp::Multiple,
                ),
                bool_method(
                    "QuestionnaireItem",
                    "disabled",
                    "Removes the question from progress, navigation and submission.",
                    ItemOp::Disabled,
                ),
            ])
            .with_documentation(
                "Typed data for one question, holding QuestionnaireChoice and QuestionnaireInput children; style is rejected.",
            ),
    )?;
    registry.register(
        ComponentDescriptor::new("QuestionnaireInput", Arc::new(InputMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "QuestionnaireInput",
                vec![
                    ArgumentDescriptor::new("state", ArgumentSchema::Entity("InputState")),
                    ArgumentDescriptor::new("label", ArgumentSchema::String),
                ],
                |arguments| match arguments {
                    [
                        state @ ComponentArgument::Entity { .. },
                        ComponentArgument::String(label),
                    ] if !label.trim().is_empty() => Ok(ComponentPayload::new(InputPayload {
                        state: state.clone(),
                        label: label.clone(),
                    })),
                    _ => Err(
                        "QuestionnaireInput expects an InputState entity and a non-empty label"
                            .into(),
                    ),
                },
            )])
            .with_methods(vec![bool_method(
                "QuestionnaireInput",
                "disabled",
                "Keeps the freeform answer visible but not editable.",
                |value| value,
            )])
            .with_documentation(
                "Typed freeform-answer data for a question, backed by a retained InputState; style is rejected.",
            ),
    )?;
    registry.register(
        ComponentDescriptor::new("Questionnaire", Arc::new(RootMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "Questionnaire",
                vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(id)] if !id.trim().is_empty() => {
                        Ok(ComponentPayload::new(RootPayload(id.clone())))
                    }
                    _ => Err("Questionnaire expects a non-empty id".into()),
                },
            )])
            .with_methods(vec![
                MethodDescriptor::new(
                    "shortcuts",
                    vec![ArgumentDescriptor::new(
                        "mode",
                        ArgumentSchema::Enum(&["letters", "numbers"]),
                    )],
                    |arguments| match arguments {
                        [ComponentArgument::Enum(mode)] if mode == "letters" => Ok(
                            ComponentPayload::new(RootOp::Shortcuts(QuestionnaireShortcutMode::Letters)),
                        ),
                        [ComponentArgument::Enum(mode)] if mode == "numbers" => Ok(
                            ComponentPayload::new(RootOp::Shortcuts(QuestionnaireShortcutMode::Numbers)),
                        ),
                        _ => Err("Questionnaire.shortcuts expects letters or numbers".into()),
                    },
                )
                .with_documentation(
                    "Hands each enabled choice of the active question a letter or number shortcut.",
                ),
            ])
            .with_documentation(
                "Native retained questionnaire keyed by a stable id: it owns answers, validation, navigation, focus and shortcuts, and renders progress, the active question and the navigation actions. Changing the questions rebuilds the flow.",
            ),
    )?;
    Ok(())
}
