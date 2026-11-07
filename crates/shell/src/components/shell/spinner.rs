use std::sync::Arc;

use gpui_component::{IconName, Sizable as _, Size, spinner::Spinner, try_parse_color};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{
        self, IntoElement as _, ParentElement as _, Refineable as _, Styled as _, ease_in_out,
        ease_out_quint, linear,
    },
};

#[derive(Clone, Copy)]
struct SpinnerPayload;

#[derive(Clone)]
enum SpinnerOp {
    Size(Size),
    Icon(IconName),
    Color(gpui::Hsla),
    Ease(SpinnerEase),
}

#[derive(Clone, Copy)]
enum SpinnerEase {
    Linear,
    EaseInOut,
    EaseOutQuint,
}

struct SpinnerMaterializer;

impl SpinnerMaterializer {
    fn component<'a>(
        payload: &ComponentPayload,
        operations: impl IntoIterator<Item = &'a SpinnerOp>,
    ) -> anyhow::Result<Spinner> {
        payload
            .downcast_ref::<SpinnerPayload>()
            .ok_or_else(|| anyhow::anyhow!("Spinner received an incompatible payload"))?;
        Ok(operations
            .into_iter()
            .fold(Spinner::new(), |component, operation| match operation {
                SpinnerOp::Size(size) => component.with_size(*size),
                SpinnerOp::Icon(icon) => component.icon(icon.clone()),
                SpinnerOp::Color(color) => component.color(*color),
                SpinnerOp::Ease(SpinnerEase::Linear) => component.ease(linear),
                SpinnerOp::Ease(SpinnerEase::EaseInOut) => component.ease(ease_in_out),
                SpinnerOp::Ease(SpinnerEase::EaseOutQuint) => component.ease(ease_out_quint()),
            }))
    }
}

impl ComponentMaterializer for SpinnerMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<SpinnerOp>());
        let mut element = gpui::div().child(Self::component(request.payload(), operations)?);
        element.style().refine(&request.take_style());
        Ok(element.into_any_element())
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("Spinner", Arc::new(SpinnerMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "Spinner",
                Vec::new(),
                |_| Ok(ComponentPayload::new(SpinnerPayload)),
            )])
            .with_methods(vec![
                MethodDescriptor::new(
                    "size",
                    vec![ArgumentDescriptor::new(
                        "size",
                        ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
                    )],
                    |arguments| match arguments {
                        [ComponentArgument::Enum(size)] => match size.as_str() {
                            "xsmall" => Ok(ComponentPayload::new(SpinnerOp::Size(Size::XSmall))),
                            "small" => Ok(ComponentPayload::new(SpinnerOp::Size(Size::Small))),
                            "medium" => Ok(ComponentPayload::new(SpinnerOp::Size(Size::Medium))),
                            "large" => Ok(ComponentPayload::new(SpinnerOp::Size(Size::Large))),
                            _ => Err(format!("unsupported Spinner size `{size}`")),
                        },
                        _ => Err("Spinner.size(size) expects a size literal".into()),
                    },
                )
                .with_documentation("Sets the spinner's semantic size."),
                MethodDescriptor::new(
                    "icon",
                    vec![ArgumentDescriptor::new(
                        "icon",
                        ArgumentSchema::Enum(&["loader", "loader_circle"]),
                    )],
                    |arguments| match arguments {
                        [ComponentArgument::Enum(icon)] => match icon.as_str() {
                            "loader" => {
                                Ok(ComponentPayload::new(SpinnerOp::Icon(IconName::Loader)))
                            }
                            "loader_circle" => Ok(ComponentPayload::new(SpinnerOp::Icon(
                                IconName::LoaderCircle,
                            ))),
                            _ => Err(format!("unsupported Spinner icon `{icon}`")),
                        },
                        _ => Err("Spinner.icon(icon) expects an icon literal".into()),
                    },
                )
                .with_documentation("Selects the icon rotated by the spinner."),
                MethodDescriptor::new(
                    "color",
                    vec![ArgumentDescriptor::new("color", ArgumentSchema::String)],
                    |arguments| match arguments {
                        [ComponentArgument::String(color)] => try_parse_color(color)
                            .map(|color| ComponentPayload::new(SpinnerOp::Color(color)))
                            .map_err(|error| format!("invalid Spinner color: {error}")),
                        _ => Err("Spinner.color(color) expects a color string".into()),
                    },
                )
                .with_documentation("Sets the spinner icon color."),
                MethodDescriptor::new(
                    "ease",
                    vec![ArgumentDescriptor::new(
                        "ease",
                        ArgumentSchema::Enum(&["linear", "ease_in_out", "ease_out_quint"]),
                    )],
                    |arguments| match arguments {
                        [ComponentArgument::Enum(ease)] => match ease.as_str() {
                            "linear" => {
                                Ok(ComponentPayload::new(SpinnerOp::Ease(SpinnerEase::Linear)))
                            }
                            "ease_in_out" => Ok(ComponentPayload::new(SpinnerOp::Ease(
                                SpinnerEase::EaseInOut,
                            ))),
                            "ease_out_quint" => Ok(ComponentPayload::new(SpinnerOp::Ease(
                                SpinnerEase::EaseOutQuint,
                            ))),
                            _ => Err(format!("unsupported Spinner easing `{ease}`")),
                        },
                        _ => Err("Spinner.ease(ease) expects an easing literal".into()),
                    },
                )
                .with_documentation("Sets the spinner rotation easing curve."),
            ])
            .with_documentation("A cycling loading spinner."),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinner_payload_materializes_a_real_component_element() {
        let payload = ComponentPayload::new(SpinnerPayload);
        drop(
            SpinnerMaterializer::component(&payload, std::iter::empty())
                .unwrap()
                .into_any_element(),
        );
    }

    #[test]
    fn spinner_rejects_an_incompatible_payload() {
        let error = SpinnerMaterializer::component(&ComponentPayload::new(()), std::iter::empty())
            .err()
            .unwrap();
        assert_eq!(
            error.to_string(),
            "Spinner received an incompatible payload"
        );
    }

    #[test]
    fn spinner_operations_materialize_a_real_component() {
        let payload = ComponentPayload::new(SpinnerPayload);
        let operations = [
            SpinnerOp::Size(Size::Large),
            SpinnerOp::Icon(IconName::LoaderCircle),
            SpinnerOp::Color(try_parse_color("blue-600").unwrap()),
            SpinnerOp::Ease(SpinnerEase::Linear),
        ];
        drop(
            SpinnerMaterializer::component(&payload, &operations)
                .unwrap()
                .into_any_element(),
        );
    }
}
