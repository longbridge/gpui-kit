use gpui_fps::fps_monitor;
use gpui_kit::component::{
    ActiveTheme, IconName, Root, Sizable as _, Size as ComponentSize, StyledExt as _,
    TITLE_BAR_HEIGHT, TitleBar, WindowExt,
    button::Button,
    command::{Command, CommandEntry, CommandState},
    dock::{
        BasePanel, Panel, PanelControl, PanelEvent, PanelInfo, PanelState, TitleStyle,
        panel_handle, register_panel,
    },
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex,
    menu::PopupMenu,
    notification::Notification,
    popover::Popover,
    scroll::{ScrollableElement as _, ScrollbarMode},
    text::markdown,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde::{Deserialize, Serialize};
use std::{cell::Cell, rc::Rc};

mod app_menus;
mod embedded_themes;
mod gallery;
mod stories;
mod themes;
mod title_bar;
use crate::themes::SelectTheme;
pub use crate::title_bar::AppTitleBar;
pub use gallery::Gallery;
pub use stories::*;

rust_i18n::i18n!("locales", fallback = "en");

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectScrollbarMode(ScrollbarMode);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectLocale(SharedString);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectFont(usize);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectRadius(usize);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub(crate) struct ChangeStorySize(pub ComponentSize);

actions!(
    story,
    [
        About,
        Open,
        OpenCommandPalette,
        Quit,
        ToggleSearch,
        TestAction,
        Tab,
        TabPrev,
        ShowPanelInfo,
        ToggleListActiveHighlight,
        ToggleFpsMonitor,
        ToggleAppMenuBar
    ]
);

const PANEL_NAME: &str = "StoryContainer";

pub struct AppState {
    pub invisible_panels: Entity<Vec<SharedString>>,
    /// Whether the window root renders the performance HUD. Toggled from the
    /// title bar's settings menu, read by [`StoryRoot`].
    pub show_fps_monitor: bool,
    /// Whether the title bar draws the in-window [`AppMenuBar`] instead of the
    /// window title. Toggled from the title bar's settings menu, read by
    /// [`AppTitleBar`].
    ///
    /// [`AppMenuBar`]: gpui_kit::component::menu::AppMenuBar
    pub show_app_menu_bar: bool,
    pub(crate) previewing_theme: bool,
}
impl AppState {
    fn init(cx: &mut App) {
        let state = Self {
            invisible_panels: cx.new(|_| Vec::new()),
            show_fps_monitor: false,
            // macOS draws the app menus in the system menu bar, so an in-window
            // menu bar would be a second copy of them. Off by default there,
            // but still switchable so the component stays demoable on a Mac.
            show_app_menu_bar: !cfg!(target_os = "macos"),
            previewing_theme: false,
        };
        cx.set_global::<AppState>(state);
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }
}

pub fn create_new_window<F, E>(title: &str, crate_view_fn: F, cx: &mut App)
where
    E: Into<AnyView>,
    F: FnOnce(&mut Window, &mut App) -> E + Send + 'static,
{
    create_new_window_with_size(title, None, crate_view_fn, cx);
}

pub fn create_new_window_with_size<F, E>(
    title: &str,
    window_size: Option<Size<Pixels>>,
    crate_view_fn: F,
    cx: &mut App,
) where
    E: Into<AnyView>,
    F: FnOnce(&mut Window, &mut App) -> E + Send + 'static,
{
    let mut window_size = window_size.unwrap_or(size(px(1600.0), px(1200.0)));
    if let Some(display) = cx.primary_display() {
        let display_size = display.bounds().size;
        window_size.width = window_size.width.min(display_size.width * 0.85);
        window_size.height = window_size.height.min(display_size.height * 0.85);
    }
    let window_bounds = Bounds::centered(None, window_size, cx);
    let title = SharedString::from(title.to_string());

    cx.spawn(async move |cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            window_min_size: Some(gpui_kit::Size {
                width: px(480.),
                height: px(320.),
            }),
            kind: WindowKind::Normal,
            #[cfg(target_os = "linux")]
            window_background: story_window_background(),
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui_kit::WindowDecorations::Client),
            ..TitleBar::window_options()
        };

        let window = cx
            .open_window(options, |window, cx| {
                let view = crate_view_fn(window, cx);
                let story_root = cx.new(|cx| StoryRoot::new(title.clone(), view, window, cx));

                // Set focus to the StoryRoot to enable it's actions.
                let focus_handle = story_root.focus_handle(cx);
                window.defer(cx, move |window, cx| {
                    if window.focused(cx).is_none() {
                        focus_handle.focus(window, cx);
                    }
                });

                cx.new(|cx| Root::new(story_root, window, cx))
            })
            .expect("failed to open window");

        window.update(cx, |_, window, _| {
            window.activate_window();
            window.set_window_title(&title);
        })?;

        Ok::<_, anyhow::Error>(())
    })
    .detach();
}

