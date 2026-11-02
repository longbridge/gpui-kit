pub(super) use super::super::support::{
    bool_method, disabled_method, on_click_method, string_method,
};

use gpui_component::Size;
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentPayload, MethodDescriptor,
};

#[derive(Clone)]
pub(super) enum CommonOp {
    Size(Size),
    Label(String),
    Tooltip(String),
    Checked(bool),
    Outline,
    Change(ComponentArgument),
}

/// A `on_change(checked, cx)` method for a two-state control.
///
/// Without one the control is set-only: the script owns `checked`, and a click
/// has nowhere to report to, so the control looks interactive and never
/// changes. Every two-state control in this family needs it, so it is written
/// once.
pub(super) fn change_method(component: &'static str) -> MethodDescriptor {
    MethodDescriptor::new(
        "on_change",
        vec![ArgumentDescriptor::new(
            "on_change",
            ArgumentSchema::Callback("(checked: boolean, cx: Context) => void"),
        )],
        move |arguments| match arguments {
            [argument @ ComponentArgument::Callback(_)] => {
                Ok(ComponentPayload::new(CommonOp::Change(argument.clone())))
            }
            _ => Err(format!("{component}.on_change expects one callback")),
        },
    )
    .with_documentation("Reports the new checked state after a click.")
}

pub(super) fn size_method() -> MethodDescriptor {
    MethodDescriptor::new(
        "size",
        vec![ArgumentDescriptor::new(
            "size",
            ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
        )],
        |arguments| match arguments {
            [ComponentArgument::Enum(value)] => match value.as_str() {
                "xsmall" => Ok(ComponentPayload::new(CommonOp::Size(Size::XSmall))),
                "small" => Ok(ComponentPayload::new(CommonOp::Size(Size::Small))),
                "medium" => Ok(ComponentPayload::new(CommonOp::Size(Size::Medium))),
                "large" => Ok(ComponentPayload::new(CommonOp::Size(Size::Large))),
                _ => Err(format!("unsupported size `{value}`")),
            },
            _ => Err("size expects a semantic size literal".into()),
        },
    )
    .with_documentation("Sets the semantic control size.")
}

pub(super) fn outline_method() -> MethodDescriptor {
    MethodDescriptor::new("outline", Vec::new(), |_| {
        Ok(ComponentPayload::new(CommonOp::Outline))
    })
    .with_documentation("Uses the component's outline presentation.")
}
