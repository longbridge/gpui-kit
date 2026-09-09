use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement as _, Render, Styled as _, Window, div,
};

use gpui_kit::component::{
    IconName, Sizable, Size, StyledExt,
    button::{Toggle, ToggleGroup, ToggleVariants},
    h_flex, v_flex,
};

use crate::{ChangeStorySize, section, story_toolbar};

pub struct ToggleStory {
    focus_handle: FocusHandle,
    single_toggle: usize,
    checked: Vec<bool>,
    size: Size,
}

impl ToggleStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            single_toggle: 1,
            checked: vec![false; 12],
            size: Size::Medium,
        })
    }
}

impl super::Story for ToggleStory {
    fn title() -> &'static str {
        "Toggle"
    }

    fn description() -> &'static str {
        "Turn an option on or off, alone or in a group."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ToggleStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ToggleStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .child(story_toolbar(self.size))
            .child(
                section("Default")
                    .description("Text and icon toggles with clear selected states.")
                    .w_128()
                    .v_flex()
                    .items_center()
                    .gap_3()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Toggle::new("preview")
                                    .label("Preview")
                                    .with_size(self.size)
                                    .checked(self.single_toggle == 1)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.single_toggle = usize::from(*checked);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Toggle::new("favorite")
                                    .icon(IconName::Star)
                                    .with_size(self.size)
                                    .checked(self.checked[0])
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.checked[0] = *checked;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                section("Variants")
                    .description("Ghost and outline treatments for different surfaces.")
                    .w_128()
                    .v_flex()
                    .items_center()
                    .gap_4()
                    .child(
                        v_flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_sm().font_medium().child("Ghost"))
                            .child(
                                ToggleGroup::new("ghost-group")
                                    .with_size(self.size)
                                    .child(
                                        Toggle::new(0)
                                            .icon(IconName::Bell)
                                            .checked(self.checked[1]),
                                    )
                                    .child(
                                        Toggle::new(1)
                                            .icon(IconName::Inbox)
                                            .checked(self.checked[2]),
                                    )
                                    .child(
                                        Toggle::new(2)
                                            .icon(IconName::Check)
                                            .checked(self.checked[3]),
                                    )
                                    .on_click(cx.listener(|this, values: &Vec<bool>, _, cx| {
                                        this.checked[1..4].copy_from_slice(values);
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        v_flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_sm().font_medium().child("Outline"))
                            .child(
                                ToggleGroup::new("outline-group")
                                    .outline()
                                    .with_size(self.size)
                                    .child(
                                        Toggle::new(0)
                                            .icon(IconName::Bell)
                                            .checked(self.checked[4]),
                                    )
                                    .child(
                                        Toggle::new(1)
                                            .icon(IconName::Inbox)
                                            .checked(self.checked[5]),
                                    )
                                    .child(
                                        Toggle::new(2)
                                            .icon(IconName::Check)
                                            .checked(self.checked[6]),
                                    )
                                    .on_click(cx.listener(|this, values: &Vec<bool>, _, cx| {
                                        this.checked[4..7].copy_from_slice(values);
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                section("Group")
                    .description("Connected toggles keep related choices together.")
                    .w_128()
                    .v_flex()
                    .items_center()
                    .child(
                        ToggleGroup::new("segmented-group")
                            .segmented()
                            .outline()
                            .with_size(self.size)
                            .child(Toggle::new(0).label("Bold").checked(self.checked[7]))
                            .child(Toggle::new(1).label("Italic").checked(self.checked[8]))
                            .child(Toggle::new(2).label("Code").checked(self.checked[9]))
                            .on_click(cx.listener(|this, values: &Vec<bool>, _, cx| {
                                this.checked[7..10].copy_from_slice(values);
                                cx.notify();
                            })),
                    ),
            )
    }
}
