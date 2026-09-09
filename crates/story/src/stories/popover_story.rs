use gpui_kit::component::{
    ActiveTheme, StyledExt, WindowExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    list::{List, ListDelegate, ListItem, ListState},
    menu::{DropdownMenu as _, PopupMenu, PopupMenuItem},
    popover::Popover,
    separator::Separator,
    v_flex,
};
use gpui_kit::*;
use serde::Deserialize;
use std::time::Duration;

use crate::section;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = popover_story, no_json)]
struct Info(usize);

actions!(popover_story, [Copy, Paste, Cut, SearchAll, ToggleCheck]);
const CONTEXT: &str = "popover-story";
pub fn init(cx: &mut App) {
    cx.bind_keys([
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-v", Paste, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-v", Paste, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-x", Cut, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-x", Cut, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-f", SearchAll, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-f", SearchAll, Some(CONTEXT)),
    ])
}

struct Form {
    parent: WeakEntity<PopoverStory>,
    input1: Entity<InputState>,
}

impl Form {
    fn new(parent: WeakEntity<PopoverStory>, window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            parent: parent,
            input1: cx.new(|cx| InputState::new(window, cx)),
        })
    }
}

impl Focusable for Form {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input1.focus_handle(cx)
    }
}

struct DropdownListDelegate {
    parent: WeakEntity<PopoverStory>,
}

impl ListDelegate for DropdownListDelegate {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &App) -> usize {
        10
    }

    fn render_item(
        &mut self,
        ix: gpui_kit::component::IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        Some(ListItem::new(ix).child(format!("Item {}", ix.row)))
    }

    fn set_selected_index(
        &mut self,
        _: Option<gpui_kit::component::IndexPath>,
        _: &mut Window,
        _: &mut Context<gpui_kit::component::list::ListState<Self>>,
    ) {
    }

    fn confirm(&mut self, _: bool, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        let _ = self.parent.update(cx, |this, cx| {
            this.list_popover_open = false;
            cx.notify();
        });
    }

    fn cancel(&mut self, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        let _ = self.parent.update(cx, |this, cx| {
            this.list_popover_open = false;
            cx.notify();
        });
    }
}

impl EventEmitter<DismissEvent> for Form {}

impl Render for Form {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let parent = self.parent.clone();
        v_flex()
            .gap_2()
            .p_3()
            .size_full()
            .child("This is a form container.")
            .child("Click submit to dismiss the popover.")
            .child(Input::new(&self.input1))
            .child(
                Button::new("submit")
                    .label("Submit")
                    .primary()
                    .on_click(cx.listener(move |_, _, _, cx| {
                        let _ = parent.update(cx, |this, cx| {
                            this.form_popover_open = false;
                            cx.notify();
                        });
                    })),
            )
    }
}

pub struct PopoverStory {
    focus_handle: FocusHandle,
    form: Entity<Form>,
    list: Entity<ListState<DropdownListDelegate>>,
    form_popover_open: bool,
    list_popover_open: bool,
    checked: bool,
    message: String,
}

impl super::Story for PopoverStory {
    fn title() -> &'static str {
        "Popover"
    }

    fn description() -> &'static str {
        "Show focused content beside a trigger."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl PopoverStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let form = Form::new(cx.weak_entity(), window, cx);
        let parent = cx.weak_entity();
        let list = cx.new(|cx| {
            ListState::new(DropdownListDelegate { parent: parent }, window, cx).searchable(true)
        });

        cx.focus_self(window);

        Self {
            form,
            list,
            checked: true,
            form_popover_open: false,
            list_popover_open: false,
            focus_handle: cx.focus_handle(),
            message: "".to_string(),
        }
    }

    fn on_copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked copy".to_string();
        cx.notify()
    }

    fn on_cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked cut".to_string();
        cx.notify()
    }

    fn on_paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked paste".to_string();
        cx.notify()
    }

    fn on_search_all(&mut self, _: &SearchAll, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked search all".to_string();
        cx.notify()
    }

    fn on_action_info(&mut self, info: &Info, _: &mut Window, cx: &mut Context<Self>) {
        self.message = format!("You have clicked info: {}", info.0);
        cx.notify()
    }

    fn on_action_toggle_check(&mut self, _: &ToggleCheck, _: &mut Window, cx: &mut Context<Self>) {
        self.checked = !self.checked;
        self.message = format!("You have clicked toggle check: {}", self.checked);
        cx.notify()
    }
}

impl Focusable for PopoverStory {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PopoverStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.form.clone();

