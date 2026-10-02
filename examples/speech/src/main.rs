//! Speech: a notepad you can talk into, built on GPUI Component's speech
//! input. It doubles as a test bench for the platform recognizers: the sidebar
//! reports what this machine supports and the session log records every event.
//!
//! `cargo run -p speech` opens the app; `cargo run -p speech -- --check`
//! prints the same checks to the terminal and exits.

mod demo;

use std::{
    borrow::Cow,
    time::{Duration, Instant},
};

use cpal::traits::{DeviceTrait as _, HostTrait as _};
use gpui_kit::assets::{Assets, IconName as ExtraIcon, icon_assets};
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IndexPath, Selectable as _, Sizable as _,
    StyledExt as _, TitleBar, WindowExt as _,
    button::{Button, ButtonGroup, ButtonVariants as _},
    h_flex,
    input::{Textarea, TextareaState},
    kbd::Kbd,
    notification::Notification,
    scroll::ScrollableElement as _,
    select::{SearchableVec, Select, SelectEvent, SelectItem, SelectState},
    speech::{
        SpeechButton, SpeechEvent, SpeechState, SpeechStatus, SpeechWaveform, SystemRecognizer,
    },
    tag::Tag,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use demo::DemoRecognizer;

icon_assets!(ExtraIcons, [AudioLines, ScrollText, Eraser]);

/// The default component icons plus the few extras this app uses.
struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = ExtraIcons.load(path)? {
            return Ok(Some(bytes));
        }
        Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = Assets.list(path)?;
        paths.extend(ExtraIcons.list(path)?);
        Ok(paths)
    }
}

actions!(speech, [ToggleSpeech]);

const TOGGLE_KEYS: &str = "secondary-shift-d";
/// Most log entries kept; older ones scroll away.
const LOG_LIMIT: usize = 200;

/// Where the text comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Engine {
    /// The operating system's recognizer.
    System,
    /// [`DemoRecognizer`]: a scripted passage, no service needed.
    Demo,
}

#[derive(Clone)]
struct Language {
    tag: SharedString,
    name: SharedString,
}

impl SelectItem for Language {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.tag
    }
}

fn languages() -> Vec<Language> {
    [
        ("en-US", "English (US)"),
        ("en-GB", "English (UK)"),
        ("zh-CN", "简体中文"),
        ("zh-HK", "中文（香港）"),
        ("ja-JP", "日本語"),
    ]
    .into_iter()
    .map(|(tag, name)| Language {
        tag: tag.into(),
        name: name.into(),
    })
    .collect()
}

/// One line of the session log.
struct LogEntry {
    at: Duration,
    tone: Tone,
    label: &'static str,
    text: SharedString,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    Neutral,
    Progress,
    Success,
    Danger,
}

fn new_speech(engine: Engine, language: &str, cx: &mut App) -> Entity<SpeechState> {
    let language = language.to_string();
    cx.new(|cx| {
        let state = SpeechState::new(cx);
        match engine {
            Engine::System => state.recognizer(SystemRecognizer::new().locale(language)),
            Engine::Demo => state.recognizer(DemoRecognizer::new(&language)),
        }
    })
}

/// The name of the default audio input device, if there is one.
fn input_device_name() -> Option<SharedString> {
    let device = cpal::default_host().default_input_device()?;
    Some(
        device
            .name()
            .unwrap_or_else(|_| "Unnamed device".into())
            .into(),
    )
}

fn platform_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "linux") {
        "Linux"
    } else {
        "Other"
    }
}

struct SpeechApp {
    focus_handle: FocusHandle,
    engine: Engine,
    language: SharedString,
    speech: Entity<SpeechState>,
    notes: Entity<TextareaState>,
    language_select: Entity<SelectState<SearchableVec<Language>>>,
    input_device: Option<SharedString>,
    /// When the current or last session started.
    session_started: Option<Instant>,
    /// When the app opened; log times count from here.
    opened: Instant,
    log: Vec<LogEntry>,
    _speech_subscription: Subscription,
    _subscriptions: Vec<Subscription>,
}

