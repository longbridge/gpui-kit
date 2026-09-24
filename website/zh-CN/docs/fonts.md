---
title: Fonts
order: -8
description: 系统字体、主题字体、元素级覆盖与自定义字体打包。
---

# Fonts

本文说明应用应提供哪些字体。GPUI 如何解析、塑形、测量并绘制字形，见 [TextSystem](./text-system)。

## 默认字体

每个应用都从主题自带的一套 UI 字体和等宽字体开始：

| 用途 | 字体 | 字号 |
| --- | --- | --- |
| UI 文本 | `.SystemUIFont` | 16px |
| 代码／等宽 | macOS：`Menlo`，Windows：`Consolas`，Linux：`DejaVu Sans Mono` | 13px |

编辑器使用 `mono_font_family` 和 `mono_font_size` 绘制代码，详见
[Editor](../component/editor.md)。

应用主题时会对照系统已安装的字体检查这两个默认值：等宽默认字体缺失时换成已安装的备选；当 `.SystemUIFont` 解析到的是 GPUI 回退栈里的某个字体而不是系统字体本身（Linux 桌面通常没有 GPUI 映射到的那个字体），主题会直接记下该字体名，让文本查找一直命中缓存。你自己设置的字体保持不变。

## 系统字体

桌面应用可以直接按名称使用**操作系统已安装的任意字体**，无需打包、无需配置。GPUI 会实时向系统字库解析（macOS 用 CoreText，Windows 用 DirectWrite，Linux 用 fontconfig）。

```rust
div().font_family("Segoe UI")

Editor::new(&editor).font_family("JetBrains Mono")
```

各平台常见字体举例：

- macOS：`SF Pro`、`Helvetica`、`Arial`、`Times New Roman`、`Menlo`、`Monaco`
- Windows：`Segoe UI`、`Arial`、`Consolas`、`Courier New`
- Linux：`Noto Sans`、`DejaVu Sans`、`Liberation Sans`、`DejaVu Sans Mono`

如果名称与已安装字体不匹配，GPUI 会静默回退——请在每个目标平台上确认准确的 family 名称。

## 通过 Theme 修改字体

通过 `Theme::update` 设置应用级字体，它会同步到底层并刷新窗口：

```rust
Theme::update(cx, |theme| {
    theme.font_family = "Inter".into();
    theme.mono_font_family = "JetBrains Mono".into();
    theme.font_size = px(18.);
});
```

`font_size` 同时是应用缩放控制——`Root` 会调用
`window.set_rem_size(cx.theme().font_size)`，因此基于 [`rem` 的间距](./geometry)会跟随缩放。详见[编码指南](./coding-guides.md)。

## 元素级覆盖

任何元素都可以在不改动主题的情况下覆盖字体：

```rust
div()
    .font_family("JetBrains Mono")
    .text_size(px(15.))
    .font_weight(FontWeight::BOLD)
```

