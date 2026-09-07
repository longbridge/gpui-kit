use std::{rc::Rc, time::Duration};

use fake::Fake;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use gpui_kit::component::{
    ActiveTheme, Icon, IconName, IndexPath, Selectable, ThemeStyled as _,
    button::Button,
    h_flex,
    label::Label,
    list::{List, ListDelegate, ListEvent, ListItem, ListState},
    v_flex,
};

use crate::story_toolbar_group;

actions!(
    list_story,
    [
        SelectedCompany,
        ToggleSelectable,
        ToggleSearchable,
        ToggleLoading,
        ToggleLazyLoad,
        ToggleDraggable,
        GoToTop,
        GoToSelected,
        GoToRow,
        GoToBottom
    ]
);

#[derive(Clone, Default)]
struct Company {
    name: SharedString,
    industry: SharedString,
    last_done: f64,
    prev_close: f64,

    change_percent: f64,
    change_percent_str: SharedString,
    last_done_str: SharedString,
    prev_close_str: SharedString,
    // description: String,
}

impl Company {
    fn prepare(mut self) -> Self {
        self.change_percent = (self.last_done - self.prev_close) / self.prev_close;
        self.change_percent_str = format!("{:.2}%", self.change_percent).into();
        self.last_done_str = format!("{:.2}", self.last_done).into();
        self.prev_close_str = format!("{:.2}", self.prev_close).into();
        self
    }
}

/// Where the dragged item will be inserted relative to the drop target row.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DropPosition {
    Before,
    After,
}

impl DropPosition {
    /// The insertion gap index (0..=len) this position resolves to for `row`.
    fn gap(self, row: usize) -> usize {
        match self {
            DropPosition::Before => row,
            DropPosition::After => row + 1,
        }
    }
}

#[derive(Clone)]
struct DragCompany {
    ix: IndexPath,
    name: SharedString,
}

impl Render for DragCompany {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .text_sm()
            .bg(cx.theme().accent)
            .text_color(cx.theme().accent_foreground)
            .rounded(cx.theme().radius)
            .shadow_md()
            .child(self.name.clone())
    }
}

#[derive(IntoElement)]
struct CompanyListItem {
    base: ListItem,
    company: Rc<Company>,
    selected: bool,
    drop_position: Option<DropPosition>,
}

impl CompanyListItem {
    pub fn new(id: impl Into<ElementId>, company: Rc<Company>, selected: bool) -> Self {
        CompanyListItem {
            company,
            base: ListItem::new(id).selected(selected),
            selected,
            drop_position: None,
        }
    }

    /// Make this item draggable for reordering, by using the
    /// `InteractiveElement` methods on the inner [`ListItem`].
    pub fn draggable(
        mut self,
        ix: IndexPath,
        drop_position: Option<DropPosition>,
        on_drag_start: impl Fn(&mut App) + 'static,
        on_drag_move: impl Fn(&DragMoveEvent<DragCompany>, &mut Window, &mut App) + 'static,
        on_drop: impl Fn(&DragCompany, &mut Window, &mut App) + 'static,
    ) -> Self {
        let drag = DragCompany {
            ix,
            name: self.company.name.clone(),
        };

        self.drop_position = drop_position;
        self.base = self
            .base
            .on_drag(drag, move |drag, _, _, cx| {
                on_drag_start(cx);
                cx.new(|_| drag.clone())
            })
            .on_drag_move(on_drag_move)
            .on_drop(on_drop);
        self
    }
}

