use std::sync::Arc;

use gpui_component::{
    Sizable as _, Size,
    alert::{Alert, AlertVariant},
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, Refineable as _, Styled as _},
};

use super::common::{ensure_no_children, non_empty_id, size_operation};

#[derive(Clone)]
struct AlertPayload {
    id: String,
    message: String,
    variant: AlertVariant,
}

#[derive(Clone)]
enum AlertOp {
    Title(String),
    Banner,
    Visible(bool),
    Size(Size),
}

struct AlertMaterializer;

impl AlertMaterializer {
    fn component<'a>(
        payload: &ComponentPayload,
        operations: impl IntoIterator<Item = &'a AlertOp>,
    ) -> anyhow::Result<Alert> {
        let payload = payload
            .downcast_ref::<AlertPayload>()
            .ok_or_else(|| anyhow::anyhow!("Alert received an incompatible payload"))?;
        let alert = match payload.variant {
            AlertVariant::Default => Alert::new(payload.id.clone(), payload.message.clone()),
            AlertVariant::Info => Alert::info(payload.id.clone(), payload.message.clone()),
            AlertVariant::Success => Alert::success(payload.id.clone(), payload.message.clone()),
            AlertVariant::Warning => Alert::warning(payload.id.clone(), payload.message.clone()),
            AlertVariant::Error => Alert::error(payload.id.clone(), payload.message.clone()),
        };
        Ok(operations
            .into_iter()
            .fold(alert, |alert, operation| match operation {
                AlertOp::Title(title) => alert.title(title.clone()),
                AlertOp::Banner => alert.banner(),
                AlertOp::Visible(visible) => alert.visible(*visible),
                AlertOp::Size(size) => alert.with_size(*size),
            }))
    }
}

impl ComponentMaterializer for AlertMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        ensure_no_children("Alert", &request)?;
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<AlertOp>());
        let mut alert = Self::component(request.payload(), operations)?;
        alert.style().refine(&request.take_style());
        Ok(alert.into_any_element())
    }
}

fn constructor(export: &'static str, variant: AlertVariant) -> ConstructorDescriptor {
    ConstructorDescriptor::new(
        export,
        vec![
            ArgumentDescriptor::new("id", ArgumentSchema::String),
            ArgumentDescriptor::new("message", ArgumentSchema::String),
        ],
        move |arguments| match arguments {
            [
                ComponentArgument::String(id),
                ComponentArgument::String(message),
            ] => Ok(ComponentPayload::new(AlertPayload {
                id: non_empty_id("Alert", id)?,
                message: message.clone(),
                variant,
            })),
            _ => Err(format!("{export}(id, message) expects two strings")),
        },
    )
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor::new("Alert", Arc::new(AlertMaterializer))
.with_constructors(vec![
            constructor("Alert", AlertVariant::Default), constructor("InfoAlert", AlertVariant::Info),
            constructor("SuccessAlert", AlertVariant::Success), constructor("WarningAlert", AlertVariant::Warning),
            constructor("ErrorAlert", AlertVariant::Error),
        ])
.with_methods(vec![
            MethodDescriptor::new("title", vec![ArgumentDescriptor::new("title", ArgumentSchema::String)], |arguments| match arguments {
                [ComponentArgument::String(title)] => Ok(ComponentPayload::new(AlertOp::Title(title.clone()))), _ => Err("Alert.title(title) expects a string".into()),
            }).with_documentation("Sets the alert title."),
            MethodDescriptor::new("banner", Vec::new(), |_| Ok(ComponentPayload::new(AlertOp::Banner))).with_documentation("Uses the full-width banner presentation."),
            MethodDescriptor::new("visible", vec![ArgumentDescriptor::new("visible", ArgumentSchema::Boolean)], |arguments| match arguments {
                [ComponentArgument::Boolean(visible)] => Ok(ComponentPayload::new(AlertOp::Visible(*visible))), _ => Err("Alert.visible(visible) expects a boolean".into()),
            }).with_documentation("Controls whether the alert is rendered."),
            MethodDescriptor::new("size", vec![ArgumentDescriptor::new("size", ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]))], |arguments| size_operation("Alert", arguments, AlertOp::Size)).with_documentation("Sets the alert's semantic size."),
        ])
.with_documentation("A message banner with semantic default, info, success, warning, and error constructors."))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alert_payload_and_operations_build_the_real_component() {
        let payload = ComponentPayload::new(AlertPayload {
            id: "network".into(),
            message: "Offline".into(),
            variant: AlertVariant::Warning,
        });
        drop(
            AlertMaterializer::component(
                &payload,
                &[
                    AlertOp::Title("Connection".into()),
                    AlertOp::Banner,
                    AlertOp::Visible(true),
                    AlertOp::Size(Size::Small),
                ],
            )
            .unwrap()
            .into_any_element(),
        );
    }
    #[test]
    fn alert_rejects_an_incompatible_payload() {
        assert_eq!(
            AlertMaterializer::component(&ComponentPayload::new(()), std::iter::empty())
                .err()
                .unwrap()
                .to_string(),
            "Alert received an incompatible payload"
        );
    }
}
