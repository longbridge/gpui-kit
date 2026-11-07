---
title: Style
description: Style GPUI elements with Tailwind CSS–familiar utilities, typed values, and fluent Rust builders.
order: -2.61
---

# Style

GPUI styles an [Element](./element) where it is built. The `Styled` trait supplies chainable methods for layout, spacing, color, borders, and text. Many names deliberately correspond to <a href="https://tailwindcss.com/docs/styling-with-utility-classes" target="_blank" rel="noopener noreferrer">Tailwind CSS utilities</a>: `flex items-center gap-2 px-3` becomes `.flex().items_center().gap_2().px_3()` in Rust. This is a useful way to read and write GPUI layouts, but the values are typed Rust values rather than CSS classes.

```rust
use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;

div()
    .flex()
    .items_center()
    .gap_2()
    .px_3()
    .py_2()
    .bg(cx.theme().background)
    .text_color(cx.theme().foreground)
    .child("Search results")
```

The builder consumes and returns an element on each call. Rendering can build a fresh tree from current state; persistent application state belongs in an [`Entity`](./entity) or keyed element state. A style chain describes this frame's presentation, not a stylesheet or a retained component instance. See [RenderOnce](./render-once) for frame-local component construction.

## Build a first layout

Start with the region that owns the available space, then decide which child has a fixed width and which child can grow. Replace `examples/hello_world/src/main.rs` with this complete program, then run `cargo run -p hello_world` from the repository root:

```rust
use gpui_kit::*;
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _};
use gpui_kit::prelude::FluentBuilder as _;

struct StyleExample;

impl Render for StyleExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let compact = window.viewport_size().width < px(600.);
        let layout = if compact { v_flex() } else { h_flex() };
        let documents = (1..=60).map(|number| {
            div().p_2().child(format!("Document {number:02}"))
        });

        layout
            .items_stretch()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                v_flex()
                    .when(!compact, |nav| nav.w_64().flex_shrink_0())
                    .p_3()
                    .gap_2()
                    .child("Navigation")
                    .child("Overview · Documents"),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .child(h_flex().p_3().child("Documents"))
                    .child(
                        v_flex()
                            .id("document-list")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .child(v_flex().p_3().gap_2().children(documents)),
                    ),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| StyleExample)
        })
        .expect("failed to open window");
    });
}
```

In a wide window, navigation occupies a fixed `w_64()` rail and the document pane fills the rest. Narrow the content area below 600 logical pixels: the same navigation moves above the document pane, while the document list remains independently scrollable. Scroll to `Document 60`, then resize the window in both directions. The header stays in place while the list scrolls. This threshold is an explicit Rust condition evaluated when the view renders, not a Tailwind responsive prefix. The navigation in this small exercise is illustrative text; a real application should use reachable navigation controls.

`h_flex()` makes a row and centers its children on the cross axis. `.items_stretch()` overrides that default so both panes occupy the row's height. `v_flex()` makes a column whose children stretch across its width. The fixed navigation pane does not shrink in the wide layout; the document pane takes the remaining width. The scroll area takes the remaining height below the header. The window-sized root gives `.size_full()` a resolved height; the `min_h_0()` calls let its flexible descendants shrink into a scroll viewport.

### Decide where each size belongs

| Need | Put it on | Why |
| --- | --- | --- |
| Space between siblings | The parent with `.gap_3()` | Gap separates children without adding padding at the outer edge. |
| Space inside a surface | The surface with `.p_3()` | Padding moves its content inward and participates in its layout size. |
| A fixed rail beside flexible content | Rail `.w_64().flex_shrink_0()`; content `.flex_1().min_w_0()` | The rail keeps its width while the content may shrink below its natural text width. |
| A header above scrolling content | Column with a height; scroll child `.flex_1().min_h_0()` | The child can shrink into the available height, creating a real scroll viewport. |
| Half the parent width | `.w(relative(0.5))` on the child | The fraction resolves against the relevant parent dimension during layout. |

`w_full()` and `h_full()` mean the full *available* dimension. A percentage height still needs a definite height upstream. Min and max sizes constrain the result; they do not give an otherwise unbounded scroll area a viewport. For a long single-line label in a row, combine `.flex_1().min_w_0().truncate()` on the label container. `.truncate()` only changes text overflow; it cannot force an inflexible sibling to give up width.

### Choose clipping, scrolling, or positioning

`.overflow_hidden()` clips content; it does not make that content scrollable. On a stateful element, `.overflow_y_scroll()` creates vertical scrolling once the element has a bounded height. GPUI Kit's `.overflow_y_scrollbar()` adds a visible scrollbar and wraps the original element as its scroll area; it is an extension from `ScrollableElement`, not a `Styled` method. Keep one owner for each scroll region, and put content padding inside that region if its scrollbar should sit at the pane edge. See [Coding Guides](./coding-guides) for scroll ownership and measurement.

Normal flex children consume layout space. Use `.relative()` on a container and `.absolute().top_0().right_0()` on a badge when the badge should overlay content without consuming a row or column slot. Offset setters position an absolute child; they do not make an ordinary flex child absolute. Later siblings normally paint over earlier siblings; general `Styled` has no `z_index(...)` method.

