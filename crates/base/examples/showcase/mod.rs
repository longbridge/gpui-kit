mod components;
// Components and Motion are standalone example apps. The WASM host embeds
// both, so each deliberately instantiates its own thread-local active palette.
#[allow(clippy::duplicate_mod)]
#[path = "../shared/palette.rs"]
mod palette;
mod syntect_highlighter;

use gpui::{
    AnyElement, App, AppContext as _, Application, Context, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, ScrollHandle, StatefulInteractiveElement as _, Styled as _, Window,
    WindowOptions, actions, div, prelude::FluentBuilder as _, px, size,
};
#[cfg(not(target_family = "wasm"))]
use gpui::{KeyBinding, WindowBounds};
use gpui_base::ResizeHandleContext;
use gpui_base::dock::{
    DockArea, DockAreaRenderer, DockContext, DockLayout, DockPlacement, DropIndicator, NodeId,
    Panel, PanelEvent, PanelView, TabGroupContext, TabGroupRenderer, TileContext, TilesRenderer,
};
use gpui_base::input::InputEditorStyle;
use gpui_base::input::{EditorState, InputState, TextareaState};
use gpui_base::slider::SliderState;
use gpui_base::{
    Accordion, AccordionHeader, AccordionItem, AccordionPanel, AccordionTrigger, AlertDialog,
    AlertDialogAction, AlertDialogBackdrop, AlertDialogCancel, AlertDialogDescription,
    AlertDialogPopup, AlertDialogTitle, AutoScroll, Avatar, AvatarFallback, Button, Calendar,
    CalendarItemKind, CalendarState, Checkbox, CheckboxIndicator, CheckboxState, Collapsible,
    ColorPicker, ColorPickerState, ColorSwatch, Combobox, DatePicker, Dialog, DialogBackdrop,
    DialogDescription, DialogPopup, DialogTitle, Editor, HoverCard, Input, InputBase, NavMotion,
    NavOperation, NavStack, NavStackState, OtpState, Popup, Scrollbar, ScrollbarMode, Select,
    Sheet, Slider, SliderIndicator, SliderThumb, SliderTrack, Switch, SwitchThumb, SwitchTrack,
    Tab, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, Tabs, TextSelectionEvent,
    TextSelectionHandle, TextSelectionLayer, TextViewState, Textarea, Toast, ToastTransitionStatus,
    Toggle, ToggleGroup, Tooltip, Tree, TreeItem, TreeState, VirtualListScrollHandle,
    v_virtual_list,
};
use palette::{activate as activate_palette, canvas as example_canvas, example_rgb};
#[cfg(target_family = "wasm")]
use std::borrow::Cow;
use std::{rc::Rc, sync::Arc};
use syntect_highlighter::{ShowcaseHighlightStyles, SyntectHighlighter};

actions!(base_showcase, [Quit]);

const EDITOR_EXAMPLE: &str = r#"use std::collections::HashMap;

#[derive(Debug, Clone)]
struct Workspace {
    name: String,
    files: HashMap<String, usize>,
}

impl Workspace {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            files: HashMap::new(),
        }
    }

    fn index(&mut self, path: &str, lines: usize) {
        // Keep the latest line count for each source file.
        self.files.insert(path.to_owned(), lines);
    }

    fn summary(&self) -> String {
        let total: usize = self.files.values().sum();
        format!("{}: {} files, {total} lines", self.name, self.files.len())
    }
}

fn main() {
    let mut workspace = Workspace::new("gpui-component");
    workspace.index("src/main.rs", 128);
    workspace.index("src/editor.rs", 372);
    println!("{}", workspace.summary());
}
"#;

