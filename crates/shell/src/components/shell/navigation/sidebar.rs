use super::Empty;
use std::sync::Arc;

use gpui_component::{
    IconName, Side,
    sidebar::{
        Sidebar, SidebarCollapsible, SidebarFooter, SidebarHeader, SidebarMenu, SidebarMenuItem,
        SidebarToggleButton,
    },
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, ParentElement as _, Refineable as _, Styled as _},
};

use super::{Carrier, take};

#[derive(Clone)]
struct Id(String);

#[derive(Clone, Copy)]
enum ToggleOp {
    Side(Side),
    Collapsed(bool),
}

#[derive(Clone)]
enum ItemOp {
    DefaultOpen(bool),
    ClickToOpen(bool),
    ClickToToggle(bool),
    Icon(IconName),
}

#[derive(Clone, Copy)]
enum SidebarOp {
    Side(Side),
    Collapsible(SidebarCollapsible),
    Collapsed(bool),
}

fn nullary(name: &'static str) -> ConstructorDescriptor {
    ConstructorDescriptor::new(name, vec![], |_| Ok(ComponentPayload::new(Empty)))
}

fn bool_method<T: 'static + Send + Sync>(
    owner: &'static str,
    name: &'static str,
    docs: &'static str,
    make: fn(bool) -> T,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Boolean)],
        move |arguments| match arguments {
            [ComponentArgument::Boolean(value)] => Ok(ComponentPayload::new(make(*value))),
            _ => Err(format!("{owner}.{name} expects one boolean")),
        },
    )
    .with_documentation(docs)
}

fn side_method<T: 'static + Send + Sync>(
    owner: &'static str,
    make: fn(Side) -> T,
) -> MethodDescriptor {
    MethodDescriptor::new(
        "side",
        vec![ArgumentDescriptor::new(
            "side",
            ArgumentSchema::Enum(&["left", "right"]),
        )],
        move |arguments| match arguments {
            [ComponentArgument::Enum(value)] => match value.as_str() {
                "left" => Ok(ComponentPayload::new(make(Side::Left))),
                "right" => Ok(ComponentPayload::new(make(Side::Right))),
                _ => Err(format!("unsupported {owner} side `{value}`")),
            },
            _ => Err(format!("{owner}.side expects `left` or `right`")),
        },
    )
    .with_documentation("Sets the physical side occupied by the sidebar control.")
}

fn on_click_method(owner: &'static str) -> MethodDescriptor {
    MethodDescriptor::new(
        "on_click",
        vec![ArgumentDescriptor::new(
            "callback",
            ArgumentSchema::Callback("(event: ClickEvent, cx: Context) => void"),
        )],
        move |arguments| match arguments {
            [ComponentArgument::Callback(_)] => Ok(ComponentPayload::new(Empty)),
            _ => Err(format!("{owner}.on_click expects one callback")),
        },
    )
    .with_documentation("Invokes the callback when the control is activated.")
}

fn require_registered_child(
    parent: &str,
    expected: &'static str,
    actual: Option<&'static str>,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        actual == Some(expected),
        "{parent} accepts only {expected} children; received {}",
        actual.unwrap_or("an ordinary element")
    );
    Ok(())
}

fn require_default_item_style(style: gpui::StyleRefinement) -> anyhow::Result<()> {
    anyhow::ensure!(
        style == gpui::StyleRefinement::default(),
        "SidebarMenuItem does not support shell style operations"
    );
    Ok(())
}

fn apply_edge_selected<E: gpui_component::Selectable>(edge: E, selected: bool) -> E {
    edge.selected(selected)
}

struct MenuItemMaterializer;
impl ComponentMaterializer for MenuItemMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let label = request
            .payload()
            .downcast_ref::<Id>()
            .ok_or_else(|| anyhow::anyhow!("SidebarMenuItem received an incompatible payload"))?;
        let mut item = SidebarMenuItem::new(label.0.clone());
        for operation in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<ItemOp>())
        {
            item = match operation {
                ItemOp::DefaultOpen(value) => item.default_open(*value),
                ItemOp::ClickToOpen(value) => item.click_to_open(*value),
                ItemOp::ClickToToggle(value) => item.click_to_toggle(*value),
                ItemOp::Icon(value) => item.icon(value.clone()),
            };
        }
        item = item.active(request.selected()).disable(request.disabled());
        if let Some(callback) = request.on_click() {
            item = item.on_click(move |event, window, cx| callback.invoke(event, window, cx));
        }
        let mut nested = Vec::new();
        for mut child in request.take_typed_children()? {
            require_registered_child("SidebarMenuItem", "SidebarMenuItem", child.component_name())?;
            let mut element = request.materialize_child(&mut child)?;
            nested.push(take::<SidebarMenuItem>(&mut element, "SidebarMenuItem")?);
        }
        if !nested.is_empty() {
            item = item.children(nested);
        }
        require_default_item_style(request.take_style())?;
        Ok(Carrier::new(item).into_any_element())
    }
}

