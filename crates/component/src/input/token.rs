use super::InlineTokenContext;
use crate::{ActiveTheme as _, Icon, StyledExt as _, tooltip::Tooltip};
use gpui::{
    App, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _,
};

/// Default inline token skin. Editing and activation belong to the input.
#[derive(IntoElement)]
pub struct InputToken {
    context: InlineTokenContext,
    icon: Option<Icon>,
    tooltip: Option<SharedString>,
    style: StyleRefinement,
}
impl InputToken {
    pub fn new(context: &InlineTokenContext) -> Self {
        Self {
            context: context.clone(),
            icon: None,
            tooltip: None,
            style: Default::default(),
        }
    }
    pub fn with_icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }
    pub fn with_tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}
impl Styled for InputToken {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl RenderOnce for InputToken {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id("input-token-skin")
            .flex()
            .items_center()
            .gap_1()
            .px_1()
            .h(self.context.line_height())
            .max_w(self.context.available_width())
            .rounded(cx.theme().radius)
            .bg(if self.context.is_selected() {
                cx.theme().selection
            } else {
                cx.theme().muted
            })
            .text_color(cx.theme().foreground)
            .when(self.context.is_disabled(), |this| this.opacity(0.5))
            .when_some(self.icon, |this, icon| {
                this.child(icon.size_3().flex_shrink_0())
            })
            .child(
                div()
                    .min_w_0()
                    .text_ellipsis()
                    .child(self.context.token().label().clone()),
            )
            .when_some(self.tooltip, |this, tooltip| {
                this.tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            })
            .refine_style(&self.style)
    }
}
