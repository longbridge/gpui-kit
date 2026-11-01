use super::common::{ensure_no_children, non_empty_id, size_operation};
use gpui_component::{Size, rating::Rating, try_parse_color};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallbackArgument,
    ComponentDescriptor, ComponentMaterializer, ComponentPayload, ComponentRegistry,
    ConstructorDescriptor, MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, Refineable as _, Styled as _},
};
use std::sync::Arc;
#[derive(Clone)]
struct RatingPayload {
    id: String,
}
#[derive(Clone)]
enum RatingOp {
    Value(usize),
    Max(usize),
    Color(gpui::Hsla),
    Size(Size),
    OnChange(ComponentArgument),
}
struct RatingMaterializer;
impl RatingMaterializer {
    fn component<'a>(
        payload: &ComponentPayload,
        operations: impl IntoIterator<Item = &'a RatingOp>,
        disabled: bool,
    ) -> anyhow::Result<Rating> {
        let payload = payload
            .downcast_ref::<RatingPayload>()
            .ok_or_else(|| anyhow::anyhow!("Rating received an incompatible payload"))?;
        let operations = operations.into_iter().collect::<Vec<_>>();
        let (value, max) = rating_settings(&operations);
        Ok(operations.into_iter().fold(
            Rating::new(payload.id.clone())
                .disabled(disabled)
                .max(max)
                .value(value),
            |component, operation| match operation {
                RatingOp::Value(_) | RatingOp::Max(_) | RatingOp::OnChange(_) => component,
                RatingOp::Color(color) => component.color(*color),
                RatingOp::Size(size) => component.with_size(*size),
            },
        ))
    }
}

fn rating_settings(operations: &[&RatingOp]) -> (usize, usize) {
    let (mut value, mut max) = (0, 5);
    for operation in operations {
        match operation {
            RatingOp::Value(next) => value = (*next).min(max),
            RatingOp::Max(next) => {
                max = *next;
                value = value.min(max);
            }
            RatingOp::Color(_) | RatingOp::Size(_) | RatingOp::OnChange(_) => {}
        }
    }
    (value, max)
}
impl ComponentMaterializer for RatingMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        ensure_no_children("Rating", &request)?;
        let change = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<RatingOp>())
            .filter_map(|operation| match operation {
                RatingOp::OnChange(argument) => Some(argument.clone()),
                _ => None,
            })
            .last();
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<RatingOp>());
        let mut component = Self::component(request.payload(), operations, request.disabled())?;
        if let Some(argument) = change {
            let callback = request.resolve_callback(&argument)?;
            component = component.on_click(move |value, window, cx| {
                callback.invoke_and_report_with(
                    "Rating.on_change callback failed",
                    &[ComponentCallbackArgument::Number(*value as f64)],
                    window,
                    cx,
                );
            });
        }
        component.style().refine(&request.take_style());
        Ok(component.into_any_element())
    }
}
fn count_method(
    name: &'static str,
    documentation: &'static str,
    wrap: fn(usize) -> RatingOp,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Number)],
        move |arguments| match arguments {
            [ComponentArgument::Number(value)] => {
                rating_count(*value, name).map(|value| ComponentPayload::new(wrap(value)))
            }
            _ => Err(format!("Rating.{name}({name}) expects a number")),
        },
    )
    .with_documentation(documentation)
}

fn rating_count(value: f64, name: &str) -> Result<usize, String> {
    // `usize::MAX as f64` rounds to 2^64 on 64-bit targets, so the upper
    // boundary must be exclusive before converting with `as`.
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value >= usize::MAX as f64 {
        return Err(format!(
            "Rating.{name}({name}) expects a non-negative integer, got {value}"
        ));
    }
    Ok(value as usize)
}
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("Rating", Arc::new(RatingMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "Rating",
                vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(id)] => Ok(ComponentPayload::new(RatingPayload {
                        id: non_empty_id("Rating", id)?,
                    })),
                    _ => Err("Rating(id) expects a string".into()),
                },
            )])
            .with_methods(vec![
                crate::components::shell::support::disabled_method("Rating"),
                count_method(
                    "value",
                    "Sets the current number of active stars.",
                    RatingOp::Value,
                ),
                count_method("max", "Sets the maximum number of stars.", RatingOp::Max),
                MethodDescriptor::new(
                    "on_change",
                    vec![ArgumentDescriptor::new(
                        "on_change",
                        ArgumentSchema::Callback("(value: number, cx: Context) => void"),
                    )],
                    |arguments| match arguments {
                        [argument @ ComponentArgument::Callback(_)] => {
                            Ok(ComponentPayload::new(RatingOp::OnChange(argument.clone())))
                        }
                        _ => Err("Rating.on_change expects one callback".into()),
                    },
                )
                .with_documentation(
                    "Reports the star the reader clicked, so the script can drive `value`.",
                ),
                MethodDescriptor::new(
                    "color",
                    vec![ArgumentDescriptor::new("color", ArgumentSchema::String)],
                    |arguments| match arguments {
                        [ComponentArgument::String(color)] => try_parse_color(color)
                            .map(|color| ComponentPayload::new(RatingOp::Color(color)))
                            .map_err(|error| format!("invalid Rating color: {error}")),
                        _ => Err("Rating.color(color) expects a color string".into()),
                    },
                )
                .with_documentation("Sets the active star color."),
                MethodDescriptor::new(
                    "size",
                    vec![ArgumentDescriptor::new(
                        "size",
                        ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
                    )],
                    |arguments| size_operation("Rating", arguments, RatingOp::Size),
                )
                .with_documentation("Sets the rating's semantic size."),
            ])
            .with_documentation("An interactive star rating with configurable value and maximum."),
    )?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_real_rating() {
        drop(
            RatingMaterializer::component(
                &ComponentPayload::new(RatingPayload {
                    id: "quality".into(),
                }),
                &[
                    RatingOp::Value(3),
                    RatingOp::Max(5),
                    RatingOp::Size(Size::Small),
                ],
                false,
            )
            .unwrap()
            .into_any_element(),
        );
    }

    #[test]
    fn rejects_an_incompatible_payload() {
        assert_eq!(
            RatingMaterializer::component(&ComponentPayload::new(()), std::iter::empty(), false)
                .err()
                .unwrap()
                .to_string(),
            "Rating received an incompatible payload"
        );
    }

    #[test]
    fn later_value_and_max_operations_win_in_call_order() {
        assert_eq!(
            rating_settings(&[&RatingOp::Value(4), &RatingOp::Max(3), &RatingOp::Value(2),]),
            (2, 3)
        );
    }

    #[test]
    fn rejects_the_rounded_usize_overflow_boundary() {
        assert!(rating_count(usize::MAX as f64, "max").is_err());
    }
}
