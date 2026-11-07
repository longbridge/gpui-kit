use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, IconName, StyledExt as _,
    ai::trace::{ToggleHandler, Trace, TraceStatus},
    h_flex,
};

/// One step of a [`Plan`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanStep {
    title: SharedString,
    status: TraceStatus,
}

impl PlanStep {
    /// Create a pending step.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            status: TraceStatus::Pending,
        }
    }

    /// Set the progress of the step.
    pub fn status(mut self, status: TraceStatus) -> Self {
        self.status = status;
        self
    }

    /// The step's title.
    pub fn title(&self) -> &SharedString {
        &self.title
    }
}

/// The agent's planned steps and their progress.
///
/// The header summarizes progress as `2 of 5`; its status is derived from
/// the steps, so the application only updates each step.
#[derive(IntoElement)]
pub struct Plan {
    id: ElementId,
    style: StyleRefinement,
    title: Option<SharedString>,
    steps: Vec<PlanStep>,
    open: bool,
    on_toggle: Option<ToggleHandler>,
}

impl Plan {
    /// Create an empty, closed plan.
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

    /// Replace the default `Plan` title, for example with the goal.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Add a step.
    pub fn step(mut self, step: PlanStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Add several steps.
    pub fn steps(mut self, steps: impl IntoIterator<Item = PlanStep>) -> Self {
        self.steps.extend(steps);
        self
    }

    /// Set whether the steps are revealed.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Called with the requested open state when the header is activated.
    pub fn on_toggle(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    /// The number of completed steps.
    pub fn completed_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.status == TraceStatus::Completed)
            .count()
    }

    /// The plan's overall status, derived from its steps.
    pub fn status(&self) -> TraceStatus {
        aggregate_status(self.steps.iter().map(|step| step.status))
    }
}

/// Fold child statuses into one: any failure fails the whole, running work
/// keeps it running, and only all-finished work completes it.
pub(crate) fn aggregate_status(statuses: impl IntoIterator<Item = TraceStatus>) -> TraceStatus {
    let statuses: Vec<TraceStatus> = statuses.into_iter().collect();
    if statuses.contains(&TraceStatus::Failed) {
        TraceStatus::Failed
    } else if statuses.contains(&TraceStatus::Running) {
        TraceStatus::Running
    } else if !statuses.is_empty() && statuses.iter().all(|status| status.is_finished()) {
        TraceStatus::Completed
    } else if statuses.iter().any(|status| status.is_finished()) {
        TraceStatus::Running
    } else {
        TraceStatus::Pending
    }
}

impl Styled for Plan {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Plan {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted_foreground = cx.theme().muted_foreground;
        let progress = t!(
            "Plan.progress",
            completed = self.completed_count(),
            total = self.steps.len()
        );
        let status = self.status();

        Trace::new(self.id)
            .icon(IconName::ListChecks)
            .title(self.title.unwrap_or_else(|| t!("Plan.title").into()))
            .description(SharedString::from(progress))
            .status(status)
            .open(self.open)
            .with_toggle_handler(self.on_toggle)
            .when(!self.steps.is_empty(), |this| {
                this.content(
                    div()
                        .v_flex()
                        .gap_1p5()
                        .children(self.steps.into_iter().map(|step| {
                            let finished = step.status.is_finished();
                            h_flex()
                                .items_start()
                                .gap_2()
                                .child(div().pt_0p5().child(step.status.indicator(cx)))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .when(finished, |this| this.text_color(muted_foreground))
                                        .when(step.status == TraceStatus::Completed, |this| {
                                            this.line_through()
                                        })
                                        .child(step.title),
                                )
                        })),
                )
            })
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_builder() {
        let plan = Plan::new("plan")
            .title("Ship the release")
            .step(PlanStep::new("Bump version").status(TraceStatus::Completed))
            .steps([
                PlanStep::new("Write notes").status(TraceStatus::Running),
                PlanStep::new("Tag"),
            ])
            .open(true)
            .on_toggle(|_, _, _| {});

        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.completed_count(), 1);
        assert_eq!(plan.status(), TraceStatus::Running);
        assert!(plan.open);
        assert!(plan.on_toggle.is_some());
        assert_eq!(plan.steps[2].title(), "Tag");
    }

    #[test]
    fn test_aggregate_status() {
        use TraceStatus::*;
        assert_eq!(aggregate_status([]), Pending);
        assert_eq!(aggregate_status([Pending, Pending]), Pending);
        assert_eq!(aggregate_status([Completed, Pending]), Running);
        assert_eq!(aggregate_status([Completed, Skipped]), Completed);
        assert_eq!(aggregate_status([Completed, Running]), Running);
        assert_eq!(aggregate_status([Running, Failed]), Failed);
        assert_eq!(aggregate_status([Failed, Running]), Failed);
    }
}
