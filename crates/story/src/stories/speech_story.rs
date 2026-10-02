use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Subscription, Window, div, prelude::FluentBuilder as _, px,
};

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, WindowExt as _, h_flex,
    input::{Input, InputState},
    notification::Notification,
    speech::{
        RecognitionSession, SpeechButton, SpeechError, SpeechEvent, SpeechRecognizer, SpeechSink,
        SpeechState, SpeechWaveform,
    },
    v_flex,
};

use crate::section;

/// What [`DemoRecognizer`] "hears", one phrase at a time.
const SCRIPT: [&str; 2] = [
    "Speech input turns what you say into text.",
    "Any recognizer plugs in through one trait.",
];

/// How much 16 kHz mono audio [`DemoRecognizer`] takes per word: 0.3 s.
const SAMPLES_PER_WORD: usize = 4_800;

/// A recognizer that needs no service: it types [`SCRIPT`] one word per
/// [`SAMPLES_PER_WORD`] of audio, whatever the audio contains.
///
/// A real recognizer has the same shape: `start` opens a connection, the
/// session streams audio to it and reports the service's results to the sink.
struct DemoRecognizer;

impl SpeechRecognizer for DemoRecognizer {
    fn start(
        &self,
        sink: SpeechSink,
        cx: &mut App,
    ) -> Result<Box<dyn RecognitionSession>, SpeechError> {
        // Nothing to connect to, so audio is consumed at once.
        sink.ready(cx);
        Ok(Box::new(DemoSession {
            sink,
            phrase_ix: 0,
            words: 0,
            samples: 0,
        }))
    }
}

struct DemoSession {
    sink: SpeechSink,
    phrase_ix: usize,
    /// Words of the current phrase recognized so far.
    words: usize,
    /// Samples received since the last word.
    samples: usize,
}

impl DemoSession {
    /// The recognized part of the current phrase, if any.
    fn spoken(&self) -> Option<String> {
        let phrase = SCRIPT.get(self.phrase_ix)?;
        let words = phrase.split(' ').take(self.words).collect::<Vec<_>>();
        if words.is_empty() {
            return None;
        }
        // Phrases are joined verbatim, so separate sentences here.
        let separator = if self.phrase_ix > 0 { " " } else { "" };
        Some(format!("{separator}{}", words.join(" ")))
    }

    fn next_word(&mut self, cx: &mut App) {
        let Some(phrase) = SCRIPT.get(self.phrase_ix) else {
            return;
        };
        self.words += 1;
        if self.words < phrase.split(' ').count() {
            if let Some(spoken) = self.spoken() {
                self.sink.hypothesis(spoken, cx);
            }
        } else {
            self.commit(cx);
        }
    }

    fn commit(&mut self, cx: &mut App) {
        if let Some(spoken) = self.spoken() {
            self.sink.phrase(spoken, cx);
        }
        self.phrase_ix += 1;
        self.words = 0;
    }
}

impl RecognitionSession for DemoSession {
    fn push_audio(&mut self, samples: &[i16], cx: &mut App) {
        self.samples += samples.len();
        while self.samples >= SAMPLES_PER_WORD {
            self.samples -= SAMPLES_PER_WORD;
            self.next_word(cx);
        }
    }

    fn finish(&mut self, cx: &mut App) {
        self.commit(cx);
        self.sink.finish(cx);
    }
}

/// Stands in for the microphone on the web, which the story cannot capture:
/// pushes a tone whose loudness rises and falls like speech.
#[cfg(target_family = "wasm")]
struct GeneratedInput;

#[cfg(target_family = "wasm")]
impl gpui_kit::component::speech::AudioInput for GeneratedInput {
    fn start(
        &self,
        format: gpui_kit::component::speech::AudioFormat,
        sink: gpui_kit::component::speech::AudioSink,
        cx: &mut App,
    ) -> Result<Subscription, SpeechError> {
        const CHUNK: std::time::Duration = std::time::Duration::from_millis(100);
        let len = (format.sample_rate() / 10) as usize * format.channels() as usize;
        let task = cx.spawn(async move |cx| {
            let mut tick = 0u8;
            loop {
                cx.background_executor().timer(CHUNK).await;
                tick = tick.wrapping_add(1);
                let loudness = 0.05 + 0.25 * (tick as f32 * 0.9).sin().abs();
                let samples = (0..len)
                    .map(|ix| ((ix as f32 * 0.07).sin() * loudness * i16::MAX as f32) as i16)
                    .collect();
                cx.update(|cx| sink.push(samples, cx));
            }
        });
        Ok(Subscription::new(move || drop(task)))
    }
}

/// A text field the user can dictate into.
struct Dictation {
    speech: Entity<SpeechState>,
    input: Entity<InputState>,
}

impl Dictation {
    fn new(
        speech: Entity<SpeechState>,
        window: &mut Window,
        cx: &mut Context<SpeechStory>,
    ) -> (Self, Subscription) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Type or dictate"));
        let subscription = cx.subscribe_in(&speech, window, {
            let input = input.clone();
            move |_, _, event, window, cx| match event {
                SpeechEvent::Final(text) if !text.is_empty() => {
                    input.update(cx, |input, cx| input.insert(text.clone(), window, cx));
                }
                SpeechEvent::Error(error) => {
                    window.push_notification(Notification::error(error.to_string()), cx);
                }
                _ => {}
            }
        });
        (Self { speech, input }, subscription)
    }

    fn render(&self, show_when_unsupported: bool, cx: &App) -> impl IntoElement {
        let speech = self.speech.read(cx);
        let status = speech.status();

        v_flex()
            .w_full()
            .gap_2()
            .child(
                Input::new(&self.input).suffix(
                    h_flex()
                        .gap_2()
                        .when(status.is_capturing(), |this| {
                            this.child(SpeechWaveform::new(&self.speech).w(px(48.)).xsmall())
                        })
                        .child(
                            SpeechButton::new(&self.speech)
                                .xsmall()
                                .show_when_unsupported(show_when_unsupported),
                        ),
                ),
            )
            .when(status.is_active(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(speech.transcript()),
                )
            })
    }
}

pub struct SpeechStory {
    focus_handle: FocusHandle,
    custom: Dictation,
    system: Dictation,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for SpeechStory {
    fn title() -> &'static str {
        "Speech"
    }

    fn description() -> &'static str {
        "Dictate text through the system recognizer or your own."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SpeechStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let custom = cx.new(|cx| {
            let state = SpeechState::new(cx).recognizer(DemoRecognizer);
            #[cfg(target_family = "wasm")]
            let state = state.input(GeneratedInput);
            state
        });
        let system = cx.new(SpeechState::new);

        let (custom, custom_subscription) = Dictation::new(custom, window, cx);
        let (system, system_subscription) = Dictation::new(system, window, cx);

        Self {
            focus_handle: cx.focus_handle(),
            custom,
            system,
            _subscriptions: vec![custom_subscription, system_subscription],
        }
    }
}

impl Focusable for SpeechStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SpeechStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .justify_start()
            .gap_3()
            .child(
                section("With a custom recognizer")
                    .description(
                        "A recognizer defined in this story types a scripted sentence \
                        while it receives audio.",
                    )
                    .w_128()
                    .child(self.custom.render(false, cx)),
            )
            .child(
                section("System recognizer")
                    .description(
                        "Recognizes speech on macOS and Windows. Elsewhere, and in an app \
                        without the required usage descriptions, the button is disabled.",
                    )
                    .w_128()
                    .child(self.system.render(true, cx)),
            )
    }
}
