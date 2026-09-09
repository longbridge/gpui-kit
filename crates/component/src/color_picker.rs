use gpui::{
    Anchor, App, ElementId, Entity, FocusHandle, Focusable, Hsla, InteractiveElement as _,
    IntoElement, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, TextAlign, Window, div, hsla, linear_color_stop, linear_gradient,
    prelude::FluentBuilder as _,
};
use rust_i18n::t;

use gpui_base::{ColorPicker as BaseColorPicker, ColorSwatch};
pub use gpui_base::{ColorPickerEvent, ColorPickerState};

use crate::{
    ActiveTheme as _, Colorize as _, Icon, Selectable, Sizable, Size, StyleSized, h_flex,
    input::Input,
    popover::Popover,
    separator::Separator,
    slider::Slider,
    tab::{Tab, TabBar},
    tooltip::{ManagedTooltipExt as _, Tooltip},
    v_flex,
};

fn color_palettes() -> Vec<Vec<Hsla>> {
    use crate::theme::DEFAULT_COLORS;
    use itertools::Itertools as _;

    macro_rules! c {
        ($color:tt) => {
            DEFAULT_COLORS
                .$color
                .keys()
                .sorted()
                .map(|k| DEFAULT_COLORS.$color.get(k).map(|c| c.hsla).unwrap())
                .collect::<Vec<_>>()
        };
    }

    vec![
        c!(stone),
        c!(red),
        c!(orange),
        c!(yellow),
        c!(green),
        c!(cyan),
        c!(blue),
        c!(purple),
        c!(pink),
    ]
}

/// A color picker element.
#[derive(IntoElement)]
pub struct ColorPicker {
    id: ElementId,
    style: StyleRefinement,
    state: Entity<ColorPickerState>,
    featured_colors: Option<Vec<Hsla>>,
    label: Option<SharedString>,
    /// The announced name, when the visible label is not it.
    accessibility_label: Option<SharedString>,
    icon: Option<Icon>,
    size: Size,
    anchor: Anchor,
}

impl ColorPicker {
    /// Create a new color picker element with the given [`ColorPickerState`].
    pub fn new(state: &Entity<ColorPickerState>) -> Self {
        Self {
            id: ("color-picker", state.entity_id()).into(),
            style: StyleRefinement::default(),
            state: state.clone(),
            featured_colors: None,
            size: Size::Medium,
            label: None,
            accessibility_label: None,
            icon: None,
            anchor: Anchor::TopLeft,
        }
    }

    /// Set the featured colors to be displayed in the color picker.
    ///
    /// This is used to display a set of colors that the user can quickly select from,
    /// for example provided user's last used colors.
    pub fn featured_colors(mut self, colors: Vec<Hsla>) -> Self {
        self.featured_colors = Some(colors);
        self
    }

    /// Set the icon to the color picker button.
    ///
    /// If this is set the color picker button will display the icon.
    /// Else it will display the square color of the current value.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the label to be displayed above the color picker.
    ///
    /// Default is `None`.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the name a screen reader announces, when the visible label is not
    /// it.
    ///
    /// A color picker's name comes from its [`label`](Self::label) by default.
    /// Setting this replaces the announced name without changing the visible
    /// label.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Set the anchor corner of the color picker.
    ///
    /// Default is `Anchor::TopLeft`.
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    fn render_item(&self, color: Hsla, cx: &mut App) -> ColorSwatch {
        let selected = self.state.read(cx).value() == Some(color);
        let hover_state = self.state.clone();
        let click_state = self.state.clone();

        ColorSwatch::new(
            SharedString::from(format!("color-{}", color.to_hex())),
            color,
        )
        .selected(selected)
        .h_5()
        .w_5()
        .bg(color)
        .border_1()
        .border_color(color.darken(0.1))
        .hover(|this| this.border_color(color.darken(0.3)).bg(color.lighten(0.1)))
        .active(|this| this.border_color(color.darken(0.5)).bg(color.darken(0.2)))
        .on_hover(move |color, entered, window, cx| {
            if entered {
                hover_state.update(cx, |state, cx| state.preview_color(color, window, cx));
            }
        })
        .on_click(move |color, _, window, cx| {
            click_state.update(cx, |state, cx| state.select_color(color, window, cx));
        })
    }

