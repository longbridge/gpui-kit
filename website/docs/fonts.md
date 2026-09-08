---
title: Fonts
description: System fonts, theme fonts, per-element overrides, and bundling custom fonts.
---

# Fonts

## Default fonts

Every app starts with a UI font and a monospace font from the theme:

| Role | Family | Size |
| --- | --- | --- |
| UI text | `.SystemUIFont` | 16px |
| Code / monospace | macOS: `Menlo`, Windows: `Consolas`, Linux: `DejaVu Sans Mono` | 13px |

The editor paints its code in `mono_font_family` at `mono_font_size`. See
[Editor](../component/editor.md) for details.

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

Set the app-wide fonts on the `Theme` global, then sync to the base layer:

```rust
Theme::global_mut(cx).font_family = "Inter".into();
Theme::global_mut(cx).mono_font_family = "JetBrains Mono".into();
Theme::global_mut(cx).font_size = px(18.);
Theme::sync_base(cx);
window.refresh();
```

`font_size` doubles as the application zoom control — `Root` calls
`window.set_rem_size(cx.theme().font_size)`, so `rem`-based spacing scales
with it. See [Coding Guides](./coding-guides.md) for details.

## Per-element override

Any element accepts a font override without touching the theme:

```rust
div()
    .font_family("JetBrains Mono")
    .text_size(px(15.))
    .font_weight(FontWeight::BOLD)
```

These are ordinary [`Styled`](https://docs.rs/gpui/latest/gpui/trait.Styled.html)
methods, so they compose with the rest of the style chain.

## Bundling custom fonts

Fonts that are not installed on the user's system must be bundled and
registered with the text system **before the first frame**:

```rust
cx.text_system()
    .add_fonts(vec![Cow::Borrowed(
        include_bytes!("../fonts/MyFont-Regular.ttf").as_slice(),
    )])
    .expect("Failed to load fonts");
```

Then reference them by family name as usual:

```rust
Theme::global_mut(cx).font_family = "MyFont".into();
Theme::sync_base(cx);
```

The gallery's web build bundles `Inter`, `JetBrains Mono`, `NotoSansSC` and
`NotoEmoji` this way — see `crates/story-web/src/lib.rs`.

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
        Theme::global_mut(cx).apply_config(&theme);
    }
});
```

See [Theme](../component/theme.md) for the full config reference.

## WebAssembly note

Browsers expose **no system fonts** to WASM apps. The `story-web` gallery
(which runs at `gpui-kit.com/gallery/`) must bundle every family it uses and
re-assert them after `Theme::change`, or the text system panics. Desktop apps
skip this entirely.