struct MenuMaterializer;
impl ComponentMaterializer for MenuMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<Empty>()
            .ok_or_else(|| anyhow::anyhow!("SidebarMenu received an incompatible payload"))?;
        let mut menu = SidebarMenu::new();
        menu.style().refine(&request.take_style());
        for mut child in request.take_typed_children()? {
            require_registered_child("SidebarMenu", "SidebarMenuItem", child.component_name())?;
            let mut element = request.materialize_child(&mut child)?;
            menu = menu.child(take::<SidebarMenuItem>(&mut element, "SidebarMenuItem")?);
        }
        Ok(Carrier::new(menu).into_any_element())
    }
}

macro_rules! sidebar_edge_materializer {
    ($materializer:ident, $component:ty, $label:literal) => {
        struct $materializer;
        impl ComponentMaterializer for $materializer {
            fn materialize(
                &self,
                mut request: MaterializeRequest<'_>,
            ) -> anyhow::Result<gpui::AnyElement> {
                request.payload().downcast_ref::<Empty>().ok_or_else(|| {
                    anyhow::anyhow!(concat!($label, " received an incompatible payload"))
                })?;
                let selected = request.selected();
                let mut edge = apply_edge_selected(<$component>::new(), selected);
                edge.style().refine(&request.take_style());
                edge.extend(request.take_children()?);
                Ok(edge.into_any_element())
            }
        }
    };
}
sidebar_edge_materializer!(HeaderMaterializer, SidebarHeader, "SidebarHeader");
sidebar_edge_materializer!(FooterMaterializer, SidebarFooter, "SidebarFooter");

struct SidebarMaterializer;
impl ComponentMaterializer for SidebarMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = request
            .payload()
            .downcast_ref::<Id>()
            .ok_or_else(|| anyhow::anyhow!("Sidebar received an incompatible payload"))?;
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<SidebarOp>().copied())
            .collect::<Vec<_>>();
        let mut sidebar = Sidebar::<SidebarMenu>::new(id.0.clone());
        for operation in operations {
            sidebar = match operation {
                SidebarOp::Side(value) => sidebar.side(value),
                SidebarOp::Collapsible(value) => sidebar.collapsible(value),
                SidebarOp::Collapsed(value) => sidebar.collapsed(value),
            };
        }
        if let Some(header) = request.take_slot("header")? {
            sidebar = sidebar.header(header);
        }
        if let Some(footer) = request.take_slot("footer")? {
            sidebar = sidebar.footer(footer);
        }
        sidebar.style().refine(&request.take_style());
        for mut child in request.take_typed_children()? {
            require_registered_child("Sidebar", "SidebarMenu", child.component_name())?;
            let mut element = request.materialize_child(&mut child)?;
            sidebar = sidebar.child(take::<SidebarMenu>(&mut element, "SidebarMenu")?);
        }
        Ok(sidebar.into_any_element())
    }
}

struct ToggleMaterializer;
impl ComponentMaterializer for ToggleMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request.payload().downcast_ref::<Empty>().ok_or_else(|| {
            anyhow::anyhow!("SidebarToggleButton received an incompatible payload")
        })?;
        let mut toggle = SidebarToggleButton::new();
        for operation in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<ToggleOp>())
        {
            toggle = match operation {
                ToggleOp::Side(value) => toggle.side(*value),
                ToggleOp::Collapsed(value) => toggle.collapsed(*value),
            };
        }
        if let Some(callback) = request.on_click() {
            toggle = toggle.on_click(move |event, window, cx| callback.invoke(event, window, cx));
        }
        anyhow::ensure!(
            request.take_children()?.is_empty(),
            "SidebarToggleButton does not accept children"
        );
        let mut wrapper = gpui::div().child(toggle);
        wrapper.style().refine(&request.take_style());
        Ok(wrapper.into_any_element())
    }
}

