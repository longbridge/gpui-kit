use super::bool_method;
use super::common::{TypedChildElement, nonnegative_f32, positive_u16, take_element};
use super::require_child;
use gpui_component::{
    Sizable as _, Size,
    form::{Field, h_form, v_form},
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, ParentElement as _, Refineable as _, Styled as _, px},
};
use std::sync::Arc;

#[derive(Clone, Copy)]
struct FieldPayload;
#[derive(Clone)]
enum FieldOp {
    Label(String),
    Description(String),
    Required(bool),
    Visible(bool),
    LabelIndent(bool),
    Align(Align),
    ColSpan(u16),
}
#[derive(Clone, Copy)]
enum Align {
    Start,
    Center,
    End,
}
struct FieldMaterializer;
impl ComponentMaterializer for FieldMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<FieldPayload>()
            .ok_or_else(|| anyhow::anyhow!("Field received an incompatible payload"))?;
        let mut field = Field::new();
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<FieldOp>())
        {
            field = match op {
                FieldOp::Label(value) => field.label(value.clone()),
                FieldOp::Description(value) => field.description(value.clone()),
                FieldOp::Required(value) => field.required(*value),
                FieldOp::Visible(value) => field.visible(*value),
                FieldOp::LabelIndent(value) => field.label_indent(*value),
                FieldOp::Align(Align::Start) => field.items_start(),
                FieldOp::Align(Align::Center) => field.items_center(),
                FieldOp::Align(Align::End) => field.items_end(),
                FieldOp::ColSpan(value) => field.col_span(*value),
            };
        }
        field.style().refine(&request.take_style());
        field.extend(request.take_children()?);
        Ok(TypedChildElement::new(field).into_any_element())
    }
}

#[derive(Clone, Copy)]
enum FormPayload {
    Vertical,
    Horizontal,
}
#[derive(Clone, Copy)]
enum FormOp {
    Columns(usize),
    LabelWidth(f32),
    Size(Size),
}
struct FormMaterializer;
impl ComponentMaterializer for FormMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<FormPayload>()
            .ok_or_else(|| anyhow::anyhow!("Form received an incompatible payload"))?;
        let mut form = match payload {
            FormPayload::Vertical => v_form(),
            FormPayload::Horizontal => h_form(),
        };
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<FormOp>())
        {
            form = match op {
                FormOp::Columns(value) => form.columns(*value),
                FormOp::LabelWidth(value) => form.label_width(px(*value)),
                FormOp::Size(value) => form.with_size(*value),
            };
        }
        for mut child in request.take_typed_children()? {
            require_child("Form", child.component_name(), &["Field"])?;
            let mut element = request.materialize_child(&mut child)?;
            form = form.child(take_element::<Field>(&mut element, "Field")?);
        }
        form.style().refine(&request.take_style());
        Ok(form.into_any_element())
    }
}

