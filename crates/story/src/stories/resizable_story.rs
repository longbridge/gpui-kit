use gpui_kit::component::{
    ActiveTheme, Sizable as _,
    button::Button,
    resizable::{ResizableState, h_resizable, resizable_panel, v_resizable},
    v_flex,
};
use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement as _, Pixels, Render, SharedString, Styled, Window, div, px,
};

use crate::{section, story_toolbar_group};

pub struct ResizableStory {
    focus_handle: FocusHandle,
    show_left: bool,
    use_flex_none: bool,
    programmatic_state: Entity<ResizableState>,
}

impl super::Story for ResizableStory {
    fn title() -> &'static str {
        "Resizable"
    }

    fn description() -> &'static str {
        "The resizable panels."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ResizableStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl ResizableStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            show_left: true,
            use_flex_none: true,
            programmatic_state: cx.new(|_| ResizableState::default()),
        }
    }
}

fn panel_box(content: impl Into<SharedString>, _: &App) -> AnyElement {
    div()
        .p_4()
        .size_full()
        .child(content.into())
        .into_any_element()
}

impl Render for ResizableStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .gap_6()
            .child(
                section("Nested Panels")
                    .description(
                        "Combines horizontal and vertical splits with constrained panel sizes.",
                    )
                    .w_full()
                    .child(
                        div()
                            .w_full()
                            .h(px(600.))
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                v_resizable("resizable-1")
                                    .on_resize(|state, _, cx| {
                                        println!("Resized: {:?}", state.read(cx).sizes());
                                    })
                                    .child(
                                        h_resizable("resizable-1.1")
                                            .child(
                                                resizable_panel()
                                                    .size(px(150.))
                                                    .size_range(px(120.)..px(300.))
                                                    .child(panel_box("Left (120px .. 300px)", cx)),
                                            )
                                            .child(panel_box("Center", cx))
                                            .child(
                                                resizable_panel()
                                                    .size(px(300.))
                                                    .child(panel_box("Right", cx)),
                                            ),
                                    )
                                    .child(panel_box("Center", cx))
                                    .child(
                                        resizable_panel()
                                            .size(px(80.))
                                            .size_range(px(80.)..Pixels::MAX)
                                            .child(panel_box("Bottom (80px .. 150px)", cx)),
                                    ),
                            ),
                    ),
            )
            .child(
                section("Growing Panel")
                    .description(
                        "A flexible panel absorbs the space left by a constrained neighbor.",
                    )
                    .w_full()
                    .child(
                        div()
                            .w_full()
                            .h(px(400.))
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                h_resizable("resizable-3")
                                    .child(
                                        resizable_panel()
                                            .size(px(200.))
                                            .size_range(px(200.)..px(400.))
                                            .child(panel_box("Left 2", cx)),
                                    )
                                    .child(panel_box("Right (Grow)", cx)),
                            ),
                    ),
            )
            // Demonstrates `.flex_none()`. Toggle the left panel's
            // visibility while watching the right panel's width:
            //
            // - `Use flex_none = true` (default): right panel holds
            //   its width; the center flex panel absorbs the freed
            //   space.
            // - `Use flex_none = false`: right panel inherits the
            //   internal `flex_grow: 1` and absorbs half of the
            //   freed slack alongside the center.
            //
            // Use the two buttons to record before/after for the same
            // layout in a single session.
            .child(
                section("Flex Behavior")
                    .description("Compare fixed and growing panels while toggling visibility.")
                    .w_full()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_2()
                            .child(
                                story_toolbar_group()
                                    .child(
                                        Button::new("toggle-left")
                                            .outline()
                                            .label(if self.show_left {
                                                "Hide Left"
                                            } else {
                                                "Show Left"
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.show_left = !this.show_left;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("toggle-flex-none")
                                            .outline()
                                            .label(if self.use_flex_none {
                                                "Use flex_none: ON"
                                            } else {
                                                "Use flex_none: OFF"
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.use_flex_none = !this.use_flex_none;
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .h(px(200.))
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .child(
                                        h_resizable("resizable-flex-none")
                                            .child({
                                                let mut p = resizable_panel()
                                                    .visible(self.show_left)
                                                    .size(px(200.))
                                                    .size_range(px(150.)..px(400.))
                                                    .child(panel_box("Left", cx));
                                                if self.use_flex_none {
                                                    p = p.flex_none();
                                                }
                                                p
                                            })
                                            .child(panel_box("Center", cx))
                                            .child({
                                                let mut p = resizable_panel()
                                                    .size(px(280.))
                                                    .size_range(px(200.)..px(400.))
                                                    .child(panel_box("Right", cx));
                                                if self.use_flex_none {
                                                    p = p.flex_none();
                                                }
                                                p
                                            }),
                                    ),
                            ),
                    ),
            )
            // Programmatic resize: drive panel sizes via
            // `ResizableState::resize_panel(ix, size, window, cx)`. Buttons
            // mutate the shared state; subscribers (none here) would observe
            // a `ResizablePanelEvent::Resized` just like a drag-finish.
            .child(
                section("Programmatic Resize")
                    .description("Panel sizes can be changed by actions as well as dragging.")
                    .w_full()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_2()
                            .child(
                                story_toolbar_group()
                                    .child(
                                        Button::new("compact-left")
                                            .small()
                                            .label("Compact left → 100")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.programmatic_state.update(cx, |state, cx| {
                                                    state.resize_panel(0, px(100.), window, cx);
                                                });
                                            })),
                                    )
                                    .child(
                                        Button::new("reset-left")
                                            .small()
                                            .label("Reset left → 200")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.programmatic_state.update(cx, |state, cx| {
                                                    state.resize_panel(0, px(200.), window, cx);
                                                });
                                            })),
                                    )
                                    .child(
                                        Button::new("compact-right")
                                            .small()
                                            .label("Compact right → 80")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.programmatic_state.update(cx, |state, cx| {
                                                    state.resize_panel(2, px(80.), window, cx);
                                                });
                                            })),
                                    )
                                    .child(
                                        Button::new("reset-right")
                                            .small()
                                            .label("Reset right → 200")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.programmatic_state.update(cx, |state, cx| {
                                                    state.resize_panel(2, px(200.), window, cx);
                                                });
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .h(px(200.))
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .child(
                                        h_resizable("resizable-programmatic")
                                            .with_state(&self.programmatic_state)
                                            .child(
                                                resizable_panel()
                                                    .size(px(200.))
                                                    .child(panel_box("Left", cx)),
                                            )
                                            .child(panel_box("Center (grow)", cx))
                                            .child(
                                                resizable_panel()
                                                    .size(px(200.))
                                                    .child(panel_box("Right", cx)),
                                            ),
                                    ),
                            ),
                    ),
            )
    }
}
