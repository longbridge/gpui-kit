use std::{cell::RefCell, rc::Rc, sync::Arc};

use gpui_component::{
    FocusableExt as _, Sizable as _, Size,
    carousel::{
        Carousel, CarouselContent, CarouselEvent, CarouselItem, CarouselNext, CarouselPagination,
        CarouselPaginationItem, CarouselPrevious, CarouselState,
    },
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallback,
    ComponentCallbackArgument, ComponentDescriptor, ComponentMaterializer, ComponentPayload,
    ComponentRegistry, ConstructorDescriptor, MaterializeRequest, MethodDescriptor, RegistryError,
    StateDescriptor, anyhow,
    gpui::{
        self, App, AppContext as _, Axis, Entity, IntoElement as _, RenderOnce, Subscription,
        Window,
    },
};

use super::{
    support::{bool_method, string_method},
    typed_compound::{finish_part, finish_typed_children},
};

#[derive(Clone)]
enum Payload {
    Root {
        id: String,
        state: ComponentArgument,
    },
    Content {
        state: ComponentArgument,
    },
    Item {
        id: String,
        index: usize,
        state: ComponentArgument,
    },
    Previous {
        state: ComponentArgument,
    },
    Next {
        state: ComponentArgument,
    },
    Pagination,
    PaginationItem {
        id: String,
        index: usize,
        state: ComponentArgument,
    },
}

#[derive(Clone)]
enum Op {
    AccessibilityLabel(String),
    FocusRing(bool),
    Size(Size),
    SelectedIndex(usize),
    ItemCount(usize),
    OnChange(ComponentArgument),
}

struct Materializer;

impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<Payload>()
            .ok_or_else(|| anyhow::anyhow!("Carousel component received an incompatible payload"))?
            .clone();
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<Op>().cloned())
            .collect::<Vec<_>>();

        match payload {
            Payload::Root { id, state } => {
                let state = request.with_state::<Entity<CarouselState>, _>(&state, Clone::clone)?;
                let callback = operations
                    .iter()
                    .filter_map(|operation| match operation {
                        Op::OnChange(argument) => Some(argument),
                        _ => None,
                    })
                    .next_back()
                    .map(|argument| request.resolve_callback(argument))
                    .transpose()?;
                let selected_index = operations
                    .iter()
                    .filter_map(|operation| match operation {
                        Op::SelectedIndex(index) => Some(*index),
                        _ => None,
                    })
                    .next_back();
                let item_count = operations
                    .iter()
                    .filter_map(|operation| match operation {
                        Op::ItemCount(count) => Some(*count),
                        _ => None,
                    })
                    .next_back();
                let mut carousel = Carousel::new(id.clone(), &state);
                for operation in operations {
                    carousel = match operation {
                        Op::AccessibilityLabel(value) => carousel.accessibility_label(value),
                        Op::FocusRing(value) => carousel.focus_ring(value),
                        Op::Size(_) | Op::SelectedIndex(_) | Op::ItemCount(_) | Op::OnChange(_) => {
                            carousel
                        }
                    };
                }
                let child = finish_typed_children(
                    &mut request,
                    carousel,
                    "Carousel",
                    &[
                        "CarouselContent",
                        "CarouselPrevious",
                        "CarouselNext",
                        "CarouselPagination",
                    ],
                )?;
                Ok(BoundCarousel {
                    id,
                    state,
                    callback,
                    selected_index,
                    item_count,
                    child,
                }
                .into_any_element())
            }
            Payload::Content { state } => {
                let state = request.with_state::<Entity<CarouselState>, _>(&state, Clone::clone)?;
                finish_typed_children(
                    &mut request,
                    CarouselContent::new(&state),
                    "CarouselContent",
                    &["CarouselItem"],
                )
            }
            Payload::Item { id, index, state } => {
                let state = request.with_state::<Entity<CarouselState>, _>(&state, Clone::clone)?;
                let mut item = CarouselItem::new(id, index, &state);
                for operation in operations {
                    if let Op::AccessibilityLabel(value) = operation {
                        item = item.accessibility_label(value);
                    }
                }
                finish_part(&mut request, item)
            }
            Payload::Previous { state } => {
                let state = request.with_state::<Entity<CarouselState>, _>(&state, Clone::clone)?;
                let mut previous = CarouselPrevious::new(&state);
                for operation in operations {
                    previous = match operation {
                        Op::AccessibilityLabel(value) => previous.accessibility_label(value),
                        Op::Size(value) => previous.with_size(value),
                        Op::FocusRing(_)
                        | Op::SelectedIndex(_)
                        | Op::ItemCount(_)
                        | Op::OnChange(_) => previous,
                    };
                }
                finish_part(&mut request, previous)
            }
            Payload::Next { state } => {
                let state = request.with_state::<Entity<CarouselState>, _>(&state, Clone::clone)?;
                let mut next = CarouselNext::new(&state);
                for operation in operations {
                    next = match operation {
                        Op::AccessibilityLabel(value) => next.accessibility_label(value),
                        Op::Size(value) => next.with_size(value),
                        Op::FocusRing(_)
                        | Op::SelectedIndex(_)
                        | Op::ItemCount(_)
                        | Op::OnChange(_) => next,
                    };
                }
                finish_part(&mut request, next)
            }
            Payload::Pagination => {
                let mut pagination = CarouselPagination::new();
                for operation in operations {
                    if let Op::AccessibilityLabel(value) = operation {
                        pagination = pagination.accessibility_label(value);
                    }
                }
                finish_typed_children(
                    &mut request,
                    pagination,
                    "CarouselPagination",
                    &["CarouselPaginationItem"],
                )
            }
            Payload::PaginationItem { id, index, state } => {
                let state = request.with_state::<Entity<CarouselState>, _>(&state, Clone::clone)?;
                let mut item = CarouselPaginationItem::new(id, index, &state);
                for operation in operations {
                    item = match operation {
                        Op::AccessibilityLabel(value) => item.accessibility_label(value),
                        Op::Size(value) => item.with_size(value),
                        Op::FocusRing(_)
                        | Op::SelectedIndex(_)
                        | Op::ItemCount(_)
                        | Op::OnChange(_) => item,
                    };
                }
                finish_part(&mut request, item)
            }
        }
    }
}

