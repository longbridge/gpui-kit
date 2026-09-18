use super::bool_method;
use std::sync::Arc;

use gpui_component::input::{Textarea, TextareaState};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, StateDescriptor, anyhow,
    gpui::{self, AppContext as _, Entity, IntoElement as _, Refineable as _, Styled as _},
};

#[derive(Clone)]
enum Op {
    Appearance(bool),
    Bordered(bool),
    Readonly(bool),
    AriaLabel(String),
}

struct Materializer;
fn require_leaf(children: usize) -> anyhow::Result<()> {
    anyhow::ensure!(children == 0, "Textarea does not accept children");
    Ok(())
}
impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let argument = request
            .payload()
            .downcast_ref::<ComponentArgument>()
            .ok_or_else(|| anyhow::anyhow!("Textarea received an incompatible payload"))?;
        let state = request.with_state::<Entity<TextareaState>, _>(argument, Clone::clone)?;
        let binding = super::super::input_tokens::prepare(
            &request,
            super::super::input_tokens::State::Textarea(state.clone()),
        )?;
        let mut textarea = binding
            .textarea(Textarea::new(&state))
            .disabled(request.disabled());
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<Op>())
        {
            textarea = match op {
                Op::Appearance(value) => textarea.appearance(*value),
                Op::Bordered(value) => textarea.bordered(*value),
                Op::Readonly(value) => textarea.readonly(*value),
                Op::AriaLabel(value) => textarea.aria_label(value.clone()),
            };
        }
        require_leaf(request.children_len())?;
        textarea.style().refine(&request.take_style());
        Ok(binding.wrap(textarea.into_any_element()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn textarea_is_an_exact_leaf() {
        assert!(require_leaf(0).is_ok());
        assert_eq!(
            require_leaf(1).unwrap_err().to_string(),
            "Textarea does not accept children"
        );
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register_state(
        StateDescriptor::new(
            "TextareaState",
            "TextareaState",
            vec![ArgumentDescriptor::new(
                "initial_value",
                ArgumentSchema::Optional(Box::new(ArgumentSchema::String)),
            )],
            |arguments, window, cx| {
                let value = match arguments {
                    [ComponentArgument::Optional(Some(value))] => match value.as_ref() {
                        ComponentArgument::String(value) => value.clone(),
                        _ => return Err("TextareaState initial_value expects text".into()),
                    },
                    [ComponentArgument::Optional(None)] => String::new(),
                    _ => return Err("TextareaState expects an optional initial value".into()),
                };
                Ok(Box::new(cx.new(|cx| {
                    TextareaState::new(window, cx).default_value(value)
                })))
            },
        )
        .with_methods(gpui_shell::textarea_token_state_methods())
        .with_documentation("Retained multi-line editing state with an optional initial value."),
    )?;
    registry.register(ComponentDescriptor::new("Textarea", Arc::new(Materializer))
.with_constructors(vec![ConstructorDescriptor::new("Textarea", vec![ArgumentDescriptor::new("state", ArgumentSchema::Entity("TextareaState"))], |args| match args {
            [argument @ ComponentArgument::Entity { .. }] => Ok(ComponentPayload::new(argument.clone())),
            _ => Err("Textarea expects one TextareaState entity".into()),
        })])
.with_methods([vec![
            MethodDescriptor::new("disabled", vec![ArgumentDescriptor::new("disabled", ArgumentSchema::Boolean)], |_| Ok(ComponentPayload::new(()))).with_documentation("Sets the common disabled state."),
            bool_method("Textarea", "appearance", "Sets the corresponding native textarea presentation or editing policy.", Op::Appearance), bool_method("Textarea", "bordered", "Sets the corresponding native textarea presentation or editing policy.", Op::Bordered), bool_method("Textarea", "readonly", "Sets the corresponding native textarea presentation or editing policy.", Op::Readonly),
            MethodDescriptor::new("aria_label", vec![ArgumentDescriptor::new("label", ArgumentSchema::String)], |args| match args {
                [ComponentArgument::String(value)] if !value.trim().is_empty() => Ok(ComponentPayload::new(Op::AriaLabel(value.clone()))),
                _ => Err("Textarea.aria_label expects non-empty text".into()),
            }).with_documentation("Sets the accessibility label."),
        ], super::super::input_tokens::methods(true)].concat())
.with_documentation("A retained native multi-line text editor. Shell style and common disabled state are honored; children are rejected."))?;
    Ok(())
}
