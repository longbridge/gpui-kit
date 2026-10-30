use std::{collections::BTreeMap, rc::Rc};

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, rems,
};

use crate::{
    ActiveTheme as _, Icon, IconName, Selectable as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    v_flex,
};

/// How the agent changed a file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FileChange {
    /// The file is new.
    Added,
    /// The file's content changed.
    #[default]
    Modified,
    /// The file was removed.
    Deleted,
}

/// A file the agent changed, addressed by its `/`-separated path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileTreeEntry {
    path: SharedString,
    change: FileChange,
    additions: Option<usize>,
    deletions: Option<usize>,
}

impl FileTreeEntry {
    /// Create a modified file at `path`, such as `src/ai/mod.rs`.
    pub fn new(path: impl Into<SharedString>) -> Self {
        Self {
            path: path.into(),
            change: FileChange::Modified,
            additions: None,
            deletions: None,
        }
    }

    /// Set how the file changed.
    pub fn change(mut self, change: FileChange) -> Self {
        self.change = change;
        self
    }

    /// Set the number of added and removed lines.
    pub fn line_counts(mut self, additions: usize, deletions: usize) -> Self {
        self.additions = Some(additions);
        self.deletions = Some(deletions);
        self
    }

    /// The file's path.
    pub fn path(&self) -> &SharedString {
        &self.path
    }
}

#[derive(Default)]
struct Folder {
    folders: BTreeMap<String, Folder>,
    files: Vec<FileTreeEntry>,
}

impl Folder {
    fn build(entries: Vec<FileTreeEntry>) -> Self {
        let mut root = Folder::default();
        for entry in entries {
            let mut folder = &mut root;
            let mut parts: Vec<&str> = entry.path.split('/').filter(|p| !p.is_empty()).collect();
            parts.pop();
            for part in parts {
                folder = folder.folders.entry(part.to_string()).or_default();
            }
            folder.files.push(entry);
        }
        root
    }
}

type PathHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// The files the agent changed, shown as a folder tree.
///
/// Folders are expanded unless their path is listed with [`Self::collapsed`];
/// both folder expansion and the selected file are controlled by the
/// application.
#[derive(IntoElement)]
pub struct FileTree {
    id: ElementId,
    style: StyleRefinement,
    entries: Vec<FileTreeEntry>,
    collapsed: Vec<SharedString>,
    selected: Option<SharedString>,
    on_toggle_folder: Option<PathHandler>,
    on_select: Option<PathHandler>,
}

