---
title: TextSystem
description: Shape, measure, lay out, and paint text through GPUI's text system and GPUI Kit components.
order: -2.76
---

# TextSystem

This page follows the `gpui-pre` {{gpui_pre_version}} API pinned by this repository. `gpui-pre` is the published snapshot and version-alignment mechanism for GPUI, not a separate text engine. Start with a text element; use `TextSystem` directly when your element needs the glyph geometry that GPUI normally manages.

GPUI's `TextSystem` resolves [fonts](./fonts) and supplies font metrics. Each [Window](./window) has a `WindowTextSystem` that adds a line-layout cache to the shared text system. Ordinary text elements and GPUI Kit controls use these services for you. Reach for `window.text_system()` when writing custom text geometry, a chart label, an editor, or another element that must use shaped glyph positions directly.

Text rendering is a pipeline:

1. Resolve a `Font` to a `FontId`, including fallback when the requested family is unavailable.
2. Shape UTF-8 text and styled `TextRun`s into positioned glyph runs and line metrics.
3. Wrap and measure lines against available width where needed.
4. Use the layout for hit testing and paint the shaped glyphs in the window.

Shaping is necessary because the width of a string is not generally the sum of independent character widths. Script shaping, ligatures, kerning, font fallback, and emoji can change glyph count, positions, and advances. Measure the **actual shaped text** for a custom label rather than multiplying a character count by an average width.

## Follow a line from text to pixels

Use the existing `hello_world` example for a short experiment. Replace `examples/hello_world/src/main.rs` with the code below, then run `cargo run -p hello_world` from the repository root. It uses the normal text element for the visible label and asks the same window text system for that line's shaped advance. The measurement is displayed so you can change the sample and see the result; in an application, keep expensive intrinsic measurements in layout instead of recomputing them merely to print a number in `render`.

```rust
use gpui_kit::*;

struct TextLab;

impl Render for TextLab {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let label: SharedString = "Office fi café 👋".into();
        let font_size = px(18.);
        let run = TextRun {
            len: label.len(), // UTF-8 bytes, not character count.
            font: window.text_style().font(),
            color: window.text_style().color,
            ..Default::default()
        };
        let line = window
            .text_system()
            .shape_line(label.clone(), font_size, &[run], None);

        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .text_size(font_size)
            .child(label)
            .child(format!("Shaped advance: {:.1} logical px", line.width().as_f32()))
    }
}

fn main() {
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);
        open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| TextLab))
            .expect("Failed to open window");
    });
}
```

The window should show the sample on one line and a second line beginning `Shaped advance:`. The number is a **logical pixel width of the shaped first line**, not a frame time or the width of the enclosing `div`; its exact value depends on the resolved font and platform. Change the sample to `"iiii"`, then `"WWWW"`, and rerun: the advances should differ. Change it to `"café 👋"` and observe that `label.len()` counts more bytes than displayed characters. Do not add `\n` to this single-line call; the multi-line API is explained below.

### Paint the shaped line yourself

The first exercise measures text in `render` so the result can be printed. To see the low-level drawing path, replace the same `examples/hello_world/src/main.rs` file with this complete program and run `cargo run -p hello_world` again. `canvas` supplies a prepaint callback and a paint callback without requiring a new `Element` implementation:

```rust
use gpui_kit::*;

struct CanvasTextLab;

impl Render for CanvasTextLab {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let sample: SharedString = "Office fi café 👋".into();
        let canvas_text = sample.clone();
        let font = window.text_style().font();
        let color = window.text_style().color;
        let font_size = px(18.);
        let line_height = px(24.);

        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .text_size(font_size)
            .child("Normal text element:")
            .child(sample)
            .child("Shaped and painted in a canvas:")
            .child(
                canvas(
                    move |_, window, _| {
                        let run = TextRun {
                            len: canvas_text.len(),
                            font,
                            color,
                            ..Default::default()
                        };
                        window
                            .text_system()
                            .shape_line(canvas_text, font_size, &[run], None)
                    },
                    move |bounds, line, window, cx| {
                        let advance = line.width();
                        line.paint(bounds.origin, line_height, TextAlign::Left, None, window, cx)
                            .expect("Failed to paint shaped text");

                        let marker = Bounds {
                            origin: point(bounds.origin.x + advance, bounds.origin.y),
                            size: size(px(1.), line_height),
                        };
                        window.paint_quad(fill(marker, color));
                    },
                )
                .w(px(320.))
                .h(line_height),
            )
    }
}

fn main() {
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);
        open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| CanvasTextLab))
            .expect("Failed to open window");
    });
}
```

