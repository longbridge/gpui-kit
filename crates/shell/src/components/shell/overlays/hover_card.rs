use std::{sync::Arc, time::Duration};

use gpui_component::hover_card::HoverCard;
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallbackArgument,
    ComponentDescriptor, ComponentMaterializer, ComponentPayload, ComponentRegistry,
    ConstructorDescriptor, MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, Anchor, IntoElement as _, ParentElement as _, div},
};

#[derive(Clone)]
struct HoverCardPayload {
    id: String,
}

#[derive(Clone, Debug, PartialEq)]
enum HoverCardOp {
    Trigger(ComponentArgument),
    Anchor(Anchor),
    OpenDelay(Duration),
    CloseDelay(Duration),
    Appearance(bool),
    OnOpenChange(ComponentArgument),
}

struct HoverCardMaterializer;

fn payload_id(payload: &ComponentPayload) -> anyhow::Result<String> {
    Ok(payload
        .downcast_ref::<HoverCardPayload>()
        .ok_or_else(|| anyhow::anyhow!("HoverCard received an incompatible payload"))?
        .id
        .clone())
}

fn trigger_argument(operations: &[HoverCardOp]) -> anyhow::Result<&ComponentArgument> {
    operations
        .iter()
        .find_map(|operation| match operation {
            HoverCardOp::Trigger(argument) => Some(argument),
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("HoverCard requires trigger_element(element)"))
}

fn non_empty_id(id: &str) -> Result<String, String> {
    if id.trim().is_empty() {
        Err("HoverCard id must not be empty".into())
    } else {
        Ok(id.to_owned())
    }
}

impl ComponentMaterializer for HoverCardMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = payload_id(request.payload())?;
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<HoverCardOp>().cloned())
            .collect::<Vec<_>>();
        let trigger = request.resolve_element(trigger_argument(&operations)?)?;
        let content = request
            .take_slot_factory("content")
            .ok_or_else(|| anyhow::anyhow!("HoverCard requires content(element)"))?;

        let mut card = HoverCard::new(id)
            .trigger(trigger)
            .content(move |_, window, cx| match content.build(window, cx) {
                Ok(element) => element,
                Err(error) => div()
                    .child(format!("Failed to render HoverCard content: {error:#}"))
                    .into_any_element(),
            });
        for operation in &operations {
            card = match operation {
                HoverCardOp::Trigger(_) => card,
                HoverCardOp::Anchor(anchor) => card.anchor(*anchor),
                HoverCardOp::OpenDelay(delay) => card.open_delay(*delay),
                HoverCardOp::CloseDelay(delay) => card.close_delay(*delay),
                HoverCardOp::Appearance(appearance) => card.appearance(*appearance),
                HoverCardOp::OnOpenChange(argument) => {
                    let callback = request.resolve_callback(argument)?;
                    card.on_open_change(move |open, window, cx| {
                        callback.invoke_and_report_with(
                            "HoverCard.on_open_change callback failed",
                            &[ComponentCallbackArgument::Boolean(*open)],
                            window,
                            cx,
                        );
                    })
                }
            };
        }
        request.finish(card)
    }
}

fn duration_operation(
    method: &'static str,
    arguments: &[ComponentArgument],
    wrap: impl FnOnce(Duration) -> HoverCardOp,
) -> Result<ComponentPayload, String> {
    let [ComponentArgument::Number(milliseconds)] = arguments else {
        return Err(format!("HoverCard.{method}(milliseconds) expects a number"));
    };
    if !milliseconds.is_finite() || !(0.0..=60_000.0).contains(milliseconds) {
        return Err(format!(
            "HoverCard.{method}(milliseconds) expects a finite value from 0 through 60000"
        ));
    }
    Ok(ComponentPayload::new(wrap(Duration::from_secs_f64(
        milliseconds / 1000.0,
    ))))
}

fn anchor_operation(arguments: &[ComponentArgument]) -> Result<ComponentPayload, String> {
    let [ComponentArgument::Enum(anchor)] = arguments else {
        return Err("HoverCard.card_anchor(anchor) expects an anchor literal".into());
    };
    let anchor = match anchor.as_str() {
        "top_left" => Anchor::TopLeft,
        "top_center" => Anchor::TopCenter,
        "top_right" => Anchor::TopRight,
        "bottom_left" => Anchor::BottomLeft,
        "bottom_center" => Anchor::BottomCenter,
        "bottom_right" => Anchor::BottomRight,
        "left_center" => Anchor::LeftCenter,
        "right_center" => Anchor::RightCenter,
        _ => return Err(format!("unsupported HoverCard anchor `{anchor}`")),
    };
    Ok(ComponentPayload::new(HoverCardOp::Anchor(anchor)))
}

