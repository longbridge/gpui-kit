use std::sync::Arc;

use gpui_component::{separator::Separator, try_parse_color};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, Refineable as _, Styled as _},
};

#[derive(Clone, Copy)]
enum SeparatorPayload {
    Horizontal,
    Vertical,
    HorizontalDashed,
    VerticalDashed,
}

#[derive(Clone)]
enum SeparatorOp {
    Label(String),
    Color(gpui::Hsla),
    Dashed,
}

impl SeparatorPayload {
    fn into_component(self) -> Separator {
        match self {
            Self::Horizontal => Separator::horizontal(),
            Self::Vertical => Separator::vertical(),
            Self::HorizontalDashed => Separator::horizontal_dashed(),
            Self::VerticalDashed => Separator::vertical_dashed(),
        }
    }
}

struct SeparatorMaterializer;

impl SeparatorMaterializer {
    fn component<'a>(
        payload: &ComponentPayload,
        operations: impl IntoIterator<Item = &'a SeparatorOp>,
    ) -> anyhow::Result<Separator> {
        let payload = payload
            .downcast_ref::<SeparatorPayload>()
            .ok_or_else(|| anyhow::anyhow!("Separator received an incompatible payload"))?;
        Ok(operations.into_iter().fold(
            payload.to_owned().into_component(),
            |component, operation| match operation {
                SeparatorOp::Label(label) => component.label(label.clone()),
                SeparatorOp::Color(color) => component.color(*color),
                SeparatorOp::Dashed => component.dashed(),
            },
        ))
    }
}

impl ComponentMaterializer for SeparatorMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<SeparatorOp>());
        let mut element = Self::component(request.payload(), operations)?;
        element.style().refine(&request.take_style());
        Ok(element.into_any_element())
    }
}

fn constructor(export: &'static str, payload: SeparatorPayload) -> ConstructorDescriptor {
    ConstructorDescriptor::new(export, Vec::new(), move |_| {
        Ok(ComponentPayload::new(payload))
    })
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("Separator", Arc::new(SeparatorMaterializer))
            .with_constructors(vec![
                constructor("Separator", SeparatorPayload::Horizontal),
                constructor("VerticalSeparator", SeparatorPayload::Vertical),
                constructor("DashedSeparator", SeparatorPayload::HorizontalDashed),
                constructor("VerticalDashedSeparator", SeparatorPayload::VerticalDashed),
            ])
            .with_methods(vec![
                MethodDescriptor::new(
                    "label",
                    vec![ArgumentDescriptor::new("label", ArgumentSchema::String)],
                    |arguments| match arguments {
                        [ComponentArgument::String(label)] => {
                            Ok(ComponentPayload::new(SeparatorOp::Label(label.clone())))
                        }
                        _ => Err("Separator.label(label) expects a string".into()),
                    },
                )
                .with_documentation("Displays text centered over the separator line."),
                MethodDescriptor::new(
                    "color",
                    vec![ArgumentDescriptor::new("color", ArgumentSchema::String)],
                    |arguments| match arguments {
                        [ComponentArgument::String(color)] => try_parse_color(color)
                            .map(|color| ComponentPayload::new(SeparatorOp::Color(color)))
                            .map_err(|error| format!("invalid Separator color: {error}")),
                        _ => Err("Separator.color(color) expects a color string".into()),
                    },
                )
                .with_documentation("Sets the separator line color."),
                MethodDescriptor::new("dashed", Vec::new(), |_| {
                    Ok(ComponentPayload::new(SeparatorOp::Dashed))
                })
                .with_documentation("Uses a dashed separator line."),
            ])
            .with_documentation("A horizontal or vertical, solid or dashed separator."),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separator_variants_materialize_real_component_elements() {
        for payload in [
            SeparatorPayload::Horizontal,
            SeparatorPayload::Vertical,
            SeparatorPayload::HorizontalDashed,
            SeparatorPayload::VerticalDashed,
        ] {
            drop(
                SeparatorMaterializer::component(
                    &ComponentPayload::new(payload),
                    std::iter::empty(),
                )
                .unwrap()
                .into_any_element(),
            );
        }
    }

    #[test]
    fn separator_rejects_an_incompatible_payload() {
        let error =
            SeparatorMaterializer::component(&ComponentPayload::new(()), std::iter::empty())
                .err()
                .unwrap();
        assert_eq!(
            error.to_string(),
            "Separator received an incompatible payload"
        );
    }

    #[test]
    fn separator_operations_materialize_a_real_component() {
        let payload = ComponentPayload::new(SeparatorPayload::Horizontal);
        let operations = [
            SeparatorOp::Label("Account".into()),
            SeparatorOp::Color(try_parse_color("blue-600").unwrap()),
            SeparatorOp::Dashed,
        ];
        drop(
            SeparatorMaterializer::component(&payload, &operations)
                .unwrap()
                .into_any_element(),
        );
    }
}
