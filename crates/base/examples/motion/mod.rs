use std::time::Duration;

// Components and Motion are standalone example apps. The WASM host embeds
// both, so each deliberately instantiates its own thread-local active palette.
#[allow(clippy::duplicate_mod)]
#[path = "../shared/palette.rs"]
mod palette;

#[cfg(target_family = "wasm")]
use gpui::ApplicationHandle;
#[cfg(not(target_family = "wasm"))]
use gpui::WindowBounds;
use gpui::{
    App, AppContext as _, Application, AsyncApp, Context, IntoElement, ParentElement as _, Render,
    Styled as _, Window, WindowOptions, div, prelude::FluentBuilder as _, px,
};
use gpui_base::{
    Button, Easing, IterationCount, Keyframe, Keyframes, MotionStatus, Presence, Sequence, Spring,
    Stagger, StaggerOrigin, Timing, Transition, animate_keyframes, spring, transition,
};
use palette::{activate as activate_palette, canvas as example_canvas, example_rgb};
#[cfg(target_family = "wasm")]
use std::borrow::Cow;

const START_MINUTES: u32 = 8 * 60;
const END_MINUTES: u32 = 20 * 60;
const DIGIT_HEIGHT: f32 = 38.;

#[derive(Clone, Copy, PartialEq)]
enum Demo {
    SlidingTime,
    Spring,
    Keyframes,
    Presence,
    Stagger,
    Sequence,
}

impl Demo {
    const ALL: [Self; 6] = [
        Self::SlidingTime,
        Self::Spring,
        Self::Keyframes,
        Self::Stagger,
        Self::Presence,
        Self::Sequence,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::SlidingTime => "Sliding time",
            Self::Spring => "Spring",
            Self::Keyframes => "Keyframes",
            Self::Presence => "Presence",
            Self::Stagger => "Stagger",
            Self::Sequence => "Sequence",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::SlidingTime => {
                "Interruptible transitions roll the clock from morning to evening."
            }
            Self::Spring => "A retargeted spring keeps its velocity instead of restarting.",
            Self::Keyframes => "Seven values follow one keyframe track with offset timing.",
            Self::Presence => "A surface stays mounted until its exit transition completes.",
            Self::Stagger => "List rows enter in order from one allocation-free delay policy.",
            Self::Sequence => "Three steps play in turn: slide in, fill, then rest and fade out.",
        }
    }
}

pub struct MotionExample {
    demo: Demo,
    minutes: u32,
    digit_targets: [f32; 4],
    playback: Option<gpui::Task<()>>,
    spring_selected: bool,
    present: bool,
    stagger_generation: usize,
    sequence_generation: usize,
}

impl MotionExample {
    fn new() -> Self {
        Self {
            demo: Demo::SlidingTime,
            minutes: START_MINUTES,
            digit_targets: [0., 8., 0., 0.],
            playback: None,
            spring_selected: false,
            present: true,
            stagger_generation: 0,
            sequence_generation: 0,
        }
    }

