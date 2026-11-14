use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, IconName, StyledExt as _,
    ai::trace::{ToggleHandler, Trace, TraceStatus, section_label},
    v_flex,
};

/// A subagent run: the task it was given, the work it did and its result.
///
/// Children are the subagent's own trace — usually [`Tool`](super::Tool)
/// calls — and are indented under the run so ownership stays visible.
#[derive(IntoElement)]
pub struct Subagent {
    id: ElementId,
    style: StyleRefinement,
    name: SharedString,
    task: Option<SharedString>,
    result: Option<AnyElement>,
    status: TraceStatus,
    open: bool,
    on_toggle: Option<ToggleHandler>,
    children: Vec<AnyElement>,
}

impl Subagent {
    /// Create a running subagent named after its role, such as `Explorer`.
    pub fn new(id: impl Into<ElementId>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            name: name.into(),
            task: None,
            result: None,
            status: TraceStatus::Running,
            open: false,
            on_toggle: None,
            children: Vec::new(),
        }
    }

    /// Set the instruction the subagent was given.
    pub fn task(mut self, task: impl Into<SharedString>) -> Self {
        self.task = Some(task.into());
        self
    }

    /// Set what the subagent reported back.
    pub fn result(mut self, result: impl IntoElement) -> Self {
        self.result = Some(result.into_any_element());
        self
    }

    /// Set the progress of the run.
    pub fn status(mut self, status: TraceStatus) -> Self {
        self.status = status;
        self
    }

    /// Set whether the run is revealed.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Called with the requested open state when the header is activated.
    pub fn on_toggle(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for Subagent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Subagent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Subagent {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted_foreground = cx.theme().muted_foreground;
        let border = cx.theme().border;

        Trace::new(self.id)
            .icon(IconName::Workflow)
            .title(self.name)
            .when_some(self.task.clone(), |this, task| {
                this.description(
                    div()
                        .max_w_80()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(task),
                )
            })
            .status(self.status)
            .open(self.open)
            .with_toggle_handler(self.on_toggle)
            .when_some(self.task, |this, task| {
                this.content(div().text_color(muted_foreground).child(task))
            })
            .when(!self.children.is_empty(), |this| {
                this.content(
                    v_flex()
                        .pl_2()
                        .border_l_1()
                        .border_color(border)
                        .children(self.children),
                )
            })
            .when_some(self.result, |this, result| {
                this.content(
                    v_flex()
                        .gap_1()
                        .child(section_label(t!("Subagent.result"), cx))
                        .child(result),
                )
            })
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subagent_builder() {
        let subagent = Subagent::new("subagent", "Explorer")
            .task("Find where themes load")
            .status(TraceStatus::Completed)
            .result("crates/component/src/theme")
            .open(true)
            .on_toggle(|_, _, _| {})
            .child("tool call");

        assert_eq!(subagent.name, "Explorer");
        assert!(subagent.task.is_some());
        assert!(subagent.result.is_some());
        assert_eq!(subagent.status, TraceStatus::Completed);
        assert_eq!(subagent.children.len(), 1);
        assert!(subagent.open);
    }
}
