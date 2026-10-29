use std::rc::Rc;

use gpui::{
    Anchor, App, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    Disableable as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::{DropdownMenu as _, PopupMenuItem},
};

/// One choice in a [`ModelSelector`] menu: a model, or a reasoning effort.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelOption {
    id: SharedString,
    label: SharedString,
}

impl ModelOption {
    /// Create a choice; `id` is sent to the provider, `label` is shown.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }

    /// The value sent to the provider, such as `gpt-5` or `high`.
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    /// The name shown to the user.
    pub fn label(&self) -> &SharedString {
        &self.label
    }
}

type OptionHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// Two menus for picking the model and how hard it thinks.
///
/// Both selections are controlled. The effort menu is shown only when
/// effort options are given, since not every model reasons.
#[derive(IntoElement)]
pub struct ModelSelector {
    id: ElementId,
    style: StyleRefinement,
    models: Vec<ModelOption>,
    efforts: Vec<ModelOption>,
    model: Option<SharedString>,
    effort: Option<SharedString>,
    disabled: bool,
    on_model_change: Option<OptionHandler>,
    on_effort_change: Option<OptionHandler>,
}

impl ModelSelector {
    /// Create a selector without options.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            models: Vec::new(),
            efforts: Vec::new(),
            model: None,
            effort: None,
            disabled: false,
            on_model_change: None,
            on_effort_change: None,
        }
    }

    /// Set the models to choose from.
    pub fn models(mut self, models: impl IntoIterator<Item = ModelOption>) -> Self {
        self.models = models.into_iter().collect();
        self
    }

    /// Set the reasoning efforts to choose from.
    pub fn efforts(mut self, efforts: impl IntoIterator<Item = ModelOption>) -> Self {
        self.efforts = efforts.into_iter().collect();
        self
    }

    /// Set the selected model id.
    pub fn model(mut self, id: impl Into<SharedString>) -> Self {
        self.model = Some(id.into());
        self
    }

    /// Set the selected effort id.
    pub fn effort(mut self, id: impl Into<SharedString>) -> Self {
        self.effort = Some(id.into());
        self
    }

    /// Set whether the selection can change, for example while a reply
    /// streams.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the chosen model id.
    pub fn on_model_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_model_change = Some(Rc::new(handler));
        self
    }

    /// Called with the chosen effort id.
    pub fn on_effort_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_effort_change = Some(Rc::new(handler));
        self
    }

    fn menu(
        id: ElementId,
        icon: Option<IconName>,
        accessibility_label: SharedString,
        options: Vec<ModelOption>,
        selected: Option<SharedString>,
        disabled: bool,
        on_change: Option<OptionHandler>,
    ) -> impl IntoElement {
        let label = options
            .iter()
            .find(|option| Some(&option.id) == selected.as_ref())
            .map(|option| option.label.clone())
            .or_else(|| selected.clone())
            .unwrap_or_default();

        Button::new(id)
            .ghost()
            .small()
            .dropdown_caret(true)
            .disabled(disabled)
            .accessibility_label(accessibility_label)
            .when_some(icon, |this, icon| this.icon(Icon::new(icon)))
            .label(label)
            .dropdown_menu_with_anchor(Anchor::BottomLeft, move |menu, _, _| {
                options.iter().fold(menu, |menu, option| {
                    let on_change = on_change.clone();
                    let id = option.id.clone();
                    menu.item(
                        PopupMenuItem::new(option.label.clone())
                            .checked(Some(&option.id) == selected.as_ref())
                            .on_click(move |_, window, cx| {
                                if let Some(on_change) = &on_change {
                                    on_change(&id, window, cx);
                                }
                            }),
                    )
                })
            })
    }
}

impl Styled for ModelSelector {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ModelSelector {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let id = self.id;

        h_flex()
            .gap_1()
            .child(Self::menu(
                (id.clone(), "model").into(),
                None,
                t!("ModelSelector.model").into(),
                self.models,
                self.model,
                self.disabled,
                self.on_model_change,
            ))
            .when(!self.efforts.is_empty(), |this| {
                this.child(Self::menu(
                    (id, "effort").into(),
                    Some(IconName::Brain),
                    t!("ModelSelector.effort").into(),
                    self.efforts,
                    self.effort,
                    self.disabled,
                    self.on_effort_change,
                ))
            })
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_selector_builder() {
        let selector = ModelSelector::new("selector")
            .models([ModelOption::new("gpt-5", "GPT-5")])
            .efforts([
                ModelOption::new("low", "Low"),
                ModelOption::new("high", "High"),
            ])
            .model("gpt-5")
            .effort("high")
            .disabled(true)
            .on_model_change(|_, _, _| {})
            .on_effort_change(|_, _, _| {});

        assert_eq!(selector.models[0].label(), "GPT-5");
        assert_eq!(selector.efforts.len(), 2);
        assert_eq!(selector.model.as_deref(), Some("gpt-5"));
        assert_eq!(selector.effort.as_deref(), Some("high"));
        assert!(selector.disabled);
        assert!(selector.on_model_change.is_some());
        assert!(selector.on_effort_change.is_some());
    }
}
