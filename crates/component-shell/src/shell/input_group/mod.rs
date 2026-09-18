use std::sync::Arc;

use gpui_component::{
    Disableable as _, FocusableExt as _, Sizable as _, Size,
    button::{ButtonVariant, ButtonVariants as _},
    input::{
        InputGroup, InputGroupAddon, InputGroupAddonAlignment, InputGroupButton, InputGroupInput,
        InputGroupText, InputGroupTextarea, InputState, TextareaState,
    },
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, Entity, IntoElement as _, ParentElement as _, Refineable as _, Styled as _},
};

use super::{
    support::{bool_method, disabled_method, on_click_method, string_method},
    typed_compound::{TypedChildElement, finish_part, take_element},
};
use binding::{NativeState, TextareaLayout};

mod binding;
mod content_type;

#[derive(Clone)]
enum Part {
    Root(String),
    Addon(String),
    Button(String),
    Input(ComponentArgument),
    Textarea(ComponentArgument),
    Text,
}

#[derive(Clone)]
enum Op {
    Addon(ComponentArgument),
    Align(InputGroupAddonAlignment),
    Size(Size),
    Variant(ButtonVariant),
    Outline,
    Readonly(bool),
    Invalid(bool),
    FocusRing(bool),
    Masked(bool),
    Loading(bool),
    Label(String),
    Icon(String),
    AriaLabel(String),
    AccessibilityId(String),
    Tooltip(String),
    Value(String),
    Placeholder(String),
    ContentType(gpui_component::input::InputContentType),
    Layout(TextareaLayout),
    OnChange(ComponentArgument),
}

struct Materializer;

enum Control {
    Input(InputGroupInput),
    Textarea(InputGroupTextarea),
}

#[derive(gpui::IntoElement)]
struct BoundControl {
    control: Control,
    binding: binding::Binding,
}

impl gpui::Styled for BoundControl {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        match &mut self.control {
            Control::Input(input) => input.style(),
            Control::Textarea(textarea) => textarea.style(),
        }
    }
}

impl gpui::RenderOnce for BoundControl {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        self.binding.apply(window, cx);
        match self.control {
            Control::Input(input) => input.into_any_element(),
            Control::Textarea(textarea) => textarea.into_any_element(),
        }
    }
}

#[derive(gpui::IntoElement)]
struct BoundGroup {
    group: InputGroup,
    binding: Option<binding::Binding>,
}

impl gpui::RenderOnce for BoundGroup {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        // CLI checks materialize outside a frame. Retain subscriptions only
        // while the control participates in the actual rendering lifecycle.
        if let Some(binding) = self.binding {
            binding.apply(window, cx);
        }
        self.group
    }
}

fn leaf(request: &MaterializeRequest<'_>, name: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        request.children_len() == 0,
        "{name} does not accept ordinary children; use its named parts"
    );
    Ok(())
}

fn typed<T: gpui::IntoElement + gpui::Styled + 'static>(
    request: &mut MaterializeRequest<'_>,
    mut component: T,
) -> gpui::AnyElement {
    component.style().refine(&request.take_style());
    TypedChildElement::new(component).into_any_element()
}

impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let part = request
            .payload()
            .downcast_ref::<Part>()
            .ok_or_else(|| anyhow::anyhow!("InputGroup received an incompatible payload"))?
            .clone();
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<Op>().cloned())
            .collect::<Vec<_>>();
        match part {
            Part::Root(id) => {
                leaf(&request, "InputGroup")?;
                let mut group = InputGroup::new(id).disabled(request.disabled());
                // `input` is a common shell slot, like Empty's header/content.
                // Let that lane replace old values before materialization.
                let mut control = None;
                let mut binding = None;
                while let Some(element) = request.take_slot("input")? {
                    control = Some(element);
                }
                if let Some(mut control) = control {
                    if control
                        .downcast_mut::<TypedChildElement<BoundControl>>()
                        .is_none()
                    {
                        anyhow::bail!(
                            "InputGroup.input expects InputGroupInput or InputGroupTextarea"
                        );
                    }
                    let control: BoundControl = take_element(&mut control, "InputGroup control")?;
                    binding = Some(control.binding);
                    group = match control.control {
                        Control::Input(input) => group.input(input),
                        Control::Textarea(textarea) => group.input(textarea),
                    };
                }
                for operation in &operations {
                    group = match operation {
                        Op::Addon(argument) => group.addon(take_element(
                            &mut request.resolve_element(argument)?,
                            "InputGroupAddon",
                        )?),
                        Op::Readonly(value) => group.readonly(*value),
                        Op::Invalid(value) => group.invalid(*value),
                        Op::FocusRing(value) => group.focus_ring(*value),
                        Op::AriaLabel(value) => group.aria_label(value.clone()),
                        Op::Size(value) => group.with_size(*value),
                        _ => group,
                    };
                }
                group.style().refine(&request.take_style());
                Ok(BoundGroup { group, binding }.into_any_element())
            }
            Part::Addon(id) => {
                let mut addon = InputGroupAddon::new(id);
                addon.extend(request.take_children()?);
                for operation in &operations {
                    addon = match operation {
                        Op::Align(value) => addon.align(*value),
                        _ => addon,
                    };
                }
                Ok(typed(&mut request, addon))
            }
            Part::Button(id) => {
                let mut button = InputGroupButton::new(id).disabled(request.disabled());
                for operation in &operations {
                    button = match operation {
                        Op::Size(value) => button.with_size(*value),
                        Op::Variant(value) => button.with_variant(*value),
                        Op::Label(value) => button.label(value.clone()),
                        Op::Icon(value) => {
                            button.icon(gpui_component::Icon::default().path(value.clone()))
                        }
                        Op::AriaLabel(value) => button.accessibility_label(value.clone()),
                        Op::Tooltip(value) => button.tooltip(value.clone()),
                        Op::Loading(value) => button.loading(*value),
                        Op::Outline => button.outline(),
                        _ => button,
                    };
                }
                if let Some(callback) = request.on_click() {
                    button = button
                        .on_click(move |event, window, cx| callback.invoke(event, window, cx));
                }
                button.style().refine(&request.take_style());
                button.extend(request.take_children()?);
                Ok(button.into_any_element())
            }
            Part::Input(argument) => {
                leaf(&request, "InputGroupInput")?;
                let state = request.with_state::<Entity<InputState>, _>(&argument, Clone::clone)?;
                let binding =
                    binding::prepare(&mut request, NativeState::Input(state.clone()), &operations)?;
                let tokens = super::input_tokens::prepare(
                    &request,
                    super::input_tokens::State::Input(state.clone()),
                )?;
                let mut input = tokens
                    .input(InputGroupInput::new(&state))
                    .disabled(request.disabled());
                for operation in &operations {
                    input = match operation {
                        Op::AriaLabel(value) => input.aria_label(value.clone()),
                        Op::AccessibilityId(value) => input.accessibility_id(value.clone()),
                        Op::Readonly(value) => input.readonly(*value),
                        Op::ContentType(value) => input.content_type(*value),
                        _ => input,
                    };
                }
                Ok(typed(
                    &mut request,
                    BoundControl {
                        control: Control::Input(input),
                        binding,
                    },
                ))
            }
            Part::Textarea(argument) => {
                leaf(&request, "InputGroupTextarea")?;
                let state =
                    request.with_state::<Entity<TextareaState>, _>(&argument, Clone::clone)?;
                let binding = binding::prepare(
                    &mut request,
                    NativeState::Textarea(state.clone()),
                    &operations,
                )?;
                let tokens = super::input_tokens::prepare(
                    &request,
                    super::input_tokens::State::Textarea(state.clone()),
                )?;
                let mut textarea = tokens
                    .textarea(InputGroupTextarea::new(&state))
                    .disabled(request.disabled());
                for operation in &operations {
                    textarea = match operation {
                        Op::AriaLabel(value) => textarea.aria_label(value.clone()),
                        Op::AccessibilityId(value) => textarea.accessibility_id(value.clone()),
                        Op::Readonly(value) => textarea.readonly(*value),
                        _ => textarea,
                    };
                }
                Ok(typed(
                    &mut request,
                    BoundControl {
                        control: Control::Textarea(textarea),
                        binding,
                    },
                ))
            }
            Part::Text => finish_part(&mut request, InputGroupText::new()),
        }
    }
}

fn id_constructor(name: &'static str, make: fn(String) -> Part) -> ConstructorDescriptor {
    ConstructorDescriptor::new(
        name,
        vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
        move |arguments| match arguments {
            [ComponentArgument::String(id)] if !id.trim().is_empty() => {
                Ok(ComponentPayload::new(make(id.clone())))
            }
            _ => Err(format!("{name} expects one nonempty string id")),
        },
    )
}

