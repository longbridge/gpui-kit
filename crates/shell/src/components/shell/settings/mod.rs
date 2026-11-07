//! Native typed settings hierarchy.
//!
//! Value fields and reset callbacks are deferred: native getters accept only
//! `&App`, while shell callbacks currently require live `Window` + `App` authority.

use super::support::{Empty, bool_method, reject_style, require_child};

use super::typed_child::{Carrier, take};
use gpui_component::{
    Sizable as _, Size,
    setting::{SelectIndex, SettingField, SettingGroup, SettingItem, SettingPage, Settings},
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, Axis, IntoElement as _, ParentElement as _, Refineable as _, Styled as _, px},
};
use std::sync::Arc;

#[derive(Clone)]
struct Text(String);
#[derive(Clone)]
enum TextOp {
    Description(String),
    Title(String),
}
#[derive(Clone, Copy)]
enum BoolOp {
    DefaultOpen(bool),
    Resettable(bool),
}
#[derive(Clone)]
enum ItemOp {
    Description(String),
    Layout(Axis),
    Keywords(Vec<String>),
    Disabled(bool),
}
#[derive(Clone, Copy)]
enum SettingsOp {
    Size(Size),
    SidebarWidth(f32),
    SidebarRange(f32, f32),
    Selected(usize),
}
fn positive(value: f64, label: &str) -> Result<f32, String> {
    if value.is_finite() && value > 0. && value <= f32::MAX as f64 {
        Ok(value as f32)
    } else {
        Err(format!("{label} expects a positive finite pixel value"))
    }
}
struct ItemMaterializer;
impl ComponentMaterializer for ItemMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let title = request
            .payload()
            .downcast_ref::<Text>()
            .ok_or_else(|| anyhow::anyhow!("SettingItem payload"))?
            .0
            .clone();
        let field = request
            .take_slot_factory("content")
            .ok_or_else(|| anyhow::anyhow!("SettingItem requires content(element)"))?;
        anyhow::ensure!(
            request.children_len() == 0,
            "SettingItem does not accept children"
        );
        let mut sf = SettingField::render(move |_, window, cx| match field.build(window, cx) {
            Ok(e) => e,
            Err(e) => gpui::div()
                .child(format!("Failed to render setting field: {e:#}"))
                .into_any_element(),
        });
        sf.style().refine(&request.take_style());
        let mut item = SettingItem::new(title, sf);
        for op in request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<ItemOp>())
        {
            item = match op {
                ItemOp::Description(value) => item.description(value.clone()),
                ItemOp::Layout(value) => item.layout(*value),
                ItemOp::Keywords(value) => item.keywords(value.clone()),
                ItemOp::Disabled(value) => item.disabled(*value),
            }
        }
        Ok(Carrier::new(item).into_any_element())
    }
}
struct GroupMaterializer;
impl ComponentMaterializer for GroupMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<Empty>()
            .ok_or_else(|| anyhow::anyhow!("SettingGroup payload"))?;
        let mut group = SettingGroup::new();
        for op in request.methods() {
            if let Some(TextOp::Title(value)) = op.payload().downcast_ref::<TextOp>() {
                group = group.title(value.clone())
            }
            if let Some(TextOp::Description(value)) = op.payload().downcast_ref::<TextOp>() {
                group = group.description(value.clone())
            }
        }
        group.style().refine(&request.take_style());
        for mut child in request.take_typed_children()? {
            require_child("SettingItem", child.component_name(), &["SettingItem"])?;
            let mut element = request.materialize_child(&mut child)?;
            group = group.item(take::<SettingItem>(&mut element, "SettingItem")?)
        }
        Ok(Carrier::new(group).into_any_element())
    }
}
struct PageMaterializer;
impl ComponentMaterializer for PageMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let title = request
            .payload()
            .downcast_ref::<Text>()
            .ok_or_else(|| anyhow::anyhow!("SettingPage payload"))?
            .0
            .clone();
        let mut page = SettingPage::new(title);
        for op in request.methods() {
            if let Some(TextOp::Description(value)) = op.payload().downcast_ref::<TextOp>() {
                page = page.description(value.clone())
            }
            if let Some(BoolOp::DefaultOpen(value)) = op.payload().downcast_ref::<BoolOp>() {
                page = page.default_open(*value)
            }
            if let Some(BoolOp::Resettable(value)) = op.payload().downcast_ref::<BoolOp>() {
                page = page.resettable(*value)
            }
        }
        if let Some(factory) = request.take_slot_factory("content") {
            page = page.title_suffix(move |window, cx| match factory.build(window, cx) {
                Ok(e) => e,
                Err(e) => gpui::div()
                    .child(format!("Failed to render title suffix: {e:#}"))
                    .into_any_element(),
            })
        }
        reject_style(request.take_style(), "SettingPage")?;
        for mut child in request.take_typed_children()? {
            require_child("SettingGroup", child.component_name(), &["SettingGroup"])?;
            let mut element = request.materialize_child(&mut child)?;
            page = page.group(take::<SettingGroup>(&mut element, "SettingGroup")?)
        }
        Ok(Carrier::new(page).into_any_element())
    }
}
struct SettingsMaterializer;
impl ComponentMaterializer for SettingsMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = request
            .payload()
            .downcast_ref::<Text>()
            .ok_or_else(|| anyhow::anyhow!("Settings payload"))?
            .0
            .clone();
        let mut settings = Settings::new(id);
        for op in request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<SettingsOp>())
        {
            settings = match op {
                SettingsOp::Size(value) => settings.with_size(*value),
                SettingsOp::SidebarWidth(value) => settings.sidebar_width(px(*value)),
                SettingsOp::SidebarRange(a, b) => settings.sidebar_size_range(px(*a)..px(*b)),
                SettingsOp::Selected(value) => settings.default_selected_index(SelectIndex {
                    page_ix: *value,
                    group_ix: None,
                }),
            }
        }
        reject_style(request.take_style(), "Settings")?;
        for mut child in request.take_typed_children()? {
            require_child("SettingPage", child.component_name(), &["SettingPage"])?;
            let mut element = request.materialize_child(&mut child)?;
            settings = settings.page(take::<SettingPage>(&mut element, "SettingPage")?)
        }
        Ok(settings.into_any_element())
    }
}