### Theme and scale

Use semantic colors and radius from `cx.theme()` for application surfaces. GPUI Kit components already apply their normal theme appearance; style their instances for local layout or an intentional refinement. The named spacing and size helpers are rem based: `_1` is `0.25rem`, `_2` is `0.5rem`, `_3` is `0.75rem`, and `_4` is `1rem`. In a GPUI Kit `Root`, the active theme's base font size sets the window rem size, so a font size or zoom change also changes rem based geometry. Use typed setters such as `.gap(rems(0.625))` when the scale has no suitable step; reserve `px(...)` for a dimension that truly needs pixels. See [Geometry](./geometry) for length types and [Fonts](./fonts) for the theme's rem setup.

### Troubleshoot the result

| Symptom | Check |
| --- | --- |
| The rail does not move above the documents when the window narrows | Resize the drawable content area below 600 logical pixels. The condition reads `window.viewport_size().width` during `render`; changing only display scale does not cross this logical-pixel threshold. |
| `Document 60` cannot be reached | Place the pointer over the document list and scroll there. Keep `.id("document-list").overflow_y_scroll()` on the bounded list region, with `.flex_1().min_h_0()` on it and its containing column. |
| The header moves when scrolling | Ensure the header is a sibling of the list viewport, not a child inside the scrolling element. |
| A pane header disappears at the top | `h_flex()` centers children by default; stretch the row's children or give that pane full height. |
| A title overflows instead of truncating | Release the flexible child's minimum width with `.min_w_0()` and bound the text width. |
| A list grows past the window instead of scrolling | Give its ancestors a resolved height, let the flexible child shrink with `.min_h_0()`, and put scrolling on the intended viewport. |
| A scrollbar sits inside the pane edge | Check which element owns scrolling and whether padding wraps the scroll owner. |
| Layout changes after theme zoom | Recheck rem based dimensions and any cached measurements that assumed the old rem size. |

## Common `Styled` methods

GPUI uses underscores where Tailwind uses hyphens. Where a matching concept exists, the first column links to its official Tailwind CSS reference in a new tab. The names are GPUI methods; call them on a `Styled` value, such as `div().gap_2()`. These tables cover the distinct `Styled` operations and the generic setters generated by its macros. Numeric variants follow the families described below, rather than occupying thousands of near-identical rows. Linked pages explain the corresponding styling concept; they do not imply identical behavior in GPUI and a browser.

### Display and visibility

| Method | Description |
| --- | --- |
| <a href="https://tailwindcss.com/docs/display" target="_blank" rel="noopener noreferrer"><code>block</code></a> | Use block layout. |
| <a href="https://tailwindcss.com/docs/display" target="_blank" rel="noopener noreferrer"><code>flex</code></a> | Use Flexbox layout. |
| <a href="https://tailwindcss.com/docs/display" target="_blank" rel="noopener noreferrer"><code>grid</code></a> | Use Grid layout. |
| <a href="https://tailwindcss.com/docs/display" target="_blank" rel="noopener noreferrer"><code>hidden</code></a> | Remove the element from layout and painting. |
| <a href="https://tailwindcss.com/docs/visibility" target="_blank" rel="noopener noreferrer"><code>invisible</code></a> | Keep its layout space but do not paint it. |
| <a href="https://tailwindcss.com/docs/visibility" target="_blank" rel="noopener noreferrer"><code>visible</code></a> | Restore painting while retaining the element's layout. |

### Flexbox and Grid

