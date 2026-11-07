use std::sync::Arc;

use gpui_component::empty::{
    Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle,
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, Refineable as _, Styled as _, prelude::FluentBuilder as _},
};

use super::typed_compound::{TypedChildElement, finish_part, take_element};

#[derive(Clone, Copy)]
enum Part {
    Root,
    Header,
    Media,
    Title,
    Description,
    Content,
}

#[derive(Clone, Copy, PartialEq)]
enum Slot {
    Media,
    Title,
    Description,
}

#[derive(Clone)]
enum Op {
    Slot(Slot, ComponentArgument),
    Variant(EmptyMediaVariant),
}

struct Materializer;

// Resolve only the final value: overwritten slots must not be materialized.
fn slot<T: gpui::IntoElement + 'static>(
    request: &mut MaterializeRequest<'_>,
    operations: &[Op],
    slot: Slot,
    name: &str,
) -> anyhow::Result<Option<T>> {
    operations
        .iter()
        .rev()
        .find_map(|operation| match operation {
            Op::Slot(candidate, argument) if *candidate == slot => Some(argument),
            _ => None,
        })
        .map(|argument| take_element(&mut request.resolve_element(argument)?, name))
        .transpose()
}

// `header` and `content` are slots the prelude installs on every element, so
// they arrive through the common slot lane rather than as recorded methods.
// Resolve only the final value: overwritten slots must not be rendered.
fn common_slot<T: gpui::IntoElement + 'static>(
    request: &mut MaterializeRequest<'_>,
    slot: &str,
    name: &str,
) -> anyhow::Result<Option<T>> {
    // `take_slot` drains one value per call and reaches the deferred lane that
    // `take_slots` leaves behind, so draining to the end is what finds the last.
    let mut last = None;
    while let Some(element) = request.take_slot(slot)? {
        last = Some(element);
    }
    last.map(|mut element| take_element(&mut element, name))
        .transpose()
}

impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let part = *request
            .payload()
            .downcast_ref::<Part>()
            .ok_or_else(|| anyhow::anyhow!("Empty received an incompatible payload"))?;
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<Op>())
            .cloned()
            .collect::<Vec<_>>();

        match part {
            Part::Root => {
                let header = common_slot(&mut request, "header", "EmptyHeader")?;
                let content = common_slot(&mut request, "content", "EmptyContent")?;
                request.finish(
                    Empty::new()
                        .when_some(header, Empty::header)
                        .when_some(content, Empty::content),
                )
            }
            Part::Header => {
                anyhow::ensure!(
                    request.children_len() == 0,
                    "EmptyHeader does not accept children; use media, title, and description"
                );
                let media = slot(&mut request, &operations, Slot::Media, "EmptyMedia")?;
                let title = slot(&mut request, &operations, Slot::Title, "EmptyTitle")?;
                let description = slot(
                    &mut request,
                    &operations,
                    Slot::Description,
                    "EmptyDescription",
                )?;
                let mut header = EmptyHeader::new()
                    .when_some(media, EmptyHeader::media)
                    .when_some(title, EmptyHeader::title)
                    .when_some(description, EmptyHeader::description);
                header.style().refine(&request.take_style());
                Ok(TypedChildElement::new(header).into_any_element())
            }
            Part::Media => {
                let variant = operations
                    .iter()
                    .rev()
                    .find_map(|operation| match operation {
                        Op::Variant(value) => Some(*value),
                        _ => None,
                    });
                finish_part(
                    &mut request,
                    EmptyMedia::new().when_some(variant, EmptyMedia::with_variant),
                )
            }
            Part::Title => finish_part(&mut request, EmptyTitle::new()),
            Part::Description => finish_part(&mut request, EmptyDescription::new()),
            Part::Content => finish_part(&mut request, EmptyContent::new()),
        }
    }
}

