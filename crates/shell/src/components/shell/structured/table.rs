use super::Empty;
use super::common::positive_usize;
use super::common::{TypedChildElement, take_element};
use super::require_child;
use gpui_component::{
    Sizable as _, Size,
    table::{
        Table, TableBody, TableCaption, TableCell, TableFooter, TableHead, TableHeader, TableRow,
    },
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, ParentElement as _, Refineable as _, Styled as _},
};
use std::sync::Arc;

#[derive(Clone)]
enum TableOp {
    AccessibilityLabel(String),
    Size(Size),
}
#[derive(Clone, Copy)]
enum CellOp {
    ColSpan(usize),
    Center,
    Right,
}

fn take_typed<T: gpui::IntoElement + 'static>(
    request: &mut MaterializeRequest<'_>,
    parent: &str,
    expected: &'static str,
) -> anyhow::Result<Vec<T>> {
    request
        .take_typed_children()?
        .into_iter()
        .map(|mut child| {
            require_child(parent, child.component_name(), &[expected])?;
            let mut element = request.materialize_child(&mut child)?;
            take_element::<T>(&mut element, expected)
        })
        .collect()
}

macro_rules! container_materializer {
    ($name:ident, $ty:ty, $parent:literal, $child:literal, $child_ty:ty) => {
        struct $name;
        impl ComponentMaterializer for $name {
            fn materialize(
                &self,
                mut request: MaterializeRequest<'_>,
            ) -> anyhow::Result<gpui::AnyElement> {
                request.payload().downcast_ref::<Empty>().ok_or_else(|| {
                    anyhow::anyhow!(concat!($parent, " received an incompatible payload"))
                })?;
                let mut component = <$ty>::new();
                component.style().refine(&request.take_style());
                for child in take_typed::<$child_ty>(&mut request, $parent, $child)? {
                    component = component.child(child);
                }
                Ok(TypedChildElement::new(component).into_any_element())
            }
        }
    };
}
container_materializer!(
    HeaderMaterializer,
    TableHeader,
    "TableHeader",
    "TableRow",
    TableRow
);
container_materializer!(
    BodyMaterializer,
    TableBody,
    "TableBody",
    "TableRow",
    TableRow
);
container_materializer!(
    FooterMaterializer,
    TableFooter,
    "TableFooter",
    "TableRow",
    TableRow
);

struct RowMaterializer;
impl ComponentMaterializer for RowMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<Empty>()
            .ok_or_else(|| anyhow::anyhow!("TableRow received an incompatible payload"))?;
        let children = request.take_typed_children()?;
        let mut row = TableRow::new();
        row.style().refine(&request.take_style());
        for mut child in children {
            let mut element = request.materialize_child(&mut child)?;
            row = match child.component_name() {
                Some("TableHead") => {
                    row.child(take_element::<TableHead>(&mut element, "TableHead")?)
                }
                Some("TableCell") => {
                    row.child(take_element::<TableCell>(&mut element, "TableCell")?)
                }
                actual => {
                    require_child("TableRow", actual, &["TableHead", "TableCell"])?;
                    unreachable!()
                }
            };
        }
        Ok(TypedChildElement::new(row).into_any_element())
    }
}

macro_rules! leaf_materializer {
    ($name:ident, $ty:ty, $label:literal) => {
        struct $name;
        impl ComponentMaterializer for $name {
            fn materialize(
                &self,
                mut request: MaterializeRequest<'_>,
            ) -> anyhow::Result<gpui::AnyElement> {
                request.payload().downcast_ref::<Empty>().ok_or_else(|| {
                    anyhow::anyhow!(concat!($label, " received an incompatible payload"))
                })?;
                let mut component = <$ty>::new();
                for op in request
                    .methods()
                    .filter_map(|method| method.payload().downcast_ref::<CellOp>())
                {
                    component = match op {
                        CellOp::ColSpan(value) => component.col_span(*value),
                        CellOp::Center => component.text_center(),
                        CellOp::Right => component.text_right(),
                    };
                }
                component.style().refine(&request.take_style());
                component.extend(request.take_children()?);
                Ok(TypedChildElement::new(component).into_any_element())
            }
        }
    };
}
leaf_materializer!(HeadMaterializer, TableHead, "TableHead");
leaf_materializer!(CellMaterializer, TableCell, "TableCell");

struct CaptionMaterializer;
impl ComponentMaterializer for CaptionMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<Empty>()
            .ok_or_else(|| anyhow::anyhow!("TableCaption received an incompatible payload"))?;
        let mut caption = TableCaption::new();
        caption.style().refine(&request.take_style());
        caption.extend(request.take_children()?);
        Ok(TypedChildElement::new(caption).into_any_element())
    }
}

