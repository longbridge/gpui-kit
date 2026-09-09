use gpui_kit::component::{
    ActiveTheme, Colorize as _, StyledExt, WindowExt,
    button::Button,
    clipboard::Clipboard,
    h_flex,
    slider::{Slider, SliderEvent, SliderScale, SliderState, SliderValue},
    v_flex,
};
use gpui_kit::*;
use serde::Deserialize;

use crate::{section, story_toolbar_group};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = slider_story, no_json)]
struct ToggleDisabled;

pub struct SliderStory {
    focus_handle: gpui_kit::FocusHandle,
    slider1: Entity<SliderState>,
    slider1_value: f32,
    slider1_released_value: f32,
    slider3: Entity<SliderState>,
    slider3_released_value: SliderValue,
    slider_hsl: [Entity<SliderState>; 4],
    slider_hsl_value: Hsla,
    slider_logarithmic: Entity<SliderState>,
    slider_reverse: Entity<SliderState>,
    disabled: bool,
    _subscritions: Vec<Subscription>,
}

impl super::Story for SliderStory {
    fn title() -> &'static str {
        "Slider"
    }

    fn description() -> &'static str {
        "Displays a slider control for selecting a value within a range."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SliderStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let slider1 = cx.new(|_| {
            SliderState::new()
                .min(-255.)
                .max(255.)
                .default_value(75.)
                .step(15.)
        });

        let slider_hsl = [
            cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.38)
            }),
            cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.5)
            }),
            cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.5)
            }),
            cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.5)
            }),
        ];

        let slider3 = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .default_value(12.0..45.0)
                .step(1.)
        });

        let slider_logarithmic = cx.new(|_| {
            SliderState::new()
                .min(0.25)
                .max(4.0)
                .default_value(1.0)
                .step(0.05)
                .scale(SliderScale::Logarithmic)
        });

        let slider_reverse = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(10.)
                .step(1.)
                .default_value(5.)
        });

        let mut _subscritions = vec![
            cx.subscribe(&slider1, |this, _, event: &SliderEvent, cx| match event {
                SliderEvent::Change(value) => {
                    this.slider1_value = value.start();
                    cx.notify();
                }
                SliderEvent::Release(value) => {
                    this.slider1_released_value = value.start();
                    cx.notify();
                }
            }),
            cx.subscribe(&slider3, |this, _, event: &SliderEvent, cx| match event {
                SliderEvent::Change(_) => {}
                SliderEvent::Release(value) => {
                    this.slider3_released_value = *value;
                    cx.notify();
                }
            }),
        ];

        _subscritions.extend(
            slider_hsl
                .iter()
                .map(|slider| {
                    cx.subscribe(slider, |this, _, event: &SliderEvent, cx| match event {
                        SliderEvent::Change(_) => {
                            this.slider_hsl_value = hsla(
                                this.slider_hsl[0].read(cx).value().start(),
                                this.slider_hsl[1].read(cx).value().start(),
                                this.slider_hsl[2].read(cx).value().start(),
                                this.slider_hsl[3].read(cx).value().start(),
                            );
                            cx.notify();
                        }
                        SliderEvent::Release(_) => {}
                    })
                })
                .collect::<Vec<_>>(),
        );

        slider_hsl[0].update(cx, |slider, cx| {
            cx.emit(SliderEvent::Change(slider.value()));
        });

        Self {
            focus_handle: cx.focus_handle(),
            slider1_value: 75.,
            slider1_released_value: 75.,
            slider1,
            slider3_released_value: (12.0, 45.0).into(),
            slider3,
            slider_hsl,
            slider_hsl_value: gpui_kit::red(),
            slider_logarithmic,
            slider_reverse,
            disabled: false,
            _subscritions,
        }
    }
}

