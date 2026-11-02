//! Retained native DataTable binding with immutable row snapshots and lazy cells.

use super::support::bool_method;

use gpui_component::table::{Column, DataTable, TableDelegate, TableState};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDataValue,
    ComponentDelegateSnapshot, ComponentDescriptor, ComponentElementCallback,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, StateDescriptor, anyhow,
    gpui::{
        self, AppContext as _, Entity, IntoElement as _, ParentElement as _, Refineable as _,
        RenderOnce, StyleRefinement, Styled as _,
    },
};
use std::sync::Arc;

struct Delegate {
    columns: Vec<Column>,
    rows: ComponentDelegateSnapshot,
    render_cell: Option<ComponentElementCallback>,
}

impl Delegate {
    fn new(keys: Vec<String>) -> Self {
        Self {
            columns: keys
                .iter()
                .map(|key| Column::new(key.clone(), key.clone()))
                .collect(),
            rows: ComponentDelegateSnapshot::new(Vec::new()),
            render_cell: None,
        }
    }
}

impl TableDelegate for Delegate {
    fn columns_count(&self, _: &gpui::App) -> usize {
        self.columns.len()
    }
    fn rows_count(&self, _: &gpui::App) -> usize {
        self.rows.len()
    }
    fn column(&self, col_ix: usize, _: &gpui::App) -> Column {
        self.columns[col_ix].clone()
    }
    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<TableState<Self>>,
    ) -> impl gpui::IntoElement {
        let result = (|| {
            let callback = self
                .render_cell
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("DataTable cell renderer is unavailable"))?;
            let row = self.rows.row(row_ix)?.clone();
            let key = self
                .columns
                .get(col_ix)
                .ok_or_else(|| anyhow::anyhow!("DataTable column index {col_ix} is out of bounds"))?
                .key
                .to_string();
            callback.build_data_with(&[row, ComponentDataValue::String(key)], window, cx)
        })();
        match result {
            Ok(Some(element)) => {
                #[cfg(test)]
                test_probe::built();
                element
            }
            Ok(None) => gpui::div().into_any_element(),
            Err(error) => {
                #[cfg(test)]
                test_probe::error(error.to_string());
                gpui::div()
                    .child(format!("Failed to render DataTable cell: {error:#}"))
                    .into_any_element()
            }
        }
    }
    fn cell_text(&self, row_ix: usize, col_ix: usize, _: &gpui::App) -> String {
        let Some(column) = self.columns.get(col_ix) else {
            return String::new();
        };
        let Ok(ComponentDataValue::Object(fields)) = self.rows.row(row_ix) else {
            return String::new();
        };
        fields
            .iter()
            .find_map(|(key, value)| {
                (key == column.key.as_ref()).then(|| match value {
                    ComponentDataValue::String(value) => value.clone(),
                    ComponentDataValue::Number(value) => value.to_string(),
                    ComponentDataValue::Boolean(value) => value.to_string(),
                    _ => String::new(),
                })
            })
            .unwrap_or_default()
    }
}

#[derive(Clone)]
struct Payload {
    state: ComponentArgument,
    rows: ComponentArgument,
    cell: ComponentArgument,
}
#[derive(Clone, Copy)]
enum Op {
    Stripe(bool),
    Bordered(bool),
    Scrollbars(bool, bool),
    RowSelectable(bool),
    ColSelectable(bool),
    CellSelectable(bool),
    RowHeader(bool),
    Sortable(bool),
    ColResizable(bool),
    ColMovable(bool),
}

struct Materializer;
impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<Payload>()
            .ok_or_else(|| anyhow::anyhow!("DataTable received an incompatible payload"))?
            .clone();
        anyhow::ensure!(
            request.children_len() == 0,
            "DataTable does not accept children"
        );
        let state =
            request.with_state::<Entity<TableState<Delegate>>, _>(&payload.state, Clone::clone)?;
        let rows = request.resolve_data_callback(&payload.rows)?;
        let cell = request.resolve_element_callback(&payload.cell)?;
        let ops = request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<Op>().copied())
            .collect::<Vec<_>>();
        let style = request.take_style();
        Ok(DataTableHost {
            state,
            rows,
            cell,
            ops,
            style,
        }
        .into_any_element())
    }
}

#[derive(gpui::IntoElement)]
struct DataTableHost {
    state: Entity<TableState<Delegate>>,
    rows: gpui_shell::ComponentDataCallback,
    cell: ComponentElementCallback,
    ops: Vec<Op>,
    style: StyleRefinement,
}