fn open_change_operation(arguments: &[ComponentArgument]) -> Result<ComponentPayload, String> {
    match arguments {
        [argument @ ComponentArgument::Callback(_)] => Ok(ComponentPayload::new(
            HoverCardOp::OnOpenChange(argument.clone()),
        )),
        _ => Err("HoverCard.on_open_change(callback) expects a callback".into()),
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("HoverCard", Arc::new(HoverCardMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "HoverCard",
                vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(id)] => {
                        Ok(ComponentPayload::new(HoverCardPayload {
                            id: non_empty_id(id)?,
                        }))
                    }
                    _ => Err("HoverCard(id) expects a string".into()),
                },
            )])
            .with_methods(vec![
                MethodDescriptor::new(
                    "trigger_element",
                    vec![ArgumentDescriptor::new("element", ArgumentSchema::Element)],
                    |arguments| match arguments {
                        [argument @ ComponentArgument::Element(_)] => Ok(ComponentPayload::new(
                            HoverCardOp::Trigger(argument.clone()),
                        )),
                        _ => Err("HoverCard.trigger_element(element) expects an element".into()),
                    },
                )
                .with_documentation("Sets the element that owns the hover interaction."),
                MethodDescriptor::new(
                    "card_anchor",
                    vec![ArgumentDescriptor::new(
                        "anchor",
                        ArgumentSchema::Enum(&[
                            "top_left",
                            "top_center",
                            "top_right",
                            "bottom_left",
                            "bottom_center",
                            "bottom_right",
                            "left_center",
                            "right_center",
                        ]),
                    )],
                    anchor_operation,
                )
                .with_documentation("Positions the card relative to its trigger."),
                MethodDescriptor::new(
                    "open_delay",
                    vec![ArgumentDescriptor::new(
                        "milliseconds",
                        ArgumentSchema::Number,
                    )],
                    |arguments| duration_operation("open_delay", arguments, HoverCardOp::OpenDelay),
                )
                .with_documentation("Sets the hover-open delay in milliseconds (0–60000)."),
                MethodDescriptor::new(
                    "close_delay",
                    vec![ArgumentDescriptor::new(
                        "milliseconds",
                        ArgumentSchema::Number,
                    )],
                    |arguments| {
                        duration_operation("close_delay", arguments, HoverCardOp::CloseDelay)
                    },
                )
                .with_documentation("Sets the hover-close delay in milliseconds (0–60000)."),
                MethodDescriptor::new(
                    "appearance",
                    vec![ArgumentDescriptor::new(
                        "appearance",
                        ArgumentSchema::Boolean,
                    )],
                    |arguments| match arguments {
                        [ComponentArgument::Boolean(appearance)] => {
                            Ok(ComponentPayload::new(HoverCardOp::Appearance(*appearance)))
                        }
                        _ => Err("HoverCard.appearance(appearance) expects a boolean".into()),
                    },
                )
                .with_documentation("Controls the component's popover surface styling."),
                MethodDescriptor::new(
                    "on_open_change",
                    vec![ArgumentDescriptor::new(
                        "callback",
                        ArgumentSchema::Callback("(open: boolean, cx: Context) => void"),
                    )],
                    open_change_operation,
                )
                .with_documentation("Runs when pointer interaction opens or closes the card."),
            ])
            .with_documentation(
                "A hover-triggered card. Supply trigger_element(element) and lazy content(element).",
            ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_schema_rejects_values_that_duration_cannot_honestly_represent() {
        for value in [f64::NAN, -1.0, 60_001.0] {
            assert!(
                duration_operation(
                    "open_delay",
                    &[ComponentArgument::Number(value)],
                    HoverCardOp::OpenDelay,
                )
                .is_err()
            );
        }
        let payload = duration_operation(
            "open_delay",
            &[ComponentArgument::Number(250.0)],
            HoverCardOp::OpenDelay,
        )
        .unwrap();
        assert_eq!(
            payload.downcast_ref::<HoverCardOp>(),
            Some(&HoverCardOp::OpenDelay(Duration::from_millis(250)))
        );
    }

    #[test]
    fn descriptor_uses_closed_anchor_and_callback_schemas() {
        let mut registry = ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        let descriptor = frozen.descriptors().next().unwrap();
        assert_eq!(descriptor.name(), "HoverCard");
        assert!(matches!(
            descriptor.methods()[1].arguments()[0].schema(),
            &ArgumentSchema::Enum(_)
        ));
        assert_eq!(
            descriptor.methods()[5].arguments()[0].schema(),
            &ArgumentSchema::Callback("(open: boolean, cx: Context) => void")
        );
    }

    #[test]
    fn whitespace_only_identity_is_rejected() {
        assert_eq!(
            non_empty_id(" \t\n"),
            Err("HoverCard id must not be empty".into())
        );
    }

    #[test]
    fn callback_operation_preserves_the_closed_callback_handle() {
        let descriptor_callback = ComponentArgument::Callback(42);
        let payload = open_change_operation(std::slice::from_ref(&descriptor_callback));
        assert_eq!(
            payload.unwrap().downcast_ref::<HoverCardOp>(),
            Some(&HoverCardOp::OnOpenChange(descriptor_callback))
        );
    }

    #[test]
    fn incompatible_payload_reports_the_materializer_contract() {
        assert_eq!(
            payload_id(&ComponentPayload::new(()))
                .unwrap_err()
                .to_string(),
            "HoverCard received an incompatible payload"
        );
    }

    #[test]
    fn materializer_requires_the_trigger_element_operation() {
        assert_eq!(
            trigger_argument(&[]).unwrap_err().to_string(),
            "HoverCard requires trigger_element(element)"
        );
        let operations = [HoverCardOp::Trigger(ComponentArgument::Element(7))];
        assert_eq!(
            trigger_argument(&operations).unwrap(),
            &ComponentArgument::Element(7)
        );
    }
}