    fn play_time(&mut self, cx: &mut Context<Self>) {
        if self.playback.take().is_some() {
            cx.notify();
            return;
        }
        if self.minutes == END_MINUTES {
            self.minutes = START_MINUTES;
            self.digit_targets = [0., 8., 0., 0.];
        }
        self.playback = Some(cx.spawn(async move |this, cx: &mut AsyncApp| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
                let finished = this
                    .update(cx, |this, cx| {
                        this.minutes = (this.minutes + 30).min(END_MINUTES);
                        for (target, digit) in
                            this.digit_targets.iter_mut().zip(time_digits(this.minutes))
                        {
                            *target = advance_digit(*target, digit);
                        }
                        cx.notify();
                        this.minutes == END_MINUTES
                    })
                    .unwrap_or(true);
                if finished {
                    _ = this.update(cx, |this, cx| {
                        this.playback = None;
                        cx.notify();
                    });
                    break;
                }
            }
        }));
        cx.notify();
    }

    fn rolling_digit(
        &self,
        ix: usize,
        target: f32,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let value = transition(
            ("clock-digit", ix.to_string()),
            target,
            Transition::new(Duration::from_millis(620)).easing(Easing::EaseInOut),
            window,
            cx,
        );
        let digit = value.floor() as i32;
        let offset = value.fract() * DIGIT_HEIGHT;
        div()
            .relative()
            .w(px(25.))
            .h(px(DIGIT_HEIGHT))
            .overflow_hidden()
            .child(clock_digit(digit, -offset))
            .child(clock_digit(digit + 1, DIGIT_HEIGHT - offset))
    }

    fn sliding_time(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().downgrade();
        let playing = self.playback.is_some();
        div()
            .flex()
            .items_center()
            .gap_6()
            .child(
                div()
                    .flex()
                    .items_center()
                    .text_size(px(30.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .children(
                        self.digit_targets[..2]
                            .iter()
                            .enumerate()
                            .map(|(ix, target)| {
                                self.rolling_digit(ix, *target, window, cx)
                                    .into_any_element()
                            }),
                    )
                    .child(div().px_1().child(":"))
                    .children(
                        self.digit_targets[2..]
                            .iter()
                            .enumerate()
                            .map(|(ix, target)| {
                                self.rolling_digit(ix + 2, *target, window, cx)
                                    .into_any_element()
                            }),
                    ),
            )
            .child(
                Button::new("play-time")
                    .h_9()
                    .px_3()
                    .border_1()
                    .border_color(example_rgb(0xd4d4d4))
                    .child(if playing {
                        "Stop"
                    } else if self.minutes == END_MINUTES {
                        "Replay"
                    } else {
                        "Play"
                    })
                    .on_click(move |_, _, cx| {
                        _ = entity.update(cx, |this, cx| this.play_time(cx));
                    }),
            )
    }

    fn spring_demo(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let x = spring(
            "selector-indicator",
            if self.spring_selected { 120. } else { 0. },
            Spring::new(Duration::from_millis(420)).with_damping(0.68),
            window,
            cx,
        );
        div()
            .relative()
            .w(px(240.))
            .h_10()
            .border_1()
            .border_color(example_rgb(0xd4d4d4))
            .child(
                div()
                    .absolute()
                    .top(px(0.))
                    .left(px(x))
                    .w(px(119.))
                    .h_full()
                    .bg(example_rgb(0x171717)),
            )
            .child(
                div().relative().h_full().flex().children(
                    [("Focus", false), ("Flow", true)]
                        .into_iter()
                        .enumerate()
                        .map(|(ix, (label, selected))| {
                            let entity = cx.entity().downgrade();
                            Button::new(("spring-option", ix))
                                .w(px(119.))
                                .h_full()
                                .text_color(if self.spring_selected == selected {
                                    example_rgb(0xffffff)
                                } else {
                                    example_rgb(0x525252)
                                })
                                .child(label)
                                .on_click(move |_, _, cx| {
                                    _ = entity.update(cx, |this, cx| {
                                        this.spring_selected = selected;
                                        cx.notify();
                                    });
                                })
                        }),
                ),
            )
    }

    fn keyframes_demo(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .w(px(320.))
            .h(px(132.))
            .p_4()
            .border_1()
            .border_color(example_rgb(0xd4d4d4))
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xs().child("Playback"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(example_rgb(0x737373))
                            .child("Infinite · 1200ms"),
                    ),
            )
            .child(
                div()
                    .h(px(64.))
                    .flex()
                    .items_end()
                    .justify_center()
                    .gap_2()
                    .children((0..7).map(|ix| {
                        let frames = Keyframes::try_new([
                            Keyframe::new(0., 0.),
                            Keyframe::new(0.35, 1.).ease(Easing::EaseOut),
                            Keyframe::new(0.7, 0.),
                            Keyframe::new(1., 0.),
                        ])
                        .expect("static keyframes are valid");
                        let value = animate_keyframes(
                            (ix, "keyframe-bar"),
                            &frames,
                            Timing::new(Duration::from_millis(1200))
                                .delay(Duration::from_millis(ix as u64 * 80).into())
                                .iterations(IterationCount::Infinite),
                            window,
                            cx,
                        )
                        .value;
                        div()
                            .w_5()
                            .h(px(18. + 38. * value))
                            .bg(example_rgb(0x171717))
                            .opacity(0.35 + 0.65 * value)
                    })),
            )
    }

    fn presence_demo(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sample = Presence::new("presence-notice", self.present)
            .transition(Transition::new(Duration::from_millis(360)).easing(Easing::EaseInOut))
            .sample(window, cx);
        let entity = cx.entity().downgrade();
        div()
            .h(px(120.))
            .flex()
            .items_center()
            .gap_4()
            .child(div().w(px(320.)).child(if sample.should_render() {
                div()
                    .w_full()
                    .p_3()
                    .border_1()
                    .border_color(example_rgb(0xd4d4d4))
                    .bg(example_rgb(0xffffff))
                    .opacity(sample.progress)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .child("Background task"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(example_rgb(0x737373))
                                    .child("Complete"),
                            ),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(example_rgb(0x737373))
                            .child("Mounted through the exit phase."),
                    )
                    .into_any_element()
            } else {
                div().into_any_element()
            }))
            .child(
                Button::new("toggle-presence")
                    .h_9()
                    .px_3()
                    .border_1()
                    .border_color(example_rgb(0xd4d4d4))
                    .child(if self.present { "Remove" } else { "Insert" })
                    .on_click(move |_, _, cx| {
                        _ = entity.update(cx, |this, cx| {
                            this.present = !this.present;
                            cx.notify();
                        });
                    }),
            )
    }

    fn stagger_demo(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stagger = Stagger::new(Duration::from_millis(90), StaggerOrigin::First);
        let generation = self.stagger_generation;
        let entity = cx.entity().downgrade();
        div()
            .w(px(380.))
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .border_1()
                    .border_color(example_rgb(0xd4d4d4))
                    .children((0..3).map(|ix| {
                        let frames =
                            Keyframes::try_new([Keyframe::new(0., 0.), Keyframe::new(1., 1.)])
                                .expect("static keyframes are valid");
                        let value = animate_keyframes(
                            ("stagger-item", format!("{generation}-{ix}")),
                            &frames,
                            Timing::new(Duration::from_millis(360))
                                .delay(stagger.delay(ix, 3).into())
                                .ease(Easing::EaseOut),
                            window,
                            cx,
                        )
                        .value;
                        let title = ["Transition", "Spring", "Keyframes"][ix];
                        div()
                            .ml(px((1. - value) * 24.))
                            .w_full()
                            .h_10()
                            .px_3()
                            .when(ix < 2, |this| {
                                this.border_b_1().border_color(example_rgb(0xe5e5e5))
                            })
                            .bg(example_rgb(0xffffff))
                            .opacity(value)
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .w_5()
                                    .text_xs()
                                    .text_color(example_rgb(0x737373))
                                    .child(format!("0{}", ix + 1)),
                            )
                            .child(div().text_sm().child(title))
                    })),
            )
            .child(
                Button::new("replay-stagger")
                    .self_end()
                    .h_9()
                    .px_3()
                    .border_1()
                    .border_color(example_rgb(0xd4d4d4))
                    .child("Replay")
                    .on_click(move |_, _, cx| {
                        _ = entity.update(cx, |this, cx| {
                            this.stagger_generation += 1;
                            cx.notify();
                        });
                    }),
            )
    }
}