impl SpeechApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let engine = Engine::System;
        let language = SharedString::from("en-US");
        let speech = new_speech(engine, &language, cx);
        let _speech_subscription = Self::subscribe_speech(&speech, window, cx);

        let notes = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("Start typing, or dictate with the microphone below.")
        });
        let language_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(languages()),
                Some(IndexPath::default()),
                window,
                cx,
            )
        });
        let _subscriptions = vec![cx.subscribe_in(
            &language_select,
            window,
            |this, _, event: &SelectEvent<SearchableVec<Language>>, window, cx| {
                if let SelectEvent::Confirm(Some(tag)) = event {
                    this.language = tag.clone();
                    this.rebuild_speech(window, cx);
                }
            },
        )];
        notes.update(cx, |notes, cx| notes.focus(window, cx));

        Self {
            focus_handle: cx.focus_handle(),
            engine,
            language,
            speech,
            notes,
            language_select,
            input_device: input_device_name(),
            session_started: None,
            opened: Instant::now(),
            log: Vec::new(),
            _speech_subscription,
            _subscriptions,
        }
    }

    fn subscribe_speech(
        speech: &Entity<SpeechState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Subscription {
        cx.subscribe_in(
            speech,
            window,
            |this, _, event: &SpeechEvent, window, cx| this.on_speech_event(event, window, cx),
        )
    }

    /// Recreate the speech state for the chosen engine and language.
    fn rebuild_speech(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.speech.update(cx, |speech, cx| speech.cancel(cx));
        self.speech = new_speech(self.engine, &self.language, cx);
        self._speech_subscription = Self::subscribe_speech(&self.speech, window, cx);
        self.input_device = input_device_name();
        cx.notify();
    }

    fn set_engine(&mut self, engine: Engine, window: &mut Window, cx: &mut Context<Self>) {
        if self.engine != engine {
            self.engine = engine;
            self.rebuild_speech(window, cx);
        }
    }

    fn toggle_speech(&mut self, _: &ToggleSpeech, _: &mut Window, cx: &mut Context<Self>) {
        self.speech.update(cx, |speech, cx| speech.toggle(cx));
    }

    fn on_speech_event(
        &mut self,
        event: &SpeechEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            SpeechEvent::Started => {
                self.session_started = Some(Instant::now());
                self.push_log(Tone::Progress, "Started", "Listening".into());
            }
            SpeechEvent::Partial(text) => {
                // Partial results arrive many times a second; keep one line per
                // stretch of them.
                if let Some(last) = self.log.last_mut()
                    && last.label == "Partial"
                {
                    last.text = text.clone();
                    last.at = self.opened.elapsed();
                } else {
                    self.push_log(Tone::Neutral, "Partial", text.clone());
                }
            }
            SpeechEvent::Final(text) => {
                if text.is_empty() {
                    self.push_log(Tone::Neutral, "Final", "No speech recognized".into());
                } else {
                    self.push_log(Tone::Success, "Final", text.clone());
                    self.insert_into_notes(text, window, cx);
                }
            }
            SpeechEvent::Cancelled => {
                self.push_log(Tone::Neutral, "Cancelled", "Transcript discarded".into());
            }
            SpeechEvent::Error(error) => {
                self.push_log(Tone::Danger, "Error", error.to_string().into());
                window.push_notification(
                    Notification::error(format!("Couldn’t dictate. {error}.")),
                    cx,
                );
            }
        }
        cx.notify();
    }

    fn insert_into_notes(
        &mut self,
        text: &SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.notes.update(cx, |notes, cx| {
            // Keep dictated passages apart from what is already there.
            let value = notes.value();
            let needs_space = value
                .chars()
                .next_back()
                .is_some_and(|c| !c.is_whitespace() && c.is_ascii());
            let text = if needs_space {
                format!(" {text}")
            } else {
                text.to_string()
            };
            notes.insert(text, window, cx);
            notes.focus(window, cx);
        });
    }

    fn push_log(&mut self, tone: Tone, label: &'static str, text: SharedString) {
        if self.log.len() == LOG_LIMIT {
            self.log.remove(0);
        }
        self.log.push(LogEntry {
            at: self.opened.elapsed(),
            tone,
            label,
            text,
        });
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let speech = self.speech.read(cx);
        let active = speech.status().is_active();
        let engine_note = match self.engine {
            Engine::System => {
                "The operating system’s recognizer. On macOS it only recognizes on this Mac; \
                 on Windows it uses Microsoft’s online service."
            }
            Engine::Demo => {
                "Types a scripted passage while it hears audio. Needs no service, network or \
                 speech permission."
            }
        };

        v_flex()
            .w(px(272.))
            .h_full()
            .flex_shrink_0()
            .gap_6()
            .p_4()
            .bg(cx.theme().sidebar)
            .text_color(cx.theme().sidebar_foreground)
            .border_r_1()
            .border_color(cx.theme().sidebar_border)
            .child(
                sidebar_section("Recognizer", cx)
                    .child(
                        ButtonGroup::new("engine")
                            .small()
                            .outline()
                            .w_full()
                            .disabled(active)
                            .child(
                                Button::new("engine-system")
                                    .flex_1()
                                    .label("System")
                                    .selected(self.engine == Engine::System),
                            )
                            .child(
                                Button::new("engine-demo")
                                    .flex_1()
                                    .label("Demo")
                                    .selected(self.engine == Engine::Demo),
                            )
                            .on_click(cx.listener(|this, clicks: &Vec<usize>, window, cx| {
                                let engine = if clicks.contains(&1) {
                                    Engine::Demo
                                } else {
                                    Engine::System
                                };
                                this.set_engine(engine, window, cx);
                            })),
                    )
                    .child(caption(engine_note, cx)),
            )
            .child(
                sidebar_section("Language", cx)
                    .child(Select::new(&self.language_select).small().disabled(active)),
            )
            .child(sidebar_section("Checks", cx).child(self.render_checks(cx)))
    }

    fn render_checks(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let speech = self.speech.read(cx);
        let recognizer = if !speech.has_recognizer() {
            Tag::danger().outline().child("Not supported")
        } else if speech.is_available(cx) {
            Tag::success().outline().child("Available")
        } else {
            Tag::warning().outline().child("Unavailable")
        };
        let session = match speech.status() {
            SpeechStatus::Idle => Tag::secondary().outline().child("Idle"),
            SpeechStatus::Connecting => Tag::info().outline().child("Connecting"),
            SpeechStatus::Recording => Tag::info().outline().child("Recording"),
            SpeechStatus::Stopping => Tag::info().outline().child("Finishing"),
        };
        let input = match &self.input_device {
            Some(name) => div()
                .min_w_0()
                .truncate()
                .child(name.clone())
                .into_any_element(),
            None => Tag::danger()
                .small()
                .outline()
                .child("None")
                .into_any_element(),
        };

        v_flex()
            .gap_2()
            .text_sm()
            .child(check_row("Platform", div().child(platform_name()), cx))
            .child(check_row("Input device", input, cx))
            .child(check_row("Recognizer", recognizer.small(), cx))
            .child(check_row("Session", session.small(), cx))
            .when(
                self.engine == Engine::System
                    && speech.has_recognizer()
                    && !speech.is_available(cx),
                |this| this.child(caption(unavailable_hint(), cx)),
            )
    }

    fn render_notes_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let characters = self.notes.read(cx).value().chars().count();

        h_flex()
            .items_center()
            .justify_between()
            .child(
                v_flex()
                    .child(div().text_lg().font_semibold().child("Notes"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(match characters {
                                0 => "Empty".to_string(),
                                1 => "1 character".to_string(),
                                n => format!("{n} characters"),
                            }),
                    ),
            )
            .child(
                Button::new("clear-notes")
                    .ghost()
                    .small()
                    .icon(Icon::new(ExtraIcon::Eraser))
                    .label("Clear")
                    .disabled(characters == 0)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.notes
                            .update(cx, |notes, cx| notes.set_value("", window, cx));
                        cx.notify();
                    })),
            )
    }

    /// The bar that runs a speech session: button, level, live text and controls.
    fn render_speech_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let speech = self.speech.read(cx);
        let status = speech.status();
        let transcript = speech.transcript();
        let supported = speech.has_recognizer();
        let available = speech.is_available(cx);
        let elapsed = self
            .session_started
            .filter(|_| status.is_active())
            .map(|started| format_duration(started.elapsed()));

        let (title, detail): (SharedString, SharedString) = match status {
            SpeechStatus::Idle if !supported => (
                "Not supported here".into(),
                "This platform has no system recognizer. Switch to Demo to try dictation.".into(),
            ),
            SpeechStatus::Idle if !available => ("Not available".into(), unavailable_hint().into()),
            SpeechStatus::Idle => (
                "Ready".into(),
                "Click the microphone to dictate into the note at the cursor.".into(),
            ),
            SpeechStatus::Connecting => (
                "Connecting…".into(),
                non_empty(
                    transcript,
                    "Start talking. What you say is kept while it connects.",
                ),
            ),
            SpeechStatus::Recording => (
                format!("Listening · {}", elapsed.unwrap_or_default()).into(),
                non_empty(transcript, "Start talking."),
            ),
            SpeechStatus::Stopping => (
                "Finishing…".into(),
                non_empty(transcript, "Waiting for the last words."),
            ),
        };
        let capturing = status.is_capturing();
        let toggle_keys = Keystroke::parse(TOGGLE_KEYS).ok().map(Kbd::new);

        h_flex()
            .gap_3()
            .px_3()
            .py_2p5()
            .items_center()
            .rounded(cx.theme().radius_lg)
            .border_1()
            .border_color(if capturing {
                cx.theme().ring
            } else {
                cx.theme().border
            })
            .bg(cx.theme().background)
            .when(capturing, |this| this.shadow_sm())
            .child(
                SpeechButton::new(&self.speech)
                    .show_when_unsupported(true)
                    .large(),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_0p5()
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .text_color(if supported && available || status.is_active() {
                                cx.theme().foreground
                            } else {
                                cx.theme().muted_foreground
                            })
                            .child(title),
                    )
                    .child(
                        div()
                            .text_sm()
                            .truncate()
                            .text_color(if status.is_active() && !speech.transcript().is_empty() {
                                cx.theme().foreground
                            } else {
                                cx.theme().muted_foreground
                            })
                            .child(detail),
                    ),
            )
            .when(status.is_active(), |this| {
                this.child(SpeechWaveform::new(&self.speech).w(px(112.)).small())
            })
            .map(|this| {
                if status.is_active() {
                    this.child(
                        Button::new("discard")
                            .ghost()
                            .small()
                            .label("Discard")
                            .tooltip("Stop without inserting the text")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.speech.update(cx, |speech, cx| speech.cancel(cx));
                            })),
                    )
                } else {
                    this.when_some(toggle_keys.filter(|_| available), |this, kbd| {
                        this.child(kbd)
                    })
                }
            })
    }

    fn render_log(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .h(px(168.))
            .flex_shrink_0()
            .rounded(cx.theme().radius_lg)
            .border_1()
            .border_color(cx.theme().border)
            .overflow_hidden()
            .child(
                h_flex()
                    .px_3()
                    .py_1p5()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .text_xs()
                            .font_medium()
                            .text_color(cx.theme().muted_foreground)
                            .child(Icon::new(ExtraIcon::ScrollText).xsmall())
                            .child("Session log"),
                    )
                    .child(
                        Button::new("clear-log")
                            .ghost()
                            .xsmall()
                            .label("Clear")
                            .disabled(self.log.is_empty())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.log.clear();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div().flex_1().min_h_0().child(
                    v_flex()
                        .id("session-log")
                        .size_full()
                        .px_3()
                        .py_2()
                        .gap_1()
                        .overflow_y_scrollbar()
                        .when(self.log.is_empty(), |this| {
                            this.items_center().justify_center().child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Events of each session appear here."),
                            )
                        })
                        .children(self.log.iter().rev().map(|entry| log_row(entry, cx))),
                ),
            )
    }
}

