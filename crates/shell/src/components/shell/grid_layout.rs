//! `GridLayout`: a grid whose items the user drags and resizes.
//!
//! The script owns which items exist and each item's limits; the user owns
//! where they sit. A `GridItem`'s placement seeds the `GridLayoutState` the
//! first time that item appears; from then on the state keeps the place the
//! user gave it, and `on_change` reports the whole layout so a script can
//! persist it and restore it through the items' placement on a later run. An
//! item the script stops rendering leaves the layout.

use std::{cell::RefCell, rc::Rc, sync::Arc};

use gpui_component::grid_layout::{
    GridArea, GridItem, GridLayout, GridLayoutEvent, GridLayoutState, grid_drag_handle, grid_layout,
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallback,
    ComponentCallbackArgument, ComponentDescriptor, ComponentMaterializer, ComponentPayload,
    ComponentRegistry, ConstructorDescriptor, MaterializeRequest, MethodDescriptor, RegistryError,
    StateDescriptor, anyhow,
    gpui::{
        self, App, AppContext as _, Axis, Entity, IntoElement as _, ParentElement as _,
        Refineable as _, RenderOnce, Styled as _, Subscription, Window, rems,
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
    },
    Handle {
        state: ComponentArgument,
        item: String,
    },
}

#[derive(Clone)]
enum Op {
    AccessibilityLabel(String),
    Cols(usize),
    Compaction(Option<Axis>),
    RowHeight(f32),
    PreventCollision(bool),
    Draggable(bool),
    Resizable(bool),
    Reflow(usize),
    OnChange(ComponentArgument),
    OnSelect(ComponentArgument),
    Col(usize),
    Row(usize),
    ColSpan(usize),
    RowSpan(usize),
    MinSpan(usize, usize),
    MaxSpan(usize, usize),
    Locked(bool),
    DragHandleOnly(bool),
}

/// What a `GridItem` hands its parent: the item itself, and the area that
/// seeds the state when the item first appears.
struct DeclaredItem {
    item: GridItem,
    seed: GridArea,
}

/// The grid's rules a script set on the root, reasserted every frame.
#[derive(Default)]
struct Rules {
    cols: Option<usize>,
    compaction: Option<Option<Axis>>,
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
                let mut grid =
                    request.with_window_app(|_, cx| Ok(grid_layout(id.clone(), &state, cx)))?;
                let mut rules = Rules::default();
                let (mut on_change, mut on_select) = (None, None);
                for operation in operations {
                    match operation {
                        Op::AccessibilityLabel(value) => grid = grid.accessibility_label(value),
                        Op::RowHeight(value) => grid = grid.row_height(rems(value)),
                        Op::Reflow(value) => grid = grid.reflow(value),
                        Op::Cols(value) => rules.cols = Some(value),
                        Op::Compaction(value) => rules.compaction = Some(value),
                        Op::PreventCollision(value) => rules.prevent_collision = Some(value),
                        Op::Draggable(value) => rules.draggable = Some(value),
                        Op::Resizable(value) => rules.resizable = Some(value),
                        Op::OnChange(argument) => on_change = Some(argument),
                        Op::OnSelect(argument) => on_select = Some(argument),
                        _ => {}
                    }
                }
                let on_change = on_change
                    .map(|argument| request.resolve_callback(&argument))
                    .transpose()?;
                let on_select = on_select
                    .map(|argument| request.resolve_callback(&argument))
                    .transpose()?;

                let mut children = request.take_typed_children()?;
                for child in &children {
                    require_child("GridLayout", child.component_name(), &["GridItem"])?;
                }
                let mut seeds = Vec::with_capacity(children.len());
                for child in &mut children {
                    let mut element = request.materialize_child(child)?;
                    let declared = take::<DeclaredItem>(&mut element, "GridItem")?;
                    anyhow::ensure!(
                        !seeds
                            .iter()
                            .any(|seed: &GridArea| seed.id == declared.seed.id),
                        "GridLayout has two items with the id `{}`",
                        declared.seed.id
                    );
                    seeds.push(declared.seed);
                    grid = grid.item(declared.item);
                }
                // The script's own styles — `gap` among them — land on the grid.
                grid.style().refine(&request.take_style());

