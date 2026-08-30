use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Entity, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _, svg,
};
use gpui_base::{Checkbox, CheckboxState, Radio, RadioGroup};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, IconName, Sizable, Size, StyledExt as _, ThemeStyled as _,
    button::{Button, ButtonVariants as _},
    icon::IconNamed as _,
    input::Input,
    kbd::Kbd,
};

use super::{QuestionnaireChoiceState, QuestionnaireState};

type ChoiceRenderer =
    Rc<dyn Fn(&QuestionnaireChoiceState, &mut Window, &mut App) -> AnyElement + 'static>;

#[derive(Clone, Copy)]
struct QuestionnaireMetrics {
    root_gap: gpui::Pixels,
    item_gap: gpui::Pixels,
    choice_gap: gpui::Pixels,
    content_gap: gpui::Pixels,
    choice_padding_x: gpui::Pixels,
    choice_padding_y: gpui::Pixels,
    choice_min_height: gpui::Pixels,
    choice_radius: gpui::Pixels,
    input_padding_y: gpui::Pixels,
    input_radius: gpui::Pixels,
    indicator_size: gpui::Pixels,
    indicator_mark_size: gpui::Pixels,
    indicator_check_size: gpui::Pixels,
    shortcut_size: gpui::Pixels,
    shortcut_text_size: gpui::Pixels,
    shortcut_radius: gpui::Pixels,
}

impl QuestionnaireMetrics {
    fn new(size: Size, cx: &App) -> Self {
        let tokens = cx.theme().semantic_tokens();
        let spacing = tokens.spacing;
        let radius = tokens.radius;
        match size {
            Size::XSmall => Self {
                root_gap: spacing.sm,
                item_gap: spacing.sm,
                choice_gap: spacing.xs,
                content_gap: spacing.xxs,
                choice_padding_x: spacing.sm,
                choice_padding_y: spacing.xs,
                choice_min_height: spacing.xl + spacing.xs,
                choice_radius: radius.md,
                input_padding_y: gpui::Pixels::ZERO,
                input_radius: radius.md,
                indicator_size: spacing.md,
                indicator_mark_size: spacing.xs,
                indicator_check_size: spacing.sm,
                shortcut_size: spacing.lg,
                shortcut_text_size: spacing.sm,
                shortcut_radius: radius.sm,
            },
            Size::Small => Self {
                root_gap: spacing.md,
                item_gap: spacing.md,
                choice_gap: spacing.xs,
                content_gap: spacing.xxs,
                choice_padding_x: spacing.sm,
                choice_padding_y: spacing.xs,
                choice_min_height: spacing.xxl,
                choice_radius: radius.lg,
                input_padding_y: spacing.xxs,
                input_radius: radius.lg,
                indicator_size: spacing.md + spacing.xxs,
                indicator_mark_size: spacing.xs + spacing.xxs,
                indicator_check_size: spacing.sm + spacing.xxs,
                shortcut_size: spacing.lg + spacing.xxs,
                shortcut_text_size: spacing.sm + spacing.xxs * 0.5,
                shortcut_radius: radius.md,
            },
            Size::Large => Self {
                root_gap: spacing.xl,
                item_gap: spacing.xl,
                choice_gap: spacing.md,
                content_gap: spacing.xs,
                choice_padding_x: spacing.lg,
                choice_padding_y: spacing.md,
                choice_min_height: spacing.xxl + spacing.lg,
                choice_radius: radius.xl,
                input_padding_y: spacing.sm,
                input_radius: radius.xl,
                indicator_size: spacing.lg + spacing.xxs,
                indicator_mark_size: spacing.sm + spacing.xxs,
                indicator_check_size: spacing.lg,
                shortcut_size: spacing.xl,
                shortcut_text_size: spacing.md,
                shortcut_radius: radius.xl,
            },
            Size::Size(value) => Self {
                root_gap: value,
                item_gap: value,
                choice_gap: value * 0.625,
                content_gap: value * 0.25,
                choice_padding_x: value * 0.75,
                choice_padding_y: value * 0.625,
                choice_min_height: value * 2.75,
                choice_radius: (radius.lg + radius.xl) * 0.5,
                input_padding_y: value * 0.25,
                input_radius: (radius.lg + radius.xl) * 0.5,
                indicator_size: value,
                indicator_mark_size: value * 0.5,
                indicator_check_size: value * 0.875,
                shortcut_size: value * 1.25,
                shortcut_text_size: value * 0.625,
                shortcut_radius: radius.lg,
            },
            Size::Medium => Self {
                root_gap: spacing.lg,
                item_gap: spacing.lg,
                choice_gap: spacing.sm + spacing.xxs,
                content_gap: spacing.xxs,
                choice_padding_x: spacing.md,
                choice_padding_y: spacing.sm + spacing.xxs,
                choice_min_height: spacing.xxl + spacing.md,
                choice_radius: (radius.lg + radius.xl) * 0.5,
                input_padding_y: spacing.xs,
                input_radius: (radius.lg + radius.xl) * 0.5,
                indicator_size: spacing.lg,
                indicator_mark_size: spacing.sm,
                indicator_check_size: spacing.md + spacing.xxs,
                shortcut_size: spacing.lg + spacing.xs,
                shortcut_text_size: spacing.sm + spacing.xxs,
                shortcut_radius: radius.lg,
            },
        }
    }
}

fn text_style<T: Styled>(element: T, size: Size, cx: &App) -> T {
    let typography = cx.theme().semantic_tokens().typography;
    let token = match size {
        Size::XSmall => typography.xs,
        Size::Small => typography.sm,
        Size::Medium => typography.sm,
        Size::Large => typography.md,
        Size::Size(value) => return element.text_size(value),
    };
    element
        .text_size(token.size)
        .line_height(token.line_height)
        .font_weight(token.weight)
}

fn progress_text_style<T: Styled>(element: T, size: Size, cx: &App) -> T {
    let typography = cx.theme().semantic_tokens().typography;
    let token = match size {
        Size::XSmall | Size::Small | Size::Medium => typography.xs,
        Size::Large => typography.sm,
        Size::Size(value) => {
            return element.text_size(value * 0.75).line_height(value);
        }
    };

    element
        .text_size(token.size)
        .line_height(token.line_height)
        .font_weight(token.weight)
}

fn description_text_style<T: Styled>(element: T, size: Size, cx: &App) -> T {
    let metrics = QuestionnaireMetrics::new(size, cx);

    // A native fieldset excludes its legend from the flex gap before the
    // description. Recreate that base-nova relationship for GPUI's group.
    text_style(element, size, cx).mt(-metrics.item_gap)
}

fn title_text_style<T: Styled>(element: T, size: Size, cx: &App) -> T {
    let typography = cx.theme().semantic_tokens().typography;
    let token = match size {
        Size::XSmall => typography.sm,
        Size::Small => typography.sm,
        Size::Medium => typography.md,
        Size::Large => typography.lg,
        Size::Size(value) => return element.text_size(value),
    };
    element
        .text_size(token.size)
        .line_height(token.line_height)
        .font_weight(gpui::FontWeight::MEDIUM)
}

