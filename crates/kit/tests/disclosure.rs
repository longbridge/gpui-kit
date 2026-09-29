mod common;
use gpui_kit::component::{
    accordion::Accordion,
    slider::{Slider, SliderState},
    stepper::{Stepper, StepperItem},
};
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{
    AppContext, Context, Entity, InputEvent as _, Pixels, Point, TestAppContext, TouchDragEvent,
    TouchPhase, Window, div, point, prelude::*, px, size,
};

struct Settings {
    open: Vec<usize>,
    step: usize,
    slider: Entity<SliderState>,
    disabled: bool,
    advanced_disabled: bool,
}
impl Render for Settings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .p_4()
            .gap_4()
            .child(
                Accordion::new("sections")
                    .h_auto()
                    .disabled(self.disabled)
                    .item(|item| {
                        item.title("General")
                            .open(self.open.contains(&0))
                            .child(div().h_12().child("General options"))
                    })
                    .item(|item| {
                        item.title("Advanced")
                            .disabled(self.advanced_disabled)
                            .open(self.open.contains(&1))
                            .child(div().h_12().child("Advanced options"))
                    })
                    .on_toggle_click(cx.listener(|this, open: &[usize], _, cx| {
                        this.open = open.to_vec();
                        cx.notify();
                    })),
            )
            .child(
                Stepper::new("wizard")
                    .selected_index(self.step)
                    .disabled(self.disabled)
                    .items([
                        StepperItem::new().child("Account"),
                        StepperItem::new().child("Review"),
                    ])
                    .on_click(cx.listener(|this, step: &usize, _, cx| {
                        this.step = *step;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("step-content")
                    .test_support()
                    .child(if self.step == 0 {
                        div().id("account").test_support().child("Account")
                    } else {
                        div().id("review").test_support().child("Review")
                    }),
            )
            .child(Slider::new(&self.slider).disabled(self.disabled).w_64())
    }
}

#[gpui_kit::test]
fn accordion_expands_one_panel_and_stepper_navigates(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(|cx| cx.set_reduce_motion(true));
    let (handle, _) = common::open_window(cx, Some(size(px(640.), px(600.))), |_, cx| {
        cx.new(|cx| Settings {
            open: vec![],
            step: 0,
            disabled: false,
            advanced_disabled: false,
            slider: cx.new(|_| SliderState::new().default_value(20.)),
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let closed_height = window.find("sections").bounds().size.height;
        window.within("sections").click(("trigger", 0usize), cx);
        assert_eq!(
            window
                .within("sections")
                .find(("trigger", 0usize))
                .expanded(),
            Some(true)
        );
        window.within("sections").click(("trigger", 1usize), cx);
        assert_eq!(
            window
                .within("sections")
                .find(("trigger", 0usize))
                .expanded(),
            Some(false)
        );
        assert_eq!(
            window
                .within("sections")
                .find(("trigger", 1usize))
                .expanded(),
            Some(true)
        );
        assert!(window.find("sections").bounds().size.height > closed_height);
        assert!(
            window
                .within("sections")
                .find(("panel", 1usize))
                .bounds()
                .size
                .height
                > px(0.)
        );
        window.within("sections").click(("trigger", 1usize), cx);
        assert_eq!(window.find("sections").bounds().size.height, closed_height);
        window.within("wizard").click(("trigger", 1usize), cx);
        assert!(window.try_find("account").is_none());
        assert!(window.find("review").visible());
        window.within("wizard").click(("trigger", 0usize), cx);
        assert!(window.find("account").visible());
    })
    .unwrap();
}

#[gpui_kit::test]
fn disabled_disclosures_and_steps_do_not_change_content(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, _) = common::open_window(cx, Some(size(px(640.), px(600.))), |_, cx| {
        cx.new(|cx| Settings {
            open: vec![],
            step: 0,
            disabled: true,
            advanced_disabled: false,
            slider: cx.new(|_| SliderState::new().default_value(20.)),
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.within("sections").click(("trigger", 0usize), cx);
        assert_eq!(
            window
                .within("sections")
                .find(("trigger", 0usize))
                .expanded(),
            Some(false)
        );
        window.within("wizard").click(("trigger", 1usize), cx);
        assert!(window.find("account").visible());
        assert!(window.try_find("review").is_none());
    })
    .unwrap();
}

#[gpui_kit::test]
fn accordion_preserves_disabled_items_when_the_group_is_enabled(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(|cx| cx.set_reduce_motion(true));
    let (handle, handle_content) =
        common::open_window(cx, Some(size(px(640.), px(600.))), |_, cx| {
            cx.new(|cx| Settings {
                open: vec![],
                step: 0,
                disabled: false,
                advanced_disabled: true,
                slider: cx.new(|_| SliderState::new().default_value(20.)),
            })
        });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let settings = handle_content.clone();
        for group_disabled in [false, true, false] {
            settings.update(cx, |settings, cx| {
                settings.disabled = group_disabled;
                settings.open.clear();
                cx.notify();
            });
            window.render_frame(cx);
            window.within("sections").click(("trigger", 1usize), cx);
            assert_eq!(
                window
                    .within("sections")
                    .find(("trigger", 1usize))
                    .expanded(),
                Some(false)
            );
            assert!(settings.read(cx).open.is_empty());

            window.within("sections").click(("trigger", 0usize), cx);
            assert_eq!(
                window
                    .within("sections")
                    .find(("trigger", 0usize))
                    .expanded(),
                Some(!group_disabled)
            );
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn slider_click_and_drag_move_the_actual_thumb(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, _) = common::open_window(cx, Some(size(px(640.), px(600.))), |_, cx| {
        cx.new(|cx| Settings {
            open: vec![],
            step: 0,
            disabled: false,
            advanced_disabled: false,
            slider: cx.new(|_| SliderState::new().default_value(20.)),
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let before = window.find(("slider-thumb", 0u32)).bounds();
        let track = window.find("slider-bar-container").bounds();
        window.click_at(
            "slider-bar-container",
            point(track.size.width * 0.8, track.size.height / 2.),
            cx,
        );
        let after = window.find(("slider-thumb", 0u32)).bounds();
        assert!(after.center().x > before.center().x);
        window.drag(
            after.center(),
            point(track.left() + track.size.width * 0.3, track.center().y),
            cx,
        );
        assert!(window.find(("slider-thumb", 0u32)).bounds().center().x < after.center().x);
    })
    .unwrap();
}

/// A finger drag reaches elements as `TouchDragEvent`s, not as mouse drags.
fn touch_drag(window: &mut Window, cx: &mut gpui_kit::App, from: Point<Pixels>, to: Point<Pixels>) {
    for (phase, position) in [
        (TouchPhase::Started, from),
        (TouchPhase::Moved, to),
        (TouchPhase::Ended, to),
    ] {
        window.dispatch_event(
            TouchDragEvent {
                phase,
                start_position: from,
                position,
            }
            .to_platform_input(),
            cx,
        );
    }
    window.render_frame(cx);
}

#[gpui_kit::test]
fn slider_touch_drag_moves_the_actual_thumb(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, _) = common::open_window(cx, Some(size(px(640.), px(600.))), |_, cx| {
        cx.new(|cx| Settings {
            open: vec![],
            step: 0,
            disabled: false,
            advanced_disabled: false,
            slider: cx.new(|_| SliderState::new().default_value(20.)),
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let before = window.find(("slider-thumb", 0u32)).bounds();
        let track = window.find("slider-bar-container").bounds();
        let right = point(track.left() + track.size.width * 0.8, track.center().y);
        touch_drag(window, cx, before.center(), right);
        let after = window.find(("slider-thumb", 0u32)).bounds();
        assert!(after.center().x > before.center().x);

        let left = point(track.left() + track.size.width * 0.3, track.center().y);
        touch_drag(window, cx, after.center(), left);
        assert!(window.find(("slider-thumb", 0u32)).bounds().center().x < after.center().x);
    })
    .unwrap();
}

/// A slider under a layer that covers it, as a dialog or sheet would.
struct CoveredSlider {
    slider: Entity<SliderState>,
}
impl Render for CoveredSlider {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .child(Slider::new(&self.slider).w_64())
            .child(div().id("cover").absolute().inset_0().occlude())
    }
}

#[gpui_kit::test]
fn slider_ignores_touch_drags_on_a_covering_layer(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, _) = common::open_window(cx, Some(size(px(640.), px(600.))), |_, cx| {
        cx.new(|cx| CoveredSlider {
            slider: cx.new(|_| SliderState::new().default_value(20.)),
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let before = window.find(("slider-thumb", 0u32)).bounds();
        let track = window.find("slider-bar-container").bounds();
        touch_drag(
            window,
            cx,
            before.center(),
            point(track.right(), track.center().y),
        );
        assert_eq!(window.find(("slider-thumb", 0u32)).bounds(), before);
    })
    .unwrap();
}

#[gpui_kit::test]
fn slider_drops_a_touch_drag_whose_end_never_arrived(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, settings) = common::open_window(cx, Some(size(px(640.), px(600.))), |_, cx| {
        cx.new(|cx| Settings {
            open: vec![],
            step: 0,
            disabled: false,
            advanced_disabled: false,
            slider: cx.new(|_| SliderState::new().default_value(20.)),
        })
    });
    let set_disabled = |disabled: bool, window: &mut Window, cx: &mut gpui_kit::App| {
        settings.update(cx, |settings, cx| {
            settings.disabled = disabled;
            cx.notify();
        });
        window.render_frame(cx);
    };
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let thumb = window.find(("slider-thumb", 0u32)).bounds().center();
        let track = window.find("slider-bar-container").bounds();
        let dispatch = |phase, from, to, window: &mut Window, cx: &mut gpui_kit::App| {
            window.dispatch_event(
                TouchDragEvent {
                    phase,
                    start_position: from,
                    position: to,
                }
                .to_platform_input(),
                cx,
            );
            window.render_frame(cx);
        };
        // The slider is disabled mid-drag, so it never sees the drag end.
        dispatch(TouchPhase::Started, thumb, thumb, window, cx);
        set_disabled(true, window, cx);
        dispatch(TouchPhase::Ended, thumb, thumb, window, cx);
        set_disabled(false, window, cx);

        // A later drag another element claimed must not move it.
        let before = window.find(("slider-thumb", 0u32)).bounds();
        let elsewhere = point(track.left(), track.bottom() + px(200.));
        dispatch(
            TouchPhase::Moved,
            elsewhere,
            point(track.right(), track.center().y),
            window,
            cx,
        );
        assert_eq!(window.find(("slider-thumb", 0u32)).bounds(), before);
    })
    .unwrap();
}

#[gpui_kit::test]
fn disabled_slider_ignores_pointer_changes(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, _) = common::open_window(cx, Some(size(px(640.), px(600.))), |_, cx| {
        cx.new(|cx| Settings {
            open: vec![],
            step: 0,
            disabled: true,
            advanced_disabled: false,
            slider: cx.new(|_| SliderState::new().default_value(20.)),
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let before = window.find(("slider-thumb", 0u32)).bounds();
        let track = window.find("slider-bar-container").bounds();
        window.click_at(
            "slider-bar-container",
            point(track.size.width * 0.8, track.size.height / 2.),
            cx,
        );
        window.drag(before.center(), point(track.right(), track.center().y), cx);
        touch_drag(
            window,
            cx,
            before.center(),
            point(track.right(), track.center().y),
        );
        assert_eq!(window.find(("slider-thumb", 0u32)).bounds(), before);
    })
    .unwrap();
}
