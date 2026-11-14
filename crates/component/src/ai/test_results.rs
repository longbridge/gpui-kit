use std::{rc::Rc, time::Duration};

use gpui::{
    App, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, IconName, StyledExt as _,
    ai::{
        plan::aggregate_status,
        trace::{ToggleHandler, Trace, TraceStatus, code_surface},
    },
    h_flex, v_flex,
};

/// One test of a [`TestSuite`].
///
/// [`TraceStatus::Completed`] means passed, [`TraceStatus::Failed`] failed
/// and [`TraceStatus::Skipped`] ignored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestCase {
    name: SharedString,
    status: TraceStatus,
    duration: Option<Duration>,
    message: Option<SharedString>,
}

impl TestCase {
    /// Create a passed test.
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            status: TraceStatus::Completed,
            duration: None,
            message: None,
        }
    }

    /// Set the outcome of the test.
    pub fn status(mut self, status: TraceStatus) -> Self {
        self.status = status;
        self
    }

    /// Set how long the test ran.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Set the failure message; it also marks the test failed.
    pub fn failure(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self.status = TraceStatus::Failed;
        self
    }
}

/// A named group of [`TestCase`]s, such as one test file or module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestSuite {
    name: SharedString,
    cases: Vec<TestCase>,
}

impl TestSuite {
    /// Create an empty suite.
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            cases: Vec::new(),
        }
    }

    /// Add a test.
    pub fn case(mut self, case: TestCase) -> Self {
        self.cases.push(case);
        self
    }
}

/// Counts of a test run's outcomes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TestSummary {
    passed: usize,
    failed: usize,
    skipped: usize,
    running: usize,
}

impl TestSummary {
    /// The number of tests that passed.
    pub fn passed(&self) -> usize {
        self.passed
    }

    /// The number of tests that failed.
    pub fn failed(&self) -> usize {
        self.failed
    }

    /// The number of tests that were skipped.
    pub fn skipped(&self) -> usize {
        self.skipped
    }

    /// The number of tests not finished yet.
    pub fn running(&self) -> usize {
        self.running
    }
}

/// A test run with its suites and failures.
///
/// The header counts outcomes. The content lists each suite; failing tests
/// show their message, passing tests stay one line each.
#[derive(IntoElement)]
pub struct TestResults {
    id: ElementId,
    style: StyleRefinement,
    suites: Vec<TestSuite>,
    open: bool,
    on_toggle: Option<ToggleHandler>,
}

impl TestResults {
    /// Create an empty, closed run.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            suites: Vec::new(),
            open: false,
            on_toggle: None,
        }
    }

    /// Add a suite.
    pub fn suite(mut self, suite: TestSuite) -> Self {
        self.suites.push(suite);
        self
    }

    /// Set whether the suites are revealed.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Called with the requested open state when the header is activated.
    pub fn on_toggle(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    /// Count the outcomes across every suite.
    pub fn summary(&self) -> TestSummary {
        let mut summary = TestSummary::default();
        for case in self.suites.iter().flat_map(|suite| &suite.cases) {
            match case.status {
                TraceStatus::Completed => summary.passed += 1,
                TraceStatus::Failed => summary.failed += 1,
                TraceStatus::Skipped => summary.skipped += 1,
                TraceStatus::Pending | TraceStatus::Running => summary.running += 1,
            }
        }
        summary
    }

    fn render_case(case: TestCase, cx: &App) -> impl IntoElement {
        let muted_foreground = cx.theme().muted_foreground;
        let danger = cx.theme().danger;

        v_flex()
            .gap_1()
            .child(
                h_flex()
                    .gap_2()
                    .child(case.status.indicator(cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(case.name),
                    )
                    .when_some(case.duration, |this, duration| {
                        this.child(
                            div()
                                .flex_none()
                                .text_xs()
                                .text_color(muted_foreground)
                                .child(format!("{} ms", duration.as_millis())),
                        )
                    }),
            )
            .when_some(case.message, |this, message| {
                this.child(
                    div()
                        .pl_5()
                        .child(code_surface(cx).text_color(danger).child(message)),
                )
            })
    }
}

impl Styled for TestResults {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TestResults {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let summary = self.summary();
        let status = aggregate_status(
            self.suites
                .iter()
                .flat_map(|suite| suite.cases.iter().map(|case| case.status)),
        );
        let muted_foreground = cx.theme().muted_foreground;

        let mut counts = vec![t!("TestResults.passed", count = summary.passed).to_string()];
        if summary.failed > 0 {
            counts.push(t!("TestResults.failed", count = summary.failed).to_string());
        }
        if summary.skipped > 0 {
            counts.push(t!("TestResults.skipped", count = summary.skipped).to_string());
        }

        let trace = Trace::new(self.id)
            .icon(IconName::FlaskConical)
            .title(SharedString::from(t!("TestResults.title")))
            .description(SharedString::from(counts.join(" · ")))
            .status(status)
            .open(self.open)
            .with_toggle_handler(self.on_toggle)
            .refine_style(&self.style);

        self.suites
            .into_iter()
            .map(|suite| {
                v_flex()
                    .gap_1p5()
                    .child(
                        div()
                            .text_xs()
                            .font_medium()
                            .text_color(muted_foreground)
                            .child(suite.name),
                    )
                    .children(
                        suite
                            .cases
                            .into_iter()
                            .map(|case| Self::render_case(case, cx)),
                    )
            })
            .fold(trace, |trace, suite| trace.content(suite))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_results_builder() {
        let results = TestResults::new("tests")
            .suite(
                TestSuite::new("button")
                    .case(TestCase::new("renders").duration(Duration::from_millis(4)))
                    .case(TestCase::new("clicks").failure("expected 1, got 0")),
            )
            .suite(TestSuite::new("input").case(TestCase::new("ime").status(TraceStatus::Skipped)))
            .open(true)
            .on_toggle(|_, _, _| {});

        let summary = results.summary();
        assert_eq!(summary.passed(), 1);
        assert_eq!(summary.failed(), 1);
        assert_eq!(summary.skipped(), 1);
        assert_eq!(summary.running(), 0);
        assert_eq!(results.suites.len(), 2);
        assert!(results.open);
    }
}
