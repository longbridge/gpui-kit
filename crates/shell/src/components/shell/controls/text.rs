use std::sync::Arc;

use gpui_component::{kbd::Kbd, label::Label, link::Link, live_region::LiveRegion};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, Keystroke, ParentElement as _, Refineable as _, Styled as _},
};

#[derive(Clone)]
struct StringPayload(String);

#[derive(Clone)]
enum LabelOp {
    Secondary(String),
    Masked(bool),
    Highlights(String),
}

#[derive(Clone)]
enum LinkOp {
    Href(String),
}

#[derive(Clone)]
struct LiveRegionPayload {
    id: String,
    text: String,
}

#[derive(Clone)]
enum LiveRegionOp {
    Assertive,
}

#[derive(Clone)]
enum KbdOp {
    Appearance(bool),
    Outline,
}

fn string_constructor(export: &'static str, argument: &'static str) -> ConstructorDescriptor {
    ConstructorDescriptor::new(
        export,
        vec![ArgumentDescriptor::new(argument, ArgumentSchema::String)],
        move |arguments| match arguments {
            [ComponentArgument::String(value)] if !value.is_empty() => {
                Ok(ComponentPayload::new(StringPayload(value.to_owned())))
            }
            [ComponentArgument::String(_)] => Err(format!("{export} {argument} must not be empty")),
            _ => Err(format!("{export} expects one string {argument}")),
        },
    )
}

fn label_string_method(
    name: &'static str,
    docs: &'static str,
    make: fn(String) -> LabelOp,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::String)],
        move |arguments| match arguments {
            [ComponentArgument::String(value)] => Ok(ComponentPayload::new(make(value.to_owned()))),
            _ => Err(format!("Label.{name} expects one string")),
        },
    )
    .with_documentation(docs)
}

struct LabelMaterializer;

impl ComponentMaterializer for LabelMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let text = request
            .payload()
            .downcast_ref::<StringPayload>()
            .ok_or_else(|| anyhow::anyhow!("Label received an incompatible payload"))?;
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<LabelOp>().cloned())
            .collect::<Vec<_>>();
        // Every label a script describes is built here, so each takes its
        // element's id; the call-site default would be the same for all.
        let mut component = Label::new(text.0.clone()).id(request.element_id().clone());
        for operation in operations {
            component = match operation {
                LabelOp::Secondary(value) => component.secondary(value),
                LabelOp::Masked(value) => component.masked(value),
                LabelOp::Highlights(value) => component.highlights(value),
            };
        }
        let mut wrapper = gpui::div().child(component);
        wrapper.style().refine(&request.take_style());
        wrapper.extend(request.take_children()?);
        Ok(wrapper.into_any_element())
    }
}

fn record_link_href(arguments: &[ComponentArgument]) -> Result<ComponentPayload, String> {
    match arguments {
        [ComponentArgument::String(value)] if gpui_shell::is_openable_url(value) => {
            Ok(ComponentPayload::new(LinkOp::Href(value.to_owned())))
        }
        [ComponentArgument::String(_)] => {
            Err("Link.href(url) expects an absolute HTTP(S) URL with a host".into())
        }
        _ => Err("Link.href expects one URL string".into()),
    }
}

struct LinkMaterializer;

impl ComponentMaterializer for LinkMaterializer {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = request
            .payload()
            .downcast_ref::<StringPayload>()
            .ok_or_else(|| anyhow::anyhow!("Link received an incompatible payload"))?;
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<LinkOp>().cloned())
            .collect::<Vec<_>>();
        let mut component = Link::new(id.0.clone()).disabled(request.disabled());
        for operation in operations {
            match operation {
                LinkOp::Href(value) => component = component.href(value),
            }
        }
        if let Some(callback) = request.on_click() {
            component =
                component.on_click(move |event, window, cx| callback.invoke(event, window, cx));
        }
        request.finish(component)
    }
}

struct KbdMaterializer;

impl ComponentMaterializer for KbdMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let stroke = request
            .payload()
            .downcast_ref::<StringPayload>()
            .ok_or_else(|| anyhow::anyhow!("Kbd received an incompatible payload"))?
            .0
            .clone();
        let stroke = request.with_window_app(|_, cx| {
            Ok(Keystroke::parse_with_os(&stroke, cx.platform_info().os)?)
        })?;
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<KbdOp>().cloned())
            .collect::<Vec<_>>();
        let mut component = Kbd::new(stroke);
        for operation in operations {
            component = match operation {
                KbdOp::Appearance(value) => component.appearance(value),
                KbdOp::Outline => component.outline(),
            };
        }
        let mut wrapper = gpui::div().child(component);
        wrapper.style().refine(&request.take_style());
        wrapper.extend(request.take_children()?);
        Ok(wrapper.into_any_element())
    }
}

struct LiveRegionMaterializer;