impl Selectable for CompanyListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for CompanyListItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_color = if self.selected {
            cx.theme().accent_foreground
        } else {
            cx.theme().foreground
        };

        let trend_color = match self.company.change_percent {
            change if change > 0.0 => cx.theme().green,
            change if change < 0.0 => cx.theme().red,
            _ => cx.theme().foreground,
        };

        self.base
            .px_2()
            .py_1()
            .overflow_x_hidden()
            .border_1()
            .rounded(cx.theme().radius)
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .text_color(text_color)
                    .when_some(
                        self.drop_position.filter(|_| cx.has_active_drag()),
                        |this, position| {
                            let line = div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .h(px(2.))
                                .rounded_full_style(cx)
                                .bg(cx.theme().blue);

                            this.child(match position {
                                DropPosition::Before => line.top(px(-5.)),
                                DropPosition::After => line.bottom(px(-5.)),
                            })
                        },
                    )
                    .child(
                        h_flex().gap_2().child(
                            v_flex()
                                .gap_1()
                                .max_w(px(500.))
                                .overflow_x_hidden()
                                .flex_nowrap()
                                .child(Label::new(self.company.name.clone()).whitespace_nowrap()),
                        ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .justify_end()
                            .child(
                                div()
                                    .w(px(65.))
                                    .text_color(text_color)
                                    .child(self.company.last_done_str.clone()),
                            )
                            .child(
                                h_flex().w(px(65.)).justify_end().child(
                                    div()
                                        .rounded(cx.theme().radius)
                                        .whitespace_nowrap()
                                        .text_size(px(12.))
                                        .px_1()
                                        .text_color(trend_color)
                                        .child(self.company.change_percent_str.clone()),
                                ),
                            ),
                    ),
            )
    }
}

struct CompanyListDelegate {
    industries: Vec<SharedString>,
    _companies: Vec<Rc<Company>>,
    matched_companies: Vec<Vec<Rc<Company>>>,
    selected_index: Option<IndexPath>,
    confirmed_index: Option<IndexPath>,
    query: SharedString,
    loading: bool,
    eof: bool,
    lazy_load: bool,
    draggable: bool,
    drop_target: Option<(IndexPath, DropPosition)>,
}

impl CompanyListDelegate {
    fn prepare(&mut self, query: impl Into<SharedString>) {
        self.query = query.into();
        for companies in &mut self.matched_companies {
            companies.clear();
        }
        let companies: Vec<Rc<Company>> = self
            ._companies
            .iter()
            .filter(|company| {
                company
                    .name
                    .to_lowercase()
                    .contains(&self.query.to_lowercase())
            })
            .cloned()
            .collect();
        for company in companies.into_iter() {
            if let Some(ix) = self.industries.iter().position(|s| s == &company.industry) {
                self.matched_companies[ix].push(company);
            } else {
                self.industries.push(company.industry.clone());
                self.matched_companies.push(vec![company]);
            }
        }
    }

    fn extend_more(&mut self, len: usize) {
        self._companies
            .extend((0..len).map(|_| Rc::new(random_company())));
        self.prepare(self.query.clone());
    }

