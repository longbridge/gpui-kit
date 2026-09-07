use crate::*;
use gpui_kit::component::{
    Icon, IconName, ThemeStyled as _,
    button::{Button, ButtonVariants as _},
    command::{CommandEntry, CommandItem},
    h_flex,
    input::{Input, InputEvent, InputState},
    resizable::{h_resizable, resizable_panel},
    separator::Separator,
    sidebar::{Sidebar, SidebarHeader, SidebarMenu, SidebarMenuItem},
    status_bar::StatusBar,
    v_flex,
};

fn component_command(name: impl Into<SharedString>) -> CommandEntry {
    CommandItem::new().label(name).into()
}

fn find_story_index<'a>(
    groups: impl IntoIterator<Item = impl IntoIterator<Item = &'a str>>,
    name: &str,
) -> Option<(usize, usize)> {
    groups
        .into_iter()
        .enumerate()
        .find_map(|(group_ix, group)| {
            group
                .into_iter()
                .position(|story_name| story_name.eq_ignore_ascii_case(name))
                .map(|story_ix| (group_ix, story_ix))
        })
}

pub struct Gallery {
    stories: Vec<(&'static str, Vec<Entity<StoryContainer>>)>,
    active_group_index: Option<usize>,
    active_index: Option<usize>,
    collapsed: bool,
    embedded: bool,
    search_input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl Gallery {
    pub fn new(init_story: Option<&str>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_with_mode(init_story, false, window, cx)
    }

    fn new_with_mode(
        init_story: Option<&str>,
        embedded: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search…"));
        let _subscriptions = vec![cx.subscribe(&search_input, |this, _, e, cx| match e {
            InputEvent::Change => {
                this.active_group_index = Some(0);
                this.active_index = Some(0);
                cx.notify()
            }
            _ => {}
        })];
        let stories = vec![(
            "",
            vec![
                StoryContainer::panel::<WelcomeStory>(window, cx),
                StoryContainer::panel::<AccordionStory>(window, cx),
                StoryContainer::panel::<AlertStory>(window, cx),
                StoryContainer::panel::<AlertDialogStory>(window, cx),
                StoryContainer::panel::<AttachmentStory>(window, cx),
                StoryContainer::panel::<AvatarStory>(window, cx),
                StoryContainer::panel::<BadgeStory>(window, cx),
                StoryContainer::panel::<BreadcrumbStory>(window, cx),
                StoryContainer::panel::<BubbleStory>(window, cx),
                StoryContainer::panel::<ButtonStory>(window, cx),
                StoryContainer::panel::<CalendarStory>(window, cx),
                StoryContainer::panel::<ChartStory>(window, cx),
                StoryContainer::panel::<CheckboxStory>(window, cx),
                StoryContainer::panel::<ClipboardStory>(window, cx),
                StoryContainer::panel::<CollapsibleStory>(window, cx),
                StoryContainer::panel::<ColorPickerStory>(window, cx),
                StoryContainer::panel::<ComboboxStory>(window, cx),
                StoryContainer::panel::<CommandStory>(window, cx),
                StoryContainer::panel::<DataTableStory>(window, cx),
                StoryContainer::panel::<DatePickerStory>(window, cx),
                StoryContainer::panel::<DescriptionListStory>(window, cx),
                StoryContainer::panel::<DialogStory>(window, cx),
                StoryContainer::panel::<DockStory>(window, cx),
                StoryContainer::panel::<DropdownButtonStory>(window, cx),
                StoryContainer::panel::<EditorStory>(window, cx),
                StoryContainer::panel::<FormStory>(window, cx),
                StoryContainer::panel::<GroupBoxStory>(window, cx),
                StoryContainer::panel::<HoverCardStory>(window, cx),
                StoryContainer::panel::<IconStory>(window, cx),
                StoryContainer::panel::<ImageStory>(window, cx),
                StoryContainer::panel::<InputStory>(window, cx),
                StoryContainer::panel::<KbdStory>(window, cx),
                StoryContainer::panel::<LabelStory>(window, cx),
                StoryContainer::panel::<ListStory>(window, cx),
                StoryContainer::panel::<MenuStory>(window, cx),
                StoryContainer::panel::<MarkerStory>(window, cx),
                StoryContainer::panel::<MessageStory>(window, cx),
                StoryContainer::panel::<MessageScrollerStory>(window, cx),
                StoryContainer::panel::<NativeMenuStory>(window, cx),
                StoryContainer::panel::<NotificationStory>(window, cx),
                StoryContainer::panel::<NumberInputStory>(window, cx),
                StoryContainer::panel::<OtpInputStory>(window, cx),
                StoryContainer::panel::<PaginationStory>(window, cx),
                StoryContainer::panel::<PopoverStory>(window, cx),
                StoryContainer::panel::<ProgressStory>(window, cx),
                StoryContainer::panel::<RadioStory>(window, cx),
                StoryContainer::panel::<RatingStory>(window, cx),
                StoryContainer::panel::<ResizableStory>(window, cx),
                StoryContainer::panel::<ScrollbarStory>(window, cx),
                StoryContainer::panel::<SelectStory>(window, cx),
                StoryContainer::panel::<SeparatorStory>(window, cx),
                StoryContainer::panel::<SettingsStory>(window, cx),
                #[cfg(not(target_family = "wasm"))]
                StoryContainer::panel::<ShellStory>(window, cx),
                StoryContainer::panel::<SheetStory>(window, cx),
                StoryContainer::panel::<ShimmerStory>(window, cx),
                StoryContainer::panel::<SidebarStory>(window, cx),
                StoryContainer::panel::<SkeletonStory>(window, cx),
                StoryContainer::panel::<SliderStory>(window, cx),
                StoryContainer::panel::<SpinnerStory>(window, cx),
                StoryContainer::panel::<StatusBarStory>(window, cx),
                StoryContainer::panel::<StepperStory>(window, cx),
                StoryContainer::panel::<SwitchStory>(window, cx),
                StoryContainer::panel::<TableStory>(window, cx),
                StoryContainer::panel::<TabsStory>(window, cx),
                StoryContainer::panel::<TagStory>(window, cx),
                StoryContainer::panel::<TextareaStory>(window, cx),
                StoryContainer::panel::<ThemeColorsStory>(window, cx),
                StoryContainer::panel::<ToggleStory>(window, cx),
                StoryContainer::panel::<TooltipStory>(window, cx),
                StoryContainer::panel::<TreeStory>(window, cx),
                StoryContainer::panel::<VirtualListStory>(window, cx),
            ],
        )];

        let mut this = Self {
            search_input,
            stories,
            active_group_index: Some(0),
            active_index: Some(0),
            collapsed: false,
            embedded,
            _subscriptions,
        };

        if let Some(init_story) = init_story {
            this.set_active_story(init_story, window, cx);
        }

        this
    }

    fn set_active_story(&mut self, name: &str, window: &mut Window, cx: &mut App) {
        let name = name.trim().to_string();
        let exact_index = self
            .stories
            .iter()
            .flat_map(|(_, stories)| stories)
            .filter(|story| {
                story
                    .read(cx)
                    .name
                    .to_lowercase()
                    .contains(&name.to_lowercase())
            })
            .position(|story| story.read(cx).name.eq_ignore_ascii_case(&name));
        self.search_input.update(cx, |this, cx| {
            this.set_value(&name, window, cx);
        });
        self.active_group_index = Some(0);
        self.active_index = Some(exact_index.unwrap_or(0));
    }

    pub(crate) fn command_entries(&self, cx: &App) -> Vec<CommandEntry> {
        self.stories
            .iter()
            .flat_map(|(_, stories)| stories)
            .map(|story| component_command(story.read(cx).name.clone()))
            .collect()
    }

    pub(crate) fn select_story(
        &mut self,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let names = self
            .stories
            .iter()
            .map(|(_, stories)| {
                stories
                    .iter()
                    .map(|story| story.read(cx).name.clone())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let Some((group_ix, story_ix)) = find_story_index(
            names
                .iter()
                .map(|group| group.iter().map(|name| name.as_ref())),
            name,
        ) else {
            return false;
        };

        self.search_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.active_group_index = Some(group_ix);
        self.active_index = Some(story_ix);
        cx.notify();
        true
    }

    pub(crate) fn select_story_index(
        &mut self,
        index: gpui_kit::component::IndexPath,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if index.section != 0 {
            return false;
        }
        let Some(name) = self
            .stories
            .iter()
            .flat_map(|(_, stories)| stories)
            .nth(index.row)
            .map(|story| story.read(cx).name.clone())
        else {
            return false;
        };
        self.select_story(&name, window, cx)
    }

    pub fn view(init_story: Option<&str>, window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(init_story, window, cx))
    }

    pub fn embedded_view(story: &str, window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new_with_mode(Some(story), true, window, cx))
    }
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search_input.read(cx).value().trim().to_lowercase();

        let stories: Vec<_> = self
            .stories
            .iter()
            .filter_map(|(name, items)| {
                let filtered_items: Vec<_> = items
                    .iter()
                    .filter(|story| story.read(cx).name.to_lowercase().contains(&query))
                    .cloned()
                    .collect();

                if !filtered_items.is_empty() {
                    Some((name, filtered_items))
                } else {
                    None
                }
            })
            .collect();

        let active_group = self.active_group_index.and_then(|index| stories.get(index));
        let active_story = self
            .active_index
            .and(active_group)
            .and_then(|group| group.1.get(self.active_index.unwrap()));
        let (story_name, description) =
            if let Some(story) = active_story.as_ref().map(|story| story.read(cx)) {
                (story.name.clone(), story.description.clone())
            } else {
                ("".into(), "".into())
            };

        let current_story = story_name.clone();
        let total_components: usize = self.stories.iter().map(|(_, items)| items.len()).sum();

        if self.embedded {
            return div()
                .id("embedded-story")
                .size_full()
                .when_some(active_story, |this, story| this.child(story.clone()))
                .into_any_element();
        }

        let body = h_resizable("gallery-container")
            .child(
                resizable_panel()
                    .size(px(255.))
                    .size_range(px(200.)..px(320.))
                    .child(
                        Sidebar::new("gallery-sidebar")
                            .w(relative(1.))
                            .border_0()
                            .collapsed(self.collapsed)
                            .header(
                                v_flex()
                                    .w_full()
                                    .gap_4()
                                    .child(
                                        SidebarHeader::new()
                                            .w_full()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded(cx.theme().radius_lg)
                                                    .bg(cx.theme().primary)
                                                    .text_color(cx.theme().primary_foreground)
                                                    .size_8()
                                                    .flex_shrink_0()
                                                    .when(!self.collapsed, |this| {
                                                        this.child(Icon::new(
                                                            IconName::GalleryVerticalEnd,
                                                        ))
                                                    })
                                                    .when(self.collapsed, |this| {
                                                        this.size_4()
                                                            .bg(cx.theme().transparent)
                                                            .text_color(cx.theme().foreground)
                                                            .child(Icon::new(
                                                                IconName::GalleryVerticalEnd,
                                                            ))
                                                    }),
                                            )
                                            .when(!self.collapsed, |this| {
                                                this.child(
                                                    v_flex()
                                                        .gap_0()
                                                        .text_sm()
                                                        .flex_1()
                                                        .line_height(relative(1.25))
                                                        .overflow_hidden()
                                                        .text_ellipsis()
                                                        .child("GPUI Component")
                                                        .child(
                                                            div()
                                                                .text_color(
                                                                    cx.theme().muted_foreground,
                                                                )
                                                                .child("Component showcase")
                                                                .text_xs(),
                                                        ),
                                                )
                                            }),
                                    )
                                    .child(
                                        div()
                                            .bg(cx.theme().sidebar_accent)
                                            .rounded_full_style(cx)
                                            .px_1()
                                            .flex_1()
                                            .mx_1()
                                            .child(
                                                Input::new(&self.search_input)
                                                    .appearance(false)
                                                    .cleanable(true),
                                            ),
                                    ),
                            )
                            .children(stories.clone().into_iter().enumerate().map(
                                |(group_ix, (_, sub_stories))| {
                                    SidebarMenu::new().children(sub_stories.iter().enumerate().map(
                                        |(ix, story)| {
                                            SidebarMenuItem::new(story.read(cx).name.clone())
                                                .active(
                                                    self.active_group_index == Some(group_ix)
                                                        && self.active_index == Some(ix),
                                                )
                                                .on_click(cx.listener(
                                                    move |this, _: &ClickEvent, _, cx| {
                                                        this.active_group_index = Some(group_ix);
                                                        this.active_index = Some(ix);
                                                        cx.notify();
                                                    },
                                                ))
                                        },
                                    ))
                                },
                            )),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .h_full()
                    .overflow_x_hidden()
                    .child(
                        h_flex()
                            .id("header")
                            .p_4()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .justify_between()
                            .items_start()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(div().text_2xl().font_semibold().child(story_name))
                                    .child(
                                        div()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(description),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("story")
                            .flex_1()
                            .overflow_y_scroll()
                            .when_some(active_story, |this, active_story| {
                                this.child(active_story.clone())
                            }),
                    )
                    .into_any_element(),
            );

        v_flex()
            .size_full()
            .child(div().flex_1().min_h_0().child(body))
            .child(
                StatusBar::new()
                    .child(Icon::new(IconName::GalleryVerticalEnd).xsmall())
                    .child(format!("{total_components} components"))
                    .child(Separator::vertical())
                    .when(!current_story.is_empty(), |this| {
                        this.child(current_story.clone())
                    })
                    .right(cx.theme().theme_name().clone())
                    .right(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .right(
                        Button::new("assistant")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Github)
                            .tooltip("GPUI Component GitHub repository")
                            .on_click(|_, _, cx| {
                                cx.open_url("https://github.com/longbridge/gpui-kit")
                            }),
                    ),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{component_command, find_story_index};
    use gpui_kit::component::command::CommandEntry;

    #[test]
    fn component_command_uses_story_name() {
        let entry = component_command("Command");

        assert!(matches!(entry, CommandEntry::Item(_)));
    }

    #[test]
    fn story_index_matches_names_without_filtering() {
        let groups = [vec!["Welcome", "Button"], vec!["Command", "Dialog"]];

        assert_eq!(
            find_story_index(groups.iter().map(|group| group.iter().copied()), "command"),
            Some((1, 0))
        );
        assert_eq!(
            find_story_index(groups.iter().map(|group| group.iter().copied()), "missing"),
            None
        );
    }
}
