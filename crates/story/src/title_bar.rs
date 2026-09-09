use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme as _, IconName, Side, Sizable as _, Theme, TitleBar, WindowExt as _,
    badge::Badge,
    button::{Button, ButtonVariants as _},
    menu::{AppMenuBar, DropdownMenu as _},
    scroll::ScrollbarMode,
};
use gpui_kit::{
    Anchor, AnyElement, App, AppContext, Context, Entity, FocusHandle, FontWeight,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, Render, SharedString,
    Styled as _, Subscription, Window, div, prelude::FluentBuilder as _, px,
};

use crate::{
    AppState, SelectFont, SelectRadius, SelectScrollbarMode, ToggleAppMenuBar, ToggleFpsMonitor,
    ToggleListActiveHighlight, app_menus,
};

pub struct AppTitleBar {
    title: SharedString,
    app_menu_bar: Entity<AppMenuBar>,
    font_size_selector: Entity<FontSizeSelector>,
    child: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
    _subscriptions: Vec<Subscription>,
}

impl AppTitleBar {
    pub fn new(
        title: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title: SharedString = title.into();
        let app_menu_bar = app_menus::init(title.clone(), cx);
        let font_size_selector = cx.new(|cx| FontSizeSelector::new(window, cx));

        Self {
            title,
            app_menu_bar,
            font_size_selector,
            child: Rc::new(|_, _| div().into_any_element()),
            _subscriptions: vec![],
        }
    }

    pub fn child<F, E>(mut self, f: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.child = Rc::new(move |window, cx| f(window, cx).into_any_element());
        self
    }
}

impl Render for AppTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notifications_count = window.notifications(cx).len();
        let show_app_menu_bar = AppState::global(cx).show_app_menu_bar;

        TitleBar::new()
            // left side
            .child(div().flex().items_center().map(|this| {
                if show_app_menu_bar {
                    this.child(self.app_menu_bar.clone())
                } else {
                    // The system menu bar owns the menus, so the freed up left
                    // side names the window the way a Mac app does.
                    this.child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child(self.title.clone()),
                    )
                }
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .px_2()
                    .gap_2()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child((self.child.clone())(window, cx))
                    .child(self.font_size_selector.clone())
                    .child(
                        Button::new("github")
                            .icon(IconName::Github)
                            .small()
                            .ghost()
                            .on_click(|_, _, cx| {
                                cx.open_url("https://github.com/longbridge/gpui-kit")
                            }),
                    )
                    .child(
                        div().relative().child(
                            Badge::new().count(notifications_count).max(99).child(
                                Button::new("bell")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::Bell),
                            ),
                        ),
                    ),
            )
    }
}

struct FontSizeSelector {
    focus_handle: FocusHandle,
}

impl FontSizeSelector {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    fn on_select_font(
        &mut self,
        font_size: &SelectFont,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        Theme::global_mut(cx).font_size = px(font_size.0 as f32);
        Theme::sync_base(cx);
        window.refresh();
    }

    fn on_select_radius(
        &mut self,
        radius: &SelectRadius,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        Theme::global_mut(cx).radius = px(radius.0 as f32);
        Theme::global_mut(cx).radius_lg = if cx.theme().radius > px(0.) {
            cx.theme().radius + px(2.)
        } else {
            px(0.)
        };
        // The scrollbar paints from the Base layer's own copy of the theme.
        Theme::sync_base(cx);
        window.refresh();
    }

    fn on_select_scrollbar_mode(
        &mut self,
        show: &SelectScrollbarMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        Theme::set_scrollbar_mode(show.0, cx);
        window.refresh();
    }

    fn on_toggle_list_active_highlight(
        &mut self,
        _: &ToggleListActiveHighlight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let theme = Theme::global_mut(cx);
        theme.list.active_highlight = !theme.list.active_highlight;
        window.refresh();
    }

    fn on_toggle_fps_monitor(
        &mut self,
        _: &ToggleFpsMonitor,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let state = AppState::global_mut(cx);
        state.show_fps_monitor = !state.show_fps_monitor;
        window.refresh();
    }

    fn on_toggle_app_menu_bar(
        &mut self,
        _: &ToggleAppMenuBar,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let state = AppState::global_mut(cx);
        state.show_app_menu_bar = !state.show_app_menu_bar;
        window.refresh();
    }
}

impl Render for FontSizeSelector {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle.clone();
        let font_size = cx.theme().font_size.as_f32() as i32;
        let radius = cx.theme().radius.as_f32() as i32;
        let scroll_show = cx.theme().scrollbar_mode;
        let show_fps_monitor = AppState::global(cx).show_fps_monitor;
        let show_app_menu_bar = AppState::global(cx).show_app_menu_bar;

        div()
            .id("font-size-selector")
            .track_focus(&focus_handle)
            .on_action(cx.listener(Self::on_select_font))
            .on_action(cx.listener(Self::on_select_radius))
            .on_action(cx.listener(Self::on_select_scrollbar_mode))
            .on_action(cx.listener(Self::on_toggle_list_active_highlight))
            .on_action(cx.listener(Self::on_toggle_fps_monitor))
            .on_action(cx.listener(Self::on_toggle_app_menu_bar))
            .child(
                Button::new("btn")
                    .small()
                    .ghost()
                    .icon(IconName::Settings2)
                    .dropdown_menu(move |this, _, cx| {
                        this.scrollable(true)
                            .check_side(Side::Right)
                            .max_h(px(480.))
                            .label("Font Size")
                            .menu_with_check("Large", font_size == 18, Box::new(SelectFont(18)))
                            .menu_with_check(
                                "Medium (default)",
                                font_size == 16,
                                Box::new(SelectFont(16)),
                            )
                            .menu_with_check("Small", font_size == 14, Box::new(SelectFont(14)))
                            .separator()
                            .label("Border Radius")
                            .menu_with_check("8px", radius == 8, Box::new(SelectRadius(8)))
                            .menu_with_check(
                                "6px (default)",
                                radius == 6,
                                Box::new(SelectRadius(6)),
                            )
                            .menu_with_check("4px", radius == 4, Box::new(SelectRadius(4)))
                            .menu_with_check("0px", radius == 0, Box::new(SelectRadius(0)))
                            .separator()
                            .label("Scrollbar")
                            .menu_with_check(
                                "Scrolling to show",
                                scroll_show == ScrollbarMode::Scrolling,
                                Box::new(SelectScrollbarMode(ScrollbarMode::Scrolling)),
                            )
                            .menu_with_check(
                                "Hover to show",
                                scroll_show == ScrollbarMode::Hover,
                                Box::new(SelectScrollbarMode(ScrollbarMode::Hover)),
                            )
                            .menu_with_check(
                                "Always show",
                                scroll_show == ScrollbarMode::Always,
                                Box::new(SelectScrollbarMode(ScrollbarMode::Always)),
                            )
                            .separator()
                            .menu_with_check(
                                "List Active Highlight",
                                cx.theme().list.active_highlight,
                                Box::new(ToggleListActiveHighlight),
                            )
                            .menu_with_check(
                                "FPS Monitor",
                                show_fps_monitor,
                                Box::new(ToggleFpsMonitor),
                            )
                            .menu_with_check(
                                "App Menu Bar",
                                show_app_menu_bar,
                                Box::new(ToggleAppMenuBar),
                            )
                    })
                    .anchor(Anchor::TopRight),
            )
    }
}
