---
title: Speech
description: Dictate text from the microphone through the system speech recognizer or any recognizer the application provides.
maturity: [experimental, platform-dependent]
---

# Speech

The speech module turns what the user says into text. `SpeechState` owns a
dictation session: it captures audio from an `AudioInput`, feeds it to a
`SpeechRecognizer`, keeps the transcript, and emits `SpeechEvent`s.
`SpeechButton` starts and stops the session and `SpeechWaveform` shows the
input level while it captures.

The application owns where the text goes. The state never edits an input on its
own; subscribe to its events and insert the final transcript where it belongs.

Recognition and capture are both replaceable. Without further setup, the state
uses the recognizer built into macOS or Windows and the default microphone.
Implement `SpeechRecognizer` to use a cloud service or a local model, and
`AudioInput` to feed audio from elsewhere.

## Enable the feature

The microphone and the system recognizer are behind the `speech` feature:

```toml
[dependencies]
gpui-kit = { version = "{{gpui_kit_version}}", features = ["speech"] }
```

The feature adds [cpal](https://crates.io/crates/cpal) for audio capture. On
Linux, building it needs the ALSA development package (`libasound2-dev` on
Debian and Ubuntu).

Without the feature, and on the web, the types are still available but there is
no default input and no system recognizer. A state then works only with both an
application recognizer and an application input.

## Import

```rust
use gpui_kit::component::speech::{
    SpeechButton, SpeechEvent, SpeechState, SpeechStatus, SpeechWaveform,
};
```

## Usage

### Dictate into an input

Create the state beside the input it fills, subscribe to it, and put the button
and waveform in the input's suffix:

```rust
use gpui_kit::component::{
    WindowExt as _, h_flex,
    input::{Input, InputState},
    notification::Notification,
    speech::{SpeechButton, SpeechEvent, SpeechState, SpeechWaveform},
};

let input = cx.new(|cx| InputState::new(window, cx));
let speech = cx.new(SpeechState::new);

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
```

```rust
let capturing = self.speech.read(cx).status().is_capturing();

Input::new(&self.input).suffix(
    h_flex()
        .gap_2()
        .when(capturing, |this| {
            this.child(SpeechWaveform::new(&self.speech).w(px(48.)).xsmall())
        })
        .child(SpeechButton::new(&self.speech).xsmall()),
)
```

Keep the subscription on the view that owns the input, for example in its
`_subscriptions` list.

### Events

| Event | When |
| --- | --- |
| `Started` | A session started and audio is being captured. |
| `Partial(text)` | The transcript changed while the user speaks. |
| `Final(text)` | The session ended normally. The text may be empty. |
| `Cancelled` | The session was cancelled and its transcript discarded. |
| `Error(error)` | The session failed and ended. |

`Partial` and `Final` carry the **whole** transcript of the session so far:
every committed phrase followed by the current hypothesis. A later event
replaces an earlier one, so show the latest `Partial` text as a preview and
commit only the `Final` text. `SpeechState::transcript()` returns the same text
during render:

```rust
let speech = self.speech.read(cx);

v_flex()
    .gap_2()
    .child(Input::new(&self.input))
    .when(speech.status().is_active(), |this| {
        this.child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(speech.transcript()),
        )
    })
```

### Session control

`SpeechButton` toggles the session. To drive it from an action, a key binding,
or another control, call the state directly:

```rust
speech.update(cx, |speech, cx| speech.start(cx));  // Does nothing while running.
speech.update(cx, |speech, cx| speech.stop(cx));   // Emits `Final` once the result is in.
speech.update(cx, |speech, cx| speech.cancel(cx)); // Emits `Cancelled` at once.
speech.update(cx, |speech, cx| speech.toggle(cx)); // `start` when idle, otherwise `stop`.
```

`status()` reports where the session is:

| `SpeechStatus` | Meaning |
| --- | --- |
| `Idle` | No session is running. |
| `Connecting` | Audio is captured while the recognizer connects. |
| `Recording` | Audio is captured and recognized. |
| `Stopping` | Capture stopped; waiting for the final result. |

`is_active()` is true for every status but `Idle`, and `is_capturing()` for
`Connecting` and `Recording`. While `Stopping`, the button shows a spinner and
ignores clicks. If the recognizer does not deliver its final result within the
stop timeout, the session ends with the transcript so far. The timeout is
3 seconds by default:

```rust
let speech = cx.new(|cx| SpeechState::new(cx).stop_timeout(Duration::from_secs(5)));
```

### Button states

`SpeechButton` is a ghost icon `Button` with a localized tooltip and accessible
name: a microphone at rest, and a pressed stop glyph while capturing.

- When the state has no recognizer or no input on this platform
  (`has_recognizer()` is `false`), the button renders **nothing**, so an
  application can place it unconditionally. Use `.show_when_unsupported(true)`
  to render it disabled instead.
- When the recognizer reports itself unavailable (`is_available(cx)` is
  `false`), for example because the language is not installed, the button is
  disabled with an “unavailable” tooltip.
- `.disabled(true)` disables it for application reasons, such as a readonly
  field.

```rust
SpeechButton::new(&speech)
    .small()
    .show_when_unsupported(true)
    .disabled(readonly)
```

## Choose a recognizer

The state picks its recognizer in this order:

1. the one passed to `.recognizer(...)`;
2. otherwise the platform's `SystemRecognizer`, unless `.system_fallback(false)`
   turned it off;
3. otherwise none, and `has_recognizer()` is `false`.

```rust
use gpui_kit::component::speech::SystemRecognizer;

// Dictate Chinese regardless of the system language.
let speech = cx.new(|cx| {
    SpeechState::new(cx).recognizer(SystemRecognizer::new().locale("zh-CN"))
});

// Use the application's own recognizer instead of the system's.
let speech = cx.new(|cx| SpeechState::new(cx).recognizer(CloudRecognizer::new(client)));

// Never fall back to the system recognizer. Without a recognizer of its own,
// `has_recognizer()` is false and the button renders nothing.
let speech = cx.new(|cx| SpeechState::new(cx).system_fallback(false));
```

`CloudRecognizer` stands for an application type that implements
`SpeechRecognizer`; see [Implement a recognizer](#implement-a-recognizer).

`SystemRecognizer::new()` recognizes the system's current language;
`.locale(...)` takes a BCP 47 tag such as `en-US` or `zh-CN`. A language the
platform cannot recognize makes the recognizer unavailable.

## Implement a recognizer

Implement `SpeechRecognizer` to use any speech service. `start` opens a
`RecognitionSession` and reports results through the `SpeechSink` it receives:

```rust
use gpui_kit::App;
use gpui_kit::component::speech::{
    RecognitionSession, SpeechError, SpeechRecognizer, SpeechSink,
};

/// Reports how much audio it has heard, as a stand-in for a real service.
struct DurationRecognizer;

impl SpeechRecognizer for DurationRecognizer {
    fn start(
        &self,
        sink: SpeechSink,
        cx: &mut App,
    ) -> Result<Box<dyn RecognitionSession>, SpeechError> {
        // A service would connect here and call `ready` once connected.
        sink.ready(cx);
        Ok(Box::new(DurationSession { sink, samples: 0 }))
    }
}

struct DurationSession {
    sink: SpeechSink,
    samples: usize,
}

impl RecognitionSession for DurationSession {
    fn push_audio(&mut self, samples: &[i16], cx: &mut App) {
        // 16 kHz mono, the default `audio_format`.
        self.samples += samples.len();
        let seconds = self.samples / 16_000;
        self.sink.hypothesis(format!("{seconds} s of audio"), cx);
    }

    fn finish(&mut self, cx: &mut App) {
        let seconds = self.samples / 16_000;
        self.sink.phrase(format!("{seconds} s of audio"), cx);
        self.sink.finish(cx);
    }
}
```

A session runs as follows:

1. **`start`** opens the session. Connecting may take a while, so return at
   once, buffer the audio pushed in the meantime, and call `sink.ready(cx)`
   when the service accepts audio. The status moves from `Connecting` to
   `Recording`.
2. **`push_audio`** delivers interleaved 16-bit PCM in the recognizer's
   `audio_format()`, 16 kHz mono by default. Report the phrase being spoken
   with `sink.hypothesis(...)`, which replaces the previous hypothesis, and a
   recognized phrase with `sink.phrase(...)`, which commits it and clears the
   hypothesis.
3. **`finish`** means the user stopped talking: send the remaining audio, wait
   for the last result, then call `sink.finish(cx)`. The state then emits
   `Final`.
4. **Dropping** the session cancels it. Close the connection there and report
   nothing more.

Report a failure with `sink.error(SpeechError::recognizer(error), cx)`; the
session ends with `SpeechEvent::Error`.

A few rules keep a recognizer simple:

- The sink is cheap to clone, so a task that reads the service's responses can
  own a copy. Once its session is stopped, cancelled, or replaced, its calls are
  ignored, so a late response cannot leak into the next session.
- Sink calls are applied after the current update. They may be made from
  anywhere on the main thread, including from inside `start` or `push_audio`.
- Phrases are joined verbatim. Include any separator the language needs, such
  as a leading space between English sentences and none between Chinese ones.
- Override `is_available` to return `false` while the recognizer cannot work,
  for example before the user signs in. It is called on every render, so keep
  it cheap.

## Provide audio

`AudioInput` is the audio seam. The built-in `Microphone` captures the default
input device and converts it to the recognizer's format. Replace it to feed
audio from elsewhere, such as a file in tests:

```rust
use std::time::Duration;

use gpui_kit::{App, Subscription};
use gpui_kit::component::speech::{AudioFormat, AudioInput, AudioSink, SpeechError};

/// Pushes 100 ms of silence at a time.
struct Silence;

impl AudioInput for Silence {
    fn start(
        &self,
        format: AudioFormat,
        sink: AudioSink,
        cx: &mut App,
    ) -> Result<Subscription, SpeechError> {
        let len = format.sample_rate() as usize / 10 * format.channels() as usize;
        let task = cx.spawn(async move |cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(100)).await;
                cx.update(|cx| sink.push(vec![0; len], cx));
            }
        });
        // Capture runs until the state drops this subscription.
        Ok(Subscription::new(move || drop(task)))
    }
}

let speech = cx.new(|cx| SpeechState::new(cx).recognizer(recognizer).input(Silence));
```

`levels()` exposes the recent input levels that `SpeechWaveform` draws, in
`0.0..=1.0` with the oldest first, one per 80 ms of audio, for an application
that renders its own meter. Peaks rise at once and fall back smoothly, and
background noise reads as `0.0`.

## Platform support

| Platform | `SystemRecognizer` | `Microphone` |
| --- | --- | --- |
| macOS | `SFSpeechRecognizer`, on the device only | Core Audio |
| Windows | `Windows.Media.SpeechRecognition`, through Microsoft's online service | WASAPI |
| Linux | None; provide a recognizer | ALSA |
| Web | None | None; provide an input |

### macOS

The application's `Info.plist` must describe why it uses the microphone and
speech recognition:

```xml
<key>NSMicrophoneUsageDescription</key>
<string>Dictate text into messages.</string>
<key>NSSpeechRecognitionUsageDescription</key>
<string>Turn your speech into text.</string>
```

Without `NSMicrophoneUsageDescription`, the system refuses microphone access
without asking. Without `NSSpeechRecognitionUsageDescription`, the system
recognizer reports itself unavailable instead of asking, because asking without
it would terminate the process. A binary run outside an app bundle, such as
from `cargo run`, has neither key.

Recognition runs on the device only, so audio never leaves the machine. A
language the Mac cannot recognize offline is unavailable.

### Windows

Dictation needs the language's speech pack and the **Online speech
recognition** setting in **Settings > Privacy & security > Speech**. Audio is
sent to Microsoft's online speech service. An application that must keep audio
on the device should use `.system_fallback(false)` with its own recognizer.

The Windows recognizer listens to the default microphone itself. The state
still captures through its input, but only to drive the waveform, so a custom
`AudioInput` does not change what the system recognizer hears.

### Linux

Linux has no system recognizer. Speech input works there only with an
application recognizer; without one, `SpeechButton` renders nothing. The
`Microphone` captures through ALSA, and building it needs `libasound2-dev`.

## API Reference

- [SpeechState] — the session: `recognizer`, `input`, `system_fallback`,
  `stop_timeout`, `start`, `stop`, `cancel`, `toggle`, `status`,
  `has_recognizer`, `is_available`, `transcript`, `levels`
- [SpeechEvent] and [SpeechStatus]
- [SpeechButton] — `show_when_unsupported`, plus `Sizable` and `Disableable`
- [SpeechWaveform] — a live waveform that grows from its trailing edge as audio arrives and scrolls at a steady pace (one bar per 80 ms) until it fills its width (96 px unless styled); silence shows as dots; `Sizable` sets its height, `Styled` its width
- [SpeechRecognizer], [RecognitionSession] and [SpeechSink] — the recognition
  seam
- [AudioInput], [AudioSink] and [AudioFormat] — the audio seam
- [SpeechError]
- [SystemRecognizer] and [Microphone] — the `speech` feature's defaults

[SpeechState]: https://docs.rs/gpui-component/latest/gpui_component/speech/struct.SpeechState.html
[SpeechEvent]: https://docs.rs/gpui-component/latest/gpui_component/speech/enum.SpeechEvent.html
[SpeechStatus]: https://docs.rs/gpui-component/latest/gpui_component/speech/enum.SpeechStatus.html
[SpeechButton]: https://docs.rs/gpui-component/latest/gpui_component/speech/struct.SpeechButton.html
[SpeechWaveform]: https://docs.rs/gpui-component/latest/gpui_component/speech/struct.SpeechWaveform.html
[SpeechRecognizer]: https://docs.rs/gpui-component/latest/gpui_component/speech/trait.SpeechRecognizer.html
[RecognitionSession]: https://docs.rs/gpui-component/latest/gpui_component/speech/trait.RecognitionSession.html
[SpeechSink]: https://docs.rs/gpui-component/latest/gpui_component/speech/struct.SpeechSink.html
[AudioInput]: https://docs.rs/gpui-component/latest/gpui_component/speech/trait.AudioInput.html
[AudioSink]: https://docs.rs/gpui-component/latest/gpui_component/speech/struct.AudioSink.html
[AudioFormat]: https://docs.rs/gpui-component/latest/gpui_component/speech/struct.AudioFormat.html
[SpeechError]: https://docs.rs/gpui-component/latest/gpui_component/speech/enum.SpeechError.html
[SystemRecognizer]: https://docs.rs/gpui-component/latest/gpui_component/speech/struct.SystemRecognizer.html
[Microphone]: https://docs.rs/gpui-component/latest/gpui_component/speech/struct.Microphone.html
