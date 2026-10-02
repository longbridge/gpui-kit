---
title: Speech
description: 通过系统语音识别器或应用自己的识别器，把麦克风中的语音转成文本。
maturity: [experimental, platform-dependent]
---

# Speech

Speech 模块把用户说的话转成文本。`SpeechState` 管理一次语音输入会话：它从 `AudioInput` 采集音频，交给 `SpeechRecognizer` 识别，保存转写文本并发出 `SpeechEvent`。`SpeechButton` 负责开始和停止会话，`SpeechWaveform` 在采集期间显示输入音量。

文本写到哪里由应用决定。状态本身不会修改任何输入框；应用订阅它的事件，再把最终文本插入到合适的位置。

识别和采集都可以替换。不做额外配置时，状态使用 macOS 或 Windows 自带的识别器和默认麦克风。实现 `SpeechRecognizer` 可以接入云端服务或本地模型，实现 `AudioInput` 可以从其他来源提供音频。

## 启用 feature

麦克风和系统识别器位于 `speech` feature 之后：

```toml
[dependencies]
gpui-kit = { version = "{{gpui_kit_version}}", features = ["speech"] }
```

该 feature 会引入 [cpal](https://crates.io/crates/cpal) 采集音频。在 Linux 上构建需要 ALSA 开发包（Debian 和 Ubuntu 上为 `libasound2-dev`）。

未启用该 feature 时，以及在 Web 上，这些类型依然可用，但没有默认输入，也没有系统识别器。此时状态只有在应用同时提供识别器和输入时才能工作。

## 导入

```rust
use gpui_kit::component::speech::{
    SpeechButton, SpeechEvent, SpeechState, SpeechStatus, SpeechWaveform,
};
```

## 用法

### 语音输入到 Input

在要填充的输入框旁边创建状态并订阅它，再把按钮和波形放到输入框的 suffix 中：

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

把订阅保存在拥有该输入框的视图上，例如放进它的 `_subscriptions` 列表。

### 事件

| 事件 | 触发时机 |
| --- | --- |
| `Started` | 会话已开始，正在采集音频。 |
| `Partial(text)` | 用户说话期间转写文本发生变化。 |
| `Final(text)` | 会话正常结束。文本可能为空。 |
| `Cancelled` | 会话被取消，转写文本被丢弃。 |
| `Error(error)` | 会话失败并结束。 |

`Partial` 和 `Final` 携带的是本次会话到目前为止的**完整**转写文本：所有已确认的短语，加上当前的识别假设。后一个事件会取代前一个，因此把最新的 `Partial` 文本作为预览显示，只提交 `Final` 文本。在 render 中，`SpeechState::transcript()` 返回同样的文本：

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

### 控制会话

`SpeechButton` 会切换会话。如果要从 Action、快捷键或其他控件触发，直接调用状态的方法：

```rust
speech.update(cx, |speech, cx| speech.start(cx));  // 会话进行中时不做任何事。
speech.update(cx, |speech, cx| speech.stop(cx));   // 结果到达后发出 `Final`。
speech.update(cx, |speech, cx| speech.cancel(cx)); // 立即发出 `Cancelled`。
speech.update(cx, |speech, cx| speech.toggle(cx)); // 空闲时 `start`，否则 `stop`。
```

`status()` 表示会话所处的阶段：

| `SpeechStatus` | 含义 |
| --- | --- |
| `Idle` | 没有进行中的会话。 |
| `Connecting` | 正在采集音频，识别器仍在连接。 |
| `Recording` | 正在采集并识别音频。 |
| `Stopping` | 已停止采集，等待最终结果。 |

除 `Idle` 外，`is_active()` 都为 true；`is_capturing()` 只在 `Connecting` 和 `Recording` 时为 true。处于 `Stopping` 时，按钮显示 spinner 并忽略点击。如果识别器在停止超时内没有给出最终结果，会话会以当前已有的文本结束。超时默认为 3 秒：

```rust
let speech = cx.new(|cx| SpeechState::new(cx).stop_timeout(Duration::from_secs(5)));
```

### 按钮状态

`SpeechButton` 是一个 ghost 样式的图标 `Button`，带有本地化的 tooltip 和无障碍名称：空闲时显示麦克风，采集时显示按下状态的停止图标。

- 当状态在当前平台上没有识别器或没有输入（`has_recognizer()` 为 `false`）时，按钮**不渲染任何内容**，应用可以无条件放置它。使用 `.show_when_unsupported(true)` 可以改为渲染禁用的按钮。
- 当识别器报告自己不可用（`is_available(cx)` 为 `false`），例如语言未安装时，按钮会禁用，tooltip 提示不可用。
- `.disabled(true)` 用于应用自身的禁用原因，例如 readonly 的输入框。

```rust
SpeechButton::new(&speech)
    .small()
    .show_when_unsupported(true)
    .disabled(readonly)
```

## 选择识别器

状态按以下顺序选择识别器：

1. 通过 `.recognizer(...)` 传入的识别器；
2. 否则使用平台的 `SystemRecognizer`，除非 `.system_fallback(false)` 关闭了回退；
3. 否则没有识别器，`has_recognizer()` 为 `false`。

```rust
use gpui_kit::component::speech::SystemRecognizer;

// 无论系统语言是什么，都识别中文。
let speech = cx.new(|cx| {
    SpeechState::new(cx).recognizer(SystemRecognizer::new().locale("zh-CN"))
});

// 使用应用自己的识别器代替系统识别器。
let speech = cx.new(|cx| SpeechState::new(cx).recognizer(CloudRecognizer::new(client)));

// 不回退到系统识别器。没有自己的识别器时，
// `has_recognizer()` 为 false，按钮不渲染任何内容。
let speech = cx.new(|cx| SpeechState::new(cx).system_fallback(false));
```

`CloudRecognizer` 代表应用中实现了 `SpeechRecognizer` 的类型，参见[实现识别器](#实现识别器)。

`SystemRecognizer::new()` 识别系统当前语言；`.locale(...)` 接受 BCP 47 语言标签，例如 `en-US` 或 `zh-CN`。平台无法识别的语言会使识别器不可用。

## 实现识别器

实现 `SpeechRecognizer` 即可接入任意语音服务。`start` 打开一个 `RecognitionSession`，并通过收到的 `SpeechSink` 报告结果：

```rust
use gpui_kit::App;
use gpui_kit::component::speech::{
    RecognitionSession, SpeechError, SpeechRecognizer, SpeechSink,
};

/// 报告已收到多少音频，用来代替真实服务。
struct DurationRecognizer;

impl SpeechRecognizer for DurationRecognizer {
    fn start(
        &self,
        sink: SpeechSink,
        cx: &mut App,
    ) -> Result<Box<dyn RecognitionSession>, SpeechError> {
        // 真实服务在这里建立连接，连接成功后再调用 `ready`。
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
        // 16 kHz 单声道，即默认的 `audio_format`。
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

一次会话的流程如下：

1. **`start`** 打开会话。连接可能需要一段时间，因此应立即返回，缓存这期间推送的音频，并在服务开始接收音频时调用 `sink.ready(cx)`。状态随之从 `Connecting` 变为 `Recording`。
2. **`push_audio`** 按识别器的 `audio_format()`（默认 16 kHz 单声道）传入交错的 16 位 PCM。用 `sink.hypothesis(...)` 报告正在说的短语，它会替换上一个识别假设；用 `sink.phrase(...)` 报告识别完成的短语，它会确认该短语并清空识别假设。
3. **`finish`** 表示用户已停止说话：发送剩余音频，等待最后的结果，然后调用 `sink.finish(cx)`。状态随后发出 `Final`。
4. **drop** 会话即取消会话。在 drop 时关闭连接，之后不再报告任何内容。

出错时调用 `sink.error(SpeechError::recognizer(error), cx)`，会话以 `SpeechEvent::Error` 结束。

以下几点能让识别器保持简单：

- sink 可以低成本 clone，读取服务响应的任务可以持有一份。会话停止、取消或被替换后，它的调用会被忽略，迟到的响应不会混入下一次会话。
- sink 的调用在当前 update 结束后生效。可以在主线程的任何位置调用，包括在 `start` 或 `push_audio` 内部。
- 短语按原样拼接。请带上语言需要的分隔符，例如英文句子之间的前导空格；中文句子之间则不需要。
- 识别器暂时无法工作时（例如用户登录之前），重写 `is_available` 返回 `false`。它在每次 render 时都会调用，应保持轻量。

## 提供音频

`AudioInput` 是音频的扩展点。内置的 `Microphone` 从默认输入设备采集，并转换为识别器需要的格式。如需从其他来源提供音频（例如测试中的文件），替换它即可：

```rust
use std::time::Duration;

use gpui_kit::{App, Subscription};
use gpui_kit::component::speech::{AudioFormat, AudioInput, AudioSink, SpeechError};

/// 每次推送 100 ms 的静音。
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
        // 采集一直持续到状态 drop 这个 subscription。
        Ok(Subscription::new(move || drop(task)))
    }
}