    fn selected_company(&self) -> Option<Rc<Company>> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.matched_companies
            .get(ix.section)
            .and_then(|c| c.get(ix.row))
            .cloned()
    }

    /// Record the pending drop target row, returns true if it changed.
    fn update_drop_target(&mut self, ix: IndexPath, position: Option<DropPosition>) -> bool {
        match position {
            Some(position) => {
                if self.drop_target != Some((ix, position)) {
                    self.drop_target = Some((ix, position));
                    true
                } else {
                    false
                }
            }
            None => {
                if self.drop_target.is_some_and(|(target, _)| target == ix) {
                    self.drop_target = None;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Move the company at `from` to before or after the company at `to`.
    fn move_company(&mut self, from: IndexPath, to: IndexPath, position: DropPosition) {
        self.drop_target = None;
        if from == to {
            return;
        }

        let Some(company) = self
            .matched_companies
            .get_mut(from.section)
            .filter(|companies| from.row < companies.len())
            .map(|companies| companies.remove(from.row))
        else {
            return;
        };

        let mut row = position.gap(to.row);
        if from.section == to.section && from.row < row {
            row -= 1;
        }

        if let Some(companies) = self.matched_companies.get_mut(to.section) {
            let row = row.min(companies.len());
            companies.insert(row, company);
            self.selected_index = Some(IndexPath::new(row).section(to.section));
        }
    }
}

impl ListDelegate for CompanyListDelegate {
    type Item = CompanyListItem;

    fn sections_count(&self, _: &App) -> usize {
        self.industries.len()
    }

    fn items_count(&self, section: usize, _: &App) -> usize {
        if matches!(section, 0 | 2 | 3) {
            // Return some empty sections for testing.
            return 0;
        }

        self.matched_companies[section].len()
    }

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.prepare(query.to_owned());
        Task::ready(())
    }

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        println!("Confirmed with secondary: {}", secondary);
        window.dispatch_action(Box::new(SelectedCompany), cx);
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn set_right_clicked_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
        println!("right_clicked_index: {:?}", ix);
    }

    fn render_section_header(
        &mut self,
        section: usize,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        let Some(industry) = self.industries.get(section) else {
            return None;
        };

        Some(
            h_flex()
                .pb_1()
                .px_2()
                .gap_2()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(Icon::new(IconName::Folder))
                .child(industry.clone())
                .child(format!("(section: {})", section)),
        )
    }

    fn render_section_footer(
        &mut self,
        section: usize,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        let Some(_) = self.industries.get(section) else {
            return None;
        };

        Some(
            div()
                .pt_1()
                .pb_5()
                .px_2()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!(
                    "Total {} items in section.",
                    self.matched_companies[section].len()
                )),
        )
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(company) = self.matched_companies[ix.section].get(ix.row) {
            let item =
                CompanyListItem::new(ix, company.clone(), selected).when(self.draggable, |this| {
                    // Normalize the pending drop target to an insertion gap, so the
                    // indicator renders at the same edge no matter which of the two
                    // rows around the gap the cursor is over.
                    let count = self.matched_companies[ix.section].len();
                    let drop_position = self.drop_target.and_then(|(target, position)| {
                        if target.section != ix.section {
                            return None;
                        }

                        let gap = position.gap(target.row);
                        if gap == ix.row {
                            Some(DropPosition::Before)
                        } else if gap == count && ix.row + 1 == count {
                            Some(DropPosition::After)
                        } else {
                            None
                        }
                    });

                    let state = cx.entity().downgrade();
                    this.draggable(
                        ix,
                        drop_position,
                        // A drag may end without a drop (e.g. released outside the
                        // list), clear the stale drop target when a new one starts.
                        move |cx| {
                            _ = state.update(cx, |this, cx| {
                                if this.delegate_mut().drop_target.take().is_some() {
                                    cx.notify();
                                }
                            });
                        },
                        cx.listener(move |this, e: &DragMoveEvent<DragCompany>, _, cx| {
                            let bounds = e.bounds;
                            let from = e.drag(cx).ix;
                            let position = if bounds.contains(&e.event.position) {
                                let position = if e.event.position.y < bounds.center().y {
                                    DropPosition::Before
                                } else {
                                    DropPosition::After
                                };

                                // Gaps adjacent to the dragged row are no-op moves.
                                let gap = position.gap(ix.row);
                                if from.section == ix.section
                                    && (gap == from.row || gap == from.row + 1)
                                {
                                    None
                                } else {
                                    Some(position)
                                }
                            } else {
                                None
                            };

                            if this.delegate_mut().update_drop_target(ix, position) {
                                cx.notify();
                            }
                        }),
                        cx.listener(move |this, drag: &DragCompany, _, cx| {
                            let Some(position) = this
                                .delegate()
                                .drop_target
                                .filter(|(target, _)| *target == ix)
                                .map(|(_, position)| position)
                            else {
                                return;
                            };

                            this.delegate_mut().move_company(drag.ix, ix, position);
                            cx.notify();
                        }),
                    )
                });

            return Some(item);
        }

        None
    }

    fn loading(&self, _: &App) -> bool {
        self.loading
    }

    fn has_more(&self, _: &App) -> bool {
        if self.loading {
            return false;
        }

        return !self.eof;
    }

    fn load_more_threshold(&self) -> usize {
        150
    }

    fn load_more(&mut self, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        if !self.lazy_load {
            return;
        }

        cx.spawn_in(window, async move |view, window| {
            // Simulate network request, delay 1s to load data.
            window
                .background_executor()
                .timer(Duration::from_secs(1))
                .await;

            _ = view.update_in(window, move |view, window, cx| {
                let query = view.delegate().query.clone();
                view.delegate_mut().extend_more(200);
                _ = view.delegate_mut().perform_search(&query, window, cx);
                view.delegate_mut().eof = view.delegate()._companies.len() >= 6000;
            });
        })
        .detach();
    }
}

