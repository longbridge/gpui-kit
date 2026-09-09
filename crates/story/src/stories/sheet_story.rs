use std::{sync::Arc, time::Duration};

use fake::Fake;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, IndexPath, Placement, WindowExt,
    button::{Button, ButtonVariant, ButtonVariants as _},
    date_picker::{DatePicker, DatePickerState},
    h_flex,
    input::{Input, InputState},
    list::{List, ListDelegate, ListItem, ListState},
    v_flex,
};
use serde::Deserialize;

use crate::TestAction;
use crate::{Story, section, story_toolbar_group};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sheet_story, no_json)]
enum ToggleSheetOption {
    Overlay,
    OverlayClosable,
}

pub struct ListItemDeletegate {
    story: WeakEntity<SheetStory>,
    confirmed_index: Option<usize>,
    selected_index: Option<usize>,
    items: Vec<Arc<String>>,
    matches: Vec<Arc<String>>,
}

impl ListDelegate for ListItemDeletegate {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &App) -> usize {
        self.matches.len()
    }

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        let query = query.to_string();
        cx.spawn(async move |this, cx| {
            // Simulate a slow search.
            let sleep = (0.05..0.1).fake();
            cx.background_executor()
                .timer(Duration::from_secs_f64(sleep))
                .await;

            this.update(cx, |this, cx| {
                this.delegate_mut().matches = this
                    .delegate()
                    .items
                    .iter()
                    .filter(|item| item.to_lowercase().contains(&query.to_lowercase()))
                    .cloned()
                    .collect();
                cx.notify();
            })
            .ok();
        })
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let confirmed = Some(ix.row) == self.confirmed_index;

        if let Some(item) = self.matches.get(ix.row) {
            let list_item = ListItem::new(("item", ix.row))
                .check_icon(IconName::Check)
                .confirmed(confirmed)
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(item.to_string()),
                )
                .suffix(|_, _| {
                    Button::new("like")
                        .tab_stop(false)
                        .icon(IconName::Heart)
                        .with_variant(ButtonVariant::Ghost)
                        .size(px(18.))
                        .on_click(move |_, window, cx| {
                            cx.stop_propagation();
                            window.prevent_default();

                            println!("You have clicked like.");
                        })
                });
            Some(list_item)
        } else {
            None
        }
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                Icon::new(IconName::Inbox)
                    .size(px(50.))
                    .text_color(cx.theme().muted_foreground),
            )
            .child("No matches found")
            .items_center()
            .justify_center()
            .p_3()
            .bg(cx.theme().muted)
            .text_color(cx.theme().muted_foreground)
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        _ = self.story.update(cx, |this, cx| {
            this.close_sheet(window, cx);
        });
    }

    fn confirm(&mut self, _secondary: bool, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        _ = self.story.update(cx, |this, _| {
            self.confirmed_index = self.selected_index;
            if let Some(ix) = self.confirmed_index {
                if let Some(item) = self.matches.get(ix) {
                    this.selected_value = Some(SharedString::from(item.to_string()));
                }
            }
        });
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix.map(|ix| ix.row);

        if let Some(_) = ix {
            cx.notify();
        }
    }
}

pub struct SheetStory {
    focus_handle: FocusHandle,
    placement: Option<Placement>,
    selected_value: Option<SharedString>,
    list: Entity<ListState<ListItemDeletegate>>,
    input1: Entity<InputState>,
    input2: Entity<InputState>,
    date: Entity<DatePickerState>,
    overlay: bool,
    overlay_closable: bool,
}

