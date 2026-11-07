use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, prelude::FluentBuilder as _,
};

use crate::{
    ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _, button::Button, h_flex,
};

/// A short prompt the user can send to the agent in one click.
///
/// The handler receives the suggestion's text, so one handler can serve a
/// whole [`Suggestions`] row.
#[derive(IntoElement)]
pub struct Suggestion {
    id: ElementId,
    text: SharedString,
    disabled: bool,
    on_click: Option<Rc<dyn Fn(&SharedString, &mut Window, &mut App)>>,
}

impl Suggestion {
    /// Create a suggestion that sends `text`.
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            disabled: false,
            on_click: None,
        }
    }

    /// Set whether the suggestion can be sent, for example while a reply
    /// streams.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the suggestion's text when it is chosen.
    pub fn on_click(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Suggestion {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text = self.text.clone();
        Button::new(self.id)
            .outline()
            .small()
            .rounded(cx.theme().radius_full())
            .label(self.text)
            .disabled(self.disabled)
            .when_some(self.on_click, |this, on_click| {
                this.on_click(move |_, window, cx| on_click(&text, window, cx))
            })
    }
}

/// A wrapping row of [`Suggestion`]s.
#[derive(IntoElement, Default)]
pub struct Suggestions {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl Suggestions {
    /// Create an empty row.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ParentElement for Suggestions {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Suggestions {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Suggestions {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        h_flex()
            .w_full()
            .flex_wrap()
            .gap_2()
            .refine_style(&self.style)
            .children(self.children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggestion_builder() {
        let suggestion = Suggestion::new("s", "Summarize this thread")
            .disabled(true)
            .on_click(|_, _, _| {});
        assert_eq!(suggestion.text, "Summarize this thread");
        assert!(suggestion.disabled);
        assert!(suggestion.on_click.is_some());

        let row = Suggestions::new().child(Suggestion::new("a", "A"));
        assert_eq!(row.children.len(), 1);
    }
}
