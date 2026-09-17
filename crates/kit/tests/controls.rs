use gpui_kit::component::{
    Disableable, IndexPath,
    checkbox::Checkbox,
    select::{SearchableVec, Select, SelectState},
    switch::Switch,
    tab::{Tab, TabBar},
};
use gpui_kit::test::{TestAppContextExt, TestWindowExt};
use gpui_kit::{
    AnyWindowHandle, AppContext, Context, Entity, TestAppContext, Window, div, prelude::*, px, size,
};
use std::time::Duration;

struct Form {
    agreed: bool,
    notifications: bool,
    tab: usize,
    language: Entity<SelectState<SearchableVec<&'static str>>>,
}
impl Render for Form {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                Checkbox::new("agree")
                    .label("Agree")
                    .checked(self.agreed)
                    .on_change(cx.listener(|this, checked, _, cx| {
                        this.agreed = *checked;
                        cx.notify();
                    })),
            )
            .child(
                Switch::new("notifications")
                    .checked(self.notifications)
                    .on_change(cx.listener(|this, checked, _, cx| {
                        this.notifications = *checked;
                        cx.notify();
                    })),
            )
            .child(
                Checkbox::new("locked")
                    .label("Locked")
                    .checked(true)
                    .disabled(true),
            )
            .child(
                TabBar::new("settings-tabs")
                    .selected_index(self.tab)
                    .children([Tab::new().label("General"), Tab::new().label("Advanced")])
                    .on_click(cx.listener(|this, index, _, cx| {
                        this.tab = *index;
                        cx.notify();
                    })),
            )
            .child(
                Select::new(&self.language)
                    .id("language")
                    .title_prefix("Language: ")
                    .w(px(240.)),
            )
    }
}

#[gpui_kit::test]
fn checkbox_switch_and_tabs_report_controlled_state(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.open_window(size(px(640.), px(600.)), |window, cx| Form {
        agreed: false,
        notifications: false,
        tab: 0,
        language: cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(vec!["Rust", "Go"]),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        }),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let before = window.find("agree");
        assert_eq!(before.checked(), Some(false));
        window.click("agree", cx);
        assert_eq!(window.find("agree").checked(), Some(true));
        assert_eq!(
            before.checked(),
            Some(false),
            "an owned snapshot remains a record of its original frame"
        );
        window.click("notifications", cx);
        assert_eq!(window.find("notifications").checked(), Some(true));
        window.click("notifications", cx);
        assert_eq!(window.find("notifications").checked(), Some(false));
        window.click("locked", cx);
        assert_eq!(window.find("locked").disabled(), None);
        assert_eq!(window.find("locked").checked(), Some(true));
        assert_eq!(
            window.within("settings-tabs").find(0usize).selected(),
            Some(true)
        );
        window.within("settings-tabs").click(1usize, cx);
        assert_eq!(
            window.within("settings-tabs").find(0usize).selected(),
            Some(false)
        );
        assert_eq!(
            window.within("settings-tabs").find(1usize).selected(),
            Some(true)
        );
    })
    .unwrap();
}

#[gpui_kit::test]
async fn select_reports_value_and_keyboard_open_state(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.open_window(size(px(640.), px(600.)), |window, cx| Form {
        agreed: false,
        notifications: false,
        tab: 0,
        language: cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(vec!["Rust", "Go"]),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        }),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("language").value(), Some("Language: Rust"));
        assert_eq!(window.find("language").expanded(), Some(false));
        window.within("language").click("input", cx);
        assert_eq!(window.find("language").expanded(), Some(true));
        window.press("escape", cx);
        assert_eq!(window.find("language").expanded(), Some(false));
        window.within("language").click("input", cx);
        window.press("down", cx);
        window.press("enter", cx);
    })
    .unwrap();
    // Select commits through defer_in after the dispatch callback returns.
    cx.wait_for(handle.into(), Duration::from_millis(100), |window, _| {
        window.find("language").expanded() == Some(false)
            && window.find("language").value() == Some("Language: Go")
    })
    .await;
}

struct HoverHelp;
impl Render for HoverHelp {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        use gpui_kit::component::{button::Button, hover_card::HoverCard};
        use gpui_kit::test::TestSupportExt as _;
        div().size_full().flex().flex_col().gap_8().children([
            HoverCard::new("help")
                .open_delay(Duration::from_millis(30))
                .close_delay(Duration::from_millis(20))
                .trigger(Button::new("help-trigger").label("Help"))
                .content(|_, _, _| {
                    div()
                        .id("help-content")
                        .test_support()
                        .aria_label("Explanation")
                        .w(px(100.))
                        .h(px(40.))
                        .child("Explanation")
                })
                .into_any_element(),
            Button::new("away")
                .label("Away")
                .absolute()
                .bottom_0()
                .right_0()
                .into_any_element(),
        ])
    }
}