| Method | Description |
| --- | --- |
| <a href="https://tailwindcss.com/docs/flex-direction" target="_blank" rel="noopener noreferrer"><code>flex_row</code></a> | Place flex children along a row. |
| <a href="https://tailwindcss.com/docs/flex-direction" target="_blank" rel="noopener noreferrer"><code>flex_col</code></a> | Place flex children along a column. |
| <a href="https://tailwindcss.com/docs/flex-wrap" target="_blank" rel="noopener noreferrer"><code>flex_wrap</code></a> | Allow flex children to wrap. |
| <a href="https://tailwindcss.com/docs/flex-wrap" target="_blank" rel="noopener noreferrer"><code>flex_nowrap</code></a> | Keep flex children on one line. |
| <a href="https://tailwindcss.com/docs/align-items" target="_blank" rel="noopener noreferrer"><code>items_start</code></a> | Align children to the start of the cross axis. |
| <a href="https://tailwindcss.com/docs/align-items" target="_blank" rel="noopener noreferrer"><code>items_center</code></a> | Center children on the cross axis. |
| <a href="https://tailwindcss.com/docs/align-items" target="_blank" rel="noopener noreferrer"><code>items_stretch</code></a> | Stretch children along the cross axis. |
| <a href="https://tailwindcss.com/docs/align-self" target="_blank" rel="noopener noreferrer"><code>self_center</code></a> | Center this child on its parent's cross axis. |
| <a href="https://tailwindcss.com/docs/align-self" target="_blank" rel="noopener noreferrer"><code>self_stretch</code></a> | Stretch this child on its parent's cross axis. |
| <a href="https://tailwindcss.com/docs/justify-content" target="_blank" rel="noopener noreferrer"><code>justify_center</code></a> | Center children on the main axis. |
| <a href="https://tailwindcss.com/docs/justify-content" target="_blank" rel="noopener noreferrer"><code>justify_between</code></a> | Put free space between children on the main axis. |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_between</code></a> | Distribute wrapped lines along the cross axis. |
| <a href="https://tailwindcss.com/docs/flex-basis" target="_blank" rel="noopener noreferrer"><code>flex_basis</code></a> | Set a typed initial main axis size. |
| <a href="https://tailwindcss.com/docs/flex" target="_blank" rel="noopener noreferrer"><code>flex_1</code></a> | Grow and shrink with a zero flex basis. |
| <a href="https://tailwindcss.com/docs/flex" target="_blank" rel="noopener noreferrer"><code>flex_auto</code></a> | Grow and shrink from the item's automatic basis. |
| <a href="https://tailwindcss.com/docs/flex-grow" target="_blank" rel="noopener noreferrer"><code>flex_grow_1</code></a> | Allow a flex child to grow. |
| <a href="https://tailwindcss.com/docs/flex-shrink" target="_blank" rel="noopener noreferrer"><code>flex_shrink_0</code></a> | Prevent a flex child from shrinking. |
| <a href="https://tailwindcss.com/docs/grid-template-columns" target="_blank" rel="noopener noreferrer"><code>grid_cols</code></a> | Set a numbered column template. |
| <a href="https://tailwindcss.com/docs/grid-template-rows" target="_blank" rel="noopener noreferrer"><code>grid_rows</code></a> | Set a numbered row template. |
| <a href="https://tailwindcss.com/docs/grid-column" target="_blank" rel="noopener noreferrer"><code>col_span</code></a> | Span a specified number of grid columns. |
| <a href="https://tailwindcss.com/docs/grid-row" target="_blank" rel="noopener noreferrer"><code>row_span</code></a> | Span a specified number of grid rows. |
| <a href="https://tailwindcss.com/docs/flex-direction" target="_blank" rel="noopener noreferrer"><code>flex_row_reverse</code></a> | Reverse row order. |
| <a href="https://tailwindcss.com/docs/flex-direction" target="_blank" rel="noopener noreferrer"><code>flex_col_reverse</code></a> | Reverse column order. |
| <a href="https://tailwindcss.com/docs/flex-wrap" target="_blank" rel="noopener noreferrer"><code>flex_wrap_reverse</code></a> | Wrap flex lines in reverse order. |
| <a href="https://tailwindcss.com/docs/align-items" target="_blank" rel="noopener noreferrer"><code>items_end</code></a> | Align children to the cross-axis end. |
| <a href="https://tailwindcss.com/docs/align-items" target="_blank" rel="noopener noreferrer"><code>items_baseline</code></a> | Align children's text baselines. |
| <a href="https://tailwindcss.com/docs/align-self" target="_blank" rel="noopener noreferrer"><code>self_start</code></a> | Align this item to the cross-axis start. |
| <a href="https://tailwindcss.com/docs/align-self" target="_blank" rel="noopener noreferrer"><code>self_end</code></a> | Align this item to the cross-axis end. |
| <a href="https://tailwindcss.com/docs/align-self" target="_blank" rel="noopener noreferrer"><code>self_flex_start</code></a> | Align this item to flex start. |
| <a href="https://tailwindcss.com/docs/align-self" target="_blank" rel="noopener noreferrer"><code>self_flex_end</code></a> | Align this item to flex end. |
| <a href="https://tailwindcss.com/docs/align-self" target="_blank" rel="noopener noreferrer"><code>self_baseline</code></a> | Align this item's text baseline. |
| <a href="https://tailwindcss.com/docs/justify-content" target="_blank" rel="noopener noreferrer"><code>justify_start</code></a> | Pack children at the main-axis start. |
| <a href="https://tailwindcss.com/docs/justify-content" target="_blank" rel="noopener noreferrer"><code>justify_end</code></a> | Pack children at the main-axis end. |
| <a href="https://tailwindcss.com/docs/justify-content" target="_blank" rel="noopener noreferrer"><code>justify_around</code></a> | Distribute space around children. |
| <a href="https://tailwindcss.com/docs/justify-content" target="_blank" rel="noopener noreferrer"><code>justify_evenly</code></a> | Distribute equal spaces along the main axis. |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_normal</code></a> | Use the default cross-axis line packing. |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_start</code></a> | Pack wrapped lines at cross-axis start. |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_center</code></a> | Center wrapped lines on the cross axis. |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_end</code></a> | Pack wrapped lines at cross-axis end. |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_around</code></a> | Distribute space around wrapped lines. |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_evenly</code></a> | Distribute equal space between wrapped lines. |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_stretch</code></a> | Stretch wrapped lines along the cross axis. |
| <a href="https://tailwindcss.com/docs/flex" target="_blank" rel="noopener noreferrer"><code>flex_initial</code></a> | Use an automatic basis and shrink without growing. |
| <a href="https://tailwindcss.com/docs/flex" target="_blank" rel="noopener noreferrer"><code>flex_none</code></a> | Prevent both flex growth and shrinkage. |
| <a href="https://tailwindcss.com/docs/flex-grow" target="_blank" rel="noopener noreferrer"><code>flex_grow</code></a> | Set a numeric flex growth factor. |
| <a href="https://tailwindcss.com/docs/flex-grow" target="_blank" rel="noopener noreferrer"><code>flex_grow_0</code></a> | Prevent flex growth. |
| <a href="https://tailwindcss.com/docs/flex-shrink" target="_blank" rel="noopener noreferrer"><code>flex_shrink</code></a> | Set a numeric flex shrink factor. |
| <a href="https://tailwindcss.com/docs/flex-shrink" target="_blank" rel="noopener noreferrer"><code>flex_shrink_1</code></a> | Allow flex shrinkage. |
| <a href="https://tailwindcss.com/docs/grid-template-columns" target="_blank" rel="noopener noreferrer"><code>grid_cols_min_content</code></a> | Create columns with min-content minimums. |
| <a href="https://tailwindcss.com/docs/grid-template-columns" target="_blank" rel="noopener noreferrer"><code>grid_cols_max_content</code></a> | Create columns with max-content limits. |
| <a href="https://tailwindcss.com/docs/grid-template-rows" target="_blank" rel="noopener noreferrer"><code>grid_rows_min_content</code></a> | Create rows with min-content minimums. |
| <a href="https://tailwindcss.com/docs/grid-template-rows" target="_blank" rel="noopener noreferrer"><code>grid_rows_max_content</code></a> | Create rows with max-content limits. |
| <a href="https://tailwindcss.com/docs/grid-column" target="_blank" rel="noopener noreferrer"><code>col_start</code></a> | Set the starting grid column line. |
| <a href="https://tailwindcss.com/docs/grid-column" target="_blank" rel="noopener noreferrer"><code>col_start_auto</code></a> | Use automatic column start placement. |
| <a href="https://tailwindcss.com/docs/grid-column" target="_blank" rel="noopener noreferrer"><code>col_end</code></a> | Set the ending grid column line. |
| <a href="https://tailwindcss.com/docs/grid-column" target="_blank" rel="noopener noreferrer"><code>col_end_auto</code></a> | Use automatic column end placement. |
| <a href="https://tailwindcss.com/docs/grid-column" target="_blank" rel="noopener noreferrer"><code>col_span_full</code></a> | Span the full grid column range. |
| <a href="https://tailwindcss.com/docs/grid-row" target="_blank" rel="noopener noreferrer"><code>row_start</code></a> | Set the starting grid row line. |
| <a href="https://tailwindcss.com/docs/grid-row" target="_blank" rel="noopener noreferrer"><code>row_start_auto</code></a> | Use automatic row start placement. |
| <a href="https://tailwindcss.com/docs/grid-row" target="_blank" rel="noopener noreferrer"><code>row_end</code></a> | Set the ending grid row line. |
| <a href="https://tailwindcss.com/docs/grid-row" target="_blank" rel="noopener noreferrer"><code>row_end_auto</code></a> | Use automatic row end placement. |
| <a href="https://tailwindcss.com/docs/grid-row" target="_blank" rel="noopener noreferrer"><code>row_span_full</code></a> | Span the full grid row range. |

