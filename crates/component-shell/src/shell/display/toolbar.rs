use super::common::non_empty_id;
use gpui_component::toolbar::Toolbar;
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, RegistryError, anyhow,
    gpui::{self, IntoElement as _, ParentElement as _, Refineable as _, Styled as _},
};
use std::sync::Arc;
#[derive(Clone)]
struct ToolbarPayload {
    id: String,
}
struct ToolbarMaterializer;
impl ToolbarMaterializer {
    fn component(payload: &ComponentPayload) -> anyhow::Result<Toolbar> {
        let payload = payload
            .downcast_ref::<ToolbarPayload>()
            .ok_or_else(|| anyhow::anyhow!("Toolbar received an incompatible payload"))?;
        Ok(Toolbar::new(payload.id.clone()))
    }
}
impl ComponentMaterializer for ToolbarMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let mut component = Self::component(request.payload())?;
        component.style().refine(&request.take_style());
        component.extend(request.take_children()?);
        Ok(component.into_any_element())
    }
}
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("Toolbar", Arc::new(ToolbarMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "Toolbar",
                vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(id)] => Ok(ComponentPayload::new(ToolbarPayload {
                        id: non_empty_id("Toolbar", id)?,
                    })),
                    _ => Err("Toolbar(id) expects a string".into()),
                },
            )])
            .with_documentation(
                "A transparent, sizable toolbar container whose controls render in source order.",
            ),
    )?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_real_toolbar() {
        drop(
            ToolbarMaterializer::component(&ComponentPayload::new(ToolbarPayload {
                id: "toolbar".into(),
            }))
            .unwrap()
            .into_any_element(),
        );
    }

    #[test]
    fn rejects_an_incompatible_payload() {
        assert_eq!(
            ToolbarMaterializer::component(&ComponentPayload::new(()))
                .err()
                .unwrap()
                .to_string(),
            "Toolbar received an incompatible payload"
        );
    }
}