impl Story for SheetStory {
    fn title() -> &'static str {
        "Sheet"
    }

    fn description() -> &'static str {
        "Sheet for open a popup in the edge of the window"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SheetStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let items: Vec<Arc<String>> = [
            "Baguette (France)",
            "Baklava (Turkey)",
            "Beef Wellington (UK)",
            "Biryani (India)",
            "Borscht (Ukraine)",
            "Bratwurst (Germany)",
            "Bulgogi (Korea)",
            "Burrito (USA)",
            "Ceviche (Peru)",
            "Chicken Tikka Masala (India)",
            "Churrasco (Brazil)",
            "Couscous (North Africa)",
            "Croissant (France)",
            "Dim Sum (China)",
            "Empanada (Argentina)",
            "Fajitas (Mexico)",
            "Falafel (Middle East)",
            "Feijoada (Brazil)",
            "Fish and Chips (UK)",
            "Fondue (Switzerland)",
            "Goulash (Hungary)",
            "Haggis (Scotland)",
            "Kebab (Middle East)",
            "Kimchi (Korea)",
            "Lasagna (Italy)",
            "Maple Syrup Pancakes (Canada)",
            "Moussaka (Greece)",
            "Pad Thai (Thailand)",
            "Paella (Spain)",
            "Pancakes (USA)",
            "Pasta Carbonara (Italy)",
            "Pavlova (Australia)",
            "Peking Duck (China)",
            "Pho (Vietnam)",
            "Pierogi (Poland)",
            "Pizza (Italy)",
            "Poutine (Canada)",
            "Pretzel (Germany)",
            "Ramen (Japan)",
            "Rendang (Indonesia)",
            "Sashimi (Japan)",
            "Satay (Indonesia)",
            "Shepherd's Pie (Ireland)",
            "Sushi (Japan)",
            "Tacos (Mexico)",
            "Tandoori Chicken (India)",
            "Tortilla (Spain)",
            "Tzatziki (Greece)",
            "Wiener Schnitzel (Austria)",
        ]
        .iter()
        .map(|s| Arc::new(s.to_string()))
        .collect();

        let story = cx.entity().downgrade();
        let delegate = ListItemDeletegate {
            story,
            selected_index: None,
            confirmed_index: None,
            items: items.clone(),
            matches: items.clone(),
        };
        let list = cx.new(|cx| ListState::new(delegate, window, cx).searchable(true));
        let input1 = cx.new(|cx| InputState::new(window, cx).placeholder("Your Name"));
        let input2 = cx.new(|cx| {
            InputState::new(window, cx).placeholder("For test focus back on dialog close.")
        });
        let date = cx.new(|cx| DatePickerState::new(window, cx));

        Self {
            focus_handle: cx.focus_handle(),
            placement: None,
            selected_value: None,
            list,
            input1,
            input2,
            date,
            overlay: true,
            overlay_closable: true,
        }
    }

    fn open_sheet_at(&mut self, placement: Placement, window: &mut Window, cx: &mut Context<Self>) {
        let list = self.list.clone();

        let drawer_h = match placement {
            Placement::Left | Placement::Right => px(400.),
            Placement::Top | Placement::Bottom => px(540.),
        };

        let overlay = self.overlay;
        let overlay_closable = self.overlay_closable;
        let input1 = self.input1.clone();
        let date = self.date.clone();
        window.open_sheet_at(placement, cx, move |this, _, cx| {
            this.overlay(overlay)
                .overlay_closable(overlay_closable)
                .size(drawer_h)
                .title("Sheet Title")
                .child(
                    v_flex()
                        .size_full()
                        .gap_3()
                        .child(Input::new(&input1))
                        .child(DatePicker::new(&date).placeholder("Date of Birth"))
                        .child(
                            Button::new("send-notification")
                                .child("Test Notification")
                                .on_click(|_, window, cx| {
                                    window
                                        .push_notification("Hello this is message from Sheet.", cx)
                                }),
                        )
                        .child(
                            Button::new("confirm-dialog-from-sheet")
                                .child("Open Confirm Dialog")
                                .on_click(|_, window, cx| {
                                    window.open_alert_dialog(cx, move |dialog, _, _| {
                                        dialog
                                            .child("Confirm dialog opened from sheet.")
                                            .on_ok(|_, window, cx| {
                                                window
                                                    .push_notification("You have pressed ok.", cx);
                                                true
                                            })
                                            .on_cancel(|_, window, cx| {
                                                window.push_notification(
                                                    "You have pressed cancel.",
                                                    cx,
                                                );
                                                true
                                            })
                                    });
                                }),
                        )
                        .child(
                            List::new(&list)
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded(cx.theme().radius),
                        ),
                )
                .footer(
                    h_flex()
                        .gap_6()
                        .items_center()
                        .child(Button::new("confirm").primary().label("Confirm").on_click(
                            |_, window, cx| {
                                window.close_sheet(cx);
                            },
                        ))
                        .child(
                            Button::new("cancel")
                                .label("Cancel")
                                .on_click(|_, window, cx| {
                                    window.close_sheet(cx);
                                }),
                        ),
                )
        });
    }

    fn close_sheet(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.placement = None;
        cx.notify();
    }

    fn on_action_test_action(
        &mut self,
        _: &TestAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.push_notification("You have clicked the TestAction.", cx);
    }
}

