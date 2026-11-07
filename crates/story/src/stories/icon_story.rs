use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable,
    button::{Button, ButtonVariant, ButtonVariants},
    dock::PanelControl,
    h_flex,
    menu::{DropdownMenu as _, PopupMenuItem},
    neutral_500, v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window, px, radians,
};

use crate::section;

const SEARCH_SVG: &[u8] = include_bytes!("../../../assets/assets/icons/search.svg");
const ARROW_SVG: &[u8] = include_bytes!("../../../assets/assets/icons/arrow-up.svg");
const LOADER_SVG: &[u8] = include_bytes!("../../../assets/assets/icons/loader-circle.svg");

struct Search;

impl From<Search> for Icon {
    fn from(_: Search) -> Self {
        Icon::default().data(SEARCH_SVG)
    }
}

pub struct IconStory {
    focus_handle: gpui_kit::FocusHandle,
    arrow: Icon,
    arrow_view: Entity<Icon>,
    message: &'static str,
}

impl IconStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let arrow = Icon::default()
            .data(ARROW_SVG)
            .rotate(radians(std::f32::consts::FRAC_PI_2))
            .large();
        Self {
            focus_handle: cx.focus_handle(),
            arrow_view: arrow.clone().view(cx),
            arrow,
            message: "Choose a button or menu item to try the icon slots.",
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for IconStory {
    fn title() -> &'static str {
        "Icon"
    }

    fn description() -> &'static str {
        "SVG Icons based on Lucide.dev"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for IconStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for IconStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        v_flex()
            .items_center()
            .gap_6()
            .child(
                section("SVG bytes")
                    .description(
                        "Embedded icons share the same sizing, colors, and loading behavior.",
                    )
                    .w(gpui_kit::rems(30.))
                    .child(
                        v_flex()
                            .gap_3()
                            .child(
                                h_flex()
                                    .gap_4()
                                    .child(Icon::default().data(SEARCH_SVG).small())
                                    .child(
                                        Icon::default()
                                            .data(SEARCH_SVG)
                                            .large()
                                            .text_color(cx.theme().primary),
                                    )
                                    .child(
                                        Button::new("embedded-search")
                                            .icon(Search)
                                            .label("Search")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.message = "Search selected from the button.";
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("embedded-loading")
                                            .icon(Search)
                                            .loading_icon(Icon::default().data(LOADER_SVG))
                                            .loading(true)
                                            .label("Searching"),
                                    )
                                    .child(
                                        Button::new("embedded-menu")
                                            .label("Actions")
                                            .dropdown_menu(move |menu, window, _| {
                                                menu.item(
                                                    PopupMenuItem::new("Search")
                                                        .icon(Search)
                                                        .on_click(window.listener_for(
                                                        &view,
                                                        |this, _, _, cx| {
                                                            this.message =
                                                                "Search selected from the menu.";
                                                            cx.notify();
                                                        },
                                                    )),
                                                )
                                            }),
                                    ),
                            )
                            .child(
                                gpui_kit::div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(self.message),
                            ),
                    ),
            )
            .child(
                section("Cloning and views")
                    .description("Both arrows retain their SVG bytes and 90° rotation.")
                    .w(gpui_kit::rems(30.))
                    .child(h_flex().gap_4().child("Clone").child(self.arrow.clone()))
                    .child(
                        h_flex()
                            .gap_4()
                            .child("Entity")
                            .child(self.arrow_view.clone()),
                    ),
            )
            .child(
                section("Icons")
                    .description("Common interface symbols from the bundled icon set.")
                    .w(px(480.))
                    .text_lg()
                    .child(IconName::Info)
                    .child(IconName::Map)
                    .child(IconName::Bot)
                    .child(IconName::Github)
                    .child(IconName::Calendar)
                    .child(IconName::Globe)
                    .child(IconName::Heart),
            )
            .child(
                section("Color")
                    .description("Icons inherit semantic foreground colors.")
                    .w(px(480.))
                    .child(
                        Icon::new(IconName::Maximize)
                            .size_6()
                            .text_color(cx.theme().green),
                    )
                    .child(
                        Icon::new(IconName::Minimize)
                            .size_6()
                            .text_color(cx.theme().red),
                    ),
            )
            .child(
                section("Icon Buttons")
                    .description("Icons can be used as compact button content.")
                    .w(px(480.))
                    .child(
                        h_flex()
                            .gap_4()
                            .child(
                                Button::new("like1")
                                    .icon(
                                        Icon::new(IconName::Heart)
                                            .text_color(neutral_500())
                                            .size_6(),
                                    )
                                    .with_variant(ButtonVariant::Ghost),
                            )
                            .child(
                                Button::new("like2")
                                    .icon(
                                        Icon::new(IconName::HeartOff)
                                            .text_color(cx.theme().red)
                                            .size_6(),
                                    )
                                    .with_variant(ButtonVariant::Ghost),
                            )
                            .child(
                                Button::new("like3")
                                    .icon(
                                        Icon::new(IconName::Heart)
                                            .text_color(cx.theme().green)
                                            .size_6(),
                                    )
                                    .with_variant(ButtonVariant::Ghost),
                            ),
                    ),
            )
            .child(
                section("Custom Size")
                    .description("Explicit dimensions support dense controls and counters.")
                    .w(px(480.))
                    .child(
                        Button::new("button-with-size")
                            .outline()
                            .size_5()
                            .small()
                            .px_0()
                            .label("10"),
                    ),
            )
    }
}