### Space and size

| Method | Description |
| --- | --- |
| <a href="https://tailwindcss.com/docs/gap" target="_blank" rel="noopener noreferrer"><code>gap_2</code></a> | Set row and column gaps to `0.5rem`. |
| <a href="https://tailwindcss.com/docs/gap" target="_blank" rel="noopener noreferrer"><code>gap</code></a> | Set a typed gap on both axes. |
| <a href="https://tailwindcss.com/docs/gap" target="_blank" rel="noopener noreferrer"><code>gap_x_2</code></a> | Set the column gap to `0.5rem`. |
| <a href="https://tailwindcss.com/docs/gap" target="_blank" rel="noopener noreferrer"><code>gap_y_2</code></a> | Set the row gap to `0.5rem`. |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>p_4</code></a> | Set padding on all sides to `1rem`. |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>p</code></a> | Set typed padding on all sides. |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>px_3</code></a> | Set horizontal padding to `0.75rem`. |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>py_2</code></a> | Set vertical padding to `0.5rem`. |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>mt_4</code></a> | Set top margin to `1rem`. |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>m</code></a> | Set typed margins on all sides. |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>m_auto</code></a> | Set automatic margins on all sides. |
| <a href="https://tailwindcss.com/docs/width" target="_blank" rel="noopener noreferrer"><code>w</code></a> | Set a typed width. |
| <a href="https://tailwindcss.com/docs/width" target="_blank" rel="noopener noreferrer"><code>w_full</code></a> | Fill the available width. |
| <a href="https://tailwindcss.com/docs/height" target="_blank" rel="noopener noreferrer"><code>h</code></a> | Set a typed height. |
| <a href="https://tailwindcss.com/docs/height" target="_blank" rel="noopener noreferrer"><code>h_full</code></a> | Fill the available height. |
| <a href="https://tailwindcss.com/docs/min-width" target="_blank" rel="noopener noreferrer"><code>min_w</code></a> | Set a typed minimum width. |
| <a href="https://tailwindcss.com/docs/min-width" target="_blank" rel="noopener noreferrer"><code>min_w_0</code></a> | Permit width to shrink to zero. |
| <a href="https://tailwindcss.com/docs/max-width" target="_blank" rel="noopener noreferrer"><code>max_w</code></a> | Set a typed maximum width. |
| <a href="https://tailwindcss.com/docs/width#setting-both-width-and-height" target="_blank" rel="noopener noreferrer"><code>size</code></a> | Set typed width and height together. |
| <a href="https://tailwindcss.com/docs/width#setting-both-width-and-height" target="_blank" rel="noopener noreferrer"><code>size_4</code></a> | Set width and height to `1rem`. |
| <a href="https://tailwindcss.com/docs/aspect-ratio" target="_blank" rel="noopener noreferrer"><code>aspect_square</code></a> | Keep a 1:1 width to height ratio. |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>mt</code></a> | Set a typed top margin. |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>mb</code></a> | Set a typed bottom margin. |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>mx</code></a> | Set typed horizontal margins. |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>my</code></a> | Set typed vertical margins. |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>ml</code></a> | Set a typed left margin. |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>mr</code></a> | Set a typed right margin. |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>px</code></a> | Set typed horizontal padding. |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>py</code></a> | Set typed vertical padding. |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>pt</code></a> | Set a typed top padding. |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>pb</code></a> | Set a typed bottom padding. |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>pl</code></a> | Set a typed left padding. |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>pr</code></a> | Set a typed right padding. |
| <a href="https://tailwindcss.com/docs/gap" target="_blank" rel="noopener noreferrer"><code>gap_x</code></a> | Set a typed column gap. |
| <a href="https://tailwindcss.com/docs/gap" target="_blank" rel="noopener noreferrer"><code>gap_y</code></a> | Set a typed row gap. |
| <a href="https://tailwindcss.com/docs/min-height" target="_blank" rel="noopener noreferrer"><code>min_h</code></a> | Set a typed minimum height. |
| <a href="https://tailwindcss.com/docs/max-height" target="_blank" rel="noopener noreferrer"><code>max_h</code></a> | Set a typed maximum height. |
| <a href="https://tailwindcss.com/docs/aspect-ratio" target="_blank" rel="noopener noreferrer"><code>aspect_ratio</code></a> | Set a numeric width to height ratio. |