                Ok(BoundGridLayout {
                    id,
                    state,
                    rules,
                    seeds,
                    callbacks: Callbacks {
                        on_change,
                        on_select,
                    },
                    grid,
                }
                .into_any_element())
            }
            Payload::Item { id } => {
                let mut item = GridItem::new(id.clone());
                let mut seed = GridArea::new(id);
                for operation in operations {
                    match operation {
                        Op::AccessibilityLabel(value) => item = item.accessibility_label(value),
                        Op::DragHandleOnly(value) => item = item.drag_handle_only(value),
                        Op::Col(value) => seed = seed.col(value),
                        Op::Row(value) => seed = seed.row(value),
                        Op::ColSpan(value) => seed = seed.col_span(value),
                        Op::RowSpan(value) => seed = seed.row_span(value),
                        Op::MinSpan(cols, rows) => seed = seed.min_span(cols, rows),
                        Op::MaxSpan(cols, rows) => seed = seed.max_span(cols, rows),
                        Op::Locked(value) => seed = seed.locked(value),
                        _ => {}
                    }
                }
                item.style().refine(&request.take_style());
                item.extend(request.take_children()?);
                Ok(Carrier::new(DeclaredItem { item, seed }).into_any_element())
            }
            Payload::Handle { state, item } => {
                let state =
                    request.with_state::<Entity<GridLayoutState>, _>(&state, Clone::clone)?;
                finish_part(&mut request, grid_drag_handle(&state, item))
            }
        }
    }
}

#[derive(Clone, Default)]
struct Callbacks {
    on_change: Option<ComponentCallback>,
    on_select: Option<ComponentCallback>,
}

struct EventHost {
    callbacks: Rc<RefCell<Callbacks>>,
    _events: Subscription,
}

#[derive(gpui::IntoElement)]
struct BoundGridLayout {
    id: String,
    state: Entity<GridLayoutState>,
    rules: Rules,
    seeds: Vec<GridArea>,
    callbacks: Callbacks,
    grid: GridLayout,
}