fn item_label(definition: &super::QuestionnaireItemDefinition) -> Option<SharedString> {
    Some(definition.accessibility_label().clone())
}

fn item_description(definition: &super::QuestionnaireItemDefinition) -> Option<SharedString> {
    definition.description().cloned()
}

fn element_id(state: &Entity<QuestionnaireState>, suffix: impl std::fmt::Display) -> ElementId {
    ElementId::Name(format!("questionnaire-{}-{suffix}", state.entity_id()).into())
}

/// The composable questionnaire root. It owns layout and keyboard routing while
/// [`QuestionnaireState`] remains the single source of behavioral state.
#[derive(IntoElement)]
pub struct Questionnaire {
    state: Entity<QuestionnaireState>,
    style: StyleRefinement,
    size: Size,
    children: Vec<AnyElement>,
}

impl Questionnaire {
    pub fn new(state: &Entity<QuestionnaireState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            size: Size::Medium,
            children: Vec::new(),
        }
    }

    fn on_key_down(
        state: &Entity<QuestionnaireState>,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        if window.default_prevented()
            || event.is_held
            || event.prefer_character_input
            || event.keystroke.is_ime_in_progress()
        {
            return;
        }

        let modifiers = event.keystroke.modifiers;
        let key = event.keystroke.key.as_str();
        let input_focused = state.read(cx).is_current_input_focused(window);
        let input_has_text = input_focused && state.read(cx).current_input_has_text(cx);
        let single_radio_focused = {
            let state = state.read(cx);
            state
                .current_item()
                .and_then(|item| state.item_state(item))
                .is_some_and(|item| !item.is_multiple())
                && state.focused_current_choice(window).is_some()
        };

        let handled = if key == "enter"
            && modifiers.secondary()
            && modifiers.number_of_modifiers() == 1
        {
            state.update(cx, |state, cx| state.confirm_current(window, cx))
        } else if modifiers.number_of_modifiers() != 0 {
            false
        } else if input_focused {
            match key {
                "enter" if Self::focused_answer_is_filled(state, window, cx) => {
                    state.update(cx, |state, cx| state.confirm_current(window, cx))
                }
                "up" if !input_has_text => {
                    state.update(cx, |state, cx| state.focus_previous_answer(window, cx))
                }
                "down" if !input_has_text => {
                    state.update(cx, |state, cx| state.focus_next_answer(window, cx))
                }
                _ => false,
            }
        } else {
            match key {
                "up" => {
                    state.update(cx, |state, cx| state.focus_previous_answer(window, cx))
                        || (single_radio_focused
                            && state
                                .update(cx, |state, cx| state.move_current_radio(-1, window, cx)))
                }
                "down" => {
                    state.update(cx, |state, cx| state.focus_next_answer(window, cx))
                        || (single_radio_focused
                            && state
                                .update(cx, |state, cx| state.move_current_radio(1, window, cx)))
                }
                "left" if single_radio_focused => {
                    state.update(cx, |state, cx| state.move_current_radio(-1, window, cx))
                }
                "right" if single_radio_focused => {
                    state.update(cx, |state, cx| state.move_current_radio(1, window, cx))
                }
                "left" => state.update(cx, |state, cx| state.go_previous(window, cx)),
                "right" if state.read(cx).navigation_state().is_confirmable() => {
                    state.update(cx, |state, cx| state.go_next(window, cx))
                }
                "right" => false,
                "enter" if Self::focused_answer_is_filled(state, window, cx) => {
                    state.update(cx, |state, cx| state.confirm_current(window, cx))
                }
                "enter" => false,
                _ => state.update(cx, |state, cx| state.activate_shortcut(key, window, cx)),
            }
        };

        if handled {
            window.prevent_default();
        }
    }

    fn focused_answer_is_filled(
        state: &Entity<QuestionnaireState>,
        window: &Window,
        cx: &App,
    ) -> bool {
        let state = state.read(cx);
        let Some(item) = state.current_item() else {
            return false;
        };
        if state.is_current_input_focused(window) {
            return state
                .answer(item)
                .is_some_and(|answer| answer.freeform().is_some());
        }
        state
            .focused_current_choice(window)
            .and_then(|value| state.choice_state(item, value))
            .is_some_and(|choice| choice.is_selected())
    }
}

impl Styled for Questionnaire {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Questionnaire {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ParentElement for Questionnaire {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Questionnaire {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = QuestionnaireMetrics::new(self.size, cx);
        let focus_handle = self.state.read(cx).focus_handle().clone();
        let state = self.state.clone();
        let debug_selector = format!("questionnaire-{}-root", self.state.entity_id());

        div()
            .id(element_id(&self.state, "root"))
            .debug_selector(move || debug_selector)
            .role(Role::Form)
            .key_context("Questionnaire")
            .track_focus(&focus_handle)
            .capture_key_down(move |event, window, cx| Self::on_key_down(&state, event, window, cx))
            .flex()
            .flex_col()
            .min_w_0()
            .gap(metrics.root_gap)
            .w_full()
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// Textual progress matching shadcn/ui's base-nova default presentation.
#[derive(IntoElement)]
pub struct QuestionnaireProgress {
    state: Entity<QuestionnaireState>,
    style: StyleRefinement,
    size: Size,
    children: Vec<AnyElement>,
}

impl QuestionnaireProgress {
    pub fn new(state: &Entity<QuestionnaireState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            size: Size::Medium,
            children: Vec::new(),
        }
    }
}

impl Styled for QuestionnaireProgress {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for QuestionnaireProgress {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ParentElement for QuestionnaireProgress {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for QuestionnaireProgress {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let progress = self.state.read(cx).progress();
        let current = progress.current();
        let total = progress.total();
        let label: SharedString =
            t!("Questionnaire.progress", current = current, total = total).into();
        let colors = cx.theme().semantic_tokens().colors;
        let has_children = !self.children.is_empty();

