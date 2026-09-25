---
title: Fonts
order: -8
description: 系统字体、主题字体、元素级覆盖与自定义字体打包。
---

# Fonts

本文说明应用应提供哪些字体。GPUI 如何解析、塑形、测量并绘制字形，见 [TextSystem](./text-system)。

## 从这里开始

桌面应用可以先使用主题默认字体。如果需要指定外观，就选目标机器都已安装的字体族，或把字体文件打包进应用。通过 `Theme::update` 设置 UI 与等宽字体；只调整某个元素时用 `.font_family(...)`。在各目标平台上用拉丁文、CJK、emoji 和混排内容检查结果。仅有字体族名称，并不代表它包含这些字形。

WebAssembly 应用要在创建首个窗口或测量文字之前注册字体文件。浏览器已安装的字体列表不是 GPUI Web 的字体集合。下文“WebAssembly：选择字体来源”说明浏览器构建所需的额外选择。

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

请求的字体族无法加载时，`TextSystem::resolve_font` 会尝试 GPUI 的默认回退栈；如果全都无法加载，排版时会 panic。字体族能加载但缺少某个字形，是另一种情况：字形回退可能用其他字体绘制该字符。应在每个目标平台上同时确认字体族名称和所需字形。`Font::fallbacks` 用于主字体加载成功之后的缺字回退，不能使不存在的主字体变得可用。

在初始化完成、打包字体注册之后，可以查看 GPUI 当前能找到的字体族：

```rust
let families = cx.text_system().all_font_names();
println!("Available font families: {families:?}");
```

列表包含通过 `add_fonts` 注册的字体族，但不能证明某个字重或每个字符都得到支持。

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

实现了 `Styled` 的元素可以在不改动主题的情况下覆盖字体：

```rust
div()
    .font_family("JetBrains Mono")
    .text_size(px(15.))
    .font_weight(FontWeight::BOLD)
```

这些就是普通的 [`Styled`](https://docs.rs/gpui-pre/0.3.6/gpui/trait.Styled.html)
方法，与样式链的其余部分组合使用。

## 打包自定义字体

用户系统中没有的字体必须打包，并在**首帧之前**注册到文本系统。把字体文件放进应用，在启动时调用 `add_fonts`，顺序要早于打开窗口或构造会测量文字的对象：

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

这里要写字体文件**内部**的 family 名称，它可能与文件名不同。若要稳定显示常规、粗体和斜体，应注册所需的各个字体文件；单个常规字体文件不保证所有样式。打包字体也能让桌面应用不依赖用户是否已安装该字体族。分发时要保留相应的字体许可。

[GPUI Kit Web 画廊](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/src/lib.rs)就这样打包 `Inter Variable`、`JetBrains Mono`、`Noto Sans SC` 子集和 `IBM Plex Sans`。Rust 的 [`include_bytes!`](https://doc.rust-lang.org/std/macro.include_bytes.html) 会把这些字体字节放入 WebAssembly 下载包。画廊所用的 CJK 子集约 25 KB，源字体约 1.2 MB：已知界面文案可以制作子集来控制初始体积，但用户任意输入的文字需要另行安排字体来源。

### 跟做：在 `hello_world` 中注册打包字体

仓库已有 `crates/story-web/fonts/Inter-Regular.ttf`，它在文件内部的 family 名称是 `Inter Variable`。将 `examples/hello_world/src/main.rs` 替换为下面的完整示例，然后在仓库根目录运行 `cargo run -p hello_world`。下方 `include_bytes!` 的路径相对于这个 `main.rs` 文件。自己的应用应把许可允许分发的字体放在自己的资源目录，并相应调整路径。

```rust
use std::borrow::Cow;

use gpui_kit::component::theme::Theme;
use gpui_kit::*;

struct FontLab;

impl Render for FontLab {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .child("Theme font: Inter Variable")
            .child(
                div()
                    .font_family(".SystemUIFont")
                    .child("System UI font for comparison"),
            )
    }
}

fn main() {
    application().run(|cx| {
        init(cx);
        cx.text_system()
            .add_fonts(vec![Cow::Borrowed(
                include_bytes!("../../../crates/story-web/fonts/Inter-Regular.ttf").as_slice(),
            )])
            .expect("Failed to load bundled font");

        let families = cx.text_system().all_font_names();
        assert!(families.iter().any(|family| family == "Inter Variable"));
        println!("Registered font: Inter Variable");

        Theme::update(cx, |theme| theme.font_family = "Inter Variable".into());
        open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| FontLab))
            .expect("Failed to open window");
    });
}
```

终端应输出 `Registered font: Inter Variable`，窗口应显示两行标签。第一行继承主题中刚注册的字体；第二行明确请求系统 UI 字体。两者外观差异取决于平台。此练习验证注册与主题选择，并不能证明字体覆盖所有字形；使用用户输入前，还要换成代表性文本并在目标平台检查。字体注册放在 `Theme::update` 与 `open_window` 之前，让首次布局使用新的字体。

## 更换字体会改变布局

不同字体的字形宽度、上升部、下降部和字符覆盖范围各不相同。即使字号相同，回退字体或新加载的 CJK 字体也可能改变换行、控件高度、光标位置和对齐。主题更改会刷新窗口；`add_fonts` 会使字体解析和已缓存的行布局失效。如果窗口已经显示，注册之后还要调用 `cx.refresh_windows()`。在新一帧重新检查文字，尤其是窄控件和多文字体系混排的段落。自行测量时，应使用 [TextSystem](./text-system) 塑形后的行，而不是按字符数估算宽度。

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

在桌面应用的启动回调中，先执行 `init(cx)`，再选择主题名称并监听存有主题文件的目录。下面是需要放入该回调的片段：`cx` 来自启动回调，`"My Theme"` 必须与 `./themes` 内某个主题文件中的名称一致。

```rust
use std::path::PathBuf;
use gpui_kit::component::theme::{Theme, ThemeRegistry};
use gpui_kit::SharedString;

let theme_name: SharedString = "My Theme".into();
ThemeRegistry::watch_dir(PathBuf::from("./themes"), cx, move |cx| {
    if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
        Theme::update(cx, |current| current.apply_config(&theme));
    }
})
.expect("Failed to watch theme directory");
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

下载后的字体字节仍交给**同一个** `TextSystem::add_fonts` API，但使用 owned 数据。HTTP 下载可以通过应用的 `cx.http_client()` 或其他客户端完成；注册步骤如下：

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

## 排查字体问题

| 现象 | 检查方法 |
| --- | --- |
| 首次排版时程序 panic | 打开窗口之前查看 `all_font_names()`。确认主字体族和可用的默认字体族已注册；Web 端尤其要检查 `.SystemUIFont` 的映射字体。 |
| 文字显示成意外的字体 | 对照请求的字体族、`all_font_names()` 和文件内部的 family 名称，并检查切换主题后是否覆盖了设置。 |
| CJK 出现方框或字体混杂 | 确认字体包含这些**具体字符**。子集可能覆盖界面标签却漏掉用户输入；此时要加载覆盖更广的字体或相应文字体系的字体。 |
| 加载字体后换行发生变化 | 用新字体的度量重新检查布局；如果窗口已显示，`add_fonts` 后刷新窗口。 |
| Web 端缺字回调没有触发 | 检查 Canvas 回退是否已绘制该字素。需要统一排版时，根据内容或语言选择主动加载字体。 |

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
