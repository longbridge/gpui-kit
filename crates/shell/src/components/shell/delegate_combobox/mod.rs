use gpui_component::{
    IndexPath,
    combobox::{Combobox, ComboboxEvent, ComboboxState},
    searchable_list::{SearchableListDelegate, SearchableListItem},
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallback,
    ComponentCallbackArgument, ComponentDataCallback, ComponentDataValue, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{
        self, App, AppContext as _, Entity, IntoElement as _, ParentElement as _, Refineable as _,
        RenderOnce, SharedString, Styled as _, Subscription, Task, Window,
    },
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[derive(Clone)]
struct Payload {
    id: String,
    rows: ComponentArgument,
    on_change: ComponentArgument,
    on_confirm: ComponentArgument,
}
#[derive(Clone)]
enum Op {
    Placeholder(String),
    SearchPlaceholder(String),
    Searchable(bool),
    Disabled(bool),
    MenuWidth(f32),
}
#[derive(Clone, PartialEq, Eq)]
struct Item {
    id: String,
    label: SharedString,
    disabled: bool,
}
impl SearchableListItem for Item {
    type Value = String;
    fn title(&self) -> SharedString {
        self.label.clone()
    }
    fn value(&self) -> &Self::Value {
        &self.id
    }
    fn disabled(&self) -> bool {
        self.disabled
    }
}
#[derive(Clone, PartialEq, Eq)]
struct Delegate {
    all: Vec<Item>,
    visible: Vec<Item>,
}
impl SearchableListDelegate for Delegate {
    type Item = Item;
    fn items_count(&self, section: usize) -> usize {
        usize::from(section == 0) * self.visible.len()
    }
    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        (ix.section == 0)
            .then(|| self.visible.get(ix.row))
            .flatten()
    }
    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SearchableListItem<Value = V>,
        V: PartialEq,
    {
        self.visible
            .iter()
            .position(|item| item.value() == value)
            .map(IndexPath::new)
    }
    fn perform_search(&mut self, query: &str, _: &mut Window, _: &mut App) -> Task<()> {
        self.visible = self
            .all
            .iter()
            .filter(|item| item.matches(query))
            .cloned()
            .collect();
        Task::ready(())
    }
}

fn field<'a>(row: &'a ComponentDataValue, name: &str) -> Option<&'a ComponentDataValue> {
    let ComponentDataValue::Object(fields) = row else {
        return None;
    };
    fields
        .iter()
        .find_map(|(key, value)| (key == name).then_some(value))
}
fn snapshot(
    callback: &ComponentDataCallback,
    window: &mut Window,
    cx: &mut App,
) -> anyhow::Result<Delegate> {
    let rows = callback.snapshot_rows_with(&[], window, cx)?;
    let mut items = Vec::with_capacity(rows.len());
    for index in 0..rows.len() {
        let row = rows.row(index)?;
        let Some(ComponentDataValue::String(id)) = field(row, "id") else {
            anyhow::bail!("Combobox row {index} requires a string `id`");
        };
        let Some(ComponentDataValue::String(label)) = field(row, "label") else {
            anyhow::bail!("Combobox row {index} requires a string `label`");
        };
        items.push(Item {
            id: id.clone(),
            label: label.clone().into(),
            disabled: matches!(
                field(row, "disabled"),
                Some(ComponentDataValue::Boolean(true))
            ),
        });
    }
    Ok(Delegate {
        all: items.clone(),
        visible: items,
    })
}

