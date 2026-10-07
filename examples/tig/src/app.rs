use std::sync::Arc;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, IconName, Selectable as _, Sizable as _, StyledExt as _,
    TitleBar,
    button::{Button, ButtonGroup, ButtonVariants as _},
    diff::{Diff, DiffEvent, DiffFile, DiffFileStatus, DiffInlineUnit, DiffMode, DiffState},
    h_flex,
    list::ListItem,
    menu::DropdownMenu as _,
    resizable::{h_resizable, resizable_panel, v_resizable},
    scroll::{ScrollableElement as _, Scrollbar},
    spinner::Spinner,
    status_bar::StatusBar,
    tooltip::Tooltip,
    v_flex,
};
use gpui_kit::{
    AnyElement, App, AppContext as _, ClipboardItem, Context, Entity, FocusHandle, Focusable as _,
    InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, Render, ScrollStrategy,
    SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Task,
    UniformListScrollHandle, Window, div, prelude::FluentBuilder as _, rems, uniform_list,
};

use super::repository::{Commit, CommitDetails, Repository};

mod commands {
    gpui_kit::actions!(
        tig,
        [
            OlderCommit,
            NewerCommit,
            NextFile,
            PreviousFile,
            FocusDiff,
            FocusHistory,
            NextChange,
            PreviousChange,
            Refresh,
            CopyHash,
            ToggleWrap,
            ToggleNumbers,
            ToggleMessage,
            Words,
            Characters,
            WholeLines,
            ThreeContext,
            AllContext,
            ExpandContext,
            CollapseContext,
            Quit
        ]
    );
}
use commands::*;

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("down", OlderCommit, Some("TigHistory")),
        KeyBinding::new("j", OlderCommit, Some("TigHistory")),
        KeyBinding::new("up", NewerCommit, Some("TigHistory")),
        KeyBinding::new("k", NewerCommit, Some("TigHistory")),
        KeyBinding::new("enter", FocusDiff, Some("TigHistory")),
        KeyBinding::new("down", NextFile, Some("TigFiles")),
        KeyBinding::new("up", PreviousFile, Some("TigFiles")),
        KeyBinding::new("alt-down", NextFile, Some("Tig")),
        KeyBinding::new("alt-up", PreviousFile, Some("Tig")),
        KeyBinding::new("escape", FocusHistory, Some("Tig")),
        KeyBinding::new("f7", NextChange, Some("Tig")),
        KeyBinding::new("shift-f7", PreviousChange, Some("Tig")),
        KeyBinding::new("secondary-r", Refresh, Some("Tig")),
        KeyBinding::new("secondary-shift-c", CopyHash, Some("Tig")),
        KeyBinding::new("secondary-q", Quit, Some("Tig")),
    ]);
}

pub(super) struct Tig {
    repository: Repository,
    commits: Arc<Vec<Commit>>,
    files: Arc<Vec<DiffFile>>,
    selected: Option<usize>,
    selected_file: Option<SharedString>,
    diff: Entity<DiffState>,
    message: SharedString,
    history_focus: FocusHandle,
    history_scroll: UniformListScrollHandle,
    files_focus: FocusHandle,
    files_scroll: UniformListScrollHandle,
    wrap: bool,
    line_numbers: bool,
    message_visible: bool,
    loading_history: bool,
    loading_commit: bool,
    error: Option<String>,
    revision: usize,
    _history_task: Task<()>,
    _commit_task: Task<()>,
    _subscriptions: Vec<Subscription>,
}

impl Tig {
    pub(super) fn new(repository: Repository, window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.set_window_title("Git history — GPUI Kit");
        let diff = cx.new(|cx| DiffState::new([], cx));
        let history_focus = cx.focus_handle().tab_stop(true);
        history_focus.focus(window, cx);
        let subscriptions = vec![
            cx.observe(&diff, |_, _, cx| cx.notify()),
            cx.subscribe(&diff, |this, _, event: &DiffEvent, cx| {
                if let DiffEvent::SelectionChanged(Some(range)) = event {
                    this.selected_file = Some(range.path().clone());
                    cx.notify();
                }
            }),
        ];
        let mut this = Self {
            repository,
            commits: Arc::default(),
            files: Arc::default(),
            selected: None,
            selected_file: None,
            diff,
            message: SharedString::default(),
            history_focus,
            history_scroll: UniformListScrollHandle::new(),
            files_focus: cx.focus_handle().tab_stop(true),
            files_scroll: UniformListScrollHandle::new(),
            wrap: true,
            line_numbers: true,
            message_visible: false,
            loading_history: false,
            loading_commit: false,
            error: None,
            revision: 0,
            _history_task: Task::ready(()),
            _commit_task: Task::ready(()),
            _subscriptions: subscriptions,
        };
        this.refresh(cx);
        this
    }