struct ChangeHost {
    callback: Rc<RefCell<Option<ComponentCallback>>>,
    _change: Subscription,
}

#[derive(gpui::IntoElement)]
struct BoundCarousel {
    id: String,
    state: Entity<CarouselState>,
    callback: Option<ComponentCallback>,
    selected_index: Option<usize>,
    item_count: Option<usize>,
    child: gpui::AnyElement,
}

impl RenderOnce for BoundCarousel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl gpui::IntoElement {
        // A script that passes `item_count` or `selected_index` owns that
        // value, so every frame reasserts it. Both setters are silent and only
        // run when the value actually differs, which keeps a gesture in
        // progress from being cancelled and cannot loop through `on_change`.
        if let Some(count) = self.item_count.filter(|count| {
            let state = self.state.read(cx);
            state.item_count() != *count
        }) {
            self.state
                .update(cx, |state, cx| state.set_item_count(count, cx));
        }
        if let Some(index) = self.selected_index.filter(|index| {
            let state = self.state.read(cx);
            state.item_count() > 0 && state.selected_index() != Some(*index)
        }) {
            self.state
                .update(cx, |state, cx| state.set_selected_index(index, cx));
        }

        let initial_callback = self.callback.clone();
        let state = self.state.clone();
        let host: Entity<ChangeHost> = window.use_keyed_state(
            format!("shell-carousel:{}:{:?}", self.id, self.state.entity_id()),
            cx,
            move |window, cx| {
                let callback = Rc::new(RefCell::new(initial_callback));
                let event_callback = callback.clone();
                let change =
                    window.subscribe(&state, cx, move |_, event: &CarouselEvent, window, cx| {
                        let CarouselEvent::Change(index) = event;
                        let callback = event_callback.borrow().clone();
                        if let Some(callback) = callback {
                            callback.invoke_and_report_with(
                                "Carousel.on_change callback failed",
                                &[ComponentCallbackArgument::Number(*index as f64)],
                                window,
                                cx,
                            );
                        }
                    });
                ChangeHost {
                    callback,
                    _change: change,
                }
            },
        );
        *host.read(cx).callback.borrow_mut() = self.callback;
        self.child
    }
}

fn nonnegative_usize(argument: &ComponentArgument, callable: &str) -> Result<usize, String> {
    match argument {
        ComponentArgument::Number(value)
            if value.is_finite()
                && *value >= 0.
                && value.fract() == 0.
                // `usize::MAX as f64` rounds up to 2^64 on 64-bit targets.
                && *value < usize::MAX as f64 =>
        {
            Ok(*value as usize)
        }
        _ => Err(format!("{callable} expects a nonnegative integer")),
    }
}