### Position and overflow

| Method | Description |
| --- | --- |
| <a href="https://tailwindcss.com/docs/position" target="_blank" rel="noopener noreferrer"><code>relative</code></a> | Keep normal layout placement and establish a positioned ancestor. |
| <a href="https://tailwindcss.com/docs/position" target="_blank" rel="noopener noreferrer"><code>absolute</code></a> | Position relative to an ancestor using inset offsets. |
| <a href="https://tailwindcss.com/docs/top-right-bottom-left" target="_blank" rel="noopener noreferrer"><code>inset</code></a> | Set a typed offset on all four sides. |
| <a href="https://tailwindcss.com/docs/top-right-bottom-left" target="_blank" rel="noopener noreferrer"><code>inset_0</code></a> | Set top, right, bottom, and left offsets to zero. |
| <a href="https://tailwindcss.com/docs/top-right-bottom-left" target="_blank" rel="noopener noreferrer"><code>top</code></a> | Set a typed top offset. |
| <a href="https://tailwindcss.com/docs/top-right-bottom-left" target="_blank" rel="noopener noreferrer"><code>top_0</code></a> | Set the top offset to zero. |
| <a href="https://tailwindcss.com/docs/overflow" target="_blank" rel="noopener noreferrer"><code>overflow_hidden</code></a> | Clip overflowing content on both axes. |
| <a href="https://tailwindcss.com/docs/overflow" target="_blank" rel="noopener noreferrer"><code>overflow_x_hidden</code></a> | Clip horizontal overflow only. |
| <a href="https://tailwindcss.com/docs/overflow" target="_blank" rel="noopener noreferrer"><code>overflow_y_hidden</code></a> | Clip vertical overflow only. |
| <a href="https://tailwindcss.com/docs/top-right-bottom-left" target="_blank" rel="noopener noreferrer"><code>bottom</code></a> | Set a typed bottom offset. |
| <a href="https://tailwindcss.com/docs/top-right-bottom-left" target="_blank" rel="noopener noreferrer"><code>left</code></a> | Set a typed left offset. |
| <a href="https://tailwindcss.com/docs/top-right-bottom-left" target="_blank" rel="noopener noreferrer"><code>right</code></a> | Set a typed right offset. |
| <a href="https://tailwindcss.com/docs/scrollbar-width" target="_blank" rel="noopener noreferrer"><code>scrollbar_width</code></a> | Reserve a typed scrollbar width for scrolling layout. |

### Color and borders

