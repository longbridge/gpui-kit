use std::sync::Arc;

use gpui_component::{
    InteractiveElementExt as _,
    scroll::{Scrollbar, ScrollbarAxis, ScrollbarMode},
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, StateDescriptor, anyhow,
    gpui::{
        self, InteractiveElement as _, IntoElement as _, ParentElement as _, Refineable as _,
        StatefulInteractiveElement as _, Styled as _,
    },
};

#[derive(Clone)]
struct Payload {
    id: Option<String>,
    state: ComponentArgument,
}

#[derive(Clone, Copy)]
enum Op {
    Axis(ScrollbarAxis),
    Mode(ScrollbarMode),
    ViewportFromLayout(bool),
}

fn resolve_ops<'a>(
    ops: impl Iterator<Item = &'a Op>,
) -> (Option<ScrollbarAxis>, Option<ScrollbarMode>, bool) {
    let mut axis = None;
    let mut mode = None;
    let mut viewport_from_layout = false;
    for op in ops {
        match op {
            Op::Axis(value) => axis = Some(*value),
            Op::Mode(value) => mode = Some(*value),
            Op::ViewportFromLayout(value) => viewport_from_layout = *value,
        }
    }
    (axis, mode, viewport_from_layout)
}

fn axis_method() -> MethodDescriptor {
    MethodDescriptor::new(
        // Not `axis`: the runtime's element prototype checks that name against
        // `horizontal | vertical` before the call can reach a registered
        // component, so `both` — which this surface does support — would be
        // refused there and never arrive.
        "scroll_axis",
        vec![ArgumentDescriptor::new(
            "axis",
            ArgumentSchema::Enum(&["vertical", "horizontal", "both"]),
        )],
        |args| match args {
            [ComponentArgument::Enum(value)] => match value.as_str() {
                "vertical" => Ok(ComponentPayload::new(Op::Axis(ScrollbarAxis::Vertical))),
                "horizontal" => Ok(ComponentPayload::new(Op::Axis(ScrollbarAxis::Horizontal))),
                "both" => Ok(ComponentPayload::new(Op::Axis(ScrollbarAxis::Both))),
                _ => Err("axis expects vertical, horizontal, or both".into()),
            },
            _ => Err("axis expects one enum string".into()),
        },
    )
    .with_documentation("Selects the native scroll axes.")
}

fn require_leaf(children: usize, style: &gpui::StyleRefinement) -> anyhow::Result<()> {
    if children != 0 {
        #[cfg(test)]
        test_probe::error("Scrollbar does not accept children");
        anyhow::bail!("Scrollbar does not accept children");
    }
    if style != &gpui::StyleRefinement::default() {
        #[cfg(test)]
        test_probe::error("Scrollbar is a low-level Element and does not support shell style");
        anyhow::bail!("Scrollbar is a low-level Element and does not support shell style");
    }
    Ok(())
}

struct ScrollMaterializer;

impl ComponentMaterializer for ScrollMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<Payload>()
            .ok_or_else(|| anyhow::anyhow!("Scroll received an incompatible payload"))?;
        let handle = request.with_state::<gpui::ScrollHandle, _>(&payload.state, Clone::clone)?;
        #[cfg(test)]
        test_probe::handle(&handle);
        let (axis, _, _) = resolve_ops(
            request
                .methods()
                .filter_map(|method| method.payload().downcast_ref::<Op>()),
        );
        let axis = axis.unwrap_or(ScrollbarAxis::Vertical);
        let state_handle = match &payload.state {
            ComponentArgument::Entity { handle, .. } => *handle,
            _ => anyhow::bail!("Scroll expects a ScrollbarHandle entity"),
        };
        let mut area = gpui::div()
            .id(("shell-scroll", state_handle))
            .flex()
            .track_scroll(&handle)
            .lock_scroll_axis();
        area = match axis {
            ScrollbarAxis::Vertical => area.flex_col().overflow_y_scroll(),
            ScrollbarAxis::Horizontal => area.flex_row().overflow_x_scroll(),
            ScrollbarAxis::Both => area.overflow_scroll(),
        };
        area.extend(request.take_children()?);
        area.style().refine(&request.take_style());
        Ok(area.into_any_element())
    }
}

struct ScrollbarMaterializer;

