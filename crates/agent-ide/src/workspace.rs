//! The workspace skeleton: a title bar over three columns — the Projects
//! sidebar flanking a central dock area whose default layout is one tab
//! group of terminal panels.
//!
//! Layout persistence is deliberately absent (P0 has no session restore);
//! the area always starts from the default layout below. What *is*
//! persisted is the recent-project list, in a single JSON file.

use std::path::{Path, PathBuf};

use gpui_kit::component::{
    ActiveTheme as _, IconName, Side, Sizable as _, TitleBar,
    button::{Button, ButtonVariants as _},
    dock::{DockArea, DockLayout, DockPlacement, DockSkin, panel_handle},
    h_flex,
    sidebar::{
        Sidebar, SidebarCollapsible, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem,
    },
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::terminal_panel::TerminalPanel;

const DOCK_AREA_ID: &str = "agent-ide-dock";
const LEFT_SIDEBAR_WIDTH: Pixels = px(250.);
const RIGHT_SIDEBAR_WIDTH: Pixels = px(260.);
const MAX_RECENT_PROJECTS: usize = 10;

/// The on-disk shape of `state.json` (`dirs::data_dir()/agent-ide/`).
#[derive(serde::Serialize, serde::Deserialize)]
struct PersistedState {
    recent_projects: Vec<String>,
}

pub struct Workspace {
    dock_area: Entity<DockArea>,
    left_collapsed: bool,
    right_collapsed: bool,
    /// Recently opened directories, newest first, deduplicated, capped at
    /// [`MAX_RECENT_PROJECTS`].
    recent_projects: Vec<PathBuf>,
    /// The project new terminals are opened in; `None` means the login
    /// shell's own default directory.
    current_project: Option<PathBuf>,
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (dock_area, _skin) = DockSkin::dock_area(DOCK_AREA_ID, None, window, cx);

        // Default center layout: a single Tabs node holding one terminal.
        // Panels added later join this group as new tabs.
        let terminal = cx.new(|cx| TerminalPanel::new(cx));
        let center = DockLayout::tabs().panel_view(panel_handle(terminal), cx);
        dock_area.update(cx, |area, cx| area.set_center(center, window, cx));

        Self {
            dock_area,
            left_collapsed: false,
            right_collapsed: false,
            recent_projects: load_recent_projects(),
            current_project: None,
        }
    }

    /// Open a new terminal tab in the center; the PTY starts in `cwd` when
    /// given, otherwise in the login shell's own default directory.
    fn new_terminal_tab(
        &mut self,
        cwd: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let panel = cx.new(|cx| TerminalPanel::new_in(cx, cwd));
        self.dock_area.update(cx, |area, cx| {
            area.add_panel_view(panel_handle(panel), DockPlacement::Center, None, window, cx);
        });
    }

    /// Ask for a directory (native folder picker) and open it as the
    /// current project.
    fn open_project_dialog(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Open Project")
            .pick_folder()
        else {
            return; // the user cancelled the dialog
        };
        self.open_project(path, window, cx);
    }

    /// Make `path` the current project: bump it to the top of the recents,
    /// persist the list, and open a terminal rooted there.
    fn open_project(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.current_project = Some(path.clone());
        self.push_recent_project(path.clone());
        self.save_recent_projects();
        self.new_terminal_tab(Some(path), window, cx);
    }

    fn push_recent_project(&mut self, path: PathBuf) {
        self.recent_projects.retain(|p| *p != path);
        self.recent_projects.insert(0, path);
        self.recent_projects.truncate(MAX_RECENT_PROJECTS);
    }

    fn render_title_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new()
            // Transparent like the original example: the blurred window
            // background shows through, keeping the glass look.
            .bg(hsla(0., 0., 0., 0.))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("AgentIDE"),
            )
            // The title bar spreads its children apart, so this one group is
            // what lands on the trailing edge.
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("toggle-left-sidebar")
                            .ghost()
                            .xsmall()
                            .icon(IconName::PanelLeft)
                            .tooltip("Toggle Projects sidebar")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.left_collapsed = !this.left_collapsed;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("new-terminal")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Plus)
                            .tooltip("New terminal tab")
                            .on_click(cx.listener(|this, _, window, cx| {
                                // New terminals open in the current project
                                // when one is selected.
                                let cwd = this.current_project.clone();
                                this.new_terminal_tab(cwd, window, cx);
                            })),
                    )
                    .child(
                        Button::new("toggle-right-sidebar")
                            .ghost()
                            .xsmall()
                            .icon(IconName::PanelRight)
                            .tooltip("Toggle Session sidebar")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.right_collapsed = !this.right_collapsed;
                                cx.notify();
                            })),
                    ),
            )
    }

    /// Left rail: an open-directory action over the recent-project list.
    fn render_left_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace = cx.entity();
        let current = self.current_project.clone();
        let items: Vec<SidebarMenuItem> = if self.recent_projects.is_empty() {
            vec![SidebarMenuItem::new("No recent projects").disable(true)]
        } else {
            self.recent_projects
                .iter()
                .map(|path| {
                    let path = path.clone();
                    let workspace = workspace.clone();
                    SidebarMenuItem::new(project_name(&path))
                        .icon(IconName::Folder)
                        .active(current.as_deref() == Some(path.as_path()))
                        .on_click(move |_, window, cx| {
                            let path = path.clone();
                            workspace.update(cx, |ws, cx| ws.open_project(path, window, cx));
                        })
                })
                .collect()
        };

        Sidebar::new("left-sidebar")
            .collapsible(SidebarCollapsible::Icon)
            .collapsed(self.left_collapsed)
            .w(LEFT_SIDEBAR_WIDTH)
            // Translucent over the blurred window so the glass look from the
            // original example survives the sidebar's opaque token default.
            // Sidebar's token background is fully opaque in the new palette;
            // clear it so the sides stay as glassy as the original example.
            .bg(hsla(0., 0., 0., 0.))
            .header(
                SidebarHeader::new().child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .when(!self.left_collapsed, |row| {
                            row.child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .child("Projects"),
                            )
                        })
                        .child(
                            Button::new("open-project")
                                .ghost()
                                .xsmall()
                                .icon(IconName::FolderOpen)
                                .tooltip("Open directory…")
                                .ml_auto()
                                .on_click(cx.listener(Self::open_project_dialog)),
                        ),
                ),
            )
            .child(SidebarGroup::new("Projects").child(SidebarMenu::new().children(items)))
    }

    /// Right rail: static session placeholders until the session panel lands.
    fn render_right_sidebar(&self, cx: &Context<Self>) -> impl IntoElement {
        Sidebar::new("right-sidebar")
            .side(Side::Right)
            .collapsible(SidebarCollapsible::Icon)
            .collapsed(self.right_collapsed)
            .w(RIGHT_SIDEBAR_WIDTH)
            .bg(hsla(0., 0., 0., 0.))
            .child(SidebarGroup::new("Session").children([
                SidebarMenuItem::new("Model: claude-fable-5-1"),
                SidebarMenuItem::new("Task: Fix flaky table test"),
                SidebarMenuItem::new("Uptime: 42m"),
            ]))
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .text_color(cx.theme().foreground)
            .bg(cx.theme().background.opacity(0.72))
            .child(self.render_title_bar(cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_stretch()
                    .child(self.render_left_sidebar(cx))
                    .child(div().flex_1().min_w_0().child(self.dock_area.clone()))
                    .child(self.render_right_sidebar(cx)),
            )
    }
}