pub const COMPONENTS: &[&str] = &[
    "accordion",
    "alert-dialog",
    "avatar",
    "button",
    "calendar",
    "checkbox",
    "collapsible",
    "color-picker",
    "combobox",
    "date-picker",
    "dialog",
    "dock",
    "editor",
    "hover-card",
    "input",
    "link",
    "nav-stack",
    "number-input",
    "otp-input",
    "pagination",
    "popover",
    "popup",
    "progress",
    "radio",
    "radio-group",
    "resizable",
    "scrollbar",
    "select",
    "sheet",
    "slider",
    "switch",
    "table",
    "tabs",
    "text-selection",
    "text-view",
    "textarea",
    "toast",
    "toggle",
    "toggle-group",
    "tooltip",
    "tree",
    "virtual-list",
];

pub struct BaseShowcase {
    /// The shell: the overview at the root, and the component opened from it
    /// above. A showcase started on one component has that page as its root.
    pages: gpui::Entity<NavStackState>,
    checkbox_checked: bool,
    radio_selected: usize,
    switch_checked: bool,
    toggle_pressed: bool,
    toggle_group_selection: u8,
    selected_tab: usize,
    select_open: bool,
    select_index: usize,
    sheet_open: bool,
    toast_visible: bool,
    tooltip_visible: bool,
    accordion_items: [bool; 3],
    alert_dialog_open: bool,
    collapsible_open: bool,
    combobox_open: bool,
    combobox_query: gpui::Entity<InputState>,
    combobox_selection: String,
    color_picker: gpui::Entity<ColorPickerState>,
    date_open: bool,
    dialog_open: bool,
    popup_open: bool,
    page: usize,
    slider: gpui::Entity<SliderState>,
    stack: gpui::Entity<NavStackState>,
    input: gpui::Entity<InputState>,
    textarea: gpui::Entity<TextareaState>,
    editor: gpui::Entity<EditorState>,
    otp: gpui::Entity<OtpState>,
    calendar: gpui::Entity<CalendarState>,
    tree: gpui::Entity<TreeState>,
    date_focus: gpui::FocusHandle,
    scroll: ScrollHandle,
    example_scroll: ScrollHandle,
    virtual_scroll: VirtualListScrollHandle,
    dock: gpui::Entity<DockArea>,
    text_selection_handles: [TextSelectionHandle; 4],
    text_selection_scroll: ScrollHandle,
    text_selection_auto_scroll: AutoScroll,
    text_selection_active: bool,
    text_selection_text: String,
    text_view: gpui::Entity<TextViewState>,
    #[cfg(test)]
    text_selection_footer_bounds: Rc<std::cell::RefCell<Option<gpui::Bounds<gpui::Pixels>>>>,
}

