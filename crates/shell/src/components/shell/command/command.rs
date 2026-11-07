use super::bool_method;
use super::{Carrier, take};
use super::{reject_style, require_child};
use gpui_component::{
    Disableable as _,
    command::{Command, CommandGroup, CommandItem, CommandState},
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallbackArgument,
    ComponentDescriptor, ComponentMaterializer, ComponentPayload, ComponentRegistry,
    ConstructorDescriptor, MaterializeRequest, MethodDescriptor, RegistryError, StateDescriptor,
    action::ShellAction,
    anyhow,
    gpui::{
        self, AppContext as _, Entity, IntoElement as _, ParentElement as _, Refineable as _,
        Styled as _,
    },
};
use std::sync::Arc;

#[derive(Clone)]
struct ItemPayload(String);
#[derive(Clone)]
struct GroupPayload(String);
#[derive(Clone)]
struct CommandPayload(ComponentArgument);
#[derive(Clone, Copy)]
struct Separator;
#[derive(Clone)]
enum ItemOp {
    Keyword(String),
    Checked(bool),
    Action(String),
}
#[derive(Clone)]
enum CommandOp {
    Searchable(bool),
    Filterable(bool),
    Bordered(bool),
    Placeholder(String),
    MaxHeight(f32),
    OnQuery(ComponentArgument),
    OnSelect(ComponentArgument),
    OnConfirm(ComponentArgument),
    OnCancel(ComponentArgument),
}

struct ItemMaterializer;
impl ComponentMaterializer for ItemMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let label = request
            .payload()
            .downcast_ref::<ItemPayload>()
            .ok_or_else(|| anyhow::anyhow!("CommandItem incompatible payload"))?
            .0
            .clone();
        let mut item = CommandItem::new().label(label).disabled(request.disabled());
        let mut keywords = Vec::new();
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<ItemOp>())
        {
            match op {
                ItemOp::Keyword(value) => keywords.push(value.clone()),
                ItemOp::Checked(value) => item = item.checked(*value),
                ItemOp::Action(value) => {
                    item = item.action(Box::new(ShellAction::new(value.clone())))
                }
            }
        }
        item = item.keywords(keywords);
        reject_style(request.take_style(), "CommandItem")?;
        if let Some(factory) = request.take_slot_factory("content") {
            item = item.child(move |window, cx| match factory.build(window, cx) {
                Ok(element) => {
                    #[cfg(test)]
                    test_probe::built("item");
                    element
                }
                Err(error) => gpui::div()
                    .child(format!("Failed to render CommandItem content: {error:#}"))
                    .into_any_element(),
            });
        }
        for child in request.take_typed_children()? {
            require_child("CommandItem", child.component_name(), &[])?;
        }
        Ok(Carrier::new(item).into_any_element())
    }
}
struct GroupMaterializer;
impl ComponentMaterializer for GroupMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let label = request
            .payload()
            .downcast_ref::<GroupPayload>()
            .ok_or_else(|| anyhow::anyhow!("CommandGroup incompatible payload"))?
            .0
            .clone();
        let mut group = CommandGroup::new().label(label);
        reject_style(request.take_style(), "CommandGroup")?;
        for mut child in request.take_typed_children()? {
            require_child("CommandGroup", child.component_name(), &["CommandItem"])?;
            let mut element = request.materialize_child(&mut child)?;
            group = group.item(take::<CommandItem>(&mut element, "CommandItem")?);
        }
        Ok(Carrier::new(group).into_any_element())
    }
}
struct SeparatorMaterializer;
impl ComponentMaterializer for SeparatorMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<Separator>()
            .ok_or_else(|| anyhow::anyhow!("CommandSeparator incompatible payload"))?;
        reject_style(request.take_style(), "CommandSeparator")?;
        for child in request.take_typed_children()? {
            require_child("CommandSeparator", child.component_name(), &[])?;
        }
        Ok(Carrier::new(Separator).into_any_element())
    }
}
struct CommandMaterializer;
impl ComponentMaterializer for CommandMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let argument = &request
            .payload()
            .downcast_ref::<CommandPayload>()
            .ok_or_else(|| anyhow::anyhow!("Command incompatible payload"))?
            .0;
        let state = request.with_state::<Entity<CommandState>, _>(argument, Clone::clone)?;
        let mut command = Command::new(&state);
        if let Some(factory) = request.take_slot_factory("header") {
            command = command.header(move |_, window, cx| match factory.build(window, cx) {
                Ok(element) => {
                    #[cfg(test)]
                    test_probe::built("header");
                    element
                }
                Err(error) => gpui::div()
                    .child(format!("Failed to render Command header: {error:#}"))
                    .into_any_element(),
            });
        }
        if let Some(factory) = request.take_slot_factory("footer") {
            command = command.footer(move |_, window, cx| match factory.build(window, cx) {
                Ok(element) => {
                    #[cfg(test)]
                    test_probe::built("footer");
                    element
                }
                Err(error) => gpui::div()
                    .child(format!("Failed to render Command footer: {error:#}"))
                    .into_any_element(),
            });
        }
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<CommandOp>())
        {
            command = match op {
                CommandOp::Searchable(value) => command.searchable(*value),
                CommandOp::Filterable(value) => command.filterable(*value),
                CommandOp::Bordered(value) => command.bordered(*value),
                CommandOp::Placeholder(value) => command.placeholder(value.clone()),
                CommandOp::MaxHeight(value) => command.max_h(gpui::px(*value)),
                CommandOp::OnQuery(arg) => {
                    let callback = request.resolve_callback(arg)?;
                    command.on_query(move |query, window, cx| {
                        callback.invoke_and_report_with(
                            "Command.on_query",
                            &[ComponentCallbackArgument::String(query.to_owned())],
                            window,
                            cx,
                        )
                    })
                }
                CommandOp::OnSelect(arg) => {
                    let callback = request.resolve_callback(arg)?;
                    command.on_select(move |path, window, cx| {
                        callback.invoke_and_report_with(
                            "Command.on_select",
                            &[
                                ComponentCallbackArgument::Number(path.section as f64),
                                ComponentCallbackArgument::Number(path.row as f64),
                            ],
                            window,
                            cx,
                        )
                    })
                }
                CommandOp::OnConfirm(arg) => {
                    let callback = request.resolve_callback(arg)?;
                    command.on_confirm(move |path, window, cx| {
                        callback.invoke_and_report_with(
                            "Command.on_confirm",
                            &[
                                ComponentCallbackArgument::Number(path.section as f64),
                                ComponentCallbackArgument::Number(path.row as f64),
                            ],
                            window,
                            cx,
                        )
                    })
                }
                CommandOp::OnCancel(arg) => {
                    let callback = request.resolve_callback(arg)?;
                    command.on_cancel(move |window, cx| {
                        callback.invoke_and_report_with("Command.on_cancel", &[], window, cx)
                    })
                }
            }
        }
        for mut child in request.take_typed_children()? {
            let name = child.component_name();
            require_child(
                "Command",
                name,
                &["CommandItem", "CommandGroup", "CommandSeparator"],
            )?;
            let mut element = request.materialize_child(&mut child)?;
            command = match name {
                Some("CommandItem") => {
                    command.item(take::<CommandItem>(&mut element, "CommandItem")?)
                }
                Some("CommandGroup") => {
                    command.group(take::<CommandGroup>(&mut element, "CommandGroup")?)
                }
                Some("CommandSeparator") => {
                    take::<Separator>(&mut element, "CommandSeparator")?;
                    command.separator()
                }
                _ => unreachable!(),
            };
        }
        command.style().refine(&request.take_style());
        Ok(command.into_any_element())
    }
}

