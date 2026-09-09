use std::rc::Rc;

use gpui_kit::assets::Assets;
use gpui_kit::component::{
    ActiveTheme, Colorize as _, ElementExt, IconName, Sizable,
    button::Button,
    checkbox::Checkbox,
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex,
    slider::{Slider, SliderState},
    v_flex,
};
use gpui_kit::{StyleRefinement, prelude::FluentBuilder, *};

pub struct BrushStory {
    focus_handle: gpui_kit::FocusHandle,
    brush_size: Entity<SliderState>,
    brush_opacity: Entity<SliderState>,
    brush_color: Hsla,
    strokes: Rc<Vec<Stroke>>,
    current_stroke: Option<Stroke>,
    is_drawing: bool,
    show_grid: bool,
    canvas_bounds: Option<Bounds<Pixels>>,
}

#[derive(Clone, Debug)]
struct Stroke {
    points: Vec<Point<Pixels>>,
    color: Hsla,
    size: f32,
}

impl BrushStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let brush_size = cx.new(|_| {
            SliderState::new()
                .min(1.)
                .max(50.)
                .default_value(5.)
                .step(1.)
        });

        let brush_opacity = cx.new(|_| {
            SliderState::new()
                .min(0.1)
                .max(1.0)
                .default_value(1.0)
                .step(0.05)
        });

        Self {
            focus_handle: cx.focus_handle(),
            brush_size,
            brush_opacity,
            brush_color: black(),
            strokes: Rc::new(vec![]),
            current_stroke: None,
            is_drawing: false,
            show_grid: false,
            canvas_bounds: None,
        }
    }

    fn handle_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Left {
            self.is_drawing = true;
            let brush_size = self.brush_size.read(cx).value().start();
            let brush_opacity = self.brush_opacity.read(cx).value().start();
            let color = self.brush_color.opacity(brush_opacity);

            let local_pos = if let Some(bounds) = self.canvas_bounds {
                Point::new(
                    event.position.x - bounds.origin.x,
                    event.position.y - bounds.origin.y,
                )
            } else {
                event.position
            };

            self.current_stroke = Some(Stroke {
                points: vec![local_pos],
                color,
                size: brush_size,
            });
            cx.notify();
        }
    }

    fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_drawing {
            if let Some(ref mut stroke) = self.current_stroke {
                let local_pos = if let Some(bounds) = self.canvas_bounds {
                    Point::new(
                        event.position.x - bounds.origin.x,
                        event.position.y - bounds.origin.y,
                    )
                } else {
                    event.position
                };

                let should_add = if let Some(last) = stroke.points.last() {
                    let dx_px = local_pos.x - last.x;
                    let dy_px = local_pos.y - last.y;

                    let dx_abs = if dx_px < px(0.0) {
                        px(0.0) - dx_px
                    } else {
                        dx_px
                    };
                    let dy_abs = if dy_px < px(0.0) {
                        px(0.0) - dy_px
                    } else {
                        dy_px
                    };
                    dx_abs >= px(1.0) || dy_abs >= px(1.0)
                } else {
                    true
                };

                if should_add {
                    stroke.points.push(local_pos);
                    cx.notify();
                }
            }
        }
    }

    fn handle_mouse_up(&mut self, _event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_drawing {
            self.is_drawing = false;
            if let Some(stroke) = self.current_stroke.take() {
                if stroke.points.len() > 1 {
                    let mut new_strokes = (*self.strokes).clone();
                    new_strokes.push(stroke);
                    self.strokes = Rc::new(new_strokes);
                }
            }
            cx.notify();
        }
    }

    fn clear_canvas(&mut self, cx: &mut Context<Self>) {
        self.strokes = Rc::new(vec![]);
        self.current_stroke = None;
        self.is_drawing = false;
        cx.notify();
    }

    fn set_brush_color(&mut self, color: Hsla, cx: &mut Context<Self>) {
        self.brush_color = color;
        cx.notify();
    }
}

