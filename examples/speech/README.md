# Speech

A notepad you can talk into, built on GPUI Component's speech input. It is also
the test bench for the platform recognizers: the sidebar shows what this machine
supports, and the session log records every `SpeechEvent`.

```sh
cargo run -p speech             # open the app
cargo run -p speech -- --check  # print the checks and exit
```

`--check` prints the platform, the default input device, and whether the system
recognizer is available for each language in the picker:

```text
Platform       macOS
Input device   MacBook Pro Microphone
System recognizer, by language:
  en-US   available    English (US)
  zh-CN   available    简体中文
  ja-JP   unavailable  日本語
```

## Trying it

- **Demo** types a scripted passage while it hears audio. It needs no service,
  network, or speech permission, so it works on every platform, Linux included.
  Use it to check the capture, waveform, and event flow.
- **System** uses the operating system's recognizer in the language you pick.
- Click the microphone or press <kbd>⇧⌘D</kbd> (<kbd>Ctrl+Shift+D</kbd> on
  Windows and Linux) to start, and again to stop. The text goes into the note
  at the cursor. **Discard** stops without inserting anything.

## Platform notes

### macOS

`build.rs` links `Info.plist` into the executable, so `cargo run` can ask for
microphone and speech recognition access without an app bundle. The first
session asks for both. Recognition stays on the Mac, so a language the Mac can't
recognize offline shows **Unavailable**.

When you run from a terminal, macOS attributes the microphone to the terminal
app. To ask again after denying access:

```sh
tccutil reset Microphone
tccutil reset SpeechRecognition
```

### Windows

Install the language's speech pack (Settings › Time & language › Speech) and
turn on **Online speech recognition** (Settings › Privacy & security › Speech).
Dictation runs through Microsoft's online service. The recognizer records from
the default input device itself, and the waveform follows the same device.

### Linux

There is no system recognizer, so **System** shows **Not supported**. Use
**Demo**, or plug in your own `SpeechRecognizer`. Building needs
`libasound2-dev`.
