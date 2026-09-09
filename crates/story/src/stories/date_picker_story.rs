use chrono::{Datelike, Days, Duration, Utc};
use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, Size, StyledExt, calendar,
    date_picker::{DatePicker, DatePickerEvent, DatePickerState, DateRangePreset},
    v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, Focusable, InteractiveElement, IntoElement,
    ParentElement as _, Render, Styled as _, Subscription, Window, div, px,
};

use crate::{ChangeStorySize, section, story_toolbar};

pub struct DatePickerStory {
    date_picker: Entity<DatePickerState>,
    date_picker_small: Entity<DatePickerState>,
    date_picker_large: Entity<DatePickerState>,
    data_picker_custom: Entity<DatePickerState>,
    date_picker_value: Option<String>,
    date_range_picker: Entity<DatePickerState>,
    default_range_mode_picker: Entity<DatePickerState>,
    birthday_picker: Entity<DatePickerState>,
    without_appearance_picker: Entity<DatePickerState>,
    size: Size,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for DatePickerStory {
    fn title() -> &'static str {
        "DatePicker"
    }

    fn description() -> &'static str {
        "A date picker to select a date or date range."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl DatePickerStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let now = chrono::Local::now().naive_local().date();
        let date_picker = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx).disabled_matcher(vec![0, 6]);
            picker.set_date(now, window, cx);
            picker
        });
        let date_picker_large = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx)
                .date_format("%Y-%m-%d")
                .disabled_matcher(calendar::Matcher::range(
                    Some(now),
                    now.checked_add_days(Days::new(7)),
                ));
            picker.set_date(
                now.checked_sub_days(Days::new(1)).unwrap_or_default(),
                window,
                cx,
            );
            picker
        });
        let date_picker_small = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx).disabled_matcher(
                calendar::Matcher::interval(Some(now), now.checked_add_days(Days::new(5))),
            );
            picker.set_date(now, window, cx);
            picker
        });
        let data_picker_custom = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx)
                .disabled_matcher(calendar::Matcher::custom(|date| date.day0() < 5));
            picker.set_date(now, window, cx);
            picker
        });
        let date_range_picker = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx);
            picker.set_date(
                (now, now.checked_add_days(Days::new(4)).unwrap()),
                window,
                cx,
            );
            picker
        });

        let default_range_mode_picker = cx.new(|cx| DatePickerState::range(window, cx));

        let birthday_picker = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx);
            picker.set_year_range((1927, now.year() + 1), cx);
            picker
        });

        let without_appearance_picker = cx.new(|cx| DatePickerState::new(window, cx));

        let _subscriptions = vec![
            cx.subscribe(&date_picker, |this, _, ev, _| match ev {
                DatePickerEvent::Change(date) => {
                    this.date_picker_value = date.format("%Y-%m-%d").map(|s| s.to_string());
                }
            }),
            cx.subscribe(&date_range_picker, |this, _, ev, _| match ev {
                DatePickerEvent::Change(date) => {
                    this.date_picker_value = date.format("%Y-%m-%d").map(|s| s.to_string());
                }
            }),
            cx.subscribe(&default_range_mode_picker, |this, _, ev, _| match ev {
                DatePickerEvent::Change(date) => {
                    this.date_picker_value = date.format("%Y-%m-%d").map(|s| s.to_string());
                }
            }),
        ];

        Self {
            date_picker,
            date_picker_large,
            date_picker_small,
            data_picker_custom,
            date_range_picker,
            default_range_mode_picker,
            birthday_picker,
            without_appearance_picker,
            size: Size::Medium,
            date_picker_value: None,
            _subscriptions,
        }
    }
}

impl Focusable for DatePickerStory {
    fn focus_handle(&self, cx: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.date_picker.focus_handle(cx)
    }
}

impl Render for DatePickerStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let presets = vec![
            DateRangePreset::single(
                "Yesterday",
                (Utc::now() - Duration::days(1)).naive_local().date(),
            ),
            DateRangePreset::single(
                "Last Week",
                (Utc::now() - Duration::weeks(1)).naive_local().date(),
            ),
            DateRangePreset::single(
                "Last Month",
                (Utc::now() - Duration::days(30)).naive_local().date(),
            ),
        ];
        let range_presets = vec![
            DateRangePreset::range(
                "Last 7 Days",
                (Utc::now() - Duration::days(7)).naive_local().date(),
                Utc::now().naive_local().date(),
            ),
            DateRangePreset::range(
                "Last 14 Days",
                (Utc::now() - Duration::days(14)).naive_local().date(),
                Utc::now().naive_local().date(),
            ),
            DateRangePreset::range(
                "Last 30 Days",
                (Utc::now() - Duration::days(30)).naive_local().date(),
                Utc::now().naive_local().date(),
            ),
            DateRangePreset::range(
                "Last 90 Days",
                (Utc::now() - Duration::days(90)).naive_local().date(),
                Utc::now().naive_local().date(),
            ),
        ];

        v_flex()
            .gap_3()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .child(story_toolbar(self.size))
            .child(
                section("Default")
                    .description("Single-date selection with presets and clear action.")
                    .w_128()
                    .v_flex()
                    .gap_3()
                    .child(
                        DatePicker::new(&self.date_picker)
                            .with_size(self.size)
                            .w(px(280.))
                            .cleanable(true)
                            .presets(presets),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Value: {:?}", self.date_picker_value)),
                    ),
            )
            .child(
                section("Disabled dates")
                    .description("Matchers can block intervals, ranges, or custom dates.")
                    .w_128()
                    .v_flex()
                    .gap_3()
                    .child(
                        DatePicker::new(&self.date_picker_small)
                            .with_size(self.size)
                            .w(px(280.)),
                    )
                    .child(
                        DatePicker::new(&self.date_picker_large)
                            .with_size(self.size)
                            .w(px(280.)),
                    )
                    .child(
                        DatePicker::new(&self.data_picker_custom)
                            .with_size(self.size)
                            .w(px(280.)),
                    ),
            )
            .child(
                section("Date range")
                    .description("Two months with range presets.")
                    .w_128()
                    .child(
                        DatePicker::new(&self.date_range_picker)
                            .with_size(self.size)
                            .w(px(280.))
                            .number_of_months(2)
                            .cleanable(true)
                            .presets(range_presets.clone()),
                    ),
            )
            .child(
                section("Empty range")
                    .description("Empty range with presets.")
                    .w_128()
                    .child(
                        DatePicker::new(&self.default_range_mode_picker)
                            .with_size(self.size)
                            .w(px(280.))
                            .placeholder("Range mode picker")
                            .cleanable(true)
                            .presets(range_presets.clone()),
                    ),
            )
            .child(
                section("Year range")
                    .description("Custom year range.")
                    .w_128()
                    .child(
                        DatePicker::new(&self.birthday_picker)
                            .with_size(self.size)
                            .w(px(280.))
                            .number_of_months(1)
                            .cleanable(true)
                            .placeholder("Select birthday"),
                    ),
            )
            .child(
                section("Custom style")
                    .description("Appearance-free input.")
                    .w_128()
                    .child(
                        div().w(px(280.)).bg(cx.theme().secondary).child(
                            DatePicker::new(&self.without_appearance_picker)
                                .with_size(self.size)
                                .w(px(280.))
                                .appearance(false)
                                .placeholder("Without appearance"),
                        ),
                    ),
            )
    }
}
