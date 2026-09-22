//! Honest retained-state adapters for form controls whose state has a
//! concrete, delegate-free construction API.
//!
//! Change callbacks are deliberately not exposed here yet. These controls
//! emit events from their retained `Entity` state, while the current shell
//! materializer has no subscription owner (`Context<T>`) whose lifetime can
//! retain a GPUI subscription. Adding an `on_change` method without that owner
//! would either drop the subscription immediately or leak it globally.
//! Delegate-backed Select and Combobox are likewise deferred until scripts
//! can provide an honest searchable-list delegate rather than fabricated
//! options.

use gpui_component::{
    Disableable as _,
    calendar::{Calendar, CalendarState},
    color_picker::{ColorPicker, ColorPickerState},
    date_picker::{DatePicker, DatePickerState},
    input::{Input, InputState, NumberInput, OtpInput, OtpState},
    slider::{Slider, SliderState},
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, StateDescriptor, anyhow,
    gpui::{
        self, AppContext as _, Entity, IntoElement as _, ParentElement as _, Refineable as _,
        Styled as _,
    },
};
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_forms_publish_matching_state_and_component_contracts() {
        let mut registry = ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        let states = frozen
            .states()
            .map(|state| state.export())
            .collect::<Vec<_>>();
        let components = frozen
            .descriptors()
            .map(|descriptor| descriptor.name())
            .collect::<Vec<_>>();

        assert_eq!(
            states,
            [
                "InputState",
                "CalendarState",
                "OtpState",
                "SliderState",
                "ColorPickerState",
                "DatePickerState",
            ]
        );
        assert_eq!(
            components,
            [
                "Input",
                "NumberInput",
                "OtpInput",
                "Slider",
                "ColorPicker",
                "Calendar",
                "DatePicker"
            ]
        );
        assert!(frozen.states().all(|state| state.documentation().is_some()));
        assert!(frozen.descriptors().all(|descriptor| {
            descriptor.documentation().is_some()
                && descriptor
                    .methods()
                    .iter()
                    .all(|method| method.documentation().is_some())
        }));
    }

    #[test]
    fn state_arguments_are_closed_and_component_state_kinds_match() {
        let mut registry = ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();

        let otp_state = frozen
            .states()
            .find(|state| state.export() == "OtpState")
            .unwrap();
        assert_eq!(otp_state.arguments()[0].schema(), &ArgumentSchema::Number);
        for descriptor in frozen.descriptors() {
            let constructor = &descriptor.constructors()[0];
            assert_eq!(constructor.arguments().len(), 1);
            assert!(matches!(
                constructor.arguments()[0].schema(),
                &ArgumentSchema::Entity(_)
            ));
        }
    }

    #[test]
    fn positive_usize_rejects_values_that_round_past_usize_max() {
        let rounded_overflow = usize::MAX as f64;
        let error =
            positive_usize(&[ComponentArgument::Number(rounded_overflow)], "OtpState").unwrap_err();
        assert!(error.contains("positive integer"), "{error}");
    }

    #[test]
    fn positive_usize_accepts_only_exact_positive_integers() {
        assert_eq!(
            positive_usize(&[ComponentArgument::Number(6.0)], "OtpState").unwrap(),
            6
        );
        for value in [0.0, -1.0, 1.5, f64::NAN, f64::INFINITY] {
            assert!(
                positive_usize(&[ComponentArgument::Number(value)], "OtpState").is_err(),
                "accepted {value}"
            );
        }
    }

    #[test]
    fn otp_leaf_contract_rejects_ordinary_children() {
        let error = ensure_leaf(1, "OtpInput").unwrap_err();
        assert_eq!(error.to_string(), "OtpInput does not accept children");
        ensure_leaf(0, "OtpInput").unwrap();
    }
}

