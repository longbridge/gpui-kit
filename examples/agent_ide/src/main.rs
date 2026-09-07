mod terminal;

use gpui_kit::assets::Assets;
use gpui_kit::component::{
    ActiveTheme, Icon, IconName, Side, TitleBar, WindowExt,
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    button::{Button, ButtonVariants as _},
    input::{Editor, EditorState, Input, InputState, TabSize},
    scroll::ScrollableElement,
    sidebar::{
        Sidebar, SidebarCollapsible, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu,
        SidebarMenuItem, SidebarToggleButton,
    },
    switch::Switch,
    tab::{Tab, TabBar},
    text::TextView,
    *,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gpui_wry::WebView;
use raw_window_handle::HasWindowHandle;

const EDITOR_CODE: &str = r#"use gpui_kit::component::button::Button;
use gpui_kit::*;

pub struct Example;

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child(Button::new("ok").primary().label("Let's Go!"))
    }
}
"#;

/// A running agent inside a project folder.
struct Agent {
    name: &'static str,
    status: AgentStatus,
}

#[derive(Clone, Copy, PartialEq)]
enum AgentStatus {
    Running,
    Idle,
    Waiting,
    Stopped,
}

impl AgentStatus {
    fn color(&self, cx: &App) -> Hsla {
        match self {
            Self::Running => cx.theme().success,
            Self::Idle => cx.theme().muted_foreground,
            Self::Waiting => cx.theme().warning,
            Self::Stopped => cx.theme().danger,
        }
    }
}

const CLAUDE: &str = "claude";
const CODEX: &str = "codex";
const PI: &str = "pi";
const GEMINI: &str = "gemini";

fn agent_icon(name: &str) -> IconName {
    match name {
        CLAUDE => IconName::Bot,
        CODEX => IconName::SquareTerminal,
        PI => IconName::Cpu,
        _ => IconName::Globe,
    }
}

/// A rounded status pill for the session toolbar.
fn status_pill(label: &'static str, color: Hsla) -> impl IntoElement {
    div()
        .rounded_full()
        .px_2()
        .py(px(2.))
        .text_xs()
        .font_medium()
        .text_color(color)
        .bg(color.opacity(0.12))
        .child(label)
}

/// Center panes, shown as tabs in the workspace.
const PANE_AGENT: usize = 0;
const PANE_TERMINAL: usize = 1;
const PANE_EDITOR: usize = 2;
const PANE_BROWSER: usize = 3;

pub struct AgentIDE {
    left_collapsed: bool,
    right_collapsed: bool,
    active_pane: usize,
    prompt: Entity<InputState>,
    terminal: Entity<terminal::TerminalState>,
    editor: Entity<EditorState>,
    webview: Option<Entity<WebView>>,
}

