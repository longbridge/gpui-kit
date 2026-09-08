use gpui_kit::component::{
    accordion::Accordion,
    slider::{Slider, SliderState},
    stepper::{Stepper, StepperItem},
};
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{
    AppContext, Context, Entity, TestAppContext, Window, div, point, prelude::*, px, size,
};

struct Settings {
    open: Vec<usize>,
    step: usize,
    slider: Entity<SliderState>,
    disabled: bool,
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
    let handle = cx.open_window(size(px(640.), px(600.)), |_, cx| Settings {
        open: vec![],
        step: 0,
        disabled: false,
        slider: cx.new(|_| SliderState::new().default_value(20.)),
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
    let handle = cx.open_window(size(px(640.), px(600.)), |_, cx| Settings {
        open: vec![],
        step: 0,
        disabled: true,
        slider: cx.new(|_| SliderState::new().default_value(20.)),
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
fn slider_click_and_drag_move_the_actual_thumb(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.open_window(size(px(640.), px(600.)), |_, cx| Settings {
        open: vec![],
        step: 0,
        disabled: false,
        slider: cx.new(|_| SliderState::new().default_value(20.)),
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

#[gpui_kit::test]
fn disabled_slider_ignores_pointer_changes(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.open_window(size(px(640.), px(600.)), |_, cx| Settings {
        open: vec![],
        step: 0,
        disabled: true,
        slider: cx.new(|_| SliderState::new().default_value(20.)),
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
        assert_eq!(window.find(("slider-thumb", 0u32)).bounds(), before);
    })
    .unwrap();
}