fn slot_method(name: &'static str, slot: Slot, documentation: &'static str) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Element)],
        move |arguments| match arguments {
            [argument @ ComponentArgument::Element(_)] => {
                Ok(ComponentPayload::new(Op::Slot(slot, argument.clone())))
            }
            _ => Err(format!("{name} expects one registered Empty part")),
        },
    )
    .with_documentation(documentation)
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    for (name, part, documentation, methods) in [
        (
            "Empty",
            Part::Root,
            "A stateless empty state. Ordinary children follow the header and content slots.",
            vec![],
        ),
        (
            "EmptyHeader",
            Part::Header,
            "An independently styled header. Accepts only named media, title, and description slots, in that order.",
            vec![
                slot_method(
                    "media",
                    Slot::Media,
                    "Sets an EmptyMedia, replacing the previous media.",
                ),
                slot_method(
                    "title",
                    Slot::Title,
                    "Sets an EmptyTitle, replacing the previous title.",
                ),
                slot_method(
                    "description",
                    Slot::Description,
                    "Sets an EmptyDescription, replacing the previous description.",
                ),
            ],
        ),
        (
            "EmptyMedia",
            Part::Media,
            "A centered media part accepting arbitrary children such as an icon, avatar, or image.",
            vec![
                MethodDescriptor::new(
                    "variant",
                    vec![ArgumentDescriptor::new(
                        "variant",
                        ArgumentSchema::Enum(&["default", "icon"]),
                    )],
                    |arguments| match arguments {
                        [ComponentArgument::Enum(value)] => match value.as_str() {
                            "default" => Ok(ComponentPayload::new(Op::Variant(
                                EmptyMediaVariant::Default,
                            ))),
                            "icon" => {
                                Ok(ComponentPayload::new(Op::Variant(EmptyMediaVariant::Icon)))
                            }
                            _ => Err("EmptyMedia.variant expects default or icon".into()),
                        },
                        _ => Err("EmptyMedia.variant expects default or icon".into()),
                    },
                )
                .with_documentation("Sets the unframed default treatment or a muted icon frame."),
            ],
        ),
        (
            "EmptyTitle",
            Part::Title,
            "An independently styled title with arbitrary children.",
            vec![],
        ),
        (
            "EmptyDescription",
            Part::Description,
            "An independently styled, wrapping description with arbitrary children.",
            vec![],
        ),
        (
            "EmptyContent",
            Part::Content,
            "An independently styled content column for application-owned controls and actions.",
            vec![],
        ),
    ] {
        registry.register(
            ComponentDescriptor::new(name, Arc::new(Materializer))
                .with_constructors(vec![ConstructorDescriptor::new(name, vec![], move |_| {
                    Ok(ComponentPayload::new(part))
                })])
                .with_methods(methods)
                .with_documentation(documentation),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_publishes_all_parts_with_closed_documented_methods() {
        let mut registry = ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        let descriptors = frozen.descriptors().collect::<Vec<_>>();
        assert_eq!(
            descriptors
                .iter()
                .map(|part| part.name())
                .collect::<Vec<_>>(),
            [
                "Empty",
                "EmptyHeader",
                "EmptyMedia",
                "EmptyTitle",
                "EmptyDescription",
                "EmptyContent"
            ]
        );
        for (descriptor, methods) in descriptors.iter().zip([
            vec![],
            vec!["media", "title", "description"],
            vec!["variant"],
            vec![],
            vec![],
            vec![],
        ]) {
            assert!(descriptor.documentation().is_some());
            assert!(descriptor.constructors()[0].arguments().is_empty());
            assert_eq!(
                descriptor
                    .methods()
                    .iter()
                    .map(|method| method.name())
                    .collect::<Vec<_>>(),
                methods
            );
            for method in descriptor.methods() {
                assert!(method.documentation().is_some());
                assert_eq!(
                    method.arguments()[0].schema(),
                    &if method.name() == "variant" {
                        ArgumentSchema::Enum(&["default", "icon"])
                    } else {
                        ArgumentSchema::Element
                    }
                );
            }
        }
    }
}
