use std::rc::Rc;

use gpui::{
    App, ClickEvent, ElementId, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};

/// Where a request for approval stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ConfirmationState {
    /// Waiting for the user to decide.
    #[default]
    Requested,
    /// The user allowed the action.
    Approved,
    /// The user refused the action.
    Denied,
}

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// Asks the user to approve or deny a tool before it runs.
///
/// While [`ConfirmationState::Requested`], it shows the request with `Deny`
/// and the default commit `Approve`; once decided it collapses to a line that
/// records the decision, so the transcript keeps the history.
#[derive(IntoElement)]
pub struct Confirmation {
    id: ElementId,
    style: StyleRefinement,
    title: SharedString,
    description: Option<SharedString>,
    state: ConfirmationState,
    approve_label: Option<SharedString>,
    deny_label: Option<SharedString>,
    on_approve: Option<ClickHandler>,
    on_deny: Option<ClickHandler>,
}

impl Confirmation {
    /// Create a pending request that names the action, such as
    /// `Run “cargo test”?`.
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            title: title.into(),
            description: None,
            state: ConfirmationState::Requested,
            approve_label: None,
            deny_label: None,
            on_approve: None,
            on_deny: None,
        }
    }

    /// Set the scope or consequence of the action.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set where the request stands.
    pub fn state(mut self, state: ConfirmationState) -> Self {
        self.state = state;
        self
    }

    /// Replace `Approve` with a label that names the result, such as `Run`.
    pub fn approve_label(mut self, label: impl Into<SharedString>) -> Self {
        self.approve_label = Some(label.into());
        self
    }

    /// Replace `Deny` with a more specific label.
    pub fn deny_label(mut self, label: impl Into<SharedString>) -> Self {
        self.deny_label = Some(label.into());
        self
    }

    /// Called when the user approves.
    pub fn on_approve(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_approve = Some(Rc::new(handler));
        self
    }

    /// Called when the user denies.
    pub fn on_deny(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_deny = Some(Rc::new(handler));
        self
    }
}

impl Styled for Confirmation {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Confirmation {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let muted_foreground = theme.muted_foreground;

        if self.state != ConfirmationState::Requested {
            let (icon, color, label) = match self.state {
                ConfirmationState::Approved => (
                    IconName::CircleCheck,
                    theme.success,
                    t!("Confirmation.approved"),
                ),
                _ => (
                    IconName::CircleSlash,
                    muted_foreground,
                    t!("Confirmation.denied"),
                ),
            };
            return h_flex()
                .id(self.id)
                .w_full()
                .min_w_0()
                .gap_2()
                .px_2()
                .text_sm()
                .child(Icon::new(icon).xsmall().text_color(color))
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(self.title),
                )
                .child(div().flex_none().text_color(muted_foreground).child(label))
                .refine_style(&self.style)
                .into_any_element();
        }

        let id = self.id;
        v_flex()
            .id(id.clone())
            .w_full()
            .min_w_0()
            .gap_3()
            .p_3()
            .rounded(theme.radius_lg)
            .border_1()
            .border_color(theme.border)
            .bg(theme.background)
            .text_sm()
            .child(
                h_flex()
                    .items_start()
                    .gap_2()
                    .child(
                        Icon::new(IconName::ShieldAlert)
                            .small()
                            .text_color(theme.warning),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .child(div().font_medium().child(self.title))
                            .when_some(self.description, |this, description| {
                                this.child(div().text_color(muted_foreground).child(description))
                            }),
                    ),
            )
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new((id.clone(), "deny"))
                            .small()
                            .label(
                                self.deny_label
                                    .unwrap_or_else(|| t!("Confirmation.deny").into()),
                            )
                            .when_some(self.on_deny, |this, on_deny| {
                                this.on_click(move |event, window, cx| on_deny(event, window, cx))
                            }),
                    )
                    .child(
                        Button::new((id, "approve"))
                            .small()
                            .primary()
                            .label(
                                self.approve_label
                                    .unwrap_or_else(|| t!("Confirmation.approve").into()),
                            )
                            .when_some(self.on_approve, |this, on_approve| {
                                this.on_click(move |event, window, cx| {
                                    on_approve(event, window, cx)
                                })
                            }),
                    ),
            )
            .refine_style(&self.style)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confirmation_builder() {
        let confirmation = Confirmation::new("confirm", "Run “cargo test”?")
            .description("Runs in the project directory.")
            .state(ConfirmationState::Approved)
            .approve_label("Run")
            .deny_label("Skip")
            .on_approve(|_, _, _| {})
            .on_deny(|_, _, _| {});

        assert_eq!(confirmation.state, ConfirmationState::Approved);
        assert_eq!(confirmation.approve_label.as_deref(), Some("Run"));
        assert_eq!(confirmation.deny_label.as_deref(), Some("Skip"));
        assert!(confirmation.description.is_some());
        assert!(confirmation.on_approve.is_some());
        assert!(confirmation.on_deny.is_some());
        assert_eq!(
            Confirmation::new("c", "t").state,
            ConfirmationState::Requested
        );
    }
}
