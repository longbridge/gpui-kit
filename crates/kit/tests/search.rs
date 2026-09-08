//! Real searchable controls: native input, filtering, selection and owner callbacks.
use gpui_kit::component::{
    Disableable, IndexPath, Root,
    command::{Command, CommandGroup, CommandItem, CommandState},
};
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{AppContext, Context, Entity, TestAppContext, Window, div, prelude::*, px, size};

gpui_kit::actions!(search_test, [Save]);

struct Palette {
    state: Entity<CommandState>,
    confirmed: Vec<IndexPath>,
    cancellations: usize,
    saves: usize,
    queries: Vec<String>,
}
impl Render for Palette {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let confirm = cx.entity();
        let cancel = cx.entity();
        let query = cx.entity();
        div()
            .size_full()
            .on_action(cx.listener(|this, _: &Save, _, cx| {
                this.saves += 1;
                cx.notify();
            }))
            .child(
                Command::new(&self.state)
                    .group(
                        CommandGroup::new()
                            .label("Files")
                            .item(CommandItem::new().label("Open"))
                            .item(CommandItem::new().label("Delete").disabled(true))
                            .item(
                                CommandItem::new()
                                    .label("Save")
                                    .keywords(["persist", "保存"])
                                    .action(Box::new(Save)),
                            ),
                    )
                    .empty(|_, _, _| div().id("no-results").test_support().child("No commands"))
                    .on_confirm(move |index, _, cx| {
                        confirm.update(cx, |view, cx| {
                            view.confirmed.push(index);
                            cx.notify();
                        })
                    })
                    .on_cancel(move |_, cx| {
                        cancel.update(cx, |view, cx| {
                            view.cancellations += 1;
                            cx.notify();
                        })
                    })
                    .on_query(move |value, _, cx| {
                        query.update(cx, |view, _| {
                            view.queries.push(value.to_owned());
                        })
                    }),
            )
    }
}
fn palette(cx: &mut TestAppContext) -> (gpui_kit::WindowHandle<Root>, Entity<Palette>) {
    cx.update(gpui_kit::init);
    let mut view = None;
    let window = cx.open_window(size(px(640.), px(480.)), |window, cx| {
        let entity = cx.new(|cx| Palette {
            state: cx.new(|cx| CommandState::new(window, cx)),
            confirmed: vec![],
            cancellations: 0,
            saves: 0,
            queries: vec![],
        });
        view = Some(entity.clone());
        Root::new(entity, window, cx)
    });
    let view = view.unwrap();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        let state = view.read(cx).state.clone();
        state.update(cx, |state, cx| state.focus(window, cx));
        window.render_frame(cx);
    })
    .unwrap();
    (window, view)
}

#[gpui_kit::test]
fn command_skips_disabled_rows_wraps_and_confirms_original_index(cx: &mut TestAppContext) {
    let (handle, view) = palette(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(window.find(IndexPath::new(0)).selected(), Some(true));
        window.click(IndexPath::new(1), cx);
        window.press("down", cx);
        assert_eq!(window.find(IndexPath::new(2)).selected(), Some(true));
        window.press("down", cx);
        assert_eq!(window.find(IndexPath::new(0)).selected(), Some(true));
        window.press("up", cx);
        assert_eq!(window.find(IndexPath::new(2)).selected(), Some(true));
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(view.read(cx).confirmed, vec![IndexPath::new(2)]);
        assert_eq!(view.read(cx).saves, 1);
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(IndexPath::new(0), cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(
            view.read(cx).confirmed,
            vec![IndexPath::new(2), IndexPath::new(0)]
        )
    });
}

