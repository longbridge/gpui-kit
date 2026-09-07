use anyhow::{Context as _, Result};
use gpui_kit::component::{
    IconName, Root, Sizable,
    button::{Button, ButtonVariants as _},
    dock::{
        ClosePanel, DockArea, DockAreaState, DockEvent, DockLayout, DockPlacement, DockSkin,
        ToggleZoom, panel_handle,
    },
    menu::DropdownMenu,
    status_bar::StatusBar,
};
use gpui_kit::*;

use gpui_component_story::{
    AccordionStory, AppState, AppTitleBar, ButtonStory, CalendarStory, DataTableStory, DialogStory,
    FormStory, IconStory, ImageStory, InputStory, LabelStory, ListStory, NotificationStory, Open,
    PopoverStory, ProgressStory, ResizableStory, ScrollbarStory, SelectStory, SidebarStory,
    StoryContainer, SwitchStory, TooltipStory,
};
use gpui_kit::assets::Assets;
use serde::Deserialize;
use std::{rc::Rc, time::Duration};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct AddPanel(DockPlacement);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct TogglePanelVisible(SharedString);

actions!(story, [ToggleDockToggleButton]);

const MAIN_DOCK_AREA: DockAreaTab = DockAreaTab {
    id: "main-dock",
    version: 5,
};

#[cfg(debug_assertions)]
const STATE_FILE: &str = "target/docks.json";
#[cfg(not(debug_assertions))]
const STATE_FILE: &str = "docks.json";

pub fn init(cx: &mut App) {
    cx.on_action(|_action: &Open, _cx: &mut App| {});
    gpui_component_story::init(cx);

    cx.bind_keys(vec![
        KeyBinding::new("shift-escape", ToggleZoom, None),
        KeyBinding::new("ctrl-w", ClosePanel, None),
    ]);

    cx.activate(true);
}

pub struct StoryWorkspace {
    title_bar: Entity<AppTitleBar>,
    dock_area: Entity<DockArea>,
    skin: Rc<DockSkin>,
    last_layout_state: Option<DockAreaState>,
    toggle_button_visible: bool,
    _save_layout_task: Option<Task<()>>,
}

struct DockAreaTab {
    id: &'static str,
    version: usize,
}

