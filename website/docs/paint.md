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

`PathBuilder` was introduced to GPUI for candlestick chart drawing needs. It uses Lyon's SVG path builder internally, so its segment vocabulary is familiar from SVG. If you know [SVG path commands](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/d), the segment concepts transfer directly:

| SVG path | GPUI builder | Meaning |
| --- | --- | --- |
| `M x y` | `move_to(point)` | Start a subpath. |
| `L x y` | `line_to(point)` | Draw a straight segment. |
| `Q cx cy x y` | `curve_to(end, control)` | Quadratic Bézier. |
| `C c1x c1y c2x c2y x y` | `cubic_bezier_to(end, control_a, control_b)` | Cubic Bézier. |
| `A rx ry rotation large sweep x y` | `arc_to(radii, rotation, large_arc, sweep, end)` | Elliptical arc. |
| `Z` | `close()` | Close the current subpath. |

The geometry model is familiar, but the Rust argument order is not a literal transcription of SVG syntax: `curve_to` and `cubic_bezier_to` take the **end point first**, followed by control points. `x_rotation` is passed as a `Pixels` value whose numeric part represents degrees in the current API. Coordinates are typed `Point<Pixels>`, and `build()` tessellates the path before `paint_path` submits it. This makes porting SVG drawing logic straightforward while keeping GPUI's typed coordinate and error handling rules explicit.

### The GPUI Kit mark in two path notations

The GPUI Kit mark makes the relationship between SVG paths and `PathBuilder` concrete. Its two closed paths paint separately: the outer shape uses the theme foreground and the inner stroke uses the theme blue accent. Switch between the GPUI and SVG source, then compare them with the rendered mark below. The example uses a local 32 × 32 coordinate space; the GPUI code adds the bounds origin to each point. `foreground` and `accent_color` are colors supplied by the caller.