impl Focusable for SpeechApp {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SpeechApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .on_action(cx.listener(Self::toggle_speech))
            .child(
                TitleBar::new().child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .text_sm()
                        .font_medium()
                        .child(
                            Icon::new(ExtraIcon::AudioLines)
                                .small()
                                .text_color(cx.theme().primary),
                        )
                        .child("Speech"),
                ),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.render_sidebar(cx))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .gap_4()
                            .p_5()
                            .child(self.render_notes_header(cx))
                            .child(
                                div()
                                    .flex_1()
                                    .min_h_0()
                                    .child(Textarea::new(&self.notes).h_full()),
                            )
                            .child(self.render_speech_bar(cx))
                            .child(self.render_log(cx)),
                    ),
            )
    }
}

fn sidebar_section(title: &'static str, cx: &App) -> Div {
    v_flex().gap_2().child(
        div()
            .text_xs()
            .font_medium()
            .text_color(cx.theme().muted_foreground)
            .child(title),
    )
}

fn caption(text: &'static str, cx: &App) -> impl IntoElement {
    div()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(text)
}

fn check_row(label: &'static str, value: impl IntoElement, cx: &App) -> impl IntoElement {
    h_flex()
        .gap_3()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex_shrink_0()
                .text_color(cx.theme().muted_foreground)
                .child(label),
        )
        .child(h_flex().flex_1().min_w_0().justify_end().child(value))
}