#[cfg(target_os = "linux")]
fn story_window_background() -> gpui_kit::WindowBackgroundAppearance {
    // The component gallery is a normal application window. Advertising an
    // alpha surface lets compositors show the desktop through light themes,
    // even though every story is designed against an opaque canvas.
    gpui_kit::WindowBackgroundAppearance::Opaque
}

impl Global for AppState {}

/// Layers the story locales over the component ones. `extend!` takes the
/// crate as an identifier and stringifies it into the namespace key, so the
/// layer is bound under its crate name here.
fn extend_component_translations() {
    use gpui_kit::component as gpui_component;
    rust_i18n::extend!(gpui_component);
}

pub fn init(cx: &mut App) {
    // Try to initialize tracing subscriber, but ignore if already initialized
    #[cfg(not(target_family = "wasm"))]
    {
        use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("gpui_component=trace".parse().unwrap()),
            )
            .try_init();
    }

    // For WASM, use a subscriber without time support
    #[cfg(target_family = "wasm")]
    {
        use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().without_time())
            .with(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("gpui_component=trace".parse().unwrap()),
            )
            .try_init();
    }

    extend_component_translations();
    gpui_kit::init(cx);
    AppState::init(cx);
    themes::init(cx);
    stories::init(cx);

    #[cfg(not(target_family = "wasm"))]
    {
        let http_client =
            reqwest_client::ReqwestClient::user_agent("gpui-component/story").unwrap();
        cx.set_http_client(std::sync::Arc::new(http_client));
    }

    cx.bind_keys([
        KeyBinding::new("/", ToggleSearch, None),
        KeyBinding::new("ctrl-shift-p", OpenCommandPalette, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-o", Open, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-o", Open, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-q", Quit, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("alt-f4", Quit, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-k", SelectTheme, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-k", SelectTheme, None),
    ]);

    cx.on_action(|_: &Quit, cx: &mut App| {
        cx.quit();
    });

    cx.on_action(|_: &About, cx: &mut App| {
        if let Some(window) = cx.active_window().and_then(|w| w.downcast::<Root>()) {
            cx.defer(move |cx| {
                window
                    .update(cx, |_, window, cx| {
                        window.defer(cx, |window, cx| {
                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert.title("About").description(markdown(
                                    "GPUI Component Storybook\n\n\
                                    Version 0.1.0\n\n\
                                    https://gpui-kit.com",
                                ))
                            });
                        });
                    })
                    .unwrap();
            });
        }
    });

    register_panel(cx, PANEL_NAME, |context, window, cx| {
        let story_state = match context.info() {
            PanelInfo::Panel(value) => StoryState::from_value(value.clone()),
            info => {
                unreachable!("Invalid PanelInfo: {:?}", info)
            }
        };

        let view = cx.new(|cx| {
            let (title, description, closable, zoomable, story, on_active) =
                story_state.to_story(window, cx);
            let mut container = StoryContainer::new(window, cx)
                .story(story, story_state.story_klass)
                .on_active(on_active);

            cx.on_focus_in(
                &container.focus_handle,
                window,
                |this: &mut StoryContainer, _, _| {
                    println!("StoryContainer focus in: {}", this.name);
                },
            )
            .detach();

            container.name = title.into();
            container.description = description.into();
            container.closable = closable;
            container.zoomable = zoomable;
            container
        });
        panel_handle(view)
    });

    cx.activate(true);
}

