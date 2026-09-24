---
title: Paint
description: Draw custom geometry in GPUI and understand the boundary between layout, hit testing, and painting.
order: -2.75
---

# Paint

GPUI paints after layout and `prepaint`. The resolved `Bounds<Pixels>` tell a custom [`Element`](./element) where it can draw. `paint` records drawing commands for the current frame; it does not establish layout or input geometry. Use `window.paint_quad` for rectangles and borders, existing image and text elements for those media, and `window.paint_path` for freeform shapes. A `canvas` is a convenient way to run prepaint and paint callbacks without implementing the whole `Element` trait.

## Build a path

`PathBuilder` describes vector geometry and tessellates it into a `Path<Pixels>` when `build()` succeeds. Choose `fill()` for a closed area or `stroke(width)` for a line. Its points use window pixel coordinates, so add the element's resolved origin when your data is local to its bounds.

```rust
use gpui_kit::*;

fn paint_triangle(bounds: Bounds<Pixels>, color: Hsla, window: &mut Window) {
    let mut builder = PathBuilder::fill();
    builder.move_to(bounds.origin);
    builder.line_to(point(bounds.right(), bounds.top()));
    builder.line_to(point(bounds.left() + bounds.size.width / 2., bounds.bottom()));
    builder.close();

    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}
```

`move_to`, `line_to`, `curve_to` (quadratic), `cubic_bezier_to`, `arc_to`, `add_polygon`, and `close` describe segments. `translate`, `scale`, and `rotate` transform the path before tessellation. `PathBuilder::stroke(px(1.)).dash_array(&[px(4.), px(2.)])` produces a dashed outline. `build()` returns a `Result`; handle failure rather than assuming arbitrary geometry can always be tessellated. A built `Path` can be cloned and painted in more than one color or frame when its geometry has not changed.

### From SVG paths to GPUI

`PathBuilder` was introduced to GPUI for candlestick chart drawing needs. It uses Lyon's SVG path builder internally, so its segment vocabulary is familiar from SVG. If you know [SVG `d` path commands](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/d), the segment concepts transfer directly:

| SVG path | GPUI builder | Meaning |
| --- | --- | --- |
| `M x y` | `move_to(point)` | Start a subpath. |
| `L x y` | `line_to(point)` | Draw a straight segment. |
| `Q cx cy x y` | `curve_to(end, control)` | Quadratic Bézier. |
| `C c1x c1y c2x c2y x y` | `cubic_bezier_to(end, control_a, control_b)` | Cubic Bézier. |
| `A rx ry rotation large sweep x y` | `arc_to(radii, rotation, large_arc, sweep, end)` | Elliptical arc. |
| `Z` | `close()` | Close the current subpath. |

The geometry model is familiar, but the Rust argument order is not a literal transcription of SVG syntax: `curve_to` and `cubic_bezier_to` take the **end point first**, followed by control points. `x_rotation` is passed as a `Pixels` value whose numeric part represents degrees in the current API. Coordinates are typed `Point<Pixels>`, and `build()` tessellates the path before `paint_path` submits it. This makes porting SVG drawing logic straightforward while keeping GPUI's typed coordinate and error handling rules explicit.

### One candlestick, two path notations

