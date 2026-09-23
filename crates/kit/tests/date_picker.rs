mod common;
use gpui_kit::component::{
    Disableable,
    date_picker::{DatePicker, DatePickerEvent, DatePickerState, DateRangePreset, DateTime},
    time_field::TimePrecision,
};
use gpui_kit::test::{TestAppContextExt, TestWindowExt};
use gpui_kit::{
    AppContext, Context, ElementId, Entity, TestAppContext, Window, div, prelude::*, px, size,
};
use std::time::Duration;
struct Schedule {
    date: Entity<DatePickerState>,
    disabled: bool,
}
impl Render for Schedule {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p_4().child(
            DatePicker::new(&self.date)
                .cleanable(true)
                .disabled(self.disabled)
                .presets(vec![DateRangePreset::single(
                    "Release day",
                    "2026-09-15".parse().unwrap(),
                )]),
        )
    }
}
#[gpui_kit::test]
async fn date_picker_opens_selects_preset_clears_and_cancels(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut id: Option<ElementId> = None;
    let (handle, _) = common::open_window(cx, Some(size(px(640.), px(600.))), |window, cx| {
        cx.new(|cx| {
            let date = cx.new(|cx| DatePickerState::new(window, cx));
            id = Some(("date-picker", date.entity_id()).into());
            Schedule {
                date,
                disabled: false,
            }
        })
    });
    let id = id.unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find(id.clone()).expanded(), Some(false));
        window.click(id.clone(), cx);
        assert_eq!(window.find(id.clone()).expanded(), Some(true));
        let preset = window.find(("preset", 0usize)).bounds();
        assert!(preset.top() >= window.find(id.clone()).bounds().bottom());
        window.click(("preset", 0usize), cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.find(id.clone()).expanded() == Some(false)
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(window.find(id.clone()).value(), Some("2026/09/15"));
        window.click(id.clone(), cx);
        window.click("calendar-next", cx);
        assert!(window.try_find("calendar-2026-09-16-0-2").is_none());
        window.click("calendar-prev", cx);
        assert_eq!(
            window.find("calendar-2026-09-16-0-2").label(),
            Some("2026-09-16")
        );
        window.click("calendar-2026-09-16-0-2", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find(id.clone()).value(), Some("2026/09/16"));
        // The clear action exists only when a date is actually selected.
        assert!(window.find("clean").visible());
        window.click("clean", cx);
        assert!(window.try_find("clean").is_none());
        assert_eq!(window.find(id.clone()).value(), None);
        window.click(id.clone(), cx);
        window.press("escape", cx);
        assert_eq!(window.find(id.clone()).expanded(), Some(false));
    })
    .unwrap();
}
#[gpui_kit::test]
fn disabled_date_picker_does_not_open(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut id: Option<ElementId> = None;
    let (handle, _) = common::open_window(cx, Some(size(px(640.), px(600.))), |window, cx| {
        cx.new(|cx| {
            let date = cx.new(|cx| DatePickerState::new(window, cx));
            id = Some(("date-picker", date.entity_id()).into());
            Schedule {
                date,
                disabled: true,
            }
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click(id.clone().unwrap(), cx);
        assert_eq!(window.find(id.clone().unwrap()).expanded(), Some(false));
        assert!(window.try_find(("preset", 0usize)).is_none());
    })
    .unwrap();
}

struct TimedSchedule {
    date: Entity<DatePickerState>,
    changes: Vec<DateTime>,
    _subscription: gpui_kit::Subscription,
}
impl TimedSchedule {
    fn new(date: Entity<DatePickerState>, cx: &mut Context<Self>) -> Self {
        let _subscription = cx.subscribe(&date, |this, _, event, _| match event {
            DatePickerEvent::Change(value) => this.changes.push(*value),
        });
        Self {
            date,
            changes: vec![],
            _subscription,
        }
    }
}
impl Render for TimedSchedule {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .child(DatePicker::new(&self.date).number_of_months(2))
    }
}

fn at(value: &str) -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").unwrap()
}

#[gpui_kit::test]
fn date_time_picker_reports_each_edit_and_stays_open(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut id: Option<ElementId> = None;
    let (handle, view) = common::open_window(cx, Some(size(px(800.), px(600.))), |window, cx| {
        cx.new(|cx| {
            let date = cx.new(|cx| {
                let mut state =
                    DatePickerState::new(window, cx).time_precision(TimePrecision::Second);
                state.set_date_time(at("2026-09-15 08:00:00"), window, cx);
                state
            });
            id = Some(("date-picker", date.entity_id()).into());
            TimedSchedule::new(date, cx)
        })
    });
    let id = id.unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find(id.clone()).value(), Some("2026/09/15 08:00:00"));
        window.click(id.clone(), cx);
        window.click("calendar-2026-09-16-0-2", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        // Picking a date keeps the popup open so the time can be edited next.
        assert_eq!(window.find(id.clone()).expanded(), Some(true));
        window.within("start-time").click("minute", cx);
        window.press("4", cx);
        window.press("5", cx);
        // The minute is complete, so the seconds segment is selected next.
        window.press("up", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find(id.clone()).value(), Some("2026/09/16 08:45:01"));
        window.press("escape", cx);
        assert_eq!(window.find(id.clone()).expanded(), Some(false));
    })
    .unwrap();
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.changes,
            [
                "2026-09-16 08:00:00",
                "2026-09-16 08:04:00",
                "2026-09-16 08:45:00",
                "2026-09-16 08:45:01",
            ]
            .map(|value| DateTime::Single(Some(at(value))))
        );
    });
}

#[gpui_kit::test]
fn inverted_time_range_is_not_reported_and_closes_ordered(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut id: Option<ElementId> = None;
    let (handle, view) = common::open_window(cx, Some(size(px(800.), px(600.))), |window, cx| {
        cx.new(|cx| {
            let date = cx.new(|cx| {
                let mut state =
                    DatePickerState::range(window, cx).time_precision(TimePrecision::Minute);
                state.set_date_time(
                    (at("2026-09-15 09:00:00"), at("2026-09-15 18:00:00")),
                    window,
                    cx,
                );
                state
            });
            id = Some(("date-picker", date.entity_id()).into());
            TimedSchedule::new(date, cx)
        })
    });
    let id = id.unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click(id.clone(), cx);
        window.within("end-time").click("hour", cx);
        window.press("0", cx);
        window.press("8", cx);
    })
    .unwrap();
    cx.run_until_parked();
    view.read_with(cx, |view, _| assert!(view.changes.is_empty()));
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find(id.clone()).expanded(), Some(false));
        assert_eq!(
            window.find(id.clone()).value(),
            Some("2026/09/15 09:00 - 2026/09/15 09:00")
        );
    })
    .unwrap();
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.changes,
            [DateTime::Range(
                Some(at("2026-09-15 09:00:00")),
                Some(at("2026-09-15 09:00:00"))
            )]
        );
    });
}