pub struct ListStory {
    focus_handle: FocusHandle,
    company_list: Entity<ListState<CompanyListDelegate>>,
    selected_company: Option<Rc<Company>>,
    selectable: bool,
    searchable: bool,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for ListStory {
    fn title() -> &'static str {
        "List"
    }

    fn description() -> &'static str {
        "A list displays a series of items."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl ListStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut delegate = CompanyListDelegate {
            industries: vec![],
            matched_companies: vec![],
            _companies: vec![],
            selected_index: Some(IndexPath::default()),
            confirmed_index: None,
            query: "".into(),
            loading: false,
            eof: false,
            lazy_load: false,
            draggable: false,
            drop_target: None,
        };
        delegate.extend_more(100);

        let company_list = cx.new(|cx| ListState::new(delegate, window, cx).searchable(true));

        let _subscriptions =
            vec![
                cx.subscribe(&company_list, |_, _, ev: &ListEvent, _| match ev {
                    ListEvent::Select(ix) => {
                        println!("List Selected: {:?}", ix);
                    }
                    ListEvent::Confirm(ix) => {
                        println!("List Confirmed: {:?}", ix);
                    }
                    ListEvent::Cancel => {
                        println!("List Cancelled");
                    }
                }),
            ];

        // Spawn a background to random refresh the list
        cx.spawn(async move |this, cx| {
            this.update(cx, |this, cx| {
                this.company_list.update(cx, |picker, _| {
                    picker
                        .delegate_mut()
                        ._companies
                        .iter_mut()
                        .for_each(|company| {
                            let mut new_company = random_company();
                            new_company.name = company.name.clone();
                            new_company.industry = company.industry.clone();
                            *company = Rc::new(new_company);
                        });
                    picker.delegate_mut().prepare("");
                });
                cx.notify();
            })
            .ok();
        })
        .detach();

        Self {
            focus_handle: cx.focus_handle(),
            searchable: true,
            selectable: true,
            company_list,
            selected_company: None,
            _subscriptions,
        }
    }

    fn selected_company(&mut self, _: &SelectedCompany, _: &mut Window, cx: &mut Context<Self>) {
        let picker = self.company_list.read(cx);
        if let Some(company) = picker.delegate().selected_company() {
            self.selected_company = Some(company);
        }
    }

    fn toggle_selectable(&mut self, selectable: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.selectable = selectable;
        self.company_list.update(cx, |list, cx| {
            list.set_selectable(self.selectable, cx);
        })
    }

    fn toggle_searchable(&mut self, searchable: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.searchable = searchable;
        self.company_list.update(cx, |list, cx| {
            list.set_searchable(self.searchable, cx);
        })
    }
}

fn random_company() -> Company {
    let last_done = (0.0..999.0).fake::<f64>();
    let prev_close = last_done * (-0.1..0.1).fake::<f64>();

    Company {
        name: fake::faker::company::en::CompanyName()
            .fake::<String>()
            .into(),
        industry: fake::faker::company::en::Industry().fake::<String>().into(),
        last_done,
        prev_close,
        ..Default::default()
    }
    .prepare()
}

