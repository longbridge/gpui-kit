use super::{Carrier, take};
use super::{reject_style, require_child};
use gpui_component::{Disableable as _, button::Button, native_menu::NativeMenu};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError,
    action::ShellAction,
    anyhow,
    gpui::{self, IntoElement as _, Refineable as _, Styled as _},
};
use std::sync::Arc;

#[derive(Clone)]
struct Item {
    label: String,
    action: String,
    disabled: bool,
    checked: bool,
}
#[derive(Clone, Copy)]
struct Separator;
#[derive(Clone)]
struct Trigger {
    id: String,
    label: String,
}
#[derive(Clone)]
enum ItemOp {
    Checked(bool),
    Action(String),
}
#[derive(Clone)]
struct ErrorCallback(ComponentArgument);
#[derive(Clone)]
enum Entry {
    Item(Item),
    Separator,
}

fn resolve_item<'a>(
    base: &Item,
    disabled: bool,
    ops: impl Iterator<Item = &'a ItemOp>,
) -> anyhow::Result<Item> {
    let mut item = base.clone();
    item.disabled = disabled;
    for op in ops {
        match op {
            ItemOp::Checked(value) => item.checked = *value,
            ItemOp::Action(value) => item.action = value.clone(),
        }
    }
    anyhow::ensure!(
        !item.disabled || !item.checked,
        "NativeMenuItem cannot be both disabled and checked because the native API has no combined constructor"
    );
    Ok(item)
}
fn last_reporter<'a>(ops: impl Iterator<Item = &'a ErrorCallback>) -> Option<&'a ErrorCallback> {
    ops.last()
}

struct ItemMaterializer;
impl ComponentMaterializer for ItemMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let base = request
            .payload()
            .downcast_ref::<Item>()
            .ok_or_else(|| anyhow::anyhow!("NativeMenuItem incompatible payload"))?
            .clone();
        let item = resolve_item(
            &base,
            request.disabled(),
            request
                .methods()
                .filter_map(|m| m.payload().downcast_ref::<ItemOp>()),
        )?;
        reject_style(request.take_style(), "NativeMenuItem")?;
        for child in request.take_typed_children()? {
            require_child("NativeMenuItem", child.component_name(), &[])?;
        }
        Ok(Carrier::new(Entry::Item(item)).into_any_element())
    }
}
struct SeparatorMaterializer;
impl ComponentMaterializer for SeparatorMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        reject_style(request.take_style(), "NativeMenuSeparator")?;
        for child in request.take_typed_children()? {
            require_child("NativeMenuSeparator", child.component_name(), &[])?;
        }
        Ok(Carrier::new(Entry::Separator).into_any_element())
    }
}
struct TriggerMaterializer;
impl ComponentMaterializer for TriggerMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let trigger = request
            .payload()
            .downcast_ref::<Trigger>()
            .ok_or_else(|| anyhow::anyhow!("NativeMenuTrigger incompatible payload"))?
            .clone();
        let reporter = last_reporter(
            request
                .methods()
                .filter_map(|m| m.payload().downcast_ref::<ErrorCallback>()),
        )
        .ok_or_else(|| anyhow::anyhow!("NativeMenuTrigger requires on_effect_error(callback)"))?;
        let effects = request.resolve_callback(&reporter.0)?.window_effects();
        let mut entries = Vec::new();
        for mut child in request.take_typed_children()? {
            require_child(
                "NativeMenuTrigger",
                child.component_name(),
                &["NativeMenuItem", "NativeMenuSeparator"],
            )?;
            let mut element = request.materialize_child(&mut child)?;
            entries.push(take::<Entry>(&mut element, "NativeMenu entry")?);
        }
        let key = format!("native-menu:{}", trigger.id);
        let mut button = Button::new(trigger.id)
            .label(trigger.label)
            .disabled(request.disabled())
            .on_click(move |event, window, cx| {
                let entries = entries.clone();
                let _ = effects.event(window, cx, |effects| {
                    effects
                        .run_once(key.clone(), |window, cx| {
                            let mut menu = NativeMenu::new();
                            for entry in entries {
                                menu = match entry {
                                    Entry::Item(item) if item.disabled => menu.menu_with_disabled(
                                        item.label,
                                        true,
                                        Box::new(ShellAction::new(item.action)),
                                    ),
                                    Entry::Item(item) if item.checked => menu.menu_with_check(
                                        item.label,
                                        true,
                                        Box::new(ShellAction::new(item.action)),
                                    ),
                                    Entry::Item(item) => menu
                                        .menu(item.label, Box::new(ShellAction::new(item.action))),
                                    Entry::Separator => menu.separator(),
                                };
                            }
                            #[cfg(test)]
                            test_probe::shown();
                            menu.show(event.position(), window, cx);
                            Ok(())
                        })
                        .map(|_| ())
                });
            });
        button.style().refine(&request.take_style());
        Ok(button.into_any_element())
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor::new("NativeMenuItem", Arc::new(ItemMaterializer))
.with_constructors(vec![ConstructorDescriptor::new(
            "NativeMenuItem",
            vec![ArgumentDescriptor::new("label", ArgumentSchema::String), ArgumentDescriptor::new("action", ArgumentSchema::String)],
            |args| match args {
                [ComponentArgument::String(label), ComponentArgument::String(action)] if !label.trim().is_empty() && !action.trim().is_empty() => Ok(ComponentPayload::new(Item { label: label.clone(), action: action.clone(), disabled: false, checked: false })),
                _ => Err("NativeMenuItem expects non-empty label and action id".into()),
            },
        )])
