mod common;

use gpui_kit::component::slider::{Slider, SliderEvent, SliderState, SliderValue};
use gpui_kit::test::{TestEventExt, TestWindowExt};
use gpui_kit::{
    AppContext, Context, Entity, MouseButton, Subscription, TestAppContext, Window, div, point,
    prelude::*, px, size,
};
use std::{cell::RefCell, rc::Rc};

struct Volume {
    state: Entity<SliderState>,
    disabled: bool,
}

impl Render for Volume {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_8()
            .w(px(264.))
            .child(Slider::new(&self.state).disabled(self.disabled))
    }
}

type Events = Rc<RefCell<Vec<(&'static str, SliderValue)>>>;

fn record(state: &Entity<SliderState>, cx: &mut TestAppContext) -> (Events, Subscription) {
    let events = Events::default();
    let recorded = events.clone();
    let subscription = cx.update(|cx| {
        cx.subscribe(state, move |_, event: &SliderEvent, _| {
            recorded.borrow_mut().push(match event {
                SliderEvent::Change(value) => ("change", *value),
                SliderEvent::Release(value) => ("release", *value),
            });
        })
    });
    (events, subscription)
}

#[gpui_kit::test]
fn slider_track_changes_before_release_and_releases_only_once(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let state = cx.new(|_| SliderState::new().step(10.).default_value(10.));
    let (events, _subscription) = record(&state, cx);
    let (handle, _) = common::open_window(cx, Some(size(px(400.), px(240.))), |_, cx| {
        cx.new(|_| Volume {
            state: state.clone(),
            disabled: false,
        })
    });
    let position = cx
        .update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            let bounds = window.find("slider-bar-container").bounds();
            let position = point(bounds.left() + bounds.size.width * 0.6, bounds.center().y);
            window.pointer_move(position, None, cx);
            window.pointer_down(position, MouseButton::Left, cx);
            assert_eq!(state.read(cx).value(), SliderValue::Single(60.));
            position
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(&*events.borrow(), &[("change", SliderValue::Single(60.))]);

    cx.update_window(handle.into(), |_, window, cx| {
        window.pointer_up(position, MouseButton::Left, cx);
        window.pointer_up(position, MouseButton::Left, cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(
        &*events.borrow(),
        &[
            ("change", SliderValue::Single(60.)),
            ("release", SliderValue::Single(60.)),
        ]
    );
}

#[gpui_kit::test]
fn slider_thumb_drag_updates_live_and_stops_after_release(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let state = cx.new(|_| SliderState::new().step(10.).default_value(20.));
    let (events, _subscription) = record(&state, cx);
    let (handle, _) = common::open_window(cx, Some(size(px(400.), px(240.))), |_, cx| {
        cx.new(|_| Volume {
            state: state.clone(),
            disabled: false,
        })
    });
    let destination = cx
        .update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            let thumb = window.find(("slider-thumb", 0u32)).bounds().center();
            let track = window.find("slider-bar-container").bounds();
            let destination = point(track.left() + track.size.width * 0.8, track.center().y);
            window.pointer_move(thumb, None, cx);
            window.pointer_down(thumb, MouseButton::Left, cx);
            assert_eq!(state.read(cx).value(), SliderValue::Single(20.));
            window.pointer_move(destination, Some(MouseButton::Left), cx);
            // The first motion starts the native drag; the next dispatch moves it.
            window.pointer_move(destination, Some(MouseButton::Left), cx);
            assert_eq!(state.read(cx).value(), SliderValue::Single(80.));
            destination
        })
        .unwrap();
    cx.run_until_parked();
    assert!(!events.borrow().is_empty());
    assert!(events.borrow().iter().all(|(event, _)| *event == "change"));
    assert_eq!(
        events.borrow().last(),
        Some(&("change", SliderValue::Single(80.)))
    );
    events.borrow_mut().clear();

    cx.update_window(handle.into(), |_, window, cx| {
        window.pointer_up(destination, MouseButton::Left, cx);
        window.pointer_move(point(px(360.), px(180.)), None, cx);
        assert_eq!(state.read(cx).value(), SliderValue::Single(80.));
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(&*events.borrow(), &[("release", SliderValue::Single(80.))]);
}

#[gpui_kit::test]
fn disabled_slider_gesture_neither_changes_value_nor_emits_events(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let state = cx.new(|_| SliderState::new().default_value(20.));
    let (events, _subscription) = record(&state, cx);
    let (handle, _) = common::open_window(cx, Some(size(px(400.), px(240.))), |_, cx| {
        cx.new(|_| Volume {
            state: state.clone(),
            disabled: true,
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let track = window.find("slider-bar-container").bounds();
        let start = point(track.left() + track.size.width * 0.4, track.center().y);
        let end = point(track.left() + track.size.width * 0.8, track.center().y);
        window.pointer_move(start, None, cx);
        window.pointer_down(start, MouseButton::Left, cx);
        assert_eq!(state.read(cx).value(), SliderValue::Single(20.));
        window.pointer_move(end, Some(MouseButton::Left), cx);
        window.pointer_move(end, Some(MouseButton::Left), cx);
        assert_eq!(state.read(cx).value(), SliderValue::Single(20.));
        window.pointer_up(end, MouseButton::Left, cx);
        assert_eq!(state.read(cx).value(), SliderValue::Single(20.));
    })
    .unwrap();
    cx.run_until_parked();
    assert!(events.borrow().is_empty());
}
