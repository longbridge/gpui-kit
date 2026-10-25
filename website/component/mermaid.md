---
title: Mermaid
description: Native Mermaid diagram previews using Merman and GPUI's SVG renderer.
---

# Mermaid

`Mermaid` displays a diagram from Mermaid source without a browser or JavaScript
runtime. Merman parses and lays out the diagram, produces resvg-compatible SVG,
and GPUI rasterizes it as a full-color image. Use it for readonly previews of
flowcharts, sequence diagrams, class diagrams and other Merman-supported families.

## Enable

Enable the optional `mermaid` feature on your `gpui-kit` dependency. The component
is available on native platforms only; it is not exported on WebAssembly.
Merman 0.9.2 requires Rust 1.95 or newer, in addition to GPUI's requirements.

```toml
[dependencies]
gpui-kit = { version = "0.8", features = ["mermaid"] }
```

Initialize with `gpui_kit::init(cx)` and open windows with
`gpui_kit::open_window`, as in [Getting Started](../docs/getting-started.md).

## Usage

```rust
use gpui_kit::*;
use gpui_kit::component::mermaid::{Mermaid, MermaidState};
```

Create the state once and store it as an `Entity<MermaidState>` in the owner:

```rust
let diagram = cx.new(|cx| MermaidState::new("flowchart LR\nA[Start] --> B[Done]", cx));
```

Build only the viewport during rendering:

```rust
Mermaid::new(&self.diagram)
    .accessibility_label("The process moves from Start to Done")
    .w_full()
    .h(rems(24.))
```

The default viewport is full width and 20 rem high. The image fits the viewport
while preserving its aspect ratio. Supply a meaningful accessible description;
the diagram's individual nodes are not interactive or exposed as text controls.

Update the source from an editor or other application state:

```rust
self.diagram.update(cx, |state, cx| {
    state.set_source("sequenceDiagram\nAlice->>Bob: Hello", cx);
});
```

## Rendering behavior

Rendering starts as soon as the preview has a nonzero layout size. Source updates
have a 150 ms debounce for live editing; the initial render and theme/DPI changes
do not add that delay. Parsing, layout, SVG preparation and rasterization run off the UI thread.
Source changes cancel previous work and discard stale results. Empty source shows
an empty state; invalid or unsupported source shows a diagnostic. Read
`source()`, `is_loading()` and `error()` to inspect the retained state.

The component maps the active theme's semantic colors and chart palette through
Merman's `HostTheme`, including family-specific settings. Theme changes
rerender the diagram; preview-size and display-scale changes reuse the parsed SVG.
The default appearance is Classic, with solid class-compartment boundaries.
Labels use native SVG text; `resvg_safe` removes browser-only content before GPUI
parses the image. A compatibility adjustment centers Merman 0.9.2's native class
relationship labels within their backgrounds, aligns Use Case labels with their
nodes, and supplies the Info diagram's missing SVG viewport. Generated HTML-label
fallbacks allow glyph overflow so estimated text widths do not cut off characters.
This does not replace Merman's text measurement or layout.
Raster resolution fits the actual preview in both dimensions at twice the device
resolution, so diagrams remain sharp without rasterizing an entire window.
It retains one diagram per state and releases GPUI's cached image when replaced or released.

Rendering uses Merman's constrained resource profile, a cooperative ten-second
deadline, a 128 KiB source limit, and an 8 MiB SVG limit. Raster images are bounded
to 4096 pixels per edge and approximately four million pixels in total. Large
diagrams may therefore look softer when enlarged. Cancellation and deadlines are
cooperative, not process isolation or a hard timeout.

For responsive development builds in your own application, optimize `resvg`,
`tiny-skia` and `tiny-skia-path` in your workspace's `[profile.dev.package]`
settings, as this repository does. Optimizing `resvg` alone leaves its underlying
pixel loops unoptimized; dependency crates cannot set your application's profiles.

## Compatibility

This uses Merman 0.9.2, not Mermaid.js. Syntax and layout may differ from a browser
preview. All built-in diagram families and Cytoscape layout are compiled in;
ELK layout, math labels, external/embedded images and browser interactions are
not supported. The default layout is Dagre. Explicit unsupported source options
can produce diagnostics. Deterministic text measurement may differ from the
system fonts used by the SVG rasterizer, particularly for long or mixed-script
labels.

The component includes scoped repairs for Merman 0.9.2. Timeline retains events
before the first named section, and vertical sections leave room for the preceding
events. The renderer adapts a private source copy using parser-provided spans;
`MermaidState::source()` remains unchanged. Railroad titles are displayed above the
rules, with wrapping for long titles. Rule names align vertically with the rails
and fit the reserved width before their start markers. Flowchart edge labels use
opaque backgrounds so lines do not run through their text. Requirement stereotypes
fit their content width; C4 relationship labels and State label backgrounds are aligned. Cynefin,
Event Modeling and Journey use readable text on categorical fills, and Tree View
inherits the component font.

Focused regression fixtures and optional `.labels` sidecars are kept in
`crates/component/src/mermaid/fixtures`; the ignored `render_audit_fixtures` test
uses the production SVG and GPUI rasterization path in light and dark themes. Set `GPUI_MERMAID_OUTPUT` to
an output directory and run
`cargo test -p gpui-component --features mermaid --lib render_audit_fixtures -- --ignored --nocapture`.
This coverage does not establish complete Mermaid.js compatibility or guarantee
identical font metrics on every platform. Long mixed-script labels can still have
tight spacing, and very large diagrams are scaled to the bounded preview.

This standalone component does not replace Markdown code-fence rendering.
The native Story Gallery's **Mermaid** story provides editable source and
a searchable example selector covering all 34 enabled diagram families, including
Chinese labels. In Timeline, `section Delivery` starts the group named “Delivery”;
subsequent time periods belong to that group until the next `section`.

Gallery sources live in `crates/story/src/fixtures/mermaid`. To audit the actual
Gallery examples, set `GPUI_MERMAID_FIXTURES` to that directory when running the
manual audit above.
