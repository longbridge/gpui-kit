use std::sync::Arc;

use gpui_component::{
    button::Button,
    menu::{DropdownMenu as _, PopupMenuItem},
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, Refineable as _, Styled as _},
};

#[derive(Clone)]
struct DropdownMenuPayload {
    id: String,
    label: String,
}

#[derive(Clone)]
struct MenuItemOp {
    label: String,
    callback: ComponentArgument,
}

struct DropdownMenuMaterializer;

fn require_item_only_children(count: usize) -> anyhow::Result<()> {
    anyhow::ensure!(
        count == 0,
        "DropdownMenu accepts item(label, callback) methods only; ordinary and typed children are unsupported"
    );
    Ok(())
}

impl ComponentMaterializer for DropdownMenuMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        require_item_only_children(request.children_len())?;
        let payload = request
            .payload()
            .downcast_ref::<DropdownMenuPayload>()
            .ok_or_else(|| anyhow::anyhow!("DropdownMenu received an incompatible payload"))?
            .clone();
        let items = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<MenuItemOp>().cloned())
            .map(|item| Ok((item.label, request.resolve_callback(&item.callback)?)))
            .collect::<anyhow::Result<Vec<_>>>()?;

        let mut button = Button::new(payload.id).label(payload.label);
        button.style().refine(&request.take_style());
        let menu = button.dropdown_menu(move |menu, _, _| {
            items.iter().fold(menu, |menu, (label, callback)| {
                let callback = callback.clone();
                menu.item(
                    PopupMenuItem::new(label.clone()).on_click(move |_, window, cx| {
                        callback.invoke_and_report_with(
                            "DropdownMenu.item callback failed",
                            &[],
                            window,
                            cx,
                        );
                    }),
                )
            })
        });
        Ok(menu.into_any_element())
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("DropdownMenu", Arc::new(DropdownMenuMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "DropdownMenu",
                vec![
                    ArgumentDescriptor::new("id", ArgumentSchema::String),
                    ArgumentDescriptor::new("label", ArgumentSchema::String),
                ],
                |arguments| match arguments {
                    [
                        ComponentArgument::String(id),
                        ComponentArgument::String(label),
                    ] if !id.trim().is_empty() && !label.trim().is_empty() => {
                        Ok(ComponentPayload::new(DropdownMenuPayload {
                            id: id.clone(),
                            label: label.clone(),
                        }))
                    }
                    [ComponentArgument::String(_), ComponentArgument::String(_)] => {
                        Err("DropdownMenu id and label must not be empty".into())
                    }
                    _ => Err("DropdownMenu(id, label) expects two strings".into()),
                },
            )])
            .with_methods(vec![
                MethodDescriptor::new(
                    "item",
                    vec![
                        ArgumentDescriptor::new("label", ArgumentSchema::String),
                        ArgumentDescriptor::new(
                            "callback",
                            ArgumentSchema::Callback("(cx: Context) => void"),
                        ),
                    ],
                    |arguments| match arguments {
                        [
                            ComponentArgument::String(label),
                            callback @ ComponentArgument::Callback(_),
                        ] if !label.trim().is_empty() => Ok(ComponentPayload::new(MenuItemOp {
                            label: label.clone(),
                            callback: callback.clone(),
                        })),
                        [ComponentArgument::String(_), ComponentArgument::Callback(_)] => {
                            Err("DropdownMenu.item label must not be empty".into())
                        }
                        _ => Err(
                            "DropdownMenu.item(label, callback) expects a string and callback"
                                .into(),
                        ),
                    },
                )
                .with_documentation("Appends a command item in script order."),
            ])
            .with_documentation(
                "A button-triggered native popup menu containing closed command items.",
            ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_callback_schema_includes_the_script_context() {
        let mut registry = ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        let descriptor = frozen.descriptors().next().unwrap();
        assert_eq!(
            descriptor.methods()[0].arguments()[1].schema(),
            &ArgumentSchema::Callback("(cx: Context) => void")
        );
    }

    #[test]
    fn item_only_contract_rejects_every_child_lane() {
        assert!(require_item_only_children(0).is_ok());
        assert_eq!(
            require_item_only_children(1).unwrap_err().to_string(),
            "DropdownMenu accepts item(label, callback) methods only; ordinary and typed children are unsupported"
        );
    }
}