impl ComponentMaterializer for LiveRegionMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        anyhow::ensure!(
            request.children_len() == 0,
            "LiveRegion does not accept children; pass its text to the constructor"
        );
        let payload = request
            .payload()
            .downcast_ref::<LiveRegionPayload>()
            .ok_or_else(|| anyhow::anyhow!("LiveRegion received an incompatible payload"))?
            .clone();
        let assertive = request.methods().any(|method| {
            matches!(
                method.payload().downcast_ref::<LiveRegionOp>(),
                Some(LiveRegionOp::Assertive)
            )
        });
        let mut region = LiveRegion::new(payload.id, payload.text);
        if assertive {
            region = region.assertive();
        }
        region.style().refine(&request.take_style());
        Ok(region.into_any_element())
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("Label", Arc::new(LabelMaterializer))
            .with_constructors(vec![string_constructor("Label", "text")])
            .with_methods(vec![
                super::super::support::on_click_method("Label"),
                super::super::support::disabled_method("Label"),
                label_string_method(
                    "secondary",
                    "Adds muted secondary text.",
                    LabelOp::Secondary,
                ),
                MethodDescriptor::new(
                    "masked",
                    vec![ArgumentDescriptor::new("masked", ArgumentSchema::Boolean)],
                    |arguments| match arguments {
                        [ComponentArgument::Boolean(value)] => {
                            Ok(ComponentPayload::new(LabelOp::Masked(*value)))
                        }
                        _ => Err("Label.masked expects one boolean".into()),
                    },
                )
                .with_documentation("Controls whether the main text is masked."),
                label_string_method(
                    "highlights",
                    "Highlights matching text fragments.",
                    LabelOp::Highlights,
                ),
            ])
            .with_documentation(
                "A text label with optional secondary, masking, and highlight presentation.",
            ),
    )?;
    registry.register(ComponentDescriptor::new("Link", Arc::new(LinkMaterializer))
.with_constructors(vec![string_constructor("Link", "id")])
.with_methods(vec![
            super::super::support::on_click_method("Link"),
            super::super::support::disabled_method("Link"),
            MethodDescriptor::new(
            "href",
            vec![ArgumentDescriptor::new("href", ArgumentSchema::String)],
            record_link_href,
        )
        .with_documentation("Sets the absolute HTTP(S) URL opened when activated.")])
.with_documentation(
            "An external-resource link. Ordinary children, shell style, disabled state, and on_click are honored.",
        ))?;
    registry.register(
        ComponentDescriptor::new("Kbd", Arc::new(KbdMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "Kbd",
                vec![ArgumentDescriptor::new("keystroke", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(value)] => {
                        Keystroke::parse_with_os(value, gpui::OperatingSystem::Unknown)
                            .map(|_| ComponentPayload::new(StringPayload(value.to_owned())))
                            .map_err(|error| format!("invalid Kbd keystroke: {error}"))
                    }
                    _ => Err("Kbd expects one keystroke string".into()),
                },
            )])
            .with_methods(vec![
                MethodDescriptor::new(
                    "appearance",
                    vec![ArgumentDescriptor::new(
                        "appearance",
                        ArgumentSchema::Boolean,
                    )],
                    |arguments| match arguments {
                        [ComponentArgument::Boolean(value)] => {
                            Ok(ComponentPayload::new(KbdOp::Appearance(*value)))
                        }
                        _ => Err("Kbd.appearance expects one boolean".into()),
                    },
                )
                .with_documentation("Controls whether the keystroke uses keycap presentation."),
                MethodDescriptor::new("outline", Vec::new(), |_| {
                    Ok(ComponentPayload::new(KbdOp::Outline))
                })
                .with_documentation("Uses the outlined keycap presentation."),
            ])
            .with_documentation("A platform-formatted keyboard shortcut keycap."),
    )?;
    registry.register(
        ComponentDescriptor::new("LiveRegion", Arc::new(LiveRegionMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "LiveRegion",
                vec![
                    ArgumentDescriptor::new("id", ArgumentSchema::String),
                    ArgumentDescriptor::new("text", ArgumentSchema::String),
                ],
                |arguments| match arguments {
                    [ComponentArgument::String(id), _] if id.is_empty() => {
                        Err("LiveRegion id must not be empty".into())
                    }
                    [
                        ComponentArgument::String(id),
                        ComponentArgument::String(text),
                    ] => Ok(ComponentPayload::new(LiveRegionPayload {
                        id: id.to_owned(),
                        text: text.to_owned(),
                    })),
                    _ => Err("LiveRegion expects a string id and a string text".into()),
                },
            )])
            .with_methods(vec![
                MethodDescriptor::new("assertive", Vec::new(), |_| {
                    Ok(ComponentPayload::new(LiveRegionOp::Assertive))
                })
                .with_documentation("Interrupts current speech instead of waiting for it."),
            ])
            .with_documentation(
                "Text that screen readers announce whenever it changes. Keep the id stable.",
            ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn href(url: &str) -> Result<ComponentPayload, String> {
        record_link_href(&[ComponentArgument::String(url.to_owned())])
    }

    #[test]
    fn link_href_accepts_only_http_urls_with_a_host() {
        let payload = href("https://gpui-kit.com/docs").unwrap();
        assert!(matches!(
            payload.downcast_ref::<LinkOp>(),
            Some(LinkOp::Href(url)) if url == "https://gpui-kit.com/docs"
        ));

        for refused in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "smb://server/share",
            "docs/intro.html",
            "https://",
        ] {
            let error = href(refused)
                .err()
                .unwrap_or_else(|| panic!("accepted {refused}"));
            assert!(error.contains("absolute HTTP(S) URL"), "{refused}: {error}");
        }
    }
}