fn text_method(
    name: &'static str,
    documentation: &'static str,
    op: fn(String) -> TextOp,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new("text", ArgumentSchema::String)],
        move |arguments| match arguments {
            [ComponentArgument::String(value)] if !value.trim().is_empty() => {
                Ok(ComponentPayload::new(op(value.clone())))
            }
            _ => Err(format!("{name} expects non-empty text")),
        },
    )
    .with_documentation(documentation)
}
pub(super) fn register(reg: &mut ComponentRegistry) -> Result<(), RegistryError> {
    reg.register(ComponentDescriptor::new("SettingItem", Arc::new(ItemMaterializer))
.with_constructors(vec![ConstructorDescriptor::new("SettingItem",vec![ArgumentDescriptor::new("title",ArgumentSchema::String)],|a|match a{[ComponentArgument::String(value)]if !value.trim().is_empty()=>Ok(ComponentPayload::new(Text(value.clone()))),_=>Err("SettingItem expects non-empty title".into())})])
.with_methods(vec![MethodDescriptor::new("description",vec![ArgumentDescriptor::new("text",ArgumentSchema::String)],|a|match a{[ComponentArgument::String(value)]=>Ok(ComponentPayload::new(ItemOp::Description(value.clone()))),_=>Err("description expects text".into())}).with_documentation("Sets the supporting description shown under the title."),MethodDescriptor::new("layout",vec![ArgumentDescriptor::new("axis",ArgumentSchema::Enum(&["horizontal","vertical"]))],|a|match a{[ComponentArgument::Enum(value)]if value=="horizontal"=>Ok(ComponentPayload::new(ItemOp::Layout(Axis::Horizontal))),[ComponentArgument::Enum(value)]if value=="vertical"=>Ok(ComponentPayload::new(ItemOp::Layout(Axis::Vertical))),_=>Err("layout expects horizontal or vertical".into())}).with_documentation("Lays the item's label and field out along the given axis."),MethodDescriptor::new("keywords",vec![ArgumentDescriptor::new("keywords",ArgumentSchema::Array(Box::new(ArgumentSchema::String)))],|a|match a{[ComponentArgument::Array(value)]=>value.iter().map(|value|match value{ComponentArgument::String(s)=>Ok(s.clone()),_=>Err("keywords expects strings".into())}).collect::<Result<Vec<_>,String>>().map(ItemOp::Keywords).map(ComponentPayload::new),_=>Err("keywords expects string array".into())}).with_documentation("Adds search keywords that match this item."),MethodDescriptor::new("disabled",vec![ArgumentDescriptor::new("disabled",ArgumentSchema::Boolean)],|a|match a{[ComponentArgument::Boolean(value)]=>Ok(ComponentPayload::new(ItemOp::Disabled(*value))),_=>Err("disabled expects boolean".into())}).with_documentation("Disables the item's field.")])
.with_documentation("A typed native setting item requiring lazy content(element); style applies to the field."))?;
    reg.register(
        ComponentDescriptor::new("SettingGroup", Arc::new(GroupMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "SettingGroup",
                vec![],
                |_| Ok(ComponentPayload::new(Empty)),
            )])
            .with_methods(vec![
                text_method("title", "Sets the displayed title.", TextOp::Title),
                text_method(
                    "description",
                    "Sets the supporting description shown under the title.",
                    TextOp::Description,
                ),
            ])
            .with_documentation(
                "A styled native setting group accepting only SettingItem children.",
            ),
    )?;
    reg.register(ComponentDescriptor::new("SettingPage", Arc::new(PageMaterializer))
.with_constructors(vec![ConstructorDescriptor::new("SettingPage",vec![ArgumentDescriptor::new("title",ArgumentSchema::String)],|a|match a{[ComponentArgument::String(value)]if !value.trim().is_empty()=>Ok(ComponentPayload::new(Text(value.clone()))),_=>Err("SettingPage expects non-empty title".into())})])
.with_methods(vec![text_method("description", "Sets the supporting description shown under the title.", TextOp::Description),bool_method("Setting", "default_open", "Opens the page's groups by default.", BoolOp::DefaultOpen),bool_method("Setting", "resettable", "Shows the control that restores the default value.", BoolOp::Resettable)])
.with_documentation("A typed native setting page accepting SettingGroup children and lazy content(element) as its title suffix; style is rejected."))?;
    reg.register(ComponentDescriptor::new("Settings", Arc::new(SettingsMaterializer))
.with_constructors(vec![ConstructorDescriptor::new("Settings",vec![ArgumentDescriptor::new("id",ArgumentSchema::String)],|a|match a{[ComponentArgument::String(value)]if !value.trim().is_empty()=>Ok(ComponentPayload::new(Text(value.clone()))),_=>Err("Settings expects non-empty id".into())})])
.with_methods(vec![MethodDescriptor::new("size",vec![ArgumentDescriptor::new("size",ArgumentSchema::Enum(&["xsmall","small","medium","large"]))],|a|match a{[ComponentArgument::Enum(value)]=>match value.as_str(){"xsmall"=>Ok(Size::XSmall),"small"=>Ok(Size::Small),"medium"=>Ok(Size::Medium),"large"=>Ok(Size::Large),_=>Err("unsupported size".into())}.map(SettingsOp::Size).map(ComponentPayload::new),_=>Err("size expects semantic size".into())}).with_documentation("Sets the settings surface's semantic size."),MethodDescriptor::new("sidebar_width",vec![ArgumentDescriptor::new("pixels",ArgumentSchema::Number)],|a|match a{[ComponentArgument::Number(value)]=>positive(*value,"sidebar_width").map(SettingsOp::SidebarWidth).map(ComponentPayload::new),_=>Err("sidebar_width expects number".into())}).with_documentation("Sets the sidebar's width in pixels."),MethodDescriptor::new("sidebar_size_range",vec![ArgumentDescriptor::new("minimum",ArgumentSchema::Number),ArgumentDescriptor::new("maximum",ArgumentSchema::Number)],|a|match a{[ComponentArgument::Number(min),ComponentArgument::Number(max)]=>{let min=positive(*min,"minimum")?;let max=positive(*max,"maximum")?;if min>max{return Err("minimum must not exceed maximum".into())}Ok(ComponentPayload::new(SettingsOp::SidebarRange(min,max)))},_=>Err("sidebar_size_range expects two numbers".into())}).with_documentation("Bounds how far the sidebar can be resized, in pixels."),MethodDescriptor::new("default_selected_page",vec![ArgumentDescriptor::new("index",ArgumentSchema::Number)],|a|match a{[ComponentArgument::Number(value)]if value.is_finite()&&*value>=0.&&value.fract()==0.&&*value<=usize::MAX as f64=>Ok(ComponentPayload::new(SettingsOp::Selected(*value as usize))),_=>Err("default_selected_page expects a nonnegative integer".into())}).with_documentation("Selects the page shown when the surface first opens.")])
.with_documentation("A native keyed-state settings container accepting only SettingPage children; style is rejected."))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_and_structural_contracts_are_closed() {
        assert_eq!(positive(1., "width").unwrap(), 1.);
        assert!(positive(0., "width").is_err());
        assert!(positive(f64::INFINITY, "width").is_err());
        assert!(require_child("Settings", Some("SettingPage"), &["SettingPage"]).is_ok());
        assert!(require_child("Settings", Some("SettingGroup"), &["SettingPage"]).is_err());
        assert!(require_child("SettingPage", None, &["SettingPage"]).is_err());
        assert!(reject_style(gpui::StyleRefinement::default(), "Settings").is_ok());
        assert!(reject_style(gpui::StyleRefinement::default().p_2(), "Settings").is_err());
    }
}
