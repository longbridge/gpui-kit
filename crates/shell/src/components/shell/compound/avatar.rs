use std::sync::Arc;

use gpui_component::{Sizable as _, Size, avatar::Avatar};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, ParentElement as _, Refineable as _, Styled as _},
};

#[derive(Clone, Copy)]
struct AvatarPayload;

#[derive(Clone)]
enum AvatarOp {
    Name(String),
    Size(Size),
}

struct AvatarMaterializer;

impl AvatarMaterializer {
    fn component<'a>(
        payload: &ComponentPayload,
        ops: impl IntoIterator<Item = &'a AvatarOp>,
    ) -> anyhow::Result<Avatar> {
        payload
            .downcast_ref::<AvatarPayload>()
            .ok_or_else(|| anyhow::anyhow!("Avatar received an incompatible payload"))?;
        Ok(ops.into_iter().fold(Avatar::new(), |avatar, op| match op {
            AvatarOp::Name(name) => avatar.name(name.clone()),
            AvatarOp::Size(size) => avatar.with_size(*size),
        }))
    }
}

impl ComponentMaterializer for AvatarMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        anyhow::ensure!(
            request.children_len() == 0,
            "Avatar does not accept children"
        );
        let ops = request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<AvatarOp>());
        let mut wrapper = gpui::div().child(Self::component(request.payload(), ops)?);
        wrapper.style().refine(&request.take_style());
        Ok(wrapper.into_any_element())
    }
}

fn size_method() -> MethodDescriptor {
    MethodDescriptor::new(
        "size",
        vec![ArgumentDescriptor::new(
            "size",
            ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
        )],
        |args| match args {
            [ComponentArgument::Enum(value)] => match value.as_str() {
                "xsmall" => Ok(ComponentPayload::new(AvatarOp::Size(Size::XSmall))),
                "small" => Ok(ComponentPayload::new(AvatarOp::Size(Size::Small))),
                "medium" => Ok(ComponentPayload::new(AvatarOp::Size(Size::Medium))),
                "large" => Ok(ComponentPayload::new(AvatarOp::Size(Size::Large))),
                _ => Err(format!("unsupported Avatar size `{value}`")),
            },
            _ => Err("Avatar.size(size) expects a size literal".into()),
        },
    )
    .with_documentation("Sets the avatar's semantic size.")
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("Avatar", Arc::new(AvatarMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new("Avatar", vec![], |_| {
                Ok(ComponentPayload::new(AvatarPayload))
            })])
            .with_methods(vec![
                MethodDescriptor::new(
                    "name",
                    vec![ArgumentDescriptor::new("name", ArgumentSchema::String)],
                    |args| match args {
                        [ComponentArgument::String(name)] => {
                            Ok(ComponentPayload::new(AvatarOp::Name(name.clone())))
                        }
                        _ => Err("Avatar.name(name) expects a string".into()),
                    },
                )
                .with_documentation("Sets the person's name and generated initials fallback."),
                size_method(),
            ])
            .with_documentation("A circular avatar with a name-derived fallback."),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_avatar_accepts_recorded_operations() {
        let payload = ComponentPayload::new(AvatarPayload);
        drop(
            AvatarMaterializer::component(
                &payload,
                [
                    &AvatarOp::Name("Ada Lovelace".into()),
                    &AvatarOp::Size(Size::Large),
                ],
            )
            .unwrap()
            .into_any_element(),
        );
    }
    #[test]
    fn incompatible_payload_is_rejected() {
        assert!(AvatarMaterializer::component(&ComponentPayload::new(()), []).is_err());
    }
}