The canvas draws the same sample beneath the ordinary text element. Its thin vertical marker sits at the line's **advance**, which is the pen position after shaping, not necessarily the last glyph's visible edge. `canvas` requests the fixed `320 × 24` area during layout. Once bounds are known, its prepaint callback shapes the line and returns a `ShapedLine`; its paint callback uses that same value for both `width()` and `paint(...)`. Change the sample in the code, rerun, and watch the marker move. A very long sample can run past the fixed canvas width; this exercise does not wrap or clip it.

If the ordinary line appears but the canvas line does not, check the canvas height, the `TextRun::len` byte count, and the result of `line.paint`. The canvas line has no text-node accessibility identity, selection, hitbox, or keyboard behavior. Use the ordinary text element for interface copy. When text width must determine the custom element's **requested size**, shape in a measured-layout callback instead of this fixed-size canvas; [Element](./element) explains that layout boundary.

This is the progression to use in a real view: ordinary label → only if needed, shaped width for intrinsic layout → only if needed, reuse shaped glyphs in a custom element's paint phase. The [Element](./element) tutorial supplies the full lifecycle for an element with its own layout or children. You do not need direct `TextSystem` calls for a label, button, or ordinary paragraph.

## Start with a text element

For interface copy, use a normal text child and let GPUI own layout and painting:

```rust
use gpui_kit::*;

div()
    .font_family(".SystemUIFont")
    .text_size(px(14.))
    .child(text!("Recent activity"))
```

`text!(...)` creates a `Text` element with an ID based on this macro call's source location. That ID lets GPUI expose a label to the accessibility tree and report content changes under a stable identity. `text!(id = "activity-label", value)` sets an explicit ID when the same call site creates multiple labels; each simultaneously rendered element needs a distinct ID. A plain `.child("Recent activity")` also lays out and paints text, but has no text-node ID. Neither form returns a measured width from `render`: GPUI shapes it during the element's measured-layout callback, places its bounds in prepaint, then paints it.

GPUI Kit's [TextView](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/text/compat.rs) renders Markdown or HTML with styled runs, selection, and optional scrolling; it delegates parsing, layout, and selection behavior to Base. Use `TextView::markdown("article", source)` for a rich document. [Input](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) shapes visible lines in [`prepaint`](./element#prepaint), where the resolved width and input geometry are known; its caret and pointer mapping use the same shaped layout. A plain label needs neither implementation.

## Font resolution and metrics

`Font` names a family and carries weight, style, features, and optional fallbacks. `font("Family")` constructs one; `Font::default()` requests `.SystemUIFont`. `cx.text_system().resolve_font(&font)` returns a `FontId`, trying GPUI's fallback stack if the requested family cannot load. If no fallback resolves, it panics. `all_font_names()` lists available families, including fonts added by `add_fonts(...)`. Add bundled fonts before the first frame so the first layout uses the intended metrics; adding fonts later invalidates cached font resolution and line layouts, while a layout already underway may finish against the earlier set.

GPUI exposes `ascent`, `descent`, `cap_height`, `x_height`, `bounding_box`, `advance`, and `typographic_bounds` for a resolved font and `Pixels` size. These answer different questions. `advance(font_id, size, ch)` returns pen movement for one character's glyph in that font; `typographic_bounds(font_id, size, ch)` describes that glyph's typographic rectangle; ascent and descent establish vertical metrics. Neither API shapes a string or applies its font fallback. `WindowTextSystem::layout_width(font_id, size, ch)` shapes one character; use it for a specific cell or space measurement, not for a sentence.

The displayed result depends on installed families and platform font fallback. Desktop GPUI resolves against the operating system's font collection; a requested family may resolve to a different available family. A web build cannot assume desktop system families are exposed and must register the fonts it requires. Test representative Latin, CJK, emoji, and mixed-script strings on target platforms when line breaks or alignment are important.

## Shape one line