| Method | Description |
| --- | --- |
| <a href="https://tailwindcss.com/docs/background-color" target="_blank" rel="noopener noreferrer"><code>bg</code></a> | Set a typed background fill. |
| <a href="https://tailwindcss.com/docs/color" target="_blank" rel="noopener noreferrer"><code>text_color</code></a> | Set a typed foreground color. |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_1</code></a> | Set a one pixel border on all sides. |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_t_1</code></a> | Set a one pixel top border. |
| <a href="https://tailwindcss.com/docs/border-color" target="_blank" rel="noopener noreferrer"><code>border_color</code></a> | Set a typed border color. |
| <a href="https://tailwindcss.com/docs/border-style" target="_blank" rel="noopener noreferrer"><code>border_dashed</code></a> | Draw borders with a dashed style. |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded</code></a> | Set a typed corner radius. |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_lg</code></a> | Use the named large corner radius. |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_full</code></a> | Round corners as far as the size permits. |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border</code></a> | Set a typed border width on all sides. |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_t</code></a> | Set a typed top border width. |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_b</code></a> | Set a typed bottom border width. |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_l</code></a> | Set a typed left border width. |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_r</code></a> | Set a typed right border width. |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_x</code></a> | Set typed left and right border widths. |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_y</code></a> | Set typed top and bottom border widths. |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_t</code></a> | Set typed radii on the top corners. |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_b</code></a> | Set typed radii on the bottom corners. |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_l</code></a> | Set typed radii on the left corners. |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_r</code></a> | Set typed radii on the right corners. |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_tl</code></a> | Set a typed top-left radius. |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_tr</code></a> | Set a typed top-right radius. |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_bl</code></a> | Set a typed bottom-left radius. |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_br</code></a> | Set a typed bottom-right radius. |

### Typography

| Method | Description |
| --- | --- |
| <a href="https://tailwindcss.com/docs/font-family" target="_blank" rel="noopener noreferrer"><code>font_family</code></a> | Set a font family by name. |
| <a href="https://tailwindcss.com/docs/font-weight" target="_blank" rel="noopener noreferrer"><code>font_weight</code></a> | Set a typed font weight. |
| <a href="https://tailwindcss.com/docs/font-style" target="_blank" rel="noopener noreferrer"><code>italic</code></a> | Use italic text. |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_size</code></a> | Set a typed font size. |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_xs</code></a> | Use the extra small text size. |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_sm</code></a> | Use the small text size. |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_lg</code></a> | Use the large text size. |
| <a href="https://tailwindcss.com/docs/text-align" target="_blank" rel="noopener noreferrer"><code>text_left</code></a> | Align text to the left. |
| <a href="https://tailwindcss.com/docs/text-align" target="_blank" rel="noopener noreferrer"><code>text_center</code></a> | Center text within its line. |
| <a href="https://tailwindcss.com/docs/text-align" target="_blank" rel="noopener noreferrer"><code>text_right</code></a> | Align text to the right. |
| <a href="https://tailwindcss.com/docs/line-height" target="_blank" rel="noopener noreferrer"><code>line_height</code></a> | Set a typed line height. |
| <a href="https://tailwindcss.com/docs/white-space" target="_blank" rel="noopener noreferrer"><code>whitespace_normal</code></a> | Allow normal text wrapping. |
| <a href="https://tailwindcss.com/docs/white-space" target="_blank" rel="noopener noreferrer"><code>whitespace_nowrap</code></a> | Prevent text from wrapping. |
| <a href="https://tailwindcss.com/docs/text-overflow" target="_blank" rel="noopener noreferrer"><code>text_ellipsis</code></a> | Truncate overflowing text at the end with an ellipsis. |
| <a href="https://tailwindcss.com/docs/text-overflow" target="_blank" rel="noopener noreferrer"><code>truncate</code></a> | Clip single line text and add an ellipsis. |
| <a href="https://tailwindcss.com/docs/line-clamp" target="_blank" rel="noopener noreferrer"><code>line_clamp</code></a> | Limit text to a chosen number of lines. |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_base</code></a> | Use the base text size. |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_xl</code></a> | Use the extra large text size. |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_2xl</code></a> | Use the 2× large text size. |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_3xl</code></a> | Use the 3× large text size. |
| <a href="https://tailwindcss.com/docs/text-align" target="_blank" rel="noopener noreferrer"><code>text_align</code></a> | Set a typed text alignment. |
| <a href="https://tailwindcss.com/docs/font-style" target="_blank" rel="noopener noreferrer"><code>not_italic</code></a> | Use upright text. |
| <a href="https://tailwindcss.com/docs/text-decoration-line" target="_blank" rel="noopener noreferrer"><code>underline</code></a> | Underline the text. |
| <a href="https://tailwindcss.com/docs/text-decoration-line" target="_blank" rel="noopener noreferrer"><code>line_through</code></a> | Strike through the text. |
| <a href="https://tailwindcss.com/docs/text-decoration-line" target="_blank" rel="noopener noreferrer"><code>text_decoration_none</code></a> | Remove text decoration. |
| <a href="https://tailwindcss.com/docs/text-decoration-color" target="_blank" rel="noopener noreferrer"><code>text_decoration_color</code></a> | Set text decoration color. |
| <a href="https://tailwindcss.com/docs/text-decoration-style" target="_blank" rel="noopener noreferrer"><code>text_decoration_solid</code></a> | Use a solid text decoration line. |
| <a href="https://tailwindcss.com/docs/text-decoration-style" target="_blank" rel="noopener noreferrer"><code>text_decoration_wavy</code></a> | Use a wavy text decoration line. |
| <a href="https://tailwindcss.com/docs/text-decoration-thickness" target="_blank" rel="noopener noreferrer"><code>text_decoration_0</code></a> | Set decoration thickness to zero. |
| <a href="https://tailwindcss.com/docs/text-decoration-thickness" target="_blank" rel="noopener noreferrer"><code>text_decoration_1</code></a> | Set decoration thickness to one pixel. |
| <a href="https://tailwindcss.com/docs/text-decoration-thickness" target="_blank" rel="noopener noreferrer"><code>text_decoration_2</code></a> | Set decoration thickness to two pixels. |
| <a href="https://tailwindcss.com/docs/text-decoration-thickness" target="_blank" rel="noopener noreferrer"><code>text_decoration_4</code></a> | Set decoration thickness to four pixels. |
| <a href="https://tailwindcss.com/docs/text-decoration-thickness" target="_blank" rel="noopener noreferrer"><code>text_decoration_8</code></a> | Set decoration thickness to eight pixels. |
| <a href="https://tailwindcss.com/docs/text-overflow" target="_blank" rel="noopener noreferrer"><code>text_overflow</code></a> | Set typed text overflow behavior. |
| <a href="https://tailwindcss.com/docs/font-feature-settings" target="_blank" rel="noopener noreferrer"><code>font_features</code></a> | Set OpenType font features. |

