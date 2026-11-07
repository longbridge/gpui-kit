use super::common::positive_usize;
use super::require_child;
use super::{Carrier, take};
use gpui_component::{
    Sizable as _, Size,
    description_list::{DescriptionItem, DescriptionList},
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow, gpui,
    gpui::{
        IntoElement as _, ParentElement as _, Refineable as _, StyleRefinement, Styled as _, div,
    },
};
use std::sync::Arc;

#[derive(Clone)]
struct ItemPayload(String);
#[derive(Clone)]
enum ItemOp {
    Value(String),
    Span(usize),
}
struct ItemMaterializer;
impl ComponentMaterializer for ItemMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        ensure_item_surface(request.children_len(), &request.take_style())?;
        let label = &request
            .payload()
            .downcast_ref::<ItemPayload>()
            .ok_or_else(|| anyhow::anyhow!("DescriptionItem received an incompatible payload"))?
            .0;
        let mut item = DescriptionItem::new(label.clone());
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<ItemOp>())
        {
            item = match op {
                ItemOp::Value(value) => item.value(value.clone()),
                ItemOp::Span(span) => item.span(*span),
            };
        }
        Ok(Carrier::new(item).into_any_element())
    }
}

fn ensure_item_surface(children: usize, style: &StyleRefinement) -> anyhow::Result<()> {
    anyhow::ensure!(
        children == 0,
        "DescriptionItem does not accept children; use value(string)"
    );
    anyhow::ensure!(
        *style == StyleRefinement::default(),
        "DescriptionItem does not accept style methods because its native value is a typed, non-element list part"
    );
    Ok(())
}

#[derive(Clone, Copy)]
struct ListPayload;
#[derive(Clone, Copy)]
enum ListOp {
    Vertical,
    Bordered(bool),
    Columns(usize),
    Size(Size),
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct ListConfig {
    vertical: bool,
    bordered: bool,
    columns: usize,
    size: Size,
}
impl Default for ListConfig {
    fn default() -> Self {
        Self {
            vertical: false,
            bordered: true,
            columns: 3,
            size: Size::Medium,
        }
    }
}
impl ListConfig {
    fn apply(&mut self, op: &ListOp) {
        match op {
            ListOp::Vertical => self.vertical = true,
            ListOp::Bordered(value) => self.bordered = *value,
            ListOp::Columns(value) => self.columns = *value,
            ListOp::Size(value) => self.size = *value,
        }
    }
}
struct ListMaterializer;
impl ComponentMaterializer for ListMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<ListPayload>()
            .ok_or_else(|| anyhow::anyhow!("DescriptionList received an incompatible payload"))?;
        let mut config = ListConfig::default();
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<ListOp>())
        {
            config.apply(op);
        }
        let mut list = if config.vertical {
            DescriptionList::vertical()
        } else {
            DescriptionList::horizontal()
        }
        .bordered(config.bordered)
        .columns(config.columns)
        .with_size(config.size);
        for mut child in request.take_typed_children()? {
            require_child(
                "DescriptionList",
                child.component_name(),
                &["DescriptionItem"],
            )?;
            let mut element = request.materialize_child(&mut child)?;
            list = list.child(take::<DescriptionItem>(&mut element, "DescriptionItem")?);
        }
        let mut wrapper = div().child(list);
        wrapper.style().refine(&request.take_style());
        Ok(wrapper.into_any_element())
    }
}

