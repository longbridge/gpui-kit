use gpui::{
    App, ElementId, Entity, Focusable as _, IntoElement, ParentElement as _, RenderOnce,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

use crate::{
    ActiveTheme as _, Disableable, Sizable, Size, StyleSized as _, StyledExt as _,
    ThemeStyled as _, input::input_style,
};

use gpui_base::TimeField as BaseTimeField;
pub use gpui_base::{TimeFieldEvent, TimeFieldState, TimePrecision, TimeSegment};

/// A segmented time editor, e.g. `09:30` or `09:30:15`.
///
/// The value lives in [`TimeFieldState`]; see it for the keyboard model.
#[derive(IntoElement)]
pub struct TimeField {
    id: ElementId,
    state: Entity<TimeFieldState>,
    size: Size,
    style: StyleRefinement,
    disabled: bool,
    invalid: bool,
}

impl TimeField {
    pub fn new(state: &Entity<TimeFieldState>) -> Self {
        Self {
            id: ("time-field", state.entity_id()).into(),
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
            disabled: false,
            invalid: false,
        }
    }

    pub(crate) fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Display the caller's validation result. This does not reject edits.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }
}

impl Sizable for TimeField {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for TimeField {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for TimeField {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TimeField {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focused = self.state.read(cx).focus_handle(cx).is_focused(window);
        let (bg, fg) = input_style(self.disabled, cx);
        let segment_radius = cx.theme().radius / 2.;

        div()
            .flex()
            .items_center()
            .flex_none()
            .bg(bg)
            .text_color(fg)
            .border_1()
            .border_color(cx.theme().input)
            .rounded(cx.theme().radius)
            .input_text_size(self.size)
            .input_h(self.size)
            .px_1()
            .when(self.disabled, |this| this.opacity(0.5))
            .when(focused && !self.disabled, |this| {
                this.focus_ring_style(window, cx)
            })
            .when(self.invalid, |this| this.border_color(cx.theme().danger))
            .child(
                BaseTimeField::new(self.id, &self.state)
                    .disabled(self.disabled)
                    .h_full()
                    .items_center()
                    .segment(move |segment, state, _, cx| {
                        segment
                            .px_0p5()
                            .rounded(segment_radius)
                            .when(state.is_selected(), |this| this.bg(cx.theme().selection))
                            .into_any_element()
                    }),
            )
            .refine_style(&self.style)
    }
}
