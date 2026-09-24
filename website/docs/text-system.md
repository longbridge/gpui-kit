---
title: TextSystem
description: Shape, measure, lay out, and paint text through GPUI's text system and GPUI Kit components.
order: -2.76
---

# TextSystem

GPUI's `TextSystem` resolves [fonts](./fonts) and supplies font metrics. Each [Window](./window) has a `WindowTextSystem` that adds a line-layout cache to the shared text system. Ordinary text elements and GPUI Kit controls use these services for you. Reach for `window.text_system()` when writing custom text geometry, a chart label, an editor, or another element that must use shaped glyph positions directly.

Text rendering is a pipeline:

1. Resolve a `Font` to a `FontId`, including fallback when the requested family is unavailable.
2. Shape UTF-8 text and styled `TextRun`s into positioned glyph runs and line metrics.
3. Wrap and measure lines against available width where needed.
4. Use the layout for hit testing and paint the shaped glyphs in the window.

Shaping is necessary because the width of a string is not generally the sum of independent character widths. Script shaping, ligatures, kerning, font fallback, and emoji can change glyph count, positions, and advances. Measure the **actual shaped text** for a custom label rather than multiplying a character count by an average width.

## Start with an element

For interface copy, use a normal text child and let GPUI own layout and painting:

```rust
use gpui_kit::*;

div()
    .font_family(".SystemUIFont")
    .text_size(px(14.))
    .child("Recent activity")
```

GPUI Kit's [TextView](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/text/compat.rs) renders Markdown or HTML with styled runs, selection, and optional scrolling; it delegates parsing, layout, and selection behavior to Base. Use `TextView::markdown("article", source)` for a rich document. [Input](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) shapes visible lines in [`prepaint`](./element#prepaint), where the resolved width and input geometry are known; its caret and pointer mapping use the same shaped layout. A plain label needs neither implementation.

## Font resolution and metrics

`Font` names a family and carries weight, style, features, and optional fallbacks. `font("Family")` constructs one; `Font::default()` requests `.SystemUIFont`. `cx.text_system().resolve_font(&font)` returns a `FontId`, trying GPUI's fallback stack if the requested family cannot load. If no fallback resolves, it panics. `all_font_names()` lists available families, including fonts added by `add_fonts(...)`. Add bundled fonts before the first frame so the first layout uses the intended metrics; adding fonts later invalidates cached font resolution and line layouts, while a layout already underway may finish against the earlier set.

GPUI exposes `ascent`, `descent`, `cap_height`, `x_height`, `bounding_box`, `advance`, and `typographic_bounds` for a resolved font and `Pixels` size. These answer different questions. `advance(font_id, size, ch)` returns pen movement for one character's glyph in that font; `typographic_bounds(font_id, size, ch)` describes that glyph's typographic rectangle; ascent and descent establish vertical metrics. Neither API shapes a string or applies its font fallback. `WindowTextSystem::layout_width(font_id, size, ch)` shapes one character; use it for a specific cell or space measurement, not for a sentence.

The displayed result depends on installed families and platform font fallback. Desktop GPUI resolves against the operating system's font collection; a requested family may resolve to a different available family. A web build cannot assume desktop system families are exposed and must register the fonts it requires. Test representative Latin, CJK, emoji, and mixed-script strings on target platforms when line breaks or alignment are important.

## Shape one line

`TextRun::len` is a **UTF-8 byte length**, and all runs together should cover the text they style. A run selects the font, color, background, underline, and strikethrough for its byte range. The text argument is a [SharedString](./shared-string). This example follows the [GPUI Kit Plot label](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/plot/label.rs):

```rust
use gpui_kit::*;

fn shape_label(text: SharedString, color: Hsla, window: &Window) -> ShapedLine {
    let run = TextRun {
        len: text.len(),
        font: window.text_style().font(),
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };

    window.text_system().shape_line(text, px(14.), &[run], None)
}

let width = shape_label(label, color, window).width();
```