impl MotionExample {
    /// One `Sequence<f32>` runs 0 → 1 → 2 → 3 with a different transition per
    /// step, and the demo reads each property off the segment the value is in:
    /// step 0 slides the card in, step 1 fills its bar, step 2 rests and fades.
    fn sequence_demo(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        const STEPS: [&str; 3] = ["Slide in", "Fill", "Rest, then fade out"];
        let sample = Sequence::new(format!("sequence-card-{}", self.sequence_generation), 0.)
            .with_step(
                1.,
                Transition::new(Duration::from_millis(360)).easing(Easing::EaseOut),
            )
            .with_step(
                2.,
                Transition::new(Duration::from_millis(900)).easing(Easing::EaseInOut),
            )
            .with_step(
                3.,
                Transition::new(Duration::from_millis(420))
                    .delay(Duration::from_millis(600))
                    .easing(Easing::EaseIn),
            )
            .sample(window, cx);
        let value = *sample.value();
        let slide = value.min(1.);
        let fill = (value - 1.).clamp(0., 1.);
        let fade = (value - 2.).clamp(0., 1.);
        let status = match sample.status() {
            MotionStatus::Idle => "idle",
            MotionStatus::Delayed => "delayed",
            MotionStatus::Running => "running",
            MotionStatus::Finished => "finished",
        };
        let entity = cx.entity().downgrade();
        div()
            .w(px(380.))
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div().h(px(96.)).flex().items_center().child(
                    div()
                        .w_full()
                        .ml(px((1. - slide) * 48.))
                        .opacity(slide * (1. - fade))
                        .p_3()
                        .border_1()
                        .border_color(example_rgb(0xd4d4d4))
                        .bg(example_rgb(0xffffff))
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .child("Uploading report.pdf"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(example_rgb(0x737373))
                                        .child(format!("{}%", (fill * 100.).round())),
                                ),
                        )
                        .child(
                            div().h(px(6.)).w_full().bg(example_rgb(0xe5e5e5)).child(
                                div()
                                    .h_full()
                                    .w(gpui::relative(fill))
                                    .bg(example_rgb(0x171717)),
                            ),
                        ),
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .text_color(example_rgb(0x737373))
                            .child(format!(
                                "step {} of {} · {} · {status}",
                                sample.step() + 1,
                                STEPS.len(),
                                STEPS[sample.step()],
                            )),
                    )
                    .child(
                        Button::new("replay-sequence")
                            .h_9()
                            .px_3()
                            .border_1()
                            .border_color(example_rgb(0xd4d4d4))
                            .child("Replay")
                            .on_click(move |_, _, cx| {
                                _ = entity.update(cx, |this, cx| {
                                    this.sequence_generation += 1;
                                    cx.notify();
                                });
                            }),
                    ),
            )
    }
}

