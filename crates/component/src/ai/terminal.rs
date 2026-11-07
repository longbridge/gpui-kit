use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, IconName, StyledExt as _,
    ai::trace::{ToggleHandler, Trace, TraceStatus, code_surface},
    clipboard::Clipboard,
    h_flex,
};

const DEFAULT_MAX_LINES: usize = 30;

/// Split `output` into the number of hidden leading lines and the visible tail.
fn tail(output: &str, max_lines: usize) -> (usize, &str) {
    let output = output.strip_suffix('\n').unwrap_or(output);
    let total = output.lines().count();
    if total <= max_lines {
        return (0, output);
    }
    let hidden = total - max_lines;
    let start = output
        .match_indices('\n')
        .nth(hidden - 1)
        .map_or(0, |(ix, _)| ix + 1);
    (hidden, &output[start..])
}

/// A command the agent runs, with its output as it streams.
///
/// Pass the accumulated output on every render while the command runs. The
/// status follows the exit code once it is known: zero completes, anything
/// else fails.
#[derive(IntoElement)]
pub struct Terminal {
    id: ElementId,
    style: StyleRefinement,
    command: SharedString,
    output: SharedString,
    status: TraceStatus,
    exit_code: Option<i32>,
    max_lines: usize,
    open: bool,
    on_toggle: Option<ToggleHandler>,
}

impl Terminal {
    /// Create a running command.
    pub fn new(id: impl Into<ElementId>, command: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            command: command.into(),
            output: SharedString::default(),
            status: TraceStatus::Running,
            exit_code: None,
            max_lines: DEFAULT_MAX_LINES,
            open: false,
            on_toggle: None,
        }
    }

    /// Set the output received so far.
    pub fn output(mut self, output: impl Into<SharedString>) -> Self {
        self.output = output.into();
        self
    }

    /// Set the progress of the command.
    pub fn status(mut self, status: TraceStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the exit code; it also finishes the command.
    pub fn exit_code(mut self, exit_code: i32) -> Self {
        self.exit_code = Some(exit_code);
        self.status = if exit_code == 0 {
            TraceStatus::Completed
        } else {
            TraceStatus::Failed
        };
        self
    }

    /// Set how many trailing output lines stay visible; earlier lines are
    /// summarized, so the newest output is always in view. Defaults to 30.
    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.max_lines = max_lines.max(1);
        self
    }

    /// Set whether the output is revealed.
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

impl Styled for Terminal {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Terminal {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let mono = theme.mono_font_family.clone();
        let muted_foreground = theme.muted_foreground;
        let id = self.id;

        Trace::new(id.clone())
            .icon(IconName::SquareTerminal)
            .title(div().font_family(mono).child(self.command))
            .when_some(self.exit_code.filter(|code| *code != 0), |this, code| {
                this.description(SharedString::from(t!("Terminal.exit_code", code = code)))
            })
            .status(self.status)
            .open(self.open)
            .with_toggle_handler(self.on_toggle)
            .content(
                code_surface(cx)
                    .relative()
                    .child(
                        h_flex().absolute().top_1().right_1().child(
                            Clipboard::new((id, "copy"))
                                .accessibility_label(t!("Terminal.copy_output"))
                                .value(self.output.clone()),
                        ),
                    )
                    .map(|this| {
                        if self.output.is_empty() {
                            this.text_color(muted_foreground)
                                .child(SharedString::from(t!("Terminal.no_output")))
                        } else {
                            let (hidden, visible) = tail(&self.output, self.max_lines);
                            this.pr_8()
                                .when(hidden > 0, |this| {
                                    this.child(div().text_color(muted_foreground).child(
                                        SharedString::from(t!(
                                            "Terminal.earlier_lines",
                                            count = hidden
                                        )),
                                    ))
                                })
                                .child(SharedString::from(visible.to_string()))
                        }
                    }),
            )
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_builder() {
        let terminal = Terminal::new("terminal", "cargo test")
            .output("running 3 tests")
            .open(true)
            .on_toggle(|_, _, _| {});

        assert_eq!(terminal.command, "cargo test");
        assert_eq!(terminal.output, "running 3 tests");
        assert_eq!(terminal.status, TraceStatus::Running);
        assert!(terminal.open);
        assert!(terminal.on_toggle.is_some());
    }

    #[test]
    fn test_terminal_tail_keeps_newest_lines() {
        assert_eq!(tail("a\nb\nc\n", 5), (0, "a\nb\nc"));
        assert_eq!(tail("a\nb\nc\nd", 2), (2, "c\nd"));
        assert_eq!(tail("a\nb\nc\nd\n", 1), (3, "d"));
        assert_eq!(tail("", 3), (0, ""));
    }

    #[test]
    fn test_terminal_exit_code_sets_status() {
        assert_eq!(
            Terminal::new("a", "true").exit_code(0).status,
            TraceStatus::Completed
        );
        assert_eq!(
            Terminal::new("b", "false").exit_code(1).status,
            TraceStatus::Failed
        );
    }
}