这些就是普通的 [`Styled`](https://docs.rs/gpui-pre/0.3.6/gpui/trait.Styled.html)
方法，与样式链的其余部分组合使用。

## 打包自定义字体

用户系统中没有的字体必须打包，并在**首帧之前**注册到文本系统：

```rust
use std::borrow::Cow;

cx.text_system()
    .add_fonts(vec![Cow::Borrowed(
        include_bytes!("../fonts/MyFont-Regular.ttf").as_slice(),
    )])
    .expect("Failed to load fonts");
```

之后照常用 family 名称引用：

```rust
Theme::update(cx, |theme| theme.font_family = "MyFont".into());
```

[GPUI Kit Web 画廊](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/src/lib.rs)就这样打包 `Inter`、`JetBrains Mono`、`Noto Sans SC` 子集和 `IBM Plex Sans`。Rust 的 [`include_bytes!`](https://doc.rust-lang.org/std/macro.include_bytes.html) 会把这些字体字节放入 WebAssembly 下载包。画廊所用的 CJK 子集约 25 KB，源字体约 1.2 MB：已知界面文案可以制作子集来控制初始体积，但用户任意输入的文字需要另行安排字体来源。分发字体文件时还要遵守相应许可。

## 主题 JSON 配置

字体与字号也可以来自主题文件：

```json
{
    "font.family": "Inter",
    "font.size": 16,
    "mono_font.family": "JetBrains Mono",
    "mono_font.size": 13
}
```

用 `ThemeRegistry` 加载：

```rust
ThemeRegistry::watch_dir(PathBuf::from("./themes"), cx, move |cx| {
    if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
        Theme::update(cx, |current| current.apply_config(&theme));
    }
});
```

完整配置说明参见 [Theme](../component/theme.md)。

## WebAssembly：选择字体来源

浏览器构建及字体初始化流程见 [WebAssembly 指南](./webassembly)。

GPUI 的 Web 文本系统**不会把浏览器已安装字体枚举、加载为主要字体集合**。必须在创建窗口或测量第一段文字前，注册所有需要稳定塑形的字体族，包括初始文本样式使用的字体。在这个 Web 平台上，`.SystemUIFont` 映射到 `IBM Plex Sans`；如果主题生效前可能使用这个别名，也要注册该字体。先注册字体，**之后**再应用或切换主题，并让主题的 `font_family` 与 `mono_font_family` 指向已加载的字体。主题文件若指定 Web 中不可用的桌面字体，字体解析可能失败。[画廊初始化代码](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/src/lib.rs)依次初始化 GPUI Kit、注册字体字节、应用主题，最后打开窗口；之后切换主题也会重新指定已加载的字体族。

常用的来源有三种：

| 选择 | 初始下载 | 覆盖与取舍 |
| --- | --- | --- |
| 通过 `include_bytes!` 打包完整字体 | WebAssembly 包体较大 | 可离线使用，对字体覆盖范围内的用户输入也有稳定效果。 |
| 只打包已知界面文字的子集 | 包体较小 | 其他 CJK 字符和新增输入需要其他来源。 |
| 需要时再请求字体文件 | 初始包体较小，稍后增加网络请求 | 运行时注册，并刷新窗口以重新塑形文字；需要处理加载和失败状态。 |

下载后的字体字节仍交给**同一个** `TextSystem::add_fonts(&self, Vec<Cow<'static, [u8]>>) -> Result<()>` API，但使用 owned 数据。HTTP 下载可以通过应用的 `cx.http_client()` 或其他客户端完成；注册步骤如下：

```rust
use std::borrow::Cow;
use gpui_kit::*;

fn install_downloaded_font(cx: &mut App, bytes: Vec<u8>) -> Result<()> {
    cx.text_system().add_fonts(vec![Cow::Owned(bytes)])?;
    cx.refresh_windows();
    Ok(())
}
```

`add_fonts` 会使字体解析和行布局缓存失效，但已经显示的窗口仍需 `refresh_windows()` 才能显示重新塑形后的文字。注册前应确认 HTTP 响应成功且内容非空。应提供文本系统支持的实际字体文件，例如原始 TTF；字体服务的 CSS 地址可能返回样式表或 WOFF2 子集，而不是此文本系统接受的字节。外部请求仍受浏览器跨源规则约束。只下载当前内容需要的字体，并对并发请求去重。

`App::on_missing_glyphs(callback) -> Subscription` 可以在塑形后报告仍无法解析的字素簇。保存 subscription，检查 `MissingGlyph::grapheme()` 与 `font_class()`，即可按文字系统请求一次相应字体。新注册会替换之前的 callback；报告有去重和队列上限，因此它适合作为加载提示，不保证是完整缺字清单。如果 Canvas fallback 已能绘制某个 CJK 字素，它**不会**触发缺字报告。需要准确 CJK 排版时，应根据用户选择的语言或已知内容覆盖范围主动加载，不能只依赖缺字事件。

## 浏览器 Canvas 回退

已加载字体无法绘制的文字，有时仍可由浏览器提供。Web 平台可以借助访问者本机字体，通过 Canvas 2D 绘制符合条件的 emoji，因此不必为它们打包字体。构造平台时选择回退策略，之后不能更改：

| `CanvasFontFallback` | 由浏览器绘制的内容 |
| --- | --- |
| `Emoji`（默认） | emoji，包括肤色、旗帜、键帽和 ZWJ 序列 |
| `EmojiAndCjk` | emoji，以及符合条件的横排汉字、假名、现代谚文和相关标点 |
| `Disabled` | 不回退，只使用打包字体 |

`gpui_kit::application()` 和 `gpui_kit::platform::single_threaded_web()`
沿用默认策略。要放宽范围，就自己构造平台：

```rust
use gpui_kit::web::{CanvasFontFallback, WebBackendPreference, WebPlatform};

let platform = Rc::new(WebPlatform::new_with_backend_and_font_fallback(
    false,
    WebBackendPreference::Auto,
    CanvasFontFallback::EmojiAndCjk,
));
let http_client = Arc::new(platform.fetch_http_client());
let app = Application::with_platform(platform).with_http_client(http_client);
```

已加载字体只要有对应字形和所需呈现形式，就仍然优先使用已加载字体。Canvas 回退只处理符合条件的**单个完整字素簇**；它不是通用系统字体 API，也不保证浏览器一定有相应字形。CJK 回退逐个独立绘制符合条件的横排字素，因此优先保证可读性，不保证精确的字距、塑形或字体特性；外观取决于访问者机器上的字体。需要准确度量、换行或视觉一致性时，应加载真正的 CJK 字体。画廊选择 `EmojiAndCjk`，因为打包的 CJK 子集覆盖已知文案，而访问者还可能在输入框中键入其他字符。
