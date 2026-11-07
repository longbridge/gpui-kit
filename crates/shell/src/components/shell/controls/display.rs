use std::sync::Arc;

use gpui_component::{
    Sizable as _, Size,
    badge::Badge,
    tag::{Tag, TagVariant},
    try_parse_color,
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, ParentElement as _, Refineable as _, Styled as _},
};

#[derive(Clone, Copy)]
struct UnitPayload;

#[derive(Clone)]
enum BadgeOp {
    Size(gpui_component::Size),
    Dot,
    Count(usize),
    Max(usize),
    Color(gpui::Hsla),
}

#[derive(Clone)]
enum TagOp {
    Size(Size),
    Variant(TagVariant),
    Outline,
    RoundedFull,
}

fn nullary(export: &'static str) -> ConstructorDescriptor {
    ConstructorDescriptor::new(export, Vec::new(), |_| {
        Ok(ComponentPayload::new(UnitPayload))
    })
}

fn natural_number(name: &'static str, make: fn(usize) -> BadgeOp) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Number)],
        move |arguments| match arguments {
            [ComponentArgument::Number(value)] => parse_natural(*value)
                .map(make)
                .map(ComponentPayload::new)
                .ok_or_else(|| format!("Badge.{name} expects a non-negative integer")),
            _ => Err(format!("Badge.{name} expects a non-negative integer")),
        },
    )
    .with_documentation(if name == "count" {
        "Sets the displayed count; zero hides a numeric badge."
    } else {
        "Sets the largest count displayed before the plus suffix."
    })
}

fn parse_natural(value: f64) -> Option<usize> {
    (value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value < usize::MAX as f64)
        .then_some(value as usize)
}

struct BadgeMaterializer;

impl ComponentMaterializer for BadgeMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<UnitPayload>()
            .ok_or_else(|| anyhow::anyhow!("Badge received an incompatible payload"))?;
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<BadgeOp>().cloned())
            .collect::<Vec<_>>();
        let mut component = Badge::new();
        for operation in operations {
            component = match operation {
                BadgeOp::Size(size) => component.with_size(size),
                BadgeOp::Dot => component.dot(),
                BadgeOp::Count(count) => component.count(count),
                BadgeOp::Max(max) => component.max(max),
                BadgeOp::Color(color) => component.color(color),
            };
        }
        component.extend(request.take_children()?);
        let mut wrapper = gpui::div().child(component);
        wrapper.style().refine(&request.take_style());
        Ok(wrapper.into_any_element())
    }
}

struct TagMaterializer;

impl ComponentMaterializer for TagMaterializer {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<UnitPayload>()
            .ok_or_else(|| anyhow::anyhow!("Tag received an incompatible payload"))?;
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<TagOp>().cloned())
            .collect::<Vec<_>>();
        let mut component = Tag::new();
        for operation in operations {
            component = match operation {
                TagOp::Size(size) => component.with_size(size),
                TagOp::Variant(variant) => component.with_variant(variant),
                TagOp::Outline => component.outline(),
                TagOp::RoundedFull => component.rounded_full(),
            };
        }
        request.finish(component)
    }
}