#[derive(IntoElement)]
struct StorySection {
    base: Div,
    title: SharedString,
    description: Option<SharedString>,
    sub_title: Vec<AnyElement>,
    children: Vec<AnyElement>,
}

impl StorySection {
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn sub_title(mut self, sub_title: impl IntoElement) -> Self {
        self.sub_title.push(sub_title.into_any_element());
        self
    }

    #[allow(unused)]
    fn max_w_md(mut self) -> Self {
        self.base = self.base.max_w(rems(48.));
        self
    }

    #[allow(unused)]
    fn max_w_lg(mut self) -> Self {
        self.base = self.base.max_w(rems(64.));
        self
    }

    #[allow(unused)]
    fn max_w_xl(mut self) -> Self {
        self.base = self.base.max_w(rems(80.));
        self
    }

    #[allow(unused)]
    fn max_w_2xl(mut self) -> Self {
        self.base = self.base.max_w(rems(96.));
        self
    }
}

impl ParentElement for StorySection {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for StorySection {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for StorySection {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        GroupBox::new()
            .id(self.title.clone())
            .outline()
            .mb_6()
            .title(
                h_flex()
                    .justify_between()
                    .items_start()
                    .w_full()
                    .gap_4()
                    .child(
                        v_flex()
                            .min_w_0()
                            .flex_1()
                            .gap_1()
                            .child(div().font_medium().child(self.title))
                            .when_some(self.description, |this, description| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(description),
                                )
                            }),
                    )
                    .children(self.sub_title),
            )
            .content_style(
                StyleRefinement::default()
                    .rounded(cx.theme().radius_lg)
                    .overflow_x_hidden()
                    .items_center()
                    .justify_center(),
            )
            .child(self.base.children(self.children))
    }
}

pub(crate) fn section(title: impl Into<SharedString>) -> StorySection {
    StorySection {
        title: title.into(),
        description: None,
        sub_title: vec![],
        base: h_flex()
            .w_full()
            .flex_wrap()
            .justify_center()
            .items_center()
            .gap_4(),
        children: vec![],
    }
}

#[derive(IntoElement)]
pub(crate) struct StoryToolbar {
    base: Div,
    items: Vec<StoryToolbarItem>,
}

enum StoryToolbarItem {
    Button(Button),
    Dropdown {
        button: Button,
        builder: StoryMenuBuilder,
    },
}

impl StoryToolbar {
    pub(crate) fn child(mut self, button: Button) -> Self {
        self.items.push(StoryToolbarItem::Button(button));
        self
    }

    pub(crate) fn dropdown_child(
        mut self,
        button: Button,
        builder: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.items.push(StoryToolbarItem::Dropdown {
            button,
            builder: Rc::new(builder),
        });
        self
    }
}

type StoryMenuBuilder = Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>;

#[derive(Default)]
struct StoryToolbarMenuState {
    menu: Option<Entity<PopupMenu>>,
}

#[derive(IntoElement)]
struct StoryToolbarMenu {
    id: SharedString,
    button: Button,
    builder: StoryMenuBuilder,
}

impl RenderOnce for StoryToolbarMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state =
            window.use_keyed_state(self.id.clone(), cx, |_, _| StoryToolbarMenuState::default());
        let builder = self.builder;

        let popover_id = SharedString::from(format!("story-toolbar-popover-{}", self.id));

        Popover::new(popover_id)
            .appearance(false)
            .overlay_closable(false)
            .anchor(Anchor::TopRight)
            .trigger(self.button)
            .content(move |_, window, cx| {
                if let Some(menu) = state.read(cx).menu.clone() {
                    return menu;
                }

                let builder = builder.clone();
                let menu = PopupMenu::build(window, cx, move |menu, window, cx| {
                    builder(menu, window, cx)
                });
                state.update(cx, |state, _| state.menu = Some(menu.clone()));
                menu.focus_handle(cx).focus(window, cx);

                let popover = cx.entity();
                window
                    .subscribe(&menu, cx, {
                        let state = state.clone();
                        move |_, _: &DismissEvent, window, cx| {
                            popover.update(cx, |popover, cx| popover.dismiss(window, cx));
                            state.update(cx, |state, _| state.menu = None);
                        }
                    })
                    .detach();

                menu
            })
    }
}

