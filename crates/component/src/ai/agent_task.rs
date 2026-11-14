use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString, StyleRefinement,
    Styled, Window, prelude::FluentBuilder as _,
};

use crate::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    ai::trace::{ToggleHandler, Trace, TraceStatus},
    h_flex, v_flex,
};

/// One step of an [`AgentTask`], optionally naming the file it touched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentTaskItem {
    text: SharedString,
    file: Option<SharedString>,
}

impl AgentTaskItem {
    /// Create a step described by `text`.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            file: None,
        }
    }

    /// Set the file the step read or changed.
    pub fn file(mut self, file: impl Into<SharedString>) -> Self {
        self.file = Some(file.into());
        self
    }
}

/// The steps the agent takes for one task, listed as it takes them.
///
/// Named `AgentTask` so it does not shadow GPUI's async `Task`.
#[derive(IntoElement)]
pub struct AgentTask {
    id: ElementId,
    style: StyleRefinement,
    title: SharedString,
    status: TraceStatus,
    items: Vec<AgentTaskItem>,
    open: bool,
    on_toggle: Option<ToggleHandler>,
}

impl AgentTask {
    /// Create a running task with a title such as `Searching the codebase`.
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            title: title.into(),
            status: TraceStatus::Running,
            items: Vec::new(),
            open: false,
            on_toggle: None,
        }
    }

    /// Set the progress of the task.
    pub fn status(mut self, status: TraceStatus) -> Self {
        self.status = status;
        self
    }

    /// Add a step.
    pub fn item(mut self, item: AgentTaskItem) -> Self {
        self.items.push(item);
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
}

impl Styled for AgentTask {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AgentTask {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let foreground = theme.foreground;
        let muted_foreground = theme.muted_foreground;
        let border = theme.border;
        let muted = theme.muted;
        let radius = theme.radius;
        let mono = theme.mono_font_family.clone();

        Trace::new(self.id)
            .icon(IconName::ListTodo)
            .title(self.title)
            .status(self.status)
            .open(self.open)
            .with_toggle_handler(self.on_toggle)
            .when(!self.items.is_empty(), |this| {
                this.content(
                    v_flex()
                        .gap_1p5()
                        .pl_3()
                        .border_l_1()
                        .border_color(border)
                        .children(self.items.into_iter().map(|item| {
                            h_flex()
                                .flex_wrap()
                                .gap_1p5()
                                .text_color(muted_foreground)
                                .child(item.text)
                                .when_some(item.file, |this, file| {
                                    this.child(
                                        h_flex()
                                            .gap_1()
                                            .px_1p5()
                                            .rounded(radius)
                                            .bg(muted)
                                            .text_xs()
                                            .font_family(mono.clone())
                                            .text_color(foreground)
                                            .child(Icon::new(IconName::File).xsmall())
                                            .child(file),
                                    )
                                })
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
    fn test_agent_task_builder() {
        let task = AgentTask::new("task", "Searching the codebase")
            .status(TraceStatus::Completed)
            .item(AgentTaskItem::new("Read").file("src/lib.rs"))
            .item(AgentTaskItem::new("Found 3 matches"))
            .open(true)
            .on_toggle(|_, _, _| {});

        assert_eq!(task.status, TraceStatus::Completed);
        assert_eq!(task.items.len(), 2);
        assert_eq!(task.items[0].file.as_deref(), Some("src/lib.rs"));
        assert!(task.open);
        assert!(task.on_toggle.is_some());
    }
}
