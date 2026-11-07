use gpui_component::clipboard::Clipboard;
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, ParentElement as _},
};
use std::sync::Arc;

use super::common::{ensure_no_children, non_empty_id};
#[derive(Clone)]
struct ClipboardPayload {
    id: String,
}
#[derive(Clone)]
enum ClipboardOp {
    Value(String),
    Tooltip(String),
}
struct ClipboardMaterializer;
impl ClipboardMaterializer {
    fn component<'a>(
        payload: &ComponentPayload,
        operations: impl IntoIterator<Item = &'a ClipboardOp>,
    ) -> anyhow::Result<Clipboard> {
        let payload = payload
            .downcast_ref::<ClipboardPayload>()
            .ok_or_else(|| anyhow::anyhow!("Clipboard received an incompatible payload"))?;
        Ok(operations.into_iter().fold(
            Clipboard::new(payload.id.clone()),
            |component, operation| match operation {
                ClipboardOp::Value(value) => component.value(value.clone()),
                ClipboardOp::Tooltip(text) => component.tooltip(text.clone()),
            },
        ))
    }
}
impl ComponentMaterializer for ClipboardMaterializer {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        ensure_no_children("Clipboard", &request)?;
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<ClipboardOp>());
        let component = Self::component(request.payload(), operations)?;
        request.finish(gpui::div().child(component))
    }
}
fn string_method(
    name: &'static str,
    documentation: &'static str,
    wrap: fn(String) -> ClipboardOp,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::String)],
        move |arguments| match arguments {
            [ComponentArgument::String(value)] => Ok(ComponentPayload::new(wrap(value.clone()))),
            _ => Err(format!("Clipboard.{name}({name}) expects a string")),
        },
    )
    .with_documentation(documentation)
}
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("Clipboard", Arc::new(ClipboardMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "Clipboard",
                vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(id)] => {
                        Ok(ComponentPayload::new(ClipboardPayload {
                            id: non_empty_id("Clipboard", id)?,
                        }))
                    }
                    _ => Err("Clipboard(id) expects a string".into()),
                },
            )])
            .with_methods(vec![
                string_method(
                    "value",
                    "Sets the text copied when the button is pressed.",
                    ClipboardOp::Value,
                ),
                string_method(
                    "tooltip",
                    "Sets the copy button tooltip.",
                    ClipboardOp::Tooltip,
                ),
            ])
            .with_documentation(
                "A button that copies a configured string to the system clipboard.",
            ),
    )?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use gpui_shell::gpui::IntoElement as _;
    #[test]
    fn builds_real_clipboard() {
        drop(
            ClipboardMaterializer::component(
                &ComponentPayload::new(ClipboardPayload { id: "copy".into() }),
                &[
                    ClipboardOp::Value("value".into()),
                    ClipboardOp::Tooltip("Copy".into()),
                ],
            )
            .unwrap()
            .into_any_element(),
        );
    }

    #[test]
    fn rejects_an_incompatible_payload() {
        assert_eq!(
            ClipboardMaterializer::component(&ComponentPayload::new(()), std::iter::empty())
                .err()
                .unwrap()
                .to_string(),
            "Clipboard received an incompatible payload"
        );
    }
}
