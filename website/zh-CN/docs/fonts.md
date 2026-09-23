---
title: Fonts
order: -8
description: 系统字体、主题字体、元素级覆盖与自定义字体打包。
---

# Fonts

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
`window.set_rem_size(cx.theme().font_size)`，因此基于 `rem` 的间距会跟随缩放。详见[编码指南](./coding-guides.md)。

## 元素级覆盖

任何元素都可以在不改动主题的情况下覆盖字体：

```rust
div()
    .font_family("JetBrains Mono")
    .text_size(px(15.))
    .font_weight(FontWeight::BOLD)
```

这些就是普通的 [`Styled`](https://docs.rs/gpui/latest/gpui/trait.Styled.html)
方法，与样式链的其余部分组合使用。

## 打包自定义字体

用户系统中没有的字体必须打包，并在**首帧之前**注册到文本系统：

```rust
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

Web 版画廊就是这样打包 `Inter`、`JetBrains Mono`、`Noto Sans SC` 和
`IBM Plex Sans` 的，参见 `crates/story-web/src/lib.rs`。

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

## WebAssembly 说明

浏览器不会向 WASM 应用暴露系统字体。在 `gpui-kit.com/gallery/` 运行的
`story-web` 画廊必须打包它用到的每一种字体，并在 `Theme::change` 之后重新
声明，否则文本系统会 panic。桌面应用完全不需要这一步。

打包字体画不出来的文字仍然可以交给浏览器绘制。Web 平台会用 Canvas 2D
和访问者本机的字体渲染 emoji，应用不必再打包 emoji 字体。回退策略在构造
平台时选定，之后不能更改：

| `CanvasFontFallback` | 由浏览器绘制的内容 |
| --- | --- |
| `Emoji`（默认） | emoji，包括肤色、旗帜、键帽和 ZWJ 序列 |
| `EmojiAndCjk` | emoji，外加横排的汉字、假名和现代谚文 |
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

只要打包字体里有对应字形，就仍然优先使用打包字体。回退是逐个字素独立绘制的，
所以这样渲染的 CJK 文字以可读为先，不保证精确的间距和字体特性，外观也取决于
访问者机器上安装的字体。画廊选择了 `EmojiAndCjk`：它打包的字体只包含
故事本身用到的字形，访问者在输入框里键入的其他文字否则都会显示成方块。