fn id_constructor(name: &'static str, argument: &'static str) -> ConstructorDescriptor {
    ConstructorDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(argument, ArgumentSchema::String)],
        move |arguments| match arguments {
            [ComponentArgument::String(value)] if !value.is_empty() => {
                Ok(ComponentPayload::new(Id(value.to_owned())))
            }
            [ComponentArgument::String(_)] => Err(format!("{name} {argument} must not be empty")),
            _ => Err(format!("{name} expects one string {argument}")),
        },
    )
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor::new("SidebarMenuItem", Arc::new(MenuItemMaterializer))
.with_constructors(vec![id_constructor("SidebarMenuItem", "label")])
.with_methods(vec![
            on_click_method("SidebarMenuItem"),
            bool_method(
                "SidebarMenuItem",
                "selected",
                "Sets the active destination state.",
                |_| Empty,
            ),
            bool_method(
                "SidebarMenuItem",
                "default_open",
                "Sets the initial submenu disclosure state.",
                ItemOp::DefaultOpen,
            ),
            bool_method(
                "SidebarMenuItem",
                "click_to_open",
                "Lets a row click open its submenu.",
                ItemOp::ClickToOpen,
            ),
            bool_method(
                "SidebarMenuItem",
                "click_to_toggle",
                "Lets a row click toggle its submenu.",
                ItemOp::ClickToToggle,
            ),
            bool_method(
                "SidebarMenuItem",
                "disabled",
                "Disables pointer activation.",
                |_| Empty,
            ),
            MethodDescriptor::new(
                "icon",
                vec![ArgumentDescriptor::new(
                    "icon",
                    ArgumentSchema::Enum(&[
                        "home",
                        "components",
                        "settings",
                        "archive",
                        "account",
                    ]),
                )],
                |arguments| match arguments {
                    [ComponentArgument::Enum(value)] => Ok(ComponentPayload::new(ItemOp::Icon(
                        match value.as_str() {
                            "home" => IconName::SquareTerminal,
                            "components" => IconName::LayoutDashboard,
                            "settings" => IconName::Settings2,
                            "archive" => IconName::BookOpen,
                            "account" => IconName::User,
                            _ => return Err(format!("unsupported SidebarMenuItem icon `{value}`")),
                        },
                    ))),
                    _ => Err("SidebarMenuItem.icon expects one icon name".into()),
                },
            )
            .with_documentation("Sets the navigation icon used in expanded and icon-collapse modes."),
        ])