impl ComponentMaterializer for ScrollbarMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<Payload>()
            .ok_or_else(|| anyhow::anyhow!("Scrollbar received an incompatible payload"))?;
        let id = payload
            .id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Scrollbar requires a stable id"))?;
        let handle = request.with_state::<gpui::ScrollHandle, _>(&payload.state, Clone::clone)?;
        #[cfg(test)]
        test_probe::handle(&handle);
        let (axis, mode, viewport_from_layout) = resolve_ops(
            request
                .methods()
                .filter_map(|method| method.payload().downcast_ref::<Op>()),
        );
        let mut scrollbar = Scrollbar::new(&handle).id(id.clone());
        if let Some(axis) = axis {
            scrollbar = scrollbar.axis(axis);
        }
        if let Some(mode) = mode {
            scrollbar = scrollbar.mode(mode);
        }
        if viewport_from_layout {
            scrollbar = scrollbar.viewport_from_layout();
        }
        let style = request.take_style();
        require_leaf(request.children_len(), &style)?;
        Ok(scrollbar.into_any_element())
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register_state(
        StateDescriptor::new("ScrollbarHandle", "ScrollbarHandle", vec![], |_, _, _| {
            Ok(Box::new(gpui::ScrollHandle::default()))
        })
        .with_documentation("A retained native scroll capability owned by one Scroll viewport and shared with any Scrollbar elements that control it."),
    )?;
    registry.register(ComponentDescriptor::new("Scroll", Arc::new(ScrollMaterializer))
.with_constructors(vec![ConstructorDescriptor::new(
            "Scroll",
            vec![ArgumentDescriptor::new(
                "handle",
                ArgumentSchema::Entity("ScrollbarHandle"),
            )],
            |args| match args {
                [state @ ComponentArgument::Entity { .. }] => Ok(ComponentPayload::new(Payload {
                    id: None,
                    state: state.clone(),
                })),
                _ => Err("Scroll expects one ScrollbarHandle entity".into()),
            },
        )])
.with_methods(vec![axis_method()])
.with_documentation(
            "Adapter wrapper for ScrollableElement overflow behavior. It shares a retained native handle, accepts ordinary children and shell style, and defaults to vertical scrolling.",
        ))?;
    registry.register(ComponentDescriptor::new("Scrollbar", Arc::new(ScrollbarMaterializer))
.with_constructors(vec![ConstructorDescriptor::new(
            "Scrollbar",
            vec![
                ArgumentDescriptor::new("id", ArgumentSchema::String),
                ArgumentDescriptor::new("handle", ArgumentSchema::Entity("ScrollbarHandle")),
            ],
            |args| match args {
                [ComponentArgument::String(id), state @ ComponentArgument::Entity { .. }]
                    if !id.trim().is_empty() =>
                {
                    Ok(ComponentPayload::new(Payload {
                        id: Some(id.clone()),
                        state: state.clone(),
                    }))
                }
                _ => Err("Scrollbar expects a non-empty window-unique id and ScrollbarHandle".into()),
            },
        )])
.with_methods(vec![
            axis_method(),
            MethodDescriptor::new(
                "mode",
                vec![ArgumentDescriptor::new(
                    "mode",
                    ArgumentSchema::Enum(&["scrolling", "hover", "always"]),
                )],
                |args| match args {
                    [ComponentArgument::Enum(value)] => match value.as_str() {
                        "scrolling" => Ok(ComponentPayload::new(Op::Mode(ScrollbarMode::Scrolling))),
                        "hover" => Ok(ComponentPayload::new(Op::Mode(ScrollbarMode::Hover))),
                        "always" => Ok(ComponentPayload::new(Op::Mode(ScrollbarMode::Always))),
                        _ => Err("Scrollbar.mode expects scrolling, hover, or always".into()),
                    },
                    _ => Err("Scrollbar.mode expects one enum string".into()),
                },
            )
            .with_documentation("Sets the native scrollbar visibility policy."),
            MethodDescriptor::new(
                "viewport_from_layout",
                vec![ArgumentDescriptor::new("enabled", ArgumentSchema::Boolean)],
                |args| match args {
                    [ComponentArgument::Boolean(value)] => {
                        Ok(ComponentPayload::new(Op::ViewportFromLayout(*value)))
                    }
                    _ => Err("Scrollbar.viewport_from_layout expects boolean".into()),
                },
            )
            .with_documentation("Uses this element's layout bounds as the native viewport."),
        ])
.with_documentation(
            "The real native low-level Scrollbar sharing a retained handle. Its stable id must be window-unique; children and generic shell style are rejected.",
        ))?;
    Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_probe {
    use std::cell::RefCell;

    use gpui_shell::gpui;

    thread_local! {
        static HANDLE: RefCell<Option<gpui::ScrollHandle>> = const { RefCell::new(None) };
        static ERRORS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }
    pub(super) fn handle(handle: &gpui::ScrollHandle) {
        HANDLE.with(|slot| *slot.borrow_mut() = Some(handle.clone()));
    }
    pub(super) fn error(error: &str) {
        ERRORS.with(|errors| errors.borrow_mut().push(error.to_owned()));
    }
    pub(crate) fn latest_handle() -> gpui::ScrollHandle {
        HANDLE.with(|slot| {
            slot.borrow()
                .clone()
                .expect("scroll handle was materialized")
        })
    }
    pub(crate) fn take_errors() -> Vec<String> {
        ERRORS.with(|errors| std::mem::take(&mut *errors.borrow_mut()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrollbar_leaf_contract_is_exact() {
        assert!(require_leaf(0, &gpui::StyleRefinement::default()).is_ok());
        assert!(require_leaf(1, &gpui::StyleRefinement::default()).is_err());
        assert!(require_leaf(0, &gpui::StyleRefinement::default().p(gpui::px(2.))).is_err());
    }

    #[test]
    fn repeated_configuration_is_last_call_wins() {
        let ops = [
            Op::Axis(ScrollbarAxis::Horizontal),
            Op::Mode(ScrollbarMode::Hover),
            Op::ViewportFromLayout(true),
            Op::Axis(ScrollbarAxis::Vertical),
            Op::Mode(ScrollbarMode::Always),
            Op::ViewportFromLayout(false),
        ];
        let (axis, mode, viewport) = resolve_ops(ops.iter());
        assert_eq!(axis, Some(ScrollbarAxis::Vertical));
        assert_eq!(mode, Some(ScrollbarMode::Always));
        assert!(!viewport);
    }
}
