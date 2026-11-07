use super::bool_method;
use super::reject_style;
use super::{Carrier, take};
use gpui_component::{GlobalState, menu::AppMenuBar};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, RegistryError,
    action::ShellAction,
    anyhow,
    gpui::{
        self, IntoElement as _, Menu, MenuItem, OwnedMenu, OwnedMenuItem, ParentElement as _, div,
    },
};
use std::sync::Arc;

#[derive(Clone, Debug, Hash)]
struct ItemSpec {
    label: String,
    action: String,
    disabled: bool,
    checked: bool,
}
#[derive(Clone, Debug, Hash)]
enum Entry {
    Item(ItemSpec),
    Separator,
}
#[derive(Clone, Debug, Hash)]
struct MenuSpec {
    label: String,
    disabled: bool,
    entries: Vec<Entry>,
}
#[derive(Clone)]
struct Label(String);
#[derive(Clone, Copy)]
enum BoolOp {
    Disabled(bool),
    Checked(bool),
}

struct ItemMaterializer;
impl ComponentMaterializer for ItemMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let mut item = request
            .payload()
            .downcast_ref::<ItemSpec>()
            .ok_or_else(|| anyhow::anyhow!("MenuItem incompatible payload"))?
            .clone();
        for op in request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<BoolOp>())
        {
            match op {
                BoolOp::Disabled(value) => item.disabled = *value,
                BoolOp::Checked(value) => item.checked = *value,
            }
        }
        reject_style(request.take_style(), "MenuItem")?;
        Ok(Carrier::new(Entry::Item(item)).into_any_element())
    }
}
struct SeparatorMaterializer;
impl ComponentMaterializer for SeparatorMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        reject_style(request.take_style(), "MenuSeparator")?;
        Ok(Carrier::new(Entry::Separator).into_any_element())
    }
}
struct MenuMaterializer;
impl ComponentMaterializer for MenuMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let mut spec = MenuSpec {
            label: request
                .payload()
                .downcast_ref::<Label>()
                .ok_or_else(|| anyhow::anyhow!("Menu incompatible payload"))?
                .0
                .clone(),
            disabled: false,
            entries: vec![],
        };
        for op in request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<BoolOp>())
        {
            if let BoolOp::Disabled(value) = op {
                spec.disabled = *value
            }
        }
        reject_style(request.take_style(), "Menu")?;
        for mut child in request.take_typed_children()? {
            anyhow::ensure!(
                matches!(child.component_name(), Some("MenuItem" | "MenuSeparator")),
                "Menu accepts only MenuItem or MenuSeparator children"
            );
            let mut element = request.materialize_child(&mut child)?;
            spec.entries
                .push(take::<Entry>(&mut element, "Menu entry")?);
        }
        Ok(Carrier::new(spec).into_any_element())
    }
}
fn build_menu(spec: &MenuSpec) -> Menu {
    Menu::new(spec.label.clone())
        .disabled(spec.disabled)
        .items(spec.entries.iter().map(|entry| {
            match entry {
                Entry::Separator => MenuItem::separator(),
                Entry::Item(item) => {
                    MenuItem::action(item.label.clone(), ShellAction::new(item.action.clone()))
                        .disabled(item.disabled)
                        .checked(item.checked)
                }
            }
        }))
}
fn restore_menu(menu: &OwnedMenu) -> Menu {
    Menu {
        name: menu.name.clone(),
        disabled: menu.disabled,
        items: menu
            .items
            .iter()
            .map(|item| match item {
                OwnedMenuItem::Separator => MenuItem::Separator,
                OwnedMenuItem::Submenu(menu) => MenuItem::Submenu(restore_menu(menu)),
                OwnedMenuItem::SystemMenu(menu) => MenuItem::SystemMenu(gpui::OsMenu {
                    name: menu.name.clone(),
                    menu_type: menu.menu_type,
                }),
                OwnedMenuItem::Action {
                    name,
                    action,
                    os_action,
                    checked,
                    disabled,
                } => MenuItem::Action {
                    name: name.clone().into(),
                    action: action.boxed_clone(),
                    os_action: *os_action,
                    checked: *checked,
                    disabled: *disabled,
                },
            })
            .collect(),
    }
}
struct BarMaterializer;
impl ComponentMaterializer for BarMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = request
            .payload()
            .downcast_ref::<Label>()
            .ok_or_else(|| anyhow::anyhow!("MenuBar incompatible payload"))?
            .0
            .clone();
        let effects = request.app_effects()?;
        let mut specs = Vec::new();
        for mut child in request.take_typed_children()? {
            anyhow::ensure!(
                child.component_name() == Some("Menu"),
                "MenuBar accepts only Menu children"
            );
            let mut element = request.materialize_child(&mut child)?;
            specs.push(take::<MenuSpec>(&mut element, "Menu")?);
        }
        // A revision only has to change when the menu does. Hashing the specs
        // gives that without formatting the whole tree into a string on every
        // render, and without leaning on `Debug`, whose output is explicitly
        // not a stable format to key on.
        let revision = {
            let mut hasher = std::hash::DefaultHasher::new();
            std::hash::Hash::hash(&specs, &mut hasher);
            format!("{:016x}", std::hash::Hasher::finish(&hasher))
        };
        let bar = request.with_window_app(|window, cx| {
            let retained = window.use_keyed_state(format!("shell-menu-bar:{id}"), cx, |_, cx| {
                AppMenuBar::new(cx)
            });
            let bar = retained.read(cx).clone();
            let install_bar = bar.clone();
            effects.replace(format!("menu-bar:{id}"), revision, window, cx, move |cx| {
                let previous = cx.get_menus().unwrap_or_default();
                let owned = specs
                    .iter()
                    .map(build_menu)
                    .map(Menu::owned)
                    .collect::<Vec<_>>();
                cx.set_menus(specs.iter().map(build_menu));
                GlobalState::global_mut(cx).set_app_menus(owned);
                install_bar.update(cx, |bar, cx| bar.reload(cx));
                let cleanup_bar = install_bar.clone();
                Box::new(move |cx| {
                    cx.set_menus(previous.iter().map(restore_menu));
                    GlobalState::global_mut(cx).set_app_menus(previous);
                    cleanup_bar.update(cx, |bar, cx| bar.reload(cx));
                })
            })?;
            Ok(bar)
        })?;
        request.finish(div().child(bar))
    }
}
fn label_constructor(name: &'static str) -> ConstructorDescriptor {
    ConstructorDescriptor::new(
        name,
        vec![ArgumentDescriptor::new("label", ArgumentSchema::String)],
        move |arguments| match arguments {
            [ComponentArgument::String(value)] if !value.trim().is_empty() => {
                Ok(ComponentPayload::new(Label(value.clone())))
            }
            _ => Err(format!("{name} expects a non-empty label")),
        },
    )
}
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("MenuItem", Arc::new(ItemMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "MenuItem",
                vec![
                    ArgumentDescriptor::new("label", ArgumentSchema::String),
                    ArgumentDescriptor::new("action", ArgumentSchema::String),
                ],
                |arguments| match arguments {
                    [
                        ComponentArgument::String(label),
                        ComponentArgument::String(action),
                    ] if !label.trim().is_empty() && !action.trim().is_empty() => {
                        Ok(ComponentPayload::new(ItemSpec {
                            label: label.clone(),
                            action: action.clone(),
                            disabled: false,
                            checked: false,
                        }))
                    }
                    _ => Err("MenuItem expects non-empty label and action".into()),
                },
            )])
            .with_methods(vec![
                bool_method(
                    "Menu",
                    "disabled",
                    "Sets native menu item state.",
                    BoolOp::Disabled,
                ),
                bool_method(
                    "Menu",
                    "checked",
                    "Sets native menu item state.",
                    BoolOp::Checked,
                ),
            ])
            .with_documentation("Typed application-menu action data."),
    )?;
    registry.register(
        ComponentDescriptor::new("MenuSeparator", Arc::new(SeparatorMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "MenuSeparator",
                vec![],
                |a| {
                    if a.is_empty() {
                        Ok(ComponentPayload::new(()))
                    } else {
                        Err("MenuSeparator expects no arguments".into())
                    }
                },
            )])
            .with_methods(vec![])
            .with_documentation("Typed application-menu separator data."),
    )?;
    registry.register(
        ComponentDescriptor::new("Menu", Arc::new(MenuMaterializer))
            .with_constructors(vec![label_constructor("Menu")])
            .with_methods(vec![bool_method(
                "Menu",
                "disabled",
                "Disables the whole menu.",
                BoolOp::Disabled,
            )])
            .with_documentation("Typed top-level application menu data."),
    )?;
    registry.register(
        ComponentDescriptor::new("MenuBar", Arc::new(BarMaterializer))
            .with_constructors(vec![label_constructor("MenuBar")])
            .with_methods(vec![])
            .with_documentation("A generation-owned native and in-window application menu bar."),
    )?;
    Ok(())
}
