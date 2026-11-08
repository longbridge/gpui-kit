use std::sync::Arc;

use gpui_component::{
    Disableable as _, Selectable as _, Sizable as _,
    button::{Button, ButtonVariants as _, Toggle, ToggleVariants as _},
    checkbox::Checkbox,
    switch::Switch,
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallbackArgument,
    ComponentDescriptor, ComponentMaterializer, ComponentPayload, ComponentRegistry,
    ConstructorDescriptor, MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, ParentElement as _},
};

use super::support::{self, CommonOp};

#[derive(Clone)]
struct IdPayload(String);

#[derive(Clone)]
enum ButtonOp {
    Size(gpui_component::Size),
    Label(String),
    AccessibilityLabel(String),
    Tooltip(String),
    Loading(bool),
    Outline,
    Primary,
    Secondary,
    Danger,
    Success,
    Warning,
    Ghost,
    Link,
    Compact,
}

fn id_constructor(export: &'static str) -> ConstructorDescriptor {
    ConstructorDescriptor::new(
        export,
        vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
        move |arguments| match arguments {
            [ComponentArgument::String(id)] => {
                validate_id(export, id).map(|id| ComponentPayload::new(IdPayload(id)))
            }
            _ => Err(format!("{export}(id) expects one string")),
        },
    )
}

fn validate_id(export: &str, id: &str) -> Result<String, String> {
    if id.is_empty() {
        Err(format!("{export} id must not be empty"))
    } else {
        Ok(id.to_owned())
    }
}

fn operations(request: &MaterializeRequest<'_>) -> Vec<CommonOp> {
    request
        .methods()
        .filter_map(|method| method.payload().downcast_ref::<CommonOp>().cloned())
        .collect()
}

struct ButtonMaterializer;

impl ComponentMaterializer for ButtonMaterializer {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = request
            .payload()
            .downcast_ref::<IdPayload>()
            .ok_or_else(|| anyhow::anyhow!("Button received an incompatible payload"))?;
        let button_ops = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<ButtonOp>().cloned())
            .collect::<Vec<_>>();
        let mut component = Button::new(id.0.clone())
            .disabled(request.disabled())
            .selected(request.selected());
        for operation in button_ops {
            component = match operation {
                ButtonOp::Size(size) => component.with_size(size),
                ButtonOp::Label(label) => component.label(label),
                ButtonOp::AccessibilityLabel(label) => component.accessibility_label(label),
                ButtonOp::Tooltip(tooltip) => component.tooltip(tooltip),
                ButtonOp::Loading(loading) => component.loading(loading),
                ButtonOp::Outline => component.outline(),
                ButtonOp::Primary => component.primary(),
                ButtonOp::Secondary => component.secondary(),
                ButtonOp::Danger => component.danger(),
                ButtonOp::Success => component.success(),
                ButtonOp::Warning => component.warning(),
                ButtonOp::Ghost => component.ghost(),
                ButtonOp::Link => component.link(),
                ButtonOp::Compact => component.compact(),
            };
        }
        if let Some(callback) = request.on_click() {
            component =
                component.on_click(move |event, window, cx| callback.invoke(event, window, cx));
        }
        request.finish(component)
    }
}

macro_rules! variant_method {
    ($name:literal, $variant:expr, $docs:literal) => {
        MethodDescriptor::new($name, Vec::new(), |_| Ok(ComponentPayload::new($variant)))
            .with_documentation($docs)
    };
}

fn button_string(
    name: &'static str,
    docs: &'static str,
    make: fn(String) -> ButtonOp,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::String)],
        move |args| match args {
            [ComponentArgument::String(value)] => Ok(ComponentPayload::new(make(value.clone()))),
            _ => Err(format!("Button.{name} expects one string")),
        },
    )
    .with_documentation(docs)
}

fn button_loading() -> MethodDescriptor {
    MethodDescriptor::new(
        "loading",
        vec![ArgumentDescriptor::new("loading", ArgumentSchema::Boolean)],
        |args| match args {
            [ComponentArgument::Boolean(value)] => {
                Ok(ComponentPayload::new(ButtonOp::Loading(*value)))
            }
            _ => Err("Button.loading expects one boolean".into()),
        },
    )
    .with_documentation("Sets the loading presentation.")
}