.with_methods(vec![
            MethodDescriptor::new("disabled", vec![ArgumentDescriptor::new("disabled", ArgumentSchema::Boolean)], |_| Ok(ComponentPayload::new(()))).with_documentation("Disables the native menu item."),
            MethodDescriptor::new("checked", vec![ArgumentDescriptor::new("checked", ArgumentSchema::Boolean)], |args| match args { [ComponentArgument::Boolean(value)] => Ok(ComponentPayload::new(ItemOp::Checked(*value))), _ => Err("NativeMenuItem.checked expects boolean".into()) }).with_documentation("Sets the native menu item checked state."),
            MethodDescriptor::new("action", vec![ArgumentDescriptor::new("action", ArgumentSchema::String)], |args| match args { [ComponentArgument::String(value)] if !value.trim().is_empty() => Ok(ComponentPayload::new(ItemOp::Action(value.clone()))), _ => Err("NativeMenuItem.action expects non-empty action id".into()) }).with_documentation("Dispatches the named shell action."),
        ])
.with_documentation("Typed native-menu item data with last-call-wins checked/action; disabled+checked is rejected because the native API cannot express it. ShellAction dispatches selection."))?;
    registry.register(
        ComponentDescriptor::new("NativeMenuSeparator", Arc::new(SeparatorMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "NativeMenuSeparator",
                vec![],
                |_| Ok(ComponentPayload::new(Separator)),
            )])
            .with_methods(vec![])
            .with_documentation("Typed native-menu separator data."),
    )?;
    registry.register(ComponentDescriptor::new("NativeMenuTrigger", Arc::new(TriggerMaterializer))
.with_constructors(vec![ConstructorDescriptor::new("NativeMenuTrigger", vec![ArgumentDescriptor::new("id", ArgumentSchema::String), ArgumentDescriptor::new("label", ArgumentSchema::String)], |args| match args { [ComponentArgument::String(id), ComponentArgument::String(label)] if !id.trim().is_empty() && !label.trim().is_empty() => Ok(ComponentPayload::new(Trigger { id: id.clone(), label: label.clone() })), _ => Err("NativeMenuTrigger expects non-empty id and label".into()) })])
.with_methods(vec![
            MethodDescriptor::new("disabled", vec![ArgumentDescriptor::new("disabled", ArgumentSchema::Boolean)], |_| Ok(ComponentPayload::new(()))).with_documentation("Disables the native menu trigger."),
            MethodDescriptor::new("on_effect_error", vec![ArgumentDescriptor::new("callback", ArgumentSchema::Callback("(message: string, cx: Context) => void"))], |args| match args { [argument @ ComponentArgument::Callback(_)] => Ok(ComponentPayload::new(ErrorCallback(argument.clone()))), _ => Err("NativeMenuTrigger.on_effect_error expects callback".into()) }).with_documentation("Reports asynchronous native menu failures."),
        ])
.with_documentation("A real Button trigger that shows an OS NativeMenu (or fallback) in a keyed event effect. Last on_effect_error wins; typed item selection dispatches ShellAction."))?;
    Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_probe {
    use std::cell::Cell;
    thread_local! { static SHOWN: Cell<usize> = const { Cell::new(0) }; }
    pub(super) fn shown() {
        SHOWN.with(|shown| shown.set(shown.get() + 1));
    }
    pub(crate) fn take_shown() -> usize {
        SHOWN.with(|shown| shown.replace(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_lane_is_closed() {
        assert!(
            require_child(
                "NativeMenuTrigger",
                Some("NativeMenuItem"),
                &["NativeMenuItem", "NativeMenuSeparator"]
            )
            .is_ok()
        );
        assert!(require_child("NativeMenuTrigger", None, &["NativeMenuItem"]).is_err());
    }
    #[test]
    fn item_operations_are_last_call_wins_and_combination_is_honest() {
        let base = Item {
            label: "Open".into(),
            action: "old".into(),
            disabled: false,
            checked: false,
        };
        let ops = [
            ItemOp::Action("first".into()),
            ItemOp::Checked(false),
            ItemOp::Action("last".into()),
            ItemOp::Checked(true),
        ];
        let item = resolve_item(&base, false, ops.iter()).unwrap();
        assert_eq!(item.action, "last");
        assert!(item.checked);
        assert!(resolve_item(&base, true, [ItemOp::Checked(true)].iter()).is_err());
        let reporters = [
            ErrorCallback(ComponentArgument::Callback(1)),
            ErrorCallback(ComponentArgument::Callback(2)),
        ];
        assert!(matches!(
            last_reporter(reporters.iter()).unwrap().0,
            ComponentArgument::Callback(2)
        ));
    }
}
