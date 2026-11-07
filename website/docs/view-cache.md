---
title: View Cache
description: Reuse clean GPUI view subtrees and distinguish view caching from element state, geometry caching, and virtualization.
order: -2.633
---

# View Cache

GPUI keeps application state in [entities](./entity), but normally builds a new [element tree](./element) for each window draw. Keeping an `Entity<T>` alive does not keep its last element tree alive. When a window redraws, an ordinary child view may run `Render::render` again even if its own state did not change. A cached view instead reuses selected records from the last frame, including its input handlers.

There is no public type named `ViewCache` to construct. The view-cache API in current GPUI is **`Entity<T>::cached(style)`** (or **`AnyView::cached(style)`**). It creates a cached view boundary for one entity-backed subtree. GPUI Kit also uses other, narrower caches; they solve different costs.

| Mechanism | Reuses or avoids | Lifetime and owner |
| --- | --- | --- |
| `Entity::cached(style)` | A clean view's render, child layout/prepaint, and paint work | GPUI's window cache, keyed by the entity view and its element path |
| [`Window::use_keyed_state`](./window) | Small state or computed data used by a rebuilt element | Window element state under a stable [`ElementId`](./element_id) |
| A model-owned cache | A derived value, measurement, or drawing resource | An owning `Entity<T>`, with application-defined invalidation |
| `VirtualList` | Building offscreen rows | Its visible range; a separate scroll handle keeps scroll position |

These mechanisms can be combined. For example, a cached panel may contain a virtual list, and a visible chart row may reuse tessellated paths. A virtual list does **not** cache all of its row views, and `use_keyed_state` does **not** skip a view's `render`.

Think of three separate questions: **What requests a redraw?** `cx.notify()` reports changed Entity output, and `window.refresh()` requests a full window refresh. **What survives a rebuilt element?** An Entity owned by the application or state stored under an element key can survive. **What work can be skipped in the new frame?** Only a clean cached View can replay its recorded subtree; a keyed state value or path cache merely supplies reusable data to work that still runs. A cached scene is GPUI's previous-frame paint record, not an application-owned image or a separately addressable texture.

## Cache an entity-backed subtree

This example uses the existing `hello_world` package. Replace `examples/hello_world/src/main.rs` with the following code, then run `cargo run -p hello_world`. The two buttons make the cache boundary observable without a profiler: the terminal prints whenever either View's `render` runs.

```rust
use gpui_kit::base::StyledExt;
use gpui_kit::component::button::Button;
use gpui_kit::*;

struct Workspace {
    panel: Entity<ResultsPanel>,
    parent_clicks: usize,
}

struct ResultsPanel {
    panel_clicks: usize,
}

impl Render for ResultsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        println!("panel render: {}", self.panel_clicks);
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .child(format!("Panel clicks: {}", self.panel_clicks))
            .child(Button::new("panel-click").label("Update panel").on_click(
                cx.listener(|this, _, _, cx| {
                    this.panel_clicks += 1;
                    cx.notify();
                }),
            ))
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        println!("workspace render: {}", self.parent_clicks);
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .child(format!("Workspace clicks: {}", self.parent_clicks))
            .child(Button::new("workspace-click").label("Update workspace").on_click(
                cx.listener(|this, _, _, cx| {
                    this.parent_clicks += 1;
                    cx.notify();
                }),
            ))
            .child(
                div().w_full().h(px(180.)).child(
                    self.panel
                        .clone()
                        .cached(StyleRefinement::default().size_full()),
                ),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            let panel = cx.new(|_| ResultsPanel { panel_clicks: 0 });
            cx.new(|_| Workspace {
                panel,
                parent_clicks: 0,
            })
        })
        .expect("failed to open window");
    });
}
```

After the first frame, click **Update workspace** several times. `Workspace` renders again, while `ResultsPanel` can be replayed without another `panel render` line. **Update panel** still works inside the cached area: it changes the child Entity, calls `cx.notify()`, and produces another `panel render` line with the new count. The exact number of surrounding window frames is platform and input dependent; compare the two render logs rather than expecting one draw per click.

