use gpui_kit::component::{Root, button::Button, menu::DropdownMenu};
use gpui_kit::test::{TestAppContextExt, TestSupportExt, TestWindowExt};
use gpui_kit::{AppContext, Context, TestAppContext, Window, actions, div, prelude::*, px, size};
use std::time::Duration;

actions!(menu_test, [Save, Unavailable]);
struct Commands {
    saved: bool,
    focus: gpui_kit::FocusHandle,
}
impl Render for Commands {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("workspace")
            .test_support()
            .track_focus(&self.focus)
            .size_full()
            .p_4()
            .on_action(cx.listener(|this, _: &Save, _, cx| {
                this.saved = true;
                cx.notify();
            }))
            .on_action(|_: &Unavailable, _, _| panic!("disabled command dispatched"))
            .child(
                Button::new("commands")
                    .label("Commands")
                    .dropdown_menu(|menu, window, cx| {
                        menu.menu_with_disabled("Unavailable", Box::new(Unavailable), true)
                            .menu("Save", Box::new(Save))
                            .submenu("More", window, cx, |menu, _, _| {
                                menu.menu("Save copy", Box::new(Save))
                            })
                    }),
            )
            .child(div().id("result").test_support().child(if self.saved {
                div().id("saved").test_support().child("Saved")
            } else {
                div().id("unsaved").test_support().child("Unsaved")
            }))
    }
}
#[gpui_kit::test]
async fn menu_skips_disabled_commands_confirms_and_restores_focus(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.open_window(size(px(640.), px(480.)), |window, cx| {
        let view = cx.new(|cx| Commands {
            saved: false,
            focus: cx.focus_handle(),
        });
        view.read(cx).focus.clone().focus(window, cx);
        Root::new(view, window, cx)
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("commands", cx);
        assert_eq!(window.find("popup-menu").focused(), Some(true));
        window.within("popup-menu").click(0usize, cx);
        assert!(window.find("unsaved").visible());
        window.within("popup-menu").press("down", cx);
        assert_eq!(
            window.within("popup-menu").find(1usize).selected(),
            Some(true)
        );
        window.within("popup-menu").press("enter", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("popup-menu").is_none() && window.try_find("saved").is_some()
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(window.find("workspace").focused(), Some(true));
        window.click("commands", cx);
        window.press("escape", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("popup-menu").is_none()
    })
    .await;
}

#[gpui_kit::test]
async fn hovering_submenu_opens_and_clicking_item_dismisses_the_chain(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.open_window(size(px(640.), px(480.)), |window, cx| {
        let view = cx.new(|cx| Commands {
            saved: false,
            focus: cx.focus_handle(),
        });
        view.read(cx).focus.clone().focus(window, cx);
        Root::new(view, window, cx)
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("commands", cx);
        let mut menu = window.within("popup-menu");
        menu.hover(2usize, cx);
        assert_eq!(menu.find(2usize).selected(), Some(true));
    })
    .unwrap();
    // Submenus already own a native "submenu" identity scope.
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("submenu").is_some()
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(
            window.within("submenu").find(0usize).label(),
            Some("Save copy")
        );
        window.within("submenu").click(0usize, cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("saved").is_some()
    })
    .await;
    cx.update_window(handle.into(), |_, window, _| {
        assert!(window.try_find("submenu").is_none());
        assert!(window.try_find("popup-menu").is_none());
    })
    .unwrap();
}

/// A scrollable menu whose only submenu item is the last of 31 rows, so it
/// starts scrolled out of the 160px viewport.
struct ScrollableCommands {
    focus: gpui_kit::FocusHandle,
}
impl Render for ScrollableCommands {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("workspace")
            .test_support()
            .track_focus(&self.focus)
            .size_full()
            .p_4()
            .child(
                Button::new("commands")
                    .label("Commands")
                    .dropdown_menu(|menu, window, cx| {
                        (0..30)
                            .fold(menu.scrollable(true).max_h(px(160.)), |menu, ix| {
                                menu.menu(format!("Item {ix}"), Box::new(Save))
                            })
                            .submenu("More", window, cx, |menu, _, _| {
                                menu.menu("Save copy", Box::new(Save))
                            })
                    }),
            )
    }
}

/// The submenu is painted as a deferred draw outside the items container, so
/// the container's `overflow_y_scroll` clip must not hide it and its items
/// must still be hit-testable.
#[gpui_kit::test]
async fn submenu_opens_unclipped_from_a_scrollable_menu(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.open_window(size(px(640.), px(480.)), |window, cx| {
        let view = cx.new(|cx| ScrollableCommands {
            focus: cx.focus_handle(),
        });
        view.read(cx).focus.clone().focus(window, cx);
        Root::new(view, window, cx)
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("commands", cx);
        let mut menu = window.within("popup-menu");
        // `up` wraps to the last item and scrolls it into view, where the
        // pointer can reach it.
        menu.press("up", cx);
        assert!(menu.find(30usize).visible(), "{:?}", menu.find(30usize));
        menu.hover(30usize, cx);
        assert_eq!(menu.find(30usize).selected(), Some(true));
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("submenu").is_some()
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        let submenu = window.find("submenu");
        let viewport = gpui_kit::Bounds::new(Default::default(), window.viewport_size());
        assert!(submenu.visible(), "submenu is clipped: {submenu:?}");
        assert!(
            viewport.contains(&submenu.bounds().origin)
                && viewport.contains(&submenu.bounds().bottom_right()),
            "submenu {:?} is outside the window {viewport:?}",
            submenu.bounds()
        );
        let mut submenu = window.within("submenu");
        assert_eq!(submenu.find(0usize).label(), Some("Save copy"));
        submenu.click(0usize, cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("popup-menu").is_none()
    })
    .await;
}