    fn selected_commit(&self) -> Option<&Commit> {
        self.commits.get(self.selected?)
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let previous = self.selected_commit().map(|commit| commit.hash.clone());
        self.revision += 1;
        let revision = self.revision;
        self._commit_task = Task::ready(());
        self.loading_history = true;
        self.commits = Arc::default();
        self.selected = None;
        self.loading_commit = false;
        self.error = None;
        // Clear the old source immediately: copy and file navigation must not
        // act on a patch that belongs to the previous selection.
        self.diff.update(cx, |state, cx| state.set_files([], cx));
        self.selected_file = None;
        self.files = Arc::default();
        self.message = SharedString::default();
        let repository = self.repository.clone();
        self._history_task = cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { repository.history() })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.revision != revision {
                    return;
                }
                this.loading_history = false;
                match result {
                    Ok(commits) => {
                        let selected = previous
                            .and_then(|hash| commits.iter().position(|commit| commit.hash == hash))
                            .unwrap_or(0);
                        this.commits = Arc::new(commits);
                        this.selected = None;
                        if !this.commits.is_empty() {
                            this.select_commit(selected, cx);
                        }
                    }
                    Err(error) => this.error = Some(error),
                }
                cx.notify();
            });
        });
        cx.notify();
    }

    fn select_commit(&mut self, ix: usize, cx: &mut Context<Self>) {
        if self.loading_history || self.commits.get(ix).is_none() || self.selected == Some(ix) {
            return;
        }
        self.selected = Some(ix);
        self.history_scroll.scroll_to_item(ix, ScrollStrategy::Top);
        self.revision += 1;
        let revision = self.revision;
        self.loading_commit = true;
        self.message_visible = false;
        self.error = None;
        self.message = SharedString::default();
        self.selected_file = None;
        self.files = Arc::default();
        self.diff.update(cx, |state, cx| state.set_files([], cx));
        let repository = self.repository.clone();
        let hash = self.commits[ix].hash.clone();
        self._commit_task = cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { repository.details(&hash) })
                .await;
            let _ = this.update(cx, |this, cx| this.install_commit(revision, result, cx));
        });
        cx.notify();
    }

    fn install_commit(
        &mut self,
        revision: usize,
        result: Result<CommitDetails, String>,
        cx: &mut Context<Self>,
    ) {
        if self.revision != revision {
            return;
        }
        self.loading_commit = false;
        match result {
            Ok(details) => {
                self.message = details.message;
                self.files = Arc::new(details.files);
                self.load_file(0, cx);
            }
            Err(error) => self.error = Some(error),
        }
        cx.notify();
    }

    fn move_commit(&mut self, direction: isize, cx: &mut Context<Self>) {
        if self.commits.is_empty() {
            return;
        }
        let ix = self
            .selected
            .unwrap_or(0)
            .saturating_add_signed(direction)
            .min(self.commits.len() - 1);
        self.select_commit(ix, cx);
    }

    fn focus_diff(&self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.loading_history && !self.loading_commit && !self.diff.read(cx).files().is_empty() {
            self.diff.read(cx).focus_handle(cx).focus(window, cx);
        }
    }

    fn load_file(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(file) = self.files.get(ix).cloned() else {
            return;
        };
        let path = file.path().clone();
        if self.selected_file.as_ref() == Some(&path) {
            self.diff.update(cx, |state, cx| {
                state.set_file_collapsed(&path, false, cx);
                state.scroll_to_file(&path, cx);
            });
        } else {
            // One file owns the viewport. Its header remains a reliable scope
            // when the user scrolls, rather than drifting into another file.
            self.diff
                .update(cx, |state, cx| state.set_files([file], cx));
        }
        self.selected_file = Some(path);
        self.files_scroll.scroll_to_item(ix, ScrollStrategy::Top);
        cx.notify();
    }

    fn select_file(&mut self, path: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.files.iter().position(|file| file.path() == &path) {
            self.load_file(ix, cx);
            self.focus_diff(window, cx);
        }
    }

    fn move_file(&mut self, direction: isize, cx: &mut Context<Self>) {
        if self.files.is_empty() {
            return;
        }
        let ix = self
            .files
            .iter()
            .position(|file| Some(file.path()) == self.selected_file.as_ref())
            .unwrap_or(0)
            .saturating_add_signed(direction)
            .min(self.files.len() - 1);
        self.load_file(ix, cx);
    }

    fn copy_hash(&self, cx: &mut Context<Self>) {
        if let Some(commit) = self.selected_commit() {
            cx.write_to_clipboard(ClipboardItem::new_string(commit.hash.to_string()));
        }
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> AnyElement {
        let mode = self.diff.read(cx).mode();
        let context = self.diff.read(cx).context_lines();
        let inline = self.diff.read(cx).inline_unit();
        let (wrap, numbers) = (self.wrap, self.line_numbers);
        let available = !self.diff.read(cx).files().is_empty();
        h_flex()
            .h_10()
            .flex_none()
            .gap_2()
            .px_4()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                ButtonGroup::new("mode").small().outline().children([
                    Button::new("unified")
                        .label("Unified")
                        .selected(mode == DiffMode::Unified)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.diff
                                .update(cx, |state, cx| state.set_mode(DiffMode::Unified, cx));
                        })),
                    Button::new("split")
                        .label("Split")
                        .selected(mode == DiffMode::Split)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.diff
                                .update(cx, |state, cx| state.set_mode(DiffMode::Split, cx));
                        })),
                ]),
            )
            .child(
                Button::new("previous-change")
                    .ghost()
                    .small()
                    .icon(IconName::ArrowUp)
                    .accessibility_label("Previous change")
                    .tooltip("Previous change (Shift+F7)")
                    .disabled(!available)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.diff.update(cx, |state, cx| state.previous_change(cx));
                    })),
            )
            .child(
                Button::new("next-change")
                    .ghost()
                    .small()
                    .icon(IconName::ArrowDown)
                    .accessibility_label("Next change")
                    .tooltip("Next change (F7)")
                    .disabled(!available)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.diff.update(cx, |state, cx| state.next_change(cx));
                    })),
            )
            .child(div().flex_1())
            .child(
                Button::new("display-options")
                    .ghost()
                    .small()
                    .label("Display")
                    .dropdown_menu(move |menu, window, cx| {
                        menu.menu_with_check("Wrap long lines", wrap, Box::new(ToggleWrap))
                            .menu_with_check("Line numbers", numbers, Box::new(ToggleNumbers))
                            .separator()
                            .submenu("Inline changes", window, cx, move |menu, _, _| {
                                menu.menu_with_check(
                                    "Words",
                                    inline == Some(DiffInlineUnit::Word),
                                    Box::new(Words),
                                )
                                .menu_with_check(
                                    "Characters",
                                    inline == Some(DiffInlineUnit::Character),
                                    Box::new(Characters),
                                )
                                .menu_with_check(
                                    "Whole lines",
                                    inline.is_none(),
                                    Box::new(WholeLines),
                                )
                            })
                            .submenu("Context", window, cx, move |menu, _, _| {
                                menu.menu_with_check(
                                    "Three lines",
                                    context == Some(3),
                                    Box::new(ThreeContext),
                                )
                                .menu_with_check(
                                    "All supplied lines",
                                    context.is_none(),
                                    Box::new(AllContext),
                                )
                                .separator()
                                .menu("Expand unchanged lines", Box::new(ExpandContext))
                                .menu("Collapse unchanged lines", Box::new(CollapseContext))
                            })
                    }),
            )
            .into_any_element()
    }

    fn render_history(&self, cx: &mut Context<Self>) -> AnyElement {
        let selected = self.selected;
        let commits = self.commits.clone();
        let owner = cx.entity().downgrade();
        let ring = cx.theme().ring;
        v_flex()
            .size_full()
            .min_h_0()
            .child(
                h_flex()
                    .h_10()
                    .flex_none()
                    .px_4()
                    .gap_2()
                    .child(div().text_sm().font_medium().child("Commits"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{}", commits.len())),
                    ),
            )
            .child(
                div()
                    .id("history-list")
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .border_1()
                    .border_color(cx.theme().transparent)
                    .focus(move |style| style.border_color(ring))
                    .track_focus(&self.history_focus)
                    .key_context("TigHistory")
                    .role(gpui_kit::Role::List)
                    .aria_label("Commit history")
                    .on_mouse_down(
                        gpui_kit::MouseButton::Left,
                        cx.listener(|this, _, window, cx| {
                            this.history_focus.focus(window, cx);
                        }),
                    )
                    .child(
                        uniform_list("commits", commits.len(), move |range, _, cx| {
                            range
                                .map(|ix| {
                                    let commit = &commits[ix];
                                    let owner = owner.clone();
                                    let subject = if commit.subject.is_empty() {
                                        "(No subject)".into()
                                    } else {
                                        commit.subject.clone()
                                    };
                                    let tooltip = subject.clone();
                                    ListItem::new(commit.hash.clone())
                                        .tooltip(move |window, cx| {
                                            Tooltip::new(tooltip.clone()).build(window, cx)
                                        })
                                        .selected(selected == Some(ix))
                                        .h_12()
                                        .px_4()
                                        .w_full()
                                        .accessibility_label(format!(
                                            "{} {}",
                                            commit.short_hash, subject
                                        ))
                                        .child(
                                            v_flex()
                                                .gap_1()
                                                .min_w_0()
                                                .flex_1()
                                                .child(div().text_sm().truncate().child(subject))
                                                .child(
                                                    h_flex()
                                                        .gap_2()
                                                        .text_xs()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .child(
                                                            div()
                                                                .font_family(
                                                                    cx.theme()
                                                                        .mono_font_family
                                                                        .clone(),
                                                                )
                                                                .child(commit.short_hash.clone()),
                                                        )
                                                        .child(
                                                            div()
                                                                .flex_1()
                                                                .min_w_0()
                                                                .truncate()
                                                                .child(commit.author.clone()),
                                                        )
                                                        .child(commit.date.clone()),
                                                ),
                                        )
                                        .on_click(move |_, _, cx| {
                                            let _ = owner
                                                .update(cx, |this, cx| this.select_commit(ix, cx));
                                        })
                                        .into_any_element()
                                })
                                .collect()
                        })
                        .size_full()
                        .track_scroll(&self.history_scroll),
                    )
                    .child(Scrollbar::vertical(&self.history_scroll)),
            )
            .into_any_element()
    }

    fn render_diff(&self, cx: &mut Context<Self>) -> AnyElement {
        let content = if self.loading_history || self.loading_commit {
            v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_3()
                .child(Spinner::new())
                .child(if self.loading_history {
                    "Reading history…"
                } else {
                    "Reading commit…"
                })
                .into_any_element()
        } else if let Some(error) = &self.error {
            v_flex()
                .size_full()
                .p_6()
                .gap_3()
                .child(div().font_medium().child("Couldn't read Git data"))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(error.clone()),
                )
                .child(
                    Button::new("retry")
                        .label("Retry")
                        .w_auto()
                        .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                )
                .into_any_element()
        } else if self.diff.read(cx).files().is_empty() {
            v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_2()
                .child(if self.commits.is_empty() {
                    "This repository has no commits"
                } else {
                    "This commit has no file changes"
                })
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(if self.commits.is_empty() {
                            "Create a commit, then refresh the history."
                        } else {
                            "Select another commit to inspect its changes."
                        }),
                )
                .into_any_element()
        } else {
            Diff::new(&self.diff)
                .line_number(self.line_numbers)
                .soft_wrap(self.wrap)
                .change_background(false)
                .border_0()
                .into_any_element()
        };
        v_flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .when_some(self.selected_commit(), |this, commit| {
                this.child(
                    v_flex()
                        .px_4()
                        .py_3()
                        .gap_1()
                        .flex_none()
                        .child(
                            div()
                                .text_sm()
                                .font_medium()
                                .truncate()
                                .child(commit.subject.clone()),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(
                                    div()
                                        .font_family(cx.theme().mono_font_family.clone())
                                        .child(commit.short_hash.clone()),
                                )
                                .child(commit.author.clone())
                                .child("·")
                                .child(commit.date.clone())
                                .child(div().flex_1())
                                .when(!self.message.is_empty(), |this| {
                                    this.child(
                                        Button::new("commit-message-toggle")
                                            .ghost()
                                            .xsmall()
                                            .label(if self.message_visible {
                                                "Hide message"
                                            } else {
                                                "Show message"
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.message_visible = !this.message_visible;
                                                cx.notify();
                                            })),
                                    )
                                })
                                .child(
                                    Button::new("copy-hash")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Copy)
                                        .accessibility_label("Copy commit hash")
                                        .tooltip("Copy commit hash")
                                        .on_click(cx.listener(|this, _, _, cx| this.copy_hash(cx))),
                                ),
                        ),
                )
            })
            .when(
                self.message_visible && !self.loading_commit && !self.loading_history,
                |this| {
                    this.child(
                        div()
                            .id("commit-message")
                            .max_h(rems(10.))
                            .overflow_y_scrollbar()
                            .child(div().px_4().py_3().text_sm().child(self.message.clone())),
                    )
                },
            )
            .child(self.render_toolbar(cx))
            .child(div().flex_1().min_h_0().min_w_0().child(content))
            .into_any_element()
    }

    fn render_files(&self, cx: &mut Context<Self>) -> AnyElement {
        let files = self.files.clone();
        let selected_file = self.selected_file.clone();
        let owner = cx.entity().downgrade();
        let ring = cx.theme().ring;
        v_flex()
            .size_full()
            .min_h_0()
            .child(
                h_flex()
                    .h_10()
                    .flex_none()
                    .gap_2()
                    .px_4()
                    .child(div().font_medium().text_sm().child("Changed files"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{}", files.len())),
                    ),
            )
            .child(
                div()
                    .id("file-list")
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .border_1()
                    .border_color(cx.theme().transparent)
                    .focus(move |style| style.border_color(ring))
                    .track_focus(&self.files_focus)
                    .key_context("TigFiles")
                    .role(gpui_kit::Role::List)
                    .aria_label("Changed files")
                    .on_mouse_down(
                        gpui_kit::MouseButton::Left,
                        cx.listener(|this, _, window, cx| {
                            this.files_focus.focus(window, cx);
                        }),
                    )
                    .child(
                        uniform_list("files", files.len(), move |range, _, cx| {
                            range
                                .map(|ix| {
                                    let file = &files[ix];
                                    let path = file.path().clone();
                                    let owner = owner.clone();
                                    let (directory, filename) =
                                        path.rsplit_once('/').unwrap_or(("", path.as_str()));
                                    let status = match file.status() {
                                        DiffFileStatus::Added => "A",
                                        DiffFileStatus::Deleted => "D",
                                        DiffFileStatus::Renamed => "R",
                                        DiffFileStatus::Copied => "C",
                                        _ => {
                                            if file.is_binary() {
                                                "B"
                                            } else {
                                                "M"
                                            }
                                        }
                                    };
                                    let tooltip = path.clone();
                                    ListItem::new(path.clone())
                                        .tooltip(move |window, cx| {
                                            Tooltip::new(tooltip.clone()).build(window, cx)
                                        })
                                        .h_12()
                                        .px_4()
                                        .w_full()
                                        .min_w_0()
                                        .selected(selected_file.as_ref() == Some(&path))
                                        .accessibility_label(format!(
                                            "{} · {:?} · +{} −{}",
                                            path,
                                            file.status(),
                                            file.additions(),
                                            file.deletions()
                                        ))
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .min_w_0()
                                                .gap_3()
                                                .child(
                                                    v_flex()
                                                        .min_w_0()
                                                        .flex_1()
                                                        .gap_1()
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .truncate()
                                                                .child(filename.to_owned()),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_xs()
                                                                .truncate()
                                                                .text_color(
                                                                    cx.theme().muted_foreground,
                                                                )
                                                                .child(directory.to_owned()),
                                                        ),
                                                )
                                                .child(
                                                    v_flex()
                                                        .flex_none()
                                                        .items_end()
                                                        .gap_1()
                                                        .text_xs()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .child(status)
                                                        .child(format!(
                                                            "+{} −{}",
                                                            file.additions(),
                                                            file.deletions()
                                                        )),
                                                ),
                                        )
                                        .on_click(move |_, window, cx| {
                                            let _ = owner.update(cx, |this, cx| {
                                                this.select_file(path.clone(), window, cx)
                                            });
                                        })
                                        .into_any_element()
                                })
                                .collect()
                        })
                        .size_full()
                        .track_scroll(&self.files_scroll),
                    )
                    .child(Scrollbar::vertical(&self.files_scroll)),
            )
            .into_any_element()
    }
}

