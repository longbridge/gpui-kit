use gpui_component::status_bar::StatusBar;
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, ParentElement as _, Refineable as _, Styled as _},
};
use std::sync::Arc;
#[derive(Clone)]
struct StatusBarPayload;
#[derive(Clone)]
enum StatusBarOp {
    Left(ComponentArgument),
    Right(ComponentArgument),
}
struct StatusBarMaterializer;
impl StatusBarMaterializer {
    fn component(payload: &ComponentPayload) -> anyhow::Result<StatusBar> {
        payload
            .downcast_ref::<StatusBarPayload>()
            .ok_or_else(|| anyhow::anyhow!("StatusBar received an incompatible payload"))?;
        Ok(StatusBar::new())
    }
}
impl ComponentMaterializer for StatusBarMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let mut component = Self::component(request.payload())?;
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<StatusBarOp>().cloned())
            .collect::<Vec<_>>();
        for operation in operations {
            component = match operation {
                StatusBarOp::Left(argument) => component.left(request.resolve_element(&argument)?),
                StatusBarOp::Right(argument) => {
                    component.right(request.resolve_element(&argument)?)
                }
            };
        }
        component.style().refine(&request.take_style());
        component.extend(request.take_children()?);
        Ok(component.into_any_element())
    }
}
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor::new("StatusBar", Arc::new(StatusBarMaterializer))
.with_constructors(vec![ConstructorDescriptor::new("StatusBar",Vec::new(),|_|Ok(ComponentPayload::new(StatusBarPayload)))])
.with_methods(vec![
    MethodDescriptor::new("left_content", vec![ArgumentDescriptor::new("element", ArgumentSchema::Element)], |arguments| match arguments {
        [argument @ ComponentArgument::Element(_)] => Ok(ComponentPayload::new(StatusBarOp::Left(argument.clone()))),
        _ => Err("StatusBar.left_content(element) expects an element".into()),
    }).with_documentation("Appends content to the leading region."),
    MethodDescriptor::new("right_content", vec![ArgumentDescriptor::new("element", ArgumentSchema::Element)], |arguments| match arguments {
        [argument @ ComponentArgument::Element(_)] => Ok(ComponentPayload::new(StatusBarOp::Right(argument.clone()))),
        _ => Err("StatusBar.right_content(element) expects an element".into()),
    }).with_documentation("Appends content to the trailing region."),
])
.with_documentation("A three-region status bar; ordinary children fill the center and named left/right slots pin content to each edge."))?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_real_status_bar() {
        drop(
            StatusBarMaterializer::component(&ComponentPayload::new(StatusBarPayload))
                .unwrap()
                .into_any_element(),
        );
    }

    #[test]
    fn rejects_an_incompatible_payload() {
        assert_eq!(
            StatusBarMaterializer::component(&ComponentPayload::new(()))
                .err()
                .unwrap()
                .to_string(),
            "StatusBar received an incompatible payload"
        );
    }
}