impl RenderOnce for DataTableHost {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let snapshot = match self.rows.snapshot_rows_with(&[], window, cx) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                let message =
                    format!("DataTable rows callback must return an array of rows: {error:#}");
                #[cfg(test)]
                test_probe::error(message.clone());
                return gpui::div().child(message).into_any_element();
            }
        };
        self.state.update(cx, |state, cx| {
            state.delegate_mut().rows = snapshot;
            state.delegate_mut().render_cell = Some(self.cell);
            for op in &self.ops {
                match op {
                    Op::RowSelectable(value) => state.row_selectable = *value,
                    Op::ColSelectable(value) => state.col_selectable = *value,
                    Op::CellSelectable(value) => state.cell_selectable = *value,
                    Op::RowHeader(value) => state.row_header = *value,
                    Op::Sortable(value) => state.sortable = *value,
                    Op::ColResizable(value) => state.col_resizable = *value,
                    Op::ColMovable(value) => state.col_movable = *value,
                    _ => {}
                }
            }
            state.refresh(cx);
        });
        let mut table = DataTable::new(&self.state);
        for op in &self.ops {
            table = match op {
                Op::Stripe(value) => table.stripe(*value),
                Op::Bordered(value) => table.bordered(*value),
                Op::Scrollbars(value, h) => table.scrollbar_visible(*value, *h),
                _ => table,
            };
        }
        let mut host = gpui::div().size_full().child(table);
        host.style().refine(&self.style);
        host.into_any_element()
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register_state(
        StateDescriptor::new(
            "DataTableState",
            "DataTableState",
            vec![ArgumentDescriptor::new(
                "columns",
                ArgumentSchema::Array(Box::new(ArgumentSchema::String)),
            )],
            |args, window, cx| match args {
                [ComponentArgument::Array(columns)] => {
                    let keys = columns
                        .iter()
                        .map(|column| match column {
                            ComponentArgument::String(key) if !key.trim().is_empty() => {
                                Ok(key.clone())
                            }
                            _ => Err("DataTableState columns must be non-empty strings".into()),
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    if keys.is_empty() {
                        return Err("DataTableState requires at least one column".into());
                    }
                    let mut unique = std::collections::HashSet::new();
                    if !keys.iter().all(|key| unique.insert(key.clone())) {
                        return Err("DataTableState column keys must be unique".into());
                    }
                    Ok(Box::new(cx.new(|cx| {
                        TableState::new(Delegate::new(keys), window, cx)
                    })))
                }
                _ => Err("DataTableState expects a string array".into()),
            },
        )
        .with_documentation(
            "Retained native DataTable focus, selection, scrolling, measurement and column state.",
        ),
    )?;
    registry.register(ComponentDescriptor::new("DataTable", Arc::new(Materializer))
.with_constructors(vec![ConstructorDescriptor::new("DataTable", vec![
            ArgumentDescriptor::new("state", ArgumentSchema::Entity("DataTableState")),
            ArgumentDescriptor::new("rows", ArgumentSchema::Callback("(cx: Context) => readonly unknown[]")),
            ArgumentDescriptor::new("render_cell", ArgumentSchema::Callback("(row: unknown, column: string, cx: Context) => Element")),
        ], |args| match args { [state @ ComponentArgument::Entity { .. }, rows @ ComponentArgument::Callback(_), cell @ ComponentArgument::Callback(_)] => Ok(ComponentPayload::new(Payload { state: state.clone(), rows: rows.clone(), cell: cell.clone() })), _ => Err("DataTable expects DataTableState, rows callback and cell renderer".into()) })])
.with_methods(vec![
            bool_method("DataTable", "stripe", "Sets native DataTable behavior.", Op::Stripe), bool_method("DataTable", "bordered", "Sets native DataTable behavior.", Op::Bordered),
            MethodDescriptor::new("scrollbar_visible", vec![ArgumentDescriptor::new("vertical", ArgumentSchema::Boolean), ArgumentDescriptor::new("horizontal", ArgumentSchema::Boolean)], |args| match args { [ComponentArgument::Boolean(value), ComponentArgument::Boolean(h)] => Ok(ComponentPayload::new(Op::Scrollbars(*value, *h))), _ => Err("DataTable.scrollbar_visible expects two booleans".into()) }).with_documentation("Chooses when the table shows its scrollbars."),
            bool_method("DataTable", "row_selectable", "Sets native DataTable behavior.", Op::RowSelectable), bool_method("DataTable", "column_selectable", "Sets native DataTable behavior.", Op::ColSelectable), bool_method("DataTable", "cell_selectable", "Sets native DataTable behavior.", Op::CellSelectable), bool_method("DataTable", "row_header", "Sets native DataTable behavior.", Op::RowHeader), bool_method("DataTable", "sortable", "Sets native DataTable behavior.", Op::Sortable), bool_method("DataTable", "column_resizable", "Sets native DataTable behavior.", Op::ColResizable), bool_method("DataTable", "column_movable", "Sets native DataTable behavior.", Op::ColMovable),
        ])
.with_documentation("A real retained native DataTable. Rows are captured as an immutable plain-data snapshot and visible cells are built lazily from (row, column). Style applies to the full-size table host."))?;
    Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_probe {
    use std::cell::Cell;
    thread_local! { static BUILDS: Cell<usize> = const { Cell::new(0) }; static ERRORS: Cell<usize> = const { Cell::new(0) }; }
    pub(super) fn built() {
        BUILDS.with(|value| value.set(value.get() + 1));
    }
    pub(super) fn error(_: String) {
        ERRORS.with(|value| value.set(value.get() + 1));
    }
    pub(crate) fn reset() {
        BUILDS.with(|value| value.set(0));
        ERRORS.with(|value| value.set(0));
    }
    pub(crate) fn cell_builds() -> usize {
        BUILDS.with(Cell::get)
    }
    pub(crate) fn errors() -> usize {
        ERRORS.with(Cell::get)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_is_retained_data_table_only() {
        let mut registry = ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        assert_eq!(
            registry
                .freeze()
                .unwrap()
                .descriptors()
                .map(|d| d.name())
                .collect::<Vec<_>>(),
            ["DataTable"]
        );
    }
}