        progress_text_style(
            div()
                .id(element_id(&self.state, "progress"))
                .role(Role::ProgressIndicator)
                .aria_label(label.clone())
                .aria_min_numeric_value(0.)
                .aria_max_numeric_value(total as f64)
                .aria_numeric_value(current as f64)
                .text_color(colors.muted_foreground),
            self.size,
            cx,
        )
        .font_weight(gpui::FontWeight::MEDIUM)
        .refine_style(&self.style)
        .when(!has_children, |this| this.child(label))
        .children(self.children)
    }
}

macro_rules! questionnaire_item_part {
    ($name:ident, $fallback:ident, $style:ident, $color:ident) => {
        #[derive(IntoElement)]
        pub struct $name {
            state: Entity<QuestionnaireState>,
            item: SharedString,
            style: StyleRefinement,
            size: Size,
            children: Vec<AnyElement>,
        }

        impl $name {
            pub fn new(state: &Entity<QuestionnaireState>, item: impl Into<SharedString>) -> Self {
                Self {
                    state: state.clone(),
                    item: item.into(),
                    style: StyleRefinement::default(),
                    size: Size::Medium,
                    children: Vec::new(),
                }
            }
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.style
            }
        }

        impl Sizable for $name {
            fn with_size(mut self, size: impl Into<Size>) -> Self {
                self.size = size.into();
                self
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.children.extend(elements);
            }
        }

        impl RenderOnce for $name {
            fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
                let Some(definition) = self.state.read(cx).item_definition(&self.item) else {
                    return gpui::Empty.into_any_element();
                };
                let fallback = $fallback(definition);
                let has_children = !self.children.is_empty();
                if !has_children && fallback.is_none() {
                    return gpui::Empty.into_any_element();
                }
                let colors = cx.theme().semantic_tokens().colors;
                $style(div().w_full().text_color(colors.$color), self.size, cx)
                    .refine_style(&self.style)
                    .when(!has_children, |this| {
                        this.when_some(fallback, |this, fallback| this.child(fallback))
                    })
                    .children(self.children)
                    .into_any_element()
            }
        }
    };
}

questionnaire_item_part!(QuestionnaireTitle, item_label, title_text_style, foreground);
questionnaire_item_part!(
    QuestionnaireDescription,
    item_description,
    description_text_style,
    muted_foreground
);

/// The active question group. Inactive or disabled items do not enter layout,
/// focus traversal, or the accessibility tree.
#[derive(IntoElement)]
pub struct QuestionnaireItem {
    state: Entity<QuestionnaireState>,
    item: SharedString,
    style: StyleRefinement,
    size: Size,
    children: Vec<AnyElement>,
}

impl QuestionnaireItem {
    pub fn new(state: &Entity<QuestionnaireState>, item: impl Into<SharedString>) -> Self {
        Self {
            state: state.clone(),
            item: item.into(),
            style: StyleRefinement::default(),
            size: Size::Medium,
            children: Vec::new(),
        }
    }
}

impl Styled for QuestionnaireItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for QuestionnaireItem {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ParentElement for QuestionnaireItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for QuestionnaireItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let active = state.current_item().is_some_and(|name| name == &self.item);
        let Some(item_state) = state.item_state(&self.item) else {
            return gpui::Empty.into_any_element();
        };
        if !active || item_state.is_disabled() {
            return gpui::Empty.into_any_element();
        }
        let Some(definition) = state.item_definition(&self.item) else {
            return gpui::Empty.into_any_element();
        };
        let focus_handle = state.item_focus_handle(&self.item).cloned();
        let label = definition.accessibility_label().clone();
        let description = definition.description().cloned();
        let metrics = QuestionnaireMetrics::new(self.size, cx);

        div()
            .id(element_id(&self.state, format!("item-{}", self.item)))
            .role(Role::Group)
            .aria_label(label)
            .when_some(description, |this, description| {
                this.aria_description(description)
            })
            .when_some(focus_handle, |this, focus_handle| {
                this.track_focus(&focus_handle.tab_index(-1).tab_stop(false))
            })
            .flex()
            .flex_col()
            .gap(metrics.item_gap)
            .w_full()
            .refine_style(&self.style)
            .children(self.children)
            .into_any_element()
    }
}

/// Container for an item's answer controls.
#[derive(IntoElement)]
pub struct QuestionnaireChoices {
    state: Entity<QuestionnaireState>,
    item: SharedString,
    style: StyleRefinement,
    size: Size,
    children: Vec<AnyElement>,
}

impl QuestionnaireChoices {
    pub fn new(state: &Entity<QuestionnaireState>, item: impl Into<SharedString>) -> Self {
        Self {
            state: state.clone(),
            item: item.into(),
            style: StyleRefinement::default(),
            size: Size::Medium,
            children: Vec::new(),
        }
    }
}

impl Styled for QuestionnaireChoices {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for QuestionnaireChoices {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ParentElement for QuestionnaireChoices {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for QuestionnaireChoices {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let active = state.current_item().is_some_and(|name| name == &self.item);
        let Some(item) = state.item_state(&self.item) else {
            return gpui::Empty.into_any_element();
        };
        if !active || item.is_disabled() {
            return gpui::Empty.into_any_element();
        }
        let metrics = QuestionnaireMetrics::new(self.size, cx);

        if item.is_multiple() {
            div()
                .id(element_id(&self.state, format!("choices-{}", self.item)))
                .role(Role::Group)
                .flex()
                .flex_col()
                .gap(metrics.choice_gap)
                .w_full()
                .refine_style(&self.style)
                .children(self.children)
                .into_any_element()
        } else {
            RadioGroup::new(element_id(&self.state, format!("choices-{}", self.item)))
                .flex()
                .flex_col()
                .gap(metrics.choice_gap)
                .w_full()
                .refine_style(&self.style)
                .children(self.children)
                .into_any_element()
        }
    }
}

/// A selectable base-nova choice card.
#[derive(IntoElement)]
pub struct QuestionnaireChoice {
    state: Entity<QuestionnaireState>,
    item: SharedString,
    value: SharedString,
    style: StyleRefinement,
    indicator_style: StyleRefinement,
    content_style: StyleRefinement,
    shortcut_style: StyleRefinement,
    size: Size,
    children: Vec<AnyElement>,
    indicator_renderer: Option<ChoiceRenderer>,
    shortcut_renderer: Option<ChoiceRenderer>,
}

impl QuestionnaireChoice {
    pub fn new(
        state: &Entity<QuestionnaireState>,
        item: impl Into<SharedString>,
        value: impl Into<SharedString>,
    ) -> Self {
        Self {
            state: state.clone(),
            item: item.into(),
            value: value.into(),
            style: StyleRefinement::default(),
            indicator_style: StyleRefinement::default(),
            content_style: StyleRefinement::default(),
            shortcut_style: StyleRefinement::default(),
            size: Size::Medium,
            children: Vec::new(),
            indicator_renderer: None,
            shortcut_renderer: None,
        }
    }

    pub fn indicator_style(mut self, style: StyleRefinement) -> Self {
        self.indicator_style = style;
        self
    }

    pub fn content_style(mut self, style: StyleRefinement) -> Self {
        self.content_style = style;
        self
    }

    pub fn shortcut_style(mut self, style: StyleRefinement) -> Self {
        self.shortcut_style = style;
        self
    }

