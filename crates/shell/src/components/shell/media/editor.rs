use super::bool_method;
use std::sync::Arc;

use gpui_component::input::{Editor, EditorState};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, StateDescriptor, anyhow,
    gpui::{self, AppContext as _, Entity, IntoElement as _, Refineable as _, Styled as _},
};

#[derive(Clone)]
enum Op {
    Appearance(bool),
    Bordered(bool),
    Readonly(bool),
    AriaLabel(String),
}

fn require_leaf(children: usize) -> anyhow::Result<()> {
    anyhow::ensure!(children == 0, "Editor does not accept children");
    Ok(())
}

struct Materializer;
impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let argument = request
            .payload()
            .downcast_ref::<ComponentArgument>()
            .ok_or_else(|| anyhow::anyhow!("Editor received an incompatible payload"))?;
        let state = request.with_state::<Entity<EditorState>, _>(argument, Clone::clone)?;
        let mut editor = Editor::new(&state).disabled(request.disabled());
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<Op>())
        {
            editor = match op {
                Op::Appearance(value) => editor.appearance(*value),
                Op::Bordered(value) => editor.bordered(*value),
                Op::Readonly(value) => editor.readonly(*value),
                Op::AriaLabel(value) => editor.aria_label(value.clone()),
            };
        }
        require_leaf(request.children_len())?;
        editor.style().refine(&request.take_style());
        Ok(editor.into_any_element())
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register_state(
        StateDescriptor::new(
            "EditorState",
            "EditorState",
            vec![
                ArgumentDescriptor::new("initial_value", ArgumentSchema::String),
                ArgumentDescriptor::new(
                    "language",
                    ArgumentSchema::Optional(Box::new(ArgumentSchema::Enum(&["rust", "json"]))),
                ),
            ],
            |args, window, cx| match args {
                [
                    ComponentArgument::String(value),
                    ComponentArgument::Optional(language),
                ] => {
                    let language = if let Some(language) = language {
                        let ComponentArgument::Enum(language) = language.as_ref() else {
                            return Err("EditorState language expects rust or json".into());
                        };
                        Some(language.clone())
                    } else {
                        None
                    };
                    Ok(Box::new(cx.new(|cx| {
                        let state = EditorState::new(window, cx).default_value(value.clone());
                        if let Some(language) = language {
                            state.language(language)
                        } else {
                            state
                        }
                    })) as _)
                }
                _ => Err(
                    "EditorState expects initial text and optional rust or json language".into(),
                ),
            },
        )
        .with_documentation(
            "Retained source-editor state initialized with local text and a syntax language.",
        ),
    )?;
    registry.register(ComponentDescriptor::new("Editor", Arc::new(Materializer))
.with_constructors(vec![ConstructorDescriptor::new(
            "Editor",
            vec![ArgumentDescriptor::new("state", ArgumentSchema::Entity("EditorState"))],
            |args| match args {
                [argument @ ComponentArgument::Entity { .. }] => Ok(ComponentPayload::new(argument.clone())),
                _ => Err("Editor expects one EditorState entity".into()),
            },
        )])
.with_methods(vec![
            MethodDescriptor::new("disabled", vec![ArgumentDescriptor::new("disabled", ArgumentSchema::Boolean)], |_| Ok(ComponentPayload::new(()))).with_documentation("Disables the editor."),
            bool_method("Editor", "appearance", "Controls the editor appearance.", Op::Appearance),
            bool_method("Editor", "bordered", "Controls the editor border.", Op::Bordered),
            bool_method("Editor", "readonly", "Controls read-only mode.", Op::Readonly),
            MethodDescriptor::new(
                "aria_label",
                vec![ArgumentDescriptor::new("label", ArgumentSchema::String)],
                |args| match args {
                    [ComponentArgument::String(value)] if !value.trim().is_empty() => Ok(ComponentPayload::new(Op::AriaLabel(value.clone()))),
                    _ => Err("Editor.aria_label expects non-empty text".into()),
                },
            ).with_documentation("Sets the editor accessibility label."),
        ])
.with_documentation(
            "A retained native source editor. Shell style and common disabled state are honored; children are rejected.",
        ))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_is_an_exact_leaf() {
        assert!(require_leaf(0).is_ok());
        assert_eq!(
            require_leaf(1).unwrap_err().to_string(),
            "Editor does not accept children"
        );
    }
}
