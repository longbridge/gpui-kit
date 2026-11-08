use crate::common;
use gpui_component::{
    Disableable, TitleBar,
    button::{Button, DropdownButton},
    clipboard::Clipboard,
    input::{Input, InputState, Textarea, TextareaState},
    popover::Popover,
    progress::{Progress, ProgressCircle},
};
use gpui_kit::test::{TestQueryExt, TestSupportExt, TestWindowExt};
use gpui_kit::{AppContext, Context, Entity, TestAppContext, Window, div, prelude::*, px, size};

struct Controls {
    input: Entity<InputState>,
    clicks: usize,
}
impl Render for Controls {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                Button::new("disabled")
                    .label("Disabled")
                    .disabled(true)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.clicks += 1;
                        cx.notify();
                    })),
            )
            .child(Input::new(&self.input).id("search").w(px(240.)))
            .child(
                Popover::new("popover-host")
                    .trigger(Button::new("open").label("Open"))
                    .content(|_, _, _| {
                        div()
                            .id("popover-content")
                            .test_support()
                            .w(px(120.))
                            .h(px(50.))
                            .child("Details")
                    }),
            )
    }
}

#[gpui_kit::test]
fn kit_controls_use_native_events_and_report_state(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let (handle, handle_content) =
        common::open_window(cx, Some(size(px(600.), px(500.))), |window, cx| {
            cx.new(|cx| Controls {
                input: cx.new(|cx| InputState::new(window, cx)),
                clicks: 0,
            })
        });
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(window.find("disabled").disabled(), Some(true));
        window.click("disabled", cx);
        window.click("search", cx);
        assert_eq!(window.find("search").focused(), Some(true));
        window.input("GPUI 中文 🦀", cx);
        assert_eq!(window.find("search").value(), Some("GPUI 中文 🦀"));
        assert!(window.try_find("popover-content").is_none());
        window.click("open", cx);
        let content = window.find("popover-content");
        assert!(content.visible());
        assert!(content.bounds().top() >= window.find("open").bounds().bottom());
    })
    .unwrap();
    common::update_content(handle, &handle_content, cx, |view, _, _| {
        assert_eq!(view.clicks, 0)
    })
    .unwrap();
}

struct PopoverKeyboard {
    draft: Entity<TextareaState>,
    applied: usize,
}

impl Render for PopoverKeyboard {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity().downgrade();
        let draft = self.draft.clone();
        div().size_full().p_4().child(
            Popover::new("keyboard-popover")
                .trigger(Button::new("keyboard-trigger").label("Compose"))
                .content(move |_, _, _| {
                    let owner = owner.clone();
                    div()
                        .id("keyboard-content")
                        .test_support()
                        .w(px(280.))
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(Button::new("keyboard-apply").label("Apply").on_click(
                            move |_, _, cx| {
                                let _ = owner.update(cx, |view, cx| {
                                    view.applied += 1;
                                    cx.notify();
                                });
                            },
                        ))
                        .child(Textarea::new(&draft).aria_label("Draft").h(px(100.)))
                }),
        )
    }
}

#[gpui_kit::test]
fn popover_content_keeps_keyboard_activation_and_text_input(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    for activation in ["enter", "space"] {
        let (handle, view) =
            common::open_window(cx, Some(size(px(600.), px(500.))), |window, cx| {
                cx.new(|cx| PopoverKeyboard {
                    draft: cx.new(|cx| TextareaState::new(window, cx)),
                    applied: 0,
                })
            });
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            window.focus_next(cx);
            window.render_frame(cx);
            assert_eq!(window.find("keyboard-trigger").focused(), Some(true));
            window.press(activation, cx);
            assert!(window.find("keyboard-content").visible());
            window.press("tab", cx);
            assert_eq!(window.find("keyboard-apply").focused(), Some(true));
            window.press(activation, cx);
            assert_eq!(view.read(cx).applied, 1, "{activation} activates the child");
            assert!(window.find("keyboard-content").visible());

            window.press("tab", cx);
            assert_eq!(window.find_by_label("Draft").focused(), Some(true));
            window.input("A", cx);
            window.press("enter", cx);
            window.press("space", cx);
            window.input("B", cx);
            assert_eq!(view.read(cx).draft.read(cx).value(), "A\n B");
            assert!(window.find("keyboard-content").visible());
            // Textarea owns Shift-Tab for outdent; use the traversal primitive.
            window.focus_prev(cx);
            window.render_frame(cx);
            assert_eq!(window.find("keyboard-apply").focused(), Some(true));
            window.press("escape", cx);
            assert!(window.try_find("keyboard-content").is_none());
            assert_eq!(window.find("keyboard-trigger").focused(), Some(true));
            assert_eq!(
                view.read(cx).applied,
                1,
                "dismissal must not activate again"
            );
        })
        .unwrap();
    }
}

struct NamedButton;
impl Render for NamedButton {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Button::new("save")
            .label("Save")
            .accessibility_label("Save this document")
    }
}