fn form_columns(a: &[ComponentArgument]) -> Result<ComponentPayload, String> {
    match a {
        [ComponentArgument::Number(value)] => positive_u16(*value, "Form.columns")
            .map(|value| ComponentPayload::new(FormOp::Columns(usize::from(value)))),
        _ => Err("Form.columns expects an exactly representable positive integer".into()),
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("Field", Arc::new(FieldMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new("Field", vec![], |_| {
                Ok(ComponentPayload::new(FieldPayload))
            })])
            .with_methods(vec![
                MethodDescriptor::new(
                    "label",
                    vec![ArgumentDescriptor::new("label", ArgumentSchema::String)],
                    |arguments| match arguments {
                        [ComponentArgument::String(value)] => {
                            Ok(ComponentPayload::new(FieldOp::Label(value.clone())))
                        }
                        _ => Err("Field.label(label) expects a string".into()),
                    },
                )
                .with_documentation("Sets the field label."),
                MethodDescriptor::new(
                    "description",
                    vec![ArgumentDescriptor::new(
                        "description",
                        ArgumentSchema::String,
                    )],
                    |arguments| match arguments {
                        [ComponentArgument::String(value)] => {
                            Ok(ComponentPayload::new(FieldOp::Description(value.clone())))
                        }
                        _ => Err("Field.description(description) expects a string".into()),
                    },
                )
                .with_documentation("Sets supporting text below the control."),
                bool_method(
                    "Field",
                    "required",
                    "Marks the field as required.",
                    FieldOp::Required,
                ),
                bool_method(
                    "Field",
                    "visible",
                    "Controls field visibility.",
                    FieldOp::Visible,
                ),
                bool_method(
                    "Field",
                    "label_indent",
                    "Keeps unlabeled horizontal fields aligned with labeled fields.",
                    FieldOp::LabelIndent,
                ),
                MethodDescriptor::new(
                    "align",
                    vec![ArgumentDescriptor::new(
                        "align",
                        ArgumentSchema::Enum(&["start", "center", "end"]),
                    )],
                    |arguments| match arguments {
                        [ComponentArgument::Enum(value)] => match value.as_str() {
                            "start" => Ok(ComponentPayload::new(FieldOp::Align(Align::Start))),
                            "center" => Ok(ComponentPayload::new(FieldOp::Align(Align::Center))),
                            "end" => Ok(ComponentPayload::new(FieldOp::Align(Align::End))),
                            _ => Err(format!("unsupported Field alignment `{value}`")),
                        },
                        _ => Err("Field.align(align) expects an alignment literal".into()),
                    },
                )
                .with_documentation("Aligns the label and control within the field."),
                MethodDescriptor::new(
                    "col_span",
                    vec![ArgumentDescriptor::new("span", ArgumentSchema::Number)],
                    |arguments| match arguments {
                        [ComponentArgument::Number(value)] => {
                            positive_u16(*value, "Field.col_span")
                                .map(|value| ComponentPayload::new(FieldOp::ColSpan(value)))
                        }
                        _ => Err(
                            "Field.col_span expects an exactly representable positive integer"
                                .into(),
                        ),
                    },
                )
                .with_documentation("Sets the field's grid-column span."),
            ])
            .with_documentation("A typed form field containing ordinary control children."),
    )?;
    registry.register(
        ComponentDescriptor::new("Form", Arc::new(FormMaterializer))
            .with_constructors(vec![
                ConstructorDescriptor::new("Form", vec![], |_| {
                    Ok(ComponentPayload::new(FormPayload::Vertical))
                }),
                ConstructorDescriptor::new("VForm", vec![], |_| {
                    Ok(ComponentPayload::new(FormPayload::Vertical))
                }),
                ConstructorDescriptor::new("HForm", vec![], |_| {
                    Ok(ComponentPayload::new(FormPayload::Horizontal))
                }),
            ])
            .with_methods(vec![
                MethodDescriptor::new(
                    "columns",
                    vec![ArgumentDescriptor::new("columns", ArgumentSchema::Number)],
                    form_columns,
                )
                .with_documentation("Sets the form grid's column count."),
                MethodDescriptor::new(
                    "label_width",
                    vec![ArgumentDescriptor::new("width", ArgumentSchema::Number)],
                    |arguments| match arguments {
                        [ComponentArgument::Number(value)] => {
                            nonnegative_f32(*value, "Form.label_width")
                                .map(|value| ComponentPayload::new(FormOp::LabelWidth(value)))
                        }
                        _ => Err(
                            "Form.label_width(width) expects a nonnegative finite number".into(),
                        ),
                    },
                )
                .with_documentation("Sets the horizontal form label width in pixels."),
                MethodDescriptor::new(
                    "size",
                    vec![ArgumentDescriptor::new(
                        "size",
                        ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
                    )],
                    |arguments| match arguments {
                        [ComponentArgument::Enum(value)] => match value.as_str() {
                            "xsmall" => Ok(ComponentPayload::new(FormOp::Size(Size::XSmall))),
                            "small" => Ok(ComponentPayload::new(FormOp::Size(Size::Small))),
                            "medium" => Ok(ComponentPayload::new(FormOp::Size(Size::Medium))),
                            "large" => Ok(ComponentPayload::new(FormOp::Size(Size::Large))),
                            _ => Err(format!("unsupported Form size `{value}`")),
                        },
                        _ => Err("Form.size(size) expects a size literal".into()),
                    },
                )
                .with_documentation("Sets the form density."),
            ])
            .with_documentation("A vertical or horizontal form accepting Field children."),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn columns_accept_only_positive_integers() {
        assert!(form_columns(&[ComponentArgument::Number(2.)]).is_ok());
        assert!(form_columns(&[ComponentArgument::Number(u16::MAX as f64 + 1.)]).is_err());
        assert!(form_columns(&[ComponentArgument::Number(-1.)]).is_err());
        assert!(form_columns(&[ComponentArgument::Number(1.5)]).is_err());
        assert!(form_columns(&[ComponentArgument::Number(usize::MAX as f64)]).is_err());
    }

    #[test]
    fn field_span_and_label_width_reject_lossy_ranges() {
        assert!(positive_u16(u16::MAX as f64 + 1.0, "Field.col_span").is_err());
        assert!(nonnegative_f32((f32::MAX as f64) * 2.0, "Form.label_width").is_err());
    }
}