impl BaseShowcase {
    pub fn new(component: impl Into<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        activate_palette(window, cx);
        let component = component.into();
        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder("Type something…")
                .default_value(if component == "number-input" {
                    "12"
                } else {
                    "Hello GPUI"
                });
            state.set_editor_style(InputEditorStyle {
                foreground: example_rgb(0x171717).into(),
                muted_foreground: example_rgb(0x737373).into(),
                selection: gpui::hsla(0.6, 0.8, 0.7, 0.45),
                caret: example_rgb(0x171717).into(),
                ..InputEditorStyle::default()
            });
            state
        });
        let otp = cx.new(|cx| OtpState::new(6, window, cx).default_value("12"));
        let textarea = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(3)
                .default_value("Build focused interfaces.\nKeep behavior composable.")
        });
        let textarea_base = textarea.clone();
        textarea_base.update(cx, |state, _| {
            state.set_editor_style(InputEditorStyle {
                foreground: example_rgb(0x171717).into(),
                muted_foreground: example_rgb(0x737373).into(),
                selection: gpui::hsla(0.6, 0.8, 0.7, 0.45),
                caret: example_rgb(0x171717).into(),
                ..InputEditorStyle::default()
            });
        });
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("rust")
                .line_number(true)
                .folding(true)
                .show_whitespaces(true)
                .default_value(EDITOR_EXAMPLE)
        });
        let editor_base = editor.clone();
        editor_base.update(cx, |state, cx| {
            state.set_highlighter_factory(
                Rc::new(|language| {
                    SyntectHighlighter::new(language)
                        .map(|highlighter| Box::new(highlighter) as Box<_>)
                }),
                cx,
            );
            state.set_editor_style(InputEditorStyle {
                foreground: example_rgb(0x171717).into(),
                muted_foreground: example_rgb(0x737373).into(),
                selection: gpui::hsla(0.6, 0.8, 0.7, 0.45),
                caret: example_rgb(0x171717).into(),
                highlight_styles: Arc::new(ShowcaseHighlightStyles),
                ..InputEditorStyle::default()
            });
        });
        let combobox_query = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Search frameworks…");
            state.set_editor_style(InputEditorStyle {
                foreground: example_rgb(0x171717).into(),
                muted_foreground: example_rgb(0x737373).into(),
                selection: gpui::hsla(0.6, 0.8, 0.7, 0.45),
                caret: example_rgb(0x171717).into(),
                ..InputEditorStyle::default()
            });
            state
        });
        cx.subscribe(
            &combobox_query,
            |_, _, _: &gpui_base::input::InputEvent, cx| cx.notify(),
        )
        .detach();
        if matches!(component.as_str(), "input" | "number-input") {
            input.update(cx, |state, cx| state.focus(window, cx));
        } else if component == "textarea" {
            textarea.update(cx, |state, cx| state.focus(window, cx));
        } else if component == "editor" {
            editor.update(cx, |state, cx| state.focus(window, cx));
        } else if component == "otp-input" {
            otp.update(cx, |state, cx| state.focus(window, cx));
        }

        let slider = cx.new(|_| SliderState::new().min(0.).max(100.).default_value(64.));
        cx.observe(&slider, |_, _, cx| cx.notify()).detach();

        let stack = cx.new(|_| NavStackState::new());
        stack.update(cx, |state, cx| {
            let root = cx.new(|_| components::ShowcasePage::new(1, stack.downgrade()));
            state.push(root, NavMotion::Immediate, cx);
        });
        cx.observe(&stack, |_, _, cx| cx.notify()).detach();

        let color_picker =
            cx.new(|cx| ColorPickerState::new(window, cx).default_value(example_rgb(0x2563eb)));
        cx.observe(&color_picker, |_, _, cx| cx.notify()).detach();

        let text_selection_handles = [
            TextSelectionHandle::new("", cx),
            TextSelectionHandle::new("", cx),
            TextSelectionHandle::new("", cx),
            TextSelectionHandle::new("", cx),
        ];
        let text_selection_scroll = ScrollHandle::new();
        for selection in &text_selection_handles {
            selection.refresh_window_on_change(window, cx).detach();
            let view = cx.entity().downgrade();
            selection
                .subscribe(
                    move |event, cx| {
                        let TextSelectionEvent::AutoScroll(delta) = event else {
                            return;
                        };
                        let delta = *delta;
                        _ = view.update(cx, |this, cx| {
                            this.text_selection_auto_scroll
                                .set(delta, cx, |delta, this, cx| {
                                    let offset = this.text_selection_scroll.offset();
                                    this.text_selection_scroll
                                        .set_offset(gpui::point(offset.x, offset.y - delta));
                                    cx.notify();
                                });
                        });
                    },
                    cx,
                )
                .detach();
        }

        let showcase = cx.weak_entity();
        let pages = cx.new(|_| NavStackState::new());
        pages.update(cx, |pages, cx| {
            let root = cx.new(|_| ComponentPage::new(component, showcase));
            pages.push(root, NavMotion::Immediate, cx);
        });
        cx.observe(&pages, |_, _, cx| cx.notify()).detach();

        let this = Self {
            pages,
            checkbox_checked: true,
            radio_selected: 0,
            switch_checked: true,
            toggle_pressed: true,
            toggle_group_selection: 0,
            selected_tab: 0,
            select_open: false,
            select_index: 0,
            sheet_open: false,
            toast_visible: false,
            tooltip_visible: false,
            accordion_items: [true, false, false],
            alert_dialog_open: false,
            collapsible_open: false,
            combobox_open: false,
            combobox_query,
            combobox_selection: "Select framework".into(),
            color_picker,
            date_open: false,
            dialog_open: false,
            popup_open: false,
            page: 3,
            slider,
            stack,
            input,
            textarea,
            editor,
            otp,
            calendar: cx.new(|cx| CalendarState::new(window, cx)),
            tree: cx.new(|cx| {
                TreeState::new(cx).items(vec![
                    TreeItem::new("src", "src").expanded(true).children(vec![
                        TreeItem::new("components", "components")
                            .expanded(true)
                            .children(vec![
                                TreeItem::new("button", "button.rs"),
                                TreeItem::new("tree-file", "tree.rs"),
                            ]),
                        TreeItem::new("lib", "lib.rs"),
                    ]),
                    TreeItem::new("examples", "examples")
                        .children(vec![TreeItem::new("showcase", "showcase.rs")]),
                    TreeItem::new("cargo", "Cargo.toml"),
                ])
            }),
            date_focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            example_scroll: ScrollHandle::new(),
            virtual_scroll: VirtualListScrollHandle::new(),
            dock: components::build_dock(window, cx),
            text_selection_handles,
            text_selection_scroll,
            text_selection_auto_scroll: AutoScroll::default(),
            text_selection_active: false,
            text_selection_text: String::new(),
            text_view: cx.new(|cx| TextViewState::markdown(components::TEXT_VIEW_MARKDOWN, cx)),
            #[cfg(test)]
            text_selection_footer_bounds: Rc::new(std::cell::RefCell::new(None)),
        };
        cx.observe_window_appearance(window, |this, window, cx| {
            activate_palette(window, cx);
            this.refresh_editor_styles(cx);
            cx.notify();
        })
        .detach();
        this
    }

    fn refresh_editor_styles(&self, cx: &mut Context<Self>) {
        let style = || InputEditorStyle {
            foreground: example_rgb(0x171717).into(),
            muted_foreground: example_rgb(0x737373).into(),
            selection: gpui::hsla(0.6, 0.8, 0.7, 0.45),
            caret: example_rgb(0x171717).into(),
            ..InputEditorStyle::default()
        };
        self.input
            .update(cx, |state, _| state.set_editor_style(style()));
        self.textarea
            .update(cx, |state, _| state.set_editor_style(style()));
        self.combobox_query
            .update(cx, |state, _| state.set_editor_style(style()));
        self.editor.update(cx, |state, _| {
            state.set_editor_style(InputEditorStyle {
                highlight_styles: Arc::new(ShowcaseHighlightStyles),
                ..style()
            });
        });
    }

    fn overview(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let showcase = cx.weak_entity();
        let pages = self.pages.clone();
        div()
            .w(px(720.))
            .max_w_full()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("GPUI Base"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(example_rgb(0x737373))
                            .child("Choose a component to open its interactive example."),
                    ),
            )
            .child(div().w_full().grid().grid_cols(3).gap_1().children(
                COMPONENTS.iter().enumerate().map(|(ix, name)| {
                    let showcase = showcase.clone();
                    let pages = pages.clone();
                    Button::new(("overview-item", ix))
                        .h_9()
                        .px_3()
                        .flex()
                        .items_center()
                        .justify_start()
                        .border_1()
                        .border_color(example_rgb(0xd4d4d4))
                        .bg(example_rgb(0xffffff))
                        .text_xs()
                        .child(*name)
                        .on_click(move |_, _, cx| {
                            pages.update(cx, |pages, cx| {
                                let page = cx.new(|_| ComponentPage::new(*name, showcase.clone()));
                                pages.push(page, NavMotion::Animated, cx);
                            });
                        })
                }),
            ))
    }
}