struct TableMaterializer;
impl ComponentMaterializer for TableMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<Empty>()
            .ok_or_else(|| anyhow::anyhow!("Table received an incompatible payload"))?;
        let mut table = Table::new();
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<TableOp>())
        {
            table = match op {
                TableOp::AccessibilityLabel(value) => table.accessibility_label(value.clone()),
                TableOp::Size(value) => table.with_size(*value),
            };
        }
        table.style().refine(&request.take_style());
        for mut child in request.take_typed_children()? {
            let mut element = request.materialize_child(&mut child)?;
            table = match child.component_name() {
                Some("TableHeader") => {
                    table.child(take_element::<TableHeader>(&mut element, "TableHeader")?)
                }
                Some("TableBody") => {
                    table.child(take_element::<TableBody>(&mut element, "TableBody")?)
                }
                Some("TableFooter") => {
                    table.child(take_element::<TableFooter>(&mut element, "TableFooter")?)
                }
                Some("TableCaption") => {
                    table.child(take_element::<TableCaption>(&mut element, "TableCaption")?)
                }
                actual => {
                    require_child(
                        "Table",
                        actual,
                        &["TableHeader", "TableBody", "TableFooter", "TableCaption"],
                    )?;
                    unreachable!()
                }
            };
        }
        Ok(table.into_any_element())
    }
}

fn empty_descriptor(
    name: &'static str,
    materializer: Arc<dyn ComponentMaterializer>,
) -> ComponentDescriptor {
    ComponentDescriptor::new(name, materializer)
        .with_constructors(vec![ConstructorDescriptor::new(name, vec![], |_| {
            Ok(ComponentPayload::new(Empty))
        })])
        .with_methods(vec![])
        .with_documentation("A typed structural child in a simple Table.")
}
fn cell_methods() -> Vec<MethodDescriptor> {
    vec![
        MethodDescriptor::new(
            "col_span",
            vec![ArgumentDescriptor::new("span", ArgumentSchema::Number)],
            |arguments| match arguments {
                [ComponentArgument::Number(value)] => cell_span(*value),
                _ => Err("col_span(span) expects a positive integer".into()),
            },
        )
        .with_documentation("Sets the number of columns occupied by the cell."),
        MethodDescriptor::new("text_center", vec![], |_| {
            Ok(ComponentPayload::new(CellOp::Center))
        })
        .with_documentation("Centers the cell content."),
        MethodDescriptor::new("text_right", vec![], |_| {
            Ok(ComponentPayload::new(CellOp::Right))
        })
        .with_documentation("Right-aligns the cell content."),
    ]
}
fn cell_span(value: f64) -> Result<ComponentPayload, String> {
    positive_usize(value, "col_span").map(|value| ComponentPayload::new(CellOp::ColSpan(value)))
}
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(empty_descriptor(
        "TableHeader",
        Arc::new(HeaderMaterializer),
    ))?;
    registry.register(empty_descriptor("TableBody", Arc::new(BodyMaterializer)))?;
    registry.register(empty_descriptor(
        "TableFooter",
        Arc::new(FooterMaterializer),
    ))?;
    registry.register(empty_descriptor("TableRow", Arc::new(RowMaterializer)))?;
    for (name, materializer) in [
        (
            "TableHead",
            Arc::new(HeadMaterializer) as Arc<dyn ComponentMaterializer>,
        ),
        (
            "TableCell",
            Arc::new(CellMaterializer) as Arc<dyn ComponentMaterializer>,
        ),
    ] {
        registry.register(empty_descriptor(name, materializer).with_methods(cell_methods()))?;
    }
    registry.register(empty_descriptor(
        "TableCaption",
        Arc::new(CaptionMaterializer),
    ))?;
    registry.register(
        ComponentDescriptor::new("Table", Arc::new(TableMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new("Table", vec![], |_| {
                Ok(ComponentPayload::new(Empty))
            })])
            .with_methods(vec![
                MethodDescriptor::new(
                    "accessibility_label",
                    vec![ArgumentDescriptor::new("label", ArgumentSchema::String)],
                    |arguments| match arguments {
                        [ComponentArgument::String(value)] => Ok(ComponentPayload::new(
                            TableOp::AccessibilityLabel(value.clone()),
                        )),
                        _ => Err("Table.accessibility_label(label) expects a string".into()),
                    },
                )
                .with_documentation("Sets the table's screen-reader accessible name."),
                MethodDescriptor::new(
                    "size",
                    vec![ArgumentDescriptor::new(
                        "size",
                        ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
                    )],
                    |arguments| match arguments {
                        [ComponentArgument::Enum(value)] => match value.as_str() {
                            "xsmall" => Ok(ComponentPayload::new(TableOp::Size(Size::XSmall))),
                            "small" => Ok(ComponentPayload::new(TableOp::Size(Size::Small))),
                            "medium" => Ok(ComponentPayload::new(TableOp::Size(Size::Medium))),
                            "large" => Ok(ComponentPayload::new(TableOp::Size(Size::Large))),
                            _ => Err(format!("unsupported Table size `{value}`")),
                        },
                        _ => Err("Table.size(size) expects a size literal".into()),
                    },
                )
                .with_documentation(
                    "Sets the table density and propagates it to typed descendants.",
                ),
            ])
            .with_documentation(
                "A simple stateless table composed from typed table-part children.",
            ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_span_rejects_rounded_usize_overflow() {
        assert!(cell_span(usize::MAX as f64).is_err());
    }

    #[test]
    fn typed_table_carriers_hold_real_gpui_component_parts() {
        let mut row = TypedChildElement::new(TableRow::new()).into_any_element();
        take_element::<TableRow>(&mut row, "TableRow").unwrap();
        assert!(take_element::<TableRow>(&mut row, "TableRow").is_err());

        let mut wrong = TypedChildElement::new(TableCell::new()).into_any_element();
        assert!(take_element::<TableRow>(&mut wrong, "TableRow").is_err());
    }
}