#[derive(Clone)]
enum FormOp {
    Disabled(bool),
    Placeholder(String),
    AriaLabel(String),
    Groups(usize),
    Vertical,
    Reverse,
    Label(String),
    AccessibilityLabel(String),
    Months(usize),
}

fn bool_op(
    arguments: &[ComponentArgument],
    callable: &str,
    make: impl FnOnce(bool) -> FormOp,
) -> Result<ComponentPayload, String> {
    match arguments {
        [ComponentArgument::Boolean(value)] => Ok(ComponentPayload::new(make(*value))),
        _ => Err(format!("{callable} expects one boolean")),
    }
}

fn string_op(
    arguments: &[ComponentArgument],
    callable: &str,
    make: impl FnOnce(String) -> FormOp,
) -> Result<ComponentPayload, String> {
    match arguments {
        [ComponentArgument::String(value)] => Ok(ComponentPayload::new(make(value.clone()))),
        _ => Err(format!("{callable} expects one string")),
    }
}

fn positive_usize(arguments: &[ComponentArgument], callable: &str) -> Result<usize, String> {
    match arguments {
        [ComponentArgument::Number(value)]
            if value.is_finite()
                && *value >= 1.0
                && value.fract() == 0.0
                && *value < 2_f64.powi(usize::BITS as i32) =>
        {
            let converted = *value as usize;
            if converted as f64 == *value {
                Ok(converted)
            } else {
                Err(format!("{callable} expects a positive integer"))
            }
        }
        _ => Err(format!("{callable} expects a positive integer")),
    }
}

fn state_payload(arguments: &[ComponentArgument]) -> Result<ComponentPayload, String> {
    Ok(ComponentPayload::new(arguments[0].clone()))
}

fn state_constructor(export: &'static str, kind: &'static str) -> ConstructorDescriptor {
    ConstructorDescriptor::new(
        export,
        vec![ArgumentDescriptor::new(
            "state",
            ArgumentSchema::Entity(kind),
        )],
        state_payload,
    )
}

macro_rules! state_entity {
    ($request:expr, $ty:ty) => {{
        let argument = $request
            .payload()
            .downcast_ref::<ComponentArgument>()
            .ok_or_else(|| anyhow::anyhow!("retained form received an incompatible payload"))?;
        $request.with_state::<Entity<$ty>, _>(argument, Clone::clone)?
    }};
}

fn finish_leaf<E>(
    request: &mut MaterializeRequest<'_>,
    mut element: E,
) -> anyhow::Result<gpui::AnyElement>
where
    E: gpui::Styled + gpui::IntoElement + 'static,
{
    ensure_leaf(request.children_len(), std::any::type_name::<E>())?;
    element.style().refine(&request.take_style());
    Ok(element.into_any_element())
}

fn finish_unstyled_leaf<E>(
    request: &mut MaterializeRequest<'_>,
    element: E,
) -> anyhow::Result<gpui::AnyElement>
where
    E: gpui::IntoElement + 'static,
{
    ensure_leaf(request.children_len(), "OtpInput")?;
    let mut wrapper = gpui::div().child(element);
    wrapper.style().refine(&request.take_style());
    Ok(wrapper.into_any_element())
}

fn ensure_leaf(children_len: usize, component: &str) -> anyhow::Result<()> {
    anyhow::ensure!(children_len == 0, "{component} does not accept children");
    Ok(())
}

struct InputMaterializer;
impl ComponentMaterializer for InputMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let state = state_entity!(request, InputState);
        let binding = super::input_tokens::prepare(
            &request,
            super::input_tokens::State::Input(state.clone()),
        )?;
        let mut input = binding.input(Input::new(&state));
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<FormOp>())
        {
            input = match op {
                FormOp::Disabled(value) => input.disabled(*value),
                FormOp::AriaLabel(value) => input.aria_label(value.clone()),
                _ => input,
            };
        }
        Ok(binding.wrap(finish_leaf(&mut request, input)?))
    }
}