#[gpui_kit::test]
fn command_searches_unicode_keywords_and_escape_clears_before_cancel(cx: &mut TestAppContext) {
    let (handle, view) = palette(cx);
    cx.update_window(handle.into(), |_, window, cx| window.input("保存", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find(IndexPath::new(0)).is_none());
        assert_eq!(window.find(IndexPath::new(2)).selected(), Some(true));
        assert_eq!(view.read(cx).state.read(cx).matched_count(), 1);
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(view.read(cx).confirmed, vec![IndexPath::new(2)]);
        assert_eq!(view.read(cx).saves, 1);
    });
    cx.update_window(handle.into(), |_, window, cx| window.press("escape", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(view.read(cx).state.read(cx).query(cx), "");
        assert_eq!(view.read(cx).cancellations, 0);
        assert!(window.find(IndexPath::new(0)).visible());
        window.press("escape", cx);
    })
    .unwrap();
    cx.update(|cx| {
        assert_eq!(view.read(cx).cancellations, 1);
        assert!(view.read(cx).queries.iter().any(|query| query == "保存"));
    });
}

#[gpui_kit::test]
fn command_empty_and_disabled_only_results_cannot_confirm(cx: &mut TestAppContext) {
    let (handle, view) = palette(cx);
    for query in ["missing", "Delete"] {
        cx.update_window(handle.into(), |_, window, cx| window.input(query, cx))
            .unwrap();
        cx.run_until_parked();
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            assert_eq!(view.read(cx).state.read(cx).selected_index(), None);
            if query == "missing" {
                assert!(window.find("no-results").visible());
            } else {
                window.click(IndexPath::new(1), cx);
            }
            window.press("down", cx);
            window.press("enter", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update(|cx| assert!(view.read(cx).confirmed.is_empty()));
        cx.update_window(handle.into(), |_, window, cx| window.press("escape", cx))
            .unwrap();
        cx.run_until_parked();
    }
}

use gpui_kit::component::{
    combobox::{Combobox, ComboboxEvent, ComboboxState},
    searchable_list::SearchableVec,
};
use gpui_kit::test::TestAppContextExt;
use std::time::Duration;

type LanguageState = ComboboxState<SearchableVec<&'static str>>;
struct Languages {
    state: Entity<LanguageState>,
    disabled: bool,
    changes: Vec<Vec<&'static str>>,
    confirmations: Vec<Vec<&'static str>>,
    _subscription: gpui_kit::Subscription,
}
impl Render for Languages {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p_4().child(
            div()
                .id("language-field")
                .test_support()
                .w_64()
                .h_10()
                .child(
                    Combobox::new(&self.state)
                        .disabled(self.disabled)
                        .cleanable(true)
                        .empty(|_, _| {
                            div()
                                .id("no-languages")
                                .test_support()
                                .child("No languages")
                        }),
                ),
        )
    }
}
fn languages(
    cx: &mut TestAppContext,
    multiple: bool,
    disabled: bool,
) -> (gpui_kit::WindowHandle<Root>, Entity<Languages>) {
    cx.update(gpui_kit::init);
    let mut view = None;
    let handle = cx.open_window(size(px(640.), px(480.)), |window, cx| {
        let entity = cx.new(|cx| {
            let state = cx.new(|cx| {
                ComboboxState::new(
                    SearchableVec::new(vec!["Rust", "Go", "中文"]),
                    vec![],
                    window,
                    cx,
                )
                .multiple(multiple)
                .searchable(true)
            });
            let subscription =
                cx.subscribe(&state, |this: &mut Languages, _, event, _| match event {
                    ComboboxEvent::Change(values) => this.changes.push(values.clone()),
                    ComboboxEvent::Confirm(values) => this.confirmations.push(values.clone()),
                });
            Languages {
                state,
                disabled,
                changes: vec![],
                confirmations: vec![],
                _subscription: subscription,
            }
        });
        view = Some(entity.clone());
        Root::new(entity, window, cx)
    });
    (handle, view.unwrap())
}

#[gpui_kit::test]
async fn combobox_filters_and_commits_single_selection(cx: &mut TestAppContext) {
    let (handle, view) = languages(cx, false, false);
    let id = cx.update(|cx| ("multi-combo-box", view.read(cx).state.entity_id()));
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find(id).expanded(), Some(false));
        window.click("language-field", cx);
        assert_eq!(window.find(id).expanded(), Some(true));
        window.input("中文", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(view.read(cx).state.read(cx).query(cx), "中文");
        window.press("enter", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.find(id).expanded() == Some(false)
    })
    .await;
    cx.update_window(handle.into(), |_, window, _| {
        assert_eq!(window.find(id).focused(), Some(true));
    })
    .unwrap();
    cx.update(|cx| {
        let view = view.read(cx);
        assert_eq!(view.state.read(cx).selected_value(), Some("中文"));
        assert_eq!(view.changes, vec![vec!["中文"]]);
        assert_eq!(view.confirmations, vec![vec!["中文"]]);
    });
}

