use gpui_kit::component::{
    IconName, Sizable as _, Size, alert::Alert, dock::PanelControl, text::markdown, v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Window,
};

use crate::{ChangeStorySize, section, story_toolbar};

pub struct AlertStory {
    size: Size,
    banner_visible: bool,
    focus_handle: gpui_kit::FocusHandle,
}

impl AlertStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            size: Size::default(),
            banner_visible: true,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for AlertStory {
    fn title() -> &'static str {
        "Alert"
    }

    fn description() -> &'static str {
        "Communicate important status changes without interrupting the user's workflow."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for AlertStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AlertStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .child(story_toolbar(self.size))
            .child(
                section("Default")
                    .description("Title, icon, and rich text content.")
                    .w_2_3()
                    .child(
                    Alert::new(
                        "alert-default",
                        markdown(
                            "Your workspace is ready for the team.\n\
                            - **12 members** have access\n\
                            - Billing remains with the workspace owner",
                        ),
                    )
                    .with_size(self.size)
                    .title("Workspace settings saved"),
                ),
            )
            .child(
                section("Variants")
                    .description("Info, success, warning, and error states.")
                    .w_2_3()
                    .child(
                    v_flex()
                        .w_full()
                        .gap_3()
                        .child(
                            Alert::info("info1", "Maintenance starts Friday at 22:00 UTC.")
                                .with_size(self.size)
                                .title("Scheduled maintenance")
                                .on_close(cx.listener(|_, _, _, _| {
                                    println!("Info alert closed");
                                })),
                        )
                        .child(
                            Alert::success(
                                "success-1",
                                "The transfer is queued and usually settles within one business day.",
                            )
                            .with_size(self.size)
                            .title("Transfer submitted"),
                        )
                        .child(
                            Alert::warning(
                                "warning-1",
                                "Two teammates still use recovery codes generated more than a year ago.\n\
                            Ask them to generate a fresh set in Security settings.",
                            )
                            .with_size(self.size),
                        )
                        .child(
                            Alert::error(
                                "error-1",
                                markdown(
                                    "Please verify your billing information and try again.\n\
                            - Check your card details\n\
                            - Ensure sufficient funds\n\
                            - Verify billing address",
                                ),
                            )
                            .with_size(self.size)
                            .title("Unable to process your payment."),
                        ),
                ),
            )
            .child(
                section("Banner")
                    .description("Full-width and closable alerts.")
                    .w_2_3()
                    .child(
                    v_flex()
                        .w_full()
                        .gap_2()
                        .child(
                            Alert::new(
                                "banner-1",
                                "Reporting is read-only while the nightly ledger closes.",
                            )
                            .banner()
                            .on_close(cx.listener(|this, _, _, cx| {
                                this.banner_visible = !this.banner_visible;
                                cx.notify();
                            }))
                            .visible(self.banner_visible)
                            .with_size(self.size),
                        )
                        .child(
                            Alert::info(
                                "banner-info",
                                "A new desktop update will install after you restart.",
                            )
                            .banner()
                            .with_size(self.size),
                        )
                        .child(
                            Alert::success(
                                "banner-success",
                                "All 1,284 records finished importing.",
                            )
                            .banner()
                            .with_size(self.size),
                        )
                        .child(
                            Alert::warning(
                                "banner-warning",
                                "Your API key expires in 6 days. Rotate it before August 19.",
                            )
                            .banner()
                            .with_size(self.size),
                        )
                        .child(
                            Alert::error(
                                "banner-error",
                                "Live updates are disconnected. Changes may be delayed.",
                            )
                            .banner()
                            .with_size(self.size),
                        ),
                ),
            )
            .child(
                section("Custom icon")
                    .description("Custom icon and long content.")
                    .w_2_3()
                    .child(
                    Alert::new(
                        "other-1",
                        "The quarterly planning review overlaps with the APAC operations call. \
                    Move one event or invite another owner before sending the agenda.",
                    )
                    .title("Two events overlap by 30 minutes")
                    .with_size(self.size)
                    .icon(IconName::Calendar),
                ),
            )
    }
}
