use gpui_kit::component::{
    Disableable,
    date_picker::{DatePicker, DatePickerState, DateRangePreset},
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
    let handle = cx.open_window(size(px(640.), px(600.)), |window, cx| {
        let date = cx.new(|cx| DatePickerState::new(window, cx));
        id = Some(("date-picker", date.entity_id()).into());
        Schedule {
            date,
            disabled: false,
        }
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
    let handle = cx.open_window(size(px(640.), px(600.)), |window, cx| {
        let date = cx.new(|cx| DatePickerState::new(window, cx));
        id = Some(("date-picker", date.entity_id()).into());
        Schedule {
            date,
            disabled: true,
        }
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click(id.clone().unwrap(), cx);
        assert_eq!(window.find(id.clone().unwrap()).expanded(), Some(false));
        assert!(window.try_find(("preset", 0usize)).is_none());
    })
    .unwrap();
}