impl Styled for StoryToolbar {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for StoryToolbar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let last = self.items.len().saturating_sub(1);

        self.base
            .children(self.items.into_iter().enumerate().map(|(ix, item)| {
                // Join the buttons into one segmented control: square off the
                // inner corners, and let each button after the first sit on its
                // neighbour's border instead of drawing a second one.
                let joined = |button: Button| {
                    button
                        .outline()
                        .small()
                        .when(ix > 0, |this| {
                            this.rounded_tl(px(0.)).rounded_bl(px(0.)).border_l_0()
                        })
                        .when(ix < last, |this| this.rounded_tr(px(0.)).rounded_br(px(0.)))
                };

                match item {
                    StoryToolbarItem::Button(button) => joined(button).into_any_element(),
                    StoryToolbarItem::Dropdown { button, builder } => StoryToolbarMenu {
                        id: SharedString::from(format!("story-toolbar-menu-{ix}")),
                        button: joined(button),
                        builder,
                    }
                    .into_any_element(),
                }
            }))
    }
}

pub(crate) fn story_toolbar_group() -> StoryToolbar {
    StoryToolbar {
        base: h_flex().w_full().justify_end(),
        items: vec![],
    }
}

pub(crate) fn story_toolbar(size: ComponentSize) -> StoryToolbar {
    let label = match size {
        ComponentSize::XSmall => "XSmall",
        ComponentSize::Small => "Small",
        ComponentSize::Medium => "Medium",
        ComponentSize::Large => "Large",
        ComponentSize::Size(_) => "Custom",
    };

    story_toolbar_group().dropdown_child(
        Button::new("story-size").label(format!("Size: {label}")),
        move |menu, _, _| {
            menu.menu_with_check(
                "XSmall",
                size == ComponentSize::XSmall,
                Box::new(ChangeStorySize(ComponentSize::XSmall)),
            )
            .menu_with_check(
                "Small",
                size == ComponentSize::Small,
                Box::new(ChangeStorySize(ComponentSize::Small)),
            )
            .menu_with_check(
                "Medium",
                size == ComponentSize::Medium,
                Box::new(ChangeStorySize(ComponentSize::Medium)),
            )
            .menu_with_check(
                "Large",
                size == ComponentSize::Large,
                Box::new(ChangeStorySize(ComponentSize::Large)),
            )
        },
    )
}

pub struct StoryContainer {
    focus_handle: gpui_kit::FocusHandle,
    pub name: SharedString,
    pub title_bg: Option<Hsla>,
    pub description: SharedString,
    width: Option<gpui_kit::Pixels>,
    height: Option<gpui_kit::Pixels>,
    story: Option<AnyView>,
    story_klass: Option<SharedString>,
    closable: bool,
    zoomable: Option<PanelControl>,
    paddings: Pixels,
    on_active: Option<fn(AnyView, bool, &mut Window, &mut App)>,
}

#[derive(Debug)]
pub enum ContainerEvent {
    Close,
}

impl EventEmitter<ContainerEvent> for StoryContainer {}

impl StoryContainer {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        let focus_handle = cx.focus_handle();