fn nonempty_id(argument: &ComponentArgument, callable: &str) -> Result<String, String> {
    match argument {
        ComponentArgument::String(value) if !value.trim().is_empty() => Ok(value.clone()),
        _ => Err(format!("{callable} expects a nonempty string id")),
    }
}

fn entity(argument: &ComponentArgument, callable: &str) -> Result<ComponentArgument, String> {
    match argument {
        argument @ ComponentArgument::Entity { .. } => Ok(argument.clone()),
        _ => Err(format!("{callable} expects a CarouselState entity")),
    }
}

fn state_constructor(
    component: &'static str,
    payload: impl Fn(ComponentArgument) -> Payload + Send + Sync + 'static,
) -> ConstructorDescriptor {
    ConstructorDescriptor::new(
        component,
        vec![ArgumentDescriptor::new(
            "state",
            ArgumentSchema::Entity("CarouselState"),
        )],
        move |arguments| match arguments {
            [state] => entity(state, component)
                .map(&payload)
                .map(ComponentPayload::new),
            _ => Err(format!(
                "{component}(state) expects one CarouselState entity"
            )),
        },
    )
}

fn id_state_constructor(
    component: &'static str,
    payload: impl Fn(String, ComponentArgument) -> Payload + Send + Sync + 'static,
) -> ConstructorDescriptor {
    ConstructorDescriptor::new(
        component,
        vec![
            ArgumentDescriptor::new("id", ArgumentSchema::String),
            ArgumentDescriptor::new("state", ArgumentSchema::Entity("CarouselState")),
        ],
        move |arguments| match arguments {
            [id, state] => Ok(ComponentPayload::new(payload(
                nonempty_id(id, component)?,
                entity(state, component)?,
            ))),
            _ => Err(format!(
                "{component}(id, state) expects a nonempty id and CarouselState entity"
            )),
        },
    )
}

fn indexed_state_constructor(
    component: &'static str,
    payload: impl Fn(String, usize, ComponentArgument) -> Payload + Send + Sync + 'static,
) -> ConstructorDescriptor {
    ConstructorDescriptor::new(
        component,
        vec![
            ArgumentDescriptor::new("id", ArgumentSchema::String),
            ArgumentDescriptor::new("index", ArgumentSchema::Number),
            ArgumentDescriptor::new("state", ArgumentSchema::Entity("CarouselState")),
        ],
        move |arguments| match arguments {
            [id, index, state] => Ok(ComponentPayload::new(payload(
                nonempty_id(id, component)?,
                nonnegative_usize(index, &format!("{component}(id, index, state)"))?,
                entity(state, component)?,
            ))),
            _ => Err(format!(
                "{component}(id, index, state) expects an id, nonnegative index, and CarouselState"
            )),
        },
    )
}

fn accessibility_label_method(component: &'static str) -> MethodDescriptor {
    string_method(
        component,
        "accessibility_label",
        "Sets the name announced by accessibility clients.",
        Op::AccessibilityLabel,
    )
}

fn usize_method(
    component: &'static str,
    name: &'static str,
    documentation: &'static str,
    make: fn(usize) -> Op,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new("value", ArgumentSchema::Number)],
        move |arguments| match arguments {
            [argument] => Ok(ComponentPayload::new(make(nonnegative_usize(
                argument,
                &format!("{component}.{name}(value)"),
            )?))),
            _ => Err(format!("{component}.{name}(value) expects one number")),
        },
    )
    .with_documentation(documentation)
}

fn size_method(component: &'static str) -> MethodDescriptor {
    MethodDescriptor::new(
        "size",
        vec![ArgumentDescriptor::new(
            "size",
            ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
        )],
        move |arguments| match arguments {
            [ComponentArgument::Enum(value)] => match value.as_str() {
                "xsmall" => Ok(ComponentPayload::new(Op::Size(Size::XSmall))),
                "small" => Ok(ComponentPayload::new(Op::Size(Size::Small))),
                "medium" => Ok(ComponentPayload::new(Op::Size(Size::Medium))),
                "large" => Ok(ComponentPayload::new(Op::Size(Size::Large))),
                _ => Err(format!("unsupported {component} size `{value}`")),
            },
            _ => Err(format!("{component}.size(size) expects a semantic size")),
        },
    )
    .with_documentation("Sets the semantic control size.")
}