.with_documentation("A typed navigation row accepted by SidebarMenu. Shell style operations are unsupported because the native SidebarMenuItem is not Styled."))?;
    registry.register(
        ComponentDescriptor::new("SidebarMenu", Arc::new(MenuMaterializer))
            .with_constructors(vec![nullary("SidebarMenu")])
            .with_methods(vec![])
            .with_documentation("A typed Sidebar menu accepting SidebarMenuItem children."),
    )?;
    let selected = || {
        vec![bool_method(
            "Sidebar edge",
            "selected",
            "Sets the selected presentation.",
            |value| value,
        )]
    };
    registry.register(
        ComponentDescriptor::new("SidebarHeader", Arc::new(HeaderMaterializer))
            .with_constructors(vec![nullary("SidebarHeader")])
            .with_methods(selected())
            .with_documentation("A styled sidebar header accepting ordinary children."),
    )?;
    registry.register(
        ComponentDescriptor::new("SidebarFooter", Arc::new(FooterMaterializer))
            .with_constructors(vec![nullary("SidebarFooter")])
            .with_methods(selected())
            .with_documentation("A styled sidebar footer accepting ordinary children."),
    )?;
    registry.register(ComponentDescriptor::new("Sidebar", Arc::new(SidebarMaterializer))
.with_constructors(vec![id_constructor("Sidebar", "id")])
.with_methods(vec![
            side_method("Sidebar", SidebarOp::Side),
            MethodDescriptor::new(
                "collapsible",
                vec![ArgumentDescriptor::new("mode", ArgumentSchema::Enum(&["icon", "offcanvas", "none"]))],
                |arguments| match arguments {
                    [ComponentArgument::Enum(value)] => match value.as_str() {
                        "icon" => Ok(ComponentPayload::new(SidebarOp::Collapsible(SidebarCollapsible::Icon))),
                        "offcanvas" => Ok(ComponentPayload::new(SidebarOp::Collapsible(SidebarCollapsible::Offcanvas))),
                        "none" => Ok(ComponentPayload::new(SidebarOp::Collapsible(SidebarCollapsible::None))),
                        _ => Err(format!("unsupported Sidebar collapsible mode `{value}`")),
                    },
                    _ => Err("Sidebar.collapsible expects `icon`, `offcanvas`, or `none`".into()),
                },
            ).with_documentation("Sets the sidebar collapse behavior."),
            bool_method("Sidebar", "collapsed", "Sets the controlled collapsed state.", SidebarOp::Collapsed),
        ])
.with_documentation("A typed application sidebar accepting SidebarMenu children and named header/footer slots."))?;
    registry.register(ComponentDescriptor::new("SidebarToggleButton", Arc::new(ToggleMaterializer))
.with_constructors(vec![nullary("SidebarToggleButton")])
.with_methods(vec![
            on_click_method("SidebarToggleButton"),
            side_method("SidebarToggleButton", ToggleOp::Side),
            bool_method(
                "SidebarToggleButton",
                "collapsed",
                "Sets the icon for the current collapsed state.",
                ToggleOp::Collapsed,
            ),
        ])
.with_documentation(
            "A button that reflects sidebar side and collapsed state; on_click is forwarded. Shell style is applied to an explicit wrapper, and children are rejected.",
        ))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_exports_use_closed_boolean_and_enum_schemas() {
        let method = side_method("Sidebar", SidebarOp::Side);
        assert_eq!(
            method.arguments()[0].schema(),
            &ArgumentSchema::Enum(&["left", "right"])
        );
        let collapsed = bool_method("Sidebar", "collapsed", "docs", SidebarOp::Collapsed);
        assert_eq!(collapsed.arguments()[0].schema(), &ArgumentSchema::Boolean);
    }

    #[test]
    fn clickable_sidebar_parts_declare_the_exact_common_callback_schema() {
        let mut registry = ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();

        for component in ["SidebarMenuItem", "SidebarToggleButton"] {
            let descriptor = frozen
                .descriptors()
                .find(|descriptor| descriptor.name() == component)
                .unwrap();
            let method = descriptor
                .methods()
                .iter()
                .find(|method| method.name() == "on_click")
                .unwrap_or_else(|| panic!("{component} needs on_click"));
            assert_eq!(
                method.arguments(),
                [ArgumentDescriptor::new(
                    "callback",
                    ArgumentSchema::Callback("(event: ClickEvent, cx: Context) => void"),
                )]
            );
        }
    }

    #[test]
    fn sidebar_menu_items_expose_a_closed_icon_vocabulary() {
        let mut registry = ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        let descriptor = frozen
            .descriptors()
            .find(|descriptor| descriptor.name() == "SidebarMenuItem")
            .unwrap();
        let icon = descriptor
            .methods()
            .iter()
            .find(|method| method.name() == "icon")
            .expect("SidebarMenuItem needs an icon method for icon-collapse mode");
        assert_eq!(
            icon.arguments()[0].schema(),
            &ArgumentSchema::Enum(&["home", "components", "settings", "archive", "account"])
        );
    }

    #[test]
    fn typed_carrier_rejects_double_consumption() {
        let mut element = Carrier::new(SidebarMenu::new()).into_any_element();
        assert!(take::<SidebarMenu>(&mut element, "SidebarMenu").is_ok());
        assert!(take::<SidebarMenu>(&mut element, "SidebarMenu").is_err());
    }

    #[test]
    fn sidebar_typed_parents_reject_wrong_registered_and_ordinary_children() {
        assert!(require_registered_child("Sidebar", "SidebarMenu", Some("Icon")).is_err());
        let error = require_registered_child("Sidebar", "SidebarMenu", None).unwrap_err();
        assert!(error.to_string().contains("ordinary element"), "{error}");
        assert!(
            require_registered_child("SidebarMenu", "SidebarMenuItem", Some("SidebarMenuItem"))
                .is_ok()
        );
    }

    #[test]
    fn sidebar_menu_item_rejects_style_instead_of_silently_dropping_it() {
        use gpui_shell::gpui::Styled as _;

        assert!(require_default_item_style(gpui::StyleRefinement::default()).is_ok());
        let style = gpui::StyleRefinement::default().p_2();
        let error = require_default_item_style(style).unwrap_err();
        assert!(error.to_string().contains("does not support shell style"));
    }

    #[test]
    fn common_selected_state_reaches_native_header_and_footer_selection() {
        use gpui_component::Selectable as _;

        let header = apply_edge_selected(SidebarHeader::new(), true);
        let footer = apply_edge_selected(SidebarFooter::new(), true);
        assert!(header.is_selected());
        assert!(footer.is_selected());

        let header = apply_edge_selected(header, false);
        let footer = apply_edge_selected(footer, false);
        assert!(!header.is_selected());
        assert!(!footer.is_selected());
    }
}
