use std::sync::Arc;

use gpui_component::{
    Disableable as _, Selectable as _, Sizable as _, Size,
    button::{Button, ButtonVariants as _, DropdownButton},
    menu::PopupMenuItem,
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, Anchor, IntoElement as _, Refineable as _, Styled as _},
};

#[derive(Clone)]
struct DropdownPayload {
    id: String,
    label: String,
}

#[derive(Clone)]
enum DropdownOp {
    Outline,
    Size(Size),
    Variant(Variant),
    Anchor(Anchor),
    Item {
        label: String,
        callback: ComponentArgument,
    },
}

#[derive(Clone, Copy)]
enum Variant {
    Primary,
    Secondary,
    Danger,
    Ghost,
}

struct DropdownMaterializer;

#[derive(Clone, Copy)]
struct CommonBehavior;

fn common_boolean(name: &'static str) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Boolean)],
        move |arguments| match arguments {
            [ComponentArgument::Boolean(_)] => Ok(ComponentPayload::new(CommonBehavior)),
            _ => Err(format!("DropdownButton.{name}({name}) expects a boolean")),
        },
    )
    .with_documentation("Records shell-owned common control behavior.")
}

fn common_on_click() -> MethodDescriptor {
    MethodDescriptor::new(
        "on_click",
        vec![ArgumentDescriptor::new(
            "callback",
            ArgumentSchema::Callback("(event: ClickEvent, cx: Context) => void"),
        )],
        |arguments| match arguments {
            [ComponentArgument::Callback(_)] => Ok(ComponentPayload::new(CommonBehavior)),
            _ => Err("DropdownButton.on_click(callback) expects one callback".into()),
        },
    )
    .with_documentation("Invokes the callback when the labeled action half is activated.")
}

impl ComponentMaterializer for DropdownMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<DropdownPayload>()
            .ok_or_else(|| anyhow::anyhow!("DropdownButton received an incompatible payload"))?
            .clone();
        anyhow::ensure!(
            request.children_len() == 0,
            "DropdownButton does not accept children"
        );

        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<DropdownOp>().cloned())
            .collect::<Vec<_>>();
        let mut action = Button::new("action").label(payload.label);
        if let Some(callback) = request.on_click() {
            action = action.on_click(move |event, window, cx| callback.invoke(event, window, cx));
        }
        let mut dropdown = DropdownButton::new(payload.id)
            .button(action)
            .disabled(request.disabled())
            .selected(request.selected());
        let mut anchor = Anchor::TopRight;
        let mut items = Vec::new();
        for operation in operations {
            dropdown = match operation {
                DropdownOp::Outline => dropdown.outline(),
                DropdownOp::Size(value) => dropdown.with_size(value),
                DropdownOp::Variant(Variant::Primary) => dropdown.primary(),
                DropdownOp::Variant(Variant::Secondary) => dropdown.secondary(),
                DropdownOp::Variant(Variant::Danger) => dropdown.danger(),
                DropdownOp::Variant(Variant::Ghost) => dropdown.ghost(),
                DropdownOp::Anchor(value) => {
                    anchor = value;
                    dropdown
                }
                DropdownOp::Item { label, callback } => {
                    items.push((label, request.resolve_callback(&callback)?));
                    dropdown
                }
            };
        }
        if !items.is_empty() {
            dropdown = dropdown.dropdown_menu_with_anchor(anchor, move |mut menu, _, _| {
                for (label, callback) in &items {
                    let callback = callback.clone();
                    menu = menu.item(PopupMenuItem::new(label.clone()).on_click(
                        move |_, window, cx| {
                            callback.invoke_and_report_with(
                                "DropdownButton.menu_item callback failed",
                                &[],
                                window,
                                cx,
                            );
                        },
                    ));
                }
                menu
            });
        }
        dropdown.style().refine(&request.take_style());
        Ok(dropdown.into_any_element())
    }
}

