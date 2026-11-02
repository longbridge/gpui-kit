use gpui_component::breadcrumb::Breadcrumb;
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, RegistryError, anyhow,
    gpui::{self, IntoElement as _, Refineable as _, Styled as _},
};
use std::sync::Arc;

use super::common::ensure_no_children;

#[derive(Clone)]
struct BreadcrumbPayload(Vec<String>);
struct BreadcrumbMaterializer;
impl BreadcrumbMaterializer {
    fn component(payload: &ComponentPayload) -> anyhow::Result<Breadcrumb> {
        let payload = payload
            .downcast_ref::<BreadcrumbPayload>()
            .ok_or_else(|| anyhow::anyhow!("Breadcrumb received an incompatible payload"))?;
        Ok(Breadcrumb::new().children(payload.0.clone()))
    }
}
impl ComponentMaterializer for BreadcrumbMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        ensure_no_children("Breadcrumb", &request)?;
        let mut component = Self::component(request.payload())?;
        component.style().refine(&request.take_style());
        Ok(component.into_any_element())
    }
}
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("Breadcrumb", Arc::new(BreadcrumbMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "Breadcrumb",
                vec![ArgumentDescriptor::new(
                    "labels",
                    ArgumentSchema::Array(Box::new(ArgumentSchema::String)),
                )],
                |arguments| match arguments {
                    [ComponentArgument::Array(labels)] => labels
                        .iter()
                        .map(|label| match label {
                            ComponentArgument::String(label) => Ok(label.clone()),
                            _ => Err("Breadcrumb(labels) expects an array of strings".into()),
                        })
                        .collect::<Result<Vec<_>, String>>()
                        .map(|labels| ComponentPayload::new(BreadcrumbPayload(labels))),
                    _ => Err("Breadcrumb(labels) expects an array of strings".into()),
                },
            )])
            .with_methods(Vec::new())
            .with_documentation("A navigation trail built from an ordered array of labels."),
    )?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_real_breadcrumb() {
        drop(
            BreadcrumbMaterializer::component(&ComponentPayload::new(BreadcrumbPayload(vec![
                "Home".into(),
                "Settings".into(),
            ])))
            .unwrap()
            .into_any_element(),
        );
    }

    #[test]
    fn rejects_an_incompatible_payload() {
        assert_eq!(
            BreadcrumbMaterializer::component(&ComponentPayload::new(()))
                .err()
                .unwrap()
                .to_string(),
            "Breadcrumb received an incompatible payload"
        );
    }
}