impl Render for MotionExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        activate_palette(window, cx);
        let entity = cx.entity().downgrade();
        let content = match self.demo {
            Demo::SlidingTime => self.sliding_time(window, cx).into_any_element(),
            Demo::Spring => self.spring_demo(window, cx).into_any_element(),
            Demo::Keyframes => self.keyframes_demo(window, cx).into_any_element(),
            Demo::Presence => self.presence_demo(window, cx).into_any_element(),
            Demo::Stagger => self.stagger_demo(window, cx).into_any_element(),
            Demo::Sequence => self.sequence_demo(window, cx).into_any_element(),
        };
        div()
            .size_full()
            .bg(example_canvas())
            .text_color(example_rgb(0x171717))
            .font_family(".SystemUIFont")
            .text_xs()
            .flex()
            .items_center()
            .justify_center()
            .p_6()
            .child(
                div()
                    .w(px(640.))
                    .max_w_full()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Motion examples"),
                    )
                    .child(div().flex().flex_wrap().gap_1().children(
                        Demo::ALL.into_iter().enumerate().map(|(ix, demo)| {
                            let entity = entity.clone();
                            Button::new(("demo", ix))
                                .h_8()
                                .px_3()
                                .border_1()
                                .border_color(example_rgb(0xd4d4d4))
                                .when(self.demo == demo, |this| {
                                    this.bg(example_rgb(0x171717))
                                        .text_color(example_rgb(0xffffff))
                                })
                                .child(demo.label())
                                .on_click(move |_, _, cx| {
                                    _ = entity.update(cx, |this, cx| {
                                        this.demo = demo;
                                        cx.notify();
                                    });
                                })
                        }),
                    ))
                    .child(
                        div()
                            .h(px(260.))
                            .pt_4()
                            .border_1()
                            .border_color(example_rgb(0xd4d4d4))
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .px_4()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(self.demo.label()),
                            )
                            .child(
                                div()
                                    .px_4()
                                    .text_sm()
                                    .text_color(example_rgb(0x737373))
                                    .child(self.demo.description()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(content),
                            ),
                    ),
            )
    }
}

fn clock_digit(value: i32, top: f32) -> gpui::Div {
    div()
        .absolute()
        .top(px(top))
        .w_full()
        .h(px(DIGIT_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .child(value.rem_euclid(10).to_string())
}

fn time_digits(minutes: u32) -> [u8; 4] {
    let hour = minutes / 60;
    let minute = minutes % 60;
    [
        (hour / 10) as u8,
        (hour % 10) as u8,
        (minute / 10) as u8,
        (minute % 10) as u8,
    ]
}

fn advance_digit(current: f32, digit: u8) -> f32 {
    let visible = current.floor() as i32 % 10;
    current + (i32::from(digit) - visible).rem_euclid(10) as f32
}

#[cfg(not(target_family = "wasm"))]
pub fn run() {
    let app: Application = gpui_platform::application();
    app.run(|cx: &mut App| {
        gpui_base::init(cx);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::centered(gpui::size(px(820.), px(620.)), cx)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| MotionExample::new()),
        )
        .expect("failed to open motion example");
        cx.activate(true);
    });
}

#[cfg(target_family = "wasm")]
pub fn run_embedded(app: Application) -> ApplicationHandle {
    app.run_embedded(|cx: &mut App| {
        gpui_base::init(cx);
        cx.text_system()
            .add_fonts(vec![Cow::Borrowed(
                include_bytes!("../../../story-web/fonts/Inter-Regular.ttf").as_slice(),
            )])
            .expect("failed to load motion example font");
        cx.open_window(WindowOptions::default(), |_, cx| {
            cx.new(|_| MotionExample::new())
        })
        .expect("failed to open motion example");
        cx.activate(true);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_time_digits() {
        assert_eq!(time_digits(START_MINUTES), [0, 8, 0, 0]);
        assert_eq!(time_digits(END_MINUTES), [2, 0, 0, 0]);
    }

    #[test]
    fn rolls_forward_across_zero() {
        assert_eq!(advance_digit(8., 0), 10.);
        assert_eq!(advance_digit(19., 2), 22.);
    }
}