struct NumberInputMaterializer;
impl ComponentMaterializer for NumberInputMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let state = state_entity!(request, InputState);
        let mut input = NumberInput::new(&state);
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<FormOp>())
        {
            input = match op {
                FormOp::Disabled(value) => input.disabled(*value),
                FormOp::Placeholder(value) => input.placeholder(value.clone()),
                _ => input,
            };
        }
        finish_leaf(&mut request, input)
    }
}

struct OtpInputMaterializer;
impl ComponentMaterializer for OtpInputMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let state = state_entity!(request, OtpState);
        let mut input = OtpInput::new(&state);
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<FormOp>())
        {
            input = match op {
                FormOp::Disabled(value) => input.disabled(*value),
                FormOp::Groups(value) => input.groups(*value),
                _ => input,
            };
        }
        finish_unstyled_leaf(&mut request, input)
    }
}

struct SliderMaterializer;
impl ComponentMaterializer for SliderMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let state = state_entity!(request, SliderState);
        let mut slider = Slider::new(&state);
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<FormOp>())
        {
            slider = match op {
                FormOp::Disabled(value) => slider.disabled(*value),
                FormOp::Vertical => slider.vertical(),
                FormOp::Reverse => slider.reverse(),
                _ => slider,
            };
        }
        finish_leaf(&mut request, slider)
    }
}

struct ColorPickerMaterializer;
impl ComponentMaterializer for ColorPickerMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let state = state_entity!(request, ColorPickerState);
        let mut picker = ColorPicker::new(&state);
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<FormOp>())
        {
            picker = match op {
                FormOp::Label(value) => picker.label(value.clone()),
                FormOp::AccessibilityLabel(value) => picker.accessibility_label(value.clone()),
                _ => picker,
            };
        }
        finish_leaf(&mut request, picker)
    }
}

struct DatePickerMaterializer;
impl ComponentMaterializer for DatePickerMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let state = state_entity!(request, DatePickerState);
        let mut picker = DatePicker::new(&state);
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<FormOp>())
        {
            picker = match op {
                FormOp::Disabled(value) => picker.disabled(*value),
                FormOp::Placeholder(value) => picker.placeholder(value.clone()),
                _ => picker,
            };
        }
        finish_leaf(&mut request, picker)
    }
}

struct CalendarMaterializer;
impl ComponentMaterializer for CalendarMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let state = state_entity!(request, CalendarState);
        let mut calendar = Calendar::new(&state);
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<FormOp>())
        {
            if let FormOp::Months(value) = op {
                calendar = calendar.number_of_months(*value);
            }
        }
        finish_leaf(&mut request, calendar)
    }
}

fn disabled_method(owner: &'static str) -> MethodDescriptor {
    MethodDescriptor::new(
        "disabled",
        vec![ArgumentDescriptor::new("disabled", ArgumentSchema::Boolean)],
        move |arguments| bool_op(arguments, &format!("{owner}.disabled"), FormOp::Disabled),
    )
    .with_documentation("Controls whether the form control accepts interaction.")
}

fn placeholder_method(owner: &'static str) -> MethodDescriptor {
    MethodDescriptor::new(
        "placeholder",
        vec![ArgumentDescriptor::new(
            "placeholder",
            ArgumentSchema::String,
        )],
        move |arguments| {
            string_op(
                arguments,
                &format!("{owner}.placeholder"),
                FormOp::Placeholder,
            )
        },
    )
    .with_documentation("Sets the empty-value prompt shown by the control.")
}

fn aria_label_method(owner: &'static str) -> MethodDescriptor {
    MethodDescriptor::new(
        "aria_label",
        vec![ArgumentDescriptor::new("label", ArgumentSchema::String)],
        move |arguments| string_op(arguments, &format!("{owner}.aria_label"), FormOp::AriaLabel),
    )
    .with_documentation("Sets the name announced by accessibility clients.")
}

