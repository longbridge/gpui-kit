use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme as _,
    button::Button,
    dock::{
        BasePanel, DockArea, DockLayout, DockPlacement, DockSkin, Panel, PanelEvent, panel_handle,
    },
    menu::PopupMenuItem,
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    ParentElement as _, Render, SharedString, Styled as _, Window, div, px,
};

use crate::story_toolbar_group;

struct DemoPanel {
    name: &'static str,
    title: SharedString,
    body: SharedString,
    focus_handle: FocusHandle,
    closable: bool,
}

impl DemoPanel {
    fn new(
        name: &'static str,
        title: &'static str,
        body: &'static str,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| Self {
            name,
            title: title.into(),
            body: body.into(),
            focus_handle: cx.focus_handle(),
            closable: true,
        })
    }
}

impl EventEmitter<PanelEvent> for DemoPanel {}

impl Focusable for DemoPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BasePanel for DemoPanel {
    fn panel_name(&self) -> &'static str {
        self.name
    }

    fn closable(&self, _: &App) -> bool {
        self.closable
    }
}

impl Panel for DemoPanel {
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.title.clone()
    }
}

impl Render for DemoPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .text_color(cx.theme().foreground)
            .child(self.body.clone())
    }
}

pub struct DockStory {
    dock_area: Entity<DockArea>,
    skin: Rc<DockSkin>,
    close_button_visible: bool,
}

impl super::Story for DockStory {
    fn title() -> &'static str {
        "Dock"
    }

    fn description() -> &'static str {
        "Drag tabs between groups or towards an edge to split the workspace."
    }

    fn paddings() -> gpui_kit::Pixels {
        px(0.)
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        let (dock_area, skin) = DockSkin::dock_area("dock-story", Some(1), window, cx);
        let explorer = DemoPanel::new(
            "DockStoryExplorer",
            "Explorer",
            "Drag this tab into another group.",
            cx,
        );
        let search = DemoPanel::new(
            "DockStorySearch",
            "Search",
            "Two panels can share one tab group.",
            cx,
        );
        let editor = DemoPanel::new(
            "DockStoryEditor",
            "Editor",
            "Drop a tab near an edge to split this group.",
            cx,
        );
        editor.update(cx, |editor, _| editor.closable = false);
        let terminal = DemoPanel::new(
            "DockStoryTerminal",
            "Terminal",
            "The bottom dock shares the workspace column.",
            cx,
        );
        let problems = DemoPanel::new("DockStoryProblems", "Problems", "No problems detected.", cx);

        dock_area.update(cx, |area, cx| {
            area.set_center(
                DockLayout::h_split()
                    .child(
                        DockLayout::tabs()
                            .panel_view(panel_handle(explorer), cx)
                            .panel_view(panel_handle(search), cx),
                        Some(px(240.)),
                    )
                    .child(
                        DockLayout::tabs().panel_view(panel_handle(editor), cx),
                        None,
                    ),
                window,
                cx,
            );
            area.set_dock(
                DockPlacement::Bottom,
                DockLayout::tabs()
                    .panel_view(panel_handle(terminal), cx)
                    .panel_view(panel_handle(problems), cx),
                window,
                cx,
            );
            area.set_dock_size(DockPlacement::Bottom, px(160.), window, cx);
            area.set_dock_collapsible(DockPlacement::Bottom, true, window, cx);
        });
        skin.set_toggle_button_visible(true, cx);

        cx.new(|_| Self {
            dock_area,
            skin,
            close_button_visible: false,
        })
    }
}

impl Render for DockStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let close_button_visible = self.close_button_visible;
        let story = cx.entity();
        v_flex()
            .size_full()
            .child(story_toolbar_group().p_2().dropdown_child(
                Button::new("dock-options").label("Options"),
                move |menu, window, _| {
                    menu.item(
                        PopupMenuItem::new("Tab close buttons")
                            .checked(close_button_visible)
                            .on_click(window.listener_for(&story, |this, _, _, cx| {
                                this.close_button_visible = !this.close_button_visible;
                                this.skin
                                    .set_close_button_visible(this.close_button_visible, cx);
                                cx.notify();
                            })),
                    )
                },
            ))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(self.dock_area.clone()),
            )
    }
}
