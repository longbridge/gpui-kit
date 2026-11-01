use gpui_component::TitleBar;
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{self, IntoElement as _, ParentElement as _, Refineable as _, Styled as _},
};
use std::sync::Arc;
#[derive(Clone)]
struct TitleBarPayload;
#[derive(Clone)]
enum TitleBarOp {
    OnCloseWindow(ComponentArgument),
}
struct TitleBarMaterializer;
impl TitleBarMaterializer {
    fn component(payload: &ComponentPayload) -> anyhow::Result<TitleBar> {
        payload
            .downcast_ref::<TitleBarPayload>()
            .ok_or_else(|| anyhow::anyhow!("TitleBar received an incompatible payload"))?;
        Ok(TitleBar::new())
    }
}
impl ComponentMaterializer for TitleBarMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let mut component = Self::component(request.payload())?;
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<TitleBarOp>().cloned())
            .collect::<Vec<_>>();
        for operation in operations {
            component = match operation {
                TitleBarOp::OnCloseWindow(argument) => {
                    let callback = request.resolve_callback(&argument)?;
                    component.on_close_window(move |_, window, cx| {
                        callback.invoke_and_report_with(
                            "TitleBar.on_close_window callback failed",
                            &[],
                            window,
                            cx,
                        );
                    })
                }
            };
        }
        component.style().refine(&request.take_style());
        component.extend(request.take_children()?);
        Ok(component.into_any_element())
    }
}
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(
        ComponentDescriptor::new("TitleBar", Arc::new(TitleBarMaterializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "TitleBar",
                Vec::new(),
                |_| Ok(ComponentPayload::new(TitleBarPayload)),
            )])
            .with_methods(vec![
                MethodDescriptor::new(
                    "on_close_window",
                    vec![ArgumentDescriptor::new(
                        "callback",
                        ArgumentSchema::Callback("(cx: Context) => void"),
                    )],
                    |arguments| match arguments {
                        [argument @ ComponentArgument::Callback(_)] => Ok(ComponentPayload::new(
                            TitleBarOp::OnCloseWindow(argument.clone()),
                        )),
                        _ => Err("TitleBar.on_close_window(callback) expects one callback".into()),
                    },
                )
                .with_documentation(
                    "Runs the callback instead of closing the window when the title bar's own close \
                     button is clicked. Only Linux draws that button; elsewhere the system owns closing.",
                ),
            ])
            .with_documentation(
                "The window title bar, rendered as the first child of the application's root view. \
                 It draws the window controls, drags and zooms the window, and lays its children out \
                 between the leading edge and the controls; stop mouse-down propagation on \
                 interactive children so pressing them does not start a window move.",
            ),
    )?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_real_title_bar() {
        drop(
            TitleBarMaterializer::component(&ComponentPayload::new(TitleBarPayload))
                .unwrap()
                .into_any_element(),
        );
    }

    #[test]
    fn rejects_an_incompatible_payload() {
        assert_eq!(
            TitleBarMaterializer::component(&ComponentPayload::new(()))
                .err()
                .unwrap()
                .to_string(),
            "TitleBar received an incompatible payload"
        );
    }
}