    pub fn render_indicator(
        mut self,
        renderer: impl Fn(&QuestionnaireChoiceState, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.indicator_renderer = Some(Rc::new(renderer));
        self
    }

    pub fn render_shortcut(
        mut self,
        renderer: impl Fn(&QuestionnaireChoiceState, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.shortcut_renderer = Some(Rc::new(renderer));
        self
    }
}

impl Styled for QuestionnaireChoice {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for QuestionnaireChoice {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ParentElement for QuestionnaireChoice {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

#[allow(clippy::too_many_arguments)]
fn style_choice_card<T>(
    base: T,
    indicator: AnyElement,
    content: AnyElement,
    shortcut: AnyElement,
    metrics: QuestionnaireMetrics,
    selected: bool,
    disabled: bool,
    invalid: bool,
    focused: bool,
    instance_style: &StyleRefinement,
    window: &Window,
    cx: &App,
) -> T
where
    T: Styled + ParentElement + StatefulInteractiveElement + gpui::prelude::FluentBuilder,
{
    let tokens = cx.theme().semantic_tokens();
    base.flex()
        .items_start()
        .gap(metrics.choice_gap)
        .w_full()
        .min_h(metrics.choice_min_height)
        .px(metrics.choice_padding_x)
        .py(metrics.choice_padding_y)
        .border_1()
        .border_color(if invalid {
            tokens.colors.destructive
        } else if selected {
            tokens.colors.primary.opacity(0.4)
        } else {
            tokens.colors.input
        })
        .bg(if selected {
            tokens.colors.muted
        } else {
            tokens.colors.background.opacity(0.)
        })
        .rounded(metrics.choice_radius)
        .when(!disabled, |this| {
            this.hover(|style| style.bg(tokens.colors.muted.opacity(0.5)))
        })
        .when(focused, |this| this.focus_ring_style(window, cx))
        .when(disabled, |this| this.opacity(0.5))
        .refine_style(instance_style)
        .child(indicator)
        .child(content)
        .child(shortcut)
}

impl RenderOnce for QuestionnaireChoice {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let active = state.current_item().is_some_and(|name| name == &self.item);
        let Some(choice_state) = state.choice_state(&self.item, &self.value) else {
            return gpui::Empty.into_any_element();
        };
        if !active {
            return gpui::Empty.into_any_element();
        }
        let Some(item) = state.item_state(&self.item) else {
            return gpui::Empty.into_any_element();
        };
        let Some(definition) = state.choice_definition(&self.item, &self.value) else {
            return gpui::Empty.into_any_element();
        };
        let multiple = item.is_multiple();
        let label = definition.accessibility_label().clone();
        let description = definition.description().cloned();
        let position = state.item_definition(&self.item).and_then(|item| {
            let enabled: Vec<_> = item
                .choices()
                .iter()
                .filter(|choice| {
                    state
                        .choice_state(&self.item, choice.value())
                        .is_some_and(|choice| !choice.is_disabled())
                })
                .collect();
            enabled
                .iter()
                .position(|choice| choice.value() == &self.value)
                .map(|position| (position + 1, enabled.len()))
        });
        let selected = choice_state.is_selected();
        let disabled = choice_state.is_disabled();
        let invalid = choice_state.is_invalid();
        let shortcut = choice_state.shortcut().cloned();
        let focus_handle = state.choice_focus_handle(&self.item, &self.value).cloned();
        let colors = cx.theme().semantic_tokens().colors;
        let radius = cx.theme().semantic_tokens().radius;
        let mono_font = cx.theme().semantic_tokens().typography.mono.clone();
        let metrics = QuestionnaireMetrics::new(self.size, cx);
        let answer_alignment_offset = metrics.content_gap;
        let focused = focus_handle
            .as_ref()
            .is_some_and(|focus_handle| focus_handle.is_focused(window));
        let has_children = !self.children.is_empty();

        let default_indicator = || {
            div()
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .size(metrics.indicator_size)
                .border_1()
                .border_color(if selected {
                    colors.primary
                } else {
                    colors.input
                })
                .bg(if selected {
                    colors.primary
                } else {
                    colors.background
                })
                .when(multiple, |this| this.rounded(radius.sm))
                .when(!multiple, |this| this.rounded(radius.full))
                .mt(answer_alignment_offset)
                .refine_style(&self.indicator_style)
                .when(selected && multiple, |this| {
                    this.child(
                        svg()
                            .size(metrics.indicator_check_size)
                            .path(IconName::Check.path())
                            .text_color(colors.primary_foreground),
                    )
                })
                .when(selected && !multiple, |this| {
                    this.child(
                        div()
                            .size(metrics.indicator_mark_size)
                            .rounded(radius.full)
                            .bg(colors.primary_foreground),
                    )
                })
                .into_any_element()
        };

        let indicator = self
            .indicator_renderer
            .as_ref()
            .map(|renderer| renderer(&choice_state, window, cx))
            .unwrap_or_else(default_indicator);

        let content = div()
            .flex()
            .flex_1()
            .flex_col()
            .gap(metrics.content_gap)
            .refine_style(&self.content_style)
            .when(!has_children, |this| {
                this.child(text_style(
                    div().text_color(colors.foreground).child(label.clone()),
                    self.size,
                    cx,
                ))
                .when_some(description.clone(), |this, description| {
                    this.child(text_style(
                        div().text_color(colors.muted_foreground).child(description),
                        self.size.smaller(),
                        cx,
                    ))
                })
            })
            .children(self.children);

        let default_shortcut = || {
            let Some(shortcut) = shortcut.clone() else {
                return gpui::Empty.into_any_element();
            };
            let Ok(keystroke) = gpui::Keystroke::parse(&shortcut.to_lowercase()) else {
                return gpui::Empty.into_any_element();
            };
            Kbd::new(keystroke)
                .outline()
                .flex()
                .items_center()
                .justify_center()
                .size(metrics.shortcut_size)
                .p_0()
                .bg(colors.background)
                .border_color(colors.input)
                .text_color(colors.muted_foreground)
                .font_family(mono_font.clone())
                .text_size(metrics.shortcut_text_size)
                .font_weight(gpui::FontWeight::MEDIUM)
                .rounded(metrics.shortcut_radius)
                .mt(answer_alignment_offset)
                .refine_style(&self.shortcut_style)
                .into_any_element()
        };
        let shortcut_element = self
            .shortcut_renderer
            .as_ref()
            .map(|renderer| renderer(&choice_state, window, cx))
            .unwrap_or_else(default_shortcut);

        let id = element_id(&self.state, format!("choice-{}-{}", self.item, self.value));
        let instance_style = self.style.clone();
        let state = self.state.clone();
        let item_name = self.item.clone();
        let choice_value = self.value.clone();

        if multiple {
            let callback_state = state.clone();
            let callback_item = item_name.clone();
            let callback_value = choice_value.clone();
            let confirm_state = state.clone();
            let base = Checkbox::new(id)
                .state(if selected {
                    CheckboxState::Checked
                } else {
                    CheckboxState::Unchecked
                })
                .disabled(disabled)
                .accessibility_label(label)
                .when_some(description.clone(), |this, description| {
                    this.aria_description(description)
                })
                .when_some(position, |this, (position, total)| {
                    this.aria_position_in_set(position).aria_size_of_set(total)
                })
                .when_some(focus_handle, |this, focus_handle| {
                    this.track_focus(&focus_handle)
                })
                .capture_key_down(move |event, window, cx| {
                    if selected
                        && !window.default_prevented()
                        && !event.is_held
                        && event.keystroke.key == "enter"
                        && event.keystroke.modifiers.number_of_modifiers() == 0
                        && confirm_state.update(cx, |state, cx| state.confirm_current(window, cx))
                    {
                        window.prevent_default();
                    }
                })
                .on_change(move |_, _, window, cx| {
                    let _ = callback_state.update(cx, |state, cx| {
                        let result = state.activate_choice(&callback_item, &callback_value, cx);
                        state.focus_choice(&callback_item, &callback_value, window, cx);
                        result
                    });
                });
            style_choice_card(
                base,
                indicator,
                content.into_any_element(),
                shortcut_element,
                metrics,
                selected,
                disabled,
                invalid,
                focused,
                &instance_style,
                window,
                cx,
            )
            .into_any_element()
        } else {
            let confirm_state = state.clone();
            let base = Radio::new(id)
                .checked(selected)
                .disabled(disabled)
                .accessibility_label(label)
                .when_some(description, |this, description| {
                    this.aria_description(description)
                })
                .when_some(position, |this, (position, total)| {
                    this.set_position(position, total)
                })
                .when_some(focus_handle, |this, focus_handle| {
                    this.track_focus(&focus_handle)
                })
                .capture_key_down(move |event, window, cx| {
                    if selected
                        && !window.default_prevented()
                        && !event.is_held
                        && event.keystroke.key == "enter"
                        && event.keystroke.modifiers.number_of_modifiers() == 0
                        && confirm_state.update(cx, |state, cx| state.confirm_current(window, cx))
                    {
                        window.prevent_default();
                    }
                })
                .on_change(move |_, _, window, cx| {
                    let _ = state.update(cx, |state, cx| {
                        let result = state.activate_choice(&item_name, &choice_value, cx);
                        state.focus_choice(&item_name, &choice_value, window, cx);
                        result
                    });
                });
            style_choice_card(
                base,
                indicator,
                content.into_any_element(),
                shortcut_element,
                metrics,
                selected,
                disabled,
                invalid,
                focused,
                &instance_style,
                window,
                cx,
            )
            .into_any_element()
        }
    }
}

/// Secondary text for custom choice compositions.
#[derive(IntoElement)]
pub struct QuestionnaireChoiceDescription {
    style: StyleRefinement,
    size: Size,
    children: Vec<AnyElement>,
}

impl QuestionnaireChoiceDescription {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            size: Size::Medium,
            children: Vec::new(),
        }
    }
}

impl Default for QuestionnaireChoiceDescription {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for QuestionnaireChoiceDescription {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for QuestionnaireChoiceDescription {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ParentElement for QuestionnaireChoiceDescription {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for QuestionnaireChoiceDescription {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().semantic_tokens().colors;
        text_style(
            div().text_color(colors.muted_foreground),
            self.size.smaller(),
            cx,
        )
        .refine_style(&self.style)
        .children(self.children)
    }
}

/// The optional freeform answer input for an item.
#[derive(IntoElement)]
pub struct QuestionnaireInput {
    state: Entity<QuestionnaireState>,
    item: SharedString,
    style: StyleRefinement,
    size: Size,
}

impl QuestionnaireInput {
    pub fn new(state: &Entity<QuestionnaireState>, item: impl Into<SharedString>) -> Self {
        Self {
            state: state.clone(),
            item: item.into(),
            style: StyleRefinement::default(),
            size: Size::Medium,
        }
    }
}

impl Styled for QuestionnaireInput {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for QuestionnaireInput {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for QuestionnaireInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let active = state.current_item().is_some_and(|name| name == &self.item);
        let Some(item_state) = state.item_state(&self.item) else {
            return gpui::Empty.into_any_element();
        };
        let Some(definition) = state.item_definition(&self.item) else {
            return gpui::Empty.into_any_element();
        };
        let Some(input_definition) = definition.input() else {
            return gpui::Empty.into_any_element();
        };
        if !active {
            return gpui::Empty.into_any_element();
        }
        let metrics = QuestionnaireMetrics::new(self.size, cx);

        Input::new(input_definition.state())
            .aria_label(input_definition.accessibility_label().clone())
            .disabled(item_state.is_disabled() || input_definition.is_disabled())
            .with_size(self.size)
            .py(metrics.input_padding_y)
            .rounded(metrics.input_radius)
            .when(item_state.is_invalid(), |this| {
                this.border_color(cx.theme().semantic_tokens().colors.destructive)
            })
            .refine_style(&self.style)
            .into_any_element()
    }
}

/// Validation error for an item. It only enters the tree while invalid.
#[derive(IntoElement)]
pub struct QuestionnaireError {
    state: Entity<QuestionnaireState>,
    item: SharedString,
    style: StyleRefinement,
    size: Size,
    children: Vec<AnyElement>,
}

impl QuestionnaireError {
    pub fn new(state: &Entity<QuestionnaireState>, item: impl Into<SharedString>) -> Self {
        Self {
            state: state.clone(),
            item: item.into(),
            style: StyleRefinement::default(),
            size: Size::Medium,
            children: Vec::new(),
        }
    }
}

impl Styled for QuestionnaireError {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for QuestionnaireError {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ParentElement for QuestionnaireError {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

fn questionnaire_error_root(id: ElementId) -> gpui::Stateful<gpui::Div> {
    div().id(id).role(Role::Alert)
}

impl RenderOnce for QuestionnaireError {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let Some(item) = state.item_state(&self.item) else {
            return gpui::Empty.into_any_element();
        };
        let error = state.error(&self.item).cloned();
        if !item.is_invalid() || error.is_none() {
            return gpui::Empty.into_any_element();
        }
        let has_children = !self.children.is_empty();
        let colors = cx.theme().semantic_tokens().colors;
        let spacing = cx.theme().semantic_tokens().spacing;

        text_style(
            questionnaire_error_root(element_id(&self.state, format!("error-{}", self.item)))
                .mt(spacing.sm)
                .text_color(colors.destructive),
            self.size,
            cx,
        )
        .refine_style(&self.style)
        .when(!has_children, |this| {
            this.when_some(error, |this, error| this.child(error))
        })
        .children(self.children)
        .into_any_element()
    }
}

/// Layout part for questionnaire navigation actions.
#[derive(IntoElement)]
pub struct QuestionnaireActions {
    state: Entity<QuestionnaireState>,
    style: StyleRefinement,
    size: Size,
    children: Vec<AnyElement>,
}

impl QuestionnaireActions {
    pub fn new(state: &Entity<QuestionnaireState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            size: Size::Medium,
            children: Vec::new(),
        }
    }
}

impl Styled for QuestionnaireActions {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for QuestionnaireActions {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ParentElement for QuestionnaireActions {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for QuestionnaireActions {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = QuestionnaireMetrics::new(self.size, cx);
        let debug_selector = format!("questionnaire-{}-actions", self.state.entity_id());
        div()
            .id(element_id(&self.state, "actions"))
            .debug_selector(move || debug_selector)
            .flex()
            .min_w_0()
            .items_center()
            .justify_start()
            .gap(metrics.choice_gap)
            .w_full()
            .refine_style(&self.style)
            .children(self.children)
    }
}

#[derive(Clone, Copy)]
enum QuestionnaireAction {
    Previous,
    Skip,
    Next,
    Submit,
}

macro_rules! questionnaire_action_part {
    ($name:ident, $action:ident, $translation:literal, $outline:expr, $primary:expr) => {
        #[derive(IntoElement)]
        pub struct $name {
            state: Entity<QuestionnaireState>,
            style: StyleRefinement,
            size: Size,
            children: Vec<AnyElement>,
        }

        impl $name {
            pub fn new(state: &Entity<QuestionnaireState>) -> Self {
                Self {
                    state: state.clone(),
                    style: StyleRefinement::default(),
                    size: Size::Medium,
                    children: Vec::new(),
                }
            }
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.style
            }
        }

        impl Sizable for $name {
            fn with_size(mut self, size: impl Into<Size>) -> Self {
                self.size = size.into();
                self
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.children.extend(elements);
            }
        }

        impl RenderOnce for $name {
            fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
                let navigation = self.state.read(cx).navigation_state();
                let action = QuestionnaireAction::$action;
                let visible = match action {
                    QuestionnaireAction::Previous => navigation.is_previous_visible(),
                    QuestionnaireAction::Skip => navigation.is_skip_visible(),
                    QuestionnaireAction::Next => navigation.is_next_visible(),
                    QuestionnaireAction::Submit => navigation.is_submit_visible(),
                };
                if !visible {
                    return gpui::Empty.into_any_element();
                }

                let anchors_trailing_actions = match action {
                    QuestionnaireAction::Skip => true,
                    QuestionnaireAction::Next | QuestionnaireAction::Submit => {
                        !navigation.is_skip_visible()
                    }
                    QuestionnaireAction::Previous => false,
                };
                let state = self.state.clone();
                let has_children = !self.children.is_empty();
                let debug_selector = format!(
                    "questionnaire-{}-{}",
                    self.state.entity_id(),
                    stringify!($action)
                );
                Button::new(element_id(&self.state, stringify!($action)))
                    .debug_selector(move || debug_selector)
                    .with_size(self.size)
                    .when($outline, |this| this.outline())
                    .when($primary, |this| this.primary())
                    .when(anchors_trailing_actions, |this| this.ml_auto())
                    .on_click(move |_, window, cx| {
                        state.update(cx, |state, cx| match action {
                            QuestionnaireAction::Previous => state.go_previous(window, cx),
                            QuestionnaireAction::Skip => state.skip_current(window, cx),
                            QuestionnaireAction::Next => state.go_next(window, cx),
                            QuestionnaireAction::Submit => state.submit(window, cx),
                        });
                    })
                    .refine_style(&self.style)
                    .when(!has_children, |this| this.label(t!($translation)))
                    .children(self.children)
                    .into_any_element()
            }
        }
    };
}

questionnaire_action_part!(
    QuestionnairePrevious,
    Previous,
    "Questionnaire.previous",
    true,
    false
);
questionnaire_action_part!(QuestionnaireSkip, Skip, "Questionnaire.skip", true, false);
questionnaire_action_part!(QuestionnaireNext, Next, "Questionnaire.next", false, true);
questionnaire_action_part!(
    QuestionnaireSubmit,
    Submit,
    "Questionnaire.submit",
    false,
    true
);

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        AppContext as _, Context, Element as _, Focusable as _, Keystroke, Render, TestAppContext,
        VisualTestContext, accesskit, px,
    };

    use super::super::{
        QuestionnaireChoiceDefinition, QuestionnaireInputDefinition, QuestionnaireItemDefinition,
        QuestionnaireShortcutMode,
    };

    #[test]
    fn compound_parts_support_builder_customization() {
        let _ = QuestionnaireChoiceDescription::new()
            .small()
            .opacity(0.8)
            .child("Description");
    }

    struct QuestionnaireHarness {
        state: Entity<QuestionnaireState>,
        override_skip_margin: bool,
    }

    impl Render for QuestionnaireHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Questionnaire::new(&self.state)
                .size(px(480.))
                .child(
                    QuestionnaireItem::new(&self.state, "choice")
                        .child(
                            QuestionnaireChoices::new(&self.state, "choice")
                                .child(QuestionnaireChoice::new(&self.state, "choice", "alpha")),
                        )
                        .child(QuestionnaireInput::new(&self.state, "choice")),
                )
                .child(
                    QuestionnaireItem::new(&self.state, "first")
                        .child(
                            QuestionnaireChoices::new(&self.state, "first")
                                .child(QuestionnaireChoice::new(&self.state, "first", "alpha"))
                                .child(QuestionnaireChoice::new(&self.state, "first", "beta")),
                        )
                        .child(QuestionnaireInput::new(&self.state, "first")),
                )
                .child(
                    QuestionnaireItem::new(&self.state, "second")
                        .child(QuestionnaireInput::new(&self.state, "second")),
                )
                .child(
                    QuestionnaireActions::new(&self.state)
                        .child(QuestionnairePrevious::new(&self.state))
                        .child(
                            QuestionnaireSkip::new(&self.state)
                                .when(self.override_skip_margin, |this| this.ml(px(0.))),
                        )
                        .child(QuestionnaireNext::new(&self.state))
                        .child(QuestionnaireSubmit::new(&self.state)),
                )
        }
    }

    fn visual_harness(
        cx: &mut TestAppContext,
        items: Vec<QuestionnaireItemDefinition>,
        shortcuts: Option<QuestionnaireShortcutMode>,
    ) -> (&mut VisualTestContext, Entity<QuestionnaireState>) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(move |_, cx| {
            let state = cx.new(|cx| {
                let state = QuestionnaireState::new(items, cx).unwrap();
                match shortcuts {
                    Some(shortcuts) => state.with_shortcuts(shortcuts),
                    None => state,
                }
            });
            QuestionnaireHarness {
                state,
                override_skip_margin: false,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let state = cx.update(|_, cx| view.read(cx).state.clone());
        cx.update(|window, cx| {
            let focus_handle = state.read(cx).focus_handle().clone();
            focus_handle.focus(window, cx);
        });
        (cx, state)
    }

    fn input_visual_harness(
        cx: &mut TestAppContext,
        multiple: bool,
    ) -> (&mut VisualTestContext, Entity<QuestionnaireState>) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(move |window, cx| {
            let input = cx.new(|cx| crate::input::InputState::new(window, cx));
            let state = cx.new(|cx| {
                QuestionnaireState::new(
                    vec![
                        QuestionnaireItemDefinition::new("first", "First")
                            .with_required(true)
                            .with_multiple(multiple)
                            .with_choices([
                                QuestionnaireChoiceDefinition::new("alpha", "Alpha"),
                                QuestionnaireChoiceDefinition::new("beta", "Beta"),
                            ])
                            .with_input(QuestionnaireInputDefinition::new(input, "Other")),
                        QuestionnaireItemDefinition::new("second", "Second"),
                    ],
                    cx,
                )
                .unwrap()
            });
            QuestionnaireHarness {
                state,
                override_skip_margin: false,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let state = cx.update(|_, cx| view.read(cx).state.clone());
        (cx, state)
    }

    fn actions_visual_harness(
        cx: &mut TestAppContext,
        override_skip_margin: bool,
    ) -> (&mut VisualTestContext, Entity<QuestionnaireState>) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            let state = cx.new(|cx| {
                QuestionnaireState::new(
                    vec![
                        QuestionnaireItemDefinition::new("first", "First").with_required(true),
                        QuestionnaireItemDefinition::new("second", "Second"),
                        QuestionnaireItemDefinition::new("third", "Third").with_required(true),
                    ],
                    cx,
                )
                .unwrap()
                .with_current_item("second")
                .unwrap()
            });
            QuestionnaireHarness {
                state,
                override_skip_margin,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let state = cx.update(|_, cx| view.read(cx).state.clone());
        (cx, state)
    }

    fn focus_input(cx: &mut VisualTestContext, state: &Entity<QuestionnaireState>, item: &str) {
        cx.update(|window, cx| {
            let input = state.read(cx).input_state(item).unwrap();
            let focus_handle = input.read(cx).focus_handle(cx);
            focus_handle.focus(window, cx);
        });
    }

    fn simulate_key(cx: &mut VisualTestContext, key: &str, is_held: bool, simulate_ime: bool) {
        let mut keystroke = Keystroke::parse(key).unwrap();
        if simulate_ime {
            keystroke = keystroke.with_simulated_ime();
        }
        cx.simulate_event(KeyDownEvent {
            keystroke,
            is_held,
            prefer_character_input: false,
        });
    }

    #[gpui::test]
    fn progress_projects_numeric_accessibility(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            let state = cx.new(|cx| {
                QuestionnaireState::new(
                    vec![
                        QuestionnaireItemDefinition::new("first", "First"),
                        QuestionnaireItemDefinition::new("second", "Second"),
                    ],
                    cx,
                )
                .unwrap()
            });
            let mut node = accesskit::Node::new(Role::ProgressIndicator);
            QuestionnaireProgress::new(&state)
                .render(window, cx)
                .into_element()
                .write_a11y_info(&mut node);

            assert_eq!(node.numeric_value(), Some(1.));
            assert_eq!(node.min_numeric_value(), Some(0.));
            assert_eq!(node.max_numeric_value(), Some(2.));

            let root = Questionnaire::new(&state).render(window, cx).into_element();
            assert_eq!(root.a11y_role(), Some(Role::Form));
        });
    }

    #[gpui::test]
    fn actions_stay_inside_questionnaire_width(cx: &mut TestAppContext) {
        let (cx, state) = actions_visual_harness(cx, false);
        let entity_id = state.entity_id();
        let root_id = Box::leak(format!("questionnaire-{entity_id}-root").into_boxed_str());
        let actions_id = Box::leak(format!("questionnaire-{entity_id}-actions").into_boxed_str());
        let previous_id = Box::leak(format!("questionnaire-{entity_id}-Previous").into_boxed_str());
        let skip_id = Box::leak(format!("questionnaire-{entity_id}-Skip").into_boxed_str());
        let next_id = Box::leak(format!("questionnaire-{entity_id}-Next").into_boxed_str());
        let submit_id = Box::leak(format!("questionnaire-{entity_id}-Submit").into_boxed_str());
        let root = cx.debug_bounds(root_id).expect("questionnaire rendered");
        let actions = cx.debug_bounds(actions_id).expect("actions rendered");
        let previous = cx.debug_bounds(previous_id).expect("previous rendered");
        let skip = cx.debug_bounds(skip_id).expect("skip rendered");
        let next = cx.debug_bounds(next_id).expect("next rendered");

        assert!(actions.left() >= root.left());
        assert!(actions.right() <= root.right());
        assert_eq!(previous.left(), actions.left());
        assert!(previous.right() <= skip.left());
        assert!(skip.right() <= next.left());
        assert_eq!(next.right(), actions.right());

        cx.update(|window, cx| {
            state
                .update(cx, |state, cx| state.set_current_item("first", window, cx))
                .unwrap();
            window.draw(cx).clear(cx);
        });
        let first_actions = cx.debug_bounds(actions_id).expect("first actions rendered");
        let first_next = cx.debug_bounds(next_id).expect("first next rendered");
        assert!(first_next.left() >= first_actions.left());
        assert_eq!(first_next.right(), first_actions.right());

        cx.update(|window, cx| {
            state
                .update(cx, |state, cx| state.set_current_item("third", window, cx))
                .unwrap();
            window.draw(cx).clear(cx);
        });
        let last_actions = cx.debug_bounds(actions_id).expect("last actions rendered");
        let last_previous = cx
            .debug_bounds(previous_id)
            .expect("last previous rendered");
        let submit = cx.debug_bounds(submit_id).expect("submit rendered");
        assert_eq!(last_previous.left(), last_actions.left());
        assert!(last_previous.right() <= submit.left());
        assert_eq!(submit.right(), last_actions.right());
    }

    #[gpui::test]
    fn action_instance_style_overrides_default_trailing_anchor(cx: &mut TestAppContext) {
        let (cx, state) = actions_visual_harness(cx, true);
        let entity_id = state.entity_id();
        let actions_id = Box::leak(format!("questionnaire-{entity_id}-actions").into_boxed_str());
        let previous_id = Box::leak(format!("questionnaire-{entity_id}-Previous").into_boxed_str());
        let skip_id = Box::leak(format!("questionnaire-{entity_id}-Skip").into_boxed_str());
        let next_id = Box::leak(format!("questionnaire-{entity_id}-Next").into_boxed_str());
        let actions = cx.debug_bounds(actions_id).expect("actions rendered");
        let previous = cx.debug_bounds(previous_id).expect("previous rendered");
        let skip = cx.debug_bounds(skip_id).expect("skip rendered");
        let next = cx.debug_bounds(next_id).expect("next rendered");

        assert_eq!(previous.left(), actions.left());
        assert!(previous.right() <= skip.left());
        assert!(skip.right() <= next.left());
        assert!(next.right() < actions.right());
    }

    #[gpui::test]
    fn shortcut_guards_held_keys_before_activation(cx: &mut TestAppContext) {
        let (cx, state) = visual_harness(
            cx,
            vec![
                QuestionnaireItemDefinition::new("choice", "Choice")
                    .with_choice(QuestionnaireChoiceDefinition::new("alpha", "Alpha")),
                QuestionnaireItemDefinition::new("second", "Second"),
            ],
            Some(QuestionnaireShortcutMode::Letters),
        );

        simulate_key(cx, "a", true, true);
        simulate_key(cx, "a", false, false);
        simulate_key(cx, "shift-a", false, true);
        cx.update(|_, cx| assert!(state.read(cx).answer("choice").unwrap().is_empty()));

        simulate_key(cx, "a", false, true);
        cx.update(|window, cx| {
            assert_eq!(
                state.read(cx).answer("choice").unwrap().choices(),
                &[SharedString::from("alpha")]
            );
            assert_eq!(
                state
                    .read(cx)
                    .focused_current_choice(window)
                    .map(SharedString::as_ref),
                Some("alpha")
            );
        });
        simulate_key(cx, "enter", false, false);
        cx.update(|_, cx| {
            assert_eq!(state.read(cx).current_item().unwrap().as_ref(), "second");
        });
    }

    #[gpui::test]
    fn root_keyboard_preserves_navigation_and_radio_semantics(cx: &mut TestAppContext) {
        let (cx, state) = visual_harness(
            cx,
            vec![
                QuestionnaireItemDefinition::new("first", "First")
                    .with_required(true)
                    .with_choices([
                        QuestionnaireChoiceDefinition::new("alpha", "Alpha"),
                        QuestionnaireChoiceDefinition::new("beta", "Beta"),
                    ]),
                QuestionnaireItemDefinition::new("second", "Second"),
            ],
            None,
        );

        simulate_key(cx, "right", false, false);
        cx.update(|_, cx| {
            assert_eq!(state.read(cx).current_item().unwrap().as_ref(), "first");
            assert!(state.read(cx).answer("first").unwrap().is_empty());
        });

        cx.update(|window, cx| {
            let focus_handle = state
                .read(cx)
                .choice_focus_handle("first", "alpha")
                .unwrap()
                .clone();
            focus_handle.focus(window, cx);
        });
        simulate_key(cx, "enter", false, false);
        cx.update(|_, cx| {
            assert_eq!(state.read(cx).current_item().unwrap().as_ref(), "first");
            assert!(state.read(cx).answer("first").unwrap().is_empty());
        });

        simulate_key(cx, "right", false, false);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.update(|_, cx| {
            assert_eq!(
                state.read(cx).answer("first").unwrap().choices(),
                &[SharedString::from("beta")]
            );
            assert_eq!(state.read(cx).current_item().unwrap().as_ref(), "first");
        });

        simulate_key(cx, "enter", false, false);
        cx.update(|_, cx| {
            assert_eq!(state.read(cx).current_item().unwrap().as_ref(), "second");
        });
    }

    #[gpui::test]
    fn empty_input_enter_stays_put_and_arrows_move_to_answers(cx: &mut TestAppContext) {
        let (cx, state) = input_visual_harness(cx, false);
        focus_input(cx, &state, "first");

        simulate_key(cx, "enter", false, false);
        cx.update(|_, cx| {
            assert_eq!(state.read(cx).current_item().unwrap().as_ref(), "first");
            assert!(state.read(cx).answer("first").unwrap().is_empty());
            assert!(state.read(cx).error("first").is_none());
        });

        simulate_key(cx, "secondary-enter", false, false);
        cx.update(|_, cx| {
            assert_eq!(state.read(cx).current_item().unwrap().as_ref(), "first");
            assert!(state.read(cx).error("first").is_some());
        });

        focus_input(cx, &state, "first");
        simulate_key(cx, "up", false, false);
        cx.update(|window, cx| {
            assert_eq!(
                state
                    .read(cx)
                    .focused_current_choice(window)
                    .map(SharedString::as_ref),
                Some("beta")
            );
        });

        simulate_key(cx, "down", false, false);
        cx.update(|window, cx| {
            assert!(state.read(cx).is_current_input_focused(window));
            assert_eq!(
                state.read(cx).answer("first").unwrap().choices(),
                &[SharedString::from("beta")]
            );
        });

        simulate_key(cx, "down", false, false);
        cx.update(|window, cx| {
            assert_eq!(
                state
                    .read(cx)
                    .focused_current_choice(window)
                    .map(SharedString::as_ref),
                Some("alpha")
            );
            assert_eq!(
                state.read(cx).answer("first").unwrap().choices(),
                &[SharedString::from("alpha")]
            );
        });

        simulate_key(cx, "down", false, false);
        cx.update(|window, cx| {
            assert_eq!(
                state
                    .read(cx)
                    .focused_current_choice(window)
                    .map(SharedString::as_ref),
                Some("beta")
            );
            assert_eq!(
                state.read(cx).answer("first").unwrap().choices(),
                &[SharedString::from("beta")]
            );
        });

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state
                    .set_input_value("first", "Preserved draft", window, cx)
                    .unwrap();
                state.activate_choice("first", "alpha", cx).unwrap();
            });
        });
        focus_input(cx, &state, "first");
        simulate_key(cx, "enter", false, false);
        cx.update(|_, cx| {
            let answer = state.read(cx).answer("first").unwrap();
            assert_eq!(state.read(cx).current_item().unwrap().as_ref(), "first");
            assert_eq!(answer.choices(), &[SharedString::from("alpha")]);
            assert!(answer.freeform().is_none());
        });
    }

