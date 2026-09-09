use gpui_kit::component::{
    IconName, Sizable, Size, StyledExt,
    button::Button,
    stepper::{Stepper, StepperItem},
    v_flex,
};
use gpui_kit::*;
use serde::Deserialize;

use crate::{ChangeStorySize, section, story_toolbar};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = stepper_story, no_json)]
struct ToggleDisabled;

pub struct StepperStory {
    focus_handle: gpui_kit::FocusHandle,
    size: Size,
    stepper0_step: usize,
    stepper1_step: usize,
    stepper2_step: usize,
    stepper3_step: usize,
    disabled: bool,
    _subscritions: Vec<Subscription>,
}

impl super::Story for StepperStory {
    fn title() -> &'static str {
        "Stepper"
    }

    fn description() -> &'static str {
        "A step-by-step process for users to navigate through a series of steps."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl StepperStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            size: Size::default(),
            stepper0_step: 1,
            stepper1_step: 0,
            stepper2_step: 2,
            stepper3_step: 0,
            disabled: false,
            _subscritions: vec![],
        }
    }
}

impl Focusable for StepperStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for StepperStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleDisabled, _, cx| {
                this.disabled = !this.disabled;
                cx.notify();
            }))
            .child(story_toolbar(self.size).dropdown_child(
                Button::new("stepper-options").label("Options"),
                {
                    let disabled = self.disabled;
                    move |menu, _, _| {
                        menu.menu_with_check("Disabled", disabled, Box::new(ToggleDisabled))
                    }
                },
            ))
            .child(
                section("Horizontal Stepper").w(px(480.)).v_flex().child(
                    Stepper::new("stepper0")
                        .w_full()
                        .with_size(self.size)
                        .disabled(self.disabled)
                        .selected_index(self.stepper0_step)
                        .items([
                            StepperItem::new().child("Step 1"),
                            StepperItem::new().child("Step 2"),
                            StepperItem::new().child("Step 3"),
                        ])
                        .on_click(cx.listener(|this, step, _, cx| {
                            this.stepper0_step = *step;
                            cx.notify();
                        })),
                ),
            )
            .child(
                section("Icon Stepper").w(px(480.)).v_flex().child(
                    Stepper::new("stepper1")
                        .w_full()
                        .with_size(self.size)
                        .disabled(self.disabled)
                        .selected_index(self.stepper1_step)
                        .items([
                            StepperItem::new()
                                .icon(IconName::Calendar)
                                .child("Order Details"),
                            StepperItem::new().icon(IconName::Inbox).child("Shipping"),
                            StepperItem::new().icon(IconName::Frame).child("Preview"),
                            StepperItem::new().icon(IconName::Info).child("Finish"),
                        ])
                        .on_click(cx.listener(|this, step, _, cx| {
                            this.stepper1_step = *step;
                            cx.notify();
                        })),
                ),
            )
            .child(
                section("Vertical Stepper").w(px(480.)).v_flex().child(
                    Stepper::new("stepper3")
                        .vertical()
                        .with_size(self.size)
                        .disabled(self.disabled)
                        .selected_index(self.stepper2_step)
                        .items_center()
                        .items([
                            StepperItem::new()
                                .pb_8()
                                .icon(IconName::Building2)
                                .child(v_flex().child("Step 1").child("Description for step 1.")),
                            StepperItem::new()
                                .pb_8()
                                .icon(IconName::Asterisk)
                                .child(v_flex().child("Step 2").child("Description for step 2.")),
                            StepperItem::new()
                                .pb_8()
                                .icon(IconName::Folder)
                                .child(v_flex().child("Step 3").child("Description for step 3.")),
                            StepperItem::new()
                                .icon(IconName::CircleCheck)
                                .child(v_flex().child("Step 4").child("Description for step 4.")),
                        ])
                        .on_click(cx.listener(|this, step, _, cx| {
                            this.stepper2_step = *step;
                            cx.notify();
                        })),
                ),
            )
            .child(
                section("Text Center").w(px(480.)).v_flex().child(
                    Stepper::new("stepper4")
                        .with_size(self.size)
                        .disabled(self.disabled)
                        .selected_index(self.stepper3_step)
                        .text_center(true)
                        .items([
                            StepperItem::new().child(
                                v_flex()
                                    .items_center()
                                    .child("Step 1")
                                    .child("Desc for step 1."),
                            ),
                            StepperItem::new().child(
                                v_flex()
                                    .items_center()
                                    .child("Step 2")
                                    .child("Desc for step 2."),
                            ),
                            StepperItem::new().child(
                                v_flex()
                                    .items_center()
                                    .child("Step 3")
                                    .child("Desc for step 3."),
                            ),
                        ])
                        .on_click(cx.listener(|this, step, _, cx| {
                            this.stepper3_step = *step;
                            cx.notify();
                        })),
                ),
            )
    }
}
