use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, IconName, StyledExt as _,
    ai::trace::{ToggleHandler, Trace, TraceStatus, code_surface, section_label},
};

/// A tool call from its input to its output.
///
/// The header names the tool and reports its [`TraceStatus`]; the content
/// shows the arguments the model sent and the result, or the error, that came
/// back. Pass text exactly as exchanged with the model — usually JSON — and
/// it is shown monospaced and wrapped.
#[derive(IntoElement)]
pub struct Tool {
    id: ElementId,
    style: StyleRefinement,
    name: SharedString,
    status: TraceStatus,
    input: Option<SharedString>,
    output: Option<SharedString>,
    error: Option<SharedString>,
    open: bool,
    on_toggle: Option<ToggleHandler>,
}

impl Tool {
    /// Create a pending tool call named after the tool.
    pub fn new(id: impl Into<ElementId>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            name: name.into(),
            status: TraceStatus::Pending,
            input: None,
            output: None,
            error: None,
            open: false,
            on_toggle: None,
        }
    }

    /// Set the progress of the call.
    pub fn status(mut self, status: TraceStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the arguments the model passed to the tool.
    pub fn input(mut self, input: impl Into<SharedString>) -> Self {
        self.input = Some(input.into());
        self
    }

    /// Set the result the tool returned.
    pub fn output(mut self, output: impl Into<SharedString>) -> Self {
        self.output = Some(output.into());
        self
    }

    /// Set the error the tool returned. An error marks the call as failed.
    pub fn error(mut self, error: impl Into<SharedString>) -> Self {
        self.error = Some(error.into());
        self.status = TraceStatus::Failed;
        self
    }

    /// Set whether input and output are revealed.
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

impl Styled for Tool {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Tool {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let danger = cx.theme().danger;

        Trace::new(self.id)
            .icon(IconName::Wrench)
            .title(self.name)
            .status(self.status)
            .open(self.open)
            .with_toggle_handler(self.on_toggle)
            .when_some(self.input, |this, input| {
                this.content(
                    div()
                        .v_flex()
                        .gap_1()
                        .child(section_label(t!("Tool.input"), cx))
                        .child(code_surface(cx).child(input)),
                )
            })
            .when_some(self.output, |this, output| {
                this.content(
                    div()
                        .v_flex()
                        .gap_1()
                        .child(section_label(t!("Tool.output"), cx))
                        .child(code_surface(cx).child(output)),
                )
            })
            .when_some(self.error, |this, error| {
                this.content(
                    div()
                        .v_flex()
                        .gap_1()
                        .child(section_label(t!("Tool.error"), cx))
                        .child(code_surface(cx).text_color(danger).child(error)),
                )
            })
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_builder() {
        let tool = Tool::new("tool", "web_search")
            .status(TraceStatus::Completed)
            .input(r#"{"query":"gpui"}"#)
            .output("3 results")
            .open(true)
            .on_toggle(|_, _, _| {});

        assert_eq!(tool.name, "web_search");
        assert_eq!(tool.status, TraceStatus::Completed);
        assert!(tool.input.is_some());
        assert!(tool.output.is_some());
        assert!(tool.open);
        assert!(tool.on_toggle.is_some());
    }

    #[test]
    fn test_tool_error_marks_failed() {
        let tool = Tool::new("tool", "read_file")
            .status(TraceStatus::Running)
            .error("Not found");
        assert_eq!(tool.status, TraceStatus::Failed);
        assert_eq!(tool.error.as_deref(), Some("Not found"));
    }
}