impl Render for BaseShowcase {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        activate_palette(window, cx);
        let (depth, has_forward) = {
            let pages = self.pages.read(cx);
            (pages.depth(), pages.forward_views().len() > 0)
        };
        let show_bar = depth > 1 || has_forward;
        let pages = self.pages.clone();
        let nav_button = |id: &'static str, label: &'static str| {
            Button::new(id)
                .h_7()
                .px_2()
                .flex()
                .items_center()
                .justify_center()
                .border_1()
                .border_color(example_rgb(0x171717))
                .child(label)
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(example_canvas())
            .text_color(example_rgb(0x171717))
            .text_xs()
            .font_family("Inter Variable")
            .child(TextSelectionLayer)
            .when(show_bar, |this| {
                this.child(
                    div()
                        .h_10()
                        .flex_none()
                        .px_3()
                        .flex()
                        .items_center()
                        .gap_2()
                        .border_b_1()
                        .border_color(example_rgb(0xe5e5e5))
                        .when(depth > 1, |this| {
                            let pages = pages.clone();
                            this.child(nav_button("back", "Back").on_click(move |_, _, cx| {
                                pages.update(cx, |pages, cx| {
                                    pages.pop(NavMotion::Animated, cx);
                                });
                            }))
                        })
                        .when(has_forward, |this| {
                            let pages = pages.clone();
                            this.child(nav_button("forward", "Forward").on_click(
                                move |_, _, cx| {
                                    pages.update(cx, |pages, cx| {
                                        pages.forward(NavMotion::Animated, cx);
                                    });
                                },
                            ))
                        }),
                )
            })
            .child(
                NavStack::new(&self.pages)
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .transition(gpui_base::motion::Transition::new(
                        std::time::Duration::from_millis(220),
                    ))
                    .item(|page, _, _| components::slide(page)),
            )
    }
}

