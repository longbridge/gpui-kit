use std::{path::PathBuf, sync::Arc};

use gpui_kit::{
    AppContext as _, Focusable as _, Task, TestAppContext, WindowOptions,
    component::diff::DiffFile, test::TestWindowExt as _,
};

use super::{Commit, CommitDetails, Repository, Tig};

fn details(path: &str, content: &str) -> CommitDetails {
    CommitDetails {
        message: "Commit body".into(),
        files: DiffFile::parse(&format!(
            "--- /dev/null\n+++ b/{path}\n@@ -0,0 +1 @@\n+{content}\n"
        ))
        .unwrap(),
    }
}

#[gpui_kit::test]
fn commit_navigation_discards_old_results_and_preserves_keyboard_focus(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(super::init);
    let (handle, view) = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| {
                let mut view = Tig::new(Repository::new(PathBuf::from(".")), window, cx);
                view.seed_history(cx);
                view
            })
        })
        .unwrap()
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.press("down", cx);
        view.update(cx, |view, cx| {
            view.verify_loading_and_install(cx);
        });
        window.render_frame(cx);
        window.press("]", cx);
        assert_eq!(view.read(cx).selected_file.as_deref(), Some("other.txt"));
        assert_eq!(view.read(cx).diff.read(cx).files().len(), 1);
        assert_eq!(
            view.read(cx).diff.read(cx).files()[0].path().as_str(),
            "other.txt"
        );
        window.press("enter", cx);
        assert!(
            view.read(cx)
                .diff
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
        window.press("escape", cx);
        assert!(view.read(cx).history_focus.is_focused(window));
        window.render_frame(cx);
        window.click(gpui_kit::SharedString::new("new.txt"), cx);
        assert!(view.read(cx).files_focus.is_focused(window));
        window.press("down", cx);
        assert_eq!(view.read(cx).selected_file.as_deref(), Some("other.txt"));
        window.press("enter", cx);
        assert!(
            view.read(cx)
                .diff
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
        window.press(
            if cfg!(target_os = "macos") {
                "cmd-b"
            } else {
                "ctrl-b"
            },
            cx,
        );
        assert!(!view.read(cx).sidebar_visible);
        window.render_frame(cx);
        window.press("escape", cx);
        window.render_frame(cx);
        assert!(view.read(cx).sidebar_visible);
        assert!(view.read(cx).history_focus.is_focused(window));
    })
    .unwrap();
}

// Test-only access stays inside the example, rather than expanding its public API.
impl Tig {
    fn seed_history(&mut self, cx: &mut gpui_kit::Context<Self>) {
        self._history_task = Task::ready(());
        self.loading_history = false;
        self.commits = Arc::new(
            (0..2)
                .map(|ix| Commit {
                    hash: if ix == 0 {
                        "a".repeat(40)
                    } else {
                        "b".repeat(40)
                    }
                    .into(),
                    short_hash: if ix == 0 { "aaaaaaa" } else { "bbbbbbb" }.into(),
                    author: "Example author".into(),
                    date: "2026-10-07".into(),
                    subject: format!("Commit {ix}").into(),
                })
                .collect(),
        );
        self.selected = Some(0);
        self.revision = 100;
        self.install_commit(100, Ok(details("old.txt", "old")), cx);
    }

    fn verify_loading_and_install(&mut self, cx: &mut gpui_kit::Context<Self>) {
        assert_eq!(self.selected, Some(1));
        assert!(self.loading_commit);
        assert!(self.diff.read(cx).files().is_empty());
        self._commit_task = Task::ready(());
        self.install_commit(self.revision - 1, Ok(details("old.txt", "old")), cx);
        assert!(self.loading_commit);
        assert!(self.selected_file.is_none());
        let mut commit = details("new.txt", "new");
        commit.files.extend(details("other.txt", "other").files);
        self.install_commit(self.revision, Ok(commit), cx);
        assert!(!self.loading_commit);
        assert_eq!(self.selected_file.as_deref(), Some("new.txt"));
        assert_eq!(self.diff.read(cx).files()[0].path().as_str(), "new.txt");
    }
}

#[gpui_kit::test]
fn long_commit_message_scrolls_and_resets_when_the_commit_changes(cx: &mut TestAppContext) {
    use gpui_kit::component::{
        Theme,
        diff::{DiffLineRange, DiffSide},
    };
    use gpui_kit::{
        Bounds, InputEvent as _, ScrollDelta, ScrollWheelEvent, TouchPhase, WindowBounds, point,
        px, size,
    };

    cx.update(gpui_kit::init);
    cx.update(super::init);
    let (handle, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1200.), px(800.)),
                ))),
                ..WindowOptions::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut view = Tig::new(Repository::new(PathBuf::from(".")), window, cx);
                    view.seed_history(cx);
                    view.message = (0..60)
                        .map(|ix| {
                            format!("Paragraph {ix}: explain this change and its validation.\n\n")
                        })
                        .collect::<String>()
                        .into();
                    view
                })
            },
        )
        .unwrap()
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("commit-message-toggle", cx);
        view.read(cx).diff.clone().update(cx, |state, cx| {
            state.set_selected_lines(
                Some(DiffLineRange::new("old.txt", DiffSide::Modified, 1, 1)),
                cx,
            );
        });
        window.render_frame(cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(Theme::global(cx).is_dark());
        let message = window.find("commit-message").bounds();
        window.dispatch_event(
            ScrollWheelEvent {
                position: message.center(),
                delta: ScrollDelta::Pixels(point(px(0.), px(-120.))),
                touch_phase: TouchPhase::Moved,
                modifiers: Default::default(),
            }
            .to_platform_input(),
            cx,
        );
        assert!(view.read(cx).message_scroll.offset().y < px(0.));
        assert_eq!(view.read(cx).diff.read(cx).selected_text(cx), "old\n");
        window.press("escape", cx);
        window.press("down", cx);
        assert!(!view.read(cx).message_visible);
        assert_eq!(view.read(cx).message_scroll.offset().y, px(0.));
    })
    .unwrap();
}

#[gpui_kit::test]
fn startup_file_is_selected_once_and_missing_paths_report_an_error(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, view) = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| {
                let mut view = Tig::new(Repository::new(PathBuf::from(".")), window, cx);
                view._history_task = Task::ready(());
                view.loading_history = false;
                view.with_initial_file(Some("other.txt".into()), Some(1))
            })
        })
        .unwrap()
    });
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |view, cx| {
            let mut commit = details("new.txt", "new");
            commit.files.extend(details("other.txt", "other").files);
            view.install_commit(view.revision, Ok(commit), cx);
            assert_eq!(view.selected_file.as_deref(), Some("other.txt"));
            assert!(view.initial_line.is_none());
            view.install_commit(view.revision, Ok(details("new.txt", "new")), cx);
            assert_eq!(view.selected_file.as_deref(), Some("new.txt"));
            view.initial_file = Some("missing.txt".into());
            view.install_commit(view.revision, Ok(details("new.txt", "new")), cx);
            assert!(view.error.as_deref().unwrap().contains("missing.txt"));
        });
        window.render_frame(cx);
    })
    .unwrap();
}
