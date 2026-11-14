use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    ai::{
        plan::aggregate_status,
        trace::{ToggleHandler, Trace, TraceStatus},
    },
    h_flex, v_flex,
};

/// One step the agent took inside a [`ChainOfThought`].
#[derive(IntoElement)]
pub struct ChainOfThoughtStep {
    label: SharedString,
    description: Option<SharedString>,
    icon: Option<Icon>,
    status: TraceStatus,
    children: Vec<AnyElement>,
}

impl ChainOfThoughtStep {
    /// Create a completed step.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            description: None,
            icon: None,
            status: TraceStatus::Completed,
            children: Vec::new(),
        }
    }

    /// Set supporting detail shown under the label.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Replace the status glyph with an icon naming the kind of step, such
    /// as search. Running steps keep the spinner.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the progress of the step.
    pub fn status(mut self, status: TraceStatus) -> Self {
        self.status = status;
        self
    }
}

impl ParentElement for ChainOfThoughtStep {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ChainOfThoughtStep {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted_foreground = cx.theme().muted_foreground;
        let indicator = match self.icon {
            Some(icon) if self.status != TraceStatus::Running => icon
                .xsmall()
                .text_color(muted_foreground)
                .into_any_element(),
            _ => self.status.indicator(cx),
        };

        h_flex()
            .items_start()
            .gap_2()
            .child(div().pt_0p5().child(indicator))
            .child(
                v_flex()
                    .min_w_0()
                    .flex_1()
                    .gap_1()
                    .child(
                        div()
                            .when(self.status == TraceStatus::Pending, |this| {
                                this.text_color(muted_foreground)
                            })
                            .child(self.label),
                    )
                    .when_some(self.description, |this, description| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(muted_foreground)
                                .child(description),
                        )
                    })
                    .children(self.children),
            )
    }
}

/// A summary that expands into the steps the agent took.
#[derive(IntoElement)]
pub struct ChainOfThought {
    id: ElementId,
    style: StyleRefinement,
    title: Option<SharedString>,
    steps: Vec<ChainOfThoughtStep>,
    open: bool,
    on_toggle: Option<ToggleHandler>,
}

impl ChainOfThought {
    /// Create an empty, closed chain of thought.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            title: None,
            steps: Vec::new(),
            open: false,
            on_toggle: None,
        }
    }

    /// Replace the default `Chain of thought` summary.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Add a step.
    pub fn step(mut self, step: ChainOfThoughtStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Set whether the steps are revealed.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Called with the requested open state when the summary is activated.
    pub fn on_toggle(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl Styled for ChainOfThought {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ChainOfThought {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let status = aggregate_status(self.steps.iter().map(|step| step.status));

        Trace::new(self.id)
            .icon(IconName::Brain)
            .title(
                self.title
                    .unwrap_or_else(|| t!("ChainOfThought.title").into()),
            )
            .status(status)
            .open(self.open)
            .with_toggle_handler(self.on_toggle)
            .when(!self.steps.is_empty(), |this| {
                this.content(v_flex().gap_3().children(self.steps))
            })
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_of_thought_builder() {
        let chain = ChainOfThought::new("chain")
            .title("Searched the web")
            .step(
                ChainOfThoughtStep::new("Searching")
                    .description("gpui layout")
                    .icon(IconName::Search)
                    .status(TraceStatus::Running)
                    .child("Result"),
            )
            .step(ChainOfThoughtStep::new("Reading"))
            .open(true)
            .on_toggle(|_, _, _| {});

        assert_eq!(chain.steps.len(), 2);
        assert_eq!(chain.steps[0].children.len(), 1);
        assert!(chain.steps[0].icon.is_some());
        assert_eq!(chain.steps[1].status, TraceStatus::Completed);
        assert!(chain.open);
        assert!(chain.on_toggle.is_some());
    }
}
