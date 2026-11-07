use gpui_component::{Sizable as _, Size, radio::Radio};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallbackArgument,
    ComponentDescriptor, ComponentMaterializer, ComponentPayload, ComponentRegistry,
    ConstructorDescriptor, MaterializeRequest, MethodDescriptor, RegistryError, anyhow, gpui,
};
use std::sync::Arc;

use super::common::nonempty_id;
#[derive(Clone)]
struct RadioPayload(String);
#[derive(Clone)]
enum RadioOp {
    OnChange(ComponentArgument),
    Label(String),
    A11y(String),
    Checked(bool),
    TabStop(bool),
    Size(Size),
}
struct RadioMaterializer;
impl ComponentMaterializer for RadioMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = &request
            .payload()
            .downcast_ref::<RadioPayload>()
            .ok_or_else(|| anyhow::anyhow!("Radio received an incompatible payload"))?
            .0;
        // A `Radio` inside a `RadioGroup` is driven by the group, which reports
        // the selected index itself. One standing on its own has nothing above
        // it, so it reports its own click.
        let change = request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<RadioOp>())
            .filter_map(|op| match op {
                RadioOp::OnChange(argument) => Some(argument.clone()),
                _ => None,
            })
            .last();
        let mut radio = Radio::new(id.clone())
            .disabled(request.disabled())
            .checked(request.selected());
        for op in request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<RadioOp>())
        {
            radio = match op {
                RadioOp::Label(value) => radio.label(value.clone()),
                RadioOp::A11y(value) => radio.accessibility_label(value.clone()),
                RadioOp::Checked(value) => radio.checked(*value),
                RadioOp::TabStop(value) => radio.tab_stop(*value),
                RadioOp::Size(value) => radio.with_size(*value),
                RadioOp::OnChange(_) => radio,
            }
        }
        if let Some(argument) = change {
            let callback = request.resolve_callback(&argument)?;
            radio = radio.on_click(move |checked, window, cx| {
                callback.invoke_and_report_with(
                    "Radio.on_change callback failed",
                    &[ComponentCallbackArgument::Boolean(*checked)],
                    window,
                    cx,
                );
            });
        }
        crate::components::shell::typed_compound::finish_part(&mut request, radio)
    }
}
fn method(
    name: &'static str,
    schema: ArgumentSchema,
    doc: &'static str,
    f: fn(&ComponentArgument) -> Result<RadioOp, String>,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, schema)],
        move |arguments| match arguments {
            [value] => f(value).map(ComponentPayload::new),
            _ => Err(format!("Radio.{name}({name}) expects one argument")),
        },
    )
    .with_documentation(doc)
}
pub(super) fn register(r: &mut ComponentRegistry) -> Result<(), RegistryError> {
    r.register(
        ComponentDescriptor::new("Radio", Arc::new(RadioMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "Radio",
                vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(id)] => nonempty_id(id, "Radio")
                        .map(RadioPayload)
                        .map(ComponentPayload::new),
                    _ => Err("Radio(id) expects a string id".into()),
                },
            )])
            .with_methods(vec![
                method(
                    "label",
                    ArgumentSchema::String,
                    "Sets the visible label.",
                    |value| match value {
                        ComponentArgument::String(x) => Ok(RadioOp::Label(x.clone())),
                        _ => Err("Radio.label(label) expects a string".into()),
                    },
                ),
                method(
                    "accessibility_label",
                    ArgumentSchema::String,
                    "Overrides the announced name.",
                    |value| match value {
                        ComponentArgument::String(x) => Ok(RadioOp::A11y(x.clone())),
                        _ => Err("Radio.accessibility_label(label) expects a string".into()),
                    },
                ),
                method(
                    "checked",
                    ArgumentSchema::Boolean,
                    "Controls checked state.",
                    |value| match value {
                        ComponentArgument::Boolean(x) => Ok(RadioOp::Checked(*x)),
                        _ => Err("Radio.checked(checked) expects a boolean".into()),
                    },
                ),
                method(
                    "tab_stop",
                    ArgumentSchema::Boolean,
                    "Controls keyboard tab-stop participation.",
                    |value| match value {
                        ComponentArgument::Boolean(x) => Ok(RadioOp::TabStop(*x)),
                        _ => Err("Radio.tab_stop(tabStop) expects a boolean".into()),
                    },
                ),
                method(
                    "size",
                    ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
                    "Sets semantic size.",
                    |value| match value {
                        ComponentArgument::Enum(x) => match x.as_str() {
                            "xsmall" => Ok(RadioOp::Size(Size::XSmall)),
                            "small" => Ok(RadioOp::Size(Size::Small)),
                            "medium" => Ok(RadioOp::Size(Size::Medium)),
                            "large" => Ok(RadioOp::Size(Size::Large)),
                            _ => Err(format!("unsupported Radio size `{x}`")),
                        },
                        _ => Err("Radio.size(size) expects a size literal".into()),
                    },
                ),
                MethodDescriptor::new(
                    "on_change",
                    vec![ArgumentDescriptor::new(
                        "on_change",
                        ArgumentSchema::Callback("(checked: boolean, cx: Context) => void"),
                    )],
                    |arguments| match arguments {
                        [argument @ ComponentArgument::Callback(_)] => {
                            Ok(ComponentPayload::new(RadioOp::OnChange(argument.clone())))
                        }
                        _ => Err("Radio.on_change expects one callback".into()),
                    },
                )
                .with_documentation(
                    "Reports a click on a radio used on its own. Inside a `RadioGroup` the \
                     group reports the selected index instead.",
                ),
            ])
            .with_documentation(
                "A controlled radio control; selected and disabled common behavior is supported.",
            ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_rejects_empty_and_whitespace_only_values() {
        assert!(nonempty_id("", "Radio").is_err());
        assert!(nonempty_id(" \t ", "Radio").is_err());
        assert_eq!(nonempty_id("choice-a", "Radio").unwrap(), "choice-a");
    }
}
