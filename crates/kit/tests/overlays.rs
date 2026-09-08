use gpui_kit::component::{
    Root, WindowExt,
    button::Button,
    dialog::{DialogAction, DialogFooter},
    input::{Input, InputState},
    notification::Notification,
};
use gpui_kit::test::{TestAppContextExt, TestWindowExt};
use gpui_kit::{AppContext, Context, Entity, TestAppContext, Window, div, prelude::*, px, size};
use std::time::Duration;

struct Workspace {
    saved: Entity<InputState>,
    draft: Entity<InputState>,
}
impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialogs = Root::render_dialog_layer(window, cx);
        let sheets = Root::render_sheet_layer(window, cx);
        let notifications = Root::render_notification_layer(window, cx);
        let draft = self.draft.clone();
        let saved = self.saved.clone();
        div()
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .child(Input::new(&self.saved).id("name"))
            .child(
                Button::new("edit")
                    .label("Edit…")
                    .on_click(move |_, window, cx| {
                        let draft = draft.clone();
                        let saved = saved.clone();
                        window.open_dialog(cx, move |dialog, _, _| {
                            let draft = draft.clone();
                            let saved = saved.clone();
                            dialog
                                .title("Edit profile")
                                .child(Input::new(&draft).id("name"))
                                .footer(DialogFooter::new().child(
                                    DialogAction::new().child(Button::new("ok").label("Save")),
                                ))
                                .on_ok(move |_, window, cx| {
                                    if draft.read(cx).value().is_empty() {
                                        return false;
                                    }
                                    let value = draft.read(cx).value();
                                    saved
                                        .update(cx, |input, cx| input.set_value(value, window, cx));
                                    window.push_notification(
                                        Notification::new()
                                            .message("Profile saved")
                                            .autohide(false),
                                        cx,
                                    );
                                    true
                                })
                        });
                    }),
            )
            .child(
                Button::new("inspect")
                    .label("Inspect…")
                    .on_click(|_, window, cx| {
                        window.open_sheet(cx, |sheet, _, _| {
                            sheet.title("Inspector").child("Details")
                        });
                    }),
            )
            .child(
                Button::new("notify")
                    .label("Notify")
                    .on_click(|_, window, cx| {
                        window.push_notification(
                            Notification::new().message("Updated").autohide(true),
                            cx,
                        );
                    }),
            )
            .children(dialogs)
            .children(sheets)
            .children(notifications)
    }
}

#[gpui_kit::test]
async fn dialog_validates_scoped_input_saves_and_dismisses_notification(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.open_window(size(px(800.), px(700.)), |window, cx| {
        let view = cx.new(|cx| Workspace {
            saved: cx.new(|cx| InputState::new(window, cx)),
            draft: cx.new(|cx| InputState::new(window, cx)),
        });
        Root::new(view, window, cx)
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("name", cx);
        window.click("edit", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("dialog").is_some()
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        let dialog = window.within("dialog").find(0usize).bounds();
        assert!(dialog.left() >= px(0.) && dialog.right() <= window.viewport_size().width);
        assert!(dialog.top() >= px(0.) && dialog.bottom() <= window.viewport_size().height);
        window.within("dialog").click("ok", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("notification").is_none());
        assert!(
            window.find("dialog").visible(),
            "empty input must prevent confirmation"
        );
        window.within("dialog").click("name", cx);
        window.within("dialog").input("Ada 中文", cx);
        assert_eq!(
            window.within("dialog").find("name").value(),
            Some("Ada 中文")
        );
        window.within("dialog").click("ok", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("dialog").is_none() && window.try_find("notification").is_some()
    })
    .await;
    // Toast entrance also uses GPUI's wall-clock Animation (400 ms).
    std::thread::sleep(Duration::from_millis(410));
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(window.find("name").value(), Some("Ada 中文"));
        assert!(window.find("notification").visible());
        window.hover("notification", cx);
        window.within("notification").click("close", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("notification").is_none()
    })
    .await;
}

#[gpui_kit::test]
async fn escape_dismisses_dialog_and_sheet_and_restores_focus(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.open_window(size(px(800.), px(700.)), |window, cx| {
        let view = cx.new(|cx| Workspace {
            saved: cx.new(|cx| InputState::new(window, cx)),
            draft: cx.new(|cx| InputState::new(window, cx)),
        });
        Root::new(view, window, cx)
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("name", cx);
        window.click("edit", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("dialog").is_some()
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        window.press("escape", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("dialog").is_none() && window.find("name").focused() == Some(true)
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(window.find("name").focused(), Some(true));
        window.click("inspect", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("sheet-content").is_some()
    })
    .await;
    // GPUI's non-synced Animation uses std::time::Instant, not the test clock.
    // Finish the actual 150 ms entrance before asserting its final geometry.
    std::thread::sleep(Duration::from_millis(160));
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let sheet = window.find("sheet-content").bounds();
        assert!(
            sheet.size.width > px(0.) && sheet.right() <= window.viewport_size().width,
            "{sheet:?}"
        );
        window.press("escape", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("sheet").is_none() && window.try_find("sheet-content").is_none()
    })
    .await;
    cx.update_window(handle.into(), |_, window, _| {
        assert_eq!(window.find("name").focused(), Some(true));
    })
    .unwrap();
}

#[gpui_kit::test]
async fn notification_auto_dismisses_after_its_timer(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.open_window(size(px(800.), px(700.)), |window, cx| {
        let view = cx.new(|cx| Workspace {
            saved: cx.new(|cx| InputState::new(window, cx)),
            draft: cx.new(|cx| InputState::new(window, cx)),
        });
        Root::new(view, window, cx)
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("notify", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("notification").is_some()
    })
    .await;
    // The default timeout is five seconds; the notification must remain
    // mounted before that deadline, not merely disappear eventually.
    for _ in 0..40 {
        cx.background_executor
            .advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("notification").is_some());
        })
        .unwrap();
    }
    cx.wait_for(handle.into(), Duration::from_secs(10), |window, _| {
        window.try_find("notification").is_none()
    })
    .await;
}