impl AgentIDE {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("rust")
                .tab_size(TabSize {
                    tab_size: 4,
                    ..Default::default()
                })
                .default_value(EDITOR_CODE)
        });

        let prompt = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Ask the agent to do something…  (⏎ send)")
        });

        let terminal = cx.new(|cx| terminal::TerminalState::new(cx));

        // Native webview child; visible only while the Browser pane is active.
        let active_pane = PANE_AGENT;
        let webview = Self::create_webview(window, cx, active_pane == PANE_BROWSER);

        Self {
            left_collapsed: false,
            right_collapsed: false,
            active_pane,
            prompt,
            terminal,
            editor,
            webview,
        }
    }

    fn create_webview(
        window: &mut Window,
        cx: &mut Context<Self>,
        visible: bool,
    ) -> Option<Entity<WebView>> {
        let handle = window.window_handle().ok()?;
        let wry_webview = wry::WebViewBuilder::new()
            .with_url("https://gpui-kit.com")
            .build_as_child(&handle)
            .ok()?;
        let view = cx.new(|cx| WebView::new(wry_webview, window, cx));
        view.update(cx, |w, _| if visible { w.show() } else { w.hide() });
        Some(view)
    }

    fn set_pane(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.active_pane = ix;
        if ix == PANE_TERMINAL {
            let handle = self.terminal.read(cx).focus_handle.clone();
            window.focus(&handle, cx);
        }
        if let Some(webview) = &self.webview {
            webview.update(cx, |w, _| {
                if ix == PANE_BROWSER {
                    w.show()
                } else {
                    w.hide()
                }
            });
        }
        cx.notify();
    }

    fn project_item(
        &self,
        label: &'static str,
        agents: Vec<Agent>,
        active_agent: Option<&str>,
        cx: &mut Context<Self>,
    ) -> SidebarMenuItem {
        SidebarMenuItem::new(label)
            .icon(IconName::Folder)
            .default_open(true)
            .click_to_toggle(true)
            .children(agents.into_iter().map(move |agent| {
                let dot = agent.status.color(cx);
                SidebarMenuItem::new(agent.name)
                    .icon(agent_icon(agent.name))
                    .active(active_agent == Some(agent.name))
                    .suffix(move |_, _| {
                        div()
                            .size_1_5()
                            .rounded_full()
                            .flex_shrink_0()
                            .mr_1()
                            .bg(dot)
                    })
            }))
    }

    fn left_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Sidebar::new("left")
            .collapsible(SidebarCollapsible::Icon)
            .collapsed(self.left_collapsed)
            .w(px(250.))
            .header(
                SidebarHeader::new().child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size_7()
                                .flex_shrink_0()
                                .rounded(cx.theme().radius)
                                .bg(cx.theme().primary)
                                .text_color(cx.theme().primary_foreground)
                                .child(Icon::new(IconName::SquareTerminal).small()),
                        )
                        .when(!self.left_collapsed, |this| {
                            this.child(
                                v_flex()
                                    .overflow_hidden()
                                    .child(div().text_sm().font_bold().child("AgentIDE"))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("3 projects · 6 agents"),
                                    ),
                            )
                        }),
                ),
            )
            .child(
                SidebarGroup::new("Projects").child(SidebarMenu::new().children([
                    self.project_item(
                        "gpui-kit",
                        vec![
                            Agent {
                                name: CLAUDE,
                                status: AgentStatus::Running,
                            },
                            Agent {
                                name: CODEX,
                                status: AgentStatus::Idle,
                            },
                            Agent {
                                name: PI,
                                status: AgentStatus::Running,
                            },
                        ],
                        Some(CLAUDE),
                        cx,
                    ),
                    self.project_item(
                        "webapp",
                        vec![
                            Agent {
                                name: CLAUDE,
                                status: AgentStatus::Running,
                            },
                            Agent {
                                name: GEMINI,
                                status: AgentStatus::Waiting,
                            },
                        ],
                        None,
                        cx,
                    ),
                    self.project_item(
                        "ml-play",
                        vec![Agent {
                            name: CODEX,
                            status: AgentStatus::Stopped,
                        }],
                        None,
                        cx,
                    ),
                ])),
            )
            .footer(
                SidebarFooter::new().child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(IconName::Settings)
                        .when(!self.left_collapsed, |this| this.child("Settings")),
                ),
            )
    }

    fn center_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .px_3()
            .py_2()
            .child(
                SidebarToggleButton::new()
                    .collapsed(self.left_collapsed)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.left_collapsed = !this.left_collapsed;
                        cx.notify();
                    })),
            )
            .child(Breadcrumb::new().children([
                BreadcrumbItem::new("gpui-kit"),
                BreadcrumbItem::new("claude"),
                BreadcrumbItem::new("Fix flaky table test"),
            ]))
            .child(
                h_flex()
                    .ml_auto()
                    .gap_2()
                    .child(status_pill("Running", cx.theme().success))
                    .child(
                        Button::new("stop")
                            .ghost()
                            .icon(IconName::Pause)
                            .small()
                            .tooltip("Stop agent"),
                    )
                    .child(
                        Button::new("restart")
                            .ghost()
                            .icon(IconName::RotateCw)
                            .small()
                            .tooltip("Restart agent"),
                    )
                    .child(
                        Button::new("new-agent")
                            .small()
                            .icon(IconName::Plus)
                            .label("New Agent"),
                    ),
            )
    }

    /// The workspace tab bar above the center panes.
    fn pane_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        TabBar::new("panes")
            .w_full()
            .selected_index(self.active_pane)
            .on_click(cx.listener(|this, ix: &usize, window, cx| this.set_pane(*ix, window, cx)))
            .suffix(
                h_flex()
                    .mx_1()
                    .child(
                        Button::new("new-tab")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Plus)
                            .tooltip("New view"),
                    )
                    .child(
                        Button::new("tab-more")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Ellipsis),
                    ),
            )
            .child(
                Tab::new()
                    .prefix(Icon::new(IconName::Bot).small())
                    .label("Agent"),
            )
            .child(
                Tab::new()
                    .prefix(Icon::new(IconName::SquareTerminal).small())
                    .label("Terminal"),
            )
            .child(
                Tab::new()
                    .prefix(Icon::new(IconName::FileText).small())
                    .label("Editor"),
            )
            .child(
                Tab::new()
                    .prefix(Icon::new(IconName::Globe).small())
                    .label("Browser"),
            )
    }

    /// One agent turn: avatar, name/time, markdown bubble.
    fn agent_turn(
        &self,
        id: &'static str,
        when: &'static str,
        md: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .size_5()
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(cx.theme().accent)
                            .child(Icon::new(IconName::Bot).size_3()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("claude · {when}")),
                    ),
            )
            .child(
                div()
                    .max_w(rems(44.))
                    .rounded_lg()
                    .rounded_bl(px(0.))
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().secondary)
                    .p_3()
                    .child(TextView::markdown(id, md)),
            )
    }

    fn message_stream(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("stream")
            .flex_1()
            .min_h_0()
            .overflow_y_scrollbar()
            .gap_4()
            .p_4()
            .child(
                v_flex().w_full().items_end().child(
                    div()
                        .max_w(rems(36.))
                        .rounded_lg()
                        .rounded_br(px(0.))
                        .bg(cx.theme().primary)
                        .text_color(cx.theme().primary_foreground)
                        .px_3()
                        .py_2()
                        .child(
                            "The table test fails about 1 in 20 runs on CI. \
                             Find the race and fix it.",
                        ),
                ),
            )
            .child(self.agent_turn("m1", "just now", "Found it — `TableState::select_row` is called from both the click handler and the keyboard handler without a shared guard. Patch:\n\n```rust\nlet mut guard = self.selection.lock();\nguard.select(row);\n```", cx))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().muted)
                    .max_w(rems(44.))
                    .child(Icon::new(IconName::SquareTerminal).small())
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .child("cargo test -p gpui-component table"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("… 214/215 passed"),
                    )
                    .child(Icon::new(IconName::Loader).small()),
            )
            .child(self.agent_turn("m2", "2m", "Race fixed: selection now goes through one guarded path. **215/215** green across 30 CI runs.", cx))
    }

    fn composer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .items_center()
            .border_t_1()
            .border_color(cx.theme().border)
            .p_3()
            .child(
                div()
                    .flex_1()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().input)
                    .bg(cx.theme().secondary)
                    .px_2()
                    .py_1()
                    .child(Input::new(&self.prompt).appearance(false)),
            )
            .child(
                Button::new("send")
                    .primary()
                    .icon(IconName::ArrowUp)
                    .on_click(|_, window, cx| {
                        window.push_notification("Message sent (demo)", cx);
                    }),
            )
    }

    fn agent_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .min_h_0()
            .child(self.message_stream(cx))
            .child(self.composer(cx))
    }

    fn editor_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().flex_1().min_h_0().child(
            Editor::new(&self.editor)
                .text_size(px(13.))
                .size_full()
                .bg(cx.theme().secondary.opacity(0.4)),
        )
    }

    fn browser_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .min_h_0()
            .gap_1()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .px_2()
                    .py_1()
                    .child(
                        Button::new("back")
                            .ghost()
                            .xsmall()
                            .icon(IconName::ArrowLeft),
                    )
                    .child(
                        div()
                            .flex_1()
                            .rounded_md()
                            .border_1()
                            .border_color(cx.theme().input)
                            .bg(cx.theme().secondary)
                            .px_2()
                            .py_1()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("https://gpui-kit.com"),
                    )
                    .child(
                        Button::new("reload")
                            .ghost()
                            .xsmall()
                            .icon(IconName::RotateCw),
                    ),
            )
            // Native webview renders as a child view over this surface.
            .child(
                div().flex_1().min_h_0().children(
                    self.webview
                        .clone()
                        .map(|w| w.into_any_element())
                        .into_iter()
                        .chain(self.webview.is_none().then(|| {
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(cx.theme().muted_foreground)
                                .child("Webview unavailable in this build")
                                .into_any_element()
                        })),
                ),
            )
    }

    fn right_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Sidebar::new("right")
            .side(Side::Right)
            .collapsible(SidebarCollapsible::Icon)
            .collapsed(self.right_collapsed)
            .w(px(260.))
            .header(
                SidebarHeader::new().child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .when(!self.right_collapsed, |this| {
                            this.child(div().text_sm().font_bold().child("claude"))
                        })
                        .child(
                            Button::new("toggle-right")
                                .ghost()
                                .icon(if self.right_collapsed {
                                    IconName::PanelRightOpen
                                } else {
                                    IconName::PanelRightClose
                                })
                                .xsmall()
                                .ml_auto()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.right_collapsed = !this.right_collapsed;
                                    cx.notify();
                                })),
                        ),
                ),
            )
            .child(SidebarGroup::new("Session").children([
                SidebarMenuItem::new("Model: claude-fable-5-1"),
                SidebarMenuItem::new("Task: Fix flaky table test"),
                SidebarMenuItem::new("Uptime: 42m"),
                SidebarMenuItem::new("Tokens: 128k in / 34k out"),
            ]))
            .child(
                SidebarGroup::new("Tool permissions").children(
                    [("Bash", true), ("File edit", true), ("Web search", false)]
                        .into_iter()
                        .map(|(tool, enabled)| {
                            let id = SharedString::from(format!("tool-{tool}"));
                            SidebarMenuItem::new(tool).suffix(move |_, _| {
                                Switch::new(id.clone()).checked(enabled).xsmall().mr_1()
                            })
                        }),
                ),
            )
    }
}

