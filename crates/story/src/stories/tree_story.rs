use std::path::PathBuf;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use gpui_kit::component::{
    ActiveTheme as _, IconName,
    button::Button,
    dock::PanelControl,
    h_flex,
    label::Label,
    list::ListItem,
    tree::{TreeItem, TreeState, tree},
    v_flex,
};
use rand::seq::SliceRandom as _;

#[cfg(not(target_family = "wasm"))]
use autocorrect::ignorer::Ignorer;
#[cfg(not(target_family = "wasm"))]
use std::path::Path;

use crate::{Story, section};

actions!(story, [Rename, OpenFile, Delete]);

const CONTEXT: &str = "TreeStory";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("enter", Rename, Some(CONTEXT))]);
}

pub struct TreeStory {
    tree_state: Entity<TreeState>,
    items: Vec<TreeItem>,
}

#[cfg(target_family = "wasm")]
fn example_file_items() -> Vec<TreeItem> {
    vec![
        TreeItem::new("gpui-component", "gpui-component")
            .expanded(true)
            .children([
                TreeItem::new("gpui-component/crates", "crates")
                    .expanded(true)
                    .children([
                        TreeItem::new("gpui-component/crates/component", "ui")
                            .expanded(true)
                            .children([
                                TreeItem::new("gpui-component/crates/component/src", "src")
                                    .children([
                                        TreeItem::new(
                                            "gpui-component/crates/component/src/tree.rs",
                                            "tree.rs",
                                        ),
                                        TreeItem::new(
                                            "gpui-component/crates/component/src/list/mod.rs",
                                            "mod.rs",
                                        ),
                                    ]),
                                TreeItem::new(
                                    "gpui-component/crates/component/Cargo.toml",
                                    "Cargo.toml",
                                ),
                            ]),
                        TreeItem::new("gpui-component/crates/story", "story").children([
                            TreeItem::new(
                                "gpui-component/crates/story/src/stories/tree_story.rs",
                                "tree_story.rs",
                            ),
                            TreeItem::new(
                                "gpui-component/crates/story/src/gallery.rs",
                                "gallery.rs",
                            ),
                        ]),
                    ]),
                TreeItem::new("gpui-component/README.md", "README.md"),
            ]),
    ]
}

#[cfg(not(target_family = "wasm"))]
fn build_file_items(ignorer: &Ignorer, root: &Path, path: &Path) -> Vec<TreeItem> {
    let mut items = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let relative_path = path.strip_prefix(root).unwrap_or(&path);
            if ignorer.is_ignored(&relative_path.to_string_lossy())
                || relative_path.ends_with(".git")
            {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();
            let id = path.to_string_lossy().to_string();
            if path.is_dir() {
                let children = build_file_items(ignorer, root, &path);
                items.push(TreeItem::new(id, file_name).children(children));
            } else {
                items.push(TreeItem::new(id, file_name));
            }
        }
    }
    items.sort_by(|a, b| {
        b.is_folder()
            .cmp(&a.is_folder())
            .then(a.label.cmp(&b.label))
    });
    items
}

#[cfg(target_family = "wasm")]
fn load_tree_items(_: PathBuf) -> Vec<TreeItem> {
    example_file_items()
}

#[cfg(not(target_family = "wasm"))]
fn load_tree_items(path: PathBuf) -> Vec<TreeItem> {
    let ignorer = Ignorer::new(&path.to_string_lossy());
    build_file_items(&ignorer, &path, &path)
}

impl TreeStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn load_files(state: Entity<TreeState>, path: PathBuf, cx: &mut Context<Self>) {
        cx.spawn(async move |weak_self, cx| {
            let items = load_tree_items(path);
            _ = state.update(cx, |state, cx| {
                state.set_items(items.clone(), cx);
            });

            _ = weak_self.update(cx, |this, cx| {
                this.items = items;
                cx.notify();
            })
        })
        .detach();
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let tree_state = cx.new(|cx| TreeState::new(cx));

        Self::load_files(tree_state.clone(), PathBuf::from("./"), cx);

