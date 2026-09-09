use gpui_kit::component::{
    Disableable as _, Sizable, Size, StyledExt,
    button::Button,
    input::{OtpEvent, OtpInput, OtpState},
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde::Deserialize;

use crate::{ChangeStorySize, section, story_toolbar};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = otp_input_story, no_json)]
struct ToggleMasked;

pub fn init(_: &mut App) {}

pub struct OtpInputStory {
    otp_masked: bool,
    otp_state: Entity<OtpState>,
    otp_value: Option<SharedString>,
    otp_state_small: Entity<OtpState>,
    otp_state_large: Entity<OtpState>,
    otp_state_sized: Entity<OtpState>,
    otp_state_disabled: Entity<OtpState>,
    size: Size,

    _subscriptions: Vec<Subscription>,
}

impl super::Story for OtpInputStory {
    fn title() -> &'static str {
        "OtpInput"
    }

    fn description() -> &'static str {
        "Enter short verification and recovery codes with clear grouping and masking controls."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl OtpInputStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let otp_state = cx.new(|cx| OtpState::new(6, window, cx).masked(true));

        let _subscriptions =
            vec![
                cx.subscribe(&otp_state, |this, state, ev: &OtpEvent, cx| match ev {
                    OtpEvent::Complete => {
                        let text = state.read(cx).value();
                        this.otp_value = Some(text.clone());
                        cx.notify();
                    }
                    _ => {}
                }),
            ];

        Self {
            otp_masked: true,
            otp_state,
            otp_value: None,
            otp_state_small: cx.new(|cx| {
                OtpState::new(6, window, cx)
                    .default_value("123456")
                    .masked(true)
            }),
            otp_state_large: cx.new(|cx| {
                OtpState::new(6, window, cx)
                    .default_value("012345")
                    .masked(true)
            }),
            otp_state_sized: cx.new(|cx| {
                OtpState::new(4, window, cx)
                    .masked(true)
                    .default_value("654321")
            }),
            otp_state_disabled: cx.new(|cx| {
                OtpState::new(6, window, cx)
                    .masked(true)
                    .default_value("123456")
            }),
            size: Size::Medium,
            _subscriptions,
        }
    }

    fn toggle_opt_masked(&mut self, _: &bool, window: &mut Window, cx: &mut Context<Self>) {
        self.otp_masked = !self.otp_masked;
        self.otp_state.update(cx, |state, cx| {
            state.set_masked(self.otp_masked, window, cx)
        });
        self.otp_state_small.update(cx, |state, cx| {
            state.set_masked(self.otp_masked, window, cx)
        });
        self.otp_state_large.update(cx, |state, cx| {
            state.set_masked(self.otp_masked, window, cx)
        });
        self.otp_state_sized.update(cx, |state, cx| {
            state.set_masked(self.otp_masked, window, cx)
        });
        self.otp_state_disabled.update(cx, |state, cx| {
            state.set_masked(self.otp_masked, window, cx)
        });
    }
}

impl Focusable for OtpInputStory {
    fn focus_handle(&self, cx: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.otp_state.focus_handle(cx)
    }
}

impl Render for OtpInputStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("otp-input-story")
            .size_full()
            .gap_5()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleMasked, window, cx| {
                this.toggle_opt_masked(&false, window, cx);
                cx.notify();
            }))
            .child(story_toolbar(self.size).dropdown_child(
                Button::new("otp-options").label("Options"),
                {
                    let masked = self.otp_masked;
                    move |menu, _, _| menu.menu_with_check("Masked", masked, Box::new(ToggleMasked))
                },
            ))
            .child(
                section("Default")
                    .description("Six cells with masking and value updates.")
                    .v_flex()
                    .child(OtpInput::new(&self.otp_state).with_size(self.size))
                    .when_some(self.otp_value.clone(), |this, otp| {
                        this.child(format!("Value: {}", otp))
                    }),
            )
            .child(
                section("Grouping")
                    .description("Cells can be shown as one or several groups.")
                    .v_flex()
                    .gap_4()
                    .child(
                        OtpInput::new(&self.otp_state_small)
                            .groups(1)
                            .with_size(self.size),
                    )
                    .child(
                        OtpInput::new(&self.otp_state_large)
                            .groups(3)
                            .with_size(self.size),
                    ),
            )
            .child(
                section("Custom size")
                    .description("Custom cell dimensions.")
                    .child(
                        OtpInput::new(&self.otp_state_sized)
                            .groups(1)
                            .with_size(px(55.)),
                    ),
            )
            .child(
                section("Disabled")
                    .description("Disabled input with a value.")
                    .child(
                        OtpInput::new(&self.otp_state_disabled)
                            .with_size(self.size)
                            .disabled(true),
                    ),
            )
    }
}
