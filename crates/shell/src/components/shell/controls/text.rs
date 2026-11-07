use std::sync::Arc;

use gpui_component::{kbd::Kbd, label::Label, link::Link};
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
        let mut component = Label::new(text.0.clone());
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