#[gpui_kit::test]
fn button_reports_accessibility_name_without_claiming_visible_text(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let (handle, _) = common::open_window(cx, None, |_, cx| cx.new(|_| NamedButton));
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(window.find("save").label(), Some("Save this document"));
    })
    .unwrap();
}

struct NamedClipboard;
impl Render for NamedClipboard {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(
                Clipboard::new("copy-key")
                    .value("sk-1234")
                    .tooltip("Copy")
                    .accessibility_label("Copy API key"),
            )
            .child(
                Clipboard::new("copy-plain")
                    .value("sk-1234")
                    .tooltip("Copy"),
            )
    }
}

#[gpui_kit::test]
fn clipboard_reports_default_and_explicit_accessibility_names(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let (handle, _) = common::open_window(cx, None, |_, cx| cx.new(|_| NamedClipboard));
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(window.find("copy-key").label(), Some("Copy API key"));
        assert_eq!(
            window.find("copy-plain").label(),
            Some("Copy"),
            "an icon-only Clipboard needs a name without a caller-provided label"
        );
    })
    .unwrap();
}

struct NamedDropdownButtons;
impl Render for NamedDropdownButtons {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(
                DropdownButton::new("save")
                    .button(Button::new("save-action").label("Save"))
                    .menu_accessibility_label("More save options")
                    .dropdown_menu(|menu, _, _| menu),
            )
            .child(
                DropdownButton::new("plain")
                    .button(Button::new("plain-action").label("Run"))
                    .dropdown_menu(|menu, _, _| menu),
            )
    }
}

#[gpui_kit::test]
fn dropdown_button_names_its_menu_trigger(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let (handle, _) = common::open_window(cx, None, |_, cx| cx.new(|_| NamedDropdownButtons));
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(
            window.within("save").find("popup").label(),
            Some("More save options")
        );
        assert_eq!(
            window.within("plain").find("popup").label(),
            Some("More options"),
            "the caret-only trigger needs a name without a caller-provided label"
        );
    })
    .unwrap();
}

struct ScrollFocus {
    focus: gpui_kit::FocusHandle,
}
impl Render for ScrollFocus {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        use gpui_component::scroll::ScrollableElement as _;
        div()
            .id("original")
            .test_support()
            .size(px(100.))
            .overflow_y_scrollbar()
            .id("scroll")
            .track_focus(&self.focus)
    }
}
#[gpui_kit::test]
fn scrollable_elements_forward_observed_focus_binding(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let (handle, handle_content) = common::open_window(cx, None, |_, cx| {
        cx.new(|cx| ScrollFocus {
            focus: cx.focus_handle(),
        })
    });
    let focus =
        common::update_content(handle, &handle_content, cx, |view, _, _| view.focus.clone())
            .unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        let content = (gpui_kit::ElementId::from("scroll"), "content");
        window.render_frame(cx);
        assert_eq!(
            window.within("scroll").find(content.clone()).focused(),
            Some(false)
        );
        window.focus(&focus, cx);
        window.render_frame(cx);
        assert_eq!(
            window.within("scroll").find(content.clone()).focused(),
            Some(true)
        );
        window.blur(cx);
        window.render_frame(cx);
        assert_eq!(window.within("scroll").find(content).focused(), Some(false));
    })
    .unwrap();
}

struct Progresses;
impl Render for Progresses {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(240.))
            .flex()
            .flex_col()
            .gap_4()
            .child(
                Progress::new("download")
                    .value(40.)
                    .accessibility_label("Downloading release"),
            )
            .child(
                ProgressCircle::new("sync")
                    .loading(true)
                    .accessibility_label("Syncing library"),
            )
    }
}

#[gpui_kit::test]
fn progress_and_progress_circle_are_observable(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let (handle, _) = common::open_window(cx, None, |_, cx| cx.new(|_| Progresses));
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        for (id, label) in [
            ("download", "Downloading release"),
            ("sync", "Syncing library"),
        ] {
            let progress = window.find(id);
            assert_eq!(progress.role(), Some(gpui_kit::Role::ProgressIndicator));
            assert_eq!(progress.label(), Some(label));
            assert!(progress.visible(), "{id} should be visible");
        }
    })
    .unwrap();
}

struct WindowWithTitleBar;
impl Render for WindowWithTitleBar {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(TitleBar::new().child("Library"))
    }
}

#[gpui_kit::test]
fn title_bar_and_window_controls_are_observable(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let (handle, _) = common::open_window(cx, Some(size(px(600.), px(400.))), |_, cx| {
        cx.new(|_| WindowWithTitleBar)
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert!(window.find("title-bar").visible());
        // Windows draws the caption buttons in the title bar. macOS and the web
        // leave them to the system, and so does Linux under the server-side
        // decorations of the test window.
        #[cfg(target_os = "windows")]
        {
            let controls = window.find("window-controls");
            for id in ["minimize", "maximize", "close"] {
                let button = window.within("window-controls").find(id);
                assert!(button.visible(), "{id} should be visible");
                assert!(controls.bounds().contains(&button.bounds().center()));
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            assert!(window.try_find("window-controls").is_some());
            assert!(window.try_find("close").is_none());
        }
    })
    .unwrap();
}