impl FileTree {
    /// Create an empty tree.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            entries: Vec::new(),
            collapsed: Vec::new(),
            selected: None,
            on_toggle_folder: None,
            on_select: None,
        }
    }

    /// Add a changed file.
    pub fn entry(mut self, entry: FileTreeEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Add several changed files.
    pub fn entries(mut self, entries: impl IntoIterator<Item = FileTreeEntry>) -> Self {
        self.entries.extend(entries);
        self
    }

    /// Set the folder paths, such as `src/ai`, that are collapsed.
    pub fn collapsed(mut self, paths: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.collapsed = paths.into_iter().map(Into::into).collect();
        self
    }

    /// Set the selected file path.
    pub fn selected(mut self, path: Option<SharedString>) -> Self {
        self.selected = path;
        self
    }

    /// Called with a folder's path when the user expands or collapses it.
    pub fn on_toggle_folder(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle_folder = Some(Rc::new(handler));
        self
    }

    /// Called with a file's path when the user selects it.
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    fn row(id: impl Into<ElementId>, depth: usize) -> Button {
        Button::new(id).ghost().small().w_full().content_style(
            StyleRefinement::default()
                .justify_start()
                .pl(rems(depth as f32 * 0.75)),
            crate::Size::Small,
        )
    }

    fn render_folder(
        &self,
        folder: Folder,
        prefix: &str,
        depth: usize,
        rows: &mut Vec<AnyElement>,
        cx: &App,
    ) {
        let theme = cx.theme();
        for (name, child) in folder.folders {
            let path: SharedString = format!("{prefix}{name}").into();
            let open = !self.collapsed.contains(&path);
            let on_toggle = self.on_toggle_folder.clone();
            rows.push(
                Self::row((self.id.clone(), path.clone()), depth)
                    .child(
                        Icon::new(if open {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .xsmall()
                        .text_color(theme.muted_foreground),
                    )
                    .child(
                        Icon::new(if open {
                            IconName::FolderOpen
                        } else {
                            IconName::FolderClosed
                        })
                        .xsmall()
                        .text_color(theme.muted_foreground),
                    )
                    .child(name.clone())
                    .when_some(on_toggle, |this, on_toggle| {
                        let path = path.clone();
                        this.on_click(move |_, window, cx| on_toggle(&path, window, cx))
                    })
                    .into_any_element(),
            );
            if open {
                self.render_folder(child, &format!("{path}/"), depth + 1, rows, cx);
            }
        }

        for file in folder.files {
            let name = file
                .path
                .rsplit('/')
                .next()
                .unwrap_or(file.path.as_ref())
                .to_string();
            let (icon, color) = match file.change {
                FileChange::Added => (IconName::FilePlus, theme.success),
                FileChange::Modified => (IconName::FilePen, theme.muted_foreground),
                FileChange::Deleted => (IconName::FileMinus, theme.danger),
            };
            let on_select = self.on_select.clone();
            let path = file.path.clone();
            rows.push(
                Self::row((self.id.clone(), file.path.clone()), depth)
                    .selected(self.selected.as_ref() == Some(&file.path))
                    // The chevron lane keeps file icons aligned with folder icons.
                    .child(div().size_3().flex_none())
                    .child(Icon::new(icon).xsmall().text_color(color))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .when(file.change == FileChange::Deleted, |this| {
                                this.line_through()
                            })
                            .child(name),
                    )
                    .when_some(file.additions, |this, additions| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(theme.success)
                                .child(format!("+{additions}")),
                        )
                    })
                    .when_some(file.deletions, |this, deletions| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(theme.danger)
                                .child(format!("−{deletions}")),
                        )
                    })
                    .when_some(on_select, |this, on_select| {
                        this.on_click(move |_, window, cx| on_select(&path, window, cx))
                    })
                    .into_any_element(),
            );
        }
    }
}

impl Styled for FileTree {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for FileTree {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let root = Folder::build(std::mem::take(&mut self.entries));
        let mut rows = Vec::new();
        self.render_folder(root, "", 0, &mut rows, cx);

        v_flex()
            .w_full()
            .min_w_0()
            .gap_px()
            .refine_style(&self.style)
            .children(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_tree_builder() {
        let tree = FileTree::new("files")
            .entry(FileTreeEntry::new("src/ai/mod.rs").change(FileChange::Added))
            .entries([
                FileTreeEntry::new("src/lib.rs").line_counts(3, 1),
                FileTreeEntry::new("README.md").change(FileChange::Deleted),
            ])
            .collapsed(["src/ai"])
            .selected(Some("src/lib.rs".into()))
            .on_toggle_folder(|_, _, _| {})
            .on_select(|_, _, _| {});

        assert_eq!(tree.entries.len(), 3);
        assert_eq!(tree.collapsed, vec![SharedString::from("src/ai")]);
        assert_eq!(tree.selected.as_deref(), Some("src/lib.rs"));
        assert!(tree.on_toggle_folder.is_some());
        assert!(tree.on_select.is_some());
    }

    #[test]
    fn test_folder_build_groups_by_directory() {
        let root = Folder::build(vec![
            FileTreeEntry::new("src/ai/mod.rs"),
            FileTreeEntry::new("src/lib.rs"),
            FileTreeEntry::new("README.md"),
        ]);
        assert_eq!(root.files.len(), 1);
        let src = &root.folders["src"];
        assert_eq!(src.files[0].path(), "src/lib.rs");
        assert_eq!(src.folders["ai"].files[0].path(), "src/ai/mod.rs");
    }
}