fn size_method<T: 'static + Send + Sync>(make: fn(gpui_component::Size) -> T) -> MethodDescriptor {
    MethodDescriptor::new(
        "size",
        vec![ArgumentDescriptor::new(
            "size",
            ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
        )],
        move |args| match args {
            [ComponentArgument::Enum(value)] => match value.as_str() {
                "xsmall" => Ok(ComponentPayload::new(make(gpui_component::Size::XSmall))),
                "small" => Ok(ComponentPayload::new(make(gpui_component::Size::Small))),
                "medium" => Ok(ComponentPayload::new(make(gpui_component::Size::Medium))),
                "large" => Ok(ComponentPayload::new(make(gpui_component::Size::Large))),
                _ => Err(format!("unsupported size `{value}`")),
            },
            _ => Err("size expects a semantic size literal".into()),
        },
    )
    .with_documentation("Sets the semantic component size.")
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("Badge", Arc::new(BadgeMaterializer))
            .with_constructors(vec![nullary("Badge")])
            .with_methods(vec![
                MethodDescriptor::new("dot", Vec::new(), |_| {
                    Ok(ComponentPayload::new(BadgeOp::Dot))
                })
                .with_documentation("Displays a dot instead of a numeric count."),
                natural_number("count", BadgeOp::Count),
                natural_number("max", BadgeOp::Max),
                MethodDescriptor::new(
                    "color",
                    vec![ArgumentDescriptor::new("color", ArgumentSchema::String)],
                    |arguments| match arguments {
                        [ComponentArgument::String(value)] => try_parse_color(value)
                            .map(BadgeOp::Color)
                            .map(ComponentPayload::new)
                            .map_err(|error| format!("invalid Badge color: {error}")),
                        _ => Err("Badge.color expects a color string".into()),
                    },
                )
                .with_documentation("Sets the badge background from a supported color token."),
                size_method(BadgeOp::Size),
            ])
            .with_documentation("A count or dot badge positioned over its ordinary children."),
    )?;
    registry.register(
        ComponentDescriptor::new("Tag", Arc::new(TagMaterializer))
            .with_constructors(vec![nullary("Tag")])
            .with_methods(vec![
                MethodDescriptor::new(
                    "variant",
                    vec![ArgumentDescriptor::new(
                        "variant",
                        ArgumentSchema::Enum(&[
                            "primary",
                            "secondary",
                            "danger",
                            "success",
                            "warning",
                            "info",
                        ]),
                    )],
                    |arguments| match arguments {
                        [ComponentArgument::Enum(value)] => match value.as_str() {
                            "primary" => {
                                Ok(ComponentPayload::new(TagOp::Variant(TagVariant::Primary)))
                            }
                            "secondary" => {
                                Ok(ComponentPayload::new(TagOp::Variant(TagVariant::Secondary)))
                            }
                            "danger" => {
                                Ok(ComponentPayload::new(TagOp::Variant(TagVariant::Danger)))
                            }
                            "success" => {
                                Ok(ComponentPayload::new(TagOp::Variant(TagVariant::Success)))
                            }
                            "warning" => {
                                Ok(ComponentPayload::new(TagOp::Variant(TagVariant::Warning)))
                            }
                            "info" => Ok(ComponentPayload::new(TagOp::Variant(TagVariant::Info))),
                            _ => Err(format!("unsupported Tag variant `{value}`")),
                        },
                        _ => Err("Tag.variant expects a supported variant".into()),
                    },
                )
                .with_documentation("Sets the semantic tag variant."),
                MethodDescriptor::new("outline", Vec::new(), |_| {
                    Ok(ComponentPayload::new(TagOp::Outline))
                })
                .with_documentation("Uses the outline presentation."),
                MethodDescriptor::new("rounded_full", Vec::new(), |_| {
                    Ok(ComponentPayload::new(TagOp::RoundedFull))
                })
                .with_documentation("Uses pill-shaped corners."),
                size_method(TagOp::Size),
            ])
            .with_documentation("A compact semantic status tag that renders ordinary children."),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn badge_numbers_reject_fractional_negative_and_overflow_values() {
        assert_eq!(parse_natural(7.0), Some(7));
        assert_eq!(parse_natural(1.5), None);
        assert_eq!(parse_natural(-1.0), None);
        assert_eq!(parse_natural(usize::MAX as f64), None);
        assert_eq!(parse_natural(f64::INFINITY), None);
    }

    #[derive(Debug, Default, Eq, PartialEq)]
    struct TagState {
        size: Size,
        variant: TagVariant,
        outline: bool,
        rounded_full: bool,
    }

    fn apply_tag_state(mut state: TagState, operation: &TagOp) -> TagState {
        match operation {
            TagOp::Size(size) => state.size = *size,
            TagOp::Variant(variant) => state.variant = *variant,
            TagOp::Outline => state.outline = true,
            TagOp::RoundedFull => state.rounded_full = true,
        }
        state
    }

    #[test]
    fn later_tag_variant_preserves_earlier_size_outline_and_rounding() {
        let operations = [
            TagOp::Size(Size::Large),
            TagOp::Outline,
            TagOp::RoundedFull,
            TagOp::Variant(TagVariant::Danger),
        ];
        let state = operations.iter().fold(TagState::default(), apply_tag_state);

        assert_eq!(
            state,
            TagState {
                size: Size::Large,
                variant: TagVariant::Danger,
                outline: true,
                rounded_full: true,
            }
        );

        let component = operations
            .into_iter()
            .fold(Tag::new(), |component, op| match op {
                TagOp::Size(size) => component.with_size(size),
                TagOp::Variant(variant) => component.with_variant(variant),
                TagOp::Outline => component.outline(),
                TagOp::RoundedFull => component.rounded_full(),
            });
        drop(component.into_any_element());
    }

    #[test]
    fn badge_operations_materialize_a_real_component_in_recorded_order() {
        let badge = [BadgeOp::Count(2), BadgeOp::Dot, BadgeOp::Count(9)]
            .into_iter()
            .fold(Badge::new(), |component, op| match op {
                BadgeOp::Count(value) => component.count(value),
                BadgeOp::Dot => component.dot(),
                _ => component,
            });
        drop(badge.into_any_element());
    }
}
