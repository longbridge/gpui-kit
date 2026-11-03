//! `GridLayout`: a grid whose items the user drags and resizes.
//!
//! The script owns which items exist; the user owns where they sit. Each
//! `GridLayoutItem` declares its starting cell, which seeds the
//! `GridLayoutState` the first time that item appears. From then on the
//! state keeps the place the user gave it, and `on_change` reports the whole
//! layout so a script can persist it and restore it through `cell(...)` on a
//! later run. An item the script stops rendering leaves the layout.

use std::{cell::RefCell, rc::Rc, sync::Arc};

use gpui_component::grid_layout::{
    GridDragHandle, GridItem, GridLayout, GridLayoutEvent, GridLayoutItem, GridLayoutState,
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallback,
    ComponentCallbackArgument, ComponentDescriptor, ComponentMaterializer, ComponentPayload,
    ComponentRegistry, ConstructorDescriptor, MaterializeRequest, MethodDescriptor, RegistryError,
    StateDescriptor, anyhow,
    gpui::{
        self, App, AppContext as _, Axis, Entity, IntoElement as _, ParentElement as _,
        Refineable as _, RenderOnce, Styled as _, Subscription, Window, px, rems,
    },
};

use super::{
    support::{bool_method, require_child, string_method},
    typed_child::{Carrier, take},
    typed_compound::finish_part,
};

const STATE: &str = "GridLayoutState";
const COMPACTION: &[&str] = &["vertical", "horizontal", "none"];

#[derive(Clone)]
enum Payload {
    Root {
        id: String,
        state: ComponentArgument,
    },
    Item {
        id: String,
        state: ComponentArgument,
    },
    Handle {
        state: ComponentArgument,
        item: String,
    },
}

#[derive(Clone)]
enum Op {
    AccessibilityLabel(String),
    Columns(usize),
    Compaction(Option<Axis>),
    RowHeight(f32),
    Gap(f32),
    PreventCollision(bool),
    Draggable(bool),
    Resizable(bool),
    OnChange(ComponentArgument),
    Cell {
        x: usize,
        y: usize,
        w: usize,
        h: usize,
    },
    MinSize(usize, usize),
    MaxSize(usize, usize),
    Locked(bool),
    DragHandleOnly(bool),
}

/// What a `GridLayoutItem` hands its parent: the item itself, and the layout
/// entry that seeds the state when the item first appears.
struct DeclaredItem {
    item: GridLayoutItem,
    seed: GridItem,
}

/// The grid's rules a script set on the root, reasserted every frame.
#[derive(Default)]
struct Rules {
    columns: Option<usize>,
    compaction: Option<Option<Axis>>,
    row_height: Option<f32>,
    gap: Option<f32>,
    prevent_collision: Option<bool>,
    draggable: Option<bool>,
    resizable: Option<bool>,
}

struct Materializer;

impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<Payload>()
            .ok_or_else(|| {
                anyhow::anyhow!("GridLayout component received an incompatible payload")
            })?
            .clone();
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<Op>().cloned())
            .collect::<Vec<_>>();

        match payload {
            Payload::Root { id, state } => {
                let state =
                    request.with_state::<Entity<GridLayoutState>, _>(&state, Clone::clone)?;
                let mut rules = Rules::default();
                let mut callback = None;
                let mut grid = GridLayout::new(id.clone(), &state);
                for operation in operations {
                    match operation {
                        Op::AccessibilityLabel(value) => grid = grid.accessibility_label(value),
                        Op::Columns(value) => rules.columns = Some(value),
                        Op::Compaction(value) => rules.compaction = Some(value),
                        Op::RowHeight(value) => rules.row_height = Some(value),
                        Op::Gap(value) => rules.gap = Some(value),
                        Op::PreventCollision(value) => rules.prevent_collision = Some(value),
                        Op::Draggable(value) => rules.draggable = Some(value),
                        Op::Resizable(value) => rules.resizable = Some(value),
                        Op::OnChange(argument) => callback = Some(argument),
                        Op::Cell { .. }
                        | Op::MinSize(..)
                        | Op::MaxSize(..)
                        | Op::Locked(_)
                        | Op::DragHandleOnly(_) => {}
                    }
                }
                let callback = callback
                    .map(|argument| request.resolve_callback(&argument))
                    .transpose()?;

                let mut children = request.take_typed_children()?;
                for child in &children {
                    require_child("GridLayout", child.component_name(), &["GridLayoutItem"])?;
                }
                let mut seeds = Vec::with_capacity(children.len());
                for child in &mut children {
                    let mut element = request.materialize_child(child)?;
                    let declared = take::<DeclaredItem>(&mut element, "GridLayoutItem")?;
                    anyhow::ensure!(
                        !seeds
                            .iter()
                            .any(|seed: &GridItem| seed.id == declared.seed.id),
                        "GridLayout has two items with the id `{}`",
                        declared.seed.id
                    );
                    seeds.push(declared.seed);
                    grid = grid.item(declared.item);
                }
                grid.style().refine(&request.take_style());

                Ok(BoundGridLayout {
                    id,
                    state,
                    rules,
                    seeds,
                    callback,
                    grid,
                }
                .into_any_element())
            }
            Payload::Item { id, state } => {
                // The state is only checked here; the item reaches it through
                // its parent grid.
                request.with_state::<Entity<GridLayoutState>, _>(&state, |_| ())?;
                let mut cell = None;
                let mut seed_limits = Vec::new();
                let mut item = GridLayoutItem::new(id.clone());
                for operation in operations {
                    match operation {
                        Op::AccessibilityLabel(value) => item = item.accessibility_label(value),
                        Op::DragHandleOnly(value) => item = item.drag_handle_only(value),
                        Op::Cell { x, y, w, h } => cell = Some((x, y, w, h)),
                        operation @ (Op::MinSize(..) | Op::MaxSize(..) | Op::Locked(_)) => {
                            seed_limits.push(operation)
                        }
                        _ => {}
                    }
                }
                let (x, y, w, h) = cell.ok_or_else(|| {
                    anyhow::anyhow!(
                        "GridLayoutItem `{id}` needs cell(x, y, w, h) to place it in the grid"
                    )
                })?;
                let mut seed = GridItem::new(id, x, y, w, h);
                for operation in seed_limits {
                    seed = match operation {
                        Op::MinSize(w, h) => seed.with_min_size(w, h),
                        Op::MaxSize(w, h) => seed.with_max_size(w, h),
                        Op::Locked(locked) => seed.with_locked(locked),
                        _ => seed,
                    };
                }
                item.style().refine(&request.take_style());
                item.extend(request.take_children()?);
                Ok(Carrier::new(DeclaredItem { item, seed }).into_any_element())
            }
            Payload::Handle { state, item } => {
                let state =
                    request.with_state::<Entity<GridLayoutState>, _>(&state, Clone::clone)?;
                finish_part(&mut request, GridDragHandle::new(&state, item))
            }
        }
    }
}

struct ChangeHost {
    callback: Rc<RefCell<Option<ComponentCallback>>>,
    _change: Subscription,
}

#[derive(gpui::IntoElement)]
struct BoundGridLayout {
    id: String,
    state: Entity<GridLayoutState>,
    rules: Rules,
    seeds: Vec<GridItem>,
    callback: Option<ComponentCallback>,
    grid: GridLayout,
}

