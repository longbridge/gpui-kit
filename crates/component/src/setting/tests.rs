use super::*;
use crate::{
    Root,
    setting::{SettingGroup, SettingItem},
};
use gpui::{
    Context, InteractiveElement as _, Render, TestAppContext, VisualTestContext, point, size,
};

struct SettingsHost {
    pages: Vec<SettingPage>,
    state: Option<Entity<SettingsState>>,
}

impl Render for SettingsHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = Settings::new("search-test")
            .pages(self.pages.clone())
            .default_selected_index(SelectIndex {
                page_ix: 1,
                group_ix: None,
            })
            .render(window, cx)
            .into_any_element();
        self.state = Some(window.use_keyed_state("search-test", cx, |_, _| unreachable!()));
        div().size_full().child(settings)
    }
}

fn item(keyword: &'static str) -> SettingItem {
    item_with_height(keyword, 80.)
}

fn item_with_height(keyword: &'static str, height: f32) -> SettingItem {
    SettingItem::render(move |options, _, _| {
        let selector = format!(
            "setting-{}-{}-{}",
            options.page_ix(),
            options.group_ix(),
            options.item_ix()
        );
        div()
            .h(px(height))
            .child("Setting")
            .debug_selector(move || selector.clone())
    })
    .keywords([keyword])
}

fn setup(cx: &mut TestAppContext) -> (Entity<SettingsHost>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init(cx);
        crate::Theme::global_mut(cx).font_size = px(16.);
    });
    let pages = vec![
        SettingPage::new("General").group(SettingGroup::new().item(item("language"))),
        SettingPage::new("Appearance").default_open(true).groups([
            SettingGroup::new().item(item("unrelated")),
            SettingGroup::new()
                .title("Colors")
                .item(item("theme colors")),
            SettingGroup::new()
                .title("Fonts")
                .items([item("unrelated"), item("theme font")]),
        ]),
        SettingPage::new("Editor").group(SettingGroup::new().item(item("theme editor"))),
    ];
    let mut host = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| SettingsHost { pages, state: None });
        host = Some(view.clone());
        Root::new(view, window, cx)
    });
    cx.simulate_resize(size(px(1000.), px(700.)));
    draw(cx);
    (host.unwrap(), cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn search(host: &Entity<SettingsHost>, query: &str, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        let input = host
            .read(cx)
            .state
            .as_ref()
            .unwrap()
            .read(cx)
            .search_input
            .clone();
        input.update(cx, |input, cx| input.set_value(query, window, cx));
    });
    draw(cx);
}

fn selection(host: &Entity<SettingsHost>, cx: &mut VisualTestContext) -> (usize, Option<usize>) {
    cx.update(|_, cx| {
        let selected = host
            .read(cx)
            .state
            .as_ref()
            .unwrap()
            .read(cx)
            .selected_index;
        (selected.page_ix, selected.group_ix)
    })
}

fn click_nav(y: f32, cx: &mut VisualTestContext) {
    // Fixed-size fixture with a 16px rem; click the label area, away from carets.
    cx.simulate_click(point(px(50.), px(y)), Default::default());
    draw(cx);
}

#[gpui::test]
fn search_preserves_the_page_and_clicks_use_original_indices(cx: &mut TestAppContext) {
    let (host, cx) = setup(cx);
    // The old numeric index is still in range, but would point to Editor.
    search(&host, "theme", cx);
    assert_eq!(selection(&host, cx), (1, None));
    assert!(cx.debug_bounds("setting-1-1-0").is_some());
    assert!(cx.debug_bounds("setting-2-0-0").is_none());

    // Only Appearance matches; the old implementation renders a blank page.
    search(&host, "font", cx);
    assert_eq!(selection(&host, cx), (1, None));
    assert!(cx.debug_bounds("setting-1-2-1").is_some());

    // Select Editor from the compressed result list, then clear the query.
    search(&host, "theme", cx);
    click_nav(156., cx);
    assert_eq!(selection(&host, cx), (2, None));
    search(&host, "", cx);
    assert_eq!(selection(&host, cx), (2, None));
    assert!(cx.debug_bounds("setting-2-0-0").is_some());

    // The current page disappears: choose the first matching page.
    search(&host, "font", cx);
    assert_eq!(selection(&host, cx), (1, None));
    assert!(cx.debug_bounds("setting-1-2-1").is_some());
    search(&host, "no matching setting", cx);
    assert_eq!(selection(&host, cx), (1, None));
    assert!(cx.debug_bounds("setting-1-2-1").is_none());
    search(&host, "", cx);
    assert_eq!(selection(&host, cx), (1, None));
}