fn callback_method(
    name: &'static str,
    signature: &'static str,
    make: fn(ComponentArgument) -> CommandOp,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(
            "callback",
            ArgumentSchema::Callback(signature),
        )],
        move |args| match args {
            [argument @ ComponentArgument::Callback(_)] => {
                Ok(ComponentPayload::new(make(argument.clone())))
            }
            _ => Err(format!("Command.{name} expects callback")),
        },
    )
    .with_documentation("Runs after the native Command state releases its update lease.")
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register_state(
        StateDescriptor::new("CommandState", "CommandState", vec![], |_, window, cx| {
            Ok(Box::new(cx.new(|cx| CommandState::new(window, cx))))
        })
        .with_documentation(
            "Retained native command query, focus, selection, measurement and scroll state.",
        ),
    )?;
    registry.register(ComponentDescriptor::new("CommandItem", Arc::new(ItemMaterializer))
.with_constructors(vec![ConstructorDescriptor::new("CommandItem", vec![ArgumentDescriptor::new("label", ArgumentSchema::String)], |args| match args { [ComponentArgument::String(label)] if !label.trim().is_empty() => Ok(ComponentPayload::new(ItemPayload(label.clone()))), _ => Err("CommandItem expects non-empty label".into()) })])
.with_methods(vec![
        MethodDescriptor::new("disabled", vec![ArgumentDescriptor::new("disabled", ArgumentSchema::Boolean)], |_| Ok(ComponentPayload::new(()))).with_documentation("Sets common disabled state."),
        MethodDescriptor::new("keyword", vec![ArgumentDescriptor::new("keyword", ArgumentSchema::String)], |args| match args { [ComponentArgument::String(value)] if !value.trim().is_empty() => Ok(ComponentPayload::new(ItemOp::Keyword(value.clone()))), _ => Err("CommandItem.keyword expects non-empty text".into()) }).with_documentation("Sets the item search keyword."),
        MethodDescriptor::new("checked", vec![ArgumentDescriptor::new("checked", ArgumentSchema::Boolean)], |args| match args { [ComponentArgument::Boolean(value)] => Ok(ComponentPayload::new(ItemOp::Checked(*value))), _ => Err("CommandItem.checked expects boolean".into()) }).with_documentation("Sets the item checked state."),
        MethodDescriptor::new("action", vec![ArgumentDescriptor::new("action", ArgumentSchema::String)], |args| match args { [ComponentArgument::String(value)] if !value.trim().is_empty() => Ok(ComponentPayload::new(ItemOp::Action(value.clone()))), _ => Err("CommandItem.action expects non-empty action id".into()) }).with_documentation("Dispatches the named shell action when selected."),
    ])
.with_documentation("Typed native CommandItem data. Action strings map to ShellAction; style and ordinary/typed children are rejected. Named content(element) is a repeatable lazy row factory."))?;
    registry.register(ComponentDescriptor::new("CommandGroup", Arc::new(GroupMaterializer))
.with_constructors(vec![ConstructorDescriptor::new("CommandGroup", vec![ArgumentDescriptor::new("label", ArgumentSchema::String)], |args| match args { [ComponentArgument::String(label)] if !label.trim().is_empty() => Ok(ComponentPayload::new(GroupPayload(label.clone()))), _ => Err("CommandGroup expects non-empty label".into()) })])
.with_methods(vec![])
.with_documentation("Typed native CommandGroup data accepting only CommandItem children; style is rejected."))?;
    registry.register(
        ComponentDescriptor::new("CommandSeparator", Arc::new(SeparatorMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "CommandSeparator",
                vec![],
                |_| Ok(ComponentPayload::new(Separator)),
            )])
            .with_methods(vec![])
            .with_documentation("Typed Command separator data; style and children are rejected."),
    )?;
    registry.register(ComponentDescriptor::new("Command", Arc::new(CommandMaterializer))
.with_constructors(vec![ConstructorDescriptor::new("Command", vec![ArgumentDescriptor::new("state", ArgumentSchema::Entity("CommandState"))], |args| match args { [argument @ ComponentArgument::Entity { .. }] => Ok(ComponentPayload::new(CommandPayload(argument.clone()))), _ => Err("Command expects CommandState".into()) })])
.with_methods(vec![
        bool_method("Command", "searchable", "Sets native Command behavior.", CommandOp::Searchable), bool_method("Command", "filterable", "Sets native Command behavior.", CommandOp::Filterable), bool_method("Command", "bordered", "Sets native Command behavior.", CommandOp::Bordered),
        MethodDescriptor::new("placeholder", vec![ArgumentDescriptor::new("placeholder", ArgumentSchema::String)], |args| match args { [ComponentArgument::String(value)] => Ok(ComponentPayload::new(CommandOp::Placeholder(value.clone()))), _ => Err("Command.placeholder expects text".into()) }).with_documentation("Sets the command search placeholder."),
        MethodDescriptor::new("max_height", vec![ArgumentDescriptor::new("pixels", ArgumentSchema::Number)], |args| match args { [ComponentArgument::Number(value)] if value.is_finite() && *value > 0. && *value <= f32::MAX as f64 => Ok(ComponentPayload::new(CommandOp::MaxHeight(*value as f32))), _ => Err("Command.max_height expects positive finite pixels".into()) }).with_documentation("Sets the command results maximum height."),
        callback_method("on_query", "(query: string, cx: Context) => void", CommandOp::OnQuery), callback_method("on_select", "(section: number, row: number, cx: Context) => void", CommandOp::OnSelect), callback_method("on_confirm", "(section: number, row: number, cx: Context) => void", CommandOp::OnConfirm), callback_method("on_cancel", "(cx: Context) => void", CommandOp::OnCancel),
    ])
.with_documentation("Styled retained native Command palette consuming CommandItem, CommandGroup and CommandSeparator in exact order. Named header/footer elements are repeatable lazy factories; native empty content remains unavailable because the shell has no common empty(element) named-slot route."))?;
    Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_probe {
    use std::cell::RefCell;
    thread_local! { static BUILDS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) }; }
    pub(super) fn built(name: &'static str) {
        BUILDS.with(|builds| builds.borrow_mut().push(name));
    }
    pub(crate) fn take() -> Vec<&'static str> {
        BUILDS.with(|builds| std::mem::take(&mut *builds.borrow_mut()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_boundaries_are_closed() {
        assert!(
            require_child(
                "Command",
                Some("CommandItem"),
                &["CommandItem", "CommandGroup", "CommandSeparator"]
            )
            .is_ok()
        );
        assert!(require_child("Command", None, &["CommandItem"]).is_err());
        assert!(require_child("CommandGroup", Some("CommandGroup"), &["CommandItem"]).is_err());
    }
}
