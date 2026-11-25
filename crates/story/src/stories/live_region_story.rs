use gpui_kit::component::{button::Button, h_flex, live_region::LiveRegion, v_flex};
use gpui_kit::*;

use crate::section;

pub struct LiveRegionStory {
    focus_handle: gpui_kit::FocusHandle,
    saved: usize,
    failed: bool,
}

impl super::Story for LiveRegionStory {
    fn title() -> &'static str {
        "LiveRegion"
    }

    fn description() -> &'static str {
        "Announce status changes to screen readers without moving focus."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl LiveRegionStory {
    pub(crate) fn new(_: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            saved: 0,
            failed: false,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Focusable for LiveRegionStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for LiveRegionStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status: SharedString = match self.saved {
            0 => "Not saved".into(),
            1 => "Saved 1 time".into(),
            n => format!("Saved {n} times").into(),
        };
        let error: SharedString = if self.failed {
            "Could not save: disk full".into()
        } else {
            SharedString::default()
        };

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .child(
                section("Polite")
                    .description(
                        "Focus stays on the button; the status is announced after current speech.",
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(Button::new("live-region-save").label("Save").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.saved += 1;
                                    this.failed = false;
                                    cx.notify();
                                }),
                            ))
                            .child(LiveRegion::new("live-region-status", status)),
                    ),
            )
            .child(
                section("Assertive")
                    .description(
                        "Interrupts current speech. Reserve for errors that block the task.",
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(Button::new("live-region-fail").label("Fail Save").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.failed = true;
                                    cx.notify();
                                }),
                            ))
                            .child(LiveRegion::new("live-region-error", error).assertive()),
                    ),
            )
    }
}