fn descriptor(
    name: &'static str,
    constructor: ConstructorDescriptor,
    methods: Vec<MethodDescriptor>,
    documentation: &'static str,
) -> ComponentDescriptor {
    ComponentDescriptor::new(name, Arc::new(Materializer))
        .with_constructors(vec![constructor])
        .with_methods(methods)
        .with_documentation(documentation)
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register_state(
        StateDescriptor::new(
            "CarouselState",
            "CarouselState",
            vec![
                ArgumentDescriptor::new("item_count", ArgumentSchema::Number),
                ArgumentDescriptor::new(
                    "selected_index",
                    ArgumentSchema::Optional(Box::new(ArgumentSchema::Number)),
                ),
                ArgumentDescriptor::new(
                    "axis",
                    ArgumentSchema::Optional(Box::new(ArgumentSchema::Enum(&[
                        "horizontal",
                        "vertical",
                    ]))),
                ),
                ArgumentDescriptor::new(
                    "looping",
                    ArgumentSchema::Optional(Box::new(ArgumentSchema::Boolean)),
                ),
            ],
            |arguments, _, cx| {
                let [item_count, selected_index, axis, looping] = arguments else {
                    return Err(
                        "CarouselState expects item_count, selected_index, axis, and looping"
                            .into(),
                    );
                };
                let item_count = nonnegative_usize(item_count, "CarouselState(item_count)")?;
                let selected_index = match selected_index {
                    ComponentArgument::Optional(Some(index)) => {
                        Some(nonnegative_usize(index, "CarouselState selected_index")?)
                    }
                    ComponentArgument::Optional(None) => None,
                    _ => return Err("CarouselState selected_index must be optional".into()),
                };
                if selected_index.is_some_and(|index| index >= item_count) {
                    return Err("CarouselState selected_index must be within item_count".into());
                }
                let axis = match axis {
                    ComponentArgument::Optional(Some(axis)) => match axis.as_ref() {
                        ComponentArgument::Enum(axis) if axis == "horizontal" => Axis::Horizontal,
                        ComponentArgument::Enum(axis) if axis == "vertical" => Axis::Vertical,
                        _ => return Err("CarouselState axis expects horizontal or vertical".into()),
                    },
                    ComponentArgument::Optional(None) => Axis::Horizontal,
                    _ => return Err("CarouselState axis must be optional".into()),
                };
                let looping = match looping {
                    ComponentArgument::Optional(Some(looping)) => match looping.as_ref() {
                        ComponentArgument::Boolean(looping) => *looping,
                        _ => return Err("CarouselState looping expects a boolean".into()),
                    },
                    ComponentArgument::Optional(None) => false,
                    _ => return Err("CarouselState looping must be optional".into()),
                };
                Ok(Box::new(cx.new(|_| {
                    let mut state = CarouselState::new(item_count)
                        .with_axis(axis)
                        .with_looping(looping);
                    if let Some(index) = selected_index {
                        state = state.with_selected_index(index);
                    }
                    state
                })))
            },
        )
        .with_documentation("Retained Carousel selection, axis, looping, and interaction state."),
    )?;

    registry.register(descriptor(
        "Carousel",
        id_state_constructor("Carousel", |id, state| Payload::Root { id, state }),
        vec![
            accessibility_label_method("Carousel"),
            bool_method(
                "Carousel",
                "focus_ring",
                "Controls whether keyboard focus draws a focus ring.",
                Op::FocusRing,
            ),
            usize_method(
                "Carousel",
                "item_count",
                "Sets the number of logical items the state tracks.",
                Op::ItemCount,
            ),
            usize_method(
                "Carousel",
                "selected_index",
                "Selects an item without emitting a change event, for a script that owns the selection.",
                Op::SelectedIndex,
            ),
            MethodDescriptor::new(
                "on_change",
                vec![ArgumentDescriptor::new(
                    "callback",
                    ArgumentSchema::Callback("(index: number, cx: Context) => void"),
                )],
                |arguments| match arguments {
                    [argument @ ComponentArgument::Callback(_)] => {
                        Ok(ComponentPayload::new(Op::OnChange(argument.clone())))
                    }
                    _ => Err("Carousel.on_change(callback) expects one callback".into()),
                },
            )
            .with_documentation("Reports the newly selected zero-based item index."),
        ],
        "A retained snapping viewport composed from Carousel parts.",
    ))?;
    registry.register(descriptor(
        "CarouselContent",
        state_constructor("CarouselContent", |state| Payload::Content { state }),
        vec![],
        "The clipped Carousel viewport; accepts only CarouselItem children.",
    ))?;
    registry.register(descriptor(
        "CarouselItem",
        indexed_state_constructor("CarouselItem", |id, index, state| Payload::Item {
            id,
            index,
            state,
        }),
        vec![accessibility_label_method("CarouselItem")],
        "One indexed Carousel slide that accepts ordinary content children.",
    ))?;
    registry.register(descriptor(
        "CarouselPrevious",
        state_constructor("CarouselPrevious", |state| Payload::Previous { state }),
        vec![
            accessibility_label_method("CarouselPrevious"),
            size_method("CarouselPrevious"),
        ],
        "The previous-item control for a CarouselState.",
    ))?;
    registry.register(descriptor(
        "CarouselNext",
        state_constructor("CarouselNext", |state| Payload::Next { state }),
        vec![
            accessibility_label_method("CarouselNext"),
            size_method("CarouselNext"),
        ],
        "The next-item control for a CarouselState.",
    ))?;
    registry.register(descriptor(
        "CarouselPagination",
        ConstructorDescriptor::new("CarouselPagination", vec![], |_| {
            Ok(ComponentPayload::new(Payload::Pagination))
        }),
        vec![accessibility_label_method("CarouselPagination")],
        "A container that accepts only CarouselPaginationItem children.",
    ))?;
    registry.register(descriptor(
        "CarouselPaginationItem",
        indexed_state_constructor("CarouselPaginationItem", |id, index, state| {
            Payload::PaginationItem { id, index, state }
        }),
        vec![
            accessibility_label_method("CarouselPaginationItem"),
            size_method("CarouselPaginationItem"),
        ],
        "One indexed Carousel pagination control.",
    ))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_shell::{COMPONENT_REGISTRY_API_VERSION, DEFAULT_COMPONENT_MODULE};

    #[test]
    fn registers_the_closed_carousel_family_and_state() {
        let mut registry =
            ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION, DEFAULT_COMPONENT_MODULE)
                .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();

        assert_eq!(
            frozen
                .descriptors()
                .map(|descriptor| descriptor.name())
                .collect::<Vec<_>>(),
            [
                "Carousel",
                "CarouselContent",
                "CarouselItem",
                "CarouselPrevious",
                "CarouselNext",
                "CarouselPagination",
                "CarouselPaginationItem",
            ]
        );
        assert_eq!(
            frozen
                .states()
                .map(|state| (state.export(), state.kind()))
                .collect::<Vec<_>>(),
            [("CarouselState", "CarouselState")]
        );
        assert!(frozen.descriptors().all(|descriptor| {
            descriptor.documentation().is_some()
                && descriptor
                    .methods()
                    .iter()
                    .all(|method| method.documentation().is_some())
        }));
    }

    #[test]
    fn every_part_exposes_its_scriptable_surface() {
        let mut registry =
            ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION, DEFAULT_COMPONENT_MODULE)
                .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        let methods = |component: &str| {
            frozen
                .descriptors()
                .find(|descriptor| descriptor.name() == component)
                .map(|descriptor| {
                    descriptor
                        .methods()
                        .iter()
                        .map(|method| method.name().to_owned())
                        .collect::<Vec<_>>()
                })
                .unwrap()
        };

        // A script owns the selection and the item count through the root.
        let root = methods("Carousel");
        assert!(root.contains(&"item_count".to_owned()), "{root:?}");
        assert!(root.contains(&"selected_index".to_owned()), "{root:?}");

        // Every part whose Rust builder takes an accessibility label exposes it.
        for component in [
            "Carousel",
            "CarouselItem",
            "CarouselPrevious",
            "CarouselNext",
            "CarouselPagination",
            "CarouselPaginationItem",
        ] {
            let names = methods(component);
            assert!(
                names.contains(&"accessibility_label".to_owned()),
                "{component}: {names:?}"
            );
        }
    }

    #[test]
    fn identifiers_and_indices_are_closed() {
        assert!(nonempty_id(&ComponentArgument::String("carousel".into()), "Carousel").is_ok());
        assert!(nonempty_id(&ComponentArgument::String("  ".into()), "Carousel").is_err());
        assert_eq!(
            nonnegative_usize(&ComponentArgument::Number(3.), "CarouselItem").unwrap(),
            3
        );
        assert!(nonnegative_usize(&ComponentArgument::Number(-1.), "CarouselItem").is_err());
        assert!(nonnegative_usize(&ComponentArgument::Number(1.5), "CarouselItem").is_err());
        assert!(
            nonnegative_usize(
                &ComponentArgument::Number(usize::MAX as f64),
                "CarouselItem"
            )
            .is_err()
        );
    }
}