#[gpui::test]
fn search_preserves_group_and_item_identity(cx: &mut TestAppContext) {
    let (host, cx) = setup(cx);
    // Make the target group require scrolling, with an unnamed group before it.
    cx.update(|_, cx| {
        host.update(cx, |host, cx| {
            host.pages[1].groups[1].items = vec![item_with_height("theme colors", 450.)];
            cx.notify();
        });
    });
    draw(cx);
    click_nav(156., cx);
    assert_eq!(selection(&host, cx), (1, Some(2)));
    search(&host, "theme", cx);
    assert_eq!(selection(&host, cx), (1, Some(2)));
    assert!(cx.debug_bounds("setting-1-2-1").is_some());
    assert!(cx.debug_bounds("setting-1-2-0").is_none());
    // Click Fonts after the leading page/group/item have been filtered out.
    click_nav(120., cx);
    assert_eq!(selection(&host, cx), (1, Some(2)));
    let target = cx.debug_bounds("setting-1-2-1").unwrap();
    assert!(target.top() >= px(0.) && target.bottom() <= px(700.));
    search(&host, "font", cx);
    assert_eq!(selection(&host, cx), (1, Some(2)));
    search(&host, "", cx);
    assert_eq!(selection(&host, cx), (1, Some(2)));
    search(&host, "colors", cx);
    assert_eq!(selection(&host, cx), (1, None));
    assert!(cx.debug_bounds("setting-1-1-0").is_some());
}

#[gpui::test]
fn footer_follows_group_search_visibility(cx: &mut TestAppContext) {
    let (host, cx) = setup(cx);
    cx.update(|_, cx| {
        host.update(cx, |host, cx| {
            host.pages[1].groups[2] = host.pages[1].groups[2].clone().footer(|_, _| {
                div()
                    .child("Changes apply to this device only.")
                    .debug_selector(|| "font-footer".into())
            });
            cx.notify();
        });
    });
    search(&host, "font", cx);
    assert!(cx.debug_bounds("setting-1-2-1").is_some());
    assert!(cx.debug_bounds("font-footer").is_some());

    // Footer copy does not independently make a group match the query.
    search(&host, "colors", cx);
    assert!(cx.debug_bounds("setting-1-1-0").is_some());
    assert!(cx.debug_bounds("font-footer").is_none());

    search(&host, "font", cx);
    assert!(cx.debug_bounds("font-footer").is_some());
    assert_eq!(selection(&host, cx), (1, None));
}

#[gpui::test]
fn resetting_search_results_leaves_hidden_settings_unchanged(cx: &mut TestAppContext) {
    use std::{cell::Cell, rc::Rc};

    let (_, cx) = setup(cx);
    let visible = Rc::new(Cell::new(false));
    let hidden = Rc::new(Cell::new(true));
    let resettable_item = |keyword, dirty: &Rc<Cell<bool>>| {
        let read = dirty.clone();
        let reset = dirty.clone();
        item(keyword).on_reset(move |_| read.get(), move |_, _| reset.set(false))
    };
    let group = SettingGroup::new().items([
        resettable_item("theme", &visible),
        resettable_item("hidden", &hidden),
    ]);
    cx.update(|window, cx| {
        assert!(!group.is_resettable("theme", cx));
        visible.set(true);
        assert!(group.is_resettable("theme", cx));
        group.reset("theme", window, cx);
    });
    assert!(!visible.get());
    assert!(hidden.get());
}
