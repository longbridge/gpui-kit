use gpui_component::{
    IndexPath,
    searchable_list::{SearchableListDelegate, SearchableListItem},
    select::{Select, SelectEvent, SelectState},
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallback,
    ComponentCallbackArgument, ComponentDataCallback, ComponentDataValue,
    ComponentDelegateSnapshot, ComponentDescriptor, ComponentElementCallback,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{
        self, App, AppContext as _, Entity, IntoElement as _, ParentElement as _, Refineable as _,
        RenderOnce, SharedString, Styled as _, Subscription, Window,
    },
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[derive(Clone)]
struct Payload {
    id: String,
    rows: ComponentArgument,
    render_row: ComponentArgument,
    on_select: ComponentArgument,
}

#[derive(Clone)]
enum Op {
    Placeholder(String),
    MenuWidth(f32),
    Disabled(bool),
}

#[derive(Clone)]
struct Item {
    id: String,
    title: SharedString,
    disabled: bool,
    row: ComponentDataValue,
    renderer: ComponentElementCallback,
}

impl SearchableListItem for Item {
    type Value = String;

    fn title(&self) -> SharedString {
        self.title.clone()
    }
    fn value(&self) -> &Self::Value {
        &self.id
    }
    fn disabled(&self) -> bool {
        self.disabled
    }
    fn render(&self, window: &mut Window, cx: &mut App) -> impl gpui::IntoElement {
        match self
            .renderer
            .build_data_with(std::slice::from_ref(&self.row), window, cx)
        {
            Ok(Some(element)) => element,
            Ok(None) => gpui::div().child(self.title.clone()).into_any_element(),
            Err(error) => gpui::div()
                .child(format!("Failed to render Select row: {error:#}"))
                .into_any_element(),
        }
    }
}

#[derive(Clone)]
struct Delegate(Vec<Item>);
impl SearchableListDelegate for Delegate {
    type Item = Item;
    fn items_count(&self, section: usize) -> usize {
        usize::from(section == 0) * self.0.len()
    }
    fn item(&self, path: IndexPath) -> Option<&Self::Item> {
        (path.section == 0).then(|| self.0.get(path.row)).flatten()
    }
    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SearchableListItem<Value = V>,
        V: PartialEq,
    {
        self.0
            .iter()
            .position(|item| item.value() == value)
            .map(IndexPath::new)
    }
}

fn field<'a>(row: &'a ComponentDataValue, name: &str) -> Option<&'a ComponentDataValue> {
    let ComponentDataValue::Object(fields) = row else {
        return None;
    };
    fields
        .iter()
        .find_map(|(field, value)| (field == name).then_some(value))
}

fn delegate(
    snapshot: &ComponentDelegateSnapshot,
    renderer: &ComponentElementCallback,
) -> anyhow::Result<Delegate> {
    let mut items = Vec::with_capacity(snapshot.len());
    for index in 0..snapshot.len() {
        let row = snapshot.row(index)?.clone();
        let Some(ComponentDataValue::String(id)) = field(&row, "id") else {
            anyhow::bail!("Select row {index} requires a string `id`");
        };
        let Some(ComponentDataValue::String(title)) = field(&row, "label") else {
            anyhow::bail!("Select row {index} requires a string `label`");
        };
        let disabled = matches!(
            field(&row, "disabled"),
            Some(ComponentDataValue::Boolean(true))
        );
        items.push(Item {
            id: id.clone(),
            title: title.clone().into(),
            disabled,
            row,
            renderer: renderer.clone(),
        });
    }
    Ok(Delegate(items))
}

struct Host {
    state: Entity<SelectState<Delegate>>,
    callback: Rc<RefCell<ComponentCallback>>,
    _selection: Subscription,
}

#[derive(gpui::IntoElement)]
struct BoundSelect {
    id: String,
    rows: ComponentDataCallback,
    renderer: ComponentElementCallback,
    on_select: ComponentCallback,
    ops: Vec<Op>,
    style: gpui::StyleRefinement,
}

