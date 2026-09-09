use gpui_kit::component::{
    ActiveTheme, IconName, Sizable, Size, StyledExt,
    button::Button,
    h_flex,
    progress::{Progress, ProgressCircle},
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde::Deserialize;
use std::time::Duration;

use crate::{ChangeStorySize, section, story_toolbar};

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = progress_story, no_json)]
enum ProgressAction {
    SetValue(f32),
    ToggleLoading,
}

pub struct ProgressStory {
    focus_handle: gpui_kit::FocusHandle,
    value: f32,
    loading: bool,
    size: Size,
    _task: Option<Task<()>>,
}

impl super::Story for ProgressStory {
    fn title() -> &'static str {
        "Progress"
    }

    fn description() -> &'static str {
        "Show task completion with determinate or loading indicators."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl ProgressStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            value: 25.,
            loading: false,
            size: Size::Medium,
            _task: None,
        }
    }

    pub fn set_value(&mut self, value: f32) {
        self.value = value;
    }

    fn start_animation(&mut self, cx: &mut Context<Self>) {
        self.value = 0.;

        self._task = Some(cx.spawn({
            let entity = cx.entity();
            async move |_, cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(15))
                        .await;

                    let mut need_break = false;
                    _ = entity.update(cx, |this, cx| {
                        this.value = (this.value + 2.).min(100.);
                        cx.notify();

                        if this.value >= 100. {
                            this._task = None;
                            need_break = true;
                        }
                    });

                    if need_break {
                        break;
                    }
                }
            }
        }));
    }
}

impl Focusable for ProgressStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ProgressStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .on_action(cx.listener(|this, action: &ProgressAction, _, cx| {
                match action {
                    ProgressAction::SetValue(value) => this.set_value(*value),
                    ProgressAction::ToggleLoading => this.loading = !this.loading,
                }
                cx.notify();
            }))
            .child(
                story_toolbar(self.size)
                    .dropdown_child(
                        Button::new("progress-value").label(format!("Value: {}%", self.value)),
                        {
                            let value = self.value;
                            move |menu, _, _| {
                                [0., 25., 75., 100.].into_iter().fold(menu, |menu, preset| {
                                    menu.menu_with_check(
                                        format!("{}%", preset),
                                        value == preset,
                                        Box::new(ProgressAction::SetValue(preset)),
                                    )
                                })
                            }
                        },
                    )
                    .dropdown_child(Button::new("progress-options").label("Options"), {
                        let loading = self.loading;
                        move |menu, _, _| {
                            menu.menu_with_check(
                                "Loading",
                                loading,
                                Box::new(ProgressAction::ToggleLoading),
                            )
                        }
                    })
                    .child(
                        Button::new("progress-play")
                            .icon(IconName::Play)
                            .on_click(cx.listener(|this, _, _, cx| this.start_animation(cx))),
                    ),
            )
            .child(
                section("Upload")
                    .description("Pair progress with a clear label, value, and status.")
                    .w(px(560.))
                    .items_center()
                    .child(
                        v_flex()
                            .w(px(400.))
                            .gap_3()
                            .p_4()
                            .rounded(cx.theme().radius_lg)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(div().font_medium().child("Uploading design-assets.zip"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!("{}%", self.value)),
                                    ),
                            )
                            .child(
                                Progress::new("upload-progress")
                                    .value(self.value)
                                    .loading(self.loading),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("24.8 MB of 96 MB")
                                    .child(if self.loading {
                                        "Calculating…"
                                    } else {
                                        "About 1 min left"
                                    }),
                            ),
                    ),
            )
            .child(
                section("Circular")
                    .description("Use a compact radial indicator for focused tasks.")
                    .w(px(560.))
                    .items_center()
                    .child(
                        h_flex()
                            .w(px(400.))
                            .items_center()
                            .gap_5()
                            .p_4()
                            .rounded(cx.theme().radius_lg)
                            .bg(cx.theme().muted.opacity(0.4))
                            .child(
                                ProgressCircle::new("analysis-progress")
                                    .with_size(self.size)
                                    .value(self.value)
                                    .loading(self.loading)
                                    .size_20()
                                    .when(!self.loading, |this| {
                                        this.child(
                                            div()
                                                .size_full()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .font_semibold()
                                                .child(format!("{}%", self.value)),
                                        )
                                    }),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(div().font_medium().child("Analyzing project"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(if self.loading {
                                                "Preparing analysis…"
                                            } else {
                                                "Scanning components and dependencies."
                                            }),
                                    ),
                            ),
                    ),
            )
    }
}
