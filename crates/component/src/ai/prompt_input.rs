use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Disableable as _, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Textarea, TextareaState},
    v_flex,
};

/// Where the conversation stands, which decides the submit button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PromptStatus {
    /// Waiting for the user's next message.
    #[default]
    Ready,
    /// The message was sent and no reply has arrived yet.
    Submitted,
    /// The reply is streaming; the button stops it.
    Streaming,
    /// The last request failed; sending again retries.
    Error,
}

type SubmitHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// The message field with attachments, tools and a send button.
///
/// The application owns the [`TextareaState`]. Create it with
/// `.auto_grow(1, 8).submit_on_enter(true)` and subscribe to
/// `InputEvent::PressEnter` to send from the keyboard, calling the same
/// method as [`Self::on_submit`]; a `RenderOnce` element cannot hold that
/// subscription itself.
#[derive(IntoElement)]
pub struct PromptInput {
    id: ElementId,
    state: Entity<TextareaState>,
    style: StyleRefinement,
    status: PromptStatus,
    attachments: Vec<AnyElement>,
    tools: Vec<AnyElement>,
    on_submit: Option<SubmitHandler>,
    on_stop: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl PromptInput {
    /// Create a composer for the application-owned input state.
    pub fn new(id: impl Into<ElementId>, state: &Entity<TextareaState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            style: StyleRefinement::default(),
            status: PromptStatus::Ready,
            attachments: Vec::new(),
            tools: Vec::new(),
            on_submit: None,
            on_stop: None,
        }
    }

    /// Set where the conversation stands.
    pub fn status(mut self, status: PromptStatus) -> Self {
        self.status = status;
        self
    }

    /// Add an attachment shown above the field, usually an
    /// [`Attachment`](crate::attachment::Attachment).
    pub fn attachment(mut self, attachment: impl IntoElement) -> Self {
        self.attachments.push(attachment.into_any_element());
        self
    }

    /// Add a control to the leading edge of the toolbar, such as an attach
    /// button or a [`ModelSelector`](super::ModelSelector).
    pub fn tool(mut self, tool: impl IntoElement) -> Self {
        self.tools.push(tool.into_any_element());
        self
    }

    /// Called with the current text when the user sends. Empty text is not
    /// sent.
    pub fn on_submit(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_submit = Some(Rc::new(handler));
        self
    }

    /// Called when the user stops a streaming reply.
    pub fn on_stop(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_stop = Some(Rc::new(handler));
        self
    }
}

impl Styled for PromptInput {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for PromptInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let value = self.state.read(cx).value();
        let is_empty = value.trim().is_empty();
        let status = self.status;
        let state = self.state.clone();

        let submit = match status {
            PromptStatus::Streaming => Button::new((self.id.clone(), "stop"))
                .primary()
                .small()
                .icon(IconName::Square)
                .accessibility_label(SharedString::from(t!("PromptInput.stop")))
                .when_some(self.on_stop, |this, on_stop| {
                    this.on_click(move |_, window, cx| on_stop(window, cx))
                }),
            _ => Button::new((self.id.clone(), "send"))
                .primary()
                .small()
                .icon(IconName::ArrowUp)
                .loading(status == PromptStatus::Submitted)
                .disabled(is_empty || status == PromptStatus::Submitted)
                .accessibility_label(SharedString::from(t!("PromptInput.send")))
                .when_some(self.on_submit, |this, on_submit| {
                    this.on_click(move |_, window, cx| {
                        let value = state.read(cx).value();
                        if !value.trim().is_empty() {
                            on_submit(&value, window, cx);
                        }
                    })
                }),
        }
        .rounded(theme.radius_full());

        v_flex()
            .id(self.id)
            .w_full()
            .gap_1()
            .p_2()
            .rounded(theme.radius_2xl())
            .border_1()
            .border_color(theme.input)
            .bg(theme.background)
            .when(!self.attachments.is_empty(), |this| {
                this.child(
                    h_flex()
                        .flex_wrap()
                        .gap_2()
                        .px_1()
                        .children(self.attachments),
                )
            })
            .child(Textarea::new(&self.state).appearance(false))
            .child(
                h_flex()
                    .w_full()
                    .gap_1()
                    .children(self.tools)
                    .child(gpui::div().flex_1())
                    .child(submit),
            )
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, TestAppContext};

    #[gpui::test]
    fn test_prompt_input_builder(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|window, cx| {
            crate::init(cx);
            let state = cx.new(|cx| TextareaState::new(window, cx));
            let prompt = PromptInput::new("prompt", &state)
                .status(PromptStatus::Streaming)
                .attachment("file.png")
                .tool("model")
                .on_submit(|_, _, _| {})
                .on_stop(|_, _| {});

            assert_eq!(prompt.status, PromptStatus::Streaming);
            assert_eq!(prompt.attachments.len(), 1);
            assert_eq!(prompt.tools.len(), 1);
            assert!(prompt.on_submit.is_some());
            assert!(prompt.on_stop.is_some());
            gpui::Empty
        });
        cx.run_until_parked();
    }
}
