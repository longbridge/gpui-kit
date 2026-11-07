use std::{
    path::{Component, Path},
    sync::Arc,
};

use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, RegistryError, anyhow,
    gpui::{self, IntoElement as _, Refineable as _, Styled as _},
};

#[derive(Clone)]
struct Source(String);

fn asset_path(path: &str) -> Result<String, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Image path must not be empty".into());
    }
    if path.contains(':')
        || path.contains('\\')
        || path.starts_with("data:")
        || path.starts_with("file:")
        || Path::new(path).components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(
            "Image path must stay inside the application asset root; URLs are not accepted".into(),
        );
    }
    Ok(path.to_owned())
}

struct Materializer;
impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let source = request
            .payload()
            .downcast_ref::<Source>()
            .ok_or_else(|| anyhow::anyhow!("Image received an incompatible payload"))?;
        anyhow::ensure!(
            request.children_len() == 0,
            "Image does not accept children"
        );
        let mut image = gpui::img(gpui::SharedString::from(source.0.clone()));
        image.style().refine(&request.take_style());
        Ok(image.into_any_element())
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor::new("Image", Arc::new(Materializer))
.with_constructors(vec![ConstructorDescriptor::new(
            "Image",
            vec![ArgumentDescriptor::new("path", ArgumentSchema::String)],
            |args| match args {
                [ComponentArgument::String(path)] => asset_path(path).map(Source).map(ComponentPayload::new),
                _ => Err("Image expects one application-relative asset path".into()),
            },
        )])
.with_methods(vec![])
.with_documentation(
            "A local image loaded only from a relative path beneath the application asset root. URLs, absolute paths, traversal, and children are rejected; shell style is honored.",
        ))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_sources_are_confined_to_the_asset_root() {
        assert_eq!(asset_path("assets/pixel.svg").unwrap(), "assets/pixel.svg");
        for denied in [
            "",
            "/tmp/pixel.png",
            "../pixel.png",
            "a/../../pixel.png",
            "https://example.com/a.png",
            "https:example.com/a.png",
            "data:image/png;base64,x",
            "file:/tmp/a.png",
            r"..\pixel.png",
        ] {
            assert!(asset_path(denied).is_err(), "accepted {denied}");
        }
    }
}
