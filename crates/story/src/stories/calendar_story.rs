use gpui_kit::component::{
    calendar::{Calendar, CalendarState},
    v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Window,
};

use crate::section;

pub struct CalendarStory {
    focus_handle: FocusHandle,
    calendar: Entity<CalendarState>,
    calendar_wide: Entity<CalendarState>,
    calendar_with_disabled_matcher: Entity<CalendarState>,
}

impl super::Story for CalendarStory {
    fn title() -> &'static str {
        "Calendar"
    }

    fn description() -> &'static str {
        "A calendar to select a date or date range."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl CalendarStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let calendar = cx.new(|cx| CalendarState::new(window, cx));
        let calendar_wide = cx.new(|cx| CalendarState::new(window, cx));
        let calendar_with_disabled_matcher =
            cx.new(|cx| CalendarState::new(window, cx).disabled_matcher(vec![0, 3, 6]));

        Self {
            calendar,
            calendar_wide,
            calendar_with_disabled_matcher,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for CalendarStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CalendarStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                section("Single month")
                    .description("Single-date selection.")
                    .w_128()
                    .child(Calendar::new(&self.calendar)),
            )
            .child(
                section("Multiple months")
                    .description("Three months shown together.")
                    .w_128()
                    .child(Calendar::new(&self.calendar_wide).number_of_months(3)),
            )
            .child(
                section("Disabled dates")
                    .description("Recurring unavailable weekdays.")
                    .w_128()
                    .child(Calendar::new(&self.calendar_with_disabled_matcher)),
            )
    }
}