fn button_size() -> MethodDescriptor {
    MethodDescriptor::new(
        "size",
        vec![ArgumentDescriptor::new(
            "size",
            ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
        )],
        |args| match args {
            [ComponentArgument::Enum(value)] => match value.as_str() {
                "xsmall" => Ok(ComponentPayload::new(ButtonOp::Size(
                    gpui_component::Size::XSmall,
                ))),
                "small" => Ok(ComponentPayload::new(ButtonOp::Size(
                    gpui_component::Size::Small,
                ))),
                "medium" => Ok(ComponentPayload::new(ButtonOp::Size(
                    gpui_component::Size::Medium,
                ))),
                "large" => Ok(ComponentPayload::new(ButtonOp::Size(
                    gpui_component::Size::Large,
                ))),
                _ => Err(format!("unsupported Button size `{value}`")),
            },
            _ => Err("Button.size expects a semantic size literal".into()),
        },
    )
    .with_documentation("Sets the semantic control size.")
}

struct CheckboxMaterializer;

impl ComponentMaterializer for CheckboxMaterializer {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = request
            .payload()
            .downcast_ref::<IdPayload>()
            .ok_or_else(|| anyhow::anyhow!("Checkbox received an incompatible payload"))?;
        let mut change = None;
        let mut component = Checkbox::new(id.0.clone())
            .disabled(request.disabled())
            .checked(request.selected());
        for operation in operations(&request) {
            component = match operation {
                CommonOp::Size(size) => component.with_size(size),
                CommonOp::Label(label) => component.label(label),
                CommonOp::Tooltip(tooltip) => component.tooltip(tooltip),
                CommonOp::Checked(checked) => component.checked(checked),
                CommonOp::Change(argument) => {
                    change = Some(request.resolve_callback(&argument)?);
                    component
                }
                _ => component,
            };
        }
        if let Some(callback) = change {
            component = component.on_click(move |checked, window, cx| {
                callback.invoke_and_report_with(
                    "Checkbox.on_change",
                    &[ComponentCallbackArgument::Boolean(*checked)],
                    window,
                    cx,
                );
            });
        }
        request.finish(component)
    }
}

struct SwitchMaterializer;

impl ComponentMaterializer for SwitchMaterializer {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = request
            .payload()
            .downcast_ref::<IdPayload>()
            .ok_or_else(|| anyhow::anyhow!("Switch received an incompatible payload"))?;
        let mut change = None;
        let mut component = Switch::new(id.0.clone())
            .disabled(request.disabled())
            .checked(request.selected());
        for operation in operations(&request) {
            component = match operation {
                CommonOp::Size(size) => component.with_size(size),
                CommonOp::Label(label) => component.label(label),
                CommonOp::Tooltip(tooltip) => component.tooltip(tooltip),
                CommonOp::Checked(checked) => component.checked(checked),
                CommonOp::Change(argument) => {
                    change = Some(request.resolve_callback(&argument)?);
                    component
                }
                _ => component,
            };
        }
        if let Some(callback) = change {
            component = component.on_click(move |checked, window, cx| {
                callback.invoke_and_report_with(
                    "Switch.on_change",
                    &[ComponentCallbackArgument::Boolean(*checked)],
                    window,
                    cx,
                );
            });
        }
        request.finish(gpui::div().child(component))
    }
}

struct ToggleMaterializer;

impl ComponentMaterializer for ToggleMaterializer {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = request
            .payload()
            .downcast_ref::<IdPayload>()
            .ok_or_else(|| anyhow::anyhow!("Toggle received an incompatible payload"))?;
        let mut change = None;
        let mut component = Toggle::new(id.0.clone())
            .disabled(request.disabled())
            .checked(request.selected());
        for operation in operations(&request) {
            component = match operation {
                CommonOp::Size(size) => component.with_size(size),
                CommonOp::Label(label) => component.label(label),
                CommonOp::Tooltip(tooltip) => component.tooltip(tooltip),
                CommonOp::Checked(checked) => component.checked(checked),
                CommonOp::Outline => component.outline(),
                CommonOp::Change(argument) => {
                    change = Some(request.resolve_callback(&argument)?);
                    component
                }
            };
        }
        if let Some(callback) = change {
            component = component.on_click(move |checked, window, cx| {
                callback.invoke_and_report_with(
                    "Toggle.on_change",
                    &[ComponentCallbackArgument::Boolean(*checked)],
                    window,
                    cx,
                );
            });
        }
        request.finish(component)
    }
}