fn state_constructor(
    name: &'static str,
    kind: &'static str,
    make: fn(ComponentArgument) -> Part,
) -> ConstructorDescriptor {
    ConstructorDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(
            "state",
            ArgumentSchema::Entity(kind),
        )],
        move |arguments| match arguments {
            [argument @ ComponentArgument::Entity { .. }] => {
                Ok(ComponentPayload::new(make(argument.clone())))
            }
            _ => Err(format!("{name} expects one {kind} entity")),
        },
    )
}

fn part_method(
    name: &'static str,
    documentation: &'static str,
    make: fn(ComponentArgument) -> Op,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Element)],
        move |arguments| match arguments {
            [argument @ ComponentArgument::Element(_)] => {
                Ok(ComponentPayload::new(make(argument.clone())))
            }
            _ => Err(format!("{name} expects one registered InputGroup part")),
        },
    )
    .with_documentation(documentation)
}

fn enum_method(
    name: &'static str,
    values: &'static [&'static str],
    documentation: &'static str,
    parse: fn(&str) -> Option<Op>,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Enum(values))],
        move |arguments| match arguments {
            [ComponentArgument::Enum(value)] => parse(value)
                .map(ComponentPayload::new)
                .ok_or_else(|| format!("unsupported {name} `{value}`")),
            _ => Err(format!("{name} expects one of {}", values.join(", "))),
        },
    )
    .with_documentation(documentation)
}

fn callback_method(
    name: &'static str,
    signature: &'static str,
    documentation: &'static str,
    make: fn(ComponentArgument) -> Op,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(
            "callback",
            ArgumentSchema::Callback(signature),
        )],
        move |arguments| match arguments {
            [argument @ ComponentArgument::Callback(_)] => {
                Ok(ComponentPayload::new(make(argument.clone())))
            }
            _ => Err(format!("{name} expects one callback")),
        },
    )
    .with_documentation(documentation)
}

fn size_method(documentation: &'static str) -> MethodDescriptor {
    enum_method(
        "size",
        &["xsmall", "small", "medium", "large"],
        documentation,
        |value| {
            Some(Op::Size(match value {
                "xsmall" => Size::XSmall,
                "small" => Size::Small,
                "medium" => Size::Medium,
                "large" => Size::Large,
                _ => return None,
            }))
        },
    )
}