        Self {
            focus_handle,
            name: "".into(),
            title_bg: None,
            description: "".into(),
            width: None,
            height: None,
            story: None,
            story_klass: None,
            closable: true,
            zoomable: Some(PanelControl::default()),
            paddings: px(16.),
            on_active: None,
        }
    }

    pub fn panel<S: Story>(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let name = S::title();
        let description = S::description();
        let story = S::new_view(window, cx);
        let story_klass = S::klass();

        let view = cx.new(|cx| {
            let mut story = Self::new(window, cx)
                .story(story.into(), story_klass)
                .on_active(S::on_active_any);
            story.focus_handle = cx.focus_handle();
            story.closable = S::closable();
            story.zoomable = S::zoomable();
            story.name = name.into();
            story.description = description.into();
            story.title_bg = S::title_bg();
            story.paddings = S::paddings();
            story
        });

        view
    }

    pub fn width(mut self, width: gpui_kit::Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: gpui_kit::Pixels) -> Self {
        self.height = Some(height);
        self
    }

    pub fn story(mut self, story: AnyView, story_klass: impl Into<SharedString>) -> Self {
        self.story = Some(story);
        self.story_klass = Some(story_klass.into());
        self
    }

    pub fn on_active(mut self, on_active: fn(AnyView, bool, &mut Window, &mut App)) -> Self {
        self.on_active = Some(on_active);
        self
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StoryState {
    pub story_klass: SharedString,
}

impl StoryState {
    fn to_value(&self) -> serde_json::Value {
        serde_json::json!({
            "story_klass": self.story_klass,
        })
    }

    fn from_value(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap()
    }

    fn to_story(
        &self,
        window: &mut Window,
        cx: &mut App,
    ) -> (
        &'static str,
        &'static str,
        bool,
        Option<PanelControl>,
        AnyView,
        fn(AnyView, bool, &mut Window, &mut App),
    ) {
        macro_rules! story {
            ($klass:tt) => {
                (
                    $klass::title(),
                    $klass::description(),
                    $klass::closable(),
                    $klass::zoomable(),
                    $klass::view(window, cx).into(),
                    $klass::on_active_any,
                )
            };
        }

        match self.story_klass.to_string().as_str() {
            "AttachmentStory" => story!(AttachmentStory),
            "BreadcrumbStory" => story!(BreadcrumbStory),
            "BubbleStory" => story!(BubbleStory),
            "ButtonStory" => story!(ButtonStory),
            "CalendarStory" => story!(CalendarStory),
            "SelectStory" => story!(SelectStory),
            "IconStory" => story!(IconStory),
            "ImageStory" => story!(ImageStory),
            "InputStory" => story!(InputStory),
            "ListStory" => story!(ListStory),
            "MarkerStory" => story!(MarkerStory),
            "MessageStory" => story!(MessageStory),
            "MessageScrollerStory" => story!(MessageScrollerStory),
            "DialogStory" => story!(DialogStory),
            "SeparatorStory" => story!(SeparatorStory),
            "ShimmerStory" => story!(ShimmerStory),
            "PopoverStory" => story!(PopoverStory),
            "ProgressStory" => story!(ProgressStory),
            "ResizableStory" => story!(ResizableStory),
            "ScrollbarStory" => story!(ScrollbarStory),
            "SwitchStory" => story!(SwitchStory),
            "DataTableStory" => story!(DataTableStory),
            "TableStory" => story!(TableStory),
            "LabelStory" => story!(LabelStory),
            "TooltipStory" => story!(TooltipStory),
            "AccordionStory" => story!(AccordionStory),
            "SidebarStory" => story!(SidebarStory),
            "FormStory" => story!(FormStory),
            "NotificationStory" => story!(NotificationStory),
            "ThemeColorsStory" => story!(ThemeColorsStory),
            _ => {
                unreachable!("Invalid story klass: {}", self.story_klass)
            }
        }
    }
}

impl BasePanel for StoryContainer {
    fn panel_name(&self) -> &'static str {
        "StoryContainer"
    }

    fn closable(&self, _cx: &App) -> bool {
        self.closable
    }

    /// The presentation half decides *where* the control appears; this decides
    /// whether zooming is possible at all.
    fn zoomable(&self, _cx: &App) -> bool {
        self.zoomable.is_some()
    }

    fn visible(&self, cx: &App) -> bool {
        !AppState::global(cx)
            .invisible_panels
            .read(cx)
            .contains(&self.name)
    }

    fn set_zoomed(&mut self, zoomed: bool, _window: &mut Window, _cx: &mut Context<Self>) {
        println!("panel: {} zoomed: {}", self.name, zoomed);
    }

    fn set_active(&mut self, active: bool, _window: &mut Window, cx: &mut Context<Self>) {
        println!("panel: {} active: {}", self.name, active);
        if let Some(on_active) = self.on_active {
            if let Some(story) = self.story.clone() {
                on_active(story, active, _window, cx);
            }
        }
    }

    fn dump(&self, _cx: &App) -> PanelState {
        let mut state = PanelState::new(self.panel_name());
        let story_state = StoryState {
            story_klass: self.story_klass.clone().unwrap(),
        };
        state.info = PanelInfo::panel(story_state.to_value());
        state
    }
}