fn log_row(entry: &LogEntry, cx: &App) -> impl IntoElement {
    let color = match entry.tone {
        Tone::Neutral => cx.theme().muted_foreground,
        Tone::Progress => cx.theme().info,
        Tone::Success => cx.theme().success,
        Tone::Danger => cx.theme().danger,
    };

    h_flex()
        .gap_3()
        .items_start()
        .text_xs()
        .child(
            div()
                .flex_shrink_0()
                .font_family(cx.theme().mono_font_family.clone())
                .text_color(cx.theme().muted_foreground)
                .child(format_timestamp(entry.at)),
        )
        .child(
            div()
                .w(px(64.))
                .flex_shrink_0()
                .font_medium()
                .text_color(color)
                .child(entry.label),
        )
        .child(div().flex_1().min_w_0().child(entry.text.clone()))
}

fn unavailable_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "This Mac can’t recognize the language offline, or speech recognition access is off \
         in System Settings › Privacy & Security."
    } else if cfg!(target_os = "windows") {
        "Install the language’s speech pack and turn on Online speech recognition in Settings › \
         Privacy & security › Speech."
    } else {
        "The recognizer can’t start right now."
    }
}

fn non_empty(text: SharedString, fallback: &'static str) -> SharedString {
    if text.is_empty() {
        fallback.into()
    } else {
        text
    }
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn format_timestamp(duration: Duration) -> String {
    let millis = duration.as_millis();
    format!(
        "{:02}:{:02}.{:03}",
        millis / 60_000,
        millis / 1_000 % 60,
        millis % 1_000
    )
}

/// Print what this machine supports and quit.
fn print_checks(cx: &mut App) {
    println!("Platform       {}", platform_name());
    println!(
        "Input device   {}",
        input_device_name().as_deref().unwrap_or("none")
    );
    println!("System recognizer, by language:");
    for language in languages() {
        let recognizer = SystemRecognizer::new().locale(language.tag.clone());
        let available =
            gpui_kit::component::speech::SpeechRecognizer::is_available(&recognizer, cx);
        println!(
            "  {:<7} {:<12} {}",
            language.tag.as_ref(),
            if available {
                "available"
            } else {
                "unavailable"
            },
            language.name.as_ref(),
        );
    }
}

fn main() {
    let check = std::env::args().any(|arg| arg == "--check");
    let app = gpui_kit::application().with_assets(AppAssets);

    app.run(move |cx| {
        gpui_kit::init(cx);
        if check {
            print_checks(cx);
            cx.quit();
            return;
        }

        cx.bind_keys([KeyBinding::new(TOGGLE_KEYS, ToggleSpeech, None)]);
        cx.activate(true);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(980.), px(680.)), cx)),
            window_min_size: Some(size(px(760.), px(520.))),
            ..TitleBar::window_options()
        };
        gpui_kit::open_window(window_options, cx, |window, cx| {
            cx.new(|cx| SpeechApp::new(window, cx))
        })
        .expect("Failed to open window");
    });
}