`shape_line(text, font_size, runs, force_width)` returns a `ShapedLine`: its `width()` is the shaped advance, and it carries the original text, positioned glyphs, font IDs, ascent, descent, and decoration runs. Pass `None` for `force_width` unless a custom layout intentionally supplies a width. `shape_line` is for **one** line; do not pass text with `\n`. `layout_line(&str, size, runs, force_width)` returns an `Arc<LineLayout>` when geometry is enough, but `shape_line` is the direct choice if it will be painted.

`LineLayout::x_for_index(byte_index)`, `index_for_x(x)`, and `closest_index_for_x(x)` support cursor and hit-test calculations. Indices refer to UTF-8 bytes in the original text, not Unicode scalar values or visual columns. Their x positions use GPUI's [logical pixel geometry](./geometry). Keep selection boundaries on valid text boundaries, and use the same shaped layout for measurement, caret placement, and painting so they agree.

## Wrap multiple lines

Use `window.text_system().shape_text(text, font_size, runs, wrap_width, line_clamp)` for newlines and optional soft wrapping. It returns a `Result` of `WrappedLine`s. `wrap_width: Some(width)` sets the available width. `line_clamp` limits soft-wrap boundaries, but `shape_text` still returns a `WrappedLine` for every explicit newline-separated source line; it does not guarantee at most N lines in total. The line layout contains wrap boundaries and widths, so a custom text element can place each visual line and map pointer positions back to source text. A width or font change can alter those boundaries; recompute layout when either changes.

For ordinary paragraphs, let a GPUI text element or GPUI Kit `TextView` perform this work. A custom element should only shape text directly when it needs glyph-aware placement, drawing, or hit testing that existing elements cannot supply.

## Match GPUI's rendering phases

GPUI's [rendering pipeline](./render) separates layout, prepaint, and paint. Place custom text work in the matching phase:

| Phase | Text work | Why |
| --- | --- | --- |
| `request_layout` | Declare style and layout nodes; measure only what is needed to request size. | Final `Bounds<Pixels>` are not available yet. |
| `prepaint` | Shape against resolved width, calculate line origins and cursor geometry, establish hitboxes. | Layout has produced bounds; input geometry must match this frame. |
| `paint` | Paint the prepared `ShapedLine`s at their origins. | Reusing prepaint's shape keeps pixels and hit tests aligned. |

`ShapedLine::paint(origin, line_height, align, align_width, window, cx)` submits a line; `paint_background(...)` is separate when a run has a background. Handle its `Result`. The [Input element](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) applies this phase split to visible text, line numbers, selections, and carets. Text painting does not itself establish a hitbox, keyboard focus, or accessibility name; a custom interactive text element must supply those contracts too. See [Paint](./paint) for custom drawing.

## Cache and performance boundaries

`WindowTextSystem` caches line layouts across the current and previous frame and shares font resolution and metrics through `TextSystem`. The shared system also caches glyph raster bounds; painting uses glyph rendering parameters that include font, size, scale factor, and subpixel position. Repeated identical text, font runs, and size can reuse shaping results; changing text, font, features, size, or wrapping width may require new layout. For a known set of fonts, `TextSystem::prewarm_fonts(&fonts)` can prepare platform font caches on a background executor; ordinary shaping fills missing entries on demand. These caches are a reason to reuse the window's text system, not to build a separate shaper or retain frame-local `&mut Window` references.

For large virtualized editors or documents, shape only visible content and avoid measuring every row on every redraw. GPUI Kit's [TextView](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/text/text_view.rs) can virtualize scrollable blocks, while [Input](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) shapes its visible lines. If a profiler shows repeated materialization of long line text, `try_layout_line_by_hash` probes the cache without building a contiguous string, and `shape_line_by_hash` builds one only on a miss. With the latter API, `ShapedLine.text` is an empty placeholder even on a miss; keep the original text separately if needed. The caller must guarantee that the same hash implies identical text, and pass its UTF-8 byte length as `text_len`. Prefer the ordinary API until that cost is measured.