impl Focusable for ListStory {
    fn focus_handle(&self, _cx: &gpui_kit::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ListStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let lazy_load = self.company_list.read(cx).delegate().lazy_load;
        let draggable = self.company_list.read(cx).delegate().draggable;

        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::selected_company))
            .on_action(cx.listener(|this, _: &ToggleSelectable, window, cx| {
                this.toggle_selectable(!this.selectable, window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleSearchable, window, cx| {
                this.toggle_searchable(!this.searchable, window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleLoading, _, cx| {
                this.company_list.update(cx, |list, cx| {
                    list.delegate_mut().loading = !list.delegate().loading;
                    cx.notify();
                });
            }))
            .on_action(cx.listener(|this, _: &ToggleLazyLoad, _, cx| {
                this.company_list.update(cx, |list, cx| {
                    list.delegate_mut().lazy_load = !list.delegate().lazy_load;
                    cx.notify();
                });
            }))
            .on_action(cx.listener(|this, _: &ToggleDraggable, _, cx| {
                this.company_list.update(cx, |list, cx| {
                    list.delegate_mut().draggable = !list.delegate().draggable;
                    cx.notify();
                });
            }))
            .on_action(cx.listener(|this, action: &GoToTop, window, cx| {
                let _ = action;
                this.company_list.update(cx, |list, cx| {
                    list.scroll_to_item(IndexPath::default(), ScrollStrategy::Top, window, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &GoToSelected, window, cx| {
                this.company_list.update(cx, |list, cx| {
                    list.scroll_to_selected_item(window, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &GoToRow, window, cx| {
                this.company_list.update(cx, |list, cx| {
                    list.scroll_to_item(
                        IndexPath::new(1).section(5),
                        ScrollStrategy::Center,
                        window,
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &GoToBottom, window, cx| {
                this.company_list.update(cx, |list, cx| {
                    let last_section = list.delegate().sections_count(cx).saturating_sub(1);
                    let last_row = list
                        .delegate()
                        .items_count(last_section, cx)
                        .saturating_sub(1);
                    list.scroll_to_item(
                        IndexPath::default().section(last_section).row(last_row),
                        ScrollStrategy::Top,
                        window,
                        cx,
                    );
                });
            }))
            .size_full()
            .gap_4()
            .child(
                story_toolbar_group()
                    .dropdown_child(Button::new("go-to").label("Go To"), |menu, _, _| {
                        menu.menu("Top", Box::new(GoToTop))
                            .menu("Selected", Box::new(GoToSelected))
                            .menu("Section 5, Row 1", Box::new(GoToRow))
                            .menu("Bottom", Box::new(GoToBottom))
                    })
                    .dropdown_child(Button::new("list-options").label("Options"), {
                        let selectable = self.selectable;
                        let searchable = self.searchable;
                        let loading = self.company_list.read(cx).delegate().loading;
                        move |menu, _, _| {
                            menu.menu_with_check(
                                "Selectable",
                                selectable,
                                Box::new(ToggleSelectable),
                            )
                            .menu_with_check("Searchable", searchable, Box::new(ToggleSearchable))
                            .menu_with_check("Loading", loading, Box::new(ToggleLoading))
                            .menu_with_check("Lazy Load", lazy_load, Box::new(ToggleLazyLoad))
                            .menu_with_check(
                                "Draggable",
                                draggable,
                                Box::new(ToggleDraggable),
                            )
                        }
                    }),
            )
            .child(
                List::new(&self.company_list)
                    .p(px(8.))
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;

    #[test]
    fn searching_and_refreshing_replace_company_groups() {
        let companies = [
            ("Acme", "Software"),
            ("Books", "Retail"),
            ("Atlas", "Software"),
        ]
        .into_iter()
        .map(|(name, industry)| {
            Rc::new(Company {
                name: name.into(),
                industry: industry.into(),
                ..Company::default()
            })
        })
        .collect();
        let mut delegate = CompanyListDelegate {
            industries: vec![],
            matched_companies: vec![],
            _companies: companies,
            selected_index: None,
            confirmed_index: None,
            query: "".into(),
            loading: false,
            eof: false,
            lazy_load: false,
            draggable: false,
            drop_target: None,
        };

        for (query, expected) in [
            ("", vec![vec!["Acme", "Atlas"], vec!["Books"]]),
            ("ACME", vec![vec!["Acme"], vec![]]),
            ("missing", vec![vec![], vec![]]),
            ("", vec![vec!["Acme", "Atlas"], vec!["Books"]]),
        ] {
            delegate.prepare(query);
            assert_eq!(delegate.industries, vec!["Software", "Retail"]);
            assert_eq!(delegate.matched_companies.len(), delegate.industries.len());
            let names: Vec<Vec<&str>> = delegate
                .matched_companies
                .iter()
                .map(|companies| {
                    companies
                        .iter()
                        .map(|company| company.name.as_ref())
                        .collect()
                })
                .collect();
            assert_eq!(names, expected);
        }
        // Loading more data also rebuilds the current query; it must not duplicate old rows.
        delegate.extend_more(0);
        assert_eq!(
            delegate
                .matched_companies
                .iter()
                .map(Vec::len)
                .sum::<usize>(),
            3
        );
    }
}
