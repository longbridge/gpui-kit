---
title: Fonts
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

在 `Theme` 全局量上设置应用级字体，然后同步到底层：

```rust
Theme::global_mut(cx).font_family = "Inter".into();
Theme::global_mut(cx).mono_font_family = "JetBrains Mono".into();
Theme::global_mut(cx).font_size = px(18.);
Theme::sync_base(cx);
window.refresh();
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
Theme::global_mut(cx).font_family = "MyFont".into();
Theme::sync_base(cx);
```

Web 版画廊就是这样打包 `Inter`、`JetBrains Mono`、`NotoSansSC` 和
`NotoEmoji` 的，参见 `crates/story-web/src/lib.rs`。

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
        Theme::global_mut(cx).apply_config(&theme);
    }
});
```

完整配置说明参见 [Theme](../component/theme.md)。

## WebAssembly 说明

浏览器不会向 WASM 应用暴露系统字体。在 `gpui-kit.com/gallery/` 运行的
`story-web` 画廊必须打包它用到的每一种字体，并在 `Theme::change` 之后重新
声明，否则文本系统会 panic。桌面应用完全不需要这一步。
