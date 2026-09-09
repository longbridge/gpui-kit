use gpui_kit::component::{
    ActiveTheme as _, Colorize, Sizable, Size, StyledExt,
    color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState},
    h_flex, indigo_500, v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, Focusable, Hsla, InteractiveElement, IntoElement,
    ParentElement as _, Render, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder as _, px,
};

use crate::{ChangeStorySize, section, story_toolbar};

pub struct ColorPickerStory {
    color: Entity<ColorPickerState>,
    selected_color: Option<Hsla>,
    size: Size,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for ColorPickerStory {
    fn title() -> &'static str {
        "ColorPicker"
    }

    fn description() -> &'static str {
        "Choose and preview a color value."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl ColorPickerStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let default_color = indigo_500();
        let color = cx.new(|cx| ColorPickerState::new(window, cx).default_value(default_color));

        let _subscriptions = vec![cx.subscribe(&color, |this, _, ev, _| match ev {
            ColorPickerEvent::Change(color) => {
                this.selected_color = *color;
            }
        })];

        Self {
            color,
            selected_color: Some(default_color),
            size: Size::default(),
            _subscriptions,
        }
    }
}

impl Focusable for ColorPickerStory {
    fn focus_handle(&self, cx: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.color.read(cx).focus_handle(cx)
    }
}

impl Render for ColorPickerStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .child(story_toolbar(self.size))
            .child(
                section("Theme Color")
                    .description("Select a color and preview the resulting value.")
                    .w(px(440.))
                    .child(
                        v_flex()
                            .w_full()
                            .gap_4()
                            .p_4()
                            .rounded(cx.theme().radius_lg)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_4()
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(div().font_medium().child("Accent color"))
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(
                                                        "Used for primary actions and highlights.",
                                                    ),
                                            ),
                                    )
                                    .child(ColorPicker::new(&self.color).with_size(self.size)),
                            )
                            .when_some(self.selected_color, |this, color| {
                                this.child(
                                    v_flex()
                                        .w_full()
                                        .overflow_hidden()
                                        .rounded(cx.theme().radius_lg)
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .child(
                                            div()
                                                .w_full()
                                                .rounded_t(cx.theme().radius_lg)
                                                .h(px(96.))
                                                .bg(color),
                                        )
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .items_center()
                                                .justify_between()
                                                .px_3()
                                                .py_2()
                                                .bg(cx.theme().muted)
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .child("Selected color"),
                                                )
                                                .child(
                                                    div()
                                                        .font_family("monospace")
                                                        .font_medium()
                                                        .child(color.to_hex()),
                                                ),
                                        ),
                                )
                            }),
                    ),
            )
    }
}
