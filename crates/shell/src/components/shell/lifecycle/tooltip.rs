use std::sync::Arc;

use gpui_component::button::Button;
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, RegistryError, anyhow, gpui,
};

#[derive(Clone)]
struct TooltipPayload {
    id: String,
    label: String,
    text: String,
}

struct TooltipMaterializer;

impl ComponentMaterializer for TooltipMaterializer {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<TooltipPayload>()
            .ok_or_else(|| anyhow::anyhow!("Tooltip received an incompatible payload"))?
            .clone();

        #[cfg(test)]
        test_probe::record(&payload);

        request.finish(
            Button::new(payload.id.clone())
                .label(payload.label.clone())
                .tooltip(payload.text.clone()),
        )
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("Tooltip", Arc::new(TooltipMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "Tooltip",
                vec![
                    ArgumentDescriptor::new("id", ArgumentSchema::String),
                    ArgumentDescriptor::new("label", ArgumentSchema::String),
                    ArgumentDescriptor::new("text", ArgumentSchema::String),
                ],
                |arguments| match arguments {
                    [
                        ComponentArgument::String(id),
                        ComponentArgument::String(label),
                        ComponentArgument::String(text),
                    ] if !id.trim().is_empty()
                        && !label.trim().is_empty()
                        && !text.trim().is_empty() =>
                    {
                        Ok(ComponentPayload::new(TooltipPayload {
                            id: id.clone(),
                            label: label.clone(),
                            text: text.clone(),
                        }))
                    }
                    [
                        ComponentArgument::String(_),
                        ComponentArgument::String(_),
                        ComponentArgument::String(_),
                    ] => Err("Tooltip id, label, and text must not be empty".into()),
                    _ => Err("Tooltip(id, label, text) expects three strings".into()),
                },
            )])
            .with_methods(vec![])
            .with_documentation(
                "A real gpui-component Button trigger with a managed text tooltip.",
            ),
    )?;
    Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_probe {
    use super::TooltipPayload;
    use std::cell::RefCell;

    thread_local! {
        static PAYLOADS: RefCell<Vec<(String, String, String)>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn record(payload: &TooltipPayload) {
        PAYLOADS.with(|payloads| {
            payloads.borrow_mut().push((
                payload.id.clone(),
                payload.label.clone(),
                payload.text.clone(),
            ));
        });
    }

    pub(crate) fn take() -> Vec<(String, String, String)> {
        PAYLOADS.with(|payloads| std::mem::take(&mut *payloads.borrow_mut()))
    }
}
