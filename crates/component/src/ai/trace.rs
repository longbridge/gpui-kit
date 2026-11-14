use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Hsla, IntoElement, ParentElement, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

use rust_i18n::t;

use crate::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    collapsible::Collapsible,
    h_flex,
    spinner::Spinner,
    v_flex,
};

/// A controlled disclosure callback, called with the requested open state.
pub(crate) type ToggleHandler = Rc<dyn Fn(&bool, &mut Window, &mut App)>;

/// The progress of one unit of agent work: a tool call, a plan step, a task,
/// a command or a subagent run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TraceStatus {
    /// Queued but not started.
    #[default]
    Pending,
    /// Started and still producing output.
    Running,
    /// Finished without an error.
    Completed,
    /// Finished with an error.
    Failed,
    /// Not run, for example because the user denied it.
    Skipped,
}

impl TraceStatus {
    /// Whether the work has reached a final state.
    pub fn is_finished(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Skipped)
    }

    /// The localized, short state label, such as `Running`.
    pub fn label(self) -> SharedString {
        match self {
            Self::Pending => t!("Trace.pending"),
            Self::Running => t!("Trace.running"),
            Self::Completed => t!("Trace.completed"),
            Self::Failed => t!("Trace.failed"),
            Self::Skipped => t!("Trace.skipped"),
        }
        .into()
    }

    /// The status glyph: a spinner while running, otherwise an icon whose
    /// shape alone tells the states apart so color is never the only cue.
    pub(crate) fn indicator(self, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let (icon, color): (IconName, Hsla) = match self {
            Self::Running => {
                return Spinner::new()
                    .icon(IconName::LoaderCircle)
                    .xsmall()
                    .color(theme.muted_foreground)
                    .into_any_element();
            }
            Self::Pending => (IconName::Circle, theme.muted_foreground),
            Self::Completed => (IconName::CircleCheck, theme.success),
            Self::Failed => (IconName::CircleX, theme.danger),
            Self::Skipped => (IconName::CircleSlash, theme.muted_foreground),
        };
        Icon::new(icon)
            .xsmall()
            .text_color(color)
            .into_any_element()
    }
}

/// A disclosure row that records one step of agent work.
///
/// A trace shows an icon, a title, optional trailing metadata and a status;
/// activating the header reveals its content. `Reasoning`, `Tool`, `Plan` and
/// the other agent elements are compositions of this row, so they share one
/// geometry and one keyboard path.
///
/// The open state is controlled: pass the current value with [`Self::open`]
/// and update it from [`Self::on_toggle`].
#[derive(IntoElement)]
pub struct Trace {
    id: ElementId,
    style: StyleRefinement,
    icon: Option<Icon>,
    title: Option<AnyElement>,
    description: Option<AnyElement>,
    status: Option<TraceStatus>,
    open: bool,
    on_toggle: Option<ToggleHandler>,
    content: Vec<AnyElement>,
}

impl Trace {
    /// Create a closed trace with a stable identity.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            icon: None,
            title: None,
            description: None,
            status: None,
            open: false,
            on_toggle: None,
            content: Vec::new(),
        }
    }

    /// Set the leading icon that identifies the kind of work.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the title, such as a tool name or a step.
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = Some(title.into_any_element());
        self
    }

    /// Set muted metadata shown after the title, such as a duration.
    pub fn description(mut self, description: impl IntoElement) -> Self {
        self.description = Some(description.into_any_element());
        self
    }

    /// Set the status shown at the trailing edge.
    pub fn status(mut self, status: TraceStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set whether the content is revealed.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Called with the requested open state when the header is activated.
    pub fn on_toggle(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    pub(crate) fn with_toggle_handler(mut self, handler: Option<ToggleHandler>) -> Self {
        self.on_toggle = handler;
        self
    }

    /// Add content revealed while the trace is open.
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content.push(content.into_any_element());
        self
    }

    /// Whether the trace has content to disclose.
    pub fn has_content(&self) -> bool {
        !self.content.is_empty()
    }
}

impl Styled for Trace {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Trace {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted_foreground = cx.theme().muted_foreground;
        let open = self.open;
        let has_content = self.has_content();
        let on_toggle = self.on_toggle;

        let header = Button::new(self.id.clone())
            .ghost()
            .small()
            .w_full()
            .content_style(
                StyleRefinement::default().justify_start(),
                crate::Size::Small,
            )
            .when_some(self.icon, |this, icon| {
                this.child(icon.xsmall().text_color(muted_foreground))
            })
            .when_some(self.title, |this, title| {
                this.child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(title),
                )
            })
            .when_some(self.description, |this, description| {
                this.child(
                    div()
                        .flex_none()
                        .text_color(muted_foreground)
                        .child(description),
                )
            })
            .child(div().flex_1())
            .when_some(self.status, |this, status| {
                this.child(
                    h_flex()
                        .flex_none()
                        .gap_1()
                        .text_xs()
                        .text_color(muted_foreground)
                        .when(status != TraceStatus::Completed, |this| {
                            this.child(status.label())
                        })
                        .child(status.indicator(cx)),
                )
            })
            .when(has_content, |this| {
                this.child(
                    Icon::new(if open {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    })
                    .xsmall()
                    .text_color(muted_foreground),
                )
            })
            .when_some(on_toggle.filter(|_| has_content), |this, on_toggle| {
                this.on_click(move |_, window, cx| on_toggle(&!open, window, cx))
            });

        Collapsible::new()
            .motion_id((self.id, "trace"))
            .open(open)
            .w_full()
            .min_w_0()
            .refine_style(&self.style)
            .child(header)
            .content(
                v_flex()
                    .w_full()
                    .min_w_0()
                    .gap_2()
                    .pt_1()
                    .pb_2()
                    .pl_6()
                    .pr_2()
                    .text_sm()
                    .children(self.content),
            )
    }
}

/// The monospaced, bordered surface used inside agent traces for
/// structured input, output and logs.
pub(crate) fn code_surface(cx: &App) -> gpui::Div {
    v_flex()
        .w_full()
        .min_w_0()
        .p_2()
        .gap_1()
        .rounded(cx.theme().radius)
        .bg(cx.theme().muted)
        .font_family(cx.theme().mono_font_family.clone())
        .text_xs()
}

/// A muted label naming a section inside a trace, such as `Input`.
pub(crate) fn section_label(label: impl Into<SharedString>, cx: &App) -> impl IntoElement {
    div()
        .text_xs()
        .font_medium()
        .text_color(cx.theme().muted_foreground)
        .child(label.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_builder() {
        let trace = Trace::new("trace")
            .icon(IconName::Bot)
            .title("Search")
            .description("2 s")
            .status(TraceStatus::Running)
            .open(true)
            .on_toggle(|_, _, _| {})
            .content("Body");

        assert!(trace.open);
        assert!(trace.has_content());
        assert!(trace.icon.is_some());
        assert!(trace.title.is_some());
        assert!(trace.description.is_some());
        assert_eq!(trace.status, Some(TraceStatus::Running));
        assert!(trace.on_toggle.is_some());
        assert!(!Trace::new("empty").has_content());
    }

    #[test]
    fn test_trace_status_is_finished() {
        assert!(!TraceStatus::Pending.is_finished());
        assert!(!TraceStatus::Running.is_finished());
        assert!(TraceStatus::Completed.is_finished());
        assert!(TraceStatus::Failed.is_finished());
        assert!(TraceStatus::Skipped.is_finished());
    }
}