impl BaseShowcase {
    /// One page of the shell: a component's example in its scroll area, or
    /// the overview. Called from [`ComponentPage`], which is why it takes the
    /// component rather than reading a field.
    fn render_page(
        &mut self,
        component: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let content = match component {
            "accordion" => self.accordion(cx).into_any_element(),
            "alert-dialog" => self.alert_dialog(cx).into_any_element(),
            "avatar" => self.avatar().into_any_element(),
            "button" => self.button().into_any_element(),
            "calendar" => self.calendar().into_any_element(),
            "checkbox" => self.checkbox(cx).into_any_element(),
            "collapsible" => self.collapsible(cx).into_any_element(),
            "color-picker" => self.color_picker(window, cx).into_any_element(),
            "combobox" => self.combobox(window, cx).into_any_element(),
            "date-picker" => self.date_picker(cx).into_any_element(),
            "dialog" => self.dialog(cx).into_any_element(),
            "editor" => self.editor().into_any_element(),
            "hover-card" => self.hover_card().into_any_element(),
            "input" => self.input().into_any_element(),
            "link" => self.link().into_any_element(),
            "number-input" => self.number_input(cx).into_any_element(),
            "otp-input" => self.otp_input(cx).into_any_element(),
            "pagination" => self.pagination(cx).into_any_element(),
            "popover" => self.popover().into_any_element(),
            "popup" => self.popup(cx).into_any_element(),
            "progress" => self.progress().into_any_element(),
            "radio" => self.radio(cx).into_any_element(),
            "radio-group" => self.radio_group(cx).into_any_element(),
            "resizable" => self.resizable().into_any_element(),
            "scrollbar" => self.scrollbar().into_any_element(),
            "slider" => self.slider(cx).into_any_element(),
            "nav-stack" => self.nav_stack().into_any_element(),
            "select" => self.select(false, cx).into_any_element(),
            "sheet" => self.sheet(cx).into_any_element(),
            "switch" => self.switch(cx).into_any_element(),
            "table" => self.table().into_any_element(),
            "tabs" => self.tabs(cx).into_any_element(),
            "text-selection" => self.text_selection(window, cx).into_any_element(),
            "text-view" => self.text_view(window).into_any_element(),
            "textarea" => self.textarea().into_any_element(),
            "toast" => self.toast(cx).into_any_element(),
            "toggle" => self.toggle(cx).into_any_element(),
            "toggle-group" => self.toggle_group(cx).into_any_element(),
            "tooltip" => self.tooltip(cx).into_any_element(),
            "tree" => self.tree().into_any_element(),
            "dock" => self.dock(cx).into_any_element(),
            "virtual-list" => self.virtual_list(cx).into_any_element(),
            _ => self.overview(cx).into_any_element(),
        };
        // Surfaces rather than parts: these take the whole viewport.
        let fills_viewport = component == "dock";
        let is_text_view = component == "text-view";
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("showcase-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .child(
                        div()
                            .min_h_full()
                            .w_full()
                            .flex()
                            // Most examples are small parts, centered in the
                            // viewport. A few are whole surfaces that have to
                            // fill it instead: centering them inside a
                            // `flex_none` box leaves a percentage size with
                            // nothing to resolve against, and it collapses.
                            .when(!fills_viewport, |this| this.items_center().justify_center())
                            .p_4()
                            .child(
                                div()
                                    .map(|this| match (fills_viewport, is_text_view) {
                                        (true, _) => this.flex_1().size_full().min_h(px(420.)),
                                        (false, true) => this.flex_1().w_full().max_w(px(720.)),
                                        (false, false) => this.flex_none(),
                                    })
                                    .child(content),
                            ),
                    ),
            )
            .into_any_element()
    }
}