`panel` is created once with `cx.new(...)` and retained by `Workspace` (see [Context](./context)). Creating a fresh entity inside `render` gives it a new identity and loses both its state and its warm cache. The `style` argument is the cached view's **outer layout contract**. GPUI lays out that box before deciding whether to reuse its contents; it cannot ask an unrendered subtree for an intrinsic size. The example's fixed-height parent supplies a definite box, while the cached child fills it. For content-sized views, embed the entity normally with `.child(self.panel.clone())`.

`AnyView::cached(style)` has the same behavior when a parent stores a type-erased panel, as GPUI Kit's dock does. [`RenderOnce`](./render-once) values and arbitrary `ViewElement`s cannot opt into this API: they have no entity notification contract to invalidate a frozen subtree.

The cache belongs to the **window frame**, not to the `Entity<T>` itself. The same entity rendered in two windows has separate frame records. GPUI locates the record by the View's entity-derived element ID within its keyed ancestor path, so moving a View to another path does not carry its old recorded subtree with it. Retaining an Entity preserves its state; retaining its position and a clean cache key makes frame reuse possible. [GPUI Kit's dock panel](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/dock/tab_panel.rs) embeds an `AnyView` with `cached(StyleRefinement::default().absolute().size_full())` in a sized tab area.

## When does GPUI reuse it?

On a cache hit, GPUI does not call that child view's `render`. It replays the earlier subtree's prepaint and paint records into the current frame. These include hitboxes, dispatch nodes, focus state, mouse listeners, input handlers, and the drawing scene, so the cached area remains interactive. The original element values are not kept as a permanent tree. Event handlers in a reused subtree run again on input; only the work to rebuild and paint them is skipped.

For a hit, the entity must remain clean, and the cached view's **bounds, content mask, and inherited text style** must match the recorded frame. GPUI also bypasses reuse during a forced refresh; inspector picking can disable caching. If any of these conditions fail, it renders and lays out the child again, then records a new cache entry.

| Change | What happens at this boundary |
| --- | --- |
| A sibling or parent redraws, while this child and its box stay unchanged | The parent still builds its tree; the cached child can be replayed. |
| The child changes and calls `cx.notify()` | GPUI marks the view dirty, rebuilds it, and updates the cache. |
| A descendant view changes and notifies | GPUI marks its ancestor view path dirty so the cached boundary is rebuilt. |
| The cached box resizes, clips differently, or inherits a different text style | GPUI misses this cache entry and rebuilds it. |
| The child is removed or its identity/path changes | Its old cached subtree cannot be used at the new position. |

Keep state mutations in event handlers or [tasks](./task) and call `cx.notify()` on the affected entity when its visible output changes. If the child's output depends on a [Global](./global), observe that global and notify the child; changing a global by itself does not dirty cached readers. If it depends on another entity, use an observation or another explicit invalidation path. Do not assume that a parent re-render alone will refresh a clean cached child. Use `window.refresh()` for an intentional full refresh, not as a normal state-update mechanism.

An observation must target the View whose cached output depends on the source. For example, if a panel reads a model Entity during `render`, a model update alone does not tell GPUI that the panel's *View* is dirty. Arrange for the panel to observe the model and call the panel's `cx.notify()` when the displayed value changes. The same rule applies to theme, locale, or other Global values read inside the boundary. A cache hit intentionally skips both the reads and the closures that would have been built by that render, so a fresh value captured by the parent is not sufficient to refresh it.

Caching has a scope: it can skip work *inside* the child boundary, but the window still draws a frame and the parent still runs as needed. The first draw and every cache miss pay the ordinary render/layout/paint cost. Use it where a measured subtree is expensive and often stays clean while surrounding content changes.

## Element state is a different cache

GPUI recreates value-like elements on subsequent renders. If an element needs a little state across consecutive frames, `Window::use_keyed_state` stores an `Entity<S>` under the current element path plus a supplied key. It also observes that state entity and notifies the current View when the state changes. The state survives while that path is accessed on successive frames, including frames where a cached subtree replays its element-state accesses; it is released when the path disappears and no other strong handle keeps it alive. `Window::use_state` uses a call-site key, which is suitable only where that location uniquely identifies the state. An `ElementId` derived from stable domain data matters for repeated or reorderable items: changing an ID resets the state; reusing one for unrelated siblings risks collision.