impl Render for Tig {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rem = window.rem_size();
        let selection = self
            .diff
            .read(cx)
            .selected_lines()
            .map(|range| {
                format!(
                    "{} · {:?} {}–{}",
                    range.path(),
                    range.side(),
                    range.start(),
                    range.end()
                )
            })
            .unwrap_or_else(|| "Readonly · patch context only".into());
        v_flex()
            .id("tig")
            .key_context("Tig")
            .size_full()
            .min_w_0()
            .min_h_0()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .on_action(cx.listener(|this, _: &OlderCommit, _, cx| this.move_commit(1, cx)))
            .on_action(cx.listener(|this, _: &NewerCommit, _, cx| this.move_commit(-1, cx)))
            .on_action(cx.listener(|this, _: &NextFile, _, cx| this.move_file(1, cx)))
            .on_action(cx.listener(|this, _: &PreviousFile, _, cx| this.move_file(-1, cx)))
            .on_action(cx.listener(|this, _: &FocusDiff, window, cx| this.focus_diff(window, cx)))
            .on_action(cx.listener(|this, _: &FocusHistory, window, cx| {
                this.history_focus.focus(window, cx)
            }))
            .on_action(cx.listener(|this, _: &Refresh, _, cx| this.refresh(cx)))
            .on_action(cx.listener(|this, _: &CopyHash, _, cx| this.copy_hash(cx)))
            .on_action(cx.listener(|this, _: &NextChange, _, cx| {
                this.diff.update(cx, |state, cx| state.next_change(cx))
            }))
            .on_action(cx.listener(|this, _: &PreviousChange, _, cx| {
                this.diff.update(cx, |state, cx| state.previous_change(cx))
            }))
            .on_action(cx.listener(|this, _: &ToggleWrap, _, cx| {
                this.wrap = !this.wrap;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleNumbers, _, cx| {
                this.line_numbers = !this.line_numbers;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Words, _, cx| {
                this.diff.update(cx, |state, cx| {
                    state.set_inline_unit(Some(DiffInlineUnit::Word), cx)
                })
            }))
            .on_action(cx.listener(|this, _: &Characters, _, cx| {
                this.diff.update(cx, |state, cx| {
                    state.set_inline_unit(Some(DiffInlineUnit::Character), cx)
                })
            }))
            .on_action(cx.listener(|this, _: &WholeLines, _, cx| {
                this.diff
                    .update(cx, |state, cx| state.set_inline_unit(None, cx))
            }))
            .on_action(cx.listener(|this, _: &ThreeContext, _, cx| {
                this.diff
                    .update(cx, |state, cx| state.set_context_lines(Some(3), cx))
            }))
            .on_action(cx.listener(|this, _: &AllContext, _, cx| {
                this.diff
                    .update(cx, |state, cx| state.set_context_lines(None, cx))
            }))
            .on_action(cx.listener(|this, _: &ExpandContext, _, cx| {
                this.diff.update(cx, |state, cx| state.expand_unchanged(cx))
            }))
            .on_action(cx.listener(|this, _: &CollapseContext, _, cx| {
                this.diff
                    .update(cx, |state, cx| state.collapse_unchanged(cx))
            }))
            .on_action(cx.listener(|this, _: &ToggleMessage, _, cx| {
                this.message_visible = !this.message_visible;
                cx.notify();
            }))
            .on_action(|_: &Quit, _, cx| cx.quit())
            .child(
                TitleBar::new().child(
                    h_flex()
                        .size_full()
                        .px_1()
                        .pr_3()
                        .gap_2()
                        .child(div().text_sm().font_medium().child(self.repository.name()))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("HEAD history"),
                        )
                        .child(div().flex_1())
                        .child(
                            Button::new("refresh")
                                .ghost()
                                .small()
                                .label("Refresh")
                                .disabled(self.loading_history)
                                .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                        ),
                ),
            )
            .child(
                div().flex_1().min_h_0().child(
                    h_resizable("tig-panes")
                        .child(
                            resizable_panel()
                                .size(rem * 22.)
                                .size_range(rem * 18. ..rem * 30.)
                                .child(
                                    v_flex().size_full().min_h_0().bg(cx.theme().sidebar).child(
                                        v_resizable("tig-navigation")
                                            .child(
                                                resizable_panel()
                                                    .size(rem * 25.)
                                                    .size_range(rem * 12. ..rem * 100.)
                                                    .child(self.render_history(cx)),
                                            )
                                            .child(
                                                resizable_panel()
                                                    .size_range(rem * 10. ..rem * 100.)
                                                    .child(self.render_files(cx)),
                                            ),
                                    ),
                                ),
                        )
                        .child(
                            resizable_panel()
                                .size_range(rem * 38. ..rem * 200.)
                                .child(self.render_diff(cx)),
                        ),
                ),
            )
            .child(
                StatusBar::new()
                    .left(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .min_w_0()
                            .truncate()
                            .child(selection),
                    )
                    .right(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .flex_none()
                            .child("Alt+↑/↓ Files · F7 Changes"),
                    ),
            )
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
