use super::common::non_empty_id;
use gpui_component::toolbar::Toolbar;
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, ParentElement as _, Refineable as _, Styled as _},
};
use std::sync::Arc;
#[derive(Clone)]
struct ToolbarPayload {
    id: String,
}
#[derive(Clone)]
enum ToolbarOp {
    Left(ComponentArgument),
    Right(ComponentArgument),
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
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<ToolbarOp>().cloned())
            .collect::<Vec<_>>();
        for operation in operations {
            component = match operation {
                ToolbarOp::Left(argument) => component.left(request.resolve_element(&argument)?),
                ToolbarOp::Right(argument) => component.right(request.resolve_element(&argument)?),
            };
        }
        component.style().refine(&request.take_style());
        component.extend(request.take_children()?);
        Ok(component.into_any_element())
    }
}
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor::new("Toolbar", Arc::new(ToolbarMaterializer))
.with_constructors(vec![ConstructorDescriptor::new("Toolbar",vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],|arguments| match arguments {
    [ComponentArgument::String(id)] => Ok(ComponentPayload::new(ToolbarPayload { id: non_empty_id("Toolbar", id)? })),
    _ => Err("Toolbar(id) expects a string".into()),
})])
.with_methods(vec![
    MethodDescriptor::new("left_content", vec![ArgumentDescriptor::new("element", ArgumentSchema::Element)], |arguments| match arguments {
        [argument @ ComponentArgument::Element(_)] => Ok(ComponentPayload::new(ToolbarOp::Left(argument.clone()))),
        _ => Err("Toolbar.left_content(element) expects an element".into()),
    }).with_documentation("Appends content to the leading region."),
    MethodDescriptor::new("right_content", vec![ArgumentDescriptor::new("element", ArgumentSchema::Element)], |arguments| match arguments {
        [argument @ ComponentArgument::Element(_)] => Ok(ComponentPayload::new(ToolbarOp::Right(argument.clone()))),
        _ => Err("Toolbar.right_content(element) expects an element".into()),
    }).with_documentation("Appends content to the trailing region."),
])
.with_documentation("A themed toolbar that hosts a row of actions; ordinary children fill the center and named left/right slots pin content to each edge."))?;
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
