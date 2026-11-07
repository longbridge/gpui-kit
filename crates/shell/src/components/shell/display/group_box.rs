use gpui_component::group_box::{GroupBox, GroupBoxVariant, GroupBoxVariants as _};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow, gpui,
};
use std::sync::Arc;
#[derive(Clone)]
struct GroupBoxPayload;
#[derive(Clone)]
enum GroupBoxOp {
    Title(String),
    Variant(GroupBoxVariant),
}
struct GroupBoxMaterializer;
impl GroupBoxMaterializer {
    fn component<'a>(
        payload: &ComponentPayload,
        operations: impl IntoIterator<Item = &'a GroupBoxOp>,
    ) -> anyhow::Result<GroupBox> {
        payload
            .downcast_ref::<GroupBoxPayload>()
            .ok_or_else(|| anyhow::anyhow!("GroupBox received an incompatible payload"))?;
        Ok(operations
            .into_iter()
            .fold(GroupBox::new(), |component, operation| match operation {
                GroupBoxOp::Title(title) => component.title(title.clone()),
                GroupBoxOp::Variant(variant) => component.with_variant(*variant),
            }))
    }
}
impl ComponentMaterializer for GroupBoxMaterializer {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<GroupBoxOp>());
        let component = Self::component(request.payload(), operations)?;
        request.finish(component)
    }
}
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("GroupBox", Arc::new(GroupBoxMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "GroupBox",
                Vec::new(),
                |_| Ok(ComponentPayload::new(GroupBoxPayload)),
            )])
            .with_methods(vec![
                MethodDescriptor::new(
                    "title",
                    vec![ArgumentDescriptor::new("title", ArgumentSchema::String)],
                    |arguments| match arguments {
                        [ComponentArgument::String(title)] => {
                            Ok(ComponentPayload::new(GroupBoxOp::Title(title.clone())))
                        }
                        _ => Err("GroupBox.title(title) expects a string".into()),
                    },
                )
                .with_documentation("Sets the group title."),
                MethodDescriptor::new(
                    "variant",
                    vec![ArgumentDescriptor::new(
                        "variant",
                        ArgumentSchema::Enum(&["normal", "fill", "outline"]),
                    )],
                    |arguments| match arguments {
                        [ComponentArgument::Enum(variant)] => match variant.as_str() {
                            "normal" => Ok(ComponentPayload::new(GroupBoxOp::Variant(
                                GroupBoxVariant::Normal,
                            ))),
                            "fill" => Ok(ComponentPayload::new(GroupBoxOp::Variant(
                                GroupBoxVariant::Fill,
                            ))),
                            "outline" => Ok(ComponentPayload::new(GroupBoxOp::Variant(
                                GroupBoxVariant::Outline,
                            ))),
                            _ => Err(format!("unsupported GroupBox variant `{variant}`")),
                        },
                        _ => Err("GroupBox.variant(variant) expects a variant literal".into()),
                    },
                )
                .with_documentation("Sets the normal, fill, or outline presentation."),
            ])
            .with_documentation("A titled container for grouping related content."),
    )?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use gpui_shell::gpui::IntoElement as _;
    #[test]
    fn builds_real_group_box() {
        drop(
            GroupBoxMaterializer::component(
                &ComponentPayload::new(GroupBoxPayload),
                &[
                    GroupBoxOp::Title("Options".into()),
                    GroupBoxOp::Variant(GroupBoxVariant::Outline),
                ],
            )
            .unwrap()
            .into_any_element(),
        );
    }

    #[test]
    fn rejects_an_incompatible_payload() {
        assert_eq!(
            GroupBoxMaterializer::component(&ComponentPayload::new(()), std::iter::empty())
                .err()
                .unwrap()
                .to_string(),
            "GroupBox received an incompatible payload"
        );
    }
}