        v_flex()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_search_all))
            .on_action(cx.listener(Self::on_action_info))
            .on_action(cx.listener(Self::on_action_toggle_check))
            .size_full()
            .gap_6()
            .child(
                section("Default")
                    .description("Display lightweight contextual content.")
                    .child(
                        Popover::new("popover-0")
                            .max_w(px(600.))
                            .trigger(Button::new("btn").outline().label("Popover"))
                            .gap_2()
                            .text_sm()
                            .w(px(400.))
                            .child("Hello, this is a Popover.")
                            .child(Separator::horizontal())
                            .child(
                                "You can put any content here, including text,\
                            buttons, forms, and more.",
                            ),
                    )
                    .child(
                        Popover::new("default-open-popover")
                            .default_open(true)
                            .trigger(
                                Button::new("default-open-btn")
                                    .label("Default Open")
                                    .outline(),
                            )
                            .child("This popover is open by default when first rendered."),
                    ),
            )
            .child(
                section("Form")
                    .description("Keep focus and controlled open state around a form.")
                    .child(
                        Popover::new("popover-form")
                            .p_0()
                            .text_sm()
                            .trigger(Button::new("pop").outline().label("Popup Form"))
                            .track_focus(&form.focus_handle(cx))
                            .open(self.form_popover_open)
                            .on_open_change(cx.listener(move |this, open, _, cx| {
                                println!("Popover form open changed: {}", open);
                                this.form_popover_open = *open;
                                cx.notify();
                            }))
                            .child(form.clone()),
                    ),
            )
            .child(
                section("List")
                    .description("Place a scrollable selection list in the popover.")
                    .child(
                        Popover::new("popover-list")
                            .p_0()
                            .text_sm()
                            .open(self.list_popover_open)
                            .on_open_change(cx.listener(move |this, open, _, cx| {
                                this.list_popover_open = *open;
                                cx.notify();
                            }))
                            .trigger(Button::new("pop").outline().label("Popup List"))
                            .track_focus(&self.list.focus_handle(cx))
                            .child(List::new(&self.list))
                            .w_64()
                            .h(px(200.)),
                    ),
            )
            .child(
                section("Right click")
                    .description("Open from the secondary mouse button.")
                    .child(
                        Popover::new("popover-right-click")
                            .mouse_button(MouseButton::Right)
                            .trigger(Button::new("btn").outline().label("Right Click Popover"))
                            .max_w(px(600.))
                            .content(|_, _, cx| {
                                v_flex()
                                    .gap_2()
                                    .child("Hello, this is a Popover on the Bottom Right.")
                                    .child(Separator::horizontal())
                                    .child(
                                        Button::new("info1")
                                            .primary()
                                            .label("Dismiss")
                                            .w(px(80.))
                                            .on_click(cx.listener(|_, _, window, cx| {
                                                window.push_notification(
                                                    "You have clicked dismiss via DismissEvent.",
                                                    cx,
                                                );
                                                cx.emit(DismissEvent);
                                            })),
                                    )
                            }),
                    ),
            )
            .child(
                section("Custom style")
                    .description("Customize appearance, radius, and shadow.")
                    .child(
                        Popover::new("popover-1")
                            .trigger(Button::new("btn").outline().label("Style Popover"))
                            .appearance(false)
                            .py_1()
                            .px_2()
                            .bg(cx.theme().primary)
                            .text_color(cx.theme().primary_foreground)
                            .max_w(px(600.))
                            .rounded(cx.theme().radius.half())
                            .text_sm()
                            .shadow_2xl()
                            .child("A styled Popover with custom background and text color."),
                    ),
            )
            .child(
                section("Async submenu")
                    .description("Rebuild submenu content after asynchronous loading.")
                    .child(
                        Button::new("async-menu")
                            .outline()
                            .label("Async Menu")
                            .dropdown_menu(|menu, window, cx| {
                                // The submenu is attached as a plain menu value, its
                                // content is loaded asynchronously via `rebuild`.
                                let submenu = PopupMenu::build(window, cx, |menu, _, _| {
                                    menu.label("Loading...")
                                });

                                cx.spawn_in(window, {
                                    let submenu = submenu.clone();
                                    async move |_, cx| {
                                        cx.background_executor()
                                            .timer(Duration::from_secs(1))
                                            .await;
                                        _ = submenu.update_in(cx, |menu, window, cx| {
                                            menu.rebuild(window, cx, |menu, _, _| {
                                                (1..=3).fold(menu, |menu, ix| {
                                                    menu.menu(
                                                        format!("Loaded Item {}", ix),
                                                        Box::new(Info(ix)),
                                                    )
                                                })
                                            });
                                        });
                                    }
                                })
                                .detach();

                                menu.menu("Copy", Box::new(Copy))
                                    .separator()
                                    .item(PopupMenuItem::submenu("Async Submenu", submenu))
                            }),
                    )
                    .child(self.message.clone()),
            )
            .child(
                section("Anchor")
                    .description("Position content from each edge of the trigger.")
                    .w_full()
                    .min_h(px(360.))
                    .v_flex()
                    .child(
                        div().absolute().top_0().left_0().w_full().h_10().child(
                            h_flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    Popover::new("anchor-top-left")
                                        .max_w(px(600.))
                                        .anchor(Anchor::TopLeft)
                                        .trigger(Button::new("btn").outline().label("TopLeft"))
                                        .child("Anchored to the trigger's top-left."),
                                )
                                .child(
                                    Popover::new("anchor-top-center")
                                        .max_w(px(600.))
                                        .anchor(Anchor::TopCenter)
                                        .trigger(Button::new("btn").outline().label("TopCenter"))
                                        .child("Anchored to the trigger's top-center."),
                                )
                                .child(
                                    Popover::new("anchor-top-right")
                                        .anchor(Anchor::TopRight)
                                        .trigger(Button::new("btn").outline().label("TopRight"))
                                        .child("Anchored to the trigger's top-right."),
                                ),
                        ),
                    )
                    .child(
                        div().absolute().bottom_0().left_0().w_full().h_10().child(
                            h_flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    Popover::new("anchor-bottom-left")
                                        .trigger(Button::new("btn").outline().label("BottomLeft"))
                                        .anchor(Anchor::BottomLeft)
                                        .child("Anchored to the trigger's bottom-left."),
                                )
                                .child(
                                    Popover::new("anchor-bottom-center")
                                        .trigger(Button::new("btn").outline().label("BottomCenter"))
                                        .anchor(Anchor::BottomCenter)
                                        .child("Anchored to the trigger's bottom-center."),
                                )
                                .child(
                                    Popover::new("anchor-bottom-right")
                                        .anchor(Anchor::BottomRight)
                                        .trigger(Button::new("btn").outline().label("BottomRight"))
                                        .child("Anchored to the trigger's bottom-right."),
                                ),
                        ),
                    ),
            )
    }
}