#[gpui_kit::test]
async fn combobox_multiple_selection_toggles_and_confirms_on_escape(cx: &mut TestAppContext) {
    let (handle, view) = languages(cx, true, false);
    let id = cx.update(|cx| ("multi-combo-box", view.read(cx).state.entity_id()));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("language-field", cx);
        window.press("down", cx);
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find(id).expanded(), Some(true));
        assert_eq!(view.read(cx).state.read(cx).selected_values(), vec!["Rust"]);
        window.press("down", cx);
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(
            view.read(cx).state.read(cx).selected_values(),
            vec!["Rust", "Go"]
        );
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| window.press("escape", cx))
        .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.find(id).expanded() == Some(false)
    })
    .await;
    cx.update_window(handle.into(), |_, window, _| {
        assert_eq!(window.find(id).focused(), Some(true));
    })
    .unwrap();
    cx.update(|cx| {
        let view = view.read(cx);
        assert_eq!(
            view.changes,
            vec![vec!["Rust"], vec!["Rust", "Go"], vec!["Rust"]]
        );
        assert_eq!(view.confirmations, vec![vec!["Rust"]]);
        assert_eq!(view.state.read(cx).selected_values(), vec!["Rust"]);
    });
}

#[gpui_kit::test]
fn disabled_combobox_rejects_opening_and_emits_no_selection(cx: &mut TestAppContext) {
    let (handle, view) = languages(cx, false, true);
    let id = cx.update(|cx| ("multi-combo-box", view.read(cx).state.entity_id()));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("language-field", cx);
        window.press("down", cx);
        window.press("enter", cx);
        assert_eq!(window.find(id).expanded(), Some(false));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        let view = view.read(cx);
        assert!(view.state.read(cx).selected_values().is_empty());
        assert!(view.changes.is_empty());
        assert!(view.confirmations.is_empty());
    });
}

#[gpui_kit::test]
async fn combobox_empty_search_cannot_select_and_can_recover(cx: &mut TestAppContext) {
    let (handle, view) = languages(cx, false, false);
    let id = cx.update(|cx| ("multi-combo-box", view.read(cx).state.entity_id()));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("language-field", cx);
        window.input("missing", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("no-languages").is_some()
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        assert!(window.find("no-languages").visible());
        window.press("down", cx);
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert!(view.read(cx).state.read(cx).selected_values().is_empty());
        assert!(view.read(cx).changes.is_empty());
        assert!(view.read(cx).confirmations.is_empty());
        assert_eq!(window.find(id).expanded(), Some(true));
        window.press("secondary-a", cx);
        window.input("Go", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("no-languages").is_none()
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        // An empty result clears the list cursor; navigate to the recovered row.
        window.press("down", cx);
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        let view = view.read(cx);
        assert_eq!(view.state.read(cx).selected_value(), Some("Go"));
        assert_eq!(view.changes, vec![vec!["Go"]]);
        assert_eq!(view.confirmations, vec![vec!["Go"]]);
    });
}

#[gpui_kit::test]
fn combobox_clear_button_updates_selection_without_reopening(cx: &mut TestAppContext) {
    let (handle, view) = languages(cx, false, false);
    let id = cx.update(|cx| ("multi-combo-box", view.read(cx).state.entity_id()));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("language-field", cx);
        window.press("down", cx);
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(view.read(cx).state.read(cx).selected_value(), Some("Rust"));
        window.click("clean", cx);
        assert_eq!(window.find(id).expanded(), Some(false));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        let view = view.read(cx);
        assert!(view.state.read(cx).selected_values().is_empty());
        assert_eq!(view.changes, vec![vec!["Rust"], vec![]]);
        assert_eq!(view.confirmations, vec![vec!["Rust"]]);
    });
}