fn state_methods(component: &'static str) -> Vec<MethodDescriptor> {
    vec![
        support::string_method(
            component,
            "label",
            "Sets the visible control label.",
            CommonOp::Label,
        ),
        support::string_method(
            component,
            "tooltip",
            "Sets concise hover help.",
            CommonOp::Tooltip,
        ),
        support::bool_method(
            component,
            "checked",
            "Sets the controlled checked state.",
            CommonOp::Checked,
        ),
        support::size_method(),
        support::change_method(component),
        support::disabled_method(component),
    ]
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor::new("Button", Arc::new(ButtonMaterializer))
.with_constructors(vec![id_constructor("Button")])
.with_methods(vec![
            support::on_click_method("Button"),
            support::disabled_method("Button"),
            button_string("label", "Sets the visible button label.", ButtonOp::Label),
            button_string(
                "accessibility_label",
                "Sets the accessible name independently of the visible button label.",
                ButtonOp::AccessibilityLabel,
            ),
            button_string("tooltip", "Sets concise hover help.", ButtonOp::Tooltip),
            button_loading(),
            button_size(),
            variant_method!("outline", ButtonOp::Outline, "Uses the outline presentation."),
            variant_method!("primary", ButtonOp::Primary, "Uses the primary action variant."),
            variant_method!("secondary", ButtonOp::Secondary, "Uses the secondary variant."),
            variant_method!("danger", ButtonOp::Danger, "Uses the destructive-action variant."),
            variant_method!("success", ButtonOp::Success, "Uses the success variant."),
            variant_method!("warning", ButtonOp::Warning, "Uses the warning variant."),
            variant_method!("ghost", ButtonOp::Ghost, "Uses the quiet ghost variant."),
            variant_method!("link", ButtonOp::Link, "Uses the link-like visual variant."),
            variant_method!("compact", ButtonOp::Compact, "Uses compact internal spacing."),
        ])
.with_documentation(
            "A stateless command button. Shell disabled, selected, children, style, and on_click operations are honored.",
        ))?;
    for (name, materializer) in [
        (
            "Checkbox",
            Arc::new(CheckboxMaterializer) as Arc<dyn ComponentMaterializer>,
        ),
        (
            "Switch",
            Arc::new(SwitchMaterializer) as Arc<dyn ComponentMaterializer>,
        ),
        (
            "Toggle",
            Arc::new(ToggleMaterializer) as Arc<dyn ComponentMaterializer>,
        ),
    ] {
        let mut methods = state_methods(name);
        if name == "Toggle" {
            methods.push(support::outline_method());
        }
        registry.register(ComponentDescriptor::new(name, materializer)
.with_constructors(vec![id_constructor(name)])
.with_methods(methods)
.with_documentation(
                "A controlled stateless boolean control. Provide checked explicitly; boolean change callbacks are not exposed until the shell callback facade can carry values.",
            ))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_shell::gpui::IntoElement as _;

    #[test]
    fn identity_controls_reject_empty_ids() {
        assert_eq!(
            validate_id("Button", ""),
            Err("Button id must not be empty".into())
        );
        assert_eq!(validate_id("Link", "save"), Ok("save".into()));
    }

    #[test]
    fn button_operations_replay_in_recorded_order_on_a_real_component() {
        let operations = vec![
            ButtonOp::Label("First".into()),
            ButtonOp::Primary,
            ButtonOp::Label("Last".into()),
            ButtonOp::Outline,
        ];
        let component =
            operations
                .into_iter()
                .fold(Button::new("ordered"), |component, op| match op {
                    ButtonOp::Label(value) => component.label(value),
                    ButtonOp::Primary => component.primary(),
                    ButtonOp::Outline => component.outline(),
                    _ => component,
                });
        drop(component.into_any_element());
    }
}