fn component(
    name: &'static str,
    state_kind: &'static str,
    methods: Vec<MethodDescriptor>,
    documentation: &'static str,
    materializer: impl ComponentMaterializer + 'static,
) -> ComponentDescriptor {
    ComponentDescriptor::new(name, Arc::new(materializer))
        .with_constructors(vec![state_constructor(name, state_kind)])
        .with_methods(methods)
        .with_documentation(documentation)
}

/// Registers retained form states and their delegate-free controls.
pub fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register_state(
        StateDescriptor::new(
            "InputState",
            "InputState",
            vec![ArgumentDescriptor::new(
                "placeholder",
                ArgumentSchema::Optional(Box::new(ArgumentSchema::String)),
            ), ArgumentDescriptor::new(
                "initial_value",
                ArgumentSchema::Optional(Box::new(ArgumentSchema::String)),
            )],
            |arguments, window, cx| {
                let optional_text = |argument: &ComponentArgument, name: &str| match argument {
                    ComponentArgument::Optional(Some(value)) => match value.as_ref() {
                        ComponentArgument::String(value) => Ok(value.clone()),
                        _ => Err(format!("InputState {name} expects text")),
                    },
                    ComponentArgument::Optional(None) => Ok(String::new()),
                    _ => Err(format!("InputState {name} must be optional text")),
                };
                let [placeholder, initial_value] = arguments else {
                    return Err("InputState expects optional placeholder and initial value".into());
                };
                let placeholder = optional_text(placeholder, "placeholder")?;
                let initial_value = optional_text(initial_value, "initial_value")?;
                Ok(Box::new(cx.new(|cx| {
                    InputState::new(window, cx)
                        .placeholder(placeholder)
                        .default_value(initial_value)
                })))
            },
        )
        .with_methods(gpui_shell::input_token_state_methods())
        .with_documentation(
            "Retained editable text state shared by Input and NumberInput, with optional placeholder and initial value.",
        ),
    )?;
    registry.register_state(
        StateDescriptor::new("CalendarState", "CalendarState", vec![], |_, window, cx| {
            Ok(Box::new(cx.new(|cx| CalendarState::new(window, cx))))
        })
        .with_documentation("Retained calendar navigation and selection state."),
    )?;
    registry.register_state(
        StateDescriptor::new(
            "OtpState",
            "OtpState",
            vec![ArgumentDescriptor::new("length", ArgumentSchema::Number)],
            |arguments, window, cx| {
                let length = positive_usize(arguments, "OtpState")?;
                Ok(Box::new(cx.new(|cx| OtpState::new(length, window, cx))))
            },
        )
        .with_documentation("Retained fixed-length one-time-password editing state."),
    )?;
    registry.register_state(
        StateDescriptor::new(
            "SliderState",
            "SliderState",
            vec![ArgumentDescriptor::new(
                "initial_value",
                ArgumentSchema::Optional(Box::new(ArgumentSchema::Number)),
            )],
            |arguments, _, cx| {
                let value = match arguments {
                    [ComponentArgument::Optional(Some(value))] => match value.as_ref() {
                        ComponentArgument::Number(value)
                            if value.is_finite() && (0.0..=100.0).contains(value) =>
                        {
                            *value as f32
                        }
                        _ => return Err("SliderState initial_value expects 0 through 100".into()),
                    },
                    [ComponentArgument::Optional(None)] => 0.0,
                    _ => return Err("SliderState expects an optional initial value".into()),
                };
                Ok(Box::new(
                    cx.new(|_| SliderState::new().default_value(value)),
                ))
            },
        )
        .with_documentation("Retained single-value slider state with an optional initial value."),
    )?;
    registry.register_state(
        StateDescriptor::new(
            "ColorPickerState",
            "ColorPickerState",
            vec![],
            |_, window, cx| Ok(Box::new(cx.new(|cx| ColorPickerState::new(window, cx)))),
        )
        .with_documentation("Retained color selection and preview state."),
    )?;
    registry.register_state(
        StateDescriptor::new(
            "DatePickerState",
            "DatePickerState",
            vec![],
            |_, window, cx| Ok(Box::new(cx.new(|cx| DatePickerState::new(window, cx)))),
        )
        .with_documentation("Retained single-date picker and calendar state."),
    )?;

    registry.register(component(
        "Input",
        "InputState",
        [
            vec![aria_label_method("Input"), disabled_method("Input")],
            super::input_tokens::methods(true),
        ]
        .concat(),
        "A retained single-line text field.",
        InputMaterializer,
    ))?;
    registry.register(component(
        "NumberInput",
        "InputState",
        vec![
            placeholder_method("NumberInput"),
            disabled_method("NumberInput"),
        ],
        "A retained numeric text field with increment and decrement controls.",
        NumberInputMaterializer,
    ))?;
    registry.register(component(
        "OtpInput",
        "OtpState",
        vec![
            MethodDescriptor::new(
                "groups",
                vec![ArgumentDescriptor::new("groups", ArgumentSchema::Number)],
                |arguments| {
                    positive_usize(arguments, "OtpInput.groups")
                        .map(|value| ComponentPayload::new(FormOp::Groups(value)))
                },
            )
            .with_documentation("Splits the fixed-length code into the requested number of visual groups."),
            disabled_method("OtpInput"),
        ],
        "A retained fixed-length one-time-password field. Shell styles apply to its dedicated wrapper because OtpInput itself is not Styled; ordinary children are rejected.",
        OtpInputMaterializer,
    ))?;
    registry.register(component(
        "Slider",
        "SliderState",
        vec![
            MethodDescriptor::new("vertical", vec![], |_| {
                Ok(ComponentPayload::new(FormOp::Vertical))
            })
            .with_documentation("Uses a vertical track instead of the default horizontal track."),
            MethodDescriptor::new("reverse", vec![], |_| {
                Ok(ComponentPayload::new(FormOp::Reverse))
            })
            .with_documentation("Reverses the filled side for a single-value slider."),
            disabled_method("Slider"),
        ],
        "A retained numeric slider using SliderState defaults.",
        SliderMaterializer,
    ))?;
    registry.register(component(
        "ColorPicker",
        "ColorPickerState",
        vec![
            MethodDescriptor::new(
                "label",
                vec![ArgumentDescriptor::new("label", ArgumentSchema::String)],
                |arguments| string_op(arguments, "ColorPicker.label", FormOp::Label),
            )
            .with_documentation("Sets the visible label above the picker."),
            MethodDescriptor::new(
                "accessibility_label",
                vec![ArgumentDescriptor::new("label", ArgumentSchema::String)],
                |arguments| {
                    string_op(
                        arguments,
                        "ColorPicker.accessibility_label",
                        FormOp::AccessibilityLabel,
                    )
                },
            )
            .with_documentation("Sets the announced name independently of the visible label."),
        ],
        "A retained color picker with preview and commit behavior.",
        ColorPickerMaterializer,
    ))?;
    registry.register(component(
        "Calendar",
        "CalendarState",
        vec![
            MethodDescriptor::new(
                "number_of_months",
                vec![ArgumentDescriptor::new("count", ArgumentSchema::Number)],
                |arguments| {
                    positive_usize(arguments, "Calendar.number_of_months")
                        .map(|value| ComponentPayload::new(FormOp::Months(value)))
                },
            )
            .with_documentation("Sets the positive number of adjacent months to display."),
        ],
        "A retained calendar for date navigation and selection.",
        CalendarMaterializer,
    ))?;
    registry.register(component(
        "DatePicker",
        "DatePickerState",
        vec![
            placeholder_method("DatePicker"),
            disabled_method("DatePicker"),
        ],
        "A retained single-date picker backed by an internal calendar.",
        DatePickerMaterializer,
    ))?;
    Ok(())
}
