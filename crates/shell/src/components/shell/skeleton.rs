use std::sync::Arc;

use gpui_component::skeleton::Skeleton;
use gpui_shell::{
    ComponentDescriptor, ComponentMaterializer, ComponentPayload, ComponentRegistry,
    ConstructorDescriptor, MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, Refineable as _, Styled as _},
};

#[derive(Clone, Copy)]
struct SkeletonPayload;

#[derive(Clone, Copy)]
struct Secondary;

struct SkeletonMaterializer;

impl SkeletonMaterializer {
    fn component<'a>(
        payload: &ComponentPayload,
        operations: impl IntoIterator<Item = &'a Secondary>,
    ) -> anyhow::Result<Skeleton> {
        payload
            .downcast_ref::<SkeletonPayload>()
            .ok_or_else(|| anyhow::anyhow!("Skeleton received an incompatible payload"))?;
        Ok(operations
            .into_iter()
            .fold(Skeleton::new(), |component, _| component.secondary()))
    }
}

impl ComponentMaterializer for SkeletonMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<Secondary>());
        let mut element = Self::component(request.payload(), operations)?;
        element.style().refine(&request.take_style());
        Ok(element.into_any_element())
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("Skeleton", Arc::new(SkeletonMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "Skeleton",
                Vec::new(),
                |_| Ok(ComponentPayload::new(SkeletonPayload)),
            )])
            .with_methods(vec![
                MethodDescriptor::new("secondary", Vec::new(), |_| {
                    Ok(ComponentPayload::new(Secondary))
                })
                .with_documentation("Uses the secondary skeleton color."),
            ])
            .with_documentation("An animated loading placeholder."),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_payload_materializes_a_real_component_element() {
        let payload = ComponentPayload::new(SkeletonPayload);
        drop(
            SkeletonMaterializer::component(&payload, std::iter::empty())
                .unwrap()
                .into_any_element(),
        );
    }

    #[test]
    fn skeleton_rejects_an_incompatible_payload() {
        let error = SkeletonMaterializer::component(&ComponentPayload::new(()), std::iter::empty())
            .err()
            .unwrap();
        assert_eq!(
            error.to_string(),
            "Skeleton received an incompatible payload"
        );
    }

    #[test]
    fn secondary_operation_materializes_a_real_skeleton() {
        let payload = ComponentPayload::new(SkeletonPayload);
        drop(
            SkeletonMaterializer::component(&payload, [&Secondary])
                .unwrap()
                .into_any_element(),
        );
    }
}