fn control_methods(name: &'static str) -> Vec<MethodDescriptor> {
    vec![
        disabled_method(name),
        bool_method(
            name,
            "readonly",
            "Prevents edits while preserving selection and copying.",
            Op::Readonly,
        ),
        string_method(
            name,
            "aria_label",
            "Names the text control for accessibility.",
            Op::AriaLabel,
        ),
        string_method(
            name,
            "accessibility_id",
            "Sets the developer-assigned accessibility identifier.",
            Op::AccessibilityId,
        ),
        string_method(
            name,
            "value",
            "Controls the retained text. Equal values preserve the caret and undo history; programmatic changes do not emit on_change.",
            Op::Value,
        ),
        string_method(
            name,
            "placeholder",
            "Sets the empty-value prompt on the retained state.",
            Op::Placeholder,
        ),
        callback_method(
            "on_change",
            "(value: string, cx: Context) => void",
            "Reports edits from the retained input without duplicating subscriptions across renders.",
            Op::OnChange,
        ),
    ]
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    let mut input_methods = control_methods("InputGroupInput");
    input_methods.push(bool_method(
        "InputGroupInput",
        "masked",
        "Controls password masking on InputState.",
        Op::Masked,
    ));
    input_methods.push(content_type::method());
    let mut textarea_methods = control_methods("InputGroupTextarea");
    textarea_methods.extend(binding::layout_methods());
    for (name, constructor, documentation, methods) in [
        (
            "InputGroup",
            id_constructor("InputGroup", Part::Root),
            "A shared themed frame. The common input(...) slot accepts InputGroupInput or InputGroupTextarea; addons accumulate in insertion order.",
            vec![
                part_method(
                    "addon",
                    "Appends an InputGroupAddon at its logical side.",
                    Op::Addon,
                ),
                disabled_method("InputGroup"),
                bool_method(
                    "InputGroup",
                    "readonly",
                    "Prevents text edits while leaving addon actions available.",
                    Op::Readonly,
                ),
                bool_method(
                    "InputGroup",
                    "invalid",
                    "Displays the caller's validation result without rejecting input.",
                    Op::Invalid,
                ),
                bool_method(
                    "InputGroup",
                    "focus_ring",
                    "Controls the outward focus and error ring, also respecting Theme.focus_ring.",
                    Op::FocusRing,
                ),
                string_method(
                    "InputGroup",
                    "aria_label",
                    "Names the group independently of its text control.",
                    Op::AriaLabel,
                ),
                size_method("Sets the group's semantic control size."),
            ],
        ),
        (
            "InputGroupAddon",
            id_constructor("InputGroupAddon", Part::Addon),
            "An aligned addon. Children retain insertion order; direct InputGroupButton children inherit the group disabled state.",
            vec![enum_method(
                "align",
                &["inline-start", "inline-end", "block-start", "block-end"],
                "Places the addon relative to the control; default is inline-start.",
                |value| {
                    Some(Op::Align(match value {
                        "inline-start" => InputGroupAddonAlignment::InlineStart,
                        "inline-end" => InputGroupAddonAlignment::InlineEnd,
                        "block-start" => InputGroupAddonAlignment::BlockStart,
                        "block-end" => InputGroupAddonAlignment::BlockEnd,
                        _ => return None,
                    }))
                },
            )],
        ),
        (
            "InputGroupButton",
            id_constructor("InputGroupButton", Part::Button),
            "A native Button with compact input-group presentation and group disabled inheritance. Defaults to the ghost variant and xsmall size; a button with only an icon is square.",
            vec![
                disabled_method("InputGroupButton"),
                on_click_method("InputGroupButton"),
                string_method(
                    "InputGroupButton",
                    "label",
                    "Sets the visible label.",
                    Op::Label,
                ),
                string_method(
                    "InputGroupButton",
                    "icon",
                    "Sets the icon path in the application's asset bundle.",
                    Op::Icon,
                ),
                string_method(
                    "InputGroupButton",
                    "aria_label",
                    "Names an icon-only action.",
                    Op::AriaLabel,
                ),
                string_method(
                    "InputGroupButton",
                    "tooltip",
                    "Sets the native tooltip.",
                    Op::Tooltip,
                ),
                bool_method(
                    "InputGroupButton",
                    "loading",
                    "Displays progress and prevents duplicate activation.",
                    Op::Loading,
                ),
                MethodDescriptor::new("outline", vec![], |_| {
                    Ok(ComponentPayload::new(Op::Outline))
                })
                .with_documentation("Uses the native outlined button treatment."),
                size_method(
                    "Sets the button size; xsmall and small are the compact input-group sizes.",
                ),
                enum_method(
                    "variant",
                    &[
                        "default",
                        "primary",
                        "secondary",
                        "danger",
                        "warning",
                        "success",
                        "info",
                        "ghost",
                        "link",
                        "text",
                    ],
                    "Selects a native semantic button variant.",
                    |value| {
                        Some(Op::Variant(match value {
                            "default" => ButtonVariant::Default,
                            "primary" => ButtonVariant::Primary,
                            "secondary" => ButtonVariant::Secondary,
                            "danger" => ButtonVariant::Danger,
                            "warning" => ButtonVariant::Warning,
                            "success" => ButtonVariant::Success,
                            "info" => ButtonVariant::Info,
                            "ghost" => ButtonVariant::Ghost,
                            "link" => ButtonVariant::Link,
                            "text" => ButtonVariant::Text,
                            _ => return None,
                        }))
                    },
                ),
            ],
        ),
        (
            "InputGroupInput",
            state_constructor("InputGroupInput", "InputState", Part::Input),
            "An unframed single-line input using the existing retained InputState and native editing engine.",
            [input_methods, super::input_tokens::methods(false)].concat(),
        ),
        (
            "InputGroupTextarea",
            state_constructor("InputGroupTextarea", "TextareaState", Part::Textarea),
            "An unframed multiline input using the existing retained TextareaState and native editing engine.",
            [textarea_methods, super::input_tokens::methods(false)].concat(),
        ),
        (
            "InputGroupText",
            ConstructorDescriptor::new("InputGroupText", vec![], |_| {
                Ok(ComponentPayload::new(Part::Text))
            }),
            "Muted text or rich helper content inside an input group.",
            vec![],
        ),
    ] {
        registry.register(
            ComponentDescriptor::new(name, Arc::new(Materializer))
                .with_constructors(vec![constructor])
                .with_methods(methods)
                .with_documentation(documentation),
        )?;
    }
    Ok(())
}
