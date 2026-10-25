use std::sync::Arc;

use gpui_component::mermaid::{Mermaid, MermaidState};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, StateDescriptor, anyhow,
    gpui::{
        self, AppContext as _, Entity, IntoElement as _, Refineable as _, SharedString,
        Styled as _, prelude::FluentBuilder as _,
    },
};

struct Materializer;

impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let argument = request
            .payload()
            .downcast_ref::<ComponentArgument>()
            .ok_or_else(|| anyhow::anyhow!("Mermaid received an incompatible payload"))?;
        let state = request.with_state::<Entity<MermaidState>, _>(argument, Clone::clone)?;
        anyhow::ensure!(
            request.children_len() == 0,
            "Mermaid does not accept children"
        );
        let label = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<SharedString>())
            .last()
            .cloned();
        Ok(Mermaid::new(&state)
            .when_some(label, |this, label| this.accessibility_label(label))
            .map(|mut this| {
                this.style().refine(&request.take_style());
                this
            })
            .into_any_element())
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register_state(
        StateDescriptor::new(
            "MermaidState",
            "MermaidState",
            vec![ArgumentDescriptor::new("source", ArgumentSchema::String)],
            |args, _, cx| match args {
                [ComponentArgument::String(source)] => {
                    Ok(Box::new(cx.new(|cx| MermaidState::new(source.clone(), cx))) as _)
                }
                _ => Err("MermaidState expects Mermaid source text".into()),
            },
        )
        .with_documentation("Retained native diagram state initialized from Mermaid source text."),
    )?;
    registry.register(
        ComponentDescriptor::new("Mermaid", Arc::new(Materializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "Mermaid",
                vec![ArgumentDescriptor::new(
                    "state",
                    ArgumentSchema::Entity("MermaidState"),
                )],
                |args| match args {
                    [argument @ ComponentArgument::Entity { .. }] => {
                        Ok(ComponentPayload::new(argument.clone()))
                    }
                    _ => Err("Mermaid expects one MermaidState entity".into()),
                },
            )])
            .with_methods(vec![
                MethodDescriptor::new(
                    "accessibility_label",
                    vec![ArgumentDescriptor::new("label", ArgumentSchema::String)],
                    |args| match args {
                        [ComponentArgument::String(label)] => {
                            Ok(ComponentPayload::new(SharedString::from(label.clone())))
                        }
                        _ => Err("Mermaid.accessibility_label expects text".into()),
                    },
                )
                .with_documentation("Sets the diagram description for assistive technology."),
            ])
            .with_documentation(
                "A readonly native Mermaid preview. Shell style is honored; children are rejected.",
            ),
    )?;
    Ok(())
}
