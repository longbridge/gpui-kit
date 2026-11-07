use super::{Carrier, take};
use super::{bool_method, require_child};
use gpui_component::{
    Icon, IconName, h_flex,
    list::ListItem,
    tree::{Tree, TreeItem, TreeState},
};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, RegistryError, anyhow,
    gpui::{
        self, AppContext as _, IntoElement as _, ParentElement as _, Refineable as _, Styled as _,
    },
};
use std::{collections::HashSet, sync::Arc};
#[derive(Clone)]
struct ItemPayload {
    id: String,
    label: String,
}
#[derive(Clone, Copy)]
enum ItemOp {
    Expanded(bool),
    Disabled(bool),
}
#[derive(Clone)]
struct TreePayload(String);
struct RetainedTree {
    native: gpui::Entity<TreeState>,
    fingerprint: Vec<ItemFingerprint>,
    roots: Vec<TreeItem>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct ItemFingerprint {
    id: String,
    label: String,
    expanded: bool,
    disabled: bool,
    children: Vec<ItemFingerprint>,
}
/// The shared check, plus the probe this module's tests read the message from.
fn require_tree_item(parent: &str, actual: Option<&'static str>) -> anyhow::Result<()> {
    if let Err(error) = require_child(parent, actual, &["TreeItem"]) {
        #[cfg(test)]
        test_probe::error(&error.to_string());
        return Err(error);
    }
    Ok(())
}
fn require_item_style(style: &gpui::StyleRefinement) -> anyhow::Result<()> {
    if style != &gpui::StyleRefinement::default() {
        let error = "TreeItem is data and does not support shell style";
        #[cfg(test)]
        test_probe::error(error);
        anyhow::bail!(error);
    }
    Ok(())
}
struct ItemMaterializer;
impl ComponentMaterializer for ItemMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let p = request
            .payload()
            .downcast_ref::<ItemPayload>()
            .ok_or_else(|| anyhow::anyhow!("TreeItem incompatible payload"))?;
        let mut item = TreeItem::new(p.id.clone(), p.label.clone());
        for op in request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<ItemOp>())
        {
            item = match op {
                ItemOp::Expanded(value) => item.expanded(*value),
                ItemOp::Disabled(value) => item.disabled(*value),
            }
        }
        let style = request.take_style();
        require_item_style(&style)?;
        for mut child in request.take_typed_children()? {
            require_tree_item("TreeItem", child.component_name())?;
            let mut element = request.materialize_child(&mut child)?;
            item = item.child(take::<TreeItem>(&mut element, "TreeItem")?);
        }
        Ok(Carrier::new(item).into_any_element())
    }
}
struct TreeMaterializer;
impl ComponentMaterializer for TreeMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = request
            .payload()
            .downcast_ref::<TreePayload>()
            .ok_or_else(|| anyhow::anyhow!("Tree incompatible payload"))?
            .0
            .clone();
        let mut items = Vec::new();
        for mut child in request.take_typed_children()? {
            require_tree_item("Tree", child.component_name())?;
            let mut element = request.materialize_child(&mut child)?;
            items.push(take::<TreeItem>(&mut element, "TreeItem")?);
        }
        validate_unique_ids(&items)?;
        let fingerprint = fingerprint(&items);
        let state = request.with_window_app(|window, cx| {
            let retained =
                window.use_keyed_state(format!("shell-tree:{id}"), cx, |_, cx| RetainedTree {
                    native: cx.new(|cx| TreeState::new(cx).items(items.clone())),
                    fingerprint: fingerprint.clone(),
                    roots: items.clone(),
                });
            retained.update(cx, |retained, cx| {
                if retained.fingerprint != fingerprint {
                    preserve_expansion(&mut items, &retained.roots);
                    let selected_id = retained
                        .native
                        .read(cx)
                        .selected_item()
                        .map(|item| item.id.clone());
                    retained.native.update(cx, |native, cx| {
                        native.set_items(items.clone(), cx);
                        let selected_ix = selected_id
                            .as_ref()
                            .and_then(|selected_id| native.index_of(selected_id));
                        native.set_selected_index(selected_ix, cx);
                    });
                    retained.fingerprint = fingerprint.clone();
                    retained.roots = items.clone();
                }
            });
            Ok(retained.read(cx).native.clone())
        })?;
        let mut tree = Tree::new(&state, move |_ix, entry, selected, _, _| {
            #[cfg(test)]
            test_probe::row(entry.item(), selected);
            ListItem::new(entry.item().id.clone())
                .selected(selected)
                .w_full()
                .px_3()
                .pl(gpui::px(16.) * entry.depth() + gpui::px(12.))
                .child(
                    h_flex()
                        .gap_2()
                        .child(Icon::new(if !entry.is_folder() {
                            IconName::File
                        } else if entry.is_expanded() {
                            IconName::FolderOpen
                        } else {
                            IconName::Folder
                        }))
                        .child(entry.item().label.clone()),
                )
        });
        tree.style().refine(&request.take_style());
        Ok(tree.into_any_element())
    }
}
fn validate_unique_ids(items: &[TreeItem]) -> anyhow::Result<()> {
    fn walk<'a>(seen: &mut HashSet<&'a str>, item: &'a TreeItem) -> anyhow::Result<()> {
        if !seen.insert(item.id.as_ref()) {
            let error = format!(
                "TreeItem id `{}` is duplicated; ids must be unique within a Tree",
                item.id
            );
            #[cfg(test)]
            test_probe::error(&error);
            anyhow::bail!(error);
        }
        for child in &item.children {
            walk(seen, child)?;
        }
        Ok(())
    }
    let mut seen = HashSet::new();
    for item in items {
        walk(&mut seen, item)?;
    }
    Ok(())
}
#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_probe {
    use super::TreeItem;
    use std::cell::RefCell;

    thread_local! {
        static ROWS: RefCell<Vec<(String, String, bool)>> = const { RefCell::new(Vec::new()) };
        static ERRORS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }
    pub(super) fn row(item: &TreeItem, selected: bool) {
        ROWS.with(|rows| {
            rows.borrow_mut()
                .push((item.id.to_string(), item.label.to_string(), selected))
        });
    }
    pub(super) fn error(error: &str) {
        ERRORS.with(|errors| errors.borrow_mut().push(error.to_owned()));
    }
    pub(crate) fn take_rows() -> Vec<(String, String, bool)> {
        ROWS.with(|rows| std::mem::take(&mut *rows.borrow_mut()))
    }
    pub(crate) fn take_errors() -> Vec<String> {
        ERRORS.with(|errors| std::mem::take(&mut *errors.borrow_mut()))
    }
}
fn preserve_expansion(incoming: &mut [TreeItem], previous: &[TreeItem]) {
    fn collect(items: &[TreeItem], states: &mut Vec<(gpui::SharedString, bool)>) {
        for item in items {
            states.push((item.id.clone(), item.is_expanded()));
            collect(&item.children, states);
        }
    }
    fn apply(items: &mut [TreeItem], states: &[(gpui::SharedString, bool)]) {
        for item in items {
            if let Some((_, expanded)) = states.iter().find(|(id, _)| id == &item.id) {
                *item = item.clone().expanded(*expanded);
            }
            apply(&mut item.children, states);
        }
    }
    let mut states = Vec::new();
    collect(previous, &mut states);
    apply(incoming, &states);
}
fn fingerprint(items: &[TreeItem]) -> Vec<ItemFingerprint> {
    items
        .iter()
        .map(|item| ItemFingerprint {
            id: item.id.to_string(),
            label: item.label.to_string(),
            expanded: item.is_expanded(),
            disabled: item.is_disabled(),
            children: fingerprint(&item.children),
        })
        .collect()
}
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor::new("TreeItem", Arc::new(ItemMaterializer))
.with_constructors(vec![ConstructorDescriptor::new(
            "TreeItem",
            vec![
                ArgumentDescriptor::new("id", ArgumentSchema::String),
                ArgumentDescriptor::new("label", ArgumentSchema::String),
            ],
            |arguments| match arguments {
                [
                    ComponentArgument::String(id),
                    ComponentArgument::String(label),
                ] if !id.trim().is_empty() && !label.trim().is_empty() => {
                    Ok(ComponentPayload::new(ItemPayload {
                        id: id.clone(),
                        label: label.clone(),
                    }))
                }
                _ => Err("TreeItem expects non-empty id and label".into()),
            },
        )])
