use gpui::{App, SharedString};

use super::SpeechLocale;
use crate::speech::{AudioFormat, RecognitionSession, SpeechError, SpeechRecognizer, SpeechSink};

/// Placeholder until the platform recognizer lands.
pub(super) struct PlatformRecognizer;

impl PlatformRecognizer {
    pub(super) fn new(_locale: Option<SharedString>, _expected_phrases: &[SharedString]) -> Self {
        Self
    }
}

pub(super) fn supported_locales() -> Vec<SpeechLocale> {
    Vec::new()
}

impl SpeechRecognizer for PlatformRecognizer {
    fn audio_format(&self) -> AudioFormat {
        AudioFormat::default()
    }

    fn is_available(&self, _: &App) -> bool {
        false
    }

    fn start(
        &self,
        _: SpeechSink,
        _: &mut App,
    ) -> Result<Box<dyn RecognitionSession>, SpeechError> {
        Err(SpeechError::Unsupported)
    }
}