fn positive_usize_payload(
    args: &[ComponentArgument],
    callable: &str,
    make: impl FnOnce(usize) -> ComponentPayload,
) -> Result<ComponentPayload, String> {
    match args {
        [ComponentArgument::Number(value)] => positive_usize(*value, callable).map(make),
        _ => Err(format!("{callable} expects a positive integer")),
    }
}
fn columns_payload(args: &[ComponentArgument]) -> Result<ComponentPayload, String> {
    match args {
        [ComponentArgument::Number(value)] => {
            let columns = positive_usize(*value, "DescriptionList.columns")?;
            if columns > 10 {
                return Err("DescriptionList.columns expects an integer from 1 through 10".into());
            }
            Ok(ComponentPayload::new(ListOp::Columns(columns)))
        }
        _ => Err("DescriptionList.columns expects an integer from 1 through 10".into()),
    }
}
fn size(
    args: &[ComponentArgument],
    callable: &str,
    make: impl FnOnce(Size) -> ComponentPayload,
) -> Result<ComponentPayload, String> {
    match args {
        [ComponentArgument::Enum(value)] => match value.as_str() {
            "xsmall" => Ok(make(Size::XSmall)),
            "small" => Ok(make(Size::Small)),
            "medium" => Ok(make(Size::Medium)),
            "large" => Ok(make(Size::Large)),
            _ => Err(format!("unsupported {callable} size `{value}`")),
        },
        _ => Err(format!("{callable} expects a size literal")),
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor::new("DescriptionItem", Arc::new(ItemMaterializer))
.with_constructors(vec![ConstructorDescriptor::new(
            "DescriptionItem",
            vec![ArgumentDescriptor::new("label", ArgumentSchema::String)],
            |arguments| match arguments {
                [ComponentArgument::String(value)] => Ok(ComponentPayload::new(ItemPayload(value.clone()))),
                _ => Err("DescriptionItem(label) expects a string".into()),
            },
        )])
.with_methods(vec![
            MethodDescriptor::new(
                "value",
                vec![ArgumentDescriptor::new("value", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(value)] => {
                        Ok(ComponentPayload::new(ItemOp::Value(value.clone())))
                    }
                    _ => Err("DescriptionItem.value(value) expects a string".into()),
                },
            )
            .with_documentation("Sets the item's textual value."),
            MethodDescriptor::new(
                "span",
                vec![ArgumentDescriptor::new("span", ArgumentSchema::Number)],
                |a| {
                    positive_usize_payload(a, "DescriptionItem.span", |value| {
                        ComponentPayload::new(ItemOp::Span(value))
                    })
                },
            )
            .with_documentation("Sets how many description-list columns the item spans."),
        ])
.with_documentation(
            "A typed label/value child for DescriptionList. It accepts value(string) and span(number), but no children or common style methods because the native item is not an independently rendered element.",
        ))?;
    registry.register(
        ComponentDescriptor::new("DescriptionList", Arc::new(ListMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "DescriptionList",
                vec![],
                |_| Ok(ComponentPayload::new(ListPayload)),
            )])
            .with_methods(vec![
                MethodDescriptor::new("vertical", vec![], |_| {
                    Ok(ComponentPayload::new(ListOp::Vertical))
                })
                .with_documentation("Uses the vertical label/value layout."),
                MethodDescriptor::new(
                    "bordered",
                    vec![ArgumentDescriptor::new("bordered", ArgumentSchema::Boolean)],
                    |arguments| match arguments {
                        [ComponentArgument::Boolean(value)] => {
                            Ok(ComponentPayload::new(ListOp::Bordered(*value)))
                        }
                        _ => Err("DescriptionList.bordered(bordered) expects a boolean".into()),
                    },
                )
                .with_documentation("Controls the horizontal-layout border."),
                MethodDescriptor::new(
                    "columns",
                    vec![ArgumentDescriptor::new("columns", ArgumentSchema::Number)],
                    columns_payload,
                )
                .with_documentation("Sets the column count from 1 through 10."),
                MethodDescriptor::new(
                    "size",
                    vec![ArgumentDescriptor::new(
                        "size",
                        ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
                    )],
                    |a| {
                        size(a, "DescriptionList.size", |value| {
                            ComponentPayload::new(ListOp::Size(value))
                        })
                    },
                )
                .with_documentation("Sets the description-list density."),
            ])
            .with_documentation(
                "A structured label/value list accepting DescriptionItem children.",
            ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn span_rejects_fractional_and_zero_values() {
        assert!(
            positive_usize_payload(
                &[ComponentArgument::Number(0.)],
                "span",
                ComponentPayload::new
            )
            .is_err()
        );
        assert!(
            positive_usize_payload(
                &[ComponentArgument::Number(1.5)],
                "span",
                ComponentPayload::new
            )
            .is_err()
        );
        assert!(
            positive_usize_payload(
                &[ComponentArgument::Number(2.)],
                "span",
                ComponentPayload::new
            )
            .is_ok()
        );
        assert!(
            positive_usize_payload(
                &[ComponentArgument::Number(usize::MAX as f64)],
                "span",
                ComponentPayload::new
            )
            .is_err()
        );
    }

    #[test]
    fn vertical_preserves_operations_recorded_before_and_after_it() {
        let mut config = ListConfig::default();
        for op in [
            ListOp::Bordered(false),
            ListOp::Columns(7),
            ListOp::Vertical,
            ListOp::Size(Size::Large),
        ] {
            config.apply(&op);
        }
        assert_eq!(
            config,
            ListConfig {
                vertical: true,
                bordered: false,
                columns: 7,
                size: Size::Large
            }
        );
    }

    #[test]
    fn columns_rejects_values_the_component_would_otherwise_clamp() {
        assert!(columns_payload(&[ComponentArgument::Number(10.)]).is_ok());
        assert!(columns_payload(&[ComponentArgument::Number(11.)]).is_err());
    }

    #[test]
    fn description_item_explicitly_rejects_children_and_style() {
        assert!(ensure_item_surface(0, &StyleRefinement::default()).is_ok());
        assert_eq!(
            ensure_item_surface(1, &StyleRefinement::default())
                .unwrap_err()
                .to_string(),
            "DescriptionItem does not accept children; use value(string)"
        );

        let mut styled = div().p(gpui::px(1.));
        let style = std::mem::take(styled.style());
        assert!(
            ensure_item_surface(0, &style)
                .unwrap_err()
                .to_string()
                .contains("does not accept style methods")
        );
    }
}