impl Focusable for SheetStory {
    fn focus_handle(&self, _cx: &gpui_kit::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SheetStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("sheet-story")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_action_test_action))
            .on_action(cx.listener(|this, action: &ToggleSheetOption, _, cx| {
                match action {
                    ToggleSheetOption::Overlay => this.overlay = !this.overlay,
                    ToggleSheetOption::OverlayClosable => {
                        this.overlay_closable = !this.overlay_closable
                    }
                }
                cx.notify();
            }))
            .size_full()
            .child(
                v_flex()
                    .gap_6()
                    .child(story_toolbar_group().dropdown_child(
                        Button::new("sheet-options").label("Options"),
                        {
                            let overlay = self.overlay;
                            let overlay_closable = self.overlay_closable;
                            move |menu, _, _| {
                                menu.menu_with_check(
                                    "Overlay",
                                    overlay,
                                    Box::new(ToggleSheetOption::Overlay),
                                )
                                .menu_with_check(
                                    "Close on overlay click",
                                    overlay_closable,
                                    Box::new(ToggleSheetOption::OverlayClosable),
                                )
                            }
                        },
                    ))
                    .child(
                        section("Placement")
                            .description("Open a sheet from any edge of the window.")
                            .child(
                                Button::new("show-sheet-left")
                                    .outline()
                                    .label("Left Sheet...")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_sheet_at(Placement::Left, window, cx)
                                    })),
                            )
                            .child(
                                Button::new("show-sheet-top")
                                    .outline()
                                    .label("Top Sheet...")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_sheet_at(Placement::Top, window, cx)
                                    })),
                            )
                            .child(
                                Button::new("show-sheet-right")
                                    .outline()
                                    .label("Right Sheet...")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_sheet_at(Placement::Right, window, cx)
                                    })),
                            )
                            .child(
                                Button::new("show-sheet-bottom")
                                    .outline()
                                    .label("Bottom Sheet...")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_sheet_at(Placement::Bottom, window, cx)
                                    })),
                            ),
                    )
                    .child(
                        section("Scrollable Sheet").w(px(480.)).child(
                            Button::new("show-scrollable-sheet")
                                .outline()
                                .label("Scrollable Sheet...")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.open_sheet_at(
                                        Placement::Right,
                                        cx,
                                        move |this, _, _| {
                                            this.title("Scrollable Sheet")
                                                .child("This is a scrollable sheet.\n".repeat(150))
                                        },
                                    );
                                })),
                        ),
                    )
                    .child(
                        section("Focus back test")
                            .w_128()
                            .child(Input::new(&self.input2))
                            .child(
                                Button::new("test-action")
                                    .outline()
                                    .label("Test Action")
                                    .flex_shrink_0()
                                    .on_click(|_, window, cx| {
                                        window.dispatch_action(Box::new(TestAction), cx);
                                    })
                                    .tooltip(
                                        "This button for test dispatch action, \
                                        to make sure when Dialog close,\
                                        \nthis still can handle the action.",
                                    ),
                            ),
                    )
                    .when_some(self.selected_value.clone(), |this, selected_value| {
                        this.child(
                            h_flex().gap_1().child("You have selected:").child(
                                div()
                                    .child(selected_value.to_string())
                                    .text_color(gpui_kit::red()),
                            ),
                        )
                    }),
            )
    }
}
