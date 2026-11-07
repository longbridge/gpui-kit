use gpui_component::{
    IndexPath,
    list::{List, ListDelegate, ListItem, ListState},
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDataCallback,
    ComponentDataValue, ComponentDelegateSnapshot, ComponentDescriptor, ComponentElementCallback,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, RegistryError, anyhow,
    gpui::{
        self, App, AppContext as _, Entity, IntoElement as _, ParentElement as _, Refineable as _,
        RenderOnce, SharedString, Styled as _, Window,
    },
};
use std::sync::Arc;

#[derive(Clone)]
struct Payload {
    id: String,
    rows: ComponentArgument,
    render_row: ComponentArgument,
}

#[derive(Clone)]
struct Delegate {
    rows: ComponentDelegateSnapshot,
    render_row: ComponentElementCallback,
    selected: Option<IndexPath>,
}

impl Delegate {
    fn row_id(row: &ComponentDataValue, index: usize) -> SharedString {
        if let ComponentDataValue::Object(fields) = row {
            if let Some((_, ComponentDataValue::String(id))) =
                fields.iter().find(|(name, _)| name == "id")
            {
                return id.clone().into();
            }
        }
        format!("row-{index}").into()
    }
}

impl ListDelegate for Delegate {
    type Item = ListItem;

    fn items_count(&self, section: usize, _: &App) -> usize {
        usize::from(section == 0) * self.rows.len()
    }

    fn render_item(
        &mut self,
        path: IndexPath,
        window: &mut Window,
        cx: &mut gpui::Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let row = match self.rows.row(path.row) {
            Ok(row) => row.clone(),
            Err(error) => {
                return Some(
                    ListItem::new(("invalid-row", path.row))
                        .child(format!("Failed to read List row: {error:#}")),
                );
            }
        };
        let id = Self::row_id(&row, path.row);
        let child = match self.render_row.build_data_with(&[row], window, cx) {
            Ok(Some(element)) => {
                #[cfg(test)]
                test_probe::row(id.to_string());
                element
            }
            Ok(None) => gpui::div().into_any_element(),
            Err(error) => gpui::div()
                .child(format!("Failed to render List row: {error:#}"))
                .into_any_element(),
        };
        Some(
            ListItem::new(id)
                .selected(self.selected == Some(path))
                .child(child),
        )
    }

    fn set_selected_index(
        &mut self,
        path: Option<IndexPath>,
        _: &mut Window,
        _: &mut gpui::Context<ListState<Self>>,
    ) {
        self.selected = path;
    }
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_probe {
    use std::cell::RefCell;

    thread_local! {
        static ROWS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn row(id: String) {
        ROWS.with(|rows| rows.borrow_mut().push(id));
    }

    pub(crate) fn take_rows() -> Vec<String> {
        ROWS.with(|rows| std::mem::take(&mut *rows.borrow_mut()))
    }
}

#[derive(gpui::IntoElement)]
struct BoundList {
    id: String,
    rows: ComponentDataCallback,
    render_row: ComponentElementCallback,
    style: gpui::StyleRefinement,
}

impl RenderOnce for BoundList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl gpui::IntoElement {
        let snapshot = match self.rows.snapshot_rows_with(&[], window, cx) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return gpui::div()
                    .child(format!("Failed to snapshot List rows: {error:#}"))
                    .into_any_element();
            }
        };
        let next_snapshot = snapshot.clone();
        let renderer = self.render_row.clone();
        let state_holder: Entity<Entity<ListState<Delegate>>> =
            window.use_keyed_state(self.id, cx, move |window, cx| {
                cx.new(|cx| {
                    ListState::new(
                        Delegate {
                            rows: snapshot,
                            render_row: renderer,
                            selected: None,
                        },
                        window,
                        cx,
                    )
                })
            });
        let state = state_holder.read(cx).clone();
        state.update(cx, |state, _| {
            state.delegate_mut().rows = next_snapshot;
            state.delegate_mut().render_row = self.render_row;
        });
        let mut list = List::new(&state);
        list.style().refine(&self.style);
        list.into_any_element()
    }
}

struct Materializer;

impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<Payload>()
            .ok_or_else(|| anyhow::anyhow!("List incompatible payload"))?
            .clone();
        let rows = request.resolve_data_callback(&payload.rows)?;
        let render_row = request.resolve_element_callback(&payload.render_row)?;
        anyhow::ensure!(
            request.take_typed_children()?.is_empty(),
            "List does not accept children; rows come from its immutable delegate snapshot"
        );
        Ok(BoundList {
            id: payload.id,
            rows,
            render_row,
            style: request.take_style(),
        }
        .into_any_element())
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor::new("List", Arc::new(Materializer))
.with_constructors(vec![ConstructorDescriptor::new(
            "List",
            vec![
                ArgumentDescriptor::new("id", ArgumentSchema::String),
                ArgumentDescriptor::new(
                    "rows",
                    ArgumentSchema::Callback("() => readonly unknown[]"),
                ),
                ArgumentDescriptor::new(
                    "render_row",
                    ArgumentSchema::Callback("(row: unknown) => Element | null"),
                ),
            ],
            |args| match args {
                [ComponentArgument::String(id), rows @ ComponentArgument::Callback(_), render_row @ ComponentArgument::Callback(_)]
                    if !id.trim().is_empty() => Ok(ComponentPayload::new(Payload {
                        id: id.clone(),
                        rows: rows.clone(),
                        render_row: render_row.clone(),
                    })),
                _ => Err("List expects a non-empty id, rows callback, and row renderer".into()),
            },
        )])
.with_methods(vec![])
.with_documentation(
            "Native retained List backed by an immutable rows snapshot. Each row is lazily rendered; object rows should provide a stable string `id`.",
        ))?;
    Ok(())
}