impl Focusable for SliderStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SliderStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rgb = SharedString::from(self.slider_hsl_value.to_hex());

        v_flex()
            .w_full()
            .items_center()
            .gap_3()
            .on_action(cx.listener(|this, _: &ToggleDisabled, _, cx| {
                this.disabled = !this.disabled;
                cx.notify();
            }))
            .child(story_toolbar_group().dropdown_child(
                Button::new("slider-options").label("Options"),
                {
                    let disabled = self.disabled;
                    move |menu, _, _| {
                        menu.menu_with_check("Disabled", disabled, Box::new(ToggleDisabled))
                    }
                },
            ))
            .child(
                section("Default")
                    .description("Adjust a single value within a defined range.")
                    .w_128()
                    .items_center()
                    .child(
                        v_flex()
                            .w(px(360.))
                            .gap_4()
                            .p_4()
                            .rounded(cx.theme().radius_lg)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(div().font_medium().child("Output volume"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!("{}", self.slider1_value)),
                                    ),
                            )
                            .child(Slider::new(&self.slider1).disabled(self.disabled)),
                    ),
            )
            .child(
                section("Range")
                    .description("Choose minimum and maximum values together.")
                    .w_128()
                    .items_center()
                    .child(
                        v_flex()
                            .w(px(360.))
                            .gap_4()
                            .p_4()
                            .rounded(cx.theme().radius_lg)
                            .bg(cx.theme().muted.opacity(0.4))
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(div().font_medium().child("Price range"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!("${}", self.slider3.read(cx).value())),
                                    ),
                            )
                            .child(Slider::new(&self.slider3).disabled(self.disabled)),
                    ),
            )
            .child(
                section("Reverse")
                    .description("Reverse the fill direction for remaining capacity.")
                    .w_128()
                    .items_center()
                    .child(
                        v_flex()
                            .w(px(360.))
                            .gap_4()
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(div().font_medium().child("Storage remaining"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!(
                                                "{} GB",
                                                10. - self.slider_reverse.read(cx).value().start()
                                            )),
                                    ),
                            )
                            .child(
                                Slider::new(&self.slider_reverse)
                                    .horizontal()
                                    .reverse()
                                    .disabled(self.disabled),
                            ),
                    ),
            )
            .child(
                section("Color Picker")
                    .sub_title(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                h_flex()
                                    .text_color(self.slider_hsl_value)
                                    .child(rgb.clone()),
                            )
                            .child(Clipboard::new("copy-hsl").value(rgb).on_copied(
                                |_, window, cx| {
                                    window.push_notification("Color copied to clipboard.", cx)
                                },
                            )),
                    )
                    .w_128()
                    .items_center()
                    .justify_around()
                    .child(
                        v_flex()
                            .h_32()
                            .gap_3()
                            .items_center()
                            .justify_center()
                            .child(
                                Slider::new(&self.slider_hsl[0])
                                    .vertical()
                                    .disabled(self.disabled),
                            )
                            .child(
                                v_flex()
                                    .items_center()
                                    .child("Hue")
                                    .child(format!("{:.0}", self.slider_hsl_value.h * 360.)),
                            ),
                    )
                    .child(
                        v_flex()
                            .h_32()
                            .gap_3()
                            .items_center()
                            .justify_center()
                            .child(
                                Slider::new(&self.slider_hsl[1])
                                    .vertical()
                                    .disabled(self.disabled),
                            )
                            .child(
                                v_flex()
                                    .items_center()
                                    .child("Saturation")
                                    .child(format!("{:.0}", self.slider_hsl_value.s * 100.)),
                            ),
                    )
                    .child(
                        v_flex()
                            .h_32()
                            .gap_3()
                            .items_center()
                            .justify_center()
                            .child(
                                Slider::new(&self.slider_hsl[2])
                                    .vertical()
                                    .disabled(self.disabled),
                            )
                            .child(
                                v_flex()
                                    .items_center()
                                    .child("Lightness")
                                    .child(format!("{:.0}", self.slider_hsl_value.l * 100.)),
                            ),
                    )
                    .child(
                        v_flex()
                            .h_32()
                            .gap_3()
                            .items_center()
                            .justify_center()
                            .child(
                                Slider::new(&self.slider_hsl[3])
                                    .vertical()
                                    .disabled(self.disabled),
                            )
                            .child(
                                v_flex()
                                    .items_center()
                                    .child("Alpha")
                                    .child(format!("{:.0}", self.slider_hsl_value.a * 100.)),
                            ),
                    ),
            )
            .child(
                section("Playback speed")
                    .description("Logarithmic scales provide finer control near common values.")
                    .w_128()
                    .items_center()
                    .child(
                        v_flex()
                            .w(px(360.))
                            .gap_4()
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(div().font_medium().child("Speed"))
                                    .child(format!(
                                        "{:.2}×",
                                        self.slider_logarithmic.read(cx).value().start()
                                    )),
                            )
                            .child(
                                Slider::new(&self.slider_logarithmic)
                                    .horizontal()
                                    .disabled(self.disabled),
                            ),
                    ),
            )
    }
}
