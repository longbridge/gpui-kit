use std::sync::Arc;

use gpui_component::{
    button::{Button, ButtonVariants as _},
    popover::Popover,
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallbackArgument,
    ComponentDescriptor, ComponentMaterializer, ComponentPayload, ComponentRegistry,
    ConstructorDescriptor, MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, Anchor, IntoElement as _, ParentElement as _, div},
};

#[derive(Clone)]
struct PopoverPayload {
    id: String,
    label: String,
}

#[derive(Clone)]
enum PopoverOp {
    Anchor(Anchor),
    DefaultOpen(bool),
    Open(bool),
    Appearance(bool),
    OverlayClosable(bool),
    OnOpenChange(ComponentArgument),
}

struct PopoverMaterializer;

impl ComponentMaterializer for PopoverMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<PopoverPayload>()
            .ok_or_else(|| anyhow::anyhow!("Popover received an incompatible payload"))?
            .clone();
        let content = request
            .take_slot_factory("content")
            .ok_or_else(|| anyhow::anyhow!("Popover requires content(element)"))?;

        let mut popover = Popover::new(payload.id.clone()).trigger(
            Button::new(format!("popover-trigger:{}", payload.id))
                .ghost()
                .label(payload.label),
        );
        for operation in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<PopoverOp>().cloned())
        {
            popover = match operation {
                PopoverOp::Anchor(anchor) => popover.anchor(anchor),
                PopoverOp::DefaultOpen(open) => popover.default_open(open),
                PopoverOp::Open(open) => popover.open(open),
                PopoverOp::Appearance(appearance) => popover.appearance(appearance),
                PopoverOp::OverlayClosable(closable) => popover.overlay_closable(closable),
                PopoverOp::OnOpenChange(argument) => {
                    let callback = request.resolve_callback(&argument)?;
                    popover.on_open_change(move |open, window, cx| {
                        callback.invoke_and_report_with(
                            "Popover.on_open_change callback failed",
                            &[ComponentCallbackArgument::Boolean(*open)],
                            window,
                            cx,
                        );
                    })
                }
            };
        }
        popover = popover.content(move |_, window, cx| match content.build(window, cx) {
            Ok(element) => element,
            Err(error) => div()
                .child(format!("Failed to render Popover content: {error:#}"))
                .into_any_element(),
        });
        request.finish(popover)
    }
}

fn boolean_op(
    component_method: &'static str,
    arguments: &[ComponentArgument],
    wrap: impl FnOnce(bool) -> PopoverOp,
) -> Result<ComponentPayload, String> {
    match arguments {
        [ComponentArgument::Boolean(value)] => Ok(ComponentPayload::new(wrap(*value))),
        _ => Err(format!(
            "Popover.{component_method}(value) expects a boolean"
        )),
    }
}

fn anchor_op(arguments: &[ComponentArgument]) -> Result<ComponentPayload, String> {
    let [ComponentArgument::Enum(anchor)] = arguments else {
        return Err("Popover.card_anchor(anchor) expects an anchor literal".into());
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
        _ => return Err(format!("unsupported Popover anchor `{anchor}`")),
    };
    Ok(ComponentPayload::new(PopoverOp::Anchor(anchor)))
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    let boolean = |name, wrap: fn(bool) -> PopoverOp| {
        MethodDescriptor::new(
            name,
            vec![ArgumentDescriptor::new("value", ArgumentSchema::Boolean)],
            move |arguments| boolean_op(name, arguments, wrap),
        )
    };
    registry.register(
        ComponentDescriptor::new("Popover", Arc::new(PopoverMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "Popover",
                vec![
                    ArgumentDescriptor::new("id", ArgumentSchema::String),
                    ArgumentDescriptor::new("label", ArgumentSchema::String),
                ],
                |arguments| match arguments {
                    [
                        ComponentArgument::String(id),
                        ComponentArgument::String(label),
                    ] if !id.trim().is_empty() && !label.trim().is_empty() => {
                        Ok(ComponentPayload::new(PopoverPayload {
                            id: id.clone(),
                            label: label.clone(),
                        }))
                    }
                    [ComponentArgument::String(_), ComponentArgument::String(_)] => {
                        Err("Popover id and label must not be empty".into())
                    }
                    _ => Err("Popover(id, label) expects two strings".into()),
                },
            )])
            .with_methods(vec![
                // Not `anchor`: the runtime's element prototype defines that
                // name itself and would shadow this method, which is what left
                // it unreachable by any value. `HoverCard` already carries the
                // same method under this name.
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
                    anchor_op,
                )
                .with_documentation("Positions the popover relative to its trigger."),
                boolean("default_open", PopoverOp::DefaultOpen)
                    .with_documentation("Sets the initial uncontrolled open state."),
                boolean("open", PopoverOp::Open)
                    .with_documentation("Controls whether the popover is open."),
                boolean("appearance", PopoverOp::Appearance)
                    .with_documentation("Controls the native popover surface styling."),
                boolean("overlay_closable", PopoverOp::OverlayClosable)
                    .with_documentation("Controls whether pressing outside dismisses the popover."),
                MethodDescriptor::new(
                    "on_open_change",
                    vec![ArgumentDescriptor::new(
                        "callback",
                        ArgumentSchema::Callback("(open: boolean, cx: Context) => void"),
                    )],
                    |arguments| match arguments {
                        [argument @ ComponentArgument::Callback(_)] => Ok(ComponentPayload::new(
                            PopoverOp::OnOpenChange(argument.clone()),
                        )),
                        _ => Err("Popover.on_open_change(callback) expects a callback".into()),
                    },
                )
                .with_documentation("Runs when pointer interaction changes the open state."),
            ])
            .with_documentation("A button-triggered popover with lazy content(element)."),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_schema_includes_the_script_context() {
        let mut registry = ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        let descriptor = frozen.descriptors().next().unwrap();
        assert_eq!(
            descriptor.methods()[5].arguments()[0].schema(),
            &ArgumentSchema::Callback("(open: boolean, cx: Context) => void")
        );
    }
}
