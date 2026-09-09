use gpui_kit::{
    Anchor, App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement as _,
    IntoElement, ParentElement, Render, Styled, Window,
};

use gpui_kit::component::{
    ActiveTheme, Theme, WindowExt as _,
    button::{Button, ButtonVariants},
    h_flex,
    menu::PopupMenuItem,
    notification::{Notification, NotificationType},
    text::markdown,
    v_flex,
};

use crate::{section, story_toolbar_group};

const NOTIFICATION_MARKDOWN: &str = r#"
This is a custom notification.
- List item 1
- List item 2
- [Click here](https://github.com/longbridge/gpui-kit)
"#;

pub struct NotificationStory {
    focus_handle: FocusHandle,
}

impl super::Story for NotificationStory {
    fn title() -> &'static str {
        "Notification"
    }

    fn description() -> &'static str {
        "Show transient feedback without interrupting the current task."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl NotificationStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for NotificationStory {
    fn focus_handle(&self, _cx: &gpui_kit::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for NotificationStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        const ANCHORS: [Anchor; 8] = [
            Anchor::TopLeft,
            Anchor::TopCenter,
            Anchor::TopRight,
            Anchor::LeftCenter,
            Anchor::RightCenter,
            Anchor::BottomLeft,
            Anchor::BottomCenter,
            Anchor::BottomRight,
        ];

        let view = cx.entity();

        v_flex()
            .id("notification-story")
            .track_focus(&self.focus_handle)
            .size_full()
            .gap_3()
            .child(
                story_toolbar_group()
                    .dropdown_child(
                        Button::new("placement").label(format!(
                            "Placement: {:?}",
                            cx.theme().notification.placement
                        )),
                        {
                            let view = view.clone();
                            move |menu, window, cx| {
                                let menu = ANCHORS.into_iter().fold(menu, |menu, placement| {
                                    menu.item(
                                        PopupMenuItem::new(format!("{:?}", placement))
                                            .checked(cx.theme().notification.placement == placement)
                                            .on_click(window.listener_for(
                                                &view,
                                                move |_, _, _, cx| {
                                                    Theme::global_mut(cx).notification.placement =
                                                        placement;
                                                    cx.notify();
                                                },
                                            )),
                                    )
                                });
                                menu
                            }
                        },
                    )
                    .dropdown_child(
                        Button::new("max-items")
                            .label(format!("Max items: {}", cx.theme().notification.max_items)),
                        move |menu, window, cx| {
                            const MAX_ITEMS: [usize; 5] = [1, 2, 3, 5, 10];
                            MAX_ITEMS.into_iter().fold(menu, |menu, max_items| {
                                menu.item(
                                    PopupMenuItem::new(format!("{}", max_items))
                                        .checked(cx.theme().notification.max_items == max_items)
                                        .on_click(window.listener_for(
                                            &view,
                                            move |_, _, _, cx| {
                                                Theme::global_mut(cx).notification.max_items =
                                                    max_items;
                                                cx.notify();
                                            },
                                        )),
                                )
                            })
                        },
                    ),
            )
            .child(
                section("Default")
                    .description("Show a short message.")
                    .child(
                        Button::new("show-notify-0")
                            .outline()
                            .label("Show Notification")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification("This is a notification.", cx)
                            })),
                    ),
            )
            .child(
                section("Types")
                    .description("Use semantic treatments for common outcomes.")
                    .child(
                        Button::new("show-notify-info")
                            .info()
                            .label("Info")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    (
                                        NotificationType::Info,
                                        "You have been saved file successfully.",
                                    ),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new("show-notify-success")
                            .success()
                            .label("Success")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    (
                                        NotificationType::Success,
                                        "We have received your payment successfully.",
                                    ),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new("show-notify-warning")
                            .warning()
                            .label("Warning")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    (
                                        NotificationType::Warning,
                                        "The network is not stable, please check your connection.",
                                    ),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new("show-notify-error")
                            .danger()
                            .label("Error")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    (
                                        NotificationType::Error,
                                        "There have some error occurred. Please try again later.",
                                    ),
                                    cx,
                                )
                            })),
                    ),
            )
            .child(
                section("Title and description")
                    .description("Pair a concise title with supporting detail.")
                    .child(
                        Button::new("show-typed-info")
                            .info()
                            .label("Info")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    Notification::info(
                                        "Your changes have been saved to the cloud \
                                        and will sync across all of your devices.",
                                    )
                                    .title("All changes saved"),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new("show-typed-success")
                            .success()
                            .label("Success")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    Notification::success(
                                        "Your payment of $99.00 was processed and a \
                                        receipt has been emailed to you.",
                                    )
                                    .title("Payment received"),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new("show-typed-warning")
                            .warning()
                            .label("Warning")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    Notification::warning(
                                        "Your network connection is unstable. \
                                        Some changes may take longer to save.",
                                    )
                                    .title("Connection unstable"),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new("show-typed-error")
                            .danger()
                            .label("Error")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    Notification::error(
                                        "We couldn't reach the server. Check your \
                                        internet connection and try again.",
                                    )
                                    .title("Request failed"),
                                    cx,
                                )
                            })),
                    ),
            )
            .child(
                section("Unique")
                    .description("Replace duplicate notifications by type.")
                    .child(
                        Button::new("show-notify-unique")
                            .outline()
                            .label("Unique Notification")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    Notification::info("This is a unique notification.")
                                        .id::<NotificationStory>()
                                        .message("This is a unique notification.")
                                        .on_close(|_, _| {
                                            println!("Notification closed");
                                        }),
                                    cx,
                                )
                            })),
                    ),
            )
            .child(
                section("Keyed")
                    .description("Keep separate unique notifications with keys.")
                    .child(
                        h_flex()
                            .gap_3()
                            .child(
                                Button::new("show-notify-unique-key0")
                                    .outline()
                                    .label("A Notification")
                                    .on_click(cx.listener(|_, _, window, cx| {
                                        window.push_notification(
                                            Notification::info("This is A unique notification.")
                                                .id1::<NotificationStory>(1),
                                            cx,
                                        )
                                    })),
                            )
                            .child(
                                Button::new("show-notify-unique-key1")
                                    .outline()
                                    .label("B Notification")
                                    .on_click(cx.listener(|_, _, window, cx| {
                                        window.push_notification(
                                            Notification::info("This is B unique notification.")
                                                .id1::<NotificationStory>(2),
                                            cx,
                                        )
                                    })),
                            ),
                    ),
            )
            .child(
                section("Action")
                    .description("Add an inline action to the notification.")
                    .child(
                        Button::new("show-notify-with-title")
                            .outline()
                            .label("Notification with Title")
                            .on_click(cx.listener(|_, _, window, cx| {
                                struct TestNotification;

                                window.push_notification(
                                    Notification::new()
                                        .id::<TestNotification>()
                                        .title("Uh oh! Something went wrong.")
                                        .message("There was a problem with your request.")
                                        .action(|_, _, cx| {
                                            Button::new("try-again")
                                                .primary()
                                                .label("Retry")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    println!(
                                                        "You have clicked the try again action."
                                                    );
                                                    this.dismiss(window, cx);
                                                }))
                                        })
                                        .on_click(cx.listener(|_, _, _, cx| {
                                            println!("Notification clicked");
                                            cx.notify();
                                        })),
                                    cx,
                                )
                            })),
                    ),
            )
            .child(
                section("Lifecycle")
                    .description("Handle body clicks and close events independently.")
                    .child(
                        Button::new("show-notify-click-close")
                            .outline()
                            .label("Click vs Close")
                            .on_click(cx.listener(|_, _, window, cx| {
                                struct ClickCloseNotification;

                                window.push_notification(
                                    Notification::info(
                                        "Click the body to fire on_click; click the X to close. \
                                    Watch the console.",
                                    )
                                    .id::<ClickCloseNotification>()
                                    .title("on_click vs on_close")
                                    .autohide(false)
                                    .on_click(|_, _, _| {
                                        println!("[notification] on_click fired");
                                    })
                                    .on_close(|_, _| {
                                        println!("[notification] on_close fired");
                                    }),
                                    cx,
                                )
                            })),
                    ),
            )
            .child(
                section("Placement per notification")
                    .description("Override the global placement for a single notification.")
                    .children(ANCHORS.into_iter().map(|placement| {
                        Button::new(format!("show-notify-{placement:?}"))
                            .outline()
                            .label(format!("{placement:?}"))
                            .on_click(cx.listener(move |_, _, window, cx| {
                                window.push_notification(
                                    Notification::info(format!(
                                        "This notification is at {placement:?}."
                                    ))
                                    .placement(placement),
                                    cx,
                                )
                            }))
                    })),
            )
            .child({
                struct SystemNotificationKind;

                section("System notification")
                    .description(
                        "Deliver to the OS notification center; click the system \
                        notification to refocus the app. macOS shows them only when \
                        running from a bundled .app; Windows requires \
                        cx.set_app_identity().",
                    )
                    .child(
                        Button::new("show-notify-system")
                            .outline()
                            .label("System only")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    Notification::info(
                                        "Delivered straight to the notification center.",
                                    )
                                    .id::<SystemNotificationKind>()
                                    .title("Build finished")
                                    .system()
                                    .on_click(|_, _, _| {
                                        println!("[notification] system notification clicked");
                                    }),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new("show-notify-in-app-and-system")
                            .outline()
                            .label("In-app and system")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    Notification::info(
                                        "Shown as a toast and in the notification center.",
                                    )
                                    .id::<SystemNotificationKind>()
                                    .title("Build finished")
                                    .in_app_and_system()
                                    .autohide(false)
                                    .on_click(|_, _, _| {
                                        println!("[notification] system notification clicked");
                                    }),
                                    cx,
                                )
                            })),
                    )
            })
            .child(
                section("Custom content")
                    .description("Render application-owned notification content.")
                    .child(
                        Button::new("show-notify-custom")
                            .outline()
                            .label("Show Custom Notification")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    Notification::new().content(|_, _, _| {
                                        markdown(NOTIFICATION_MARKDOWN).into_any_element()
                                    }),
                                    cx,
                                )
                            })),
                    ),
            )
            .child({
                struct ManualOpenNotification;

                section("Manual close")
                    .description("Keep a notification visible until it is dismissed.")
                    .child(
                        Button::new("manual-open-notify")
                            .outline()
                            .label("Show")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    Notification::new()
                                        .id::<ManualOpenNotification>()
                                        .message(
                                            "You can close this notification by \
                                            clicking the Close button.",
                                        )
                                        .autohide(false),
                                    cx,
                                );
                            })),
                    )
                    .child(
                        Button::new("manual-close-notify")
                            .outline()
                            .label("Dismiss All")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.remove_notification::<ManualOpenNotification>(cx);
                            })),
                    )
            })
    }
}