#[gpui_kit::test]
async fn real_hover_card_opens_and_closes_after_pointer_delays(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.open_window(size(px(640.), px(480.)), |window, cx| {
        let view = cx.new(|_| HoverHelp);
        gpui_kit::component::Root::new(view, window, cx)
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("help-content").is_none());
        window.hover("help-trigger", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_millis(100), |window, _| {
        window
            .try_find("help-content")
            .is_some_and(|snapshot| snapshot.visible())
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(window.find("help-content").label(), Some("Explanation"));
        window.hover("away", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_millis(100), |window, _| {
        window.try_find("help-content").is_none()
    })
    .await;
}

struct DisconnectedCheckbox;
impl Render for DisconnectedCheckbox {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // Deliberate application defect: activation never updates the owner.
        Checkbox::new("agree").label("Agree").checked(false)
    }
}

#[gpui_kit::test]
fn checkbox_click_cannot_fabricate_a_successful_state_change(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.add_window(|_, _| DisconnectedCheckbox);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("agree", cx);
        assert_eq!(window.find("agree").checked(), Some(false));
        window.press("space", cx);
        assert_eq!(window.find("agree").checked(), Some(false));
    })
    .unwrap();
}

struct SearchableLanguageForm {
    language: Entity<SelectState<SearchableVec<&'static str>>>,
}
impl Render for SearchableLanguageForm {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(Select::new(&self.language).id("language").w(px(240.)))
    }
}

fn open_searchable_language(cx: &mut TestAppContext) -> AnyWindowHandle {
    cx.update(gpui_kit::init);
    let handle = cx.open_window(size(px(640.), px(480.)), |window, cx| {
        let items = SearchableVec::new(vec!["Dutch", "English", "French", "Hungarian"]);
        SearchableLanguageForm {
            language: cx.new(|cx| {
                SelectState::new(items, Some(IndexPath::new(1)), window, cx).searchable(true)
            }),
        }
    });
    handle.into()
}

async fn wait_select_closed(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    cx.wait_for(handle, Duration::from_millis(500), |window, _| {
        window.find("language").expanded() == Some(false)
    })
    .await;
}

fn select_value(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Option<String> {
    cx.update_window(handle, |_, window, _| {
        window.find("language").value().map(str::to_owned)
    })
    .unwrap()
}

/// Opens the menu and types the query. The list filters in a task.
fn search_language(cx: &mut TestAppContext, handle: AnyWindowHandle, query: &str) {
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.within("language").click("input", cx);
        if !query.is_empty() {
            window.input(query, cx);
        }
    })
    .unwrap();
    cx.run_until_parked();
}

fn press_language_keys(cx: &mut TestAppContext, handle: AnyWindowHandle, keys: &[&str]) {
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        for key in keys {
            window.press(key, cx);
        }
    })
    .unwrap();
}

#[gpui_kit::test]
async fn searchable_select_next_open_after_cancel_shows_all_items(cx: &mut TestAppContext) {
    let handle = open_searchable_language(cx);

    search_language(cx, handle, "hun");
    press_language_keys(cx, handle, &["escape"]);
    wait_select_closed(cx, handle).await;

    // With an empty query, Down moves from English to French.
    search_language(cx, handle, "");
    press_language_keys(cx, handle, &["down", "enter"]);
    wait_select_closed(cx, handle).await;
    assert_eq!(select_value(cx, handle).as_deref(), Some("French"));
}

#[gpui_kit::test]
async fn searchable_select_next_open_after_confirm_shows_all_items(cx: &mut TestAppContext) {
    let handle = open_searchable_language(cx);

    search_language(cx, handle, "hun");
    press_language_keys(cx, handle, &["enter"]);
    wait_select_closed(cx, handle).await;
    assert_eq!(select_value(cx, handle).as_deref(), Some("Hungarian"));

    // With an empty query, Up moves from Hungarian to French.
    search_language(cx, handle, "");
    press_language_keys(cx, handle, &["up", "enter"]);
    wait_select_closed(cx, handle).await;
    assert_eq!(select_value(cx, handle).as_deref(), Some("French"));
}