    fn render_colors(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state
            .update(cx, |state, cx| state.sync_pending_value(window, cx));

        let active_tab = self.state.read(cx).active_tab();
        let (slider_color, hovered_color) = {
            let state = self.state.read(cx);
            let slider_color = state
                .displayed_color()
                .unwrap_or_else(|| hsla(0., 0., 0., 1.));
            (slider_color, state.preview())
        };
        let tab_state = self.state.clone();

        v_flex()
            .p_0p5()
            .gap_3()
            .child(
                TabBar::new("mode")
                    .segmented()
                    .selected_index(active_tab)
                    .on_click(move |ix: &usize, _, cx| {
                        tab_state.update(cx, |state, cx| state.set_active_tab(*ix, cx));
                    })
                    .child(Tab::new().flex_1().label(t!("ColorPicker.Palette")))
                    .child(Tab::new().flex_1().label(t!("ColorPicker.HSLA"))),
            )
            .child(match active_tab {
                0 => self.render_palette_panel(cx).into_any_element(),
                _ => self
                    .render_slider_tab_panel(slider_color, cx)
                    .into_any_element(),
            })
            .when_some(hovered_color, |this, hovered_color| {
                this.child(Separator::horizontal()).child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .bg(hovered_color)
                                .flex_shrink_0()
                                .border_1()
                                .border_color(hovered_color.darken(0.2))
                                .size_5()
                                .rounded(cx.theme().radius),
                        )
                        .child(Input::new(self.state.read(cx).hex_input()).small().px_2p5()),
                )
            })
    }

    fn render_palette_panel(&self, cx: &mut App) -> impl IntoElement {
        let featured_colors = self.featured_colors.clone().unwrap_or(vec![
            cx.theme().red,
            cx.theme().red_light,
            cx.theme().blue,
            cx.theme().blue_light,
            cx.theme().green,
            cx.theme().green_light,
            cx.theme().yellow,
            cx.theme().yellow_light,
            cx.theme().cyan,
            cx.theme().cyan_light,
            cx.theme().magenta,
            cx.theme().magenta_light,
        ]);

        v_flex()
            .gap_3()
            .child(
                h_flex().gap_1().children(
                    featured_colors
                        .iter()
                        .map(|color| self.render_item(*color, cx)),
                ),
            )
            .child(Separator::horizontal())
            .child(
                v_flex()
                    .gap_1()
                    .children(color_palettes().iter().map(|sub_colors| {
                        h_flex().gap_1().children(
                            sub_colors
                                .iter()
                                .rev()
                                .map(|color| self.render_item(*color, cx)),
                        )
                    })),
            )
    }

    fn render_slider_tab_panel(&self, slider_color: Hsla, cx: &mut App) -> impl IntoElement {
        let sliders = self.state.read(cx).sliders().clone();
        let steps = 96usize;
        let hue_colors = (0..steps)
            .map(|ix| {
                let h = ix as f32 / (steps.saturating_sub(1)) as f32;
                hsla(h, 1.0, 0.5, 1.0)
            })
            .collect::<Vec<_>>();
        let saturation_start = hsla(slider_color.h, 0.0, slider_color.l, 1.0);
        let saturation_end = hsla(slider_color.h, 1.0, slider_color.l, 1.0);
        let lightness_colors = (0..steps)
            .map(|ix| {
                let l = ix as f32 / (steps.saturating_sub(1)) as f32;
                hsla(slider_color.h, 1.0, l, 1.0)
            })
            .collect::<Vec<_>>();
        let alpha_start = hsla(slider_color.h, slider_color.s, slider_color.l, 0.0);
        let alpha_end = hsla(slider_color.h, slider_color.s, slider_color.l, 1.0);

        let label_color = cx.theme().foreground.opacity(0.7);

        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .min_w_16()
                            .text_xs()
                            .text_color(label_color)
                            .child(t!("ColorPicker.Hue")),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .items_center()
                            .flex_1()
                            .h_8()
                            .child(self.render_slider_track(hue_colors, cx))
                            .child(
                                Slider::new(sliders.hue())
                                    .flex_1()
                                    .bg(cx.theme().transparent),
                            ),
                    )
                    .child(
                        div()
                            .w_10()
                            .text_xs()
                            .text_color(label_color)
                            .text_align(TextAlign::Right)
                            .child(format!("{:.0}", slider_color.h * 360.)),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .min_w_16()
                            .text_xs()
                            .text_color(label_color)
                            .child(t!("ColorPicker.Saturation")),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .items_center()
                            .flex_1()
                            .h_8()
                            .child(self.render_slider_track_gradient(
                                saturation_start,
                                saturation_end,
                                cx,
                            ))
                            .child(
                                Slider::new(sliders.saturation())
                                    .flex_1()
                                    .bg(cx.theme().transparent),
                            ),
                    )
                    .child(
                        div()
                            .w_10()
                            .text_xs()
                            .text_color(label_color)
                            .text_align(TextAlign::Right)
                            .child(format!("{:.0}", slider_color.s * 100.)),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .min_w_16()
                            .text_xs()
                            .text_color(label_color)
                            .child(t!("ColorPicker.Lightness")),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .items_center()
                            .flex_1()
                            .h_8()
                            .child(self.render_slider_track(lightness_colors, cx))
                            .child(
                                Slider::new(sliders.lightness())
                                    .flex_1()
                                    .bg(cx.theme().transparent),
                            ),
                    )
                    .child(
                        div()
                            .w_10()
                            .text_xs()
                            .text_color(label_color)
                            .text_align(TextAlign::Right)
                            .child(format!("{:.0}", slider_color.l * 100.)),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .min_w_16()
                            .text_xs()
                            .text_color(label_color)
                            .child(t!("ColorPicker.Alpha")),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .items_center()
                            .flex_1()
                            .h_8()
                            .child(self.render_slider_track_gradient(alpha_start, alpha_end, cx))
                            .child(
                                Slider::new(sliders.alpha())
                                    .flex_1()
                                    .bg(cx.theme().transparent),
                            ),
                    )
                    .child(
                        div()
                            .w_10()
                            .text_xs()
                            .text_color(label_color)
                            .text_align(TextAlign::Right)
                            .child(format!("{:.0}", slider_color.a * 100.)),
                    ),
            )
    }

    fn render_slider_track(&self, colors: Vec<Hsla>, _: &App) -> impl IntoElement {
        h_flex()
            .absolute()
            .left_0()
            .right_0()
            .h_2_5()
            .overflow_hidden()
            .children(
                colors
                    .into_iter()
                    .map(|color| div().flex_1().h_full().bg(color)),
            )
    }

    fn render_slider_track_gradient(&self, start: Hsla, end: Hsla, _: &App) -> impl IntoElement {
        div()
            .absolute()
            .left_0()
            .right_0()
            .h_2_5()
            .overflow_hidden()
            .bg(linear_gradient(
                90.,
                linear_color_stop(start, 0.),
                linear_color_stop(end, 1.),
            ))
    }
}