.with_methods(vec![
            bool_method("TreeItem", "expanded", "Sets native tree item state.", ItemOp::Expanded),
            bool_method("TreeItem", "disabled", "Sets native tree item state.", ItemOp::Disabled),
        ])
.with_documentation(
            "Typed native tree data item with a Tree-wide unique id, nested TreeItem children, and initial expanded/disabled state; style is rejected.",
        ))?;
    registry.register(ComponentDescriptor::new("Tree", Arc::new(TreeMaterializer))
.with_constructors(vec![ConstructorDescriptor::new(
            "Tree",
            vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
            |arguments| match arguments {
                [ComponentArgument::String(id)] if !id.trim().is_empty() => {
                    Ok(ComponentPayload::new(TreePayload(id.clone())))
                }
                _ => Err("Tree expects non-empty id".into()),
            },
        )])
.with_methods(vec![])
.with_documentation(
            "Native retained tree keyed only by a stable id that must be unique among Trees in the same window; label/structure/disabled data syncs by unique item id while native expansion, selection, focus, and scroll state persist.",
        ))?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wrong_children_are_rejected() {
        assert!(require_tree_item("Tree", Some("TreeItem")).is_ok());
        assert!(require_tree_item("Tree", Some("List")).is_err());
        assert!(
            require_tree_item("Tree", None)
                .unwrap_err()
                .to_string()
                .contains("ordinary element")
        );
        assert!(require_item_style(&gpui::StyleRefinement::default()).is_ok());
        let style = gpui::StyleRefinement::default().p(gpui::px(2.));
        assert!(require_item_style(&style).is_err());
    }
    #[test]
    fn fingerprint_tracks_structure_and_state_without_delimiter_collisions() {
        let a = TreeItem::new("a", "A").child(TreeItem::new("b", "B"));
        let b = TreeItem::new("a", "A").child(TreeItem::new("b", "B").expanded(true));
        assert_ne!(fingerprint(&[a]), fingerprint(&[b]));
        let delimiter_in_id = TreeItem::new("a\0b", "c\u{1f}");
        let delimiter_in_label = TreeItem::new("a", "b\0c\u{1f}");
        assert_ne!(
            fingerprint(&[delimiter_in_id]),
            fingerprint(&[delimiter_in_label])
        );
    }
    #[test]
    fn duplicate_ids_and_expansion_merge_are_defined() {
        let duplicate = vec![TreeItem::new("same", "A"), TreeItem::new("same", "B")];
        assert!(validate_unique_ids(&duplicate).is_err());
        let previous = vec![TreeItem::new("a", "Old").expanded(true)];
        let mut incoming = vec![TreeItem::new("a", "New"), TreeItem::new("b", "Added")];
        preserve_expansion(&mut incoming, &previous);
        assert!(incoming[0].is_expanded());
        assert!(!incoming[1].is_expanded());
        assert_eq!(incoming[0].label.as_ref(), "New");
    }
}