impl Render for AgentIDE {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background.opacity(0.72))
            .child(
                TitleBar::new()
                    .bg(hsla(0., 0., 0., 0.))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("AgentIDE"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .ml_2()
                            .child("5 running · 1 waiting"),
                    ),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.left_sidebar(cx))
                    .child(
                        v_flex()
                            .h_full()
                            .flex_1()
                            .min_w_0()
                            .child(self.center_toolbar(cx))
                            .child(self.pane_tabs(cx))
                            .child(match self.active_pane {
                                PANE_TERMINAL => self.terminal.clone().into_any_element(),
                                PANE_EDITOR => self.editor_pane(cx).into_any_element(),
                                PANE_BROWSER => self.browser_pane(cx).into_any_element(),
                                _ => self.agent_pane(cx).into_any_element(),
                            }),
                    )
                    .child(self.right_sidebar(cx)),
            )
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        gpui_kit::init(cx);

        let window_options = WindowOptions {
            titlebar: None,
            window_decorations: Some(WindowDecorations::Client),
            window_background: WindowBackgroundAppearance::Blurred,
            window_bounds: Some(WindowBounds::centered(size(px(1280.), px(800.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| AgentIDE::new(window, cx));
                cx.new(|cx| {
                    Root::new(view, window, cx)
                        .bordered(false)
                        .bg(hsla(0., 0., 0., 0.))
                })
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