### Effects and cursor

| Method | Description |
| --- | --- |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_none</code></a> | Remove box shadows. |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_sm</code></a> | Apply the named small box shadow. |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_md</code></a> | Apply the named medium box shadow. |
| <a href="https://tailwindcss.com/docs/opacity" target="_blank" rel="noopener noreferrer"><code>opacity</code></a> | Set opacity with a floating point value. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_pointer</code></a> | Use the pointing hand cursor on hover. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_text</code></a> | Use a text insertion cursor on hover. |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow</code></a> | Set a typed list of box shadows. |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_2xs</code></a> | Apply the named 2× extra small shadow. |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_xs</code></a> | Apply the named extra small shadow. |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_lg</code></a> | Apply the named large shadow. |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_xl</code></a> | Apply the named extra large shadow. |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_2xl</code></a> | Apply the named 2× extra large shadow. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor</code></a> | Set a typed mouse cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_default</code></a> | Use the default cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_move</code></a> | Use a move cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_not_allowed</code></a> | Use the not-allowed cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_context_menu</code></a> | Use a context-menu cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_crosshair</code></a> | Use a crosshair cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_vertical_text</code></a> | Use a vertical-text cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_alias</code></a> | Use an alias cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_copy</code></a> | Use a copy cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_no_drop</code></a> | Use a no-drop cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_grab</code></a> | Use a grab cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_grabbing</code></a> | Use a grabbing cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_ew_resize</code></a> | Use a horizontal resize cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_ns_resize</code></a> | Use a vertical resize cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_nesw_resize</code></a> | Use a northeast to southwest resize cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_nwse_resize</code></a> | Use a northwest to southeast resize cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_col_resize</code></a> | Use a column resize cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_row_resize</code></a> | Use a row resize cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_n_resize</code></a> | Use an upward resize cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_e_resize</code></a> | Use a rightward resize cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_s_resize</code></a> | Use a downward resize cursor. |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_w_resize</code></a> | Use a leftward resize cursor. |

### GPUI-specific methods

These APIs have no direct Tailwind utility. Their names remain unlinked so the table does not suggest a false correspondence.

| Method | Description |
| --- | --- |
| `font` | Replace the text font with a typed GPUI `Font`. |
| `min_size` | Set typed minimum width and height together. |
| `max_size` | Set typed maximum width and height together. |
| `text_bg` | Set the background color of text runs, rather than the element box. |
| `text_ellipsis_start` | Truncate at the start to preserve the end of text. |
| `text_ellipsis_middle` | Truncate in the middle to preserve both ends. |
| `debug` | Draw a debug outline in debug builds. |
| `debug_below` | Draw debug outlines for this element and conforming descendants in debug builds. |
| `style` | Return the mutable `StyleRefinement` used by the element; this is the trait's low-level accessor. |
| `text_style` | Return the mutable text refinement within the element style. |
| `grid_location_mut` | Access the mutable grid placement within a `StyleRefinement`. |