    #[gpui::test]
    fn filled_group_input_keeps_text_editing_directions(cx: &mut TestAppContext) {
        let (cx, state) = input_visual_harness(cx, true);
        cx.update(|window, cx| {
            state
                .update(cx, |state, cx| {
                    state.set_input_value("first", "Freeform answer", window, cx)
                })
                .unwrap();
        });
        focus_input(cx, &state, "first");

        simulate_key(cx, "down", false, false);
        cx.update(|window, cx| {
            let state = state.read(cx);
            assert!(state.is_current_input_focused(window));
            assert_eq!(
                state
                    .answer("first")
                    .unwrap()
                    .freeform()
                    .map(SharedString::as_ref),
                Some("Freeform answer")
            );
            assert!(state.answer("first").unwrap().choices().is_empty());
        });

        simulate_key(cx, "up", false, false);
        cx.update(|window, cx| {
            let state = state.read(cx);
            assert!(state.is_current_input_focused(window));
            assert_eq!(
                state
                    .answer("first")
                    .unwrap()
                    .freeform()
                    .map(SharedString::as_ref),
                Some("Freeform answer")
            );
            assert!(state.answer("first").unwrap().choices().is_empty());
        });
    }

    #[test]
    fn invalid_error_projects_alert_role() {
        let error = questionnaire_error_root("questionnaire-error-test".into()).into_element();
        assert_eq!(error.a11y_role(), Some(Role::Alert));
    }
}