<div class="doc-tabs">
  <input class="doc-tabs__input" type="radio" name="logo-source-en" id="logo-rust-en" checked>
  <input class="doc-tabs__input" type="radio" name="logo-source-en" id="logo-svg-en">
  <div class="doc-tabs__list" role="tablist" aria-label="GPUI Kit logo path source">
    <label for="logo-rust-en" role="tab">PathBuilder</label>
    <label for="logo-svg-en" role="tab">SVG</label>
  </div>
  <div class="doc-tabs__panels">
    <section class="doc-tabs__panel">
      <pre class="astro-code shiki-themes macos-classic-light macos-classic-dark" style="background-color:#f6f6f7;--shiki-dark-bg:#131313;color:#000000;--shiki-dark:#CACCCA" tabindex="0"><code><span class="line"><span style="color:#0433FF;--shiki-dark:#87B1F6">let</span><span style="color:#000000;--shiki-dark:#CACCCA"> p </span><span style="color:#0433FF;--shiki-dark:#87B1F6">=</span><span style="color:#0433FF;--shiki-dark:#87B1F6"> |</span><span style="color:#000000;--shiki-dark:#CACCCA">x</span><span style="color:#0433FF;--shiki-dark:#87B1F6">:</span><span style="color:#571AB7;--shiki-dark:#CBA6F7"> f32</span><span style="color:#000000;--shiki-dark:#CACCCA">, y</span><span style="color:#0433FF;--shiki-dark:#87B1F6">:</span><span style="color:#571AB7;--shiki-dark:#CBA6F7"> f32</span><span style="color:#0433FF;--shiki-dark:#87B1F6">|</span><span style="color:#0000A2;--shiki-dark:#B3C5F3"> point</span><span style="color:#000000;--shiki-dark:#CACCCA">(bounds</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">left</span><span style="color:#000000;--shiki-dark:#CACCCA">() </span><span style="color:#0433FF;--shiki-dark:#87B1F6">+</span><span style="color:#0000A2;--shiki-dark:#B3C5F3"> px</span><span style="color:#000000;--shiki-dark:#CACCCA">(x), bounds</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">top</span><span style="color:#000000;--shiki-dark:#CACCCA">() </span><span style="color:#0433FF;--shiki-dark:#87B1F6">+</span><span style="color:#0000A2;--shiki-dark:#B3C5F3"> px</span><span style="color:#000000;--shiki-dark:#CACCCA">(y));</span></span>
<span class="line"></span>
<span class="line"><span style="color:#0433FF;--shiki-dark:#87B1F6">let</span><span style="color:#0433FF;--shiki-dark:#87B1F6"> mut</span><span style="color:#000000;--shiki-dark:#CACCCA"> outer </span><span style="color:#0433FF;--shiki-dark:#87B1F6">=</span><span style="color:#571AB7;--shiki-dark:#CBA6F7"> PathBuilder</span><span style="color:#0433FF;--shiki-dark:#87B1F6">::</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">fill</span><span style="color:#000000;--shiki-dark:#CACCCA">();</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">move_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">4</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">4</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">4</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">9</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">10</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">9</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">10</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">23</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">23</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">4</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">close</span><span style="color:#000000;--shiki-dark:#CACCCA">();</span></span>
<span class="line"><span style="color:#0433FF;--shiki-dark:#87B1F6">if</span><span style="color:#0433FF;--shiki-dark:#87B1F6"> let</span><span style="color:#571AB7;--shiki-dark:#CBA6F7"> Ok</span><span style="color:#000000;--shiki-dark:#CACCCA">(path) </span><span style="color:#0433FF;--shiki-dark:#87B1F6">=</span><span style="color:#000000;--shiki-dark:#CACCCA"> outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">build</span><span style="color:#000000;--shiki-dark:#CACCCA">() {</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">    window</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">paint_path</span><span style="color:#000000;--shiki-dark:#CACCCA">(path, foreground);</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">}</span></span>
<span class="line"></span>
<span class="line"><span style="color:#0433FF;--shiki-dark:#87B1F6">let</span><span style="color:#0433FF;--shiki-dark:#87B1F6"> mut</span><span style="color:#000000;--shiki-dark:#CACCCA"> accent </span><span style="color:#0433FF;--shiki-dark:#87B1F6">=</span><span style="color:#571AB7;--shiki-dark:#CBA6F7"> PathBuilder</span><span style="color:#0433FF;--shiki-dark:#87B1F6">::</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">fill</span><span style="color:#000000;--shiki-dark:#CACCCA">();</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">move_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">16</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">13</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">13</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">23</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">23</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">23</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">23</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">18</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">16</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">18</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">close</span><span style="color:#000000;--shiki-dark:#CACCCA">();</span></span>
<span class="line"><span style="color:#0433FF;--shiki-dark:#87B1F6">if</span><span style="color:#0433FF;--shiki-dark:#87B1F6"> let</span><span style="color:#571AB7;--shiki-dark:#CBA6F7"> Ok</span><span style="color:#000000;--shiki-dark:#CACCCA">(path) </span><span style="color:#0433FF;--shiki-dark:#87B1F6">=</span><span style="color:#000000;--shiki-dark:#CACCCA"> accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">build</span><span style="color:#000000;--shiki-dark:#CACCCA">() {</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">    window</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">paint_path</span><span style="color:#000000;--shiki-dark:#CACCCA">(path, accent_color);</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">}</span></span></code></pre>
    </section>
    <section class="doc-tabs__panel">
      <pre><code class="language-svg">&lt;svg viewBox=&quot;0 0 32 32&quot;&gt;
  &lt;path fill=&quot;currentColor&quot; d=&quot;M4 4 L28 4 L28 9 L10 9 L10 23 L28 23 L28 28 L4 28 Z&quot; /&gt;
  &lt;path fill=&quot;#3B82F6&quot; d=&quot;M16 13 L28 13 L28 23 L23 23 L23 18 L16 18 Z&quot; /&gt;
&lt;/svg&gt;</code></pre>
    </section>
  </div>
  <figure class="path-preview">
  <svg viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="logo-title-en">
    <title id="logo-title-en">GPUI Kit mark drawn with two colored paths</title>
    <path fill="currentColor" d="M4 4 L28 4 L28 9 L10 9 L10 23 L28 23 L28 28 L4 28 Z" />
    <path fill="var(--data-2, #3B82F6)" d="M16 13 L28 13 L28 23 L23 23 L23 18 L16 18 Z" />
  </svg>
  <figcaption>Two filled paths, with foreground and the theme blue accent painted separately.</figcaption>
  </figure>
</div>

GPUI’s `PathBuilder` takes full point coordinates; it has no SVG `H` or `V` shorthand and does not parse an SVG `d` string directly. The SVG tab spells out every `L` coordinate to make the correspondence visible. If the source is already an SVG file and no `Path<Pixels>` is needed, render the asset with `svg().path("icons/logo.svg")`. Converting arbitrary SVG path data into a GPUI `Path` requires a separate parser that feeds segments into `PathBuilder`.

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
