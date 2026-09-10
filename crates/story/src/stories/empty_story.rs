use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _, StyledExt as _, WindowExt as _,
    avatar::{Avatar, AvatarGroup},
    button::Button,
    empty::{
        Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant,
        EmptyTitle,
    },
    h_flex,
    input::{Input, InputEvent, InputState},
    link::Link,
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, SharedString, Styled as _, Subscription, Window, div, prelude::FluentBuilder as _,
    rems,
};

use crate::{Story, section};

pub struct EmptyStory {
    focus_handle: FocusHandle,
    project: Option<SharedString>,
    search: Entity<InputState>,
    page_search: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl EmptyStory {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search projects…")
                .default_value("Archive")
        });
        let subscription = cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        let page_search = cx.new(|cx| InputState::new(window, cx).placeholder("Search pages…"));

        Self {
            focus_handle: cx.focus_handle(),
            project: None,
            search,
            page_search,
            _subscriptions: vec![subscription],
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Story for EmptyStory {
    fn title() -> &'static str {
        "Empty"
    }

    fn description() -> &'static str {
        "Compose empty states with icons, avatars, actions, and custom content."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for EmptyStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EmptyStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search.read(cx).value().to_lowercase();
        let projects = ["Design system", "Website", "Mobile app"]
            .into_iter()
            .filter(|name| name.to_lowercase().contains(&query))
            .collect::<Vec<_>>();

        v_flex()
            .w_full()
            .gap_4()
            .child(
                section("Default")
                    .description("A named header, multiple actions, and an extra child below the content. Actions update this example.")
                    .child(
                        div()
                            .w_full()
                            .min_h_72()
                            .flex()
                            .map(|this| {
                                if let Some(project) = self.project.clone() {
                                    this.child(
                                        v_flex()
                                            .w_full()
                                            .items_center()
                                            .justify_center()
                                            .gap_4()
                                            .child(
                                                h_flex()
                                                    .gap_2()
                                                    .child(Icon::new(IconName::Folder).size_4())
                                                    .child(project),
                                            )
                                            .child(
                                                Button::new("empty-reset-project")
                                                    .outline()
                                                    .label("Reset example")
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.project = None;
                                                        cx.notify();
                                                    })),
                                            ),
                                    )
                                } else {
                                    this.child(
                                        Empty::new()
                                            .header(
                                                EmptyHeader::new()
                                                    .media(
                                                        EmptyMedia::new()
                                                            .with_variant(EmptyMediaVariant::Icon)
                                                            .child(Icon::new(IconName::Folder)),
                                                    )
                                                    .title(EmptyTitle::new().child("No projects yet"))
                                                    .description(
                                                        EmptyDescription::new().child(
                                                            "You haven't created any projects yet. Get started by creating your first project.",
                                                        ),
                                                    ),
                                            )
                                            .content(
                                                EmptyContent::new()
                                                    .flex_row()
                                                    .flex_wrap()
                                                    .justify_center()
                                                    .gap_2()
                                                    .child(
                                                        Button::new("empty-create-project")
                                                            .label("Create project")
                                                            .on_click(cx.listener(|this, _, _, cx| {
                                                                this.project = Some("Untitled project".into());
                                                                cx.notify();
                                                            })),
                                                    )
                                                    .child(
                                                        Button::new("empty-import-project")
                                                            .outline()
                                                            .label("Import sample")
                                                            .on_click(cx.listener(|this, _, _, cx| {
                                                                this.project = Some("Starter project".into());
                                                                cx.notify();
                                                            })),
                                                    ),
                                            )
                                            .child(
                                                Link::new("empty-learn-more")
                                                    .href("https://gpui-kit.com/docs/getting-started")
                                                    .text_sm()
                                                    .child("Learn more"),
                                            ),
                                    )
                                }
                            }),
                    ),
            )
            .child(
                section("Outline")
                    .description("Add a border through Styled; the component already supplies the dashed treatment.")
                    .child(
                        Empty::new()
                            .border_1()
                            .header(
                                EmptyHeader::new()
                                    .media(
                                        EmptyMedia::new()
                                            .with_variant(EmptyMediaVariant::Icon)
                                            .child(Icon::new(IconName::HardDrive)),
                                    )
                                    .title(EmptyTitle::new().child("Cloud storage is empty"))
                                    .description(
                                        EmptyDescription::new().child(
                                            "Upload files to your cloud storage to access them anywhere.",
                                        ),
                                    ),
                            )
                            .content(
                                EmptyContent::new().child(
                                    Button::new("empty-upload")
                                        .outline()
                                        .small()
                                        .label("Upload files…")
                                        .on_click(|_, window, cx| {
                                            window.open_dialog(cx, |dialog, _, _| {
                                                dialog.title("Upload files").child(
                                                    "Connect your file picker to this action to choose files for upload.",
                                                )
                                            });
                                        }),
                                ),
                            ),
                    ),
            )
            .child(
                section("Background")
                    .description("A semantic background can blend an empty state into its surrounding surface.")
                    .child(
                        Empty::new()
                            .min_h_64()
                            .bg(cx.theme().muted.opacity(0.3))
                            .header(
                                EmptyHeader::new()
                                    .media(
                                        EmptyMedia::new()
                                            .with_variant(EmptyMediaVariant::Icon)
                                            .child(Icon::new(IconName::Bell)),
                                    )
                                    .title(EmptyTitle::new().child("No notifications"))
                                    .description(
                                        EmptyDescription::new().child(
                                            "You're all caught up. New notifications will appear here.",
                                        ),
                                    ),
                            )
                            .content(
                                EmptyContent::new().child(
                                    Button::new("empty-refresh")
                                        .outline()
                                        .icon(Icon::new(IconName::RotateCw))
                                        .label("Refresh")
                                        .on_click(|_, window, cx| {
                                            window.push_notification("No new notifications", cx);
                                        }),
                                ),
                            ),
                    ),
            )
            .child(
                section("Avatar")
                    .description("Default media keeps the Avatar component's own size and appearance.")
                    .child(
                        Empty::new()
                            .header(
                                EmptyHeader::new()
                                    .media(
                                        EmptyMedia::new().child(
                                            Avatar::new()
                                                .name("Alex Morgan")
                                                .src("https://avatars.githubusercontent.com/u/5518?v=4"),
                                        ),
                                    )
                                    .title(EmptyTitle::new().child("Alex is offline"))
                                    .description(
                                        EmptyDescription::new().child(
                                            "You can leave a message for Alex to read when they're back.",
                                        ),
                                    ),
                            )
                            .content(
                                EmptyContent::new().child(
                                    Button::new("empty-message")
                                        .small()
                                        .label("Leave a message…")
                                        .on_click(|_, window, cx| {
                                            window.open_dialog(cx, |dialog, _, _| {
                                                dialog.title("Message Alex").child(
                                                    "Your message composer can open here while the empty state stays in the background.",
                                                )
                                            });
                                        }),
                                ),
                            ),
                    ),
            )
            .child(
                section("Avatar group")
                    .description("Compose AvatarGroup inside the same media slot; no new Empty variant is needed.")
                    .child(
                        Empty::new()
                            .header(
                                EmptyHeader::new()
                                    .media(
                                        EmptyMedia::new().child(
                                            AvatarGroup::new()
                                                .child(Avatar::new().name("Alex Morgan").src("https://avatars.githubusercontent.com/u/5518?v=4"))
                                                .child(Avatar::new().name("Taylor Lee").src("https://avatars.githubusercontent.com/u/28998859?v=4"))
                                                .child(Avatar::new().name("Sam Chen").src("https://avatars.githubusercontent.com/u/20092316?v=4")),
                                        ),
                                    )
                                    .title(EmptyTitle::new().child("No team members"))
                                    .description(
                                        EmptyDescription::new().child(
                                            "Invite your team to collaborate on this project.",
                                        ),
                                    ),
                            )
                            .content(
                                EmptyContent::new().child(
                                    Button::new("empty-invite")
                                        .small()
                                        .icon(Icon::new(IconName::Plus))
                                        .label("Invite members…")
                                        .on_click(|_, window, cx| {
                                            window.open_dialog(cx, |dialog, _, _| {
                                                dialog.title("Invite members").child(
                                                    "Your invitation form can compose the existing Input and Button components here.",
                                                )
                                            });
                                        }),
                                ),
                            ),
                    ),
            )
            .child(
                section("Search")
                    .description("The parent owns search state. Clear the query or search for Design, Website, or Mobile to show results.")
                    .v_flex()
                    .gap_4()
                    .child(
                        Input::new(&self.search)
                            .prefix(Icon::new(IconName::Search).size_4())
                            .cleanable(true),
                    )
                    .child(
                        div()
                            .w_full()
                            .min_h_48()
                            .flex()
                            .map(|this| {
                                if projects.is_empty() {
                                    this.child(
                                        Empty::new()
                                            .header(
                                                EmptyHeader::new()
                                                    .title(EmptyTitle::new().child("No projects found"))
                                                    .description(EmptyDescription::new().child(
                                                        "Try a different search or clear the query to see all projects.",
                                                    )),
                                            )
                                            .content(
                                                EmptyContent::new().child(
                                                    Button::new("empty-clear-search")
                                                        .outline()
                                                        .label("Clear search")
                                                        .on_click(cx.listener(|this, _, window, cx| {
                                                            this.search.update(cx, |state, cx| {
                                                                state.set_value("", window, cx);
                                                            });
                                                            this.search.focus_handle(cx).focus(window, cx);
                                                            cx.notify();
                                                        })),
                                                ),
                                            ),
                                    )
                                } else {
                                    this.child(
                                        v_flex()
                                            .w_full()
                                            .gap_2()
                                            .children(projects.into_iter().map(|project| {
                                                h_flex()
                                                    .gap_2()
                                                    .p_3()
                                                    .child(Icon::new(IconName::Folder).size_4())
                                                    .child(project)
                                            })),
                                    )
                                }
                            }),
                    ),
            )
            .child(
                section("Custom content")
                    .description("An existing Input can live in EmptyContent, with rich supporting content beneath it.")
                    .child(
                        Empty::new()
                            .header(
                                EmptyHeader::new()
                                    .title(EmptyTitle::new().child("Looking for a project?"))
                                    .description(
                                        EmptyDescription::new().child(
                                            "The page you're looking for doesn't exist. Try searching for another page.",
                                        ),
                                    ),
                            )
                            .content(
                                EmptyContent::new()
                                    .child(
                                        Input::new(&self.page_search)
                                            .prefix(Icon::new(IconName::Search).size_4())
                                            .cleanable(true),
                                    )
                                    .child(
                                        EmptyDescription::new().child(
                                            h_flex()
                                                .flex_wrap()
                                                .justify_center()
                                                .gap_1()
                                                .child("Need help?")
                                                .child(
                                                    Link::new("empty-documentation")
                                                        .href("https://gpui-kit.com/docs/getting-started")
                                                        .child("Read the documentation"),
                                                ),
                                        ),
                                    ),
                            ),
                    ),
            )
            .child(
                section("Constrained layout")
                    .description("Refine each named slot independently. Long text wraps in this compact, leading-aligned panel.")
                    .child(
                        div()
                            .w_full()
                            .max_w(rems(20.))
                            .child(
                                Empty::new()
                                    .border_1()
                                    .p_4()
                                    .items_start()
                                    .text_left()
                                    .header(
                                        EmptyHeader::new()
                                            .items_start()
                                            .media(
                                                EmptyMedia::new()
                                                    .with_variant(EmptyMediaVariant::Icon)
                                                    .child(Icon::new(IconName::Inbox)),
                                            )
                                            .title(EmptyTitle::new().child("还没有共享文件"))
                                            .description(EmptyDescription::new().child(
                                                "将文件添加到此项目后，团队成员就能在这里查看、讨论和继续编辑。较长的说明也应该完整显示。",
                                            )),
                                    )
                                    .content(
                                        EmptyContent::new().items_start().child(
                                            Button::new("empty-add-file")
                                                .outline()
                                                .small()
                                                .label("添加文件…")
                                                .on_click(|_, window, cx| {
                                                    window.open_dialog(cx, |dialog, _, _| {
                                                        dialog.title("添加文件").child("选择要与团队共享的文件。")
                                                    });
                                                }),
                                        ),
                                    ),
                            ),
                    ),
            )
            .child(
                section("Minimal")
                    .description("Media and content slots are optional.")
                    .child(
                        Empty::new().header(
                            EmptyHeader::new()
                                .title(EmptyTitle::new().child("No recent activity"))
                                .description(
                                    EmptyDescription::new()
                                        .child("New activity will appear here."),
                                ),
                        ),
                    ),
            )
    }
}