fn string_callback_item(arguments: &[ComponentArgument]) -> Result<ComponentPayload, String> {
    match arguments {
        [
            ComponentArgument::String(label),
            callback @ ComponentArgument::Callback(_),
        ] => Ok(ComponentPayload::new(DropdownOp::Item {
            label: label.clone(),
            callback: callback.clone(),
        })),
        _ => Err("DropdownButton.menu_item(label, callback) expects a string and callback".into()),
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor::new("DropdownButton", Arc::new(DropdownMaterializer))
.with_constructors(vec![ConstructorDescriptor::new(
            "DropdownButton",
            vec![
                ArgumentDescriptor::new("id", ArgumentSchema::String),
                ArgumentDescriptor::new("label", ArgumentSchema::String),
            ],
            |arguments| match arguments {
                [ComponentArgument::String(id), ComponentArgument::String(label)]
                    if !id.trim().is_empty() => Ok(ComponentPayload::new(DropdownPayload {
                        id: id.clone(),
                        label: label.clone(),
                    })),
                [ComponentArgument::String(_), ComponentArgument::String(_)] => {
                    Err("DropdownButton id must not be empty".into())
                }
                _ => Err("DropdownButton(id, label) expects two strings".into()),
            },
        )])
.with_methods(vec![
            MethodDescriptor::new("outline", vec![], |_| {
                Ok(ComponentPayload::new(DropdownOp::Outline))
            })
            .with_documentation("Uses the outlined button treatment."),
            common_boolean("disabled"),
            common_boolean("selected"),
            common_on_click(),
            MethodDescriptor::new(
                "size",
                vec![ArgumentDescriptor::new(
                    "size",
                    ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
                )],
                |arguments| match arguments {
                    [ComponentArgument::Enum(value)] => match value.as_str() {
                        "xsmall" => Ok(ComponentPayload::new(DropdownOp::Size(Size::XSmall))),
                        "small" => Ok(ComponentPayload::new(DropdownOp::Size(Size::Small))),
                        "medium" => Ok(ComponentPayload::new(DropdownOp::Size(Size::Medium))),
                        "large" => Ok(ComponentPayload::new(DropdownOp::Size(Size::Large))),
                        _ => Err(format!("unsupported DropdownButton size `{value}`")),
                    },
                    _ => Err("DropdownButton.size(size) expects a size literal".into()),
                },
            )
            .with_documentation("Sets the size of both halves."),
            MethodDescriptor::new(
                "variant",
                vec![ArgumentDescriptor::new(
                    "variant",
                    ArgumentSchema::Enum(&["primary", "secondary", "danger", "ghost"]),
                )],
                |arguments| match arguments {
                    [ComponentArgument::Enum(value)] => match value.as_str() {
                        "primary" => Ok(ComponentPayload::new(DropdownOp::Variant(Variant::Primary))),
                        "secondary" => Ok(ComponentPayload::new(DropdownOp::Variant(Variant::Secondary))),
                        "danger" => Ok(ComponentPayload::new(DropdownOp::Variant(Variant::Danger))),
                        "ghost" => Ok(ComponentPayload::new(DropdownOp::Variant(Variant::Ghost))),
                        _ => Err(format!("unsupported DropdownButton variant `{value}`")),
                    },
                    _ => Err("DropdownButton.variant(variant) expects a variant literal".into()),
                },
            )
            .with_documentation("Sets the semantic variant of both halves."),
            MethodDescriptor::new(
                "menu_anchor",
                vec![ArgumentDescriptor::new(
                    "anchor",
                    ArgumentSchema::Enum(&["top_right", "bottom_right", "bottom_left", "top_left"]),
                )],
                |arguments| match arguments {
                    [ComponentArgument::Enum(value)] => match value.as_str() {
                        "top_right" => Ok(ComponentPayload::new(DropdownOp::Anchor(Anchor::TopRight))),
                        "bottom_right" => Ok(ComponentPayload::new(DropdownOp::Anchor(Anchor::BottomRight))),
                        "bottom_left" => Ok(ComponentPayload::new(DropdownOp::Anchor(Anchor::BottomLeft))),
                        "top_left" => Ok(ComponentPayload::new(DropdownOp::Anchor(Anchor::TopLeft))),
                        _ => Err(format!("unsupported DropdownButton anchor `{value}`")),
                    },
                    _ => Err("DropdownButton.menu_anchor(anchor) expects an anchor literal".into()),
                },
            )
            .with_documentation("Sets the popup menu anchor."),
            MethodDescriptor::new(
                "menu_item",
                vec![
                    ArgumentDescriptor::new("label", ArgumentSchema::String),
                    ArgumentDescriptor::new(
                        "callback",
                        ArgumentSchema::Callback("(cx: Context) => void"),
                    ),
                ],
                string_callback_item,
            )
            .with_documentation("Appends a clickable popup-menu item in call order."),
        ])
.with_documentation(
            "A real split DropdownButton with an adapter-owned labeled action half and optional callback menu items. It accepts no children.",
        ))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_item_keeps_label_and_closed_callback_handle() {
        let payload = string_callback_item(&[
            ComponentArgument::String("Open".into()),
            ComponentArgument::Callback(42),
        ])
        .unwrap();
        assert!(matches!(
            payload.downcast_ref::<DropdownOp>(),
            Some(DropdownOp::Item { label, callback: ComponentArgument::Callback(42) }) if label == "Open"
        ));
    }

    #[test]
    fn descriptor_uses_only_closed_schemas() {
        let mut registry = ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        let descriptor = frozen.descriptors().next().unwrap();
        assert_eq!(
            descriptor.constructors()[0].arguments()[0].schema(),
            &ArgumentSchema::String
        );
        assert_eq!(
            descriptor.methods().last().unwrap().arguments()[1].schema(),
            &ArgumentSchema::Callback("(cx: Context) => void")
        );
    }
}
