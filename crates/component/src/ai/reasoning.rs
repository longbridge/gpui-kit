use std::{rc::Rc, time::Duration};

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled,
    Window, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, IconName, StyledExt as _,
    ai::trace::{ToggleHandler, Trace},
    shimmer::ShimmerText,
};

/// The model's thinking, shown while it streams and kept behind a summary
/// once the answer arrives.
///
/// While [`Self::streaming`] is true the title shimmers; afterwards it reports
/// how long the model thought. The open state is controlled: applications
/// usually open it while streaming and close it when the answer starts.
#[derive(IntoElement)]
pub struct Reasoning {
    id: ElementId,
    style: StyleRefinement,
    streaming: bool,
    duration: Option<Duration>,
    open: bool,
    on_toggle: Option<ToggleHandler>,
    children: Vec<AnyElement>,
}

impl Reasoning {
    /// Create closed, finished reasoning with a stable identity.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            streaming: false,
            duration: None,
            open: false,
            on_toggle: None,
            children: Vec::new(),
        }
    }

    /// Set whether reasoning text is still arriving.
    pub fn streaming(mut self, streaming: bool) -> Self {
        self.streaming = streaming;
        self
    }

    /// Set how long the model thought, shown once streaming ends.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Set whether the reasoning text is revealed.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Called with the requested open state when the summary is activated.
    pub fn on_toggle(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    fn summary(&self) -> String {
        if self.streaming {
            return t!("Reasoning.thinking").into();
        }
        match self.duration.map(|duration| duration.as_secs()) {
            Some(0) => t!("Reasoning.thought_briefly").into(),
            Some(seconds) => t!("Reasoning.thought_for", seconds = seconds).into(),
            None => t!("Reasoning.reasoning").into(),
        }
    }
}

impl ParentElement for Reasoning {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Reasoning {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Reasoning {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let summary = self.summary();
        let muted_foreground = cx.theme().muted_foreground;

        Trace::new(self.id.clone())
            .icon(IconName::Brain)
            .map(|this| {
                if self.streaming {
                    this.title(
                        ShimmerText::new(summary)
                            .id((self.id, "summary"))
                            .text_color(muted_foreground),
                    )
                } else {
                    this.title(summary)
                }
            })
            .open(self.open)
            .with_toggle_handler(self.on_toggle)
            .content(
                gpui::div()
                    .w_full()
                    .text_color(muted_foreground)
                    .children(self.children),
            )
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoning_builder() {
        let reasoning = Reasoning::new("reasoning")
            .streaming(true)
            .duration(Duration::from_secs(4))
            .open(true)
            .on_toggle(|_, _, _| {})
            .child("Considering the request");

        assert!(reasoning.streaming);
        assert!(reasoning.open);
        assert_eq!(reasoning.duration, Some(Duration::from_secs(4)));
        assert!(reasoning.on_toggle.is_some());
        assert_eq!(reasoning.children.len(), 1);
    }

    #[test]
    fn test_reasoning_summary_follows_streaming_and_duration() {
        assert_eq!(Reasoning::new("a").streaming(true).summary(), "Thinking…");
        assert_eq!(
            Reasoning::new("b")
                .duration(Duration::from_secs(12))
                .summary(),
            "Thought for 12 seconds"
        );
        assert_eq!(
            Reasoning::new("c")
                .duration(Duration::from_millis(400))
                .summary(),
            "Thought for a moment"
        );
        assert_eq!(Reasoning::new("d").summary(), "Reasoning");
    }
}
