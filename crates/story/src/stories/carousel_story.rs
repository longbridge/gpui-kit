use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, Size, StyledExt as _,
    button::Button,
    carousel::{
        Carousel, CarouselContent, CarouselEvent, CarouselItem, CarouselNext, CarouselPagination,
        CarouselPaginationItem, CarouselPrevious, CarouselState,
    },
    h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::{ChangeStorySize, section, story_toolbar};

#[derive(Clone, Copy)]
enum SlideTypography {
    Large,
    Medium,
    Small,
}

pub struct CarouselStory {
    focus_handle: FocusHandle,
    horizontal: Entity<CarouselState>,
    custom_controls: Entity<CarouselState>,
    multiple: Entity<CarouselState>,
    vertical: Entity<CarouselState>,
    looped: Entity<CarouselState>,
    controlled: Entity<CarouselState>,
    controlled_index: usize,
    size: Size,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for CarouselStory {
    fn title() -> &'static str {
        "Carousel"
    }

    fn description() -> &'static str {
        "A carousel for browsing a set of related items with keyboard and pointer navigation."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl CarouselStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let horizontal = cx.new(|_| CarouselState::new(3));
            let custom_controls = cx.new(|_| CarouselState::new(3));
            let multiple = cx.new(|_| CarouselState::new(5));
            let vertical = cx.new(|_| CarouselState::new(3).with_axis(Axis::Vertical));
            let looped = cx.new(|_| CarouselState::new(4).with_looping(true));
            let controlled = cx.new(|_| CarouselState::new(3).with_selected_index(1));

            let subscription = cx.subscribe(
                &controlled,
                move |this: &mut Self, _, event: &CarouselEvent, cx| {
                    let CarouselEvent::Change(index) = event;
                    this.controlled_index = *index;
                    cx.notify();
                },
            );

            Self {
                focus_handle: cx.focus_handle(),
                horizontal,
                custom_controls,
                multiple,
                vertical,
                looped,
                controlled,
                controlled_index: 1,
                size: Size::default(),
                _subscriptions: vec![subscription],
            }
        })
    }

    fn slide(
        label: impl Into<SharedString>,
        typography: SlideTypography,
        square: bool,
        cx: &App,
    ) -> impl IntoElement {
        div()
            .w_full()
            .when(square, |this| this.aspect_square())
            .when(!square, |this| this.h_full())
            .flex()
            .items_center()
            .justify_center()
            .p_6()
            .rounded(cx.theme().radius_tokens().xl)
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .font_semibold()
            .when(matches!(typography, SlideTypography::Large), |this| {
                this.text_size(rems(2.25))
            })
            .when(matches!(typography, SlideTypography::Medium), |this| {
                this.text_3xl()
            })
            .when(matches!(typography, SlideTypography::Small), |this| {
                this.text_2xl()
            })
            .child(label.into())
    }

    fn items(
        state: &Entity<CarouselState>,
        prefix: &'static str,
        count: usize,
        typography: SlideTypography,
        square: bool,
        cx: &App,
    ) -> CarouselContent {
        Self::items_with(state, prefix, count, typography, square, |item| item, cx)
    }

    fn items_with(
        state: &Entity<CarouselState>,
        prefix: &'static str,
        count: usize,
        typography: SlideTypography,
        square: bool,
        configure: impl Fn(CarouselItem) -> CarouselItem,
        cx: &App,
    ) -> CarouselContent {
        (0..count).fold(CarouselContent::new(state), |content, index| {
            let label = (index + 1).to_string();
            content.child(
                configure(CarouselItem::new((prefix, index), index, state))
                    .child(Self::slide(label, typography, square, cx)),
            )
        })
    }
}