impl RenderOnce for BoundGridLayout {
    fn render(self, window: &mut Window, cx: &mut App) -> impl gpui::IntoElement {
        // The rules a script sets belong to the script, so every frame
        // reasserts them. Each setter runs only when the value differs, which
        // keeps a gesture in progress from being cancelled.
        let rules = &self.rules;
        let current = self.state.read(cx);
        let columns = rules.columns.filter(|value| current.columns() != *value);
        let compaction = rules
            .compaction
            .filter(|value| current.compaction() != *value);
        let prevent_collision = rules
            .prevent_collision
            .filter(|value| current.prevent_collision() != *value);
        let draggable = rules
            .draggable
            .filter(|value| current.draggable() != *value);
        let resizable = rules
            .resizable
            .filter(|value| current.resizable() != *value);
        let membership = reconcile(current.items(), &self.seeds);
        let (row_height, gap) = (rules.row_height, rules.gap);

        self.state.update(cx, |state, cx| {
            if let Some(value) = columns {
                state.set_columns(value, cx);
            }
            if let Some(value) = compaction {
                state.set_compaction(value, cx);
            }
            if let Some(value) = prevent_collision {
                state.set_prevent_collision(value, cx);
            }
            if let Some(value) = draggable {
                state.set_draggable(value, cx);
            }
            if let Some(value) = resizable {
                state.set_resizable(value, cx);
            }
            if let Some(value) = row_height {
                state.set_row_height(rems(value), cx);
            }
            state.set_gap(gap.map(px), cx);
            if let Some(items) = membership {
                state.set_items(items, cx);
            }
        });

        let initial_callback = self.callback.clone();
        let state = self.state.clone();
        let host: Entity<ChangeHost> = window.use_keyed_state(
            format!("shell-grid-layout:{}:{:?}", self.id, self.state.entity_id()),
            cx,
            move |window, cx| {
                let callback = Rc::new(RefCell::new(initial_callback));
                let event_callback = callback.clone();
                let change =
                    window.subscribe(&state, cx, move |_, event: &GridLayoutEvent, window, cx| {
                        let GridLayoutEvent::Change(items) = event;
                        let callback = event_callback.borrow().clone();
                        if let Some(callback) = callback {
                            callback.invoke_and_report_with(
                                "GridLayout.on_change callback failed",
                                &[layout_argument(items)],
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
        self.grid
    }
}

/// The layout the script's latest render asks for, or `None` when the state
/// already holds it.
///
/// Membership and each item's limits belong to the script: an item appears
/// with the cell it declares, leaves when the script stops rendering it, and
/// takes whatever `locked`, `min_size` and `max_size` the latest render gives
/// it. Only the place — `x`, `y`, `w`, `h` — belongs to the user, so an item
/// already in the grid keeps the place it was given. `set_items` then settles
/// that place under the new limits.
fn reconcile(current: &[GridItem], seeds: &[GridItem]) -> Option<Vec<GridItem>> {
    let next = seeds
        .iter()
        .map(
            |seed| match current.iter().find(|item| item.id == seed.id) {
                Some(item) => {
                    let mut kept = seed.clone();
                    (kept.x, kept.y, kept.w, kept.h) = (item.x, item.y, item.w, item.h);
                    kept
                }
                None => seed.clone(),
            },
        )
        .collect::<Vec<_>>();
    // Compare everything except the place: settling may clamp a place to the
    // new limits, and comparing it would reapply the layout every frame.
    let script_owned = |item: &GridItem| {
        (
            item.id.clone(),
            item.min_w,
            item.min_h,
            item.max_w,
            item.max_h,
            item.locked,
        )
    };
    let unchanged = next.len() == current.len()
        && next.iter().all(|item| {
            current
                .iter()
                .any(|existing| script_owned(existing) == script_owned(item))
        });
    (!unchanged).then_some(next)
}

/// The layout as a script reads it: one `[id, x, y, w, h]` tuple per item.
fn layout_argument(items: &[GridItem]) -> ComponentCallbackArgument {
    ComponentCallbackArgument::Array(
        items
            .iter()
            .map(|item| {
                ComponentCallbackArgument::Array(vec![
                    ComponentCallbackArgument::String(item.id.to_string()),
                    ComponentCallbackArgument::Number(item.x as f64),
                    ComponentCallbackArgument::Number(item.y as f64),
                    ComponentCallbackArgument::Number(item.w as f64),
                    ComponentCallbackArgument::Number(item.h as f64),
                ])
            })
            .collect(),
    )
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

fn positive_usize(argument: &ComponentArgument, callable: &str) -> Result<usize, String> {
    match nonnegative_usize(argument, callable) {
        Ok(0) | Err(_) => Err(format!("{callable} expects a positive integer")),
        Ok(value) => Ok(value),
    }
}

fn nonnegative_length(argument: &ComponentArgument, callable: &str) -> Result<f32, String> {
    match argument {
        ComponentArgument::Number(value) if value.is_finite() && *value >= 0. => Ok(*value as f32),
        _ => Err(format!("{callable} expects a nonnegative number")),
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
        _ => Err(format!("{callable} expects a {STATE} entity")),
    }
}

fn compaction(value: &str) -> Option<Option<Axis>> {
    match value {
        "vertical" => Some(Some(Axis::Vertical)),
        "horizontal" => Some(Some(Axis::Horizontal)),
        "none" => Some(None),
        _ => None,
    }
}

fn id_state_constructor(
    component: &'static str,
    payload: impl Fn(String, ComponentArgument) -> Payload + Send + Sync + 'static,
) -> ConstructorDescriptor {
    ConstructorDescriptor::new(
        component,
        vec![
            ArgumentDescriptor::new("id", ArgumentSchema::String),
            ArgumentDescriptor::new("state", ArgumentSchema::Entity(STATE)),
        ],
        move |arguments| match arguments {
            [id, state] => Ok(ComponentPayload::new(payload(
                nonempty_id(id, component)?,
                entity(state, component)?,
            ))),
            _ => Err(format!(
                "{component}(id, state) expects a nonempty id and {STATE} entity"
            )),
        },
    )
}

fn number_method(
    component: &'static str,
    name: &'static str,
    documentation: &'static str,
    make: fn(&ComponentArgument, &str) -> Result<Op, String>,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new("value", ArgumentSchema::Number)],
        move |arguments| match arguments {
            [argument] => {
                make(argument, &format!("{component}.{name}(value)")).map(ComponentPayload::new)
            }
            _ => Err(format!("{component}.{name}(value) expects one number")),
        },
    )
    .with_documentation(documentation)
}

fn size_method(
    name: &'static str,
    documentation: &'static str,
    make: fn(usize, usize) -> Op,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![
            ArgumentDescriptor::new("w", ArgumentSchema::Number),
            ArgumentDescriptor::new("h", ArgumentSchema::Number),
        ],
        move |arguments| match arguments {
            [w, h] => {
                let callable = format!("GridLayoutItem.{name}(w, h)");
                Ok(ComponentPayload::new(make(
                    positive_usize(w, &callable)?,
                    positive_usize(h, &callable)?,
                )))
            }
            _ => Err(format!("GridLayoutItem.{name}(w, h) expects two numbers")),
        },
    )
    .with_documentation(documentation)
}

fn accessibility_label_method(component: &'static str) -> MethodDescriptor {
    string_method(
        component,
        "accessibility_label",
        "Sets the name announced by accessibility clients.",
        Op::AccessibilityLabel,
    )
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
            STATE,
            STATE,
            vec![
                ArgumentDescriptor::new(
                    "columns",
                    ArgumentSchema::Optional(Box::new(ArgumentSchema::Number)),
                ),
                ArgumentDescriptor::new(
                    "compaction",
                    ArgumentSchema::Optional(Box::new(ArgumentSchema::Enum(COMPACTION))),
                ),
            ],
            |arguments, _, cx| {
                let [columns, compaction_argument] = arguments else {
                    return Err(format!("{STATE} expects columns and compaction"));
                };
                let columns = match columns {
                    ComponentArgument::Optional(Some(columns)) => {
                        positive_usize(columns, &format!("{STATE}(columns)"))?
                    }
                    ComponentArgument::Optional(None) => 12,
                    _ => return Err(format!("{STATE} columns must be optional")),
                };
                let axis = match compaction_argument {
                    ComponentArgument::Optional(Some(value)) => match value.as_ref() {
                        ComponentArgument::Enum(value) => compaction(value).ok_or_else(|| {
                            format!("{STATE} compaction expects vertical, horizontal or none")
                        })?,
                        _ => {
                            return Err(format!(
                                "{STATE} compaction expects vertical, horizontal or none"
                            ));
                        }
                    },
                    ComponentArgument::Optional(None) => Some(Axis::Vertical),
                    _ => return Err(format!("{STATE} compaction must be optional")),
                };
                Ok(Box::new(cx.new(|cx| {
                    GridLayoutState::new(cx)
                        .with_columns(columns)
                        .with_compaction(axis)
                })))
            },
        )
        .with_documentation(
            "Retained GridLayout placement, rules, selection, and drag state. Items join it \
             through GridLayoutItem.cell and keep the place the user gives them.",
        ),
    )?;

    registry.register(descriptor(
        "GridLayout",
        id_state_constructor("GridLayout", |id, state| Payload::Root { id, state }),
        vec![
            accessibility_label_method("GridLayout"),
            number_method(
                "GridLayout",
                "columns",
                "Sets the number of equal columns the width is divided into.",
                |argument, callable| positive_usize(argument, callable).map(Op::Columns),
            ),
            MethodDescriptor::new(
                "compaction",
                vec![ArgumentDescriptor::new(
                    "compaction",
                    ArgumentSchema::Enum(COMPACTION),
                )],
                |arguments| match arguments {
                    [ComponentArgument::Enum(value)] => compaction(value)
                        .map(|axis| ComponentPayload::new(Op::Compaction(axis)))
                        .ok_or_else(|| {
                            format!("GridLayout.compaction expects {}", COMPACTION.join(", "))
                        }),
                    _ => Err("GridLayout.compaction(compaction) expects one direction".into()),
                },
            )
            .with_documentation(
                "Sets the direction items float toward when space opens up, or none to leave \
                 them where they are put.",
            ),
            number_method(
                "GridLayout",
                "row_height",
                "Sets the height of one row in rems, so rows follow the application's zoom.",
                |argument, callable| nonnegative_length(argument, callable).map(Op::RowHeight),
            ),
            // Not `gap`: a registered method of that name would shadow the
            // native flex `gap` on every element.
            number_method(
                "GridLayout",
                "item_spacing",
                "Sets the space between neighbouring items in pixels; the theme's spacing \
                 applies otherwise.",
                |argument, callable| nonnegative_length(argument, callable).map(Op::Gap),
            ),
            bool_method(
                "GridLayout",
                "prevent_collision",
                "Refuses a move or resize that would overlap another item instead of pushing it.",
                Op::PreventCollision,
            ),
            bool_method(
                "GridLayout",
                "draggable",
                "Controls whether items can be moved.",
                Op::Draggable,
            ),
            bool_method(
                "GridLayout",
                "resizable",
                "Controls whether items can be resized.",
                Op::Resizable,
            ),
            MethodDescriptor::new(
                "on_change",
                vec![ArgumentDescriptor::new(
                    "callback",
                    ArgumentSchema::Callback(
                        "(layout: [id: string, x: number, y: number, w: number, h: number][], cx: Context) => void",
                    ),
                )],
                |arguments| match arguments {
                    [argument @ ComponentArgument::Callback(_)] => {
                        Ok(ComponentPayload::new(Op::OnChange(argument.clone())))
                    }
                    _ => Err("GridLayout.on_change(callback) expects one callback".into()),
                },
            )
            .with_documentation(
                "Reports the whole layout, one [id, x, y, w, h] entry per item, after the user \
                 moves or resizes an item.",
            ),
        ],
        "A grid whose items the user drags and resizes; accepts only GridLayoutItem children.",
    ))?;
    registry.register(descriptor(
        "GridLayoutItem",
        id_state_constructor("GridLayoutItem", |id, state| Payload::Item { id, state }),
        vec![
            accessibility_label_method("GridLayoutItem"),
            MethodDescriptor::new(
                "cell",
                vec![
                    ArgumentDescriptor::new("x", ArgumentSchema::Number),
                    ArgumentDescriptor::new("y", ArgumentSchema::Number),
                    ArgumentDescriptor::new("w", ArgumentSchema::Number),
                    ArgumentDescriptor::new("h", ArgumentSchema::Number),
                ],
                |arguments| match arguments {
                    [x, y, w, h] => {
                        let callable = "GridLayoutItem.cell(x, y, w, h)";
                        Ok(ComponentPayload::new(Op::Cell {
                            x: nonnegative_usize(x, callable)?,
                            y: nonnegative_usize(y, callable)?,
                            w: positive_usize(w, callable)?,
                            h: positive_usize(h, callable)?,
                        }))
                    }
                    _ => Err("GridLayoutItem.cell(x, y, w, h) expects four numbers".into()),
                },
            )
            .with_documentation(
                "Places the item at column x and row y, spanning w columns and h rows, when it \
                 first joins the grid. Required.",
            ),
            size_method(
                "min_size",
                "Sets the smallest span a resize may reach.",
                Op::MinSize,
            ),
            size_method(
                "max_size",
                "Sets the largest span a resize may reach.",
                Op::MaxSize,
            ),
            bool_method(
                "GridLayoutItem",
                "locked",
                "Locks the item in place; other items flow around it.",
                Op::Locked,
            ),
            bool_method(
                "GridLayoutItem",
                "drag_handle_only",
                "Moves the item only from a GridDragHandle inside it.",
                Op::DragHandleOnly,
            ),
        ],
        "One GridLayout item that accepts ordinary content children.",
    ))?;
    registry.register(descriptor(
        "GridDragHandle",
        ConstructorDescriptor::new(
            "GridDragHandle",
            vec![
                ArgumentDescriptor::new("state", ArgumentSchema::Entity(STATE)),
                ArgumentDescriptor::new("item", ArgumentSchema::String),
            ],
            |arguments| match arguments {
                [state, item] => Ok(ComponentPayload::new(Payload::Handle {
                    state: entity(state, "GridDragHandle")?,
                    item: nonempty_id(item, "GridDragHandle")?,
                })),
                _ => Err(format!(
                    "GridDragHandle(state, item) expects a {STATE} entity and an item id"
                )),
            },
        ),
        vec![],
        "The part of a GridLayoutItem the pointer moves it by, for an item set to \
         drag_handle_only.",
    ))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_shell::{COMPONENT_REGISTRY_API_VERSION, DEFAULT_COMPONENT_MODULE};

    fn frozen() -> gpui_shell::FrozenComponentRegistry {
        let mut registry =
            ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION, DEFAULT_COMPONENT_MODULE)
                .unwrap();
        register(&mut registry).unwrap();
        registry.freeze().unwrap()
    }

    #[test]
    fn registers_the_grid_layout_family_and_state() {
        let frozen = frozen();
        assert_eq!(
            frozen
                .descriptors()
                .map(|descriptor| descriptor.name())
                .collect::<Vec<_>>(),
            ["GridLayout", "GridLayoutItem", "GridDragHandle"]
        );
        assert_eq!(
            frozen
                .states()
                .map(|state| (state.export(), state.kind()))
                .collect::<Vec<_>>(),
            [(STATE, STATE)]
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
    fn arguments_are_closed() {
        assert_eq!(
            positive_usize(&ComponentArgument::Number(3.), "cell").unwrap(),
            3
        );
        assert!(positive_usize(&ComponentArgument::Number(0.), "cell").is_err());
        assert!(nonnegative_usize(&ComponentArgument::Number(-1.), "cell").is_err());
        assert!(nonnegative_usize(&ComponentArgument::Number(1.5), "cell").is_err());
        assert!(nonnegative_length(&ComponentArgument::Number(f64::NAN), "gap").is_err());
        assert!(nonempty_id(&ComponentArgument::String(" ".into()), "GridLayoutItem").is_err());
        assert_eq!(compaction("none"), Some(None));
        assert_eq!(compaction("diagonal"), None);
    }

    #[test]
    fn reconciling_keeps_the_users_place_and_the_scripts_limits() {
        let moved = GridItem::new("a", 6, 4, 3, 2);
        let current = vec![moved.clone(), GridItem::new("gone", 0, 0, 1, 1)];
        let seeds = vec![
            GridItem::new("a", 0, 0, 2, 2)
                .with_min_size(3, 2)
                .with_locked(true),
            GridItem::new("new", 0, 0, 1, 1),
        ];

        let next = reconcile(&current, &seeds).unwrap();
        let a = next.iter().find(|item| item.id == "a").unwrap();
        // The place is the user's; the limits and lock are the script's.
        assert_eq!((a.x, a.y, a.w, a.h), (6, 4, 3, 2));
        assert_eq!(a.min_w, 3);
        assert!(a.locked);
        assert!(next.iter().any(|item| item.id == "new"));
        assert!(!next.iter().any(|item| item.id == "gone"));

        // Once the state holds it, nothing is reapplied on the next frame,
        // even after settling clamped the place to the new limits.
        assert_eq!(reconcile(&next, &seeds), None);
        let mut settled = next.clone();
        settled[0].w = 5;
        assert_eq!(reconcile(&settled, &seeds), None);
        // A limit changed on a later render reaches an item already placed.
        let unlocked = vec![
            GridItem::new("a", 0, 0, 2, 2).with_min_size(3, 2),
            GridItem::new("new", 0, 0, 1, 1),
        ];
        assert!(!reconcile(&next, &unlocked).unwrap()[0].locked);
    }

    #[test]
    fn the_layout_reaches_a_script_as_tuples() {
        let argument = layout_argument(&[GridItem::new("revenue", 1, 2, 3, 4)]);
        assert_eq!(
            argument,
            ComponentCallbackArgument::Array(vec![ComponentCallbackArgument::Array(vec![
                ComponentCallbackArgument::String("revenue".into()),
                ComponentCallbackArgument::Number(1.),
                ComponentCallbackArgument::Number(2.),
                ComponentCallbackArgument::Number(3.),
                ComponentCallbackArgument::Number(4.),
            ])])
        );
    }
}
