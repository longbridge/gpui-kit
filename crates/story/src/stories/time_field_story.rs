use chrono::NaiveTime;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _, Size, StyledExt,
    time_field::{HourCycle, TimeField, TimeFieldEvent, TimeFieldState, TimePrecision},
    v_flex,
};
use gpui_kit::*;

use crate::{ChangeStorySize, section, story_toolbar};

pub struct TimeFieldStory {
    minute: Entity<TimeFieldState>,
    second: Entity<TimeFieldState>,
    twelve_hour: Entity<TimeFieldState>,
    disabled: Entity<TimeFieldState>,
    invalid: Entity<TimeFieldState>,
    value: NaiveTime,
    size: Size,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for TimeFieldStory {
    fn title() -> &'static str {
        "TimeField"
    }

    fn description() -> &'static str {
        "Edit a time of day segment by segment, on a 24-hour or 12-hour clock."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl TimeFieldStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let value = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
        let field = |precision: TimePrecision,
                     hour_cycle: HourCycle,
                     window: &mut Window,
                     cx: &mut Context<Self>| {
            cx.new(|cx| {
                let mut state = TimeFieldState::new(window, cx)
                    .precision(precision)
                    .hour_cycle(hour_cycle);
                state.set_time(value, window, cx);
                state
            })
        };
        let minute = field(TimePrecision::Minute, HourCycle::H23, window, cx);
        let second = field(TimePrecision::Second, HourCycle::H23, window, cx);
        let twelve_hour = field(TimePrecision::Minute, HourCycle::H12, window, cx);
        let disabled = field(TimePrecision::Minute, HourCycle::H23, window, cx);
        let invalid = field(TimePrecision::Minute, HourCycle::H23, window, cx);

        let _subscriptions = vec![cx.subscribe(&minute, |this, _, event, cx| match event {
            TimeFieldEvent::Change(time) => {
                this.value = *time;
                cx.notify();
            }
        })];

        Self {
            minute,
            second,
            twelve_hour,
            disabled,
            invalid,
            value,
            size: Size::Medium,
            _subscriptions,
        }
    }
}

impl Focusable for TimeFieldStory {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.minute.focus_handle(cx)
    }
}

impl Render for TimeFieldStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .child(story_toolbar(self.size))
            .child(
                section("Default")
                    .description(
                        "Up/Down change the selected segment; digits type it and move to the next.",
                    )
                    .v_flex()
                    .gap_3()
                    .child(TimeField::new(&self.minute).with_size(self.size))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Value: {}", self.value)),
                    ),
            )
            .child(
                section("With seconds")
                    .description("Set the precision to edit seconds as well.")
                    .child(TimeField::new(&self.second).with_size(self.size)),
            )
            .child(
                section("12-hour clock")
                    .description("An AM/PM segment follows the time; type a or p to set it.")
                    .child(TimeField::new(&self.twelve_hour).with_size(self.size)),
            )
            .child(
                section("Disabled").child(
                    TimeField::new(&self.disabled)
                        .with_size(self.size)
                        .disabled(true),
                ),
            )
            .child(
                section("Invalid")
                    .description("Show a validation result from the owner.")
                    .child(
                        TimeField::new(&self.invalid)
                            .with_size(self.size)
                            .invalid(true),
                    ),
            )
    }
}