impl Focusable for CarouselStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CarouselStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_4()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .child(story_toolbar(self.size))
            .child(
                section("Basic")
                    .description("Browse one full-width item at a time with Left and Right.")
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-horizontal", &self.horizontal)
                            .w_full()
                            .max_w_96()
                            .mx_auto()
                            .child(Self::items(
                                &self.horizontal,
                                "Horizontal",
                                3,
                                SlideTypography::Large,
                                true,
                                cx,
                            ))
                            .child(CarouselPrevious::new(&self.horizontal).with_size(self.size))
                            .child(CarouselNext::new(&self.horizontal).with_size(self.size))
                            .child(CarouselPagination::new().children((0..3).map(|index| {
                                CarouselPaginationItem::new(
                                    ("horizontal-pagination", index),
                                    index,
                                    &self.horizontal,
                                )
                                .with_size(self.size)
                                .child((index + 1).to_string())
                            }))),
                    ),
            )
            .child(
                section("Custom controls")
                    .description(
                        "Replace control content and accessibility labels while retaining navigation behavior.",
                    )
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-custom-controls", &self.custom_controls)
                            .w_full()
                            .max_w_96()
                            .mx_auto()
                            .child(Self::items(
                                &self.custom_controls,
                                "Custom Controls",
                                3,
                                SlideTypography::Large,
                                true,
                                cx,
                            ))
                            .child(
                                CarouselPrevious::new(&self.custom_controls)
                                    .with_size(self.size)
                                    .accessibility_label("Previous project")
                                    .child("Back"),
                            )
                            .child(
                                CarouselNext::new(&self.custom_controls)
                                    .with_size(self.size)
                                    .accessibility_label("Next project")
                                    .child("Forward"),
                            ),
                    ),
            )
            .child(
                section("Multiple items")
                    .description(
                        "A fractional flex basis shows several items at once. Pair the content's negative margin with matching item padding to tune the gap between them.",
                    )
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-multiple", &self.multiple)
                            .w_full()
                            .max_w_96()
                            .mx_auto()
                            .child(
                                Self::items_with(
                                    &self.multiple,
                                    "Multiple",
                                    5,
                                    SlideTypography::Small,
                                    true,
                                    |item| item.flex_basis(relative(1. / 3.)).pl_1(),
                                    cx,
                                )
                                .track_style(StyleRefinement::default().ml_neg_1()),
                            )
                            .child(CarouselPrevious::new(&self.multiple).with_size(self.size))
                            .child(CarouselNext::new(&self.multiple).with_size(self.size)),
                    ),
            )
            .child(
                section("Vertical")
                    .description("Use Up and Down to navigate a vertical carousel.")
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-vertical", &self.vertical)
                            .w_full()
                            .max_w_64()
                            .mx_auto()
                            .child(
                                Self::items_with(
                                    &self.vertical,
                                    "Vertical",
                                    3,
                                    SlideTypography::Medium,
                                    false,
                                    |item| item.flex_basis(relative(0.5)),
                                    cx,
                                )
                                .h_48(),
                            )
                            .child(CarouselPrevious::new(&self.vertical).with_size(self.size))
                            .child(CarouselNext::new(&self.vertical).with_size(self.size)),
                    ),
            )
            .child(
                section("Looping")
                    .description("Looping navigation wraps from the last slide to the first.")
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-looped", &self.looped)
                            .w_full()
                            .max_w_96()
                            .mx_auto()
                            .child(Self::items(
                                &self.looped,
                                "Looped",
                                4,
                                SlideTypography::Large,
                                true,
                                cx,
                            ))
                            .child(CarouselPrevious::new(&self.looped).with_size(self.size))
                            .child(CarouselNext::new(&self.looped).with_size(self.size)),
                    ),
            )
            .child(
                section("Controlled")
                    .description("The selected index is owned by application state and can be changed programmatically.")
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-controlled", &self.controlled)
                            .w_full()
                            .max_w_96()
                            .mx_auto()
                            .child(Self::items(
                                &self.controlled,
                                "Controlled",
                                3,
                                SlideTypography::Large,
                                true,
                                cx,
                            ))
                            .child(CarouselPrevious::new(&self.controlled).with_size(self.size))
                            .child(CarouselNext::new(&self.controlled).with_size(self.size)),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(div().child(format!("Selected slide: {}", self.controlled_index + 1)))
                            .child(
                                Button::new("controlled-first")
                                    .label("Go to first")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.controlled_index = 0;
                                        let controlled = this.controlled.clone();
                                        controlled.update(cx, |state, cx| {
                                            state.set_selected_index(0, cx);
                                        });
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("controlled-last")
                                    .label("Go to last")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.controlled_index = 2;
                                        let controlled = this.controlled.clone();
                                        controlled.update(cx, |state, cx| {
                                            state.set_selected_index(2, cx);
                                        });
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
    }
}
