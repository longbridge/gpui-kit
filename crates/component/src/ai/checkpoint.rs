use std::rc::Rc;

use gpui::{
    App, ClickEvent, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    separator::Separator,
};

/// A point in the conversation the user can return to.
///
/// Rendered as a divider between turns that names the checkpoint and offers
/// `Restore checkpoint`. Restoring is the application's job: it rewinds the
/// transcript and any files the agent changed.
#[derive(IntoElement)]
pub struct Checkpoint {
    id: ElementId,
    style: StyleRefinement,
    label: Option<SharedString>,
    on_restore: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl Checkpoint {
    /// Create a checkpoint with a stable identity.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            label: None,
            on_restore: None,
        }
    }

    /// Replace the default `Checkpoint` label, for example with a time.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Called when the user restores the checkpoint. Without a handler the
    /// restore button is not shown.
    pub fn on_restore(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_restore = Some(Rc::new(handler));
        self
    }
}

impl Styled for Checkpoint {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Checkpoint {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted_foreground = cx.theme().muted_foreground;

        h_flex()
            .w_full()
            .gap_2()
            .text_xs()
            .text_color(muted_foreground)
            .child(Separator::horizontal().flex_1())
            .child(Icon::new(IconName::Bookmark).xsmall())
            .child(self.label.unwrap_or_else(|| t!("Checkpoint.label").into()))
            .when_some(self.on_restore, |this, on_restore| {
                this.child(
                    Button::new(self.id)
                        .ghost()
                        .xsmall()
                        .label(SharedString::from(t!("Checkpoint.restore")))
                        .on_click(move |event, window, cx| on_restore(event, window, cx)),
                )
            })
            .child(Separator::horizontal().flex_1())
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_builder() {
        let checkpoint = Checkpoint::new("checkpoint")
            .label("10:42")
            .on_restore(|_, _, _| {});
        assert_eq!(checkpoint.label.as_deref(), Some("10:42"));
        assert!(checkpoint.on_restore.is_some());
    }
}