Switch between GPUI and SVG source. Both trace the same twelve corners in the same order; the preview is the actual SVG path. This is a teaching example that combines wick and body in one silhouette. [GPUI Kit's candlestick chart](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/chart/candlestick_chart.rs) instead paints a stroked path for the wick and a quad for the body. The example uses a local 100 × 128 coordinate space. In a real GPUI `canvas`, add `bounds.origin` to each point.

<div class="doc-tabs">
  <input class="doc-tabs__input" type="radio" name="candle-source-en" id="candle-rust-en" checked>
  <input class="doc-tabs__input" type="radio" name="candle-source-en" id="candle-svg-en">
  <div class="doc-tabs__list" role="tablist" aria-label="Candlestick path source">
    <label for="candle-rust-en" role="tab">GPUI PathBuilder</label>
    <label for="candle-svg-en" role="tab">SVG path</label>
  </div>
  <div class="doc-tabs__panels">
    <section class="doc-tabs__panel">
      <pre><code class="language-rust">let mut builder = PathBuilder::fill();
builder.move_to(point(px(48.), px(16.)));
builder.line_to(point(px(52.), px(16.)));
builder.line_to(point(px(52.), px(42.)));
builder.line_to(point(px(68.), px(42.)));
builder.line_to(point(px(68.), px(90.)));
builder.line_to(point(px(52.), px(90.)));
builder.line_to(point(px(52.), px(112.)));
builder.line_to(point(px(48.), px(112.)));
builder.line_to(point(px(48.), px(90.)));
builder.line_to(point(px(32.), px(90.)));
builder.line_to(point(px(32.), px(42.)));
builder.line_to(point(px(48.), px(42.)));
builder.close();
if let Ok(path) = builder.build() {
    window.paint_path(path, color);
}</code></pre>
    </section>
    <section class="doc-tabs__panel">
      <pre><code class="language-svg">&lt;svg viewBox=&quot;0 0 100 128&quot;&gt;
  &lt;path fill=&quot;currentColor&quot;
    d=&quot;M48 16 L52 16 L52 42 L68 42 L68 90
       L52 90 L52 112 L48 112 L48 90 L32 90
       L32 42 L48 42 Z&quot; /&gt;
&lt;/svg&gt;</code></pre>
    </section>
  </div>
</div>

<figure class="path-preview">
  <svg viewBox="0 0 100 128" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="candle-title-en">
    <title id="candle-title-en">Candlestick drawn from the SVG path above</title>
    <path fill="currentColor" d="M48 16 L52 16 L52 42 L68 42 L68 90 L52 90 L52 112 L48 112 L48 90 L32 90 L32 42 L48 42 Z" />
  </svg>
  <figcaption>One filled path: wick and body share the same outline.</figcaption>
</figure>

## Choose the right phase

| Work | Phase | Reason |
| --- | --- | --- |
| Declare size and child layout nodes | `request_layout` | Taffy needs the style before bounds exist. |
| Build paths tied to resolved bounds; insert a `Hitbox` | `prepaint` | Bounds and current frame input geometry are available. |
| Call `paint_path`, `paint_quad`, or paint prepared children | `paint` | Drawing order is now known. |

For a small decorative shape, `canvas(prepaint, paint)` is enough. [GPUI Kit's Plot line](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/plot/shape/line.rs) builds a stroke from data points and paints it into the chart. The [input element](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) uses paths for selections and text range decorations; it paints blinking carets as quads. Both use `PathBuilder`, but the input must also coordinate text metrics and hit testing.

The scene can clip drawing with `window.with_content_mask`. Clipping, input hitboxes, and [accessibility](./accessibility) are separate contracts: a painted path is not automatically clickable or announced to assistive technology. A chart with point interaction must also establish hitboxes or an equivalent pointer mapping, and a semantic chart needs an accessible representation.

## Avoid unnecessary tessellation

Path tessellation has real cost. Keep a path when its source points and dimensions are unchanged; rebuild it when either changes. Do not cache a path that contains absolute window coordinates across relocation unless the cache also accounts for the new origin. For simple boxes, prefer `paint_quad` or a styled `div()` so GPUI can use its standard painting and interaction machinery.

## Three GPUI Kit examples, three ownership choices

The drawing primitives are the same, but each GPUI Kit feature keeps its work at a different lifetime. The choice follows where the data lives and how often it changes.

### A model-owned gauge: an Entity keeps geometry

A gauge driven by an [Entity](./entity) can keep separate `Option<Path<Pixels>>` values for its background, value arc, and needle. On a value change, clear only the value arc and needle. On an origin change, clear all paths if they contain absolute window coordinates. A `canvas` callback can build missing paths from its bounds during prepaint, then paint them with current theme colors. This keeps geometry invalidation separate from color selection. This pattern fits a view that already owns model subscriptions; avoid calling `cx.notify()` unconditionally from prepaint, because that can schedule an extra render every frame.

### Plot: a value-like element uses keyed window state

This cache depends on a stable [ElementId](./element_id) across frames. [`Line`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/plot/shape/line.rs) is recreated as a value during render. Storing a cache on that value would lose it on the next frame. [`PathCaches::for_paint`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/plot/path_cache.rs) instead uses `window.use_keyed_state` under the plot's current element ID. `Line::paint_cached` hashes the projected points, stroke width, and curve style; `PathCache::get` tessellates only when the key changes. It builds the path relative to zero, then clones and translates cached vertices to this frame's origin, so scrolling does not trigger tessellation, although translation still costs work. Dots remain cheap quads painted at the new origin. This pattern depends on stable element identity and benefits from using the same slot for the same series across frames; reordering series by index causes avoidable cache misses when their shape keys differ.

### Input: a text editor owns the whole Element pipeline

GPUI Kit's [input element](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) owns much more than its selection paths. It measures text, tracks wrapping and viewport geometry, inserts a hitbox in prepaint, then paints selection paths, caret quads, and text from prepared layout. Its input handlers and focus behavior depend on the same geometry. This is why a complex editor needs a custom `Element`: text layout, hit testing, input routing, and drawing must agree on one snapshot. Paths for selections and text range decorations are only part of that pipeline.

The progression is useful when choosing an API: use `canvas` for a focused decoration owned by an existing Entity; use keyed window state when a value-like drawing element needs a cache across frames; implement `Element` when layout, text, hit testing, and input must be coordinated directly. See the [Element lifecycle](./element) for the trait methods and the [Event guide](./event) for input propagation.