impl StoryWorkspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (dock_area, skin) =
            DockSkin::dock_area(MAIN_DOCK_AREA.id, Some(MAIN_DOCK_AREA.version), window, cx);
        let weak_dock_area = dock_area.downgrade();

        match Self::load_layout(dock_area.clone(), window, cx) {
            Ok(_) => {
                println!("load layout success");
            }
            Err(err) => {
                eprintln!("load layout error: {:?}", err);
                Self::reset_default_layout(weak_dock_area, window, cx);
            }
        };

        cx.subscribe_in(
            &dock_area,
            window,
            |this, dock_area, ev: &DockEvent, window, cx| match ev {
                DockEvent::LayoutChanged => this.save_layout(dock_area, window, cx),
                _ => {}
            },
        )
        .detach();

        cx.on_app_quit({
            let dock_area = dock_area.clone();
            move |_, cx| {
                let state = dock_area.read(cx).dump(cx);
                cx.background_executor().spawn(async move {
                    // Save layout before quitting
                    Self::save_state(&state).unwrap();
                })
            }
        })
        .detach();

        let title_bar = cx.new(|cx| {
            AppTitleBar::new("Examples", window, cx).child({
                move |_, cx| {
                    Button::new("add-panel")
                        .icon(IconName::LayoutDashboard)
                        .small()
                        .ghost()
                        .dropdown_menu({
                            let invisible_panels = AppState::global(cx).invisible_panels.clone();

                            move |menu, _, cx| {
                                menu.menu(
                                    "Add Panel to Center",
                                    Box::new(AddPanel(DockPlacement::Center)),
                                )
                                .separator()
                                .menu("Add Panel to Left", Box::new(AddPanel(DockPlacement::Left)))
                                .menu(
                                    "Add Panel to Right",
                                    Box::new(AddPanel(DockPlacement::Right)),
                                )
                                .menu(
                                    "Add Panel to Bottom",
                                    Box::new(AddPanel(DockPlacement::Bottom)),
                                )
                                .separator()
                                .menu(
                                    "Show / Hide Dock Toggle Button",
                                    Box::new(ToggleDockToggleButton),
                                )
                                .separator()
                                .menu_with_check(
                                    "Sidebar",
                                    !invisible_panels
                                        .read(cx)
                                        .contains(&SharedString::from("Sidebar")),
                                    Box::new(TogglePanelVisible(SharedString::from("Sidebar"))),
                                )
                                .menu_with_check(
                                    "Dialog",
                                    !invisible_panels
                                        .read(cx)
                                        .contains(&SharedString::from("Dialog")),
                                    Box::new(TogglePanelVisible(SharedString::from("Dialog"))),
                                )
                                .menu_with_check(
                                    "Accordion",
                                    !invisible_panels
                                        .read(cx)
                                        .contains(&SharedString::from("Accordion")),
                                    Box::new(TogglePanelVisible(SharedString::from("Accordion"))),
                                )
                                .menu_with_check(
                                    "List",
                                    !invisible_panels
                                        .read(cx)
                                        .contains(&SharedString::from("List")),
                                    Box::new(TogglePanelVisible(SharedString::from("List"))),
                                )
                            }
                        })
                        .anchor(Anchor::TopRight)
                }
            })
        });

        Self {
            dock_area,
            skin,
            title_bar,
            last_layout_state: None,
            toggle_button_visible: true,
            _save_layout_task: None,
        }
    }

    fn save_layout(
        &mut self,
        dock_area: &Entity<DockArea>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dock_area = dock_area.clone();
        self._save_layout_task = Some(cx.spawn_in(window, async move |story, window| {
            window
                .background_executor()
                .timer(Duration::from_secs(10))
                .await;

            _ = story.update_in(window, move |this, _, cx| {
                let dock_area = dock_area.read(cx);
                let state = dock_area.dump(cx);

                let last_layout_state = this.last_layout_state.clone();
                if Some(&state) == last_layout_state.as_ref() {
                    return;
                }

                Self::save_state(&state).unwrap();
                this.last_layout_state = Some(state);
            });
        }));
    }

    fn save_state(state: &DockAreaState) -> Result<()> {
        println!("Save layout...");
        let json = serde_json::to_string_pretty(state)?;
        std::fs::write(STATE_FILE, json)?;
        Ok(())
    }

    fn load_layout(
        dock_area: Entity<DockArea>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let json = std::fs::read_to_string(STATE_FILE)?;
        let state = serde_json::from_str::<DockAreaState>(&json)?;

        // Check if the saved layout version is different from the current version
        // Notify the user and ask if they want to reset the layout to default.
        if state.version != Some(MAIN_DOCK_AREA.version) {
            let answer = window.prompt(
                PromptLevel::Info,
                "The default main layout has been updated.\n\
                Do you want to reset the layout to default?",
                None,
                &["Yes", "No"],
                cx,
            );

            let weak_dock_area = dock_area.downgrade();
            cx.spawn_in(window, async move |this, window| {
                if answer.await == Ok(0) {
                    _ = this.update_in(window, |_, window, cx| {
                        Self::reset_default_layout(weak_dock_area, window, cx);
                    });
                }
            })
            .detach();
        }

        dock_area.update(cx, |dock_area, cx| {
            dock_area.load(state, window, cx).context("load layout")?;
            for placement in [
                DockPlacement::Left,
                DockPlacement::Bottom,
                DockPlacement::Right,
            ] {
                dock_area.set_dock_collapsible(placement, true, window, cx);
            }

            Ok::<(), anyhow::Error>(())
        })
    }

    fn reset_default_layout(dock_area: WeakEntity<DockArea>, window: &mut Window, cx: &mut App) {
        let center = Self::init_default_layout(window, cx);

        let left_panels = DockLayout::v_split()
            .child(
                DockLayout::tabs().panel_view(
                    panel_handle(StoryContainer::panel::<ListStory>(window, cx)),
                    cx,
                ),
                None,
            )
            .child(
                DockLayout::tabs()
                    .panel_view(
                        panel_handle(StoryContainer::panel::<ScrollbarStory>(window, cx)),
                        cx,
                    )
                    .panel_view(
                        panel_handle(StoryContainer::panel::<AccordionStory>(window, cx)),
                        cx,
                    ),
                Some(px(360.)),
            );

        let bottom_panels = DockLayout::v_split().child(
            DockLayout::tabs()
                .panel_view(
                    panel_handle(StoryContainer::panel::<TooltipStory>(window, cx)),
                    cx,
                )
                .panel_view(
                    panel_handle(StoryContainer::panel::<IconStory>(window, cx)),
                    cx,
                ),
            None,
        );

        let right_panels = DockLayout::v_split()
            .child(
                DockLayout::tabs().panel_view(
                    panel_handle(StoryContainer::panel::<ImageStory>(window, cx)),
                    cx,
                ),
                None,
            )
            .child(
                DockLayout::tabs().panel_view(
                    panel_handle(StoryContainer::panel::<IconStory>(window, cx)),
                    cx,
                ),
                None,
            );

        _ = dock_area.update(cx, |view, cx| {
            // The area was constructed with this version, and base takes it
            // only there, so there is nothing to set here any more.
            view.set_center(center, window, cx);
            for (placement, layout, size) in [
                (DockPlacement::Left, left_panels, px(350.)),
                (DockPlacement::Bottom, bottom_panels, px(200.)),
                (DockPlacement::Right, right_panels, px(320.)),
            ] {
                view.set_dock(placement, layout, window, cx);
                view.set_dock_size(placement, size, window, cx);
            }

            Self::save_state(&view.dump(cx)).unwrap();
        });
    }

    fn init_default_layout(window: &mut Window, cx: &mut App) -> DockLayout {
        let tabs = DockLayout::tabs()
            .panel_view(
                panel_handle(StoryContainer::panel::<ButtonStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<InputStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<SelectStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<LabelStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<DialogStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<PopoverStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<SwitchStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<ProgressStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<DataTableStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<ImageStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<IconStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<TooltipStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<CalendarStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<ResizableStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<ScrollbarStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<AccordionStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<SidebarStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<FormStory>(window, cx)),
                cx,
            )
            .panel_view(
                panel_handle(StoryContainer::panel::<NotificationStory>(window, cx)),
                cx,
            );

        DockLayout::v_split().child(tabs, None)
    }

    pub fn new_local(cx: &mut App) -> Task<anyhow::Result<WindowHandle<Root>>> {
        let mut window_size = size(px(1600.0), px(1200.0));
        if let Some(display) = cx.primary_display() {
            let display_size = display.bounds().size;
            window_size.width = window_size.width.min(display_size.width * 0.85);
            window_size.height = window_size.height.min(display_size.height * 0.85);
        }

        let window_bounds = Bounds::centered(None, window_size, cx);

        cx.spawn(async move |cx| {
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                window_min_size: Some(gpui_kit::Size {
                    width: px(640.),
                    height: px(480.),
                }),
                #[cfg(target_os = "linux")]
                window_background: gpui_kit::WindowBackgroundAppearance::Transparent,
                #[cfg(target_os = "linux")]
                window_decorations: Some(gpui_kit::WindowDecorations::Client),
                kind: WindowKind::Normal,
                ..gpui_kit::component::TitleBar::window_options()
            };

            let window = cx.open_window(options, |window, cx| {
                let story_view = cx.new(|cx| StoryWorkspace::new(window, cx));
                cx.new(|cx| Root::new(story_view, window, cx))
            })?;

            window
                .update(cx, |_, window, cx| {
                    window.activate_window();
                    window.set_window_title("GPUI App");
                    cx.on_release(|_, cx| {
                        // exit app
                        cx.quit();
                    })
                    .detach();
                })
                .expect("failed to update window");

            Ok(window)
        })
    }

    fn on_action_add_panel(
        &mut self,
        action: &AddPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Random pick up a panel to add
        let panel = match rand::random::<usize>() % 18 {
            0 => panel_handle(StoryContainer::panel::<ButtonStory>(window, cx)),
            1 => panel_handle(StoryContainer::panel::<InputStory>(window, cx)),
            2 => panel_handle(StoryContainer::panel::<SelectStory>(window, cx)),
            3 => panel_handle(StoryContainer::panel::<LabelStory>(window, cx)),
            4 => panel_handle(StoryContainer::panel::<DialogStory>(window, cx)),
            5 => panel_handle(StoryContainer::panel::<PopoverStory>(window, cx)),
            6 => panel_handle(StoryContainer::panel::<SwitchStory>(window, cx)),
            7 => panel_handle(StoryContainer::panel::<ProgressStory>(window, cx)),
            8 => panel_handle(StoryContainer::panel::<DataTableStory>(window, cx)),
            9 => panel_handle(StoryContainer::panel::<ImageStory>(window, cx)),
            10 => panel_handle(StoryContainer::panel::<IconStory>(window, cx)),
            11 => panel_handle(StoryContainer::panel::<TooltipStory>(window, cx)),
            12 => panel_handle(StoryContainer::panel::<ProgressStory>(window, cx)),
            13 => panel_handle(StoryContainer::panel::<CalendarStory>(window, cx)),
            14 => panel_handle(StoryContainer::panel::<ResizableStory>(window, cx)),
            15 => panel_handle(StoryContainer::panel::<ScrollbarStory>(window, cx)),
            16 => panel_handle(StoryContainer::panel::<AccordionStory>(window, cx)),
            _ => panel_handle(StoryContainer::panel::<ButtonStory>(window, cx)),
        };

        self.dock_area.update(cx, |dock_area, cx| {
            dock_area.add_panel_view(panel, action.0, None, window, cx);
        });
    }

    fn on_action_toggle_panel_visible(
        &mut self,
        action: &TogglePanelVisible,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let panel_name = action.0.clone();
        let invisible_panels = AppState::global(cx).invisible_panels.clone();
        invisible_panels.update(cx, |names, cx| {
            if names.contains(&panel_name) {
                names.retain(|id| id != &panel_name);
            } else {
                names.push(panel_name);
            }
            cx.notify();
        });
        cx.notify();
    }

    fn on_action_toggle_dock_toggle_button(
        &mut self,
        _: &ToggleDockToggleButton,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_button_visible = !self.toggle_button_visible;
        self.skin
            .set_toggle_button_visible(self.toggle_button_visible, cx);
    }
}