impl Panel for StoryContainer {
    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.name.clone().into_any_element()
    }

    fn title_style(&self, cx: &App) -> Option<TitleStyle> {
        if let Some(bg) = self.title_bg {
            Some(TitleStyle {
                background: bg,
                foreground: cx.theme().foreground,
            })
        } else {
            None
        }
    }

    fn zoom_control(&self, _cx: &App) -> Option<PanelControl> {
        self.zoomable
    }

    fn dropdown_menu(
        &mut self,
        menu: PopupMenu,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> PopupMenu {
        menu.menu("Info", Box::new(ShowPanelInfo))
    }

    fn toolbar_buttons(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Vec<Button>> {
        Some(vec![
            Button::new("info")
                .icon(IconName::Info)
                .on_click(|_, window, cx| {
                    window.push_notification("You have clicked info button", cx);
                }),
            Button::new("search")
                .icon(IconName::Search)
                .on_click(|_, window, cx| {
                    window.push_notification("You have clicked search button", cx);
                }),
        ])
    }
}

impl EventEmitter<PanelEvent> for StoryContainer {}
impl Focusable for StoryContainer {
    fn focus_handle(&self, _: &App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for StoryContainer {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("story-container")
            .size_full()
            .overflow_y_scrollbar()
            .track_focus(&self.focus_handle)
            .when_some(self.story.clone(), |this, story| {
                this.child(div().size_full().p(self.paddings).child(story))
            })
    }
}

fn with_command_entries(
    command: Command,
    entries: impl IntoIterator<Item = CommandEntry>,
) -> Command {
    entries
        .into_iter()
        .fold(command, |command, entry| match entry {
            CommandEntry::Item(item) => command.item(item),
            CommandEntry::Group(group) => command.group(group),
            CommandEntry::Separator => command.separator(),
        })
}

pub struct StoryRoot {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) title_bar: Entity<AppTitleBar>,
    pub(crate) view: AnyView,
    pub(crate) embedded: bool,
    /// The palette behind [`SelectTheme`], rebuilt every time it opens.
    theme_palette: Entity<CommandState>,
    /// Theme commands owned by this root and refreshed before the palette opens.
    theme_entries: Vec<CommandEntry>,
    /// The component palette for Gallery windows.
    component_palette: Entity<CommandState>,
    /// Component commands owned by this root and refreshed before the palette opens.
    component_entries: Vec<CommandEntry>,
    /// Whether this root currently owns an open component palette dialog.
    component_palette_open: bool,
    gallery: Option<Entity<Gallery>>,
    /// The theme in force when the palette opened, put back if the user
    /// cancels out of the preview.
    theme_before_preview: Option<SharedString>,
    /// Identifies the current preview session so deferred selection callbacks
    /// from a closed palette cannot affect a newly opened one.
    theme_preview_generation: u64,
}

impl StoryRoot {
    pub fn new(
        title: impl Into<SharedString>,
        view: impl Into<AnyView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title_bar = cx.new(|cx| AppTitleBar::new(title, window, cx));

        Self::with_title_bar(title_bar, view, false, window, cx)
    }

    pub fn embedded(view: impl Into<AnyView>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let title_bar = cx.new(|cx| AppTitleBar::new("", window, cx));

        Self::with_title_bar(title_bar, view, true, window, cx)
    }

    fn with_title_bar(
        title_bar: Entity<AppTitleBar>,
        view: impl Into<AnyView>,
        embedded: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let view = view.into();
        let gallery = view.clone().downcast::<Gallery>().ok();
        let theme_palette = cx.new(|cx| CommandState::new(window, cx));
        let component_palette = cx.new(|cx| CommandState::new(window, cx));

        Self {
            focus_handle: cx.focus_handle(),
            title_bar,
            view,
            embedded,
            theme_palette,
            theme_entries: Vec::new(),
            component_palette,
            component_entries: Vec::new(),
            component_palette_open: false,
            gallery,
            theme_before_preview: None,
            theme_preview_generation: 0,
        }
    }

    fn on_component_palette_confirm(
        &mut self,
        index: gpui_kit::component::IndexPath,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(gallery) = self.gallery.clone() {
            gallery.update(cx, |gallery, cx| {
                gallery.select_story_index(index, window, cx);
            });
        }
        self.component_palette_open = false;
        window.close_dialog(cx);
    }

    /// Preview the highlighted theme while the palette is open: moving the
    /// highlight applies the theme, Enter keeps it, Escape puts back the one
    /// that was in force when the palette opened.
    fn on_theme_palette_select(
        &mut self,
        index: gpui_kit::component::IndexPath,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        if self.theme_before_preview.is_none() || self.theme_preview_generation != generation {
            return;
        }

        if let Some(name) = themes::theme_name_at(index, cx) {
            themes::apply_theme(&name, cx);
        }
    }

    fn on_theme_palette_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.theme_before_preview = None;
        themes::finish_theme_preview(cx);
        window.close_dialog(cx);
    }

    fn finish_theme_preview(&mut self, cx: &mut Context<Self>) {
        if let Some(name) = self.theme_before_preview.take() {
            themes::apply_theme(&name, cx);
        }
        themes::finish_theme_preview(cx);
    }

    fn on_action_panel_info(
        &mut self,
        _: &ShowPanelInfo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        struct Info;
        let note = Notification::new()
            .message("You have clicked panel info.")
            .id::<Info>();
        window.push_notification(note, cx);
    }

    /// Opens the theme Command palette, refreshed from the registry so that a
    /// theme added while the app is running shows up, and reset to an empty
    /// query.
    fn on_action_select_theme(
        &mut self,
        _: &SelectTheme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.theme_before_preview.is_some() {
            self.theme_palette
                .read(cx)
                .focus_handle(cx)
                .focus(window, cx);
            return;
        }

        self.theme_entries = themes::theme_entries(cx);
        self.theme_before_preview = Some(cx.theme().theme_name().clone());
        self.theme_preview_generation = self.theme_preview_generation.wrapping_add(1);
        let theme_preview_generation = self.theme_preview_generation;
        themes::begin_theme_preview(cx);
        self.theme_palette.update(cx, |palette, cx| {
            palette.set_query("", window, cx);
        });
        cx.notify();

        let theme_palette = self.theme_palette.clone();
        let story_root = cx.weak_entity();
        let focus_on_mount = Rc::new(Cell::new(true));
        window.open_dialog(cx, move |dialog, _, _| {
            let theme_palette = theme_palette.clone();
            let close_owner = story_root.clone();
            let content_owner = story_root.clone();
            let focus_on_mount = focus_on_mount.clone();
            dialog
                .close_button(false)
                .overlay_closable(false)
                .p_0()
                .on_close(move |_, _, cx| {
                    _ = close_owner.update(cx, |root, cx| root.finish_theme_preview(cx));
                })
                .content(move |content, window, cx| {
                    if focus_on_mount.replace(false) {
                        let theme_palette = theme_palette.clone();
                        window.defer(cx, move |window, cx| {
                            theme_palette.read(cx).focus_handle(cx).focus(window, cx);
                        });
                    }
                    let entries = content_owner
                        .read_with(cx, |root, _| root.theme_entries.clone())
                        .unwrap_or_default();
                    let select_owner = content_owner.clone();
                    let confirm_owner = content_owner.clone();
                    content.child(with_command_entries(
                        Command::new(&theme_palette)
                            .bordered(false)
                            .placeholder("Search themes...")
                            .max_h(px(400.))
                            .on_select(move |index, _, cx| {
                                _ = select_owner.update(cx, |root, cx| {
                                    root.on_theme_palette_select(
                                        index,
                                        theme_preview_generation,
                                        cx,
                                    );
                                });
                            })
                            .on_confirm(move |_, window, cx| {
                                _ = confirm_owner.update(cx, |root, cx| {
                                    root.on_theme_palette_confirm(window, cx);
                                });
                            }),
                        entries,
                    ))
                })
        });
    }

    fn on_action_open_command_palette(
        &mut self,
        _: &OpenCommandPalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(gallery) = self.gallery.as_ref() else {
            cx.propagate();
            return;
        };
        if self.component_palette_open {
            self.component_palette
                .read(cx)
                .focus_handle(cx)
                .focus(window, cx);
            return;
        }

        self.component_entries = gallery.read(cx).command_entries(cx);
        self.component_palette_open = true;
        self.component_palette.update(cx, |palette, cx| {
            palette.set_query("", window, cx);
        });
        cx.notify();

        let component_palette = self.component_palette.clone();
        let story_root = cx.weak_entity();
        let focus_on_mount = Rc::new(Cell::new(true));
        window.open_dialog(cx, move |dialog, _, _| {
            let component_palette = component_palette.clone();
            let close_owner = story_root.clone();
            let content_owner = story_root.clone();
            let focus_on_mount = focus_on_mount.clone();
            dialog
                .close_button(false)
                .overlay_closable(false)
                .p_0()
                .on_close(move |_, _, cx| {
                    _ = close_owner.update(cx, |root, _| root.component_palette_open = false);
                })
                .content(move |content, window, cx| {
                    if focus_on_mount.replace(false) {
                        let component_palette = component_palette.clone();
                        window.defer(cx, move |window, cx| {
                            component_palette
                                .read(cx)
                                .focus_handle(cx)
                                .focus(window, cx);
                        });
                    }
                    let entries = content_owner
                        .read_with(cx, |root, _| root.component_entries.clone())
                        .unwrap_or_default();
                    let confirm_owner = content_owner.clone();
                    content.child(with_command_entries(
                        Command::new(&component_palette)
                            .bordered(false)
                            .placeholder("Search components...")
                            .max_h(px(400.))
                            .on_confirm(move |index, window, cx| {
                                _ = confirm_owner.update(cx, |root, cx| {
                                    root.on_component_palette_confirm(index, window, cx);
                                });
                            }),
                        entries,
                    ))
                })
        });
    }

    fn on_action_toggle_search(
        &mut self,
        _: &ToggleSearch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.propagate();
        if window.has_focused_input(cx) {
            return;
        }

        struct Search;
        let note = Notification::new()
            .message("You have toggled search.")
            .id::<Search>();
        window.push_notification(note, cx);
    }
}