impl RenderOnce for BoundGridLayout {
    fn render(self, window: &mut Window, cx: &mut App) -> impl gpui::IntoElement {
        // The rules a script sets belong to the script, so every frame
        // reasserts them. Each setter runs only when the value differs, which
        // keeps a gesture in progress from being cancelled.
        let rules = &self.rules;
        let current = self.state.read(cx);
        let cols = rules.cols.filter(|value| current.column_count() != *value);
        let compaction = rules
            .compaction
            .filter(|value| current.compaction_axis() != *value);
        let prevent_collision = rules
            .prevent_collision
            .filter(|value| current.is_collision_prevented() != *value);
        let draggable = rules
            .draggable
            .filter(|value| current.is_draggable() != *value);
        let resizable = rules
            .resizable
            .filter(|value| current.is_resizable() != *value);
        let areas = reconcile(current.areas(), &self.seeds);

        self.state.update(cx, |state, cx| {
            if let Some(value) = cols {
                state.set_cols(value, cx);
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
            if let Some(areas) = areas {
                state.set_areas(areas, cx);
            }
        });

        let initial = self.callbacks.clone();
        let state = self.state.clone();
        let host: Entity<EventHost> = window.use_keyed_state(
            format!("shell-grid-layout:{}:{:?}", self.id, self.state.entity_id()),
            cx,
            move |window, cx| {
                let callbacks = Rc::new(RefCell::new(initial));
                let events = callbacks.clone();
                let subscription =
                    window.subscribe(&state, cx, move |_, event: &GridLayoutEvent, window, cx| {
                        let callbacks = events.borrow().clone();
                        match event {
                            GridLayoutEvent::Change(areas) => {
                                if let Some(callback) = callbacks.on_change {
                                    callback.invoke_and_report_with(
                                        "GridLayout.on_change callback failed",
                                        &[layout_argument(areas)],
                                        window,
                                        cx,
                                    );
                                }
                            }
                            GridLayoutEvent::Select(Some(id)) => {
                                if let Some(callback) = callbacks.on_select {
                                    callback.invoke_and_report_with(
                                        "GridLayout.on_select callback failed",
                                        &[ComponentCallbackArgument::String(id.to_string())],
                                        window,
                                        cx,
                                    );
                                }
                            }
                            _ => {}
                        }
                    });
                EventHost {
                    callbacks,
                    _events: subscription,
                }
            },
        );
        *host.read(cx).callbacks.borrow_mut() = self.callbacks;
        self.grid
    }
}

/// The layout the script's latest render asks for, or `None` when the state
/// already holds it.
///
/// Membership and each item's limits belong to the script: an item appears
/// with the placement it declares, leaves when the script stops rendering it,
/// and takes whatever `locked`, `min_span` and `max_span` the latest render
/// gives it. Only the place — `col`, `row`, `col_span`, `row_span` — belongs
/// to the user, so an item already in the grid keeps the place it was given.
/// `set_areas` then settles that place under the new limits.
fn reconcile(current: &[GridArea], seeds: &[GridArea]) -> Option<Vec<GridArea>> {
    let next = seeds
        .iter()
        .map(
            |seed| match current.iter().find(|area| area.id == seed.id) {
                Some(area) => {
                    let mut kept = seed.clone();
                    kept.col = area.col;
                    kept.row = area.row;
                    kept.col_span = area.col_span;
                    kept.row_span = area.row_span;
                    kept
                }
                None => seed.clone(),
            },
        )
        .collect::<Vec<_>>();
    // Compare everything except the place: settling may clamp a place to the
    // new limits, and comparing it would reapply the layout every frame.
    let script_owned = |area: &GridArea| {
        (
            area.id.clone(),
            area.min_col_span,
            area.min_row_span,
            area.max_col_span,
            area.max_row_span,
            area.locked,
        )
    };
    let unchanged = next.len() == current.len()
        && next.iter().all(|area| {
            current
                .iter()
                .any(|existing| script_owned(existing) == script_owned(area))
        });
    (!unchanged).then_some(next)
}

/// The layout as a script reads it: one `[id, col, row, col_span, row_span]`
/// tuple per item.
fn layout_argument(areas: &[GridArea]) -> ComponentCallbackArgument {
    ComponentCallbackArgument::Array(
        areas
            .iter()
            .map(|area| {
                ComponentCallbackArgument::Array(vec![
                    ComponentCallbackArgument::String(area.id.to_string()),
                    ComponentCallbackArgument::Number(area.col as f64),
                    ComponentCallbackArgument::Number(area.row as f64),
                    ComponentCallbackArgument::Number(area.col_span as f64),
                    ComponentCallbackArgument::Number(area.row_span as f64),
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

fn positive_length(argument: &ComponentArgument, callable: &str) -> Result<f32, String> {
    match argument {
        ComponentArgument::Number(value) if value.is_finite() && *value > 0. => Ok(*value as f32),
        _ => Err(format!("{callable} expects a positive number")),
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

fn span_method(
    name: &'static str,
    documentation: &'static str,
    make: fn(usize, usize) -> Op,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![
            ArgumentDescriptor::new("col_span", ArgumentSchema::Number),
            ArgumentDescriptor::new("row_span", ArgumentSchema::Number),
        ],
        move |arguments| match arguments {
            [cols, rows] => {
                let callable = format!("GridItem.{name}(col_span, row_span)");
                Ok(ComponentPayload::new(make(
                    positive_usize(cols, &callable)?,
                    positive_usize(rows, &callable)?,
                )))
            }
            _ => Err(format!(
                "GridItem.{name}(col_span, row_span) expects two numbers"
            )),
        },
    )
    .with_documentation(documentation)
}

fn callback_method(
    name: &'static str,
    signature: &'static str,
    documentation: &'static str,
    make: fn(ComponentArgument) -> Op,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(
            "callback",
            ArgumentSchema::Callback(signature),
        )],
        move |arguments| match arguments {
            [argument @ ComponentArgument::Callback(_)] => {
                Ok(ComponentPayload::new(make(argument.clone())))
            }
            _ => Err(format!("GridLayout.{name}(callback) expects one callback")),
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
                    "cols",
                    ArgumentSchema::Optional(Box::new(ArgumentSchema::Number)),
                ),
                ArgumentDescriptor::new(
                    "compaction",
                    ArgumentSchema::Optional(Box::new(ArgumentSchema::Enum(COMPACTION))),
                ),
            ],
            |arguments, _, cx| {
                let [cols, compaction_argument] = arguments else {
                    return Err(format!("{STATE} expects cols and compaction"));
                };
                let cols = match cols {
                    ComponentArgument::Optional(Some(cols)) => {
                        positive_usize(cols, &format!("{STATE}(cols)"))?
                    }
                    ComponentArgument::Optional(None) => 12,
                    _ => return Err(format!("{STATE} cols must be optional")),
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
                    GridLayoutState::new(cx).cols(cols).compaction(axis)
                })))
            },
        )
        .with_documentation(
            "Retained GridLayout areas, rules, selection, and drag state. Items join it with the \
             placement their GridItem declares and keep the place the user gives them.",
        ),
    )?;

    registry.register(descriptor(
        "GridLayout",
        ConstructorDescriptor::new(
            "GridLayout",
            vec![
                ArgumentDescriptor::new("id", ArgumentSchema::String),
                ArgumentDescriptor::new("state", ArgumentSchema::Entity(STATE)),
            ],
            |arguments| match arguments {
                [id, state] => Ok(ComponentPayload::new(Payload::Root {
                    id: nonempty_id(id, "GridLayout")?,
                    state: entity(state, "GridLayout")?,
                })),
                _ => Err(format!(
                    "GridLayout(id, state) expects a nonempty id and {STATE} entity"
                )),
            },
        ),
        vec![
            accessibility_label_method("GridLayout"),
            number_method(
                "GridLayout",
                "cols",
                "Sets the number of equal columns the width is divided into.",
                |argument, callable| positive_usize(argument, callable).map(Op::Cols),
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
                |argument, callable| positive_length(argument, callable).map(Op::RowHeight),
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
            number_method(
                "GridLayout",
                "reflow",
                "Shows the items over this many columns without changing the stored layout — a \
                 one-column stack for a narrow window. The grid is read-only while reflowed.",
                |argument, callable| positive_usize(argument, callable).map(Op::Reflow),
            ),
            callback_method(
                "on_change",
                "(layout: [id: string, col: number, row: number, col_span: number, row_span: number][], cx: Context) => void",
                "Reports the whole layout, one [id, col, row, col_span, row_span] entry per \
                 item, after the user moves or resizes an item.",
                Op::OnChange,
            ),
            callback_method(
                "on_select",
                "(id: string, cx: Context) => void",
                "Reports the item the user selects, by pressing it or with Tab, once per change.",
                Op::OnSelect,
            ),
        ],
        "A grid whose items the user drags and resizes; accepts only GridItem children. Style \
         its gaps with gap, gap_x and gap_y.",
    ))?;
    registry.register(descriptor(
        "GridItem",
        ConstructorDescriptor::new(
            "GridItem",
            vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
            |arguments| match arguments {
                [id] => Ok(ComponentPayload::new(Payload::Item {
                    id: nonempty_id(id, "GridItem")?,
                })),
                _ => Err("GridItem(id) expects a nonempty id".into()),
            },
        ),
        vec![
            accessibility_label_method("GridItem"),
            number_method(
                "GridItem",
                "col",
                "Sets the leading column, from zero, the item takes when it first joins the grid.",
                |argument, callable| nonnegative_usize(argument, callable).map(Op::Col),
            ),
            number_method(
                "GridItem",
                "row",
                "Sets the top row, from zero, the item takes when it first joins the grid.",
                |argument, callable| nonnegative_usize(argument, callable).map(Op::Row),
            ),
            number_method(
                "GridItem",
                "col_span",
                "Sets how many columns the item covers when it first joins the grid.",
                |argument, callable| positive_usize(argument, callable).map(Op::ColSpan),
            ),
            number_method(
                "GridItem",
                "row_span",
                "Sets how many rows the item covers when it first joins the grid.",
                |argument, callable| positive_usize(argument, callable).map(Op::RowSpan),
            ),
            span_method(
                "min_span",
                "Sets the fewest columns and rows a resize may leave the item.",
                Op::MinSpan,
            ),
            span_method(
                "max_span",
                "Sets the most columns and rows a resize may give the item.",
                Op::MaxSpan,
            ),
            bool_method(
                "GridItem",
                "locked",
                "Locks the item in place; other items flow around it.",
                Op::Locked,
            ),
            bool_method(
                "GridItem",
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
        "The part of a GridItem the pointer moves it by, for an item set to drag_handle_only.",
    ))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_shell::{COMPONENT_REGISTRY_API_VERSION, DEFAULT_COMPONENT_MODULE};

    fn area(id: &str, col: usize, row: usize, col_span: usize, row_span: usize) -> GridArea {
        GridArea::new(id.to_owned())
            .col(col)
            .row(row)
            .col_span(col_span)
            .row_span(row_span)
    }

    #[test]
    fn registers_the_grid_layout_family_and_state() {
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
            ["GridLayout", "GridItem", "GridDragHandle"]
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
            positive_usize(&ComponentArgument::Number(3.), "span").unwrap(),
            3
        );
        assert!(positive_usize(&ComponentArgument::Number(0.), "span").is_err());
        assert!(nonnegative_usize(&ComponentArgument::Number(-1.), "col").is_err());
        assert!(nonnegative_usize(&ComponentArgument::Number(1.5), "col").is_err());
        assert!(positive_length(&ComponentArgument::Number(f64::NAN), "row_height").is_err());
        assert!(positive_length(&ComponentArgument::Number(0.), "row_height").is_err());
        assert!(nonempty_id(&ComponentArgument::String(" ".into()), "GridItem").is_err());
        assert_eq!(compaction("none"), Some(None));
        assert_eq!(compaction("diagonal"), None);
    }

    #[test]
    fn reconciling_keeps_the_users_place_and_the_scripts_limits() {
        let current = vec![area("a", 6, 4, 3, 2), area("gone", 0, 0, 1, 1)];
        let seeds = vec![
            area("a", 0, 0, 2, 2).min_span(3, 2).locked(true),
            area("new", 0, 0, 1, 1),
        ];

        let next = reconcile(&current, &seeds).unwrap();
        let a = next.iter().find(|area| area.id == "a").unwrap();
        // The place is the user's; the limits and lock are the script's.
        assert_eq!((a.col, a.row, a.col_span, a.row_span), (6, 4, 3, 2));
        assert_eq!(a.min_col_span, 3);
        assert!(a.locked);
        assert!(next.iter().any(|area| area.id == "new"));
        assert!(!next.iter().any(|area| area.id == "gone"));

        // Once the state holds it, nothing is reapplied on the next frame,
        // even after settling clamped the place to the new limits.
        assert_eq!(reconcile(&next, &seeds), None);
        let mut settled = next.clone();
        settled[0].col_span = 5;
        assert_eq!(reconcile(&settled, &seeds), None);
        // A limit changed on a later render reaches an item already placed.
        let unlocked = vec![
            area("a", 0, 0, 2, 2).min_span(3, 2),
            area("new", 0, 0, 1, 1),
        ];
        assert!(!reconcile(&next, &unlocked).unwrap()[0].locked);
    }

    #[test]
    fn the_layout_reaches_a_script_as_tuples() {
        assert_eq!(
            layout_argument(&[area("revenue", 1, 2, 3, 4)]),
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