/// `dirs::data_dir()/agent-ide/state.json` (`~/Library/Application Support/…`
/// on macOS).
fn state_path() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("agent-ide").join("state.json"))
}

/// Read the recent-project list from disk. A missing, unreadable, or
/// malformed file just means an empty list; startup never panics nor writes.
fn load_recent_projects() -> Vec<PathBuf> {
    let mut projects = Vec::new();
    if let Some(path) = state_path() {
        if let Ok(text) = std::fs::read_to_string(&path) {
            match serde_json::from_str::<PersistedState>(&text) {
                Ok(state) => {
                    for entry in state.recent_projects {
                        let entry = PathBuf::from(entry);
                        if !projects.contains(&entry) {
                            projects.push(entry);
                        }
                    }
                    projects.truncate(MAX_RECENT_PROJECTS);
                }
                Err(err) => {
                    eprintln!("agent-ide: ignoring malformed {}: {err}", path.display());
                }
            }
        }
    }
    projects
}

impl Workspace {
    /// Persist the recent-project list. Write failures are logged, never
    /// fatal — the app works fine without history.
    fn save_recent_projects(&self) {
        let Some(path) = state_path() else {
            return;
        };
        let state = PersistedState {
            recent_projects: self
                .recent_projects
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
        };
        let write = || -> std::io::Result<()> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, serde_json::to_string_pretty(&state)?)?;
            Ok(())
        };
        if let Err(err) = write() {
            eprintln!("agent-ide: failed to save {}: {err}", path.display());
        }
    }
}

/// The folder's display name (its last component), falling back to the full
/// path for roots like `/`.
fn project_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}
