use std::path::{Component, Path};
use std::sync::Arc;

use gpui_component::{Icon, Sizable as _, Size, try_parse_color};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, Refineable as _, Styled as _},
};

#[derive(Clone)]
struct IconPayload(String);

#[derive(Clone, Copy)]
enum IconOp {
    Size(Size),
    Color(gpui::Hsla),
    Rotate(f32),
}

fn size(value: &str) -> Result<IconOp, String> {
    match value {
        "xsmall" => Ok(IconOp::Size(Size::XSmall)),
        "small" => Ok(IconOp::Size(Size::Small)),
        "medium" => Ok(IconOp::Size(Size::Medium)),
        "large" => Ok(IconOp::Size(Size::Large)),
        _ => Err(format!("unsupported Icon size `{value}`")),
    }
}

fn rotation(value: f64) -> Result<IconOp, String> {
    if value.is_finite() && value >= f32::MIN as f64 && value <= f32::MAX as f64 {
        Ok(IconOp::Rotate(value as f32))
    } else {
        Err("Icon.rotate expects finite radians representable as f32".into())
    }
}

struct IconMaterializer;

fn icon_path(path: &str) -> Result<String, String> {
    if path.trim().is_empty() {
        return Err("Icon path must not be empty".into());
    }
    if Path::new(path).components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err("Icon path must stay inside the application asset root".into());
    }
    Ok(path.to_owned())
}

impl ComponentMaterializer for IconMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let path = request
            .payload()
            .downcast_ref::<IconPayload>()
            .ok_or_else(|| anyhow::anyhow!("Icon received an incompatible payload"))?;
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<IconOp>().copied())
            .collect::<Vec<_>>();
        let mut icon = Icon::empty().path(path.0.clone());
        for operation in operations {
            icon = match operation {
                IconOp::Size(value) => icon.with_size(value),
                IconOp::Color(value) => icon.text_color(value),
                IconOp::Rotate(value) => icon.rotate(gpui::radians(value)),
            };
        }
        icon.style().refine(&request.take_style());
        anyhow::ensure!(
            request.take_children()?.is_empty(),
            "Icon does not accept children"
        );
        Ok(icon.into_any_element())
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor::new("Icon", Arc::new(IconMaterializer))
.with_constructors(vec![ConstructorDescriptor::new(
            "Icon",
            vec![ArgumentDescriptor::new("path", ArgumentSchema::String)],
            |arguments| match arguments {
                [ComponentArgument::String(path)] => icon_path(path)
                    .map(IconPayload)
                    .map(ComponentPayload::new),
                _ => Err("Icon expects one asset path string".into()),
            },
        )])
.with_methods(vec![
            MethodDescriptor::new(
                "size",
                vec![ArgumentDescriptor::new(
                    "size",
                    ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
                )],
                |arguments| match arguments {
                    [ComponentArgument::Enum(value)] => size(value).map(ComponentPayload::new),
                    _ => Err("Icon.size expects a semantic size literal".into()),
                },
            )
            .with_documentation("Sets the semantic icon size."),
            MethodDescriptor::new(
                "color",
                vec![ArgumentDescriptor::new("color", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(value)] => try_parse_color(value)
                        .map(IconOp::Color)
                        .map(ComponentPayload::new)
                        .map_err(|error| format!("invalid Icon color: {error}")),
                    _ => Err("Icon.color expects one color string".into()),
                },
            )
            .with_documentation("Sets the icon color from a supported color token."),
            MethodDescriptor::new(
                "rotate",
                vec![ArgumentDescriptor::new("radians", ArgumentSchema::Number)],
                |arguments| match arguments {
                    [ComponentArgument::Number(value)] => {
                        rotation(*value).map(ComponentPayload::new)
                    }
                    _ => Err("Icon.rotate expects one number of radians".into()),
                },
            )
            .with_documentation("Rotates the icon by a finite number of radians."),
        ])
.with_documentation(
            "An SVG icon loaded from a relative path beneath the application's asset root. Absolute paths and parent traversal are rejected.",
        ))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_numeric_and_size_arguments_are_closed() {
        assert!(matches!(size("small"), Ok(IconOp::Size(Size::Small))));
        assert!(size("tiny").is_err());
        assert!(rotation(0.5).is_ok());
        assert!(rotation(f64::NAN).is_err());
        assert!(rotation(f64::INFINITY).is_err());
    }

    #[test]
    fn icon_paths_are_relative_to_the_application_asset_root() {
        assert!(icon_path("icons/check.svg").is_ok());
        assert!(icon_path("/tmp/check.svg").is_err());
        assert!(icon_path("../check.svg").is_err());
        assert!(icon_path("icons/../../check.svg").is_err());
    }
}