        Self {
            tree_state,
            items: Vec::new(),
        }
    }

    fn on_action_rename(&mut self, _: &Rename, _: &mut Window, cx: &mut gpui_kit::Context<Self>) {
        if let Some(entry) = self.tree_state.read(cx).selected_entry() {
            let item = entry.item();
            println!("Renaming item: {} ({})", item.label, item.id);
        }
    }

    fn on_action_open(&mut self, _: &OpenFile, _: &mut Window, cx: &mut gpui_kit::Context<Self>) {
        if let Some(entry) = self.tree_state.read(cx).selected_entry() {
            let item = entry.item();
            println!("Opening item: {} ({})", item.label, item.id);
        }
    }

    fn on_action_delete(&mut self, _: &Delete, _: &mut Window, cx: &mut gpui_kit::Context<Self>) {
        if let Some(entry) = self.tree_state.read(cx).selected_entry() {
            let item = entry.item();
            println!("Deleting item: {} ({})", item.label, item.id);
        }
    }
}

impl Story for TreeStory {
    fn title() -> &'static str {
        "Tree"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Render for TreeStory {
    fn render(
        &mut self,
        _: &mut gpui_kit::Window,
        cx: &mut gpui_kit::Context<Self>,
    ) -> impl gpui_kit::IntoElement {
        let view = cx.entity();
        v_flex()
            .w_full()
            .gap_3()
            .id("tree-story")
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_action_rename))
            .on_action(cx.listener(Self::on_action_open))
            .on_action(cx.listener(Self::on_action_delete))
            .child(
                h_flex().gap_3().child(
                    Button::new("select-item")
                        .outline()
                        .label("Select Item")
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(random_item) = this.items.choose(&mut rand::thread_rng()) {
                                this.tree_state.update(cx, |state, cx| {
                                    state.set_selected_item(Some(random_item), cx);
                                });
                            }
                        })),
                ),
            )
            .child(
                section("File tree")
                    .sub_title("Press `enter` to rename. Right-click for context menu.")
                    .w(px(480.))
                    .child(
                        v_flex()
                            .w_full()
                            .gap_4()
                            .child(
                                tree(
                                    &self.tree_state,
                                    move |ix, entry, _selected, _window, cx| {
                                        view.update(cx, |_, cx| {
                                            let item = entry.item();
                                            let icon = if !entry.is_folder() {
                                                IconName::File
                                            } else if entry.is_expanded() {
                                                IconName::FolderOpen
                                            } else {
                                                IconName::Folder
                                            };

                                            ListItem::new(ix)
                                                .w_full()
                                                .rounded(cx.theme().radius)
                                                .px_3()
                                                .pl(px(16.) * entry.depth() + px(12.))
                                                .child(
                                                    h_flex()
                                                        .gap_2()
                                                        .child(icon)
                                                        .child(item.label.clone()),
                                                )
                                                .on_click(cx.listener({
                                                    let item = item.clone();
                                                    move |_, _, _window, _| {
                                                        println!(
                                                            "Clicked on item: {} ({})",
                                                            item.label, item.id
                                                        );
                                                    }
                                                }))
                                        })
                                    },
                                )
                                .context_menu(|_ix, entry, menu, _window, _cx| {
                                    let is_folder = entry.is_folder();
                                    menu.when(!is_folder, |m| m.menu("Open", Box::new(OpenFile)))
                                        .menu("Rename", Box::new(Rename))
                                        .separator()
                                        .menu("Delete", Box::new(Delete))
                                })
                                .p_1()
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded(cx.theme().radius)
                                .h(px(540.)),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .gap_3()
                                    .children(
                                        self.tree_state
                                            .read(cx)
                                            .selected_index()
                                            .map(|ix| format!("Selected Index: {}", ix)),
                                    )
                                    .children(self.tree_state.read(cx).selected_item().map(
                                        |item| Label::new("Selected:").secondary(item.id.clone()),
                                    )),
                            ),
                    ),
            )
    }
}