pub fn open_new(
    cx: &mut App,
    init: impl FnOnce(&mut Root, &mut Window, &mut Context<Root>) + 'static + Send,
) -> Task<()> {
    let task: Task<std::result::Result<WindowHandle<Root>, anyhow::Error>> =
        StoryWorkspace::new_local(cx);
    cx.spawn(async move |cx| {
        if let Some(root) = task.await.ok() {
            root.update(cx, |workspace, window, cx| init(workspace, window, cx))
                .expect("failed to init workspace");
        }
    })
}

impl Render for StoryWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sheet_layer = Root::render_sheet_layer(window, cx);
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .id("story-workspace")
            .on_action(cx.listener(Self::on_action_add_panel))
            .on_action(cx.listener(Self::on_action_toggle_panel_visible))
            .on_action(cx.listener(Self::on_action_toggle_dock_toggle_button))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .child(self.title_bar.clone())
            .child(div().flex_1().min_h_0().child(self.dock_area.clone()))
            .child(
                StatusBar::new()
                    .left(
                        Button::new("toggle-left-dock")
                            .ghost()
                            .xsmall()
                            .icon(IconName::PanelLeft)
                            .tooltip("Toggle Left Dock")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.dock_area.update(cx, |area, cx| {
                                    area.toggle_dock(DockPlacement::Left, window, cx);
                                });
                            })),
                    )
                    .left(
                        Button::new("toggle-bottom-dock")
                            .ghost()
                            .xsmall()
                            .icon(IconName::PanelBottom)
                            .tooltip("Toggle Bottom Dock")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.dock_area.update(cx, |area, cx| {
                                    area.toggle_dock(DockPlacement::Bottom, window, cx);
                                });
                            })),
                    )
                    .child(
                        Button::new("toggle-right-dock")
                            .ghost()
                            .xsmall()
                            .icon(IconName::PanelRight)
                            .tooltip("Toggle Right Dock")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.dock_area.update(cx, |area, cx| {
                                    area.toggle_dock(DockPlacement::Right, window, cx);
                                });
                            })),
                    ),
            )
            .children(sheet_layer)
            .children(dialog_layer)
            .children(notification_layer)
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        init(cx);

        open_new(cx, |_, _, _| {
            // do something
        })
        .detach();
    });
}
