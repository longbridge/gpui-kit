---
title: Fonts
order: -8
description: System fonts, theme fonts, per-element overrides, and bundling custom fonts.
---

# Fonts

This page covers which fonts an application supplies. See [TextSystem](./text-system) for how GPUI resolves, shapes, measures, and paints their glyphs.

## Default fonts

Every app starts with a UI font and a monospace font from the theme:

| Role | Family | Size |
| --- | --- | --- |
| UI text | `.SystemUIFont` | 16px |
| Code / monospace | macOS: `Menlo`, Windows: `Consolas`, Linux: `DejaVu Sans Mono` | 13px |

The editor paints its code in `mono_font_family` at `mono_font_size`. See
[Editor](../component/editor.md) for details.

Both defaults are checked against the installed fonts when the theme is
applied. A missing monospace default is swapped for an installed alternative,
and when `.SystemUIFont` resolves to one of GPUI's fallback families rather
than the system font itself (Linux desktops without the family GPUI maps it
to), the theme names that family directly so text lookups stay cached. A
family you set yourself is used as-is.

## System fonts

Desktop apps can use **any font installed on the OS** by name — no bundling,
no config. GPUI resolves the family live against the system collection
(CoreText on macOS, DirectWrite on Windows, fontconfig on Linux).

```rust
div().font_family("Segoe UI")

Editor::new(&editor).font_family("JetBrains Mono")
```

Common examples per platform:

- macOS: `SF Pro`, `Helvetica`, `Arial`, `Times New Roman`, `Menlo`, `Monaco`
- Windows: `Segoe UI`, `Arial`, `Consolas`, `Courier New`
- Linux: `Noto Sans`, `DejaVu Sans`, `Liberation Sans`, `DejaVu Sans Mono`

If the name does not match an installed font, GPUI falls back silently — so
verify the exact family name on each target platform.

## Changing fonts via Theme

Set the app-wide fonts through `Theme::update`, which syncs the base layer and refreshes every window:

```rust
Theme::update(cx, |theme| {
    theme.font_family = "Inter".into();
    theme.mono_font_family = "JetBrains Mono".into();
    theme.font_size = px(18.);
});
```

`font_size` doubles as the application zoom control — `Root` calls
`window.set_rem_size(cx.theme().font_size)`, so [`rem`-based spacing](./geometry)
scales with it. See [Coding Guides](./coding-guides.md) for details.

## Per-element override

Any element accepts a font override without touching the theme:

```rust
div()
    .font_family("JetBrains Mono")
    .text_size(px(15.))
    .font_weight(FontWeight::BOLD)
```