/// A page of the shell. It holds nothing but the component's name: the
/// example's state lives on the showcase, and rendering goes back there.
struct ComponentPage {
    component: String,
    showcase: gpui::WeakEntity<BaseShowcase>,
}

impl ComponentPage {
    fn new(component: impl Into<String>, showcase: gpui::WeakEntity<BaseShowcase>) -> Self {
        Self {
            component: component.into(),
            showcase,
        }
    }
}

impl Render for ComponentPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let component = self.component.clone();
        self.showcase
            .update(cx, |showcase, cx| {
                showcase.render_page(&component, window, cx)
            })
            .unwrap_or_else(|_| div().into_any_element())
    }
}

pub fn run(app: Application, component: impl Into<String>) {
    let component = component.into();
    app.run(move |cx: &mut App| {
        gpui_base::init(cx);
        #[cfg(not(target_family = "wasm"))]
        {
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
        }
        #[cfg(target_family = "wasm")]
        cx.text_system()
            .add_fonts(vec![Cow::Borrowed(
                include_bytes!("../../../story-web/fonts/Inter-Regular.ttf").as_slice(),
            )])
            .expect("failed to load gpui-base example font");
        let options = WindowOptions {
            #[cfg(not(target_family = "wasm"))]
            window_bounds: Some(WindowBounds::centered(size(px(840.), px(640.)), cx)),
            ..WindowOptions::default()
        };
        cx.open_window(options, move |window, cx| {
            cx.new(|cx| BaseShowcase::new(component, window, cx))
        })
        .expect("failed to open gpui-base example window");
        cx.activate(true);
    });
}

#[cfg(target_family = "wasm")]
pub fn run_embedded(app: Application, component: impl Into<String>) -> gpui::ApplicationHandle {
    let component = component.into();
    app.run_embedded(move |cx: &mut App| {
        gpui_base::init(cx);
        cx.text_system()
            .add_fonts(vec![Cow::Borrowed(
                include_bytes!("../../../story-web/fonts/Inter-Regular.ttf").as_slice(),
            )])
            .expect("failed to load gpui-base example font");
        cx.open_window(WindowOptions::default(), move |window, cx| {
            cx.new(|cx| BaseShowcase::new(component, window, cx))
        })
        .expect("failed to open gpui-base example window");
        cx.activate(true);
    })
}

#[cfg(not(target_family = "wasm"))]
#[allow(dead_code)]
pub fn run_native(component: &str) {
    run(gpui_platform::application(), component.to_owned());
}