impl Focusable for StoryRoot {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for StoryRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sheet_layer = Root::render_sheet_layer(window, cx);
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);
        let show_fps = AppState::global(cx).show_fps_monitor;

        div()
            .id("story-root")
            .on_action(cx.listener(Self::on_action_panel_info))
            .on_action(cx.listener(Self::on_action_select_theme))
            .on_action(cx.listener(Self::on_action_open_command_palette))
            .on_action(cx.listener(Self::on_action_toggle_search))
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .when(!self.embedded, |this| this.child(self.title_bar.clone()))
                    .child(
                        div()
                            .track_focus(&self.focus_handle)
                            .relative()
                            .flex_1()
                            .overflow_hidden()
                            .child(self.view.clone()),
                    )
                    .children(sheet_layer)
                    .children(dialog_layer)
                    .children(notification_layer),
            )
            .relative()
            // FPS must be the last sibling so notification/toast layers cannot
            // paint over the HUD.
            .when(show_fps, |this| {
                this.child(
                    div()
                        .absolute()
                        .top(TITLE_BAR_HEIGHT)
                        .left_0()
                        .right_0()
                        .child(fps_monitor(window, cx)),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    #[test]
    fn component_story_window_is_opaque() {
        assert_eq!(
            super::story_window_background(),
            gpui_kit::WindowBackgroundAppearance::Opaque
        );
    }

    #[test]
    fn extends_component_translations_with_story_locales() {
        super::extend_component_translations();

        assert_eq!(
            gpui_kit::component::_rust_i18n_try_translate("fr", "Calendar.month.January"),
            Some("Janvier".into())
        );
        assert_eq!(
            gpui_kit::component::_rust_i18n_try_translate("en", "Calendar.month.January"),
            Some("January".into())
        );
    }
}