let speech = cx.new(|cx| SpeechState::new(cx).recognizer(recognizer).input(Silence));
```

`levels()` 返回 `SpeechWaveform` 绘制所用的近期输入音量，取值 `0.0..=1.0`，按时间从旧到新排列，每 25 ms 音频一个值，适合需要自绘音量表的应用。音量升高时立即跟上、回落时平滑下降，背景噪音记为 `0.0`。

## 平台支持

| 平台 | `SystemRecognizer` | `Microphone` |
| --- | --- | --- |
| macOS | `SFSpeechRecognizer`，仅在本机识别 | Core Audio |
| Windows | `Windows.Media.SpeechRecognition`，经由 Microsoft 在线服务 | WASAPI |
| Linux | 无，需由应用提供识别器 | ALSA |
| Web | 无 | 无，需由应用提供输入 |

### macOS

应用的 `Info.plist` 必须说明使用麦克风和语音识别的原因：

```xml
<key>NSMicrophoneUsageDescription</key>
<string>用于在消息中语音输入文字。</string>
<key>NSSpeechRecognitionUsageDescription</key>
<string>用于把你的语音转成文字。</string>
```

缺少 `NSMicrophoneUsageDescription` 时，系统会直接拒绝麦克风访问，不会询问用户。缺少 `NSSpeechRecognitionUsageDescription` 时，系统识别器会报告不可用，而不是发起询问，因为缺少该键时发起询问会导致进程终止。在 app bundle 之外运行的二进制（例如通过 `cargo run` 启动）不包含这两个键。

识别只在本机进行，音频不会离开设备。Mac 无法离线识别的语言不可用。

### Windows

语音输入需要安装对应语言的语音包，并在**设置 > 隐私和安全性 > 语音**中打开**联机语音识别**。音频会发送到 Microsoft 的在线语音服务。音频必须保留在本机的应用应使用 `.system_fallback(false)`，并提供自己的识别器。

Windows 识别器会自行监听默认麦克风。状态仍通过其输入采集音频，但只用于驱动波形，因此自定义 `AudioInput` 不会改变系统识别器听到的内容。

### Linux

Linux 没有系统识别器，只有在应用提供识别器时才能使用语音输入；否则 `SpeechButton` 不渲染任何内容。`Microphone` 通过 ALSA 采集，构建时需要 `libasound2-dev`。

## API 参考

- [SpeechState]：会话本身，包括 `recognizer`、`input`、`system_fallback`、`stop_timeout`、`start`、`stop`、`cancel`、`toggle`、`status`、`has_recognizer`、`is_available`、`transcript`、`levels`
- [SpeechEvent] 与 [SpeechStatus]
- [SpeechButton]：`show_when_unsupported`，以及 `Sizable` 和 `Disableable`
- [SpeechWaveform]：铺满自身宽度的实时波形（不设宽度时为 96 px），采集期间滚动；`Sizable` 决定高度，`Styled` 决定宽度
- [SpeechRecognizer]、[RecognitionSession] 与 [SpeechSink]：识别的扩展点
- [AudioInput]、[AudioSink] 与 [AudioFormat]：音频的扩展点
- [SpeechError]
- [SystemRecognizer] 与 [Microphone]：`speech` feature 提供的默认实现

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