GPUI Kit's [`Plot` path cache](../component/plot) is a concrete example. A plot and each `Line` are rebuilt as values, so a path held on a `Line` would disappear with that value. `PathCaches::for_paint("lines", window, cx)` stores caches in keyed window state under the plot's element ID. A `ShapeKey` covers projected points and geometry-affecting stroke settings; `PathCache::get` tessellates only when that key changes. The path is built relative to zero and translated to the current origin for painting, so moving a chart can reuse the geometry. A color change can be applied while painting without rebuilding unchanged path geometry.

That cache saves **path construction**, not the plot view's `render` or the current frame's paint submission. Its slots are positional: when series can reorder, map stable series identities to slots or ensure the shape key safely invalidates the changed slot. See [Paint](./paint#plot-a-value-like-element-uses-keyed-window-state) for the source-level walkthrough.

The key must include every input that changes the cached **geometry**: projected points, size, stroke width, curve style, and any other tessellation setting used by the builder. It need not include a paint-only color when that color is selected at paint time. A key that omits a geometry input can draw stale paths; a key that includes the absolute origin forfeits reuse while scrolling. GPUI Kit's [`PathCache::get` and `ShapeKey`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/plot/path_cache.rs) make this dependency explicit. Their `PathCaches` owner is the window's keyed element state, so removal of the plot path for a frame ends that window-owned cache even if a later plot uses the same ID.

## Virtualization avoids work instead of replaying it

For a long collection, caching a view containing every row still leaves an expensive first render and invalidations. GPUI Kit's [`VirtualList`](../base/virtual-list) accepts item sizes and calls its render closure for the visible range (with a small overdraw); it may also render one representative item for cross-axis measurement. It never constructs most offscreen row elements in that frame. Its `VirtualListScrollHandle` is retained separately in the owner so scrolling survives element rebuilds.

```rust
use std::rc::Rc;
use gpui_kit::*;
use gpui_kit::base::{v_virtual_list, VirtualListScrollHandle};

struct Row {
    id: u64,
    name: SharedString,
}

struct ResultsList {
    rows: Vec<Row>,
    sizes: Rc<Vec<Size<Pixels>>>,
    scroll: VirtualListScrollHandle,
}

impl Render for ResultsList {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_virtual_list(cx.entity(), "results", self.sizes.clone(), |this, range, _, _| {
            range
                .map(|ix| {
                    div()
                        .id(("result", this.rows[ix].id))
                        .child(this.rows[ix].name.clone())
                })
                .collect::<Vec<_>>()
        })
        .track_scroll(&self.scroll)
        .size_full()
    }
}
```

The closure should read prepared data; avoid sorting, loading, or creating one long-lived entity per row in it. Give repeated interactive rows stable IDs from the row data. Virtualization and cached views address different dimensions: how **many** elements are made, and whether an **unchanged subtree** is replayed.

## Choose the smallest useful boundary

Start with ordinary entities and declarative rendering. If profiling shows a stable panel being rebuilt because nearby UI changes, place `cached(style)` around that panel and give it a reliable layout box. If a drawing operation remains costly on every visible frame, cache its derived geometry with explicit keys and invalidation. If the cost grows with collection length, virtualize. Do not add a broad cache to hide render work that should have been moved out of `render` or data that should have been retained by an entity.

The [Render](./render) guide explains when a View creates a new tree.

## Verify behavior before keeping a cache

Count calls to the child View's `render` in a focused test or profiler, and compare these cases after the first draw:

1. Redraw an unrelated sibling while the cached box stays fixed. The child should not render again; its controls should still receive input.
2. Change the child's displayed state, call its `cx.notify()`, and verify that the child renders again and the new content appears.
3. Change a dependency outside the child, such as a model or Global, through its real observation path. Verify that the child rebuilds. If the parent changes but the child does not, the missing dependency notification is a correctness bug.
4. Resize or move the boundary, change its clipping or inherited text style, and check that the expected miss occurs. Moving the box changes its bounds; scrolling a plot may leave its **path geometry** cache warm while the View scene still repaints.

GPUI Kit's [cached text-selection test](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/text/window_selection.rs) uses a render counter and checks selection during replayed frames. Use a release/profile build for performance measurements: an extra cache boundary has bookkeeping cost, and a subtree that changes every frame may not benefit. For stale output, inspect Entity notifications and external dependencies first; for unexpected misses, inspect entity identity, keyed ancestor path, bounds, content mask, and inherited text style.