impl Sizable for ColorPicker {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Focusable for ColorPicker {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for ColorPicker {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ColorPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let display_title: SharedString = if let Some(value) = state.value() {
            value.to_hex()
        } else {
            "".to_string()
        }
        .into();

        let open = state.is_open();
        let value = state.value();
        let focus_handle = self.state.focus_handle(cx);
        let open_state = self.state.clone();
        let popover_state = self.state.clone();

        BaseColorPicker::new(self.id.clone())
            .open(open)
            .track_focus(&focus_handle)
            .when_some(
                self.accessibility_label
                    .clone()
                    .or_else(|| self.label.clone()),
                |this, label| this.accessibility_label(label),
            )
            .on_open_change(move |open, _, cx| {
                open_state.update(cx, |state, cx| state.set_open(open, cx));
            })
            .child(
                Popover::new("popover")
                    .open(open)
                    .w_72()
                    .on_open_change(move |open: &bool, _, cx| {
                        popover_state.update(cx, |state, cx| state.set_open(*open, cx));
                    })
                    .trigger(ColorPickerButton {
                        id: "trigger".into(),
                        size: self.size,
                        label: self.label.clone(),
                        value,
                        tooltip: if display_title.is_empty() {
                            None
                        } else {
                            Some(display_title.clone())
                        },
                        icon: self.icon.clone(),
                        selected: false,
                    })
                    .child(self.render_colors(window, cx)),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;

    #[gpui::test]
    fn an_explicit_accessibility_label_replaces_the_visible_one(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let state = cx.new(|cx| ColorPickerState::new(window, cx));

            let plain = ColorPicker::new(&state).label("Color");
            assert_eq!(plain.accessibility_label, None);
            assert_eq!(plain.label.as_deref(), Some("Color"));

            let named = ColorPicker::new(&state)
                .label("Color")
                .accessibility_label("Text color");
            assert_eq!(
                named.accessibility_label.as_deref(),
                Some("Text color"),
                "an explicit name must win over the visible label"
            );
            assert_eq!(
                named.label.as_deref(),
                Some("Color"),
                "and must not change what is drawn"
            );
        });
    }
}

#[derive(IntoElement)]
struct ColorPickerButton {
    id: ElementId,
    selected: bool,
    icon: Option<Icon>,
    value: Option<Hsla>,
    size: Size,
    label: Option<SharedString>,
    tooltip: Option<SharedString>,
}

impl Selectable for ColorPickerButton {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Sizable for ColorPickerButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for ColorPickerButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let has_icon = self.icon.is_some();
        h_flex()
            .id(self.id)
            .gap_2()
            .children(self.icon)
            .when(!has_icon, |this| {
                this.child(
                    div()
                        .id("square")
                        .bg(cx.theme().tokens.background)
                        .border_1()
                        .border_color(cx.theme().input)
                        .rounded(cx.theme().radius)
                        .overflow_hidden()
                        .size_with(self.size)
                        .when_some(self.value, |this, value| {
                            this.bg(value)
                                .border_color(value.darken(0.3))
                                .when(self.selected, |this| this.border_2())
                        })
                        .when_some(self.tooltip, |this, tooltip| {
                            this.managed_tooltip(move |window, cx| {
                                Tooltip::new(tooltip.clone()).build(window, cx)
                            })
                        }),
                )
            })
            .when_some(self.label, |this, label| this.child(label))
    }
}