The macros also generate methods for the size (`w`, `h`, `size`, `min_size`, `min_w`, `min_h`, `max_size`, `max_w`, `max_h`), gap (`gap`, `gap_x`, `gap_y`), margin (`m`, `mt`, `mb`, `mx`, `my`, `ml`, `mr`), padding (`p`, `pt`, `pb`, `px`, `py`, `pl`, `pr`), and inset (`inset`, `top`, `bottom`, `left`, `right`) families. For example, `w_64`, `px_3`, and `top_0` are real methods. Their shared numeric suffixes are `0`, `0p5`, `1`, `1p5`, `2`, `2p5`, `3`, `3p5`, every integer from `4` to `12`, then `16`, `20`, `24`, `32`, `40`, `48`, `56`, `64`, `72`, `80`, `96`, `112`, and `128`. They also include `_px`, `_full`, and fractional suffixes such as `_1_2`; only families that accept `auto` generate `_auto`. Non-auto values also have `_neg_` forms, such as `mt_neg_2`. Border sides (`border`, `border_t`, `border_b`, `border_l`, `border_r`, `border_x`, `border_y`) have pixel-width suffixes `0` through `12`, plus `16`, `20`, `24`, and `32`. Rounded sides and corners use `none`, `xs`, `sm`, `md`, `lg`, `xl`, `2xl`, `3xl`, and `full`. Use these mechanically generated names only when the resulting property makes sense for the layout.

Spacing helpers use a rem based scale: `_1` is `0.25rem`, `_2` is `0.5rem`, `_3` is `0.75rem`, and `_4` is `1rem`. For a value outside the named helpers, use a typed setter such as `.gap(rems(0.625))`, `.w(px(240.))`, or `.w(relative(0.5))`. `relative(0.5)` expresses half the available relative size; `px(...)` expresses pixels. The named scale and method set are GPUI's implementation, so check the actual API rather than assuming every Tailwind class exists.

## What a style call changes

Every `Styled` element exposes `fn style(&mut self) -> &mut StyleRefinement`. A call such as `.px_3()` writes the relevant optional padding fields; `.bg(...)` writes the background field. Fields left unset do not replace existing values when refinements are merged. The resolved `Style` holds both layout data and presentation data.

```text
Styled calls → StyleRefinement → resolved Style
                                    ├─ layout fields → Taffy → bounds
                                    └─ color, text, shadow, cursor → GPUI paint and interaction
```

In an element's `request_layout` phase, GPUI passes layout fields such as display, size, padding, gap, flex alignment, position, and grid placement, together with child layout IDs, to Taffy. Taffy computes the geometry. GPUI then uses the bounds in `prepaint` and the [Paint](./paint) phase for drawing and hit testing. Taffy does not implement GPUI's text shaping, hover listeners, [Actions](./action), or painting.

`StyleRefinement` itself implements `Styled`, so a state style closure can use the same utility methods. Interaction variants such as `.hover(|style| style.bg(...))` belong to `InteractiveElement`, and need an interactive element. Conditional builder calls are different: `.when(...)` chooses a chain step while this frame is built.

## Fluent composition and trait boundaries

The fluent surface combines several traits. `Styled` supplies style methods. `ParentElement` supplies `.child(...)`. `InteractiveElement` supplies `.id(...)`, returning a `Stateful<Div>` that supports identity dependent methods such as `.overflow_y_scroll()`. `InteractiveElement` also supplies state style refinements such as `.hover(...)`. Every `IntoElement` implements `FluentBuilder`; `IntoElement` alone does not grant styling, children, or interaction. If a method is missing, check the receiver's trait and whether a preceding call changed its type.

| `FluentBuilder` method | Effect |
| --- | --- |
| `map` | Transform the value and optionally change its return type. |
| `when` | Apply a builder step when a boolean is true. |
| `when_else` | Choose between two steps that both return the same builder type. |
| `when_some` | Apply a step and pass the value inside an `Option`. |
| `when_none` | Apply a step when a referenced `Option` is empty. |

```rust
use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;

div()
    .id("result-row")
    .px_3()
    .py_2()
    .when(selected, |row| row.bg(cx.theme().selection))
    .when_some(subtitle, |row, text| row.child(text))
    .child(title)
```

Conditional closures consume and return the builder. The condition is evaluated during the current render; it is not a subscription. `.map(...)` can return a different type. Use an `Entity` to own state that changes across frames.

For example, `.when(selected, ...)` evaluates `selected` while building this frame; `.hover(|style| ...)` installs a style refinement for pointer hover. A scroll call needs a stateful element, so give the intended scroll owner an `.id(...)` first. An arbitrary component implementing `IntoElement` cannot be assumed to accept `.child(...)` or `.bg(...)`; inspect its own builder API or wrap it in a `div()` that owns those styles.

GPUI Kit adds `StyledExt` for neutral helpers such as `h_flex`, `v_flex`, and `refine_style`, and `ThemeStyled` for theme driven appearance such as `popover_style(cx)`. These are extensions on top of GPUI's `Styled`, not Tailwind utilities. When implementing a custom element, return its `StyleRefinement` from `style()` and apply the resolved style during layout and paint. Implement only the child and interaction traits that the element can actually support.

The correspondence with Tailwind is deliberately scoped: there are no CSS selectors, cascade, responsive prefixes, or promise that every Tailwind utility has a GPUI method. Use GPUI's typed layout helpers for geometry, GPUI Kit theme tokens for product appearance, and explicit element or entity APIs for behavior.