impl RenderOnce for BoundSelect {
    fn render(self, window: &mut Window, cx: &mut App) -> impl gpui::IntoElement {
        let snapshot = match self.rows.snapshot_rows_with(&[], window, cx) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return gpui::div()
                    .child(format!("Failed to snapshot Select rows: {error:#}"))
                    .into_any_element();
            }
        };
        let next = match delegate(&snapshot, &self.renderer) {
            Ok(delegate) => delegate,
            Err(error) => {
                return gpui::div()
                    .child(format!("Invalid Select rows: {error:#}"))
                    .into_any_element();
            }
        };
        let initial = next.clone();
        let callback = self.on_select.clone();
        let host: Entity<Host> = window.use_keyed_state(
            format!("shell-select:{}", self.id),
            cx,
            move |window, cx| {
                let state = cx.new(|cx| SelectState::new(initial, None, window, cx));
                let callback = Rc::new(RefCell::new(callback));
                let event_callback = callback.clone();
                let selection = window.subscribe(
                    &state,
                    cx,
                    move |_, event: &SelectEvent<Delegate>, window, cx| {
                        let SelectEvent::Confirm(value) = event;
                        if let Some(value) = value {
                            #[cfg(test)]
                            test_probe::selected(value.clone());
                            let callback = event_callback.borrow().clone();
                            callback.invoke_and_report_with(
                                "Select.on_select",
                                &[ComponentCallbackArgument::String(value.clone())],
                                window,
                                cx,
                            );
                        }
                    },
                );
                Host {
                    state,
                    callback,
                    _selection: selection,
                }
            },
        );
        let (state, callback) = {
            let host = host.read(cx);
            (host.state.clone(), host.callback.clone())
        };
        *callback.borrow_mut() = self.on_select;
        state.update(cx, |state, cx| {
            let selected = state.selected_value().cloned();
            state.set_items(next, window, cx);
            if let Some(selected) = selected {
                state.set_selected_value(&selected, window, cx);
            }
        });
        let mut select = Select::new(&state);
        for op in self.ops {
            select = match op {
                Op::Placeholder(value) => select.placeholder(value),
                Op::MenuWidth(value) => select.menu_width(gpui::px(value)),
                Op::Disabled(value) => select.disabled(value),
            };
        }
        select.style().refine(&self.style);
        select.into_any_element()
    }
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_probe {
    use std::cell::RefCell;
    thread_local! { static SELECTED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) }; }
    pub(super) fn selected(value: String) {
        SELECTED.with(|values| values.borrow_mut().push(value));
    }
    pub(crate) fn take_selected() -> Vec<String> {
        SELECTED.with(|values| std::mem::take(&mut *values.borrow_mut()))
    }
}

struct Materializer;
impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<Payload>()
            .ok_or_else(|| anyhow::anyhow!("Select incompatible payload"))?
            .clone();
        anyhow::ensure!(
            request.take_typed_children()?.is_empty(),
            "Select does not accept children"
        );
        let rows = request.resolve_data_callback(&payload.rows)?;
        let renderer = request.resolve_element_callback(&payload.render_row)?;
        let on_select = request.resolve_callback(&payload.on_select)?;
        let ops = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<Op>().cloned())
            .collect();
        Ok(BoundSelect {
            id: payload.id,
            rows,
            renderer,
            on_select,
            ops,
            style: request.take_style(),
        }
        .into_any_element())
    }
}

fn method(
    name: &'static str,
    documentation: &'static str,
    schema: ArgumentSchema,
    make: fn(&ComponentArgument) -> Option<Op>,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, schema)],
        move |args| {
            args.first()
                .and_then(make)
                .map(ComponentPayload::new)
                .ok_or_else(|| format!("Select.{name} received an invalid value"))
        },
    )
    .with_documentation(documentation)
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor::new("Select", Arc::new(Materializer))
.with_constructors(vec![ConstructorDescriptor::new("Select", vec![
            ArgumentDescriptor::new("id", ArgumentSchema::String),
            ArgumentDescriptor::new("rows", ArgumentSchema::Callback("() => readonly { id: string; label: string; disabled?: boolean }[]")),
            ArgumentDescriptor::new("render_row", ArgumentSchema::Callback("(row: unknown) => Element | null")),
            ArgumentDescriptor::new("on_select", ArgumentSchema::Callback("(value: string, cx: Context) => void")),
        ], |args| match args {
            [ComponentArgument::String(id), rows @ ComponentArgument::Callback(_), render_row @ ComponentArgument::Callback(_), on_select @ ComponentArgument::Callback(_)] if !id.trim().is_empty() => Ok(ComponentPayload::new(Payload { id:id.clone(), rows:rows.clone(), render_row:render_row.clone(), on_select:on_select.clone() })),
            _ => Err("Select expects id, rows callback, row renderer, and selection callback".into()),
        })])
.with_methods(vec![
            method("placeholder", "Sets the text shown while nothing is selected.", ArgumentSchema::String, |arg| match arg { ComponentArgument::String(value) => Some(Op::Placeholder(value.clone())), _ => None }),
            method("menu_width", "Sets the popup menu width in pixels.", ArgumentSchema::Number, |arg| match arg { ComponentArgument::Number(value) if value.is_finite() && *value > 0.0 && *value <= f32::MAX as f64 => Some(Op::MenuWidth(*value as f32)), _ => None }),
            method("disabled", "Disables the select.", ArgumentSchema::Boolean, |arg| match arg { ComponentArgument::Boolean(value) => Some(Op::Disabled(*value)), _ => None }),
        ])
.with_documentation("Native retained single-value Select backed by immutable `{id,label,disabled?}` snapshots and a lazy row renderer."))?;
    Ok(())
}
