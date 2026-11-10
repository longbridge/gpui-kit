use gpui_kit::base::{LayoutDirection, LogicalStyledExt as _};
use gpui_kit::component::{
    ActiveTheme as _, IconName, StyledExt as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputState},
    slider::{Slider, SliderState},
    switch::Switch,
    tab::{Tab, TabBar},
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, Styled, Window, div, px,
};

use crate::section;

/// Arabic labels, inputs and controls together, to check right-to-left
/// layout. Flip the direction from the gallery's settings menu
/// ("Right to Left") or start with `GPUI_KIT_RTL=1`.
pub struct LayoutDirectionStory {
    focus_handle: FocusHandle,
    name: Entity<InputState>,
    search: Entity<InputState>,
    volume: Entity<SliderState>,
    active_tab: usize,
    notifications: bool,
    remember: bool,
}

impl super::Story for LayoutDirectionStory {
    fn title() -> &'static str {
        "Layout Direction"
    }

    fn description() -> &'static str {
        "Right-to-left layout follows one global direction for the whole app."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl LayoutDirectionStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            name: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("الاسم الكامل")
                    .default_value("مرحبا بالعالم")
            }),
            search: cx.new(|cx| InputState::new(window, cx).placeholder("بحث…")),
            volume: cx.new(|_| SliderState::new().min(0.).max(100.).default_value(30.)),
            active_tab: 0,
            notifications: true,
            remember: false,
        }
    }
}

impl Focusable for LayoutDirectionStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for LayoutDirectionStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rtl = LayoutDirection::global(cx).is_rtl();

        v_flex()
            .w_full()
            .gap_6()
            .child(
                section("Direction")
                    .description(
                        "One global setting; components never take a per-instance direction.",
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(div().text_color(cx.theme().muted_foreground).child(if rtl {
                                "Right to left"
                            } else {
                                "Left to right"
                            }))
                            .child(
                                Button::new("toggle-direction")
                                    .outline()
                                    .label(if rtl {
                                        "Switch to Left to Right"
                                    } else {
                                        "Switch to Right to Left"
                                    })
                                    .on_click(|_, _, cx| {
                                        if LayoutDirection::global(cx).is_rtl() {
                                            LayoutDirection::Ltr
                                        } else {
                                            LayoutDirection::Rtl
                                        }
                                        .set_global(cx);
                                    }),
                            ),
                    ),
            )
            .child(
                section("Arabic Form").w(px(560.)).items_stretch().child(
                    v_flex()
                        .w_full()
                        .gap_4()
                        .child(
                            TabBar::new("rtl-tabs")
                                .underline()
                                .selected_index(self.active_tab)
                                .on_click(cx.listener(|this, ix: &usize, _, cx| {
                                    this.active_tab = *ix;
                                    cx.notify();
                                }))
                                .child(Tab::new().label("الحساب"))
                                .child(Tab::new().label("الملف الشخصي"))
                                .child(Tab::new().label("الإعدادات")),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(div().font_medium().child("الاسم"))
                                .child(Input::new(&self.name)),
                        )
                        .child(Input::new(&self.search).prefix(IconName::Search))
                        .child(
                            h_flex()
                                .flex_row_dir(cx)
                                .w_full()
                                .justify_between()
                                .items_center()
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(div().font_medium().child("الإشعارات"))
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("تلقي التحديثات حول نشاط حسابك."),
                                        ),
                                )
                                .child(
                                    Switch::new("rtl-switch")
                                        .checked(self.notifications)
                                        .on_click(cx.listener(|this, checked, _, cx| {
                                            this.notifications = *checked;
                                            cx.notify();
                                        })),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(div().font_medium().child("مستوى الصوت"))
                                .child(Slider::new(&self.volume)),
                        )
                        .child(
                            Checkbox::new("rtl-remember")
                                .label("تذكرني")
                                .checked(self.remember)
                                .on_click(cx.listener(|this, checked, _, cx| {
                                    this.remember = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .child(Button::new("rtl-save").primary().label("حفظ"))
                                .child(Button::new("rtl-cancel").outline().label("إلغاء"))
                                .child(
                                    Button::new("rtl-next")
                                        .ghost()
                                        .label("التالي")
                                        .icon(IconName::ChevronRight),
                                ),
                        ),
                ),
            )
    }
}