These are ordinary [`Styled`](https://docs.rs/gpui-pre/0.3.6/gpui/trait.Styled.html)
methods, so they compose with the rest of the style chain.

## Bundling custom fonts

Fonts that are not installed on the user's system must be bundled and
registered with the text system **before the first frame**:

```rust
use std::borrow::Cow;

cx.text_system()
    .add_fonts(vec![Cow::Borrowed(
        include_bytes!("../fonts/MyFont-Regular.ttf").as_slice(),
    )])
    .expect("Failed to load fonts");
```

Then reference them by family name as usual:

```rust
Theme::update(cx, |theme| theme.font_family = "MyFont".into());
```

The [GPUI Kit web gallery](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/src/lib.rs) bundles `Inter`, `JetBrains Mono`, a subset of `Noto Sans SC`, and `IBM Plex Sans` this way. Rust's [`include_bytes!`](https://doc.rust-lang.org/std/macro.include_bytes.html) puts those font bytes into the WebAssembly download. The gallery's CJK subset is about 25 KB, compared with about 1.2 MB for its source font: subset known interface copy to limit initial payload, then plan separately for arbitrary text entered by users. Font files also need their redistribution licenses.

## Theme JSON config

Font families and sizes can also come from a theme file:

```json
{
    "font.family": "Inter",
    "font.size": 16,
    "mono_font.family": "JetBrains Mono",
    "mono_font.size": 13
}
```

Load it with `ThemeRegistry`:

```rust
ThemeRegistry::watch_dir(PathBuf::from("./themes"), cx, move |cx| {
    if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
        Theme::update(cx, |current| current.apply_config(&theme));
    }
});
```

See [Theme](../component/theme.md) for the full config reference.

## WebAssembly: choose a font supply strategy

See the [WebAssembly guide](./webassembly) for the browser build and its font setup.

GPUI's Web text system does **not enumerate or load the browser's installed fonts** as its main font collection. Register every family that must be shaped reliably, including the family used by the initial text style, before creating a window or measuring its first text. On this Web platform, `.SystemUIFont` maps to `IBM Plex Sans`; if that alias can be used before the theme applies, register that family too. Apply or change the theme **after** registration and keep its `font_family` and `mono_font_family` pointed at loaded families. A theme file naming an unavailable desktop font can otherwise fail font resolution. The [gallery initialization](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/src/lib.rs) follows this order: initialize GPUI Kit, register font bytes, apply the theme, then open the window; later theme changes reassert its loaded families.

There are three useful supply choices:

| Choice | Initial download | Coverage and tradeoff |
| --- | --- | --- |
| Bundle full fonts with `include_bytes!` | Larger WebAssembly payload | Offline and predictable, including user-entered text within the font's coverage. |
| Bundle subsets for known UI strings | Smaller payload | Other CJK characters and newly entered text need another source. |
| Fetch a font file when needed | Smaller initial payload; later network request | Install it at runtime, then refresh windows so text is shaped again. Handle loading and failure states. |

For a fetched font, pass owned bytes to the **same** `TextSystem::add_fonts` API. The HTTP download can come from the application's `cx.http_client()` or another client; the registration step is:

```rust
use std::borrow::Cow;
use gpui_kit::*;

fn install_downloaded_font(cx: &mut App, bytes: Vec<u8>) -> Result<()> {
    cx.text_system().add_fonts(vec![Cow::Owned(bytes)])?;
    cx.refresh_windows();
    Ok(())
}
```

`add_fonts` invalidates font resolution and line-layout caches, but an already visible window needs `refresh_windows()` to show the newly shaped text. Validate the HTTP response and nonempty bytes before installing. Supply an actual supported font file such as raw TTF; a font-service CSS URL may return a stylesheet or WOFF2 subset rather than bytes this text system accepts. Browser cross-origin rules apply to external requests. Download only fonts the current content needs, and deduplicate concurrent requests.

`App::on_missing_glyphs(callback) -> Subscription` can report unresolved grapheme clusters after shaping. Keep the subscription alive, inspect `MissingGlyph::grapheme()` and `font_class()`, and use it to request a script-specific font once. A new registration replaces the previous callback; reports are deduplicated and bounded, so this is a loading hint rather than a guaranteed complete inventory. If Canvas fallback can already draw a CJK grapheme, that grapheme will **not** produce a missing-glyph report. For accurate CJK typography, trigger loading from the selected language or known content coverage instead of relying only on missing-glyph reports.

## Browser Canvas fallback

Text the loaded fonts cannot draw can sometimes come from the browser. The Web platform can render eligible emoji through Canvas 2D with the visitor's local fonts, avoiding a bundled emoji font. Choose the policy when constructing the platform; it cannot change afterwards:

| `CanvasFontFallback` | Browser draws |
| --- | --- |
| `Emoji` (default) | Emoji, including skin tones, flags, keycaps and ZWJ sequences |
| `EmojiAndCjk` | Emoji plus eligible horizontal Han, kana, modern Hangul, and related punctuation |
| `Disabled` | Nothing; only bundled fonts are used |

`gpui_kit::application()` and `gpui_kit::platform::single_threaded_web()` keep
the default. To widen it, build the platform yourself:

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

Loaded fonts stay preferred wherever they have the glyph and requested presentation. Canvas fallback is limited to eligible **single complete graphemes**; it is not a general system-font API or a guarantee that the browser has a matching glyph. CJK fallback draws eligible graphemes independently and horizontally, so it favors readable coverage over exact spacing, shaping, and font features. Its appearance depends on the visitor's fonts. Use a real CJK font for text whose metrics, line breaks, or visual consistency matter. The gallery opts into `EmojiAndCjk` because its bundled CJK subset covers known gallery copy while visitors can type other characters into inputs.
