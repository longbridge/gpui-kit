use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme as _, ElementExt, Icon, IconName, button::Button, native_menu::NativeMenu, v_flex,
};
use gpui_kit::*;
use serde::Deserialize;

use crate::section;

/// Dispatched by native menu items; "Word Wrap" updates its checked state.
#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = native_menu_story, no_json)]
struct MenuClick(SharedString);

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = native_menu_story, no_json)]
struct OpenGitHub;

const CONTEXT: &str = "NativeMenuStory";

/// A menu item dispatching `MenuClick(label)`.
fn click(label: &str) -> Box<dyn Action> {
    Box::new(MenuClick(label.to_string().into()))
}

/// Demo menu: normal items, a disabled item, a checked item (reflecting
/// `word_wrap`, which the story toggles), and nested submenus.
fn demo_menu(word_wrap: bool) -> NativeMenu {
    NativeMenu::new()
        .menu("Cut", click("Cut"))
        .menu("Copy", click("Copy"))
        .menu("Paste", click("Paste"))
        .separator()
        .menu_with_icon("Github", IconName::Github, Box::new(OpenGitHub))
        .menu_with_icon("Inbox", IconName::Inbox, click("Inbox"))
        .menu_with_icon(
            "Search (SVG bytes)",
            Icon::default().data(include_bytes!("../../../assets/assets/icons/search.svg")),
            click("Search"),
        )
        .separator()
        .menu_with_disabled("Disabled item", true, click("Disabled"))
        .menu_with_check("Word Wrap", word_wrap, click("Word Wrap"))
        .separator()
        .submenu(
            "Open Recent",
            NativeMenu::new()
                .menu("project-a", click("project-a"))
                .menu("project-b", click("project-b"))
                .separator()
                .submenu(
                    "More",
                    NativeMenu::new()
                        .menu("project-c", click("project-c"))
                        .menu("project-d", click("project-d")),
                ),
        )
        .separator()
        .menu("Select All", click("Select All"))
}

pub struct NativeMenuStory {
    focus_handle: FocusHandle,
    /// Demo checked state — toggled when the "Word Wrap" item is selected.
    word_wrap: bool,
}

impl super::Story for NativeMenuStory {
    fn title() -> &'static str {
        "NativeMenu"
    }

    fn description() -> &'static str {
        "A menu rendered by the operating system. Unlike `PopupMenu`, it is drawn \
        by the OS and can extend beyond the window bounds — useful for small windows."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl NativeMenuStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            word_wrap: true,
        }
    }

    fn on_click(&mut self, click: &MenuClick, _: &mut Window, cx: &mut Context<Self>) {
        if click.0.as_ref() == "Word Wrap" {
            self.word_wrap = !self.word_wrap;
        }
        cx.notify();
    }

    fn open_github(&mut self, _: &OpenGitHub, _: &mut Window, cx: &mut Context<Self>) {
        cx.open_url("https://github.com");
    }

    fn trigger(&self, label: &str, cx: &mut App) -> Div {
        div()
            .flex()
            .items_center()
            .justify_center()
            .w_full()
            .h_24()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius_lg)
            .text_color(cx.theme().muted_foreground)
            .child(SharedString::from(label.to_string()))
    }
}

impl Focusable for NativeMenuStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for NativeMenuStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let focus_handle = self.focus_handle.clone();

        v_flex()
            .track_focus(&self.focus_handle)
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_click))
            .on_action(cx.listener(Self::open_github))
            .size_full()
            .items_center()
            .gap_6()
            .child(
                section("Builder API")
                    .description("Supports disabled items, checked states, and submenus.")
                    .w(px(520.))
                    .child(self.trigger("Right-click here", cx).on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                            // Focus the story so the dispatched action reaches `on_click`.
                            this.focus_handle.focus(window, cx);
                            // Nudge right so the cursor doesn't land on the first item.
                            let position = Point {
                                x: ev.position.x + px(4.),
                                y: ev.position.y,
                            };
                            demo_menu(this.word_wrap).show(position, window, cx);
                        }),
                    )),
            )
            .child(
                section("Menu Items")
                    .description("Existing GPUI menu definitions can be reused directly.")
                    .w(px(520.))
                    .child(self.trigger("Right-click here", cx).on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                            this.focus_handle.focus(window, cx);
                            let position = Point {
                                x: ev.position.x + px(4.),
                                y: ev.position.y,
                            };
                            // Reuse a GPUI menu definition (incl. a submenu) directly.
                            NativeMenu::from(gpui_kit::Menu::new("Edit").items([
                                gpui_kit::MenuItem::action("Copy", MenuClick("Copy".into())),
                                gpui_kit::MenuItem::action("Paste", MenuClick("Paste".into())),
                                gpui_kit::MenuItem::separator(),
                                gpui_kit::MenuItem::submenu(gpui_kit::Menu::new("Share").items([
                                    gpui_kit::MenuItem::action("Email", MenuClick("Email".into())),
                                    gpui_kit::MenuItem::action(
                                        "Message",
                                        MenuClick("Message".into()),
                                    ),
                                ])),
                            ]))
                            .show(position, window, cx);
                        }),
                    )),
            )
            .child(
                section("Dropdown")
                    .description("A native menu can open from any anchored control.")
                    .w(px(520.))
                    .child({
                        // A native menu isn't limited to right-click — `show` takes
                        // any window position. Capture the trigger's bounds so the
                        // menu opens at its bottom-left, like a real dropdown.
                        let trigger_bounds: Rc<Cell<Bounds<Pixels>>> =
                            Rc::new(Cell::new(Bounds::default()));
                        let bounds_writer = trigger_bounds.clone();

                        div()
                            .on_prepaint(move |bounds, _, _| bounds_writer.set(bounds))
                            .child(
                                Button::new("native-dropdown")
                                    .outline()
                                    .label("Open Menu")
                                    .on_click(move |_: &ClickEvent, window, cx| {
                                        let bounds = trigger_bounds.get();
                                        let position = Point {
                                            x: bounds.origin.x,
                                            // Just below the button, with a small gap.
                                            y: bounds.origin.y + bounds.size.height + px(8.),
                                        };
                                        focus_handle.focus(window, cx);
                                        demo_menu(view.read(cx).word_wrap)
                                            .show(position, window, cx);
                                    }),
                            )
                    }),
            )
    }
}