`TextRun::len` is a **UTF-8 byte length**, and all runs together should cover the text they style, including newline bytes for `shape_text`. Split runs only at valid UTF-8 boundaries. A run selects the font, color, background, underline, and strikethrough for its byte range. The text argument is a [SharedString](./shared-string). This example follows [Plot label](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/plot/label.rs):

```rust
use gpui_kit::*;

fn shape_label(text: SharedString, color: Hsla, window: &mut Window) -> ShapedLine {
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

// Inside an existing render, measured-layout, or paint method with `window`:
let width = shape_label("Recent activity".into(), color, window).width();
```

The helper is a copyable pattern inside an existing GPUI view or element, where `window` and `color` are already available; it is not a standalone `main`. The [canvas exercise above](#paint-the-shaped-line-yourself) gives a complete application that shapes and paints a line. For a production example, [`crates/component/src/plot/label.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/plot/label.rs) uses this shape path in `measure_text_width` and calls `ShapedLine::paint(origin, line_height, align, align_width, window, cx)` in `PlotLabel::paint`.

`shape_line(text, font_size, runs, force_width)` returns a `ShapedLine`: its `width()` is the shaped advance, and it carries the original text, positioned glyphs, font IDs, ascent, descent, and decoration runs. Pass `None` for `force_width` unless a custom layout intentionally supplies a width. `shape_line` is for **one** line; do not pass text with `\n`. `layout_line(&str, size, runs, force_width)` returns an `Arc<LineLayout>` when geometry is enough, but `shape_line` is the direct choice if it will be painted.

An advance is the distance to move the text pen after the line; it is not necessarily the rectangle of colored pixels. A glyph can overhang its advance, and `font_size` alone does not determine a row's height. Use the line's ascent/descent and your chosen line height for vertical placement. If you need a box for clipping or hit testing, derive it from the actual placed line and the interaction area, not by treating `width()` as an ink bounding box.

`LineLayout::x_for_index(byte_index)`, `index_for_x(x)`, and `closest_index_for_x(x)` support cursor and hit-test calculations. Indices refer to UTF-8 bytes in the original text, not Unicode scalar values or visual columns. `x_for_index` yields a line-local x position; `index_for_x` returns `None` at or beyond the line width, while `closest_index_for_x` chooses an insertion boundary. To draw a caret, add the painted line origin to its local x coordinate. To handle a pointer, subtract that same origin before querying the layout. Their positions use GPUI's [logical pixel geometry](./geometry). Keep selection boundaries on valid text boundaries; a glyph or ligature need not correspond to exactly one character. Use the same shape, font size, line height, and alignment for measurement, caret placement, and painting so they agree.

## Wrap multiple lines

Use `shape_text(text, font_size, runs, wrap_width, line_clamp)` for newlines and optional soft wrapping. It returns `Result<SmallVec<[WrappedLine; 1]>>`. `wrap_width: Some(width)` sets the available width. Each `WrappedLine` represents one source line between explicit `\n` characters; it can contain several visual rows. `WrappedLine::size(line_height)` includes those rows, and `position_for_index(index, line_height)` or `closest_index_for_position(local_point, line_height)` maps between local geometry and a byte offset **within that source line**. Add the source line's byte start (and the skipped newline byte) to get an offset in the full text. `line_clamp` limits soft-wrap boundaries, but `shape_text` still returns a `WrappedLine` for every explicit newline-separated source line; it does not guarantee at most N lines in total. A width or font change can alter wrap boundaries, so recompute layout when either changes.

For ordinary paragraphs, let a GPUI text element or GPUI Kit `TextView` perform this work. A custom element should only shape text directly when it needs glyph-aware placement, drawing, or hit testing that existing elements cannot supply.

## Match GPUI's rendering phases

GPUI's [rendering pipeline](./render) separates layout, prepaint, and paint. Place custom text work in the matching phase:

| Phase | Text work | Why |
| --- | --- | --- |
| `request_layout` | Declare style and layout nodes; use a measured-layout callback if intrinsic text size or wrapping determines the requested size. | The callback receives width constraints; final `Bounds<Pixels>` are not available yet. |
| `prepaint` | Use resolved bounds to place prepared lines, calculate cursor geometry, and establish hitboxes; shape here if this custom element only now knows its content width. | Input geometry must match this frame. |
| `paint` | Paint the prepared `ShapedLine`s or `WrappedLine`s at their origins. | Reusing the shape prepared during measurement or prepaint keeps pixels and hit tests aligned. |

The ordinary GPUI text element shapes in its measured-layout callback, records bounds in prepaint, and paints the prepared `WrappedLine`s. The [Input element](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) instead shapes visible editor lines in prepaint because it uses its resolved viewport and scroll geometry. `ShapedLine::paint` submits one line; `paint_background(...)` is a separate pass for run backgrounds. Handle both `Result`s. `WrappedLine::paint` takes optional `Bounds<Pixels>` as its alignment width, whereas `ShapedLine::paint` takes `Option<Pixels>`. Text painting does not itself establish a hitbox, keyboard focus, or accessibility name; a custom interactive text element must supply those contracts too. See [Paint](./paint) for custom drawing.

For a chart label, ask whether its width changes the chart's requested size. If it does, shape it in the measured-layout callback and carry that result forward; prepaint positions it from the resolved bounds, then paint draws that same result. If the chart already has a fixed rectangle and only the label's location depends on it, shape and position in prepaint. In either case, the pointer mapping and paint origin must use the same prepared line. The `render` measurement in the small exercise above is only an observable probe, not a replacement for these element phases.

## Diagnose a text mismatch

| Symptom | First check | Next action |
| --- | --- | --- |
| First layout panics while resolving a font | Inspect `cx.text_system().all_font_names()` after initialization. On Web, check that required font bytes were registered before opening the window. | Use an available family or register the intended font; see [Fonts](./fonts#diagnose-a-font-problem). |
| Text appears in another face, or widths differ by machine | Compare the requested family with the available names and the family embedded in a bundled file. A family name does not prove glyph coverage. | Test on each target platform; bundle the needed faces and check fallback for Latin, CJK, and emoji. |
| Measured width does not match the painted label | Check the resolved font, features, size, text, and `force_width` on both paths. A parent `.text_size(...)` does not silently change a `shape_line` call's explicit size. | Shape with the same inputs and reuse the result for placement and painting. |
| Caret or click lands at the wrong character | Check whether the index is a UTF-8 byte offset and whether the line origin was added or subtracted exactly once. | Use `x_for_index` and `closest_index_for_x` on the same line layout used to paint; keep indices at valid boundaries. |
| Last glyph is cut off or the row is too short | Check whether code used `width()` as ink bounds or `font_size` as full line height. | Allow for glyph overhang and use line metrics plus the chosen line height when allocating and clipping. |
| A paragraph wraps differently after fonts load | Check whether registration happened after the first layout and whether the window was refreshed. | Register bundled fonts before the first frame, or call `cx.refresh_windows()` after a later `add_fonts` and measure again. |

## Cache and performance boundaries

`WindowTextSystem` caches line layouts in current- and previous-frame caches and shares font resolution and metrics through `TextSystem`. The shared system also caches glyph raster bounds; painting uses glyph rendering parameters that include font, size, scale factor, and subpixel position. A cache key includes text, resolved font runs, size, and width constraints. Repeated identical inputs can reuse layout; changing text, font, features, size, forced width, or wrap width may require a new layout. Paint colors and backgrounds live in decoration runs, separate from glyph positions; changing only those need not reshape glyphs. These frame caches are not a promise that a layout remains cached indefinitely. For a known set of fonts, `TextSystem::prewarm_fonts(&fonts)` can prepare platform font caches on a background executor; ordinary shaping fills missing entries on demand. Reuse the window's text system and do not retain frame-local `&mut Window` references.

For large virtualized editors or documents, shape only visible content and avoid measuring every row on every redraw. GPUI Kit's [TextView](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/text/text_view.rs) can virtualize scrollable blocks, while [Input](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) shapes its visible lines. If a profiler shows repeated materialization of long line text, `try_layout_line_by_hash` probes the cache without building a contiguous string, and `shape_line_by_hash` builds one only on a miss. With the latter API, `ShapedLine.text` is an empty placeholder even on a miss; keep the original text separately if needed. The caller must guarantee that the same hash implies identical text, and pass its UTF-8 byte length as `text_len`. Prefer the ordinary API until that cost is measured.