impl Focusable for BrushStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BrushStory {
    fn color_button(&self, color: Hsla, _label: &str, cx: &Context<Self>) -> impl IntoElement {
        let is_selected = self.brush_color.to_hex() == color.to_hex();
        let theme = cx.theme();

        div()
            .w(px(40.))
            .h(px(40.))
            .rounded(theme.radius)
            .bg(color)
            .border_2()
            .when(is_selected, |this| {
                this.border_color(theme.primary).shadow_md()
            })
            .when(!is_selected, |this| this.border_color(theme.border))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.set_brush_color(color, cx);
                }),
            )
    }

    fn render_canvas(&mut self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let strokes_for_prepaint = self.strokes.clone();
        let current_stroke_for_prepaint = self.current_stroke.clone();
        let show_grid_for_prepaint = self.show_grid;
        let theme_for_prepaint = theme.clone();

        let state_entity = cx.entity().clone();

        let base_div = div()
            .id("canvas")
            .size_full()
            .bg(theme.background)
            .cursor_crosshair()
            .relative()
            .on_mouse_down(MouseButton::Left, cx.listener(Self::handle_mouse_down))
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            .on_prepaint(move |bounds, _window, cx| {
                state_entity.update(cx, |state, _| {
                    state.canvas_bounds = Some(bounds);
                })
            });

        base_div.child(
            canvas(
                move |bounds, _window, _cx| {
                    (
                        strokes_for_prepaint,
                        current_stroke_for_prepaint,
                        show_grid_for_prepaint,
                        theme_for_prepaint,
                        bounds,
                    )
                },
                move |_bounds,
                      (strokes, current_stroke, show_grid, theme, prepaint_bounds),
                      window,
                      _cx| {
                    let origin = prepaint_bounds.origin;
                    let size = prepaint_bounds.size;

                    if show_grid {
                        let grid_color = theme.border.opacity(0.2);
                        let grid_size = 40.0;

                        let mut x = 0.0;
                        while px(x) <= size.width {
                            let mut builder = PathBuilder::stroke(px(1.0));
                            builder.move_to(Point::new(origin.x + px(x), origin.y));
                            builder.line_to(Point::new(origin.x + px(x), origin.y + size.height));
                            if let Ok(path) = builder.build() {
                                window.paint_path(path, grid_color);
                            }
                            x += grid_size;
                        }

                        let mut y = 0.0;
                        while px(y) <= size.height {
                            let mut builder = PathBuilder::stroke(px(1.0));
                            builder.move_to(Point::new(origin.x, origin.y + px(y)));
                            builder.line_to(Point::new(origin.x + size.width, origin.y + px(y)));
                            if let Ok(path) = builder.build() {
                                window.paint_path(path, grid_color);
                            }
                            y += grid_size;
                        }
                    }

                    for stroke in strokes.iter() {
                        if let Some(path) = BrushStory::build_stroke_path(stroke, &prepaint_bounds)
                        {
                            window.paint_path(path, stroke.color);
                        }
                    }

                    if let Some(ref stroke) = current_stroke {
                        if let Some(path) = BrushStory::build_stroke_path(stroke, &prepaint_bounds)
                        {
                            window.paint_path(path, stroke.color);
                        }
                    }
                },
            )
            .absolute()
            .size_full(),
        )
    }

    fn build_stroke_path(stroke: &Stroke, bounds: &Bounds<Pixels>) -> Option<Path<Pixels>> {
        if stroke.points.len() < 2 {
            return None;
        }

        let mut builder = PathBuilder::stroke(px(stroke.size));

        let first_point = Point::new(
            bounds.origin.x + stroke.points[0].x,
            bounds.origin.y + stroke.points[0].y,
        );
        builder.move_to(first_point);

        for point in stroke.points.iter().skip(1) {
            let abs_point = Point::new(bounds.origin.x + point.x, bounds.origin.y + point.y);
            builder.line_to(abs_point);
        }

        builder.build().ok()
    }
}

impl Render for BrushStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let brush_size = self.brush_size.read(cx).value().start();
        let brush_opacity = self.brush_opacity.read(cx).value().start();

        v_flex()
            .size_full()
            .gap_6()
            .child(
                section("Controls").child(
                    h_flex()
                        .gap_8()
                        .w_full()
                        .items_start()
                        .child(
                            v_flex()
                                .gap_4()
                                .flex_1()
                                .w(relative(0.5))
                                .child(
                                    h_flex()
                                        .gap_4()
                                        .items_center()
                                        .child("Size:")
                                        .child(
                                            Slider::new(&self.brush_size)
                                                .w(px(200.))
                                                .bg(theme.primary)
                                                .text_color(theme.primary_foreground),
                                        )
                                        .child(format!("{:.0}px", brush_size)),
                                )
                                .child(
                                    h_flex()
                                        .gap_4()
                                        .items_center()
                                        .child("Opacity:")
                                        .child(
                                            Slider::new(&self.brush_opacity)
                                                .w(px(200.))
                                                .bg(theme.primary)
                                                .text_color(theme.primary_foreground),
                                        )
                                        .child(format!("{:.0}%", brush_opacity * 100.0)),
                                )
                                .child(
                                    h_flex()
                                        .gap_3()
                                        .items_center()
                                        .child(
                                            Button::new("clear-canvas")
                                                .icon(IconName::Close)
                                                .label("Clear Canvas")
                                                .small()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.clear_canvas(cx);
                                                })),
                                        )
                                        .child(
                                            Checkbox::new("show-grid")
                                                .label("Show Grid")
                                                .checked(self.show_grid)
                                                .on_click(cx.listener(|this, checked, _, cx| {
                                                    this.show_grid = *checked;
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .flex_1()
                                .w(relative(0.5))
                                .child(h_flex().gap_2().items_center().child("Color:"))
                                .child(
                                    h_flex()
                                        .gap_3()
                                        .flex_wrap()
                                        .child(self.color_button(black(), "Black", cx))
                                        .child(self.color_button(white(), "White", cx))
                                        .child(self.color_button(red(), "Red", cx))
                                        .child(self.color_button(green(), "Green", cx))
                                        .child(self.color_button(blue(), "Blue", cx))
                                        .child(self.color_button(yellow(), "Yellow", cx))
                                        .child(self.color_button(
                                            hsla(0.58, 1.0, 0.5, 1.0),
                                            "Purple",
                                            cx,
                                        ))
                                        .child(self.color_button(
                                            hsla(0.083, 1.0, 0.5, 1.0),
                                            "Orange",
                                            cx,
                                        )),
                                ),
                        ),
                ),
            )
            .child(
                section("Drawing Canvas")
                    .child(self.render_canvas(cx))
                    .flex_1(),
            )
    }
}

fn section(title: impl Into<SharedString>) -> GroupBox {
    GroupBox::new()
        .outline()
        .title(
            h_flex()
                .justify_between()
                .w_full()
                .gap_4()
                .child(title.into()),
        )
        .content_style(StyleRefinement::default().flex_1().size_full())
}

pub struct Example {
    root: Entity<BrushStory>,
}

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let root = cx.new(|cx| BrushStory::new(window, cx));
        Self { root }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().p_4().size_full().child(self.root.clone())
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        gpui_component_story::init(cx);
        cx.activate(true);

        gpui_component_story::create_new_window("Brush Example", Example::view, cx);
    });
}