struct Callbacks {
    change: ComponentCallback,
    confirm: ComponentCallback,
}
struct Host {
    state: Entity<ComboboxState<Delegate>>,
    rows: RefCell<Delegate>,
    callbacks: Rc<RefCell<Callbacks>>,
    _change: Subscription,
    _confirm: Subscription,
}
#[derive(gpui::IntoElement)]
struct Bound {
    id: String,
    rows: ComponentDataCallback,
    change: ComponentCallback,
    confirm: ComponentCallback,
    ops: Vec<Op>,
    style: gpui::StyleRefinement,
}
fn report_values(
    callback: &ComponentCallback,
    phase: &'static str,
    values: &[String],
    window: &mut Window,
    cx: &mut App,
) {
    if let Some(value) = values.first() {
        callback.invoke_and_report_with(
            phase,
            &[ComponentCallbackArgument::String(value.clone())],
            window,
            cx,
        );
    }
}
impl RenderOnce for Bound {
    fn render(self, window: &mut Window, cx: &mut App) -> impl gpui::IntoElement {
        let next = match snapshot(&self.rows, window, cx) {
            Ok(next) => next,
            Err(error) => {
                return gpui::div()
                    .child(format!("Invalid Combobox rows: {error:#}"))
                    .into_any_element();
            }
        };
        let initial = next.clone();
        let searchable = self
            .ops
            .iter()
            .filter_map(|op| match op {
                Op::Searchable(value) => Some(*value),
                _ => None,
            })
            .next_back()
            .unwrap_or(true);
        let initial_callbacks = Callbacks {
            change: self.change.clone(),
            confirm: self.confirm.clone(),
        };
        let host: Entity<Host> = window.use_keyed_state(
            format!("shell-combobox:{}:{searchable}", self.id),
            cx,
            move |window, cx| {
                let state = cx.new(|cx| {
                    ComboboxState::new(initial.clone(), Vec::new(), window, cx)
                        .multiple(false)
                        .searchable(searchable)
                });
                let callbacks = Rc::new(RefCell::new(initial_callbacks));
                let changes = callbacks.clone();
                let change = window.subscribe(
                    &state,
                    cx,
                    move |_, event: &ComboboxEvent<Delegate>, window, cx| {
                        if let ComboboxEvent::Change(values) = event {
                            #[cfg(test)]
                            test_probe::change(values.clone());
                            let callback = changes.borrow().change.clone();
                            report_values(&callback, "Combobox.on_change", values, window, cx);
                        }
                    },
                );
                let confirms = callbacks.clone();
                let confirm = window.subscribe(
                    &state,
                    cx,
                    move |_, event: &ComboboxEvent<Delegate>, window, cx| {
                        if let ComboboxEvent::Confirm(values) = event {
                            #[cfg(test)]
                            test_probe::confirm(values.clone());
                            let callback = confirms.borrow().confirm.clone();
                            report_values(&callback, "Combobox.on_confirm", values, window, cx);
                        }
                    },
                );
                Host {
                    state,
                    rows: RefCell::new(initial),
                    callbacks,
                    _change: change,
                    _confirm: confirm,
                }
            },
        );
        let (state, callbacks, rows_changed) = {
            let host = host.read(cx);
            let rows_changed = *host.rows.borrow() != next;
            if rows_changed {
                *host.rows.borrow_mut() = next.clone();
            }
            (host.state.clone(), host.callbacks.clone(), rows_changed)
        };
        *callbacks.borrow_mut() = Callbacks {
            change: self.change,
            confirm: self.confirm,
        };
        if rows_changed {
            state.update(cx, |state, cx| {
                let selected = state.selected_value();
                state.set_items(next, window, cx);
                if let Some(selected) = selected {
                    state.set_selected_values(&[selected], window, cx);
                }
            });
        }
        let mut combobox = Combobox::new(&state);
        for op in self.ops {
            combobox = match op {
                Op::Placeholder(value) => combobox.placeholder(value),
                Op::SearchPlaceholder(value) => combobox.search_placeholder(value),
                Op::Searchable(_) => combobox,
                Op::Disabled(value) => combobox.disabled(value),
                Op::MenuWidth(value) => combobox.menu_width(gpui::px(value)),
            };
        }
        combobox.style().refine(&self.style);
        combobox.into_any_element()
    }
}

