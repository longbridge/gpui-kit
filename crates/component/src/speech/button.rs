use gpui::{App, ElementId, Entity, IntoElement, RenderOnce, StyleRefinement, Styled, Window, div};
use rust_i18n::t;

use crate::{
    Disableable, IconName, Selectable as _, Sizable, Size, StyledExt as _,
    button::{Button, ButtonVariants as _},
};

use super::{SpeechState, SpeechStatus};

/// The button that starts and stops a [`SpeechState`]'s session.
///
/// Shows a microphone at rest and a stop glyph, pressed, while capturing; a
/// click toggles the session. While the final result is pending it shows a
/// spinner and ignores clicks.
///
/// Renders nothing when the state has no recognizer on this platform, unless
/// [`Self::show_when_unsupported`] is set, and renders disabled while the
/// recognizer reports itself unavailable.
#[derive(IntoElement)]
pub struct SpeechButton {
    id: ElementId,
    state: Entity<SpeechState>,
    size: Size,
    disabled: bool,
    show_when_unsupported: bool,
    style: StyleRefinement,
}

impl SpeechButton {
    /// A button for `state`.
    pub fn new(state: &Entity<SpeechState>) -> Self {
        Self {
            id: ("speech-button", state.entity_id()).into(),
            state: state.clone(),
            size: Size::default(),
            disabled: false,
            show_when_unsupported: false,
            style: StyleRefinement::default(),
        }
    }

    /// Render a disabled button, instead of nothing, when the state has no
    /// recognizer on this platform. Default `false`.
    pub fn show_when_unsupported(mut self, show: bool) -> Self {
        self.show_when_unsupported = show;
        self
    }
}

impl Sizable for SpeechButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for SpeechButton {
    /// Refines the button, e.g. its size or corner radius, to match the
    /// controls around it.
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for SpeechButton {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for SpeechButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let status = state.status();
        let supported = state.has_recognizer();
        if !supported && !self.show_when_unsupported {
            return div().into_any_element();
        }
        // A running session stays stoppable even if the recognizer turns
        // unavailable mid-way.
        let available = status.is_active() || state.is_available(cx);
        let capturing = status.is_capturing();
        let label = if !available {
            t!("Speech.Unavailable")
        } else if capturing {
            t!("Speech.Stop")
        } else {
            t!("Speech.Start")
        };

        Button::new(self.id)
            .ghost()
            .with_size(self.size)
            .icon(if capturing {
                IconName::Square
            } else {
                IconName::Mic
            })
            .selected(capturing)
            .loading(status == SpeechStatus::Stopping)
            .disabled(self.disabled || !available)
            .tooltip(label.clone())
            .accessibility_label(label)
            .refine_style(&self.style)
            .on_click({
                let state = self.state.clone();
                move |_, _, cx| state.update(cx, |state, cx| state.toggle(cx))
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;

    #[gpui::test]
    fn test_speech_button_builder(cx: &mut TestAppContext) {
        let state = cx.update(|cx| cx.new(|cx| SpeechState::new(cx).system_fallback(false)));
        let button = SpeechButton::new(&state)
            .small()
            .disabled(true)
            .show_when_unsupported(true);

        assert_eq!(button.size, Size::Small);
        assert!(button.disabled);
        assert!(button.show_when_unsupported);
        assert_eq!(
            button.id,
            ElementId::from(("speech-button", state.entity_id()))
        );
    }
}