struct ComboboxMaterializer;
impl ComponentMaterializer for ComboboxMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<Payload>()
            .ok_or_else(|| anyhow::anyhow!("Combobox incompatible payload"))?
            .clone();
        anyhow::ensure!(
            request.take_typed_children()?.is_empty(),
            "Combobox does not accept children"
        );
        let rows = request.resolve_data_callback(&payload.rows)?;
        let change = request.resolve_callback(&payload.on_change)?;
        let confirm = request.resolve_callback(&payload.on_confirm)?;
        let ops = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<Op>().cloned())
            .collect();
        Ok(Bound {
            id: payload.id,
            rows,
            change,
            confirm,
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
                .ok_or_else(|| format!("Combobox.{name} received an invalid value"))
        },
    )
    .with_documentation(documentation)
}
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor::new("Combobox", Arc::new(ComboboxMaterializer))
.with_constructors(vec![ConstructorDescriptor::new("Combobox", vec![
            ArgumentDescriptor::new("id", ArgumentSchema::String),
            ArgumentDescriptor::new("rows", ArgumentSchema::Callback("() => readonly { id: string; label: string; disabled?: boolean }[]")),
            ArgumentDescriptor::new("on_change", ArgumentSchema::Callback("(value: string, cx: Context) => void")),
            ArgumentDescriptor::new("on_confirm", ArgumentSchema::Callback("(value: string, cx: Context) => void")),
        ], |args| match args {
            [ComponentArgument::String(id), rows @ ComponentArgument::Callback(_), change @ ComponentArgument::Callback(_), confirm @ ComponentArgument::Callback(_)] if !id.trim().is_empty() => Ok(ComponentPayload::new(Payload { id:id.clone(), rows:rows.clone(), on_change:change.clone(), on_confirm:confirm.clone() })),
            _ => Err("Combobox expects id, rows, on_change, and on_confirm callbacks".into()),
        })])
.with_methods(vec![
            method("placeholder", "Sets the text shown while nothing is selected.", ArgumentSchema::String, |arg| match arg { ComponentArgument::String(value) => Some(Op::Placeholder(value.clone())), _ => None }),
            method("search_placeholder", "Sets the text shown in the empty search field.", ArgumentSchema::String, |arg| match arg { ComponentArgument::String(value) => Some(Op::SearchPlaceholder(value.clone())), _ => None }),
            method("searchable", "Shows the search field above the item list.", ArgumentSchema::Boolean, |arg| match arg { ComponentArgument::Boolean(value) => Some(Op::Searchable(*value)), _ => None }),
            method("disabled", "Disables the combobox.", ArgumentSchema::Boolean, |arg| match arg { ComponentArgument::Boolean(value) => Some(Op::Disabled(*value)), _ => None }),
            method("menu_width", "Sets the popup menu width in pixels.", ArgumentSchema::Number, |arg| match arg { ComponentArgument::Number(value) if value.is_finite() && *value > 0.0 && *value <= f32::MAX as f64 => Some(Op::MenuWidth(*value as f32)), _ => None }),
        ])
.with_documentation("Native retained single-select searchable Combobox backed by immutable `{id,label,disabled?}` snapshots."))?;
    Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_probe {
    use std::cell::RefCell;
    thread_local! {
        static CHANGES: RefCell<Vec<Vec<String>>> = const { RefCell::new(Vec::new()) };
        static CONFIRMS: RefCell<Vec<Vec<String>>> = const { RefCell::new(Vec::new()) };
    }
    pub(super) fn change(value: Vec<String>) {
        CHANGES.with(|x| x.borrow_mut().push(value));
    }
    pub(super) fn confirm(value: Vec<String>) {
        CONFIRMS.with(|x| x.borrow_mut().push(value));
    }
    pub(crate) fn take_changes() -> Vec<Vec<String>> {
        CHANGES.with(|x| std::mem::take(&mut *x.borrow_mut()))
    }
    pub(crate) fn take_confirms() -> Vec<Vec<String>> {
        CONFIRMS.with(|x| std::mem::take(&mut *x.borrow_mut()))
    }
}
